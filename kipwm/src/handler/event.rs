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
        layout::{map_intent, read_struts, retile, set_fullscreen},
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
                        wm.atoms._net_wm_window_type,
                        wm.atoms._net_wm_window_type_dock,
                    );

                    if let Some(s) = read_struts(wm, window) {
                        wm.state.active_struts.insert(window, s);
                        let intent = retile(wm);
                        map_intent(wm, intent);
                    }

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
                            &ChangeWindowAttributesAux::new()
                                .event_mask(EventMask::ENTER_WINDOW)
                                .border_pixel(0xff444444),
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

                    let was_focused = wm.state.focused() == Some(e.window);
                    let killed_idx = wm.state.windows().iter().position(|&w| w == e.window);

                    wm.state.remove_window_everywhere(e.window);

                    let intent = retile(wm);
                    map_intent(wm, intent);

                    if was_focused {
                        let next = killed_idx
                            .and_then(|idx| {
                                let windows = wm.state.windows();
                                windows.get(idx).copied().or_else(|| {
                                    idx.checked_sub(1).and_then(|i| windows.get(i).copied())
                                })
                            })
                            .or_else(|| wm.state.windows().last().copied());

                        if let Some(next) = next {
                            focus_and_warp(wm, next);
                        } else {
                            wm.state.set_focused(None);
                        }
                    }

                    wm.conn
                        .clear_area(false, wm.screen.root, 0, 0, 0, 0)
                        .unwrap();
                    wm.conn.flush().unwrap();
                }
                Event::DestroyNotify(e) => {
                    wm.state.remove_window_everywhere(e.window);

                    let intent = retile(wm);
                    map_intent(wm, intent);
                }
                Event::EnterNotify(e) => {
                    if wm.ignore_enter {
                        wm.ignore_enter = false;
                    } else if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                        focus_window(wm, e.event);
                        wm.conn.flush().unwrap();
                    }
                }
                Event::ClientMessage(e) => {
                    dbg!("clientmessage: ", e);
                    let type_name = wm
                        .conn
                        .get_atom_name(e.type_)
                        .unwrap()
                        .reply()
                        .map(|r| String::from_utf8_lossy(&r.name).into_owned())
                        .unwrap_or_default();
                    let d = e.data.as_data32();
                    let a1 = wm
                        .conn
                        .get_atom_name(d[1])
                        .unwrap()
                        .reply()
                        .map(|r| String::from_utf8_lossy(&r.name).into_owned())
                        .unwrap_or_default();
                    let a2 = wm
                        .conn
                        .get_atom_name(d[2])
                        .unwrap()
                        .reply()
                        .map(|r| String::from_utf8_lossy(&r.name).into_owned())
                        .unwrap_or_default();
                    eprintln!(
                        "CLIENTMESSAGE: window={} type={} action={} prop1={} prop2={} source={}",
                        e.window, type_name, d[0], a1, a2, d[3]
                    );

                    let d = e.data.as_data32();
                    let action = d[0];

                    if d[1] == wm.atoms._net_wm_state_fullscreen
                        || d[2] == wm.atoms._net_wm_state_fullscreen
                    {
                        let is_fullscreen = wm.state.fullscreen_windows().contains(&e.window);
                        let should_be_fullscreen = match action {
                            0 => false,
                            1 => true,
                            2 => !is_fullscreen,
                            _ => is_fullscreen,
                        };

                        dbg!(should_be_fullscreen, is_fullscreen);

                        if should_be_fullscreen != is_fullscreen {
                            set_fullscreen(wm, e.window, should_be_fullscreen);
                        }
                    }
                }
                Event::PropertyNotify(e) => {
                    if e.atom == wm.atoms._net_wm_strut_partial || e.atom == wm.atoms._net_wm_strut
                    {
                        match read_struts(wm, e.window) {
                            Some(s) => {
                                wm.state.active_struts.insert(e.window, s);
                            }
                            None => {
                                wm.state.active_struts.remove(&e.window);
                            }
                        }

                        let intent = retile(wm);
                        map_intent(wm, intent);
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
