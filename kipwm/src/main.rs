mod config;
mod handler;
mod ipc;

use config::ksn_reader::load_wm_config;
use handler::wm;

fn main() {
    unsafe {
        libc::signal(libc::SIGCHLD, libc::SIG_IGN);
    }

    load_wm_config();

    // wm::run();
}
