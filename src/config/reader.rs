use std::error::Error;
use std::fs::read_to_string;

use rhai::Engine;

use crate::{config::layout::{Dir, WMSlot}, handler::wm::WMState};

pub const DEFAULT_LAYOUT_SRC: &str = include_str!("default.rhai");

pub fn master(_n: i64) -> WMSlot {
    WMSlot::Split {
        dir: Dir::Horizontal,
        ratio: 0.5,
        lhs: Box::new(WMSlot::Window),
        rhs: Box::new(WMSlot::Drain {
            dir: Dir::Vertical,
            take: None,
        }),
    }
}

pub trait EngineSetup {
    fn setup(&mut self);
}

impl EngineSetup for Engine {
    fn setup(&mut self) {
        self.set_max_expr_depths(0, 0);

        self.register_type::<WMSlot>()
            .register_fn("some_limit", |n: usize| Some(n))
            .register_fn("no_limit", || None::<usize>)
            .register_fn("h", || Dir::Horizontal)
            .register_fn("v", || Dir::Vertical)
            .register_fn("window", || WMSlot::Window)
            .register_fn("stack", |dir: Dir, slots: rhai::Array| {
                let slots = slots.into_iter().map(|s| s.cast::<WMSlot>()).collect();
                WMSlot::Stack { dir, slots }
            })
            .register_fn("drain", |dir: Dir, take: Option<usize>| WMSlot::Drain {
                dir,
                take,
            })
            .register_fn("split", |dir: Dir, ratio: f64, lhs: WMSlot, rhs: WMSlot| {
                WMSlot::Split {
                    dir,
                    ratio,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                }
            })
            .register_fn("master", master);
    }
}

pub fn load_config(wm_state: &mut WMState) -> Result<(), Box<dyn Error>> {
    let file = read_to_string("ftrc.rhai");

    if let Ok(c) = file {
	wm_state.layout_ast = wm_state.engine.compile(c)?;
    }

    Ok(())
}
