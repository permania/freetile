use std::{
    io::{self, Read, Write},
    os::fd::AsFd,
};

use nix::{
    errno::Errno,
    poll::PollTimeout,
    sys::epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
};
use x11rb::{
    connection::Connection,
    protocol::{
        Event,
        xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask, NotifyDetail, NotifyMode},
    },
};

use super::{
    ewmh,
    wm::{
        WM,
        focus::{focus_and_warp, focus_window, is_mapped},
        layout::{map_intent, retile},
    },
};
use crate::ipc::{self, Response};

pub fn event_loop(wm: &mut WM) {
    let epoll = Epoll::new(EpollCreateFlags::empty()).expect("failed to create epoll");

    epoll
        .add(wm.conn.stream(), EpollEvent::new(EpollFlags::EPOLLIN, 0))
        .expect("failed to add event");

    epoll
        .add(
            wm.ipc_listener.as_fd(),
            EpollEvent::new(EpollFlags::EPOLLIN, 0),
        )
        .expect("failed to add event");

    let mut events = vec![EpollEvent::empty(); 10];

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

                    let is_dock = ewmh::atom_in_property(
                        wm.conn,
                        window,
                        wm.atoms.net_wm_window_type,
                        wm.atoms.net_wm_window_type_dock,
                    );

                    if is_dock {
                        wm.conn.map_window(window).unwrap();
                        wm.conn.flush().unwrap();
                        continue;
                    }

                    if !wm.state.window_exists(window) {
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
                    if wm.ignore_unmaps.remove(&e.window) {
                        continue;
                    }

                    if !wm.state.window_exists(e.window) {
                        continue;
                    }

                    wm.state.remove_window_everywhere(e.window);
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

            wm.conn.flush().unwrap();
        }

        match wm.ipc_listener.accept() {
            Ok((mut stream, _)) => {
                let mut buf = [0u8; 1024];
                let n = stream.read(&mut buf).unwrap();
                let buf = &buf[..n];

                match ipc::handle_message(buf, wm) {
                    Ok(act) => {
                        act.execute(wm);
                        wm.conn.flush().unwrap();
                        stream.write_all(&[Response::Ok as u8]).unwrap();
                    }
                    Err(code) => {
                        stream.write_all(&[code as u8]).unwrap();
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
            Err(e) => eprintln!("ipc error: {e}"),
        }

        loop {
            match epoll.wait(&mut events, PollTimeout::NONE) {
                Ok(_) => break,
                Err(Errno::EINTR) => continue,
                Err(e) => panic!("Epoll wait failed: {e}"),
            }
        }
    }
}
