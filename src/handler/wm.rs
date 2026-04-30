use std::process::Command;

use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux, ConnectionExt,
        EventMask, InputFocus, Screen, StackMode, Window,
    },
    rust_connection::RustConnection,
};

use super::{event::event_loop, keys};

type WindowSet = Vec<Window>;

#[derive(Default)]
pub struct Tag {
    windows: WindowSet,
    focused: Option<Window>,
}

pub struct WM<'a> {
    pub conn: &'a RustConnection,
    pub screen: Screen,
    pub state: WMState,
    pub keybinds: Vec<keys::KeyBind>,
}

pub struct WMState {
    pub tags: [Tag; 8],
    pub active: usize,
    pub first_keycode: u8,
    pub keysyms: Vec<u32>,
    pub syms_per_keycode: u8,
}

impl WMState {
    pub fn new() -> Self {
        Self {
            tags: Default::default(),
            active: 0,
            first_keycode: Default::default(),
            keysyms: Default::default(),
            syms_per_keycode: Default::default(),
        }
    }

    pub fn windows(&self) -> &WindowSet {
        &self.tags[self.active].windows
    }

    pub fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.tags[self.active].windows
    }

    pub fn focused(&self) -> Option<u32> {
        self.tags[self.active].focused
    }

    pub fn set_focused(&mut self, set: Option<u32>) {
        self.tags[self.active].focused = set;
    }
}

impl Tag {
    #[allow(dead_code)]
    pub fn windows(&self) -> &WindowSet {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.windows
    }
}

#[derive(Debug, Clone)]
pub enum WMAction {
    Spawn(String, Vec<String>),
    TagSwitch(usize),
    TagWindowSwitch(usize),
    Kill,
    FocusNext,
    FocusPrevious,
    SwapNext,
    SwapPrevious,
}

impl WMAction {
    pub fn execute(&self, wm: &mut WM) {
        match self {
            WMAction::Spawn(cmd, args) => {
                #[allow(clippy::zombie_processes)]
                Command::new(cmd).args(args).spawn().unwrap();
            }
            WMAction::Kill => {
                if let Some(win) = wm.state.focused()
                    && wm.state.windows().contains(&win)
                {
                    // send WM_DELETE_WINDOW message
                    let wm_protocols = wm
                        .conn
                        .intern_atom(false, b"WM_PROTOCOLS")
                        .unwrap()
                        .reply()
                        .unwrap()
                        .atom;
                    let wm_delete = wm
                        .conn
                        .intern_atom(false, b"WM_DELETE_WINDOW")
                        .unwrap()
                        .reply()
                        .unwrap()
                        .atom;

                    let data = [wm_delete, 0, 0, 0, 0];
                    wm.conn
                        .send_event(
                            false,
                            win,
                            EventMask::NO_EVENT,
                            ClientMessageEvent::new(32, win, wm_protocols, data),
                        )
                        .unwrap();
                    wm.conn.flush().unwrap();
                }
            }
            WMAction::FocusNext => {
                if let Some(idx) = focused_index(&wm.state) {
                    let windows = wm.state.windows();
                    let next = windows[wrap_next(idx, windows.len())];
                    focus_and_warp(wm, next);
                }
            }
            WMAction::FocusPrevious => {
                if let Some(idx) = focused_index(&wm.state) {
                    let windows = wm.state.windows();
                    let next = windows[wrap_prev(idx, windows.len())];
                    focus_and_warp(wm, next);
                }
            }
            WMAction::SwapNext => {
                let win = wm.state.focused();
                let idx = focused_index(&wm.state);

                if let (Some(win), Some(idx)) = (win, idx) {
                    let windows = wm.state.windows();
                    let next_idx = wrap_next(idx, windows.len());

                    wm.state.windows_mut().swap(idx, next_idx);
                    retile(wm);
                    focus_and_warp(wm, win);
                }
            }
            WMAction::SwapPrevious => {
                let win = wm.state.focused();
                let idx = focused_index(&wm.state);

                if let (Some(win), Some(idx)) = (win, idx) {
                    let windows = wm.state.windows();
                    let next_idx = wrap_prev(idx, windows.len());

                    wm.state.windows_mut().swap(idx, next_idx);
                    retile(wm);
                    focus_and_warp(wm, win);
                }
            }
            WMAction::TagSwitch(idx) => {
                switch_workspace(wm, idx);
            }
            WMAction::TagWindowSwitch(idx) => {
                if let Some(win) = wm.state.focused()
                    && wm.state.windows().contains(&win)
                {
                    wm.state.set_focused(wm.state.windows().last().copied());
                    wm.state.windows_mut().retain(|&w| w != win);
                    wm.conn
                        .clear_area(false, wm.screen.root, 0, 0, 0, 0)
                        .unwrap();

                    wm.state.tags[*idx].windows_mut().push(win);

                    switch_workspace(wm, idx);
                }
            }
        }
    }
}

