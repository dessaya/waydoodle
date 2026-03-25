mod canvas;
mod tray;
mod waydoodle;
mod wayland;

fn main() {
    env_logger::init();
    wayland::App::run();
}
