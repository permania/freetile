use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeGCAux, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, CreateGCAux,
        InputFocus, MapState, Rectangle, StackMode, Window,
    },
};

use super::{
    WM,
    layout::{map_intent, retile},
};
use crate::config::ksn_reader::WMConfig;

pub fn focus_and_warp(wm: &mut WM, window: Window) {
    wm.ignore_enter = true;

    focus_window(wm, window);
    warp_to_window(wm, window);
}

pub fn focus_window(wm: &mut WM, window: Window) {
    let prev = wm.state.focused();
    wm.state.set_focused(Some(window));

    if let Some(p) = prev {
        let (prev_inner, prev_outer) = border_colors_for(wm, p);

        set_border_colors(
            wm,
            p,
            prev_inner,
            prev_outer,
            wm.config.border_weight_inner(),
            wm.config.border_weight_outer(),
        );
    } // remove the color from the previous window

    let (inner, outer) = border_colors_for(wm, window);
    set_border_colors(
        wm,
        window,
        inner,
        outer,
        wm.config.border_weight_inner(),
        wm.config.border_weight_outer(),
    );

    wm.conn
        .configure_window(
            window,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )
        .unwrap();

    wm.conn
        .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .unwrap();
}

fn warp_to_window(wm: &mut WM, window: Window) {
    if let Ok(geom) = wm.conn.get_geometry(window).unwrap().reply() {
        let cx = geom.x + (geom.width / 2) as i16;
        let cy = geom.y + (geom.height / 2) as i16;

        wm.conn
            .warp_pointer(x11rb::NONE, wm.screen.root, 0, 0, 0, 0, cx, cy)
            .unwrap();
    }
}

pub fn set_border_colors(
    wm: &mut WM,
    window: Window,
    inner_color: u32,
    outer_color: u32,
    border_inner: u16,
    border_outer: u16,
) {
    /* pixmaps have super annoying wrapping behavior,
    the top left corner of window content is also
    the bottom right corner of the pixmap,
    hence the funky workaround here */

    let border_total: u16 = border_inner + border_outer;

    let geom = match wm.conn.get_geometry(window).unwrap().reply() {
        Ok(g) => g,
        Err(_) => return,
    };

    let pw = geom.width + (2 * border_total);
    let ph = geom.height + (2 * border_total);

    let pixmap = wm.conn.generate_id().unwrap();
    wm.conn
        .create_pixmap(geom.depth, pixmap, window, pw, ph)
        .unwrap();

    let gc = wm.conn.generate_id().unwrap();
    wm.conn.create_gc(gc, pixmap, &CreateGCAux::new()).unwrap();

    wm.conn
        .change_gc(gc, &ChangeGCAux::new().foreground(inner_color))
        .unwrap();
    wm.conn
        .poly_fill_rectangle(
            // just draw the full rect for inner
            pixmap,
            gc,
            &[Rectangle {
                x: 0i16,
                y: 0i16,
                width: pw,
                height: ph,
            }],
        )
        .unwrap();

    wm.conn
        .change_gc(gc, &ChangeGCAux::new().foreground(outer_color))
        .unwrap();
    wm.conn
        .poly_fill_rectangle(
            // draw the outer strips over the full rect
            pixmap,
            gc,
            &[
                Rectangle {
                    x: 0,
                    y: (ph - border_total) as i16,
                    width: pw,
                    height: border_outer,
                }, // top
                Rectangle {
                    x: 0,
                    y: (ph - border_total - border_outer) as i16,
                    width: pw,
                    height: border_outer,
                }, // bottom
                Rectangle {
                    x: (pw - border_total) as i16,
                    y: 0,
                    width: border_outer,
                    height: ph,
                }, // left
                Rectangle {
                    x: (pw - border_total - border_outer) as i16,
                    y: 0,
                    width: border_outer,
                    height: ph,
                }, // right
            ],
        )
        .unwrap();

    wm.conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixmap(pixmap),
        )
        .unwrap();

    wm.conn.free_gc(gc).unwrap();
    wm.conn.free_pixmap(pixmap).unwrap();
    wm.conn.flush().unwrap();
}

pub fn switch_workspace(wm: &mut WM, idx: &usize) {
    if *idx == wm.state.active_tag {
        return;
    }

    for &win in wm.state.windows() {
        wm.ignore_unmaps.insert(win);
        wm.conn.unmap_window(win).unwrap();
    }

    wm.state.active_tag = *idx;

    let intent = retile(wm);

    if wm.state.focused().is_some() {
        if !intent
            .mapped
            .iter()
            .map(|p| p.0)
            .collect::<Vec<u32>>()
            .contains(&wm.state.focused().expect("this should be infallible"))
        {
            wm.state.set_focused(intent.mapped.last().map(|a| a.0));
        }
    } else {
        wm.state.set_focused(wm.state.windows().last().copied());
    }

    map_intent(wm, intent);

    if let Some(win) = wm.state.focused() {
        focus_and_warp(wm, win);
    }

    wm.conn
        .clear_area(false, wm.screen.root, 0, 0, 0, 0)
        .unwrap();
    wm.conn.flush().unwrap();
}

pub fn is_mapped(wm: &WM, window: Window) -> bool {
    wm.conn
        .get_window_attributes(window)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|attrs| attrs.map_state != MapState::UNMAPPED)
        .unwrap_or(false)
}

pub fn focused_index(wm: &WM) -> Option<usize> {
    let win = wm.state.focused()?;
    wm.state.windows().iter().position(|&w| w == win)
}

pub fn wrap_next(i: usize, len: usize) -> usize {
    (i + 1) % len
}

pub fn wrap_prev(i: usize, len: usize) -> usize {
    (i + len - 1) % len
}

pub fn border_colors_for(wm: &WM, window: Window) -> (u32, u32) {
    if wm.state.focused().is_some_and(|w| w == window) {
        (
            wm.config.active_border_color_inner(),
            wm.config.active_border_color_outer(),
        )
    } else {
        (
            wm.config.inactive_border_color_inner(),
            wm.config.inactive_border_color_outer(),
        )
    }
}
