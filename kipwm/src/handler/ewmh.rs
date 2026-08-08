use x11rb::{
    connection::Connection,
    protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window},
};

pub struct Atoms {
    pub wm_protocols: Atom,
    pub wm_delete_window: Atom,
    pub net_wm_window_type: Atom,
    pub net_wm_window_type_dock: Atom,
    pub net_wm_state_fullscreen: Atom,
    pub net_wm_state: Atom,
}

impl Atoms {
    pub fn new(conn: &impl Connection) -> Self {
        Self {
            wm_protocols: intern(conn, b"WM_PROTOCOLS"),
            wm_delete_window: intern(conn, b"WM_DELETE_WINDOW"),
            net_wm_window_type: intern(conn, b"_NET_WM_WINDOW_TYPE"),
            net_wm_window_type_dock: intern(conn, b"_NET_WM_WINDOW_TYPE_DOCK"),
            net_wm_state_fullscreen: intern(conn, b"_NET_WM_STATE_FULLSCREEN"),
            net_wm_state: intern(conn, b"_NET_WM_STATE"),
        }
    }
}

pub fn atom_in_property(
    conn: &impl Connection,
    window: Window,
    property: Atom,
    target: Atom,
) -> bool {
    conn.get_property(false, window, property, AtomEnum::ATOM, 0, 32)
        .ok()
        .and_then(|c| c.reply().ok())
        .map(|reply| reply.value32().into_iter().flatten().any(|a| a == target))
        .unwrap_or(false)
}

fn intern(conn: &impl Connection, name: &[u8]) -> Atom {
    conn.intern_atom(false, name)
        .expect("failed to send InternAtom request")
        .reply()
        .expect("failed to get InternAtom reply")
        .atom
}
