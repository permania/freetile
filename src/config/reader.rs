use std::fs::read_to_string;

use rhai::{Dynamic, Engine};

use crate::config::layout::{Dir, WMSlot};

#[allow(dead_code)]
pub fn load_config() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = Engine::new();

    engine.register_type::<WMSlot>();

    engine.register_fn("some_limit", |n: usize| Some(n));
    engine.register_fn("no_limit", || None::<usize>);
    engine.register_fn("h", || Dir::Horizontal);
    engine.register_fn("v", || Dir::Vertical);
    engine.register_fn("window", || WMSlot::Window);
    engine.register_fn("stack", |dir: Dir, slots: Vec<WMSlot>| WMSlot::Stack {
        dir,
        slots,
    });
    engine.register_fn("drain", |dir: Dir, take: Option<usize>| WMSlot::Drain {
        dir,
        take,
    });
    engine.register_fn("split", |dir: Dir, ratio: f64, lhs: WMSlot, rhs: WMSlot| {
        WMSlot::Split {
            dir,
            ratio,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    });
    engine.register_fn("master", || WMSlot::Split {
        dir: Dir::Horizontal,
        ratio: 0.5,
        lhs: Box::new(WMSlot::Window),
        rhs: Box::new(WMSlot::Drain {
            dir: Dir::Vertical,
            take: None,
        }),
    });

    let file = read_to_string("ftrc.rhai")?;

    let result: Dynamic = engine.eval(&file)?;
    let layout = result.cast::<WMSlot>();

    dbg!(layout);

    Ok(())
}
