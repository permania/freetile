use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask, NotifyDetail, NotifyMode},
    },
};

use crate::handler::{keys::event_to_keyresult, wm::map_intent};

use super::wm::{WM, focus_and_warp, focus_window, is_mapped, retile};

pub fn event_loop(wm: &mut WM) {
    loop {
        match wm.conn.wait_for_event().unwrap() {
            Event::Error(e) => eprintln!("error: {:?}", e),
            Event::MapRequest(e) => {
                let window = e.window;

                wm.state.windows_mut().push(window);

                wm.conn
                    .change_window_attributes(
                        window,
                        &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                    )
                    .unwrap();

                let prev_focused = wm.state.focused();

                let intent = retile(wm);
                map_intent(wm, intent);

                if is_mapped(wm, window) {
                    focus_and_warp(wm, window);
                } else if let Some(p) = prev_focused {
                    focus_and_warp(wm, p);
                }

                wm.conn.flush().unwrap();
            }
            Event::UnmapNotify(e) => {
                if !wm.state.windows().contains(&e.window) {
                    continue;
                }

                if wm.ignore_unmaps > 0 {
                    eprintln!("ignore unmaps is more than 0: {}", wm.ignore_unmaps);
                    wm.ignore_unmaps -= 1;
                    continue;
                }

                wm.state.windows_mut().retain(|&w| w != e.window);
                if wm.state.focused() == Some(e.window) {
                    wm.state.set_focused(wm.state.windows().last().copied());
                }

                let intent = retile(wm);
                map_intent(wm, intent);

                wm.conn
                    .clear_area(false, wm.screen.root, 0, 0, 0, 0)
                    .unwrap();
                wm.conn.flush().unwrap();
            }
            Event::DestroyNotify(e) => {
                wm.state.windows_mut().retain(|&w| w != e.window);
            }
            Event::EnterNotify(e) => {
                if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                    focus_window(wm, e.event);
                    wm.conn.flush().unwrap();
                }
            }
            Event::KeyPress(e) => {
                println!("key: {:#?}", e);
                let pressed = event_to_keyresult(wm, e);

                for bind in wm.keybinds.clone() {
                    if bind.matches(&pressed) {
                        bind.action.execute(wm);
                    }
                }
                wm.conn.flush().unwrap();
            }
            e => eprintln!("event: {:?}", e),
        }
    }
}
