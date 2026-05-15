use std::{collections::HashMap, error::Error};

use kiplib::config::rc;
use xkbcommon::xkb::{self, Keysym, keysyms};

use bitflags::bitflags;

const VARS_SECTION: &str = "vars";

bitflags! [
    #[derive(Eq, Hash,PartialEq,Debug)]
    struct Mods: u8 {
    const CTRL	= 0b0001;
    const META	= 0b0010;
    const SUPER	= 0b1000;
    }
];

#[derive(Eq, Hash, PartialEq, Debug)]
struct Keybind {
    keysym: xkbcommon::xkb::Keysym,
    mods: Mods,
}

#[derive(Debug)]
enum Action {
    Cmd(String),
    LayerSwitch(String),
    StackPush(String),
}

impl Action {
    fn resolve_action(value: rc::Value, vars: &HashMap<String, String>) -> Self {
        match value {
            rc::Value::Literal(s) => Self::Cmd(s),
            rc::Value::Bang(s) => Self::LayerSwitch(s),
            rc::Value::Question(s) => Self::StackPush(s),
            rc::Value::At(s) => Self::Cmd(vars.get(&s).cloned().unwrap_or(s)),
        }
    }
}

#[derive(Debug)]
struct KipkeyState {
    current: String,
    queue: Vec<String>,
    vars: HashMap<String, String>,
    bindings: HashMap<Keybind, Action>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut vars = HashMap::<String, String>::new();
    let mut layers = HashMap::<String, HashMap<Keybind, Action>>::new();

    let config = rc::read_config("kipkeyrc.ksn")?;

    for (s_name, section) in &config {
        if s_name != VARS_SECTION {
            continue;
        }
        for (name, item) in section {
            if let rc::Value::Literal(i) = item {
                vars.insert(name.clone(), i.clone());
            }
        }
    }

    for (s_name, section) in config {
        if s_name == VARS_SECTION {
            continue;
        }
        for (name, item) in section {
            layers
                .entry(s_name.to_string())
                .or_insert_with(HashMap::new)
                .insert(
                    keybind_from_string(name)?,
                    Action::resolve_action(item, &vars),
                );
        }
    }

    dbg!(&layers);
    dbg!(all_keybinds(&layers));

    Ok(())
}

fn keybind_from_string<T>(stri: T) -> Result<Keybind, &'static str>
where
    T: AsRef<str>,
{
    let mut flags = Mods::empty();
    let s = stri.as_ref();
    let mut parts = s.split('-');

    let key_str = parts.next_back().ok_or("missing key")?;

    let keysym = xkb::keysym_from_name(key_str, xkbcommon::xkb::ffi::XKB_KEYSYM_NO_FLAGS);

    if keysym == xkb::Keysym::NoSymbol {
        return Err("invalid keysym");
    }

    while let Some(next_mod) = parts.next() {
        let flag = match next_mod {
            "s" => Some(Mods::SUPER),
            "C" => Some(Mods::CTRL),
            "M" => Some(Mods::META),
            _ => None,
        };

        if let Some(f) = flag {
            flags |= f;
        }
    }

    Ok(Keybind {
        mods: flags,
        keysym,
    })
}

fn all_keybinds(layers: &HashMap<String, HashMap<Keybind, Action>>) -> Vec<&Keybind> {
    layers.values().flat_map(|layer| layer.keys()).collect()
}
