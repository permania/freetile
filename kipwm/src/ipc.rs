use std::os::unix::net::UnixListener;

use crate::handler::wm::{WM, WMAction};

enum Message {
    Kill,
    Focus { dir: u8, mode: u8 },
    Switch { tag: u8 },
    Follow { tag: u8 },
    Layout { name: String },
    RelLayout { prev: bool },
}

#[derive(Debug)]
#[repr(u8)]
pub enum Response {
    Ok = 0x00,
    Bad = 0x01,
    NoLayout = 0x02,
}

pub fn open_socket() -> UnixListener {
    let socket_path = "/tmp/kipwm.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path).unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

pub fn handle_message(buf: &[u8], wm: &WM) -> Result<WMAction, Response> {
    let msg = decode(buf)?;
    interpret(msg, wm)
}

fn interpret(msg: Message, wm: &WM) -> Result<WMAction, Response> {
    match msg {
        Message::Kill => Ok(WMAction::Kill),

        Message::Focus { dir, mode } => match (dir, mode) {
            (0, 0) => Ok(WMAction::FocusPrevious),
            (1, 0) => Ok(WMAction::FocusNext),
            (0, 1) => Ok(WMAction::SwapPrevious),
            (1, 1) => Ok(WMAction::SwapNext),
            _ => unreachable!(),
        },

        Message::Switch { tag } => Ok(WMAction::TagSwitch(tag as usize)),
        Message::Follow { tag } => Ok(WMAction::TagWindowSwitch(tag as usize)),
        Message::Layout { name } => {
            if !wm.layouts.contains_key(&name) {
                return Err(Response::NoLayout);
            }
            Ok(WMAction::SwitchLayout(name))
        }
        Message::RelLayout { prev } => match prev {
            false => Ok(WMAction::NextLayout),
            true => Ok(WMAction::PrevLayout),
        },
    }
}

fn decode(buf: &[u8]) -> Result<Message, Response> {
    let opcode = *buf.first().ok_or(Response::Bad)?;

    match opcode {
        0x01 => Ok(Message::Kill),

        0x02 => {
            let dir = *buf.get(1).ok_or(Response::Bad)?;
            let mode = *buf.get(2).ok_or(Response::Bad)?;

            Ok(Message::Focus { dir, mode })
        }

        0x03 => {
            let tag = *buf.get(1).ok_or(Response::Bad)?;
            Ok(Message::Switch { tag })
        }

        0x04 => {
            let tag = *buf.get(1).ok_or(Response::Bad)?;
            Ok(Message::Follow { tag })
        }

        0x05 => {
            let len = u16::from_le_bytes([buf[1], buf[2]]) as usize;
            let start = 3;
            let end = start + len;
            let name_bytes = buf.get(start..end).ok_or(Response::Bad)?;
            let name = std::str::from_utf8(name_bytes).map_err(|_| Response::Bad)?;

            Ok(Message::Layout {
                name: name.to_string(),
            })
        }

        0x06 => {
            let prev_flag = buf.get(1).ok_or(Response::Bad)?;

            match *prev_flag {
                0x00u8 => Ok(Message::RelLayout { prev: false }),
                0x01u8 => Ok(Message::RelLayout { prev: true }),
                _ => Err(Response::Bad),
            }
        }

        _ => Err(Response::Bad),
    }
}
