mod config;
mod handler;
use config::reader::register_rhai;
use handler::wm;

fn main() {
    // register_rhai();
    wm::run();
}
