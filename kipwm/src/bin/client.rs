use std::{io::Write, os::unix::net::UnixStream};

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CliArgs {
    #[clap(subcommand)]
    op: Ops,
}

#[derive(Subcommand, Debug)]
enum Ops {
    Kill,

    Focus {
        #[arg(value_enum)]
        action: FocusOpt,
    },

    Tag {
        #[arg(long, short)]
        follow: bool,

        #[arg()]
        idx: usize,
    },
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
            Ops::Focus { action } => res.extend([
                0x02,
                match action {
                    FocusOpt::Prev => 0x00,
                    FocusOpt::Next => 0x01,
                },
            ]),
            Ops::Tag { follow, idx } => {
                res.push(if follow { 0x04 } else { 0x03 });
                let idx = u8::try_from(idx).expect("tag out of range (0-255)");
                res.push(idx)
            }
        }
        res
    };

    send("/tmp/kipwm.sock", &cmd);
}

fn send(path: &str, cmd: &[u8]) {
    let mut stream = UnixStream::connect(path).expect("failed to connect");

    stream.write_all(cmd).expect("failed to write");
}
