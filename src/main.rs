mod config;
mod handler;
use config::reader::load_config;
use handler::wm;

fn main() {
    unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN); }

    wm::run();
}
