use std::{error::Error, fs::read_to_string};

use indexmap::IndexMap;
use kiplib::config::rc::Config;
use rhai::{AST, Engine};

use crate::{
    config::{
        defaults::{DEFAULT_LAYOUT_SRC, LAYOUT_CONFIG_PATH},
        layout::rhai::{Dir, WMSlot},
    },
    handler::wm::{LayoutEntry, WMState},
};

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
            .register_fn(
                "split",
                |dir: Dir, ratio: f64, lhs: WMSlot, rhs: WMSlot| -> _ {
                    WMSlot::Split {
                        dir,
                        ratio,
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    }
                },
            )
            .register_fn("master", master);
    }
}

pub fn load_layout_config(
    wm_state: &mut WMState,
    config: &Config,
) -> Result<(AST, IndexMap<String, LayoutEntry>), Box<dyn Error>> {
    let file = if let Ok(f) = read_to_string(LAYOUT_CONFIG_PATH) {
        f
    } else {
        DEFAULT_LAYOUT_SRC.to_string()
    };

    let ast = wm_state.engine.compile(file)?;

    dbg!(&config);

    let layouts_config = config
        .get_section("layouts")
        .expect("layout field is required");

    dbg!(&layouts_config);

    let mut layouts = IndexMap::new();

    for (layout_name, func_name) in layouts_config {
        let func_name = func_name.to_string();
        let valid = ast
            .iter_functions()
            .any(|f| f.name == func_name && f.params.len() == 1);

        if !valid {
            return Err(format!("invalid layout: {}", func_name).into());
        }

        layouts.insert(
            layout_name.clone(),
            LayoutEntry {
                func_name: func_name.to_string(),
            },
        );
    }

    Ok((ast, layouts))
}
