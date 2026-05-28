use std::os::unix::net::UnixListener;

use crate::handler::wm::WMAction;

enum Message {
    Kill,
    Focus { dir: u8, mode: u8 },
    Switch { tag: u8 },
    Follow { tag: u8 },
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

pub fn handle_message(buf: &[u8]) -> Result<WMAction, Response> {
    let msg = decode(buf)?;
    Ok(interpret(msg))
}

fn interpret(msg: Message) -> WMAction {
    match msg {
        Message::Kill => WMAction::Kill,

        Message::Focus { dir, mode } => match (dir, mode) {
            (0, 0) => WMAction::FocusPrevious,
            (1, 0) => WMAction::FocusNext,
            (0, 1) => WMAction::SwapPrevious,
            (1, 1) => WMAction::SwapNext,
            _ => unreachable!(),
        },

        Message::Switch { tag } => WMAction::TagSwitch(tag as usize),
        Message::Follow { tag } => WMAction::TagWindowSwitch(tag as usize),
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

        _ => Err(Response::Bad),
    }
}
