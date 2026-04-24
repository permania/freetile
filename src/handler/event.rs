use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{
            ChangeWindowAttributesAux, ConnectionExt, EventMask, NotifyDetail, NotifyMode, Screen,
        },
    },
};

use crate::handler::keys::event_to_keyresult;

use super::{
    keys::KeyBind,
    wm::{WM, WMState, focus_and_warp, focus_window, retile},
};

pub fn event_loop(
    wm: &mut WM,
) {
    loop {
        match wm.conn.wait_for_event().unwrap() {
            Event::Error(e) => eprintln!("error: {:?}", e),
            Event::MapRequest(e) => {
                let window = e.window;
                wm.state.windows_mut().push(window);
                wm.conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                )
                .unwrap();
		wm.conn.map_window(window).unwrap();
                retile(wm);

                focus_and_warp(wm, window);

                wm.conn.flush().unwrap();
            }
            Event::UnmapNotify(e) => {
                wm.state.windows_mut().retain(|&w| w != e.window);
                if wm.state.focused() == Some(e.window) {
                    wm.state.set_focused(wm.state.windows().last().copied());
                }
                retile(wm);
                wm.conn.clear_area(false, wm.screen.root, 0, 0, 0, 0).unwrap();
                wm.conn.flush().unwrap();
            }
            Event::DestroyNotify(e) => {
                wm.state.windows_mut().retain(|&w| w != e.window);
            }
            Event::EnterNotify(e) => {
                if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                    focus_window(wm, e.event );
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
            }
            e => eprintln!("event: {:?}", e),
        }
    }
}
