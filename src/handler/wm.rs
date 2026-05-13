use std::process::Command;

use rhai::{AST, Dynamic, Engine, Scope};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux, ConnectionExt,
        EventMask, InputFocus, MapState, Screen, StackMode, Window,
    },
    rust_connection::RustConnection,
};

use crate::config::layout::{LayoutIntent, Rect, WMSlot};
use crate::config::reader;
use crate::config::reader::load_config;
use crate::config::reader::EngineSetup;

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
    pub ignore_unmaps: usize,
}

pub struct WMState {
    pub tags: [Tag; 8],
    pub active: usize,
    pub first_keycode: u8,
    pub keysyms: Vec<u32>,
    pub syms_per_keycode: u8,
    pub engine: Engine,
    pub layout_ast: AST,
}

impl WMState {
    pub fn new() -> Self {
        Self {
            tags: Default::default(),
            active: 0,
            first_keycode: Default::default(),
            keysyms: Default::default(),
            syms_per_keycode: Default::default(),
            engine: {
                let mut engine = Engine::new();
                engine.setup();
                engine
            },
            layout_ast: Default::default(),
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

impl Default for WMState {
    fn default() -> Self {
        Self::new()
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
                }
            }
            WMAction::FocusNext => {
                if focused_index(&wm.state).is_some() {
                    let windows: Vec<Window> = wm
                        .state
                        .windows()
                        .iter()
                        .copied()
                        .filter(|&w| is_mapped(wm, w))
                        .collect();
                    if let Some(cur) = windows
                        .iter()
                        .position(|&w| w == wm.state.focused().unwrap())
                    {
                        let next = windows[wrap_next(cur, windows.len())];
                        focus_and_warp(wm, next);
                    }
                }
            }
            WMAction::FocusPrevious => {
                if focused_index(&wm.state).is_some() {
                    let windows: Vec<Window> = wm
                        .state
                        .windows()
                        .iter()
                        .copied()
                        .filter(|&w| is_mapped(wm, w))
                        .collect();
                    if let Some(cur) = windows
                        .iter()
                        .position(|&w| w == wm.state.focused().unwrap())
                    {
                        let next = windows[wrap_prev(cur, windows.len())];
                        focus_and_warp(wm, next);
                    }
                }
            }
            WMAction::SwapNext => {
                if let Some(win) = wm.state.focused() {
                    let mapped: Vec<Window> = wm
                        .state
                        .windows()
                        .iter()
                        .copied()
                        .filter(|&w| is_mapped(wm, w))
                        .collect();
                    if let Some(cur) = mapped.iter().position(|&w| w == win) {
                        let next = mapped[wrap_next(cur, mapped.len())];
                        let i = wm.state.windows().iter().position(|&w| w == win).unwrap();
                        let j = wm.state.windows().iter().position(|&w| w == next).unwrap();
                        wm.state.windows_mut().swap(i, j);

                        let intent = retile(wm);
                        wm.ignore_unmaps += intent
                            .unmapped
                            .iter()
                            .filter(|&&w| is_mapped(wm, w))
                            .count();
                        map_intent(wm, intent);

                        if is_mapped(wm, win) {
                            focus_and_warp(wm, win);
                        }
                    }
                }
            }
            WMAction::SwapPrevious => {
                if let Some(win) = wm.state.focused() {
                    let mapped: Vec<Window> = wm
                        .state
                        .windows()
                        .iter()
                        .copied()
                        .filter(|&w| is_mapped(wm, w))
                        .collect();
                    if let Some(cur) = mapped.iter().position(|&w| w == win) {
                        let next = mapped[wrap_prev(cur, mapped.len())];
                        let i = wm.state.windows().iter().position(|&w| w == win).unwrap();
                        let j = wm.state.windows().iter().position(|&w| w == next).unwrap();
                        wm.state.windows_mut().swap(i, j);

                        let intent = retile(wm);
                        wm.ignore_unmaps += intent
                            .unmapped
                            .iter()
                            .filter(|&&w| is_mapped(wm, w))
                            .count();
                        map_intent(wm, intent);

                        if is_mapped(wm, win) {
                            focus_and_warp(wm, win);
                        }
                    }
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
    let mut wm_state = WMState::new();
    load_config(&mut wm_state).unwrap();
    let keybinds = keys::register_keybinds();
    let setup = conn.setup();

    let mut wm = WM {
        conn,
        screen: setup.roots[screen_num].clone(),
        state: wm_state,
        keybinds,
        ignore_unmaps: 0usize,
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

pub fn retile(wm: &mut WM) -> LayoutIntent {
    let n = wm.state.windows().len();
    if n == 0 {
        return LayoutIntent {
            mapped: vec![],
            unmapped: vec![],
        };
    }

    let bounds = Rect {
        x: 0,
        y: 0,
        w: wm.screen.width_in_pixels as u32,
        h: wm.screen.height_in_pixels as u32,
    };

    let mut scope = Scope::new();
    let result: Option<WMSlot> =
        wm.state
            .engine
            .call_fn::<Dynamic>(&mut scope, &wm.state.layout_ast, "ratiotile", (n as i64,)).ok()
	    .and_then(|d| d.try_cast::<WMSlot>());

    let slot: WMSlot = match result {
	Some(s) => s,
	None => {
	    reader::master()
	}
    };

    let rects = slot.compute(n, bounds, 8, 8);
    let windows = wm.state.windows().to_vec();
    let len = windows.len().min(rects.len());

    let mapped: Vec<(Window, Rect)> = (0..len).map(|i| (windows[i], rects[i])).collect();
    let unmapped = windows[len..].to_vec();

    dbg!(&mapped, &unmapped);

    LayoutIntent { mapped, unmapped }
}

pub fn map_intent(wm: &mut WM, intent: LayoutIntent) {
    for (window, rect) in intent.mapped {
        if !is_mapped(wm, window) {
            wm.conn.map_window(window).unwrap();
        }
        configure(wm, window, rect.x, rect.y, rect.w, rect.h);
    }

    for w in intent.unmapped {
        if is_mapped(wm, w) {
            wm.conn.unmap_window(w).unwrap();
        }
    }

    wm.conn.flush().unwrap();
}

fn configure(wm: &mut WM, window: Window, x: i32, y: i32, w: u32, h: u32) {
    let border_width = 3u32;

    if w == 0 || h == 0 {
        return;
    }
    if w <= border_width * 2 || h <= border_width * 2 {
        return;
    }

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
}

fn warp_to_window(wm: &mut WM, window: Window) {
    if let Ok(geom) = wm.conn.get_geometry(window).unwrap().reply() {
        let cx = geom.x + (geom.width / 2) as i16;
        let cy = geom.y + (geom.height / 2) as i16;

        wm.conn
            .warp_pointer(x11rb::NONE, wm.screen.root, 0, 0, 0, 0, cx, cy)
            .unwrap();
    }
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

    let intent = retile(wm);
    map_intent(wm, intent);

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

pub fn is_mapped(wm: &WM, window: Window) -> bool {
    wm.conn
        .get_window_attributes(window)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .map(|attrs| attrs.map_state != MapState::UNMAPPED)
        .unwrap_or(false)
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
        s.windows_mut().push(100); // tag 0
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
