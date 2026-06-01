use std::process::Command;

use indexmap::IndexMap;
use kiplib::config::rc::{self, Config};
use rhai::AST;

use super::{
    defaults::{AUTOSTART_SECTION, DEFAULT_CONFIG_SRC, WM_CONFIG_PATH},
    layout::reader::load_layout_config,
};
use crate::handler::wm::{LayoutEntry, WMState};

pub fn load_config(
    wm_state: &mut WMState,
) -> Result<(AST, IndexMap<String, LayoutEntry>), Box<dyn std::error::Error>> {
    let config = if let Ok(c) = rc::read_config_from_path(WM_CONFIG_PATH) {
        c
    } else {
        rc::read_config_from_src(DEFAULT_CONFIG_SRC)
    };

    let (ast, layouts) = load_layout_config(wm_state, &config)?;

    autostart(&config);

    Ok((ast, layouts))
}

fn autostart(conf: &Config) {
    let cmds = conf
        .get_section(AUTOSTART_SECTION)
        .map(|s| s.scalars.as_slice())
        .unwrap_or(&[]);

    cmds.iter().for_each(|cmd| {
        let _ = Command::new("sh").arg("-c").arg(cmd.to_string()).spawn();
    });
}
