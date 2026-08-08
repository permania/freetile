use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, InputFocus, MapState,
        StackMode, Window,
    },
};

use super::{
    WM, WMState,
    layout::{map_intent, retile},
};
use crate::config::ksn_reader::WMConfig;

pub fn focus_and_warp(wm: &mut WM, window: Window) {
    wm.ignore_enter = true;

    focus_window(wm, window);
    warp_to_window(wm, window);
}

pub fn focus_window(wm: &mut WM, window: Window) {
    if let Some(prev) = wm.state.focused() {
        wm.conn
            .change_window_attributes(
                prev,
                &ChangeWindowAttributesAux::new()
                    .border_pixel(wm.config.inactive_border_color().unwrap_or(0xff444444)),
            )
            .unwrap();
    }

    wm.conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new()
                .border_pixel(wm.config.active_border_color().unwrap_or(0xff8aadf4)),
        )
        .unwrap();

    wm.conn
        .configure_window(
            window,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )
        .unwrap();

    wm.conn
        .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .unwrap();

    wm.state.set_focused(Some(window));
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

pub fn focused_index(wm: &WMState) -> Option<usize> {
    let win = wm.focused()?;
    wm.windows().iter().position(|&w| w == win)
}

pub fn wrap_next(i: usize, len: usize) -> usize {
    (i + 1) % len
}

pub fn wrap_prev(i: usize, len: usize) -> usize {
    (i + len - 1) % len
}
