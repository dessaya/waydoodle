mod actions;
mod canvas;
mod config;
mod pad;
mod tray;
mod ui;
mod waydoodle;
mod wayland;

use std::path::PathBuf;

const USAGE: &str = "\
Usage: waydoodle [OPTIONS]

Options:
      --config PATH     Read the configuration from PATH
      --tablet-pad      Listen to the buttons of drawing tablet pads
      --no-tablet-pad   Ignore the buttons of drawing tablet pads
  -h, --help            Print help
  -V, --version         Print version
";

#[derive(Default)]
pub(crate) struct Options {
    pub config: Option<PathBuf>,
    /// Overrides the configuration file when set.
    pub tablet_pad: Option<bool>,
}

fn main() {
    env_logger::init();

    let mut options = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => match args.next() {
                Some(path) => options.config = Some(PathBuf::from(path)),
                None => {
                    eprint!("Missing PATH after --config\n\n{USAGE}");
                    std::process::exit(2);
                }
            },
            "--tablet-pad" => options.tablet_pad = Some(true),
            "--no-tablet-pad" => options.tablet_pad = Some(false),
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
