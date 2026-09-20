mod actions;
mod canvas;
mod pad;
mod tray;
mod ui;
mod waydoodle;
mod wayland;

fn main() {
    env_logger::init();
    wayland::App::run();
}
