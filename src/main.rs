mod config;
mod handler;
use handler::wm;

fn main() {
    wm::run();
    // lua::run_lua_thing().unwrap();
}
