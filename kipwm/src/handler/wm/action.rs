use x11rb::protocol::xproto::{ClientMessageEvent, ConnectionExt, EventMask, Window};

use super::{
    WM,
    focus::{focus_and_warp, focused_index, is_mapped, switch_workspace, wrap_next, wrap_prev},
    layout::{map_intent, next_layout, prev_layout, retile, switch_layout},
};
use crate::handler::ewmh;

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
                    if ewmh::atom_in_property(
                        wm.conn,
                        win,
                        wm.atoms._wm_protocols,
                        wm.atoms._wm_delete_window,
                    ) {
                        let data = [wm.atoms._wm_delete_window, 0, 0, 0, 0];
                        wm.conn
                            .send_event(
                                false,
                                win,
                                EventMask::NO_EVENT,
                                ClientMessageEvent::new(32, win, wm.atoms._wm_protocols, data),
                            )
                            .unwrap();
                    } else {
                        wm.conn.kill_client(win).unwrap();
                    }
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

                        for w in intent.unmapped.iter() {
                            wm.ignore_unmaps.insert(*w);
                        }

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

                        for w in intent.unmapped.iter() {
                            wm.ignore_unmaps.insert(*w);
                        }

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
                    wm.state.remove_window_everywhere(win);
                    wm.conn
                        .clear_area(false, wm.screen.root, 0, 0, 0, 0)
                        .unwrap();

                    wm.state.window_to_tag(win, *idx);

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
