use std::process::Command;

use x11rb::{connection::Connection, protocol::Event};

use crate::{
    KipkeyState,
    grab::{KipkeyContext, event_to_keybind},
};

pub fn event_loop(ctx: &KipkeyContext, state: &mut KipkeyState) {
    loop {
        match ctx.conn.wait_for_event().unwrap() {
            Event::KeyPress(e) => {
                let caught_bind = event_to_keybind(ctx, e);

                // SHOULD always be Some
                if let Some(action) = state
                    .layers
                    .get(&state.current)
                    .and_then(|l| l.get(&caught_bind))
                {
                    dbg!(&action);

                    match action {
                        crate::Action::Cmd { head, tail } => {
                            let mut shell_str = head.clone();

                            for arg in tail {
                                shell_str.push(' ');
                                shell_str.push_str(arg);
                            }

                            if let Err(e) = Command::new("sh").arg("-c").arg(shell_str).spawn() {
                                eprintln!("failed to spawn command: {e}");
                            }

                            if let Some(origin) = state.origin.take() {
                                state.switch_layer(ctx, origin);
                            }
                        }
                        crate::Action::LayerSwitch(l) => {
                            state.origin = None;
                            state.switch_layer(ctx, l.clone());
                        }
                        crate::Action::StackPush(l) => {
                            if state.origin.is_none() {
                                state.origin = Some(state.current.clone());
                            }

                            state.switch_layer(ctx, l.clone());
                        }
                    };
                } else {
                    eprintln!("this should not be none");
                }
            }
            _ => {}
        }
    }
}
