use std::fs::read_to_string;

use rhai::{Dynamic, Engine};

pub fn register_rhai() -> Result<(), Box<dyn std::error::Error>> {
    let engine = Engine::new();
    let file = read_to_string("ftrc.rhai")?;

    let result: Dynamic = engine.eval(&file)?;
    let map = result.cast::<rhai::Map>();
    // let gaps = map.get("gaps").unwrap().clone().cast::<i64>();
    // dbg!(gaps);
    dbg!(map);

    Ok(())
}
