use std::ops::Deref;

use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ConnectionExt, GrabMode, KeyButMask, KeyPressEvent, ModMask
    },
};

use super::wm::{WM, WMAction};

#[derive(Debug, Clone)]
pub struct KeyResult {
    pub mods: KeyButMask,
    pub sym: u32,
}

#[derive(Debug, Clone)]
pub struct KeyBind {
    pub action: WMAction,
    res: KeyResult,
}

impl Deref for KeyBind {
    type Target = KeyResult;
    fn deref(&self) -> &Self::Target {
        &self.res
    }
}

impl KeyBind {
    pub fn matches(&self, res: &KeyResult) -> bool {
        normalize_keysym(self.res.sym) == normalize_keysym(res.sym) && self.res.mods == res.mods
    }
}

macro_rules! tag_keybind {
    ($key:ident, $tag:expr) => {
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::$key,
                mods: KeyButMask::from(u16::from(ModMask::M4)),
            },
            action: WMAction::TagSwitch($tag),
        }
    };
}

macro_rules! tag_keybind_window {
    ($key:ident, $tag:expr) => {
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::$key,
                mods: KeyButMask::from(u16::from(ModMask::M4) | u16::from(ModMask::SHIFT)),
            },
            action: WMAction::TagWindowSwitch($tag),
        }
    };
}

fn normalize_keysym(sym: u32) -> u32 {
    if (0x41..=0x5A).contains(&sym) {
        sym + 32
    } else {
        sym
    }
}

pub fn register_keybinds() -> Vec<KeyBind> {
    let keybinds: Vec<KeyBind> = vec![
        tag_keybind!(XK_a, 0),
        tag_keybind!(XK_s, 1),
        tag_keybind!(XK_d, 2),
        tag_keybind!(XK_f, 3),
        tag_keybind!(XK_u, 4),
        tag_keybind!(XK_i, 5),
        tag_keybind!(XK_o, 6),
        tag_keybind!(XK_p, 7),
        tag_keybind_window!(XK_a, 0),
        tag_keybind_window!(XK_s, 1),
        tag_keybind_window!(XK_d, 2),
        tag_keybind_window!(XK_f, 3),
        tag_keybind_window!(XK_u, 4),
        tag_keybind_window!(XK_i, 5),
        tag_keybind_window!(XK_o, 6),
        tag_keybind_window!(XK_p, 7),
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
                mods: KeyButMask::from(u16::from(ModMask::M4) | u16::from(ModMask::SHIFT)),
            },
            action: WMAction::SwapNext,
        },
        KeyBind {
            res: KeyResult {
                sym: x11_keysyms::XK_k,
                mods: KeyButMask::from(u16::from(ModMask::M4) | u16::from(ModMask::SHIFT)),
            },
            action: WMAction::SwapPrevious,
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

pub fn event_to_keyresult(wm: &mut WM, e: KeyPressEvent) -> KeyResult {
    let keycode = e.detail;
    let state = e.state;
    let idx = (keycode - wm.state.first_keycode) as usize * wm.state.syms_per_keycode as usize;
    let sym = keysym_from_keycode(idx, &wm.state.keysyms);

    let clean_mask: u16 = !(u16::from(ModMask::M2) | u16::from(ModMask::LOCK));
    KeyResult {
        sym,
        mods: KeyButMask::from(u16::from(state) & clean_mask),
    }
}

pub fn grab_keys(wm: &mut WM) {
    let setup = wm.conn.setup();

    let first_keycode = setup.min_keycode;
    let count = setup.max_keycode - setup.min_keycode + 1;

    let kb_map = wm.conn
        .get_keyboard_mapping(first_keycode, count)
        .unwrap()
        .reply()
        .unwrap();
    let syms_per_keycode = kb_map.keysyms_per_keycode;
    let keysyms = kb_map.keysyms;

    for bind in wm.keybinds.iter() {
        let sym = bind.res.sym;
        for keycode in first_keycode..=setup.max_keycode {
            let idx = (keycode - first_keycode) as usize * syms_per_keycode as usize;
            if keysyms.get(idx).copied().unwrap_or(0) == sym {
                wm.conn.grab_key(
                    true,
                    wm.screen.root,
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

pub fn keysym_from_keycode(idx: usize, syms: &[u32]) -> u32 {
    syms[idx]
}

mod chords {
    use bitflags::bitflags;

    bitflags! {
        struct Mods: u8 {
        const CTRL	= 0b0001;
        const ALT	= 0b0010;
        const SHIFT	= 0b0100;
        const SUPER	= 0b1000;
        }
    }
}
