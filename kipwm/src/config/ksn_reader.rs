use std::process::Command;

use hex_color::HexColor;
use indexmap::IndexMap;
use kiplib::config::rc::{self, Config};
use rhai::AST;

use super::{
    defaults::{AUTOSTART_SECTION, DEFAULT_CONFIG_SRC, WM_CONFIG_PATH},
    layout::reader::load_layout_config,
};
use crate::handler::wm::{LayoutEntry, WMState};

pub fn load_config(
) -> Result<Config, Box<dyn std::error::Error>> {
    let config = if let Ok(c) = rc::read_config_from_path(WM_CONFIG_PATH) {
        c
    } else {
        rc::read_config_from_src(DEFAULT_CONFIG_SRC)
    };

    println!("{:x?}", config.active_border_color());

    // apply_config(&config);

    autostart(&config);

    Ok(config)
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

pub(crate) trait WMConfig {
    fn active_border_color(&self) -> Option<u32>;
    fn active_border_weight(&self) -> Option<u32>;
    fn inactive_border_color(&self) -> Option<u32>;
    fn inactive_border_weight(&self) -> Option<u32>;
    fn top_layout(&self);
}

impl WMConfig for Config {
    fn active_border_color(&self) -> Option<u32> {
        let color = HexColor::parse(
            &self
                .get_section("border")?
                .entries
                .get("color")?
                .to_string(),
        )
        .ok()?;

        Some(
            ((color.a as u32) << 24)
                | ((color.r as u32) << 16)
                | ((color.g as u32) << 8)
                | (color.b as u32),
        )
    }

    fn active_border_weight(&self) -> Option<u32> {
        todo!()
    }

    fn inactive_border_color(&self) -> Option<u32> {
        todo!()
    }

    fn inactive_border_weight(&self) -> Option<u32> {
        todo!()
    }

    fn top_layout(&self) {
        todo!()
    }
}
