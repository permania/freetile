use std::ops::Deref;

use x11rb::{
    connection::Connection, protocol::xproto::{ConnectionExt, GrabMode, KeyButMask, ModMask, Screen, Setup}, rust_connection::RustConnection
};

use super::wm::WMAction;

#[derive(Debug)]
pub struct KeyResult {
    pub sym: u32,
    pub mods: KeyButMask,
}

#[derive(Debug)]
pub struct KeyBind {
    res: KeyResult,
    pub action: WMAction,
}

impl Deref for KeyBind {
    type Target = KeyResult;
    fn deref(&self) -> &Self::Target {
        &self.res
    }
}

impl KeyBind {
    pub fn matches(&self, res: &KeyResult) -> bool {
	dbg!(normalize_keysym(self.res.sym));
	dbg!(normalize_keysym(res.sym));
        normalize_keysym(self.res.sym) == normalize_keysym(res.sym) && self.res.mods == res.mods
    }
}

fn normalize_keysym(sym: u32) -> u32 {
    if sym >= 0x41 && sym <= 0x5A {
        sym + 32
    } else {
        sym
    }
}

pub fn register_keybinds() -> Vec<KeyBind> {
    let keybinds: Vec<KeyBind> = vec![
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_n,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::Spawn("alacritty".to_string(), vec![]),
        },
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_j,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::FocusNext,
        },
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_k,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::FocusPrevious,
        },
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_m,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::Kill,
        },
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_c,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::Spawn(
                "rofi".to_string(),
                vec!["-show".to_string(), "drun".to_string()],
            ),
        },
    ];

    keybinds
}

pub fn grab_keys(conn: &impl Connection, keybinds: &Vec<KeyBind>, setup: &Setup, screen: &Screen) {
    let first_keycode = setup.min_keycode;
    let count = setup.max_keycode - setup.min_keycode + 1;

    let kb_map = conn
        .get_keyboard_mapping(first_keycode, count)
        .unwrap()
        .reply()
        .unwrap();
    let syms_per_keycode = kb_map.keysyms_per_keycode;
    let keysyms = kb_map.keysyms;

    for bind in keybinds {
        let sym = bind.res.sym;
        for keycode in first_keycode..=setup.max_keycode {
            let idx = (keycode - first_keycode) as usize * syms_per_keycode as usize;
            if keysyms.get(idx).copied().unwrap_or(0) == sym {
                conn.grab_key(
                    true,
                    screen.root,
                    u16::from(bind.res.mods).into(),
                    keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .unwrap()
                .check()
                .unwrap();
                break;
            }
        }
    }
}

pub fn keysym_from_keycode(idx: usize, syms: &Vec<u32>) -> u32 {
    syms[idx]
}
