mod model;
mod tray;
mod wayland;

fn main() {
    env_logger::init();
    wayland::View::run();
}
