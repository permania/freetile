use std::io::{self, Read};

use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask, NotifyDetail, NotifyMode},
    },
};

use super::wm::{WM, focus_and_warp, focus_window, is_mapped, retile};
use crate::{
    handler::wm::map_intent,
    ipc,
};

pub fn event_loop(wm: &mut WM) {
    loop {
        while let Some(event) = wm.conn.poll_for_event().unwrap() {
            match event {
                Event::Error(e) => eprintln!("error: {:?}", e),
                Event::MapRequest(e) => {
                    let window = e.window;

                    let Some(attrs) = wm
                        .conn
                        .get_window_attributes(e.window)
                        .ok()
                        .and_then(|c| c.reply().ok())
                    else {
                        continue;
                    };

                    if attrs.override_redirect {
                        continue;
                    }

                    if !wm.state.windows().contains(&window) {
                        wm.state.windows_mut().push(window);
                    }

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
		    for tag in wm.state.tags.iter_mut() {
			tag.windows_mut().retain(|&w| w != e.window);
		    }
                }
                Event::EnterNotify(e) => {
                    if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                        focus_window(wm, e.event);
                        wm.conn.flush().unwrap();
                    }
                }
                e => eprintln!("event: {:?}", e),
            }
        }

        match wm.ipc_listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = Vec::new();
                stream.read_to_end(&mut buf).unwrap();
                let action = ipc::handle_message(&buf);
                dbg!(&action);

                if let Some(a) = action {
                    a.execute(wm);
                    wm.conn.flush().unwrap()
                } else {
                    eprintln!("bad ipc request!");
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => eprintln!("ipc error: {e}"),
        }
    }
}
