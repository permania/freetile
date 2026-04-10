use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt, EventMask, InputFocus,
        Screen, StackMode, Window,
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

    pub fn focused(&self) -> &Option<u32> {
        &self.tags[self.active].focused
    }

    pub fn set_focused(&mut self, set: Option<u32>) -> () {
        self.tags[self.active].focused = set;
    }
}

impl Tag {
    pub fn windows(&self) -> &WindowSet {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.windows
    }
}

#[derive(Debug)]
pub enum WMAction {
    Spawn(String, Vec<String>),
    TagSwitch(usize),
    Kill,
    FocusNext,
    FocusPrevious,
    SwapNext,
    SwapPrevious,
}

pub fn run() {
    let (mut wm_state, conn, screen, keybinds) = setup_wm();
    event_loop(&conn, &screen, &mut wm_state, keybinds);
}

fn setup_wm() -> (WMState, RustConnection, Screen, Vec<keys::KeyBind>) {
    let mut wm_state = WMState::new();
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let keybinds = keys::register_keybinds();

    let screen = {
        let setup = conn.setup();
        let screen = setup.roots[screen_num].clone();

        screen
    };

    let keybinds = keys::register_keybinds();
    let (first_keycode, max_keycode, screen) = {
        let setup = conn.setup();
        keys::grab_keys(&conn, &keybinds, &setup, &setup.roots[screen_num]);
        (
            setup.min_keycode,
            setup.max_keycode,
            setup.roots[screen_num].clone(),
        )
    };

    let kb_map = conn
        .get_keyboard_mapping(first_keycode, max_keycode - first_keycode + 1)
        .unwrap()
        .reply()
        .unwrap();

    wm_state.first_keycode = first_keycode;
    wm_state.keysyms = kb_map.keysyms;
    wm_state.syms_per_keycode = kb_map.keysyms_per_keycode;

    // Redirect events to the wm
    conn.change_window_attributes(
        screen.root,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY),
    )
    .unwrap()
    .check()
    .unwrap();

    // Black root window
    conn.change_window_attributes(
        screen.root,
        &ChangeWindowAttributesAux::new().background_pixel(screen.black_pixel),
    )
    .unwrap();

    conn.flush().unwrap();

    (wm_state, conn, screen.clone(), keybinds)
}

pub fn retile(conn: &impl Connection, screen: &Screen, state: &mut WMState) {
    let w = screen.width_in_pixels as u32;
    let h = screen.height_in_pixels as u32;

    match state.windows().len() {
        0 => {}
        1 => {
            configure(conn, state.windows()[0], 0, 0, w, h);
        }
        _ => {
            let master = state.windows()[0];
            let slaves = &state.windows()[1..];
            let slave_h = h / slaves.len() as u32;

            configure(conn, master, 0, 0, w / 2, h);
            for (i, &win) in slaves.iter().enumerate() {
                let y = (i as u32 * slave_h) as i32;
                let win_h = if i == slaves.len() - 1 {
                    h - (i as u32 * slave_h)
                } else {
                    slave_h
                };
                configure(conn, win, (w / 2) as i32, y, w / 2, win_h);
            }
        }
    }
    conn.flush().unwrap();
}

fn configure(conn: &impl Connection, window: Window, x: i32, y: i32, w: u32, h: u32) {
    let border_width = 3u32;

    conn.change_window_attributes(
        window,
        &ChangeWindowAttributesAux::new().border_pixel(0xff444444),
    )
    .unwrap();

    conn.configure_window(
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

pub fn focus_and_warp(
    conn: &impl Connection,
    screen: &Screen,
    window: Window,
    state: &mut WMState,
) {
    focus_window(conn, window, state);
    warp_to_window(conn, screen, window);

    conn.flush().unwrap();
}

pub fn focus_window(conn: &impl Connection, window: Window, state: &mut WMState) {
    if let &Some(prev) = state.focused() {
        conn.change_window_attributes(
            prev,
            &ChangeWindowAttributesAux::new().border_pixel(0xff444444),
        )
        .unwrap();
    }

    conn.change_window_attributes(
        window,
        &ChangeWindowAttributesAux::new().border_pixel(0xff8aadf4),
    )
    .unwrap();

    conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )
    .unwrap();
    conn.set_input_focus(InputFocus::PARENT, window, x11rb::CURRENT_TIME)
        .unwrap();

    state.set_focused(Some(window));

    conn.flush().unwrap();
}

fn warp_to_window(conn: &impl Connection, screen: &Screen, window: Window) {
    let geom = conn.get_geometry(window).unwrap().reply().unwrap();
    let cx = geom.x as i16 + (geom.width / 2) as i16;
    let cy = geom.y as i16 + (geom.height / 2) as i16;

    conn.warp_pointer(x11rb::NONE, screen.root, 0, 0, 0, 0, cx, cy)
        .unwrap();
    conn.flush().unwrap();
}

pub fn switch_workspace(conn: &impl Connection, screen: &Screen, state: &mut WMState, idx: &usize) {
    if *idx == state.active {
        return;
    }

    for &win in state.windows() {
        conn.unmap_window(win).unwrap();
    }

    state.active = *idx;
    state.set_focused(state.windows().last().copied());

    for &win in state.windows() {
        conn.map_window(win).unwrap();
    }

    retile(conn, screen, state);

    if let &Some(win) = state.focused() {
        focus_and_warp(conn, screen, win, state);
    }

    conn.clear_area(false, screen.root, 0, 0, 0, 0).unwrap();
    conn.flush().unwrap();
}
