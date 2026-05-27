use std::process::Command;

use kiplib::config::rc::{self, Config};

const AUTOSTART_SECTION: &str = "autostart";

pub fn load_wm_config() -> Result<Config, std::io::Error> {
    let config = rc::read_config("wm.ksn")?;
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
