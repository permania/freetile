use std::{collections::HashSet, os::unix::net::UnixListener};

use indexmap::IndexMap;
use kiplib::config::rc::Config;
use rhai::{AST, Engine};
use x11rb::{
    protocol::xproto::{Screen, Window},
    rust_connection::RustConnection,
};

use super::ewmh::Atoms;
use crate::config::{defaults::DEFAULT_LAYOUT_SRC, layout::reader::EngineSetup};

pub mod action;
pub mod focus;
pub mod layout;
pub mod setup;

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
    pub ignore_unmaps: HashSet<Window>,
    pub ignore_enter: bool,
    pub ipc_listener: UnixListener,
    pub layouts: IndexMap<String, LayoutEntry>,
    pub config: Config,
    pub atoms: Atoms,
}

pub struct WMState {
    pub tags: [Tag; 8],
    pub fullscreen_windows: HashSet<Window>,
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
            fullscreen_windows: Default::default(),
            active_tag: 0,
            active_layout: String::new(),
            engine,
            layout_ast: default_ast,
        }
    }

    pub fn windows(&self) -> &WindowSet {
        &self.tags[self.active_tag].windows
    }

    pub fn fullscreen_windows(&self) -> &HashSet<Window> {
        &self.fullscreen_windows
    }

    pub fn fullscreen_windows_tag(&self) -> HashSet<Window> {
        self.fullscreen_windows
            .iter()
            .copied()
            .filter(|w| self.windows().contains(w))
            .collect::<HashSet<Window>>()
    }

    pub fn fullscreen_windows_mut(&mut self) -> &mut HashSet<Window> {
        &mut self.fullscreen_windows
    }

    pub fn windows_mut(&mut self) -> &mut WindowSet {
        &mut self.tags[self.active_tag].windows
    }

    pub fn window_to_tag(&mut self, window: Window, tag: usize) {
        self.tags[tag].windows.push(window)
    }

    pub fn window_exists(&self, window: Window) -> bool {
        self.tags.iter().any(|t| t.windows.contains(&window))
    }

    pub fn remove_window_everywhere(&mut self, window: Window) {
        self.fullscreen_windows_mut().retain(|&w| w != window);
        for tag in self.tags.iter_mut() {
            tag.windows.retain(|&w| w != window);
        }
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
