use std::os::unix::net::UnixListener;

use indexmap::IndexMap;
use kiplib::config::rc::Config;
use rhai::{AST, Dynamic, Engine, Scope};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        ChangeWindowAttributesAux, ClientMessageEvent, ConfigureWindowAux, ConnectionExt,
        EventMask, InputFocus, MapState, Screen, StackMode, Window,
    },
    rust_connection::RustConnection,
};

use super::event::event_loop;
use crate::{
    config::{
        defaults::DEFAULT_LAYOUT_SRC,
        ksn_reader::{WMConfig, load_config},
        layout::{
            reader::{self, EngineSetup, load_layout_config},
            rhai::{LayoutIntent, Rect, WMSlot},
        },
    },
    ipc,
};

type WindowSet = Vec<Window>;

#[derive(Default)]
pub struct Tag {
    windows: WindowSet,
    focused: Option<Window>,
}

#[derive(Clone, Debug)]
pub struct LayoutEntry {
    pub func_name: String,
}

pub struct WM<'a> {
    pub conn: &'a RustConnection,
    pub screen: Screen,
    pub state: WMState,
    pub ignore_unmaps: usize,
    pub ipc_listener: UnixListener,
    pub layouts: IndexMap<String, LayoutEntry>,
    pub config: Config,
}

pub struct WMState {
    pub tags: [Tag; 8],
    pub active_tag: usize,
    pub active_layout: String,
    pub engine: Engine,
    pub layout_ast: AST,
}

impl WMState {
    pub fn new(mut engine: Engine) -> Self {
        engine.setup();

        dbg!("ENGINE SETUP HERE");

        let default_ast = engine
            .compile(DEFAULT_LAYOUT_SRC)
            .expect("default layout must always compile");

        for func in default_ast.iter_functions() {
            println!(
                "Function: {} with {} parameters",
                func.name,
                func.params.len()
            );
        }

        Self {
            tags: Default::default(),
            active_tag: 0,
            // TODO: read this value from config
            active_layout: String::from("monadtall"),
            engine,
            layout_ast: default_ast,
        }
    }

    pub fn windows(&self) -> &WindowSet {
        &self.tags[self.active_tag].windows
    }

    pub fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.tags[self.active_tag].windows
    }

    pub fn focused(&self) -> Option<u32> {
        self.tags[self.active_tag].focused
    }

    pub fn set_focused(&mut self, set: Option<u32>) {
        self.tags[self.active_tag].focused = set;
    }
}

impl Default for WMState {
    fn default() -> Self {
        Self::new(Engine::new())
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
    TagSwitch(usize),
    TagWindowSwitch(usize),
    Kill,
    FocusNext,
    FocusPrevious,
    SwapNext,
    SwapPrevious,
    SwitchLayout(String),
    NextLayout,
    PrevLayout,
}

impl WMAction {
    pub fn execute(&self, wm: &mut WM) {
        match self {
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
            WMAction::SwitchLayout(name) => {
                eprintln!("SWITCH_LAYOUT HIT: {name}");
                eprintln!("active_layout before = {}", wm.state.active_layout);
                switch_layout(wm, name);
                eprintln!("active_layout now = {}", wm.state.active_layout);
            }
            WMAction::NextLayout => {
                next_layout(wm);
            }
            WMAction::PrevLayout => {
                prev_layout(wm);
            }
        }
    }
}

pub fn run() {
    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let mut wm = setup_wm(&conn, screen_num);

    event_loop(&mut wm);
}

fn setup_wm_struct<'a>(conn: &'a RustConnection, screen_num: usize) -> WM<'a> {
    let mut wm_state = WMState::default();
    let config = load_config().unwrap();
    if let Ok((ast, layouts)) = load_layout_config(&mut wm_state, &config) {
        wm_state.layout_ast = ast;
        let setup = conn.setup();

        dbg!(&layouts);

        return WM {
            conn,
            screen: setup.roots[screen_num].clone(),
            state: wm_state,
            ignore_unmaps: 0usize,
            ipc_listener: ipc::open_socket(),
            layouts,
            config,
        };
    } else {
        todo!()
    }
}

fn setup_x11(wm: &mut WM) {
    wm.conn
        .change_window_attributes(
            wm.screen.root,
            &ChangeWindowAttributesAux::new()
                .event_mask(EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY),
        )
        .unwrap()
        .check()
        .unwrap();

    // Black root window
    wm.conn
        .change_window_attributes(
            wm.screen.root,
            &ChangeWindowAttributesAux::new().background_pixel(wm.screen.black_pixel),
        )
        .unwrap();

    wm.conn.flush().unwrap();
}

fn setup_wm<'a>(conn: &'a RustConnection, screen_num: usize) -> WM<'a> {
    let mut wm = setup_wm_struct(conn, screen_num);
    setup_x11(&mut wm);
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

    eprintln!("available layouts:");
    for k in wm.layouts.keys() {
        eprintln!(" - {k}");
    }

    let layout = wm
        .layouts
        .get(&wm.state.active_layout)
        .expect("invalid active layout");

    let func_name = &layout.func_name;

    eprintln!("requested layout = {}", wm.state.active_layout);

    let result: Option<WMSlot> = wm
        .state
        .engine
        .call_fn::<Dynamic>(&mut scope, &wm.state.layout_ast, func_name, (n as i64,))
        .ok()
        .and_then(|d| d.try_cast::<WMSlot>());

    let slot: WMSlot = match result {
        Some(s) => s,
        None => reader::master(0_i64),
    };

    let rects = slot.compute(
        n,
        bounds,
        wm.config.gap_inner().unwrap_or(0),
        wm.config.gap_outer().unwrap_or(0),
    );
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
    let border_width = wm.config.border_weight().unwrap_or(0);

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

pub fn switch_layout<T>(wm: &mut WM, layout: T)
where
    T: AsRef<str>,
{
    wm.state.active_layout = layout.as_ref().to_string();

    let intent = retile(wm);
    map_intent(wm, intent);
}

pub fn next_layout(wm: &mut WM) {
    let keys: Vec<String> = wm.layouts.keys().cloned().collect();

    if !keys.is_empty() {
        let i = keys
            .iter()
            .position(|k| k == &wm.state.active_layout)
            .unwrap_or(0);

        let next_key = &keys[wrap_next(i, keys.len())];

        switch_layout(wm, next_key);
    }
}

pub fn prev_layout(wm: &mut WM) {
    let keys: Vec<String> = wm.layouts.keys().cloned().collect();

    if !keys.is_empty() {
        let i = keys
            .iter()
            .position(|k| k == &wm.state.active_layout)
            .unwrap_or(0);

        let next_key = &keys[wrap_prev(i, keys.len())];

        switch_layout(wm, next_key);
    }
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
            &ChangeWindowAttributesAux::new()
                .border_pixel(wm.config.active_border_color().unwrap_or(0xff8aadf4)),
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
    if *idx == wm.state.active_tag {
        return;
    }

    for &win in wm.state.windows() {
        wm.conn.unmap_window(win).unwrap();
    }

    wm.state.active_tag = *idx;
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
