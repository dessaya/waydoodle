mod actions;
mod canvas;
mod pad;
mod tray;
mod ui;
mod waydoodle;
mod wayland;

const USAGE: &str = "\
Usage: waydoodle [OPTIONS]

Options:
      --tablet-pad  Listen to the buttons of drawing tablet pads
  -h, --help        Print help
  -V, --version     Print version
";

#[derive(Default)]
pub(crate) struct Options {
    pub tablet_pad: bool,
}

fn main() {
    env_logger::init();

    let mut options = Options::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--tablet-pad" => options.tablet_pad = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return;
            }
            "-V" | "--version" => {
                println!("waydoodle {}", env!("CARGO_PKG_VERSION"));
                return;
            }
            _ => {
                eprint!("Unknown option: {arg}\n\n{USAGE}");
                std::process::exit(2);
            }
        }
    }

    wayland::App::run(&options);
}
