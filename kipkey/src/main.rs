use std::ops::Deref;

use x11rb::{
    connection::Connection,
    protocol::xproto::{ConnectionExt, GrabMode, KeyButMask, KeyPressEvent, ModMask},
};

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

fn normalize_keysym(sym: u32) -> u32 {
    if (0x41..=0x5A).contains(&sym) {
        sym + 32
    } else {
        sym
    }
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

    let kb_map = wm
        .conn
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
                wm.conn
                    .grab_key(
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

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::xproto::{KeyButMask, ModMask};

    // --- normalize_keysym ---

    #[test]
    fn normalize_uppercase_a_to_lowercase() {
        assert_eq!(normalize_keysym(0x41), 0x61); // A -> a
    }

    #[test]
    fn normalize_uppercase_z_to_lowercase() {
        assert_eq!(normalize_keysym(0x5A), 0x7A); // Z -> z
    }

    #[test]
    fn normalize_lowercase_is_unchanged() {
        assert_eq!(normalize_keysym(0x61), 0x61); // a stays a
        assert_eq!(normalize_keysym(0x7A), 0x7A); // z stays z
    }

    #[test]
    fn normalize_non_alpha_is_unchanged() {
        assert_eq!(normalize_keysym(0x30), 0x30); // '0'
        assert_eq!(normalize_keysym(0xFF1B), 0xFF1B); // XK_Escape
        assert_eq!(normalize_keysym(0x20), 0x20); // space
    }

    #[test]
    fn normalize_boundary_below_a_is_unchanged() {
        assert_eq!(normalize_keysym(0x40), 0x40); // '@', just below 'A'
    }

    #[test]
    fn normalize_boundary_above_z_is_unchanged() {
        assert_eq!(normalize_keysym(0x5B), 0x5B); // '[', just above 'Z'
    }

    // --- KeyBind::matches ---

    fn make_bind(sym: u32, mods: u16) -> KeyBind {
        KeyBind {
            res: KeyResult {
                sym,
                mods: KeyButMask::from(mods),
            },
            action: WMAction::Kill,
        }
    }

    fn make_result(sym: u32, mods: u16) -> KeyResult {
        KeyResult {
            sym,
            mods: KeyButMask::from(mods),
        }
    }

    #[test]
    fn keybind_matches_exact() {
        let bind = make_bind(0x61, u16::from(ModMask::M4));
        let res = make_result(0x61, u16::from(ModMask::M4));
        assert!(bind.matches(&res));
    }

    #[test]
    fn keybind_matches_case_insensitive_sym() {
        // bind registered with lowercase, event fires uppercase (or vice versa)
        let bind = make_bind(0x61, u16::from(ModMask::M4)); // 'a'
        let res = make_result(0x41, u16::from(ModMask::M4)); // 'A'
        assert!(bind.matches(&res));
    }

    #[test]
    fn keybind_no_match_wrong_sym() {
        let bind = make_bind(0x61, u16::from(ModMask::M4));
        let res = make_result(0x62, u16::from(ModMask::M4)); // 'b'
        assert!(!bind.matches(&res));
    }

    #[test]
    fn keybind_no_match_wrong_mods() {
        let bind = make_bind(0x61, u16::from(ModMask::M4));
        let res = make_result(0x61, u16::from(ModMask::M4) | u16::from(ModMask::SHIFT));
        assert!(!bind.matches(&res));
    }

    #[test]
    fn keybind_no_match_no_mods() {
        let bind = make_bind(0x61, u16::from(ModMask::M4));
        let res = make_result(0x61, 0);
        assert!(!bind.matches(&res));
    }

    #[test]
    fn keybind_matches_with_shift() {
        let mods = u16::from(ModMask::M4) | u16::from(ModMask::SHIFT);
        let bind = make_bind(0x6A, mods); // Mod+Shift+j
        let res = make_result(0x6A, mods);
        assert!(bind.matches(&res));
    }

    // --- keysym_from_keycode ---

    #[test]
    fn keysym_from_keycode_returns_correct_element() {
        let syms = vec![0x61u32, 0x62, 0x63, 0x64];
        assert_eq!(keysym_from_keycode(0, &syms), 0x61);
        assert_eq!(keysym_from_keycode(2, &syms), 0x63);
    }
}

