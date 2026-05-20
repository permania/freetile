use x11rb::{
    connection::Connection,
    protocol::xproto::{ConnectionExt, GrabMode, KeyPressEvent, ModMask, Setup},
    rust_connection::RustConnection,
};

use crate::{GrabbedKeys, Keybind, Mods};

pub struct KipkeyContext<'a> {
    pub conn: &'a RustConnection,
    pub screen_root: u32,
    pub setup: &'a Setup,
    pub first_keycode: u8,
    pub syms_per_keycode: u8,
    pub keysyms: Vec<u32>,
}

pub fn event_to_keybind(ctx: &KipkeyContext, e: KeyPressEvent) -> Keybind {
    let mask = e.state;
    let clean_mask: u16 = !(u16::from(ModMask::M2) | u16::from(ModMask::LOCK));

    let keycode = e.detail;
    let idx = (keycode - ctx.first_keycode) as usize * ctx.syms_per_keycode as usize;

    let shifted = u16::from(e.state) & u16::from(ModMask::SHIFT) != 0;
    let sym = if shifted { ctx.keysyms[idx + 1] } else { ctx.keysyms[idx] };

    Keybind {
        keysym: sym.into(),
        mods: Mods::from(mask & clean_mask),
    }
}

pub fn setup_server<'a>(conn: &'a RustConnection, screen_num: usize) -> KipkeyContext<'a> {
    let setup = conn.setup();
    let screen = &setup.roots[screen_num];
    let root = screen.root;
    let first_keycode = setup.min_keycode;
    let count = setup.max_keycode - setup.min_keycode + 1;
    let kb_map = conn
        .get_keyboard_mapping(first_keycode, count)
        .unwrap()
        .reply()
        .unwrap();

    let syms_per_keycode = kb_map.keysyms_per_keycode;
    let keysyms = kb_map.keysyms;

    KipkeyContext {
        conn: &conn,
        screen_root: root,
        setup,
        first_keycode,
        syms_per_keycode,
        keysyms,
    }
}

pub fn grab_keys(ctx: &KipkeyContext, binds: Vec<&Keybind>) -> GrabbedKeys {
    let mut grabbed: GrabbedKeys = GrabbedKeys::new();

    for bind in binds.iter() {
        let sym = bind.keysym;
        let mods = ModMask::from(bind.mods.to_modmask());

        for keycode in ctx.first_keycode..=ctx.setup.max_keycode {
            let idx = (keycode - ctx.first_keycode) as usize * ctx.syms_per_keycode as usize;
            if ctx.keysyms.get(idx).copied().unwrap_or(0) == sym.raw() {
                ctx.conn
                    .grab_key(
                        true,
                        ctx.screen_root,
                        mods,
                        keycode,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                    )
                    .unwrap()
                    .check()
                    .unwrap();

                grabbed.push((keycode, mods.into()));

                break;
            } else if ctx.keysyms.get(idx + 1).copied().unwrap_or(0) == sym.raw() {
                ctx.conn
                    .grab_key(
                        true,
                        ctx.screen_root,
                        mods | ModMask::SHIFT,
                        keycode,
                        GrabMode::ASYNC,
                        GrabMode::ASYNC,
                    )
                    .unwrap()
                    .check()
                    .unwrap();

                grabbed.push((keycode, mods.into()));
                break;
            }
        }
    }

    ctx.conn.flush().unwrap();
    grabbed
}

pub fn ungrab_keys(conn: &RustConnection, root: u32, grabbed: &GrabbedKeys) {
    for (code, mods) in grabbed {
        conn.ungrab_key(*code, root, *mods).unwrap();
    }
    conn.flush().unwrap();
}
