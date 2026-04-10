use std::process::Command;

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            ChangeWindowAttributesAux, ClientMessageEvent, ConnectionExt, EventMask, KeyButMask,
            ModMask, NotifyDetail, NotifyMode, Screen,
        },
    },
};

use crate::handler::{
    keys::{KeyResult, keysym_from_keycode},
    wm::{WMAction, switch_workspace},
};

use super::{
    keys::KeyBind,
    wm::{WMState, focus_and_warp, focus_window, retile},
};

pub fn event_loop(
    conn: &impl Connection,
    screen: &Screen,
    wm_state: &mut WMState,
    keybinds: Vec<KeyBind>,
) {
    loop {
        match conn.wait_for_event().unwrap() {
            Event::Error(e) => eprintln!("error: {:?}", e),
            Event::MapRequest(e) => {
                let window = e.window;
                wm_state.windows_mut().push(window);
                conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                )
                .unwrap();
                conn.map_window(window).unwrap();
                retile(&conn, &screen, wm_state);

                focus_and_warp(&conn, &screen, window, wm_state);

                conn.flush().unwrap();
            }
            Event::UnmapNotify(e) => {
                wm_state.windows_mut().retain(|&w| w != e.window);
                if wm_state.focused() == &Some(e.window) {
                    wm_state.set_focused(wm_state.windows().last().copied());
                }
                retile(&conn, &screen, wm_state);
                conn.clear_area(false, screen.root, 0, 0, 0, 0).unwrap();
                conn.flush().unwrap();
            }
            Event::DestroyNotify(e) => {
                wm_state.windows_mut().retain(|&w| w != e.window);
            }
            Event::EnterNotify(e) => {
                if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                    focus_window(&conn, e.event, wm_state);
                }
            }
            Event::KeyPress(e) => {
                println!("key: {:#?}", e);
                let keycode = e.detail;
                let state = e.state;
                let idx = (keycode - wm_state.first_keycode) as usize
                    * wm_state.syms_per_keycode as usize;
                let sym = keysym_from_keycode(idx, &wm_state.keysyms);

                let clean_mask: u16 = !(u16::from(ModMask::M2) | u16::from(ModMask::LOCK));
                let pressed = KeyResult {
                    sym,
                    mods: KeyButMask::from(u16::from(state) & clean_mask),
                };

                for bind in &keybinds {
                    if bind.matches(&pressed) {
                        match &bind.action {
                            WMAction::Spawn(cmd, args) => {
                                Command::new(cmd).args(args).spawn().unwrap();
                            }
                            WMAction::Kill => {
                                if let &Some(win) = wm_state.focused() {
                                    if wm_state.windows().contains(&win) {
                                        // send WM_DELETE_WINDOW message
                                        let wm_protocols = conn
                                            .intern_atom(false, b"WM_PROTOCOLS")
                                            .unwrap()
                                            .reply()
                                            .unwrap()
                                            .atom;
                                        let wm_delete = conn
                                            .intern_atom(false, b"WM_DELETE_WINDOW")
                                            .unwrap()
                                            .reply()
                                            .unwrap()
                                            .atom;

                                        let data = [wm_delete, 0, 0, 0, 0];
                                        conn.send_event(
                                            false,
                                            win,
                                            EventMask::NO_EVENT,
                                            ClientMessageEvent::new(32, win, wm_protocols, data),
                                        )
                                        .unwrap();
                                        conn.flush().unwrap();
                                    }
                                }
                            }
                            WMAction::FocusNext => {
                                if let &Some(win) = wm_state.focused() {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
                                        let next = wm_state.windows()
                                            [(idx + 1) % wm_state.windows().len()];
                                        focus_and_warp(&conn, screen, next, wm_state);
                                    }
                                }
                            }
                            WMAction::FocusPrevious => {
                                if let &Some(win) = wm_state.focused() {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
                                        let prev = wm_state.windows()[(idx
                                            + wm_state.windows().len()
                                            - 1)
                                            % wm_state.windows().len()];
                                        focus_and_warp(conn, screen, prev, wm_state);
                                    }
                                }
                            }
                            WMAction::SwapNext => {
                                if let &Some(win) = wm_state.focused() {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
					let next_idx = (idx + 1) % wm_state.windows().len();
                                        wm_state.windows_mut().swap(idx, next_idx);
                                        retile(conn, screen, wm_state);
					focus_and_warp(&conn, screen, win, wm_state);
                                    }
                                }
                            }
                            WMAction::SwapPrevious => {
                                if let &Some(win) = wm_state.focused() {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
                                        let prev_idx = (idx + wm_state.windows().len() - 1)
                                            % wm_state.windows().len();
                                        wm_state.windows_mut().swap(idx, prev_idx);
                                        retile(conn, screen, wm_state);
					focus_and_warp(&conn, screen, win, wm_state);
                                    }
                                }
                            }
                            WMAction::TagSwitch(idx) => {
                                switch_workspace(&conn, screen, wm_state, idx);
                            }
                        }
                    }
                }
            }
            e => eprintln!("event: {:?}", e),
        }
    }
}
