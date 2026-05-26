mod event;
mod grab;

use std::{collections::HashMap, error::Error};

use bitflags::bitflags;
use grab::KipkeyContext;
use kiplib::config::rc;
use x11rb::protocol::xproto::{self, ModMask};
use xkbcommon::xkb::{self};

const VARS_SECTION: &str = "vars";

bitflags! [
    #[derive(Eq, Hash,PartialEq,Debug)]
    struct Mods: u8 {
    const CTRL	= 0b0001;
    const META	= 0b0010;
    const SUPER	= 0b1000;
    }
];

impl Mods {
    pub fn to_modmask(&self) -> ModMask {
        let mut mask = ModMask::default();
        if self.contains(Self::CTRL) {
            mask |= ModMask::CONTROL;
        }
        if self.contains(Self::META) {
            mask |= ModMask::M1;
        }
        if self.contains(Self::SUPER) {
            mask |= ModMask::M4;
        }
        mask
    }
}

impl From<xproto::KeyButMask> for Mods {
    fn from(mask: xproto::KeyButMask) -> Self {
        let mut mods = Mods::empty();

        if mask.contains(xproto::KeyButMask::from(u16::from(ModMask::CONTROL))) {
            mods |= Mods::CTRL;
        }
        if mask.contains(xproto::KeyButMask::from(u16::from(ModMask::M1))) {
            mods |= Mods::META;
        }
        if mask.contains(xproto::KeyButMask::from(u16::from(ModMask::M4))) {
            mods |= Mods::SUPER;
        }

        mods
    }
}
pub(crate) type GrabbedKeys = Vec<Grab>;
pub(crate) type Grab = (u8, xproto::ModMask);

#[derive(Eq, Hash, PartialEq, Debug)]
struct Keybind {
    keysym: xkbcommon::xkb::Keysym,
    mods: Mods,
}

#[derive(Debug)]
enum Action {
    Cmd { head: String, tail: Vec<String> },
    LayerSwitch(String),
    StackPush(String),
}

impl Action {
    fn resolve_action(value: rc::Value, vars: &HashMap<String, rc::Value>) -> Self {
        match value.0.first() {
            Some(rc::Tagged::Literal(_)) => {
                let flat = flatten_literal(value, vars);
                vec_to_cmd(flat).unwrap()
            }
            Some(rc::Tagged::At(_)) => {
                let flat = flatten_literal(value, vars);
                vec_to_cmd(flat).unwrap()
            }
            Some(rc::Tagged::Bang(s)) => Self::LayerSwitch(s.to_string()),
            Some(rc::Tagged::Question(s)) => Self::StackPush(s.to_string()),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
struct KipkeyState {
    current: String,
    origin: Option<String>,
    grabbed: GrabbedKeys,
    layers: HashMap<String, HashMap<Keybind, Action>>,
}

impl KipkeyState {
    fn switch_layer(&mut self, ctx: &KipkeyContext, layer: String) {
        grab::ungrab_keys(ctx.conn, ctx.screen_root, &self.grabbed);
        self.current = layer;

        let new_keys = self.current_layer_binds();
        let new_grabbed = grab::grab_keys(ctx, new_keys);

        self.grabbed = new_grabbed;
    }

    fn current_layer_binds(&self) -> Vec<&Keybind> {
        self.layers
            .get(&self.current)
            .map(|layer| layer.keys().collect())
            .unwrap_or_default()
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut vars = HashMap::<String, rc::Value>::new();

    let mut state = KipkeyState {
        current: "base".to_string(),
        origin: None,
        grabbed: GrabbedKeys::new(),
        layers: HashMap::<String, HashMap<Keybind, Action>>::new(),
    };

    let config = rc::read_config("kipkeyrc.ksn")?;

    for (s_name, section) in &config {
        if s_name != VARS_SECTION {
            continue;
        }
        for (name, item) in section {
            let s = item.clone();
            vars.insert(name.clone(), s);
        }
    }

    for (s_name, section) in config {
        if s_name == VARS_SECTION {
            continue;
        }
        for (name, item) in section {
            state
                .layers
                .entry(s_name.to_string())
                .or_insert_with(HashMap::new)
                .insert(
                    keybind_from_string(name)?,
                    Action::resolve_action(item, &vars),
                );
        }
    }

    let (conn, screen_num) = x11rb::connect(None).unwrap();
    let ctx = grab::setup_server(&conn, screen_num);

    state.switch_layer(&ctx, "base".to_string());

    event::event_loop(&ctx, &mut state);

    Ok(())
}

fn resolve_at(s: &str, vars: &HashMap<String, rc::Value>) -> Option<rc::Value> {
    vars.get(s).cloned()
}

fn flatten_literal(v: rc::Value, vars: &HashMap<String, rc::Value>) -> Vec<String> {
    let mut out = Vec::new();

    for tag in v.0 {
        match tag {
            rc::Tagged::Literal(s) => {
                out.push(s);
            }

            rc::Tagged::At(name) => {
                if let Some(resolved) = resolve_at(&name, vars) {
                    let mut inner = flatten_literal(resolved, vars);
                    out.append(&mut inner);
                }
            }
            _ => todo!(),
        }
    }

    out
}

fn vec_to_cmd(mut v: Vec<String>) -> Option<Action> {
    let head = v.first()?.clone();
    let tail = v.split_off(1);

    Some(Action::Cmd { head, tail })
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
