use std::os::unix::net::UnixListener;

use crate::handler::wm::WMAction;

enum Message {
    Kill,
    Focus { dir: u8, mode: u8 },
    Switch { tag: u8 },
    Follow { tag: u8 },
}

#[repr(u8)]
enum Response {
    Ok = 0x00,
    Invalid = 0x01,
    NoLayout = 0x02,
}

pub fn open_socket() -> UnixListener {
    let socket_path = "/tmp/kipwm.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path).unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

pub fn handle_message(buf: &[u8]) -> Option<WMAction> {
    let msg = decode(buf)?;
    Some(interpret(msg))
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

fn decode(buf: &[u8]) -> Option<Message> {
    match buf.first()? {
        0x01 => Some(Message::Kill),

        0x02 => Some(Message::Focus {
            dir: *buf.get(1)?,
            mode: *buf.get(2)?,
        }),

        0x03 => Some(Message::Switch { tag: *buf.get(1)? }),

        0x04 => Some(Message::Follow { tag: *buf.get(1)? }),

        _ => None,
    }
}