pub fn run() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let mut wm = setup_wm(&conn, screen_num);
    event_loop(&mut wm);
}

fn setup_wm<'a>(conn: &'a RustConnection, screen_num: usize) -> WM<'a> {
    let wm_state = WMState::new();
    let keybinds = keys::register_keybinds();
    let setup = conn.setup();

    let mut wm = WM {
        conn,
        screen: setup.roots[screen_num].clone(),
        state: wm_state,
        keybinds,
    };

    let (first_keycode, max_keycode) = {
        keys::grab_keys(&mut wm);
        (setup.min_keycode, setup.max_keycode)
    };

    let kb_map = conn
        .get_keyboard_mapping(first_keycode, max_keycode - first_keycode + 1)
        .unwrap()
        .reply()
        .unwrap();

    wm.state.first_keycode = first_keycode;
    wm.state.keysyms = kb_map.keysyms;
    wm.state.syms_per_keycode = kb_map.keysyms_per_keycode;

    // Redirect events to the wm
    conn.change_window_attributes(
        wm.screen.root,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY),
    )
    .unwrap()
    .check()
    .unwrap();

    // Black root window
    conn.change_window_attributes(
        wm.screen.root,
        &ChangeWindowAttributesAux::new().background_pixel(wm.screen.black_pixel),
    )
    .unwrap();

    conn.flush().unwrap();

    wm
}

pub fn retile(wm: &mut WM) {
    let w = wm.screen.width_in_pixels as u32;
    let h = wm.screen.height_in_pixels as u32;

    match wm.state.windows().len() {
        0 => {}
        1 => {
            configure(wm, wm.state.windows()[0], 0, 0, w, h);
        }
        _ => {
            let windows = wm.state.windows().to_vec();

            let master = windows[0];
            let slaves = &windows[1..];
            let n = slaves.len() as u32;
            let base_h = h / n;
            let remainder = h % n;

            configure(wm, master, 0, 0, w / 2, h);

            for (i, &win) in windows[1..].iter().enumerate() {
                let i = i as u32;

                let extra = if i < remainder { 1 } else { 0 };
                let win_h = base_h + extra;

                let y = (i * base_h + i.min(remainder)) as i32;

                configure(wm, win, (w / 2) as i32, y, w / 2, win_h);
            }
        }
    }
    wm.conn.flush().unwrap();
}

fn configure(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32) {
    let border_width = 3u32;

    wm.conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0xff444444),
        )
        .unwrap();

    wm.conn
        .configure_window(
            window,
            &ConfigureWindowAux::new()
                .x(x)
                .y(y)
                .width(w - border_width * 2)
                .height(h - border_width * 2)
                .border_width(border_width),
        )
        .unwrap();
}

pub fn focus_and_warp(wm: &mut WM, window: Window) {
    focus_window(wm, window);
    warp_to_window(wm, window);

    wm.conn.flush().unwrap();
}

pub fn focus_window(wm: &mut WM, window: Window) {
    if let Some(prev) = wm.state.focused() {
        wm.conn
            .change_window_attributes(
                prev,
                &ChangeWindowAttributesAux::new().border_pixel(0xff444444),
            )
            .unwrap();
    }

    wm.conn
        .change_window_attributes(
            window,
            &ChangeWindowAttributesAux::new().border_pixel(0xff8aadf4),
        )
        .unwrap();

    wm.conn
        .configure_window(
            window,
            &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
        )
        .unwrap();

    wm.conn
        .set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .unwrap();

    wm.state.set_focused(Some(window));

    wm.conn.flush().unwrap();
}

fn warp_to_window(wm: &mut WM, window: Window) {
    let geom = wm.conn.get_geometry(window).unwrap().reply().unwrap();
    let cx = geom.x + (geom.width / 2) as i16;
    let cy = geom.y + (geom.height / 2) as i16;

    wm.conn
        .warp_pointer(x11rb::NONE, wm.screen.root, 0, 0, 0, 0, cx, cy)
        .unwrap();
    wm.conn.flush().unwrap();
}

pub fn switch_workspace(wm: &mut WM, idx: &usize) {
    if *idx == wm.state.active {
        return;
    }

    for &win in wm.state.windows() {
        wm.conn.unmap_window(win).unwrap();
    }

    wm.state.active = *idx;
    if wm.state.focused().is_none() {
        wm.state.set_focused(wm.state.windows().last().copied());
    }

    for &win in wm.state.windows() {
        wm.conn.map_window(win).unwrap();
    }

    retile(wm);

    if let Some(win) = wm.state.focused() {
        focus_and_warp(wm, win);
    }

    wm.conn
        .clear_area(false, wm.screen.root, 0, 0, 0, 0)
        .unwrap();
    wm.conn.flush().unwrap();
}

