use x11rb::{connection::Connection, protocol::xproto::{Atom, AtomEnum, ConnectionExt, Window}};

// TODO: use these on wm startup
struct Atoms {
    wm_protocols: Atom,
    wm_delete_window: Atom,
    net_wm_window_type: Atom,
    net_wm_window_type_dock: Atom,
}

impl Atoms {
    fn new(conn: &impl Connection) -> Self {
        Self {
            wm_protocols: intern(conn, b"WM_PROTOCOLS"),
            wm_delete_window: intern(conn, b"WM_DELETE_WINDOW"),
            net_wm_window_type: intern(conn, b"_NET_WM_WINDOW_TYPE"),
            net_wm_window_type_dock: intern(conn, b"_NET_WM_WINDOW_TYPE_DOCK"),
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

pub fn intern(conn: &impl Connection, name: &[u8]) -> Atom {
    conn.intern_atom(false, name).unwrap().reply().unwrap().atom
}

