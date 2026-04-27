mod config;
mod handler;
use handler::wm;

fn main() {
    // load_config();
    wm::run();
}
