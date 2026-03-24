mod model;
mod view_wayland;

fn main() {
    env_logger::init();
    view_wayland::View::run();
}