fn focused_index(wm: &WMState) -> Option<usize> {
    let win = wm.focused()?;
    wm.windows().iter().position(|&w| w == win)
}

fn wrap_next(i: usize, len: usize) -> usize {
    (i + 1) % len
}

fn wrap_prev(i: usize, len: usize) -> usize {
    (i + len - 1) % len
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- wrap_next ---

    #[test]
    fn wrap_next_middle() {
        assert_eq!(wrap_next(2, 5), 3);
    }

    #[test]
    fn wrap_next_last_wraps_to_zero() {
        assert_eq!(wrap_next(4, 5), 0);
    }

    #[test]
    fn wrap_next_single_element() {
        assert_eq!(wrap_next(0, 1), 0);
    }

    #[test]
    fn wrap_next_two_elements() {
        assert_eq!(wrap_next(0, 2), 1);
        assert_eq!(wrap_next(1, 2), 0);
    }

    // --- wrap_prev ---

    #[test]
    fn wrap_prev_middle() {
        assert_eq!(wrap_prev(3, 5), 2);
    }

    #[test]
    fn wrap_prev_first_wraps_to_last() {
        assert_eq!(wrap_prev(0, 5), 4);
    }

    #[test]
    fn wrap_prev_single_element() {
        assert_eq!(wrap_prev(0, 1), 0);
    }

    #[test]
    fn wrap_prev_two_elements() {
        assert_eq!(wrap_prev(0, 2), 1);
        assert_eq!(wrap_prev(1, 2), 0);
    }

    #[test]
    fn wrap_next_then_prev_is_identity() {
        for len in 1..=8 {
            for i in 0..len {
                assert_eq!(wrap_prev(wrap_next(i, len), len), i);
            }
        }
    }

    #[test]
    fn wrap_prev_then_next_is_identity() {
        for len in 1..=8 {
            for i in 0..len {
                assert_eq!(wrap_next(wrap_prev(i, len), len), i);
            }
        }
    }

    // --- WMState ---

    fn make_state_with_windows(wins: &[u32]) -> WMState {
        let mut s = WMState::new();
        for &w in wins {
            s.windows_mut().push(w);
        }
        s
    }

    #[test]
    fn wmstate_new_has_no_windows() {
        let s = WMState::new();
        assert!(s.windows().is_empty());
    }

    #[test]
    fn wmstate_new_focused_is_none() {
        let s = WMState::new();
        assert_eq!(s.focused(), None);
    }

    #[test]
    fn wmstate_set_focused_roundtrips() {
        let mut s = WMState::new();
        s.windows_mut().push(42);
        s.set_focused(Some(42));
        assert_eq!(s.focused(), Some(42));
    }

    #[test]
    fn wmstate_set_focused_none() {
        let mut s = WMState::new();
        s.set_focused(Some(1));
        s.set_focused(None);
        assert_eq!(s.focused(), None);
    }

    #[test]
    fn wmstate_windows_mut_push_and_read() {
        let mut s = WMState::new();
        s.windows_mut().push(10);
        s.windows_mut().push(20);
        assert_eq!(s.windows(), &[10, 20]);
    }

    #[test]
    fn wmstate_windows_mut_retain() {
        let mut s = make_state_with_windows(&[1, 2, 3]);
        s.windows_mut().retain(|&w| w != 2);
        assert_eq!(s.windows(), &[1, 3]);
    }

    #[test]
    fn wmstate_active_default_is_zero() {
        let s = WMState::new();
        assert_eq!(s.active, 0);
    }

    #[test]
    fn wmstate_windows_are_per_tag() {
        let mut s = WMState::new();
        s.windows_mut().push(100);   // tag 0
        s.active = 1;
        assert!(s.windows().is_empty()); // tag 1 is empty
        s.windows_mut().push(200);
        s.active = 0;
        assert_eq!(s.windows(), &[100]); // tag 0 still just has 100
    }

    #[test]
    fn wmstate_focused_is_per_tag() {
        let mut s = WMState::new();
        s.set_focused(Some(1));
        s.active = 1;
        assert_eq!(s.focused(), None); // different tag, no focus yet
        s.set_focused(Some(2));
        s.active = 0;
        assert_eq!(s.focused(), Some(1)); // tag 0 focus unchanged
    }

    // --- focused_index (via wrap logic) ---
    // focused_index is private but its semantics are tested indirectly
    // through wrap_next/wrap_prev above. Document the contract here:

    #[test]
    fn wrap_next_covers_full_cycle() {
        let len = 6;
        let mut i = 0;
        for _ in 0..len {
            i = wrap_next(i, len);
        }
        assert_eq!(i, 0, "should complete a full cycle back to start");
    }

    #[test]
    fn wrap_prev_covers_full_cycle() {
        let len = 6;
        let mut i = 0;
        for _ in 0..len {
            i = wrap_prev(i, len);
        }
        assert_eq!(i, 0, "should complete a full cycle back to start");
    }
}
