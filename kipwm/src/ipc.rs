use std::os::unix::net::UnixListener;

use crate::handler::wm::WMAction;

pub fn open_socket() -> UnixListener {
    let socket_path = "/tmp/kipwm.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path).unwrap();
    listener.set_nonblocking(true).unwrap();
    listener
}

pub fn handle_message(buf: &[u8]) -> Option<WMAction> {
    match buf.first()? {
        0x01 => Some(WMAction::Kill),

        0x02 => match buf.get(1)? {
            0x00 => Some(WMAction::FocusPrevious),
            0x01 => Some(WMAction::FocusNext),
            _ => None,
        },

        0x03 => {
            let tag = *buf.get(1)? as usize;
            Some(WMAction::TagSwitch(tag))
        }

        0x04 => {
            let tag = *buf.get(1)? as usize;
            Some(WMAction::TagWindowSwitch(tag))
        }

        _ => None,
    }
}
