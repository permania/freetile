mod config;
mod handler;
use handler::wm;

fn main() {
    unsafe { libc::signal(libc::SIGCHLD, libc::SIG_IGN); }

    wm::run();
}
