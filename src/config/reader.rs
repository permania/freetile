use std::{error::Error, fs::read_to_string, io};

use rhai::{Dynamic, Engine};

use crate::config::layout::{Dir, WMSlot};

pub fn master() -> WMSlot {
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

trait EngineSetup {
    fn setup(&mut self);
}

impl EngineSetup for Engine {
    fn setup(&mut self) {
        self.register_type::<WMSlot>()
            .register_fn("some_limit", |n: usize| Some(n))
            .register_fn("no_limit", || None::<usize>)
            .register_fn("h", || Dir::Horizontal)
            .register_fn("v", || Dir::Vertical)
            .register_fn("window", || WMSlot::Window)
            .register_fn("stack", |dir: Dir, slots: Vec<WMSlot>| WMSlot::Stack {
                dir,
                slots,
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
            .register_fn("master", || master());
    }
}

#[allow(dead_code)]
pub fn load_config() -> Result<WMSlot, Box<dyn Error>> {
    let mut engine = Engine::new();
    engine.setup();

    let file = read_to_string("ftrc.rhai")?;

    let result: Dynamic = engine.eval(&file)?;
    Ok(result.cast::<WMSlot>())
}
