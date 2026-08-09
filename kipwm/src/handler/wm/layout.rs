use rhai::{Dynamic, Scope};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, PropMode, Window,
    },
    wrapper::ConnectionExt as _,
};

use super::{
    StrutPartial, WM,
    focus::{is_mapped, wrap_next, wrap_prev},
};
use crate::config::{
    ksn_reader::WMConfig,
    layout::rhai::{LayoutIntent, Rect, WMSlot},
};

pub fn retile(wm: &mut WM) -> LayoutIntent {
    let n = wm.state.windows().len() - wm.state.fullscreen_windows_tag().len();

    let all_windows = wm.state.windows().to_vec();
    let fullscreen = wm.state.fullscreen_windows_tag();
    let windows: Vec<Window> = all_windows
        .iter()
        .filter(|w| !fullscreen.contains(w))
        .copied()
        .collect();

    if n == 0 {
        return LayoutIntent {
            mapped: vec![],
            unmapped: vec![],
            fullscreen,
        };
    }

    let bounds = wm.state.bounds(Rect {
        x: 0,
        y: 0,
        w: wm.screen.width_in_pixels as u32,
        h: wm.screen.height_in_pixels as u32,
    });

    let mut scope = Scope::new();

    eprintln!("available layouts:");
    for k in wm.layouts.keys() {
        eprintln!(" - {k}");
    }

    let layout = wm
        .layouts
        .get(&wm.state.active_layout)
        .expect("invalid active layout");

    let func_name = &layout.func_name;

    eprintln!("requested layout = {}", wm.state.active_layout);

    let result: Option<WMSlot> = wm
        .state
        .engine
        .call_fn::<Dynamic>(&mut scope, &wm.state.layout_ast, func_name, (n as i64,))
        .ok()
        .and_then(|d| d.try_cast::<WMSlot>());

    let slot: WMSlot = match result {
        Some(s) => s,
        None => WMSlot::Window,
    };

    let rects = slot.compute(
        n,
        bounds,
        wm.config.gap_inner().unwrap_or(0),
        wm.config.gap_outer().unwrap_or(0),
    );

    let len = windows.len().min(rects.len());

    let mapped: Vec<(Window, Rect)> = (0..len).map(|i| (windows[i], rects[i])).collect();
    let unmapped = windows[len..].to_vec();

    dbg!(&mapped, &unmapped);

    LayoutIntent {
        mapped,
        unmapped,
        fullscreen,
    }
}

pub fn map_intent(wm: &mut WM, intent: LayoutIntent) {
    for (window, rect) in intent.mapped {
        if !is_mapped(wm, window) {
            wm.conn.map_window(window).unwrap();
        }
        configure(wm, window, rect.x, rect.y, rect.w, rect.h);
    }

    for w in intent.fullscreen {
        if !is_mapped(wm, w) {
            wm.conn.map_window(w).unwrap();
        }
        configure_borderless(
            wm,
            w,
            0,
            0,
            wm.screen.width_in_pixels as u32,
            wm.screen.height_in_pixels as u32,
        );
    }

    for w in intent.unmapped {
        if is_mapped(wm, w) {
            wm.conn.unmap_window(w).unwrap();
        }
    }

    wm.conn.flush().unwrap();
}

pub fn set_fullscreen(wm: &mut WM, window: Window, full: bool) {
    dbg!("set_fullscreen called");

    let fs = wm.state.fullscreen_windows_mut();
    if full {
        fs.insert(window);
    } else {
        fs.remove(&window);

        let color = if wm.state.focused() == Some(window) {
            wm.config.active_border_color().unwrap_or(0xff8aadf4)
        } else {
            wm.config.inactive_border_color().unwrap_or(0xff444444)
        };

        dbg!(window, wm.state.focused(), color);

        wm.conn
            .change_window_attributes(
                window,
                &ChangeWindowAttributesAux::new().border_pixel(color),
            )
            .unwrap();
    }

    let states: Vec<u32> = if full {
        vec![wm.atoms._net_wm_state_fullscreen]
    } else {
        vec![]
    };

    wm.conn
        .change_property32(
            PropMode::REPLACE,
            window,
            wm.atoms._net_wm_state,
            AtomEnum::ATOM,
            &states,
        )
        .unwrap();

    let intent = retile(wm);
    map_intent(wm, intent);
}

// TODO: add this next update
#[allow(dead_code)]
#[allow(unused_variables)]
pub fn set_max(wm: &mut WM, window: Window, max: bool) {
    todo!()
}

fn configure_inner(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32, border_width: u32) {
    if w == 0 || h == 0 {
        return;
    }
    if w <= border_width * 2 || h <= border_width * 2 {
        return;
    }
    wm.conn
        .configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(x)
                .y(y)
                .width(w - border_width * 2)
                .height(h - border_width * 2)
                .border_width(border_width),
        )
        .unwrap();
}

fn configure(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32) {
    let border_width = wm.config.border_weight().unwrap_or(0);
    configure_inner(wm, window, x, y, w, h, border_width);
}

fn configure_borderless(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32) {
    configure_inner(wm, window, x, y, w, h, 0);
}

pub fn read_struts(wm: &mut WM, window: Window) -> Option<StrutPartial> {
    let partial = wm
        .conn
        .get_property(
            false,
            window,
            wm.atoms._net_wm_strut_partial,
            AtomEnum::CARDINAL,
            0,
            12,
        )
        .ok()?
        .reply()
        .ok()?;

    if let Some(res) = partial.value32().map(|vals| {
        let v: Vec<u32> = vals.collect::<Vec<u32>>();
        StrutPartial {
            left: v[0],
            right: v[1],
            top: v[2],
            bottom: v[3],
            left_start_y: v[4],
            left_end_y: v[5],
            right_start_y: v[6],
            right_end_y: v[7],
            top_start_y: v[8],
            top_end_y: v[9],
            bottom_start_y: v[10],
            bottom_end_y: v[11],
        }
    }) {
        return Some(res);
    }

    let legacy = wm
        .conn
        .get_property(
            false,
            window,
            wm.atoms._net_wm_strut,
            AtomEnum::CARDINAL,
            0,
            4,
        )
        .ok()?
        .reply()
        .ok()?;

    legacy.value32().map(|vals| {
        let v: Vec<u32> = vals.collect::<Vec<u32>>();
        StrutPartial {
            left: v[0],
            right: v[1],
            top: v[2],
            bottom: v[3],
            ..Default::default()
        }
    })
}

pub fn switch_layout<T>(wm: &mut WM, layout: T)
where
    T: AsRef<str>,
{
    wm.state.active_layout = layout.as_ref().to_string();

    let intent = retile(wm);
    map_intent(wm, intent);
}

pub fn next_layout(wm: &mut WM) {
    let keys: Vec<String> = wm.layouts.keys().cloned().collect();

    if !keys.is_empty() {
        let i = keys
            .iter()
            .position(|k| k == &wm.state.active_layout)
            .unwrap_or(0);

        let next_key = &keys[wrap_next(i, keys.len())];

        switch_layout(wm, next_key);
    }
}

pub fn prev_layout(wm: &mut WM) {
    let keys: Vec<String> = wm.layouts.keys().cloned().collect();

    if !keys.is_empty() {
        let i = keys
            .iter()
            .position(|k| k == &wm.state.active_layout)
            .unwrap_or(0);

        let next_key = &keys[wrap_prev(i, keys.len())];

        switch_layout(wm, next_key);
    }
}
