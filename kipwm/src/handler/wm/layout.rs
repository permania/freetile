use rhai::{Dynamic, Scope};
use x11rb::{
    connection::Connection,
    protocol::xproto::{ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, Window},
};

use super::{
    WM,
    focus::{is_mapped, wrap_next, wrap_prev},
};
use crate::config::{
    ksn_reader::WMConfig,
    layout::{
        reader,
        rhai::{LayoutIntent, Rect, WMSlot},
    },
};

pub fn retile(wm: &mut WM) -> LayoutIntent {
    let n = wm.state.windows().len();
    if n == 0 {
        return LayoutIntent {
            mapped: vec![],
            unmapped: vec![],
        };
    }

    let bounds = Rect {
        x: 0,
        y: 0,
        w: wm.screen.width_in_pixels as u32,
        h: wm.screen.height_in_pixels as u32,
    };

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
        None => reader::master(0_i64),
    };

    let rects = slot.compute(
        n,
        bounds,
        wm.config.gap_inner().unwrap_or(0),
        wm.config.gap_outer().unwrap_or(0),
    );
    let windows = wm.state.windows().to_vec();
    let len = windows.len().min(rects.len());

    let mapped: Vec<(Window, Rect)> = (0..len).map(|i| (windows[i], rects[i])).collect();
    let unmapped = windows[len..].to_vec();

    dbg!(&mapped, &unmapped);

    LayoutIntent { mapped, unmapped }
}

pub fn map_intent(wm: &mut WM, intent: LayoutIntent) {
    for (window, rect) in intent.mapped {
        if !is_mapped(wm, window) {
            wm.conn.map_window(window).unwrap();
        }
        configure(wm, window, rect.x, rect.y, rect.w, rect.h);
    }

    for w in intent.unmapped {
        if is_mapped(wm, w) {
            wm.conn.unmap_window(w).unwrap();
        }
    }

    wm.conn.flush().unwrap();
}

fn configure(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32) {
    let border_width = wm.config.border_weight().unwrap_or(0);

    if w == 0 || h == 0 {
        return;
    }
    if w <= border_width * 2 || h <= border_width * 2 {
        return;
    }

    wm.conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0xff444444),
        )
        .unwrap();

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
