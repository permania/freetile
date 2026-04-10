use std::process::Command;

use x11rb::connection::Connection;
use x11rb::protocol::Event;
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::protocol::xproto::*;

use std::ops::Deref;

#[derive(Debug)]
struct KeyResult {
    sym: u32,
    mods: KeyButMask,
}

#[derive(Debug)]
struct KeyBind {
    res: KeyResult,
    action: WMAction,
}

#[derive(Debug)]
enum WMAction {
    Spawn(String, Vec<String>),
    Kill,
    FocusNext,
    FocusPrevious,
}

impl Deref for KeyBind {
    type Target = KeyResult;
    fn deref(&self) -> &Self::Target {
        &self.res
    }
}

impl KeyBind {
    fn matches(&self, res: &KeyResult) -> bool {
        normalize_keysym(self.res.sym) == normalize_keysym(res.sym) && self.res.mods == res.mods
    }
}

fn warp_to_window(conn: &impl Connection, screen: &Screen, window: Window) {
    let geom = conn.get_geometry(window).unwrap().reply().unwrap();
    let cx = geom.x as i16 + (geom.width / 2) as i16;
    let cy = geom.y as i16 + (geom.height / 2) as i16;

    conn.warp_pointer(x11rb::NONE, screen.root, 0, 0, 0, 0, cx, cy)
        .unwrap();
    conn.flush().unwrap();
}

fn normalize_keysym(sym: u32) -> u32 {
    if sym >= 0x41 && sym <= 0x5A {
        sym + 32
    } else {
        sym
    }
}

fn focus_and_warp(conn: &impl Connection, screen: &Screen, window: Window, state: &mut WMState) {
    focus_window(conn, window, state);
    warp_to_window(conn, screen, window);

    conn.flush().unwrap();
}

fn focus_window(conn: &impl Connection, window: Window, state: &mut WMState) {
    if let Some(prev) = state.focused {
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

    state.focused = Some(window);

    conn.flush().unwrap();
}

fn keysym_from_keycode(idx: usize, syms: &Vec<u32>) -> u32 {
    syms[idx]
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

fn retile(conn: &impl Connection, screen: &Screen, state: &mut WMState) {
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

type WindowSet = Vec<Window>;

struct WMState {
    workspaces: [WindowSet; 8],
    active: usize,
    focused: Option<Window>,
}

impl WMState {
    fn new() -> Self {
        Self {
            workspaces: Default::default(),
            active: 0,
            focused: None,
        }
    }

    fn windows(&self) -> &WindowSet {
        &self.workspaces[self.active]
    }

    fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.workspaces[self.active]
    }
}

fn main() {
    let mut wm_state = WMState::new();

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let setup = conn.setup();
    let screen = &setup.roots[screen_num];

    let first_keycode = setup.min_keycode;
    let count = setup.max_keycode - setup.min_keycode + 1;

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

    conn.change_window_attributes(
        screen.root,
        &ChangeWindowAttributesAux::new()
            .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY),
    )
    .unwrap()
    .check()
    .unwrap();

    conn.change_window_attributes(
        screen.root,
        &ChangeWindowAttributesAux::new().background_pixel(screen.black_pixel),
    )
    .unwrap();

    let kb_map = conn
        .get_keyboard_mapping(first_keycode, count)
        .unwrap()
        .reply()
        .unwrap();
    let syms_per_keycode = kb_map.keysyms_per_keycode;
    let keysyms = kb_map.keysyms;

    println!(
        "First keycode: {}, last: {}, syms per code: {}",
        first_keycode, setup.max_keycode, syms_per_keycode
    );

    for bind in &keybinds {
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

    conn.flush().unwrap();

    loop {
        match conn.wait_for_event().unwrap() {
            Event::Error(e) => eprintln!("error: {:?}", e),
            Event::MapRequest(e) => {
                let window = e.window;
                wm_state.windows_mut().push(window);
                conn.change_window_attributes(
                    window,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::ENTER_WINDOW),
                )
                .unwrap();
                conn.map_window(window).unwrap();
                retile(&conn, &screen, &mut wm_state);

                focus_and_warp(&conn, screen, window, &mut wm_state );

                conn.flush().unwrap();
            }
            Event::UnmapNotify(e) => {
                wm_state.windows_mut().retain(|&w| w != e.window);
                if wm_state.focused == Some(e.window) {
                    wm_state.focused = wm_state.windows().last().copied();
                }
                retile(&conn, &screen, &mut wm_state);
                conn.clear_area(false, screen.root, 0, 0, 0, 0).unwrap();
                conn.flush().unwrap();
            }
            Event::DestroyNotify(e) => {
                wm_state.windows_mut().retain(|&w| w != e.window);
            }
            Event::EnterNotify(e) => {
                if e.mode == NotifyMode::NORMAL && e.detail != NotifyDetail::INFERIOR {
                    focus_window(&conn, e.event, &mut wm_state);
                }
            }
            Event::KeyPress(e) => {
                println!("key: {:#?}", e);
                let keycode = e.detail;
                let state = e.state;
                let idx = (keycode - first_keycode) as usize * kb_map.keysyms_per_keycode as usize;
                let sym = keysym_from_keycode(idx, &keysyms);

                let clean_mask: u16 = !(u16::from(ModMask::M2) | u16::from(ModMask::LOCK));
                let pressed = KeyResult {
                    sym,
                    mods: KeyButMask::from(u16::from(state) & clean_mask),
                };

                for bind in &keybinds {
                    if bind.matches(&pressed) {
                        match &bind.action {
                            WMAction::Spawn(cmd, args) => {
                                Command::new(cmd).args(args).spawn().unwrap();
                            }
                            WMAction::Kill => {
                                if let Some(win) = wm_state.focused {
                                    if wm_state.windows().contains(&win) {
                                        // send WM_DELETE_WINDOW message
                                        let wm_protocols = conn
                                            .intern_atom(false, b"WM_PROTOCOLS")
                                            .unwrap()
                                            .reply()
                                            .unwrap()
                                            .atom;
                                        let wm_delete = conn
                                            .intern_atom(false, b"WM_DELETE_WINDOW")
                                            .unwrap()
                                            .reply()
                                            .unwrap()
                                            .atom;

                                        let data = [wm_delete, 0, 0, 0, 0];
                                        conn.send_event(
                                            false,
                                            win,
                                            EventMask::NO_EVENT,
                                            ClientMessageEvent::new(32, win, wm_protocols, data),
                                        )
                                        .unwrap();
                                        conn.flush().unwrap();
                                    }
                                }
                            }
                            WMAction::FocusNext => {
                                if let Some(win) = wm_state.focused {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
                                        let next =
                                            wm_state.windows()[(idx + 1) % wm_state.windows().len()];
                                        focus_and_warp(&conn, screen, next, &mut wm_state);
                                    }
                                }
                            }
                            WMAction::FocusPrevious => {
                                if let Some(win) = wm_state.focused {
                                    if let Some(idx) =
                                        wm_state.windows().iter().position(|&w| w == win)
                                    {
                                        let prev = wm_state.windows()[(idx
                                            + wm_state.windows().len()
                                            - 1)
                                            % wm_state.windows().len()];
                                        focus_and_warp(&conn, screen, prev, &mut wm_state);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            e => eprintln!("event: {:?}", e),
        }
    }
}
