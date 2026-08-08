use std::collections::HashSet;

use x11rb::{
    connection::Connection,
    protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt, EventMask},
    rust_connection::RustConnection,
};

use super::{WM, WMState};
use crate::{
    config::{ksn_reader::load_config, layout::reader::load_layout_config},
    handler::{event::event_loop, ewmh},
    ipc,
};

pub fn run() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let mut wm = setup_wm(&conn, screen_num);

    event_loop(&mut wm);
}

fn setup_wm_struct<'a>(conn: &'a RustConnection, screen_num: usize) -> WM<'a> {
    let mut wm_state = WMState::default();
    let config = load_config().unwrap();
    if let Ok((ast, layouts)) = load_layout_config(&mut wm_state, &config) {
        wm_state.layout_ast = ast;
        wm_state.active_layout = layouts
            .keys()
            .next()
            .cloned()
            .expect("no valid layouts found in layout config");

        let setup = conn.setup();

        dbg!(&layouts);

        WM {
            conn,
            screen: setup.roots[screen_num].clone(),
            state: wm_state,
            ignore_unmaps: HashSet::new(),
            ignore_enter: false,
            ipc_listener: ipc::open_socket(),
            layouts,
            config,
            atoms: ewmh::Atoms::new(conn),
        }
    } else {
        todo!()
    }
}

fn setup_x11(wm: &mut WM) {
    wm.conn
        .change_window_attributes(
            wm.screen.root,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY),
        )
        .unwrap()
        .check()
        .unwrap();

    wm.conn
        .change_window_attributes(
            wm.screen.root,
            &ChangeWindowAttributesAux::new().background_pixel(wm.screen.black_pixel),
        )
        .unwrap();

    wm.conn.flush().unwrap();
}

fn setup_wm<'a>(conn: &'a RustConnection, screen_num: usize) -> WM<'a> {
    let mut wm = setup_wm_struct(conn, screen_num);
    setup_x11(&mut wm);
    wm
}
