use std::process::Command;

use hex_color::HexColor;
use kiplib::config::rc::{self, Config};

use super::defaults::{AUTOSTART_SECTION, DEFAULT_CONFIG_SRC, WM_CONFIG_PATH};

pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let config = if let Ok(c) = rc::read_config_from_path(WM_CONFIG_PATH) {
        c
    } else {
        rc::read_config_from_src(DEFAULT_CONFIG_SRC)
    };

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
    fn active_border_color_inner(&self) -> u32;
    fn active_border_color_outer(&self) -> u32;
    fn inactive_border_color_inner(&self) -> u32;
    fn inactive_border_color_outer(&self) -> u32;
    fn border_weight_inner(&self) -> u16;
    fn border_weight_outer(&self) -> u16;
    fn gap_inner(&self) -> u32 {
        0
    }
    fn gap_outer(&self) -> u32 {
        0
    }
}

impl WMConfig for Config {
    fn active_border_color_inner(&self) -> u32 {
        (|| {
            let color = HexColor::parse(
                &self
                    .get_section("border")?
                    .entries
                    .get("active_color_inner")?
                    .to_string(),
            )
            .ok()?;
            Some(
                ((color.a as u32) << 24)
                    | ((color.r as u32) << 16)
                    | ((color.g as u32) << 8)
                    | (color.b as u32),
            )
        })()
        .unwrap_or(0xff8aadf4)
    }

    fn active_border_color_outer(&self) -> u32 {
        (|| {
            let color = HexColor::parse(
                &self
                    .get_section("border")?
                    .entries
                    .get("active_color_outer")?
                    .to_string(),
            )
            .ok()?;
            Some(
                ((color.a as u32) << 24)
                    | ((color.r as u32) << 16)
                    | ((color.g as u32) << 8)
                    | (color.b as u32),
            )
        })()
        .unwrap_or(0xff444444)
    }

    fn inactive_border_color_inner(&self) -> u32 {
        (|| {
            let color = HexColor::parse(
                &self
                    .get_section("border")?
                    .entries
                    .get("inactive_color_inner")?
                    .to_string(),
            )
            .ok()?;
            Some(
                ((color.a as u32) << 24)
                    | ((color.r as u32) << 16)
                    | ((color.g as u32) << 8)
                    | (color.b as u32),
            )
        })()
        .unwrap_or(0xff444444)
    }

    fn inactive_border_color_outer(&self) -> u32 {
        (|| {
            let color = HexColor::parse(
                &self
                    .get_section("border")?
                    .entries
                    .get("inactive_color_outer")?
                    .to_string(),
            )
            .ok()?;
            Some(
                ((color.a as u32) << 24)
                    | ((color.r as u32) << 16)
                    | ((color.g as u32) << 8)
                    | (color.b as u32),
            )
        })()
        .unwrap_or(0xff444444)
    }

    fn border_weight_inner(&self) -> u16 {
        (|| {
            self.get_section("border")?
                .entries
                .get("weight_inner")?
                .to_string()
                .trim()
                .parse::<u16>()
                .ok()
        })()
        .unwrap_or(2)
    }

    fn border_weight_outer(&self) -> u16 {
        (|| {
            self.get_section("border")?
                .entries
                .get("weight_outer")?
                .to_string()
                .trim()
                .parse::<u16>()
                .ok()
        })()
        .unwrap_or(4)
    }

    fn gap_inner(&self) -> u32 {
        (|| {
            self.get_section("gaps")?
                .entries
                .get("inner")?
                .to_string()
                .trim()
                .parse::<u32>()
                .ok()
        })()
        .unwrap_or(0)
    }

    fn gap_outer(&self) -> u32 {
        (|| {
            self.get_section("gaps")?
                .entries
                .get("outer")?
                .to_string()
                .trim()
                .parse::<u32>()
                .ok()
        })()
        .unwrap_or(0)
    }
}
