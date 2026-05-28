use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    op: Ops,
}

#[derive(Subcommand, Debug)]
enum Ops {
    /// Request for the WM to kill a window
    Kill,

    /// Request for the WM to change window focus
    Focus {
	/// Whether or not the focused window should follow the change in focus
        #[arg(long, short)]
        follow: bool,

        #[arg(value_enum)]
        action: FocusOpt,
    },

    /// Request for the WM to switch to a certain tag
    Tag {
	/// Whether or not the focused window should be moved to the new tag
        #[arg(long, short)]
        follow: bool,

	/// The tag to switch to
        #[arg()]
        idx: usize,
    }, 

    /// Debug: Send a malformed command packet to test error handling
    Bad,
}

#[derive(ValueEnum, Clone, Debug)]
enum FocusOpt {
    Next,
    Prev,
}

fn main() {
    let cli = CliArgs::parse();

    let cmd: Vec<u8> = {
        let mut res = Vec::new();
        match cli.op {
            Ops::Kill => res.extend([0x01]),
            Ops::Focus { follow, action } => res.extend([
                0x02,
                match action {
                    FocusOpt::Prev => 0x00,
                    FocusOpt::Next => 0x01,
                },
                if follow { 0x01 } else { 0x00 },
            ]),
            Ops::Tag { follow, idx } => {
                res.push(if follow { 0x04 } else { 0x03 });
                let idx = u8::try_from(idx).expect("tag out of range (0-255)");
                res.push(idx)
            }
            Ops::Bad => res.push(0xFF),
        }
        res
    };

    let resp = send("/tmp/kipwm.sock", &cmd);
    eprintln!("{:?}", resp);
}

fn send(path: &str, cmd: &[u8]) -> [u8; 1] {
    let mut stream = UnixStream::connect(path).expect("failed to connect");

    stream.write_all(cmd).expect("failed to write");

    let mut buf = [0u8; 1];
    stream.read_exact(&mut buf).expect("failed to read response");

    buf
}
