//! Optional configuration file, read from `$XDG_CONFIG_HOME/waydoodle/config.toml`.
//!
//! A missing or broken configuration is never fatal: it is reported and the
//! defaults are used instead.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use smithay_client_toolkit::seat::keyboard::Keysym;
use xkbcommon::xkb;

use crate::actions::{Action, GlobalAccels, GlobalAction, GlobalTrigger, KeyMode, Keybindings};
use crate::canvas::Color;
use crate::notify::warn_user;
use crate::ui::{MenuFont, Palette};
use crate::waydoodle::OverlaySettings;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct Config {
    pub pad: PadConfig,
    pub keys: KeysConfig,
    pub menu: MenuConfig,
    pub drawing: DrawingConfig,
}

impl Config {
    /// The configured overlay settings, ignoring (and reporting) invalid
    /// values.
    pub fn overlay_settings(&self) -> OverlaySettings {
        let default = OverlaySettings::default();
        OverlaySettings {
            keybindings: self.keys.keybindings(),
            palette: self.menu.palette(),
            menu_font: self.menu.font(),
            pen: color_or(self.drawing.pen.as_deref(), default.pen),
            background: color_or(self.drawing.background.as_deref(), default.background),
            pen_radius: match self.drawing.pen_radius {
                Some(radius) if radius.is_finite() && radius > 0.0 => radius,
                Some(radius) => {
                    warn_user!("Ignoring pen_radius {radius}: it must be greater than zero");
                    default.pen_radius
                }
                None => default.pen_radius,
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct PadConfig {
    /// `None` when the user didn't say either way, in which case pad support
    /// is on, but quietly.
    pub enabled: Option<bool>,
    /// Action names by button number, added to the defaults or overriding
    /// them.
    buttons: HashMap<u32, String>,
}

/// An action name that removes a default binding.
const NONE: &str = "none";

impl PadConfig {
    /// The default bindings with the configured ones applied, ignoring (and
    /// reporting) unknown action names.
    pub fn accels(&self) -> GlobalAccels {
        let mut accels = GlobalAccels::default();
        for (&button, name) in &self.buttons {
            let trigger = GlobalTrigger::PadButton(button);
            if name == NONE {
                accels.unbind(trigger);
                continue;
            }
            match GlobalAction::from_name(name) {
                Some(action) => accels.bind(trigger, action),
                None => {
                    warn_user!("Ignoring unknown action '{name}' bound to pad button {button}")
                }
            }
        }
        accels
    }
}

/// Keyboard bindings by key name, added to the defaults or overriding them.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct KeysConfig {
    always: BTreeMap<String, String>,
    menu_closed: BTreeMap<String, String>,
    menu_open: BTreeMap<String, String>,
}

impl KeysConfig {
    /// The default bindings with the configured ones applied, ignoring (and
    /// reporting) unknown key and action names.
    pub fn keybindings(&self) -> Keybindings {
        let mut keybindings = Keybindings::default();
        let modes = [
            (KeyMode::Always, &self.always),
            (KeyMode::MenuClosed, &self.menu_closed),
            (KeyMode::MenuOpen, &self.menu_open),
        ];
        for (mode, keys) in modes {
            for (key, name) in keys {
                let Some(keysym) = parse_key(key) else {
                    warn_user!("Ignoring unknown key '{key}'");
                    continue;
                };
                if name == NONE {
                    keybindings.unbind(mode, keysym);
                    continue;
                }
                match Action::from_name(name) {
                    Some(action) => keybindings.bind(mode, keysym, action),
                    None => warn_user!("Ignoring unknown action '{name}' bound to key '{key}'"),
                }
            }
        }
        keybindings
    }
}

/// How drawing starts out.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct DrawingConfig {
    pen: Option<String>,
    background: Option<String>,
    pen_radius: Option<f64>,
}

fn color_or(name: Option<&str>, default: Color) -> Color {
    let Some(name) = name else {
        return default;
    };
    Color::from_name(name).unwrap_or_else(|| {
        warn_user!("Ignoring unknown color '{name}'");
        default
    })
}

/// The colors offered in the context menu. Each list replaces its default.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct MenuConfig {
    pens: Option<Vec<String>>,
    backgrounds: Option<Vec<String>>,
    font: Option<String>,
    font_size: Option<f64>,
}

impl MenuConfig {
    /// The configured palette, ignoring (and reporting) unknown color names.
    pub fn palette(&self) -> Palette {
        let default = Palette::default();
        Palette {
            pens: self.pens.as_deref().map_or(default.pens, parse_colors),
            backgrounds: self
                .backgrounds
                .as_deref()
                .map_or(default.backgrounds, parse_colors),
        }
    }

    /// The configured font, ignoring (and reporting) an invalid size.
    pub fn font(&self) -> MenuFont {
        let default = MenuFont::default();
        MenuFont {
            family: self.font.clone().unwrap_or(default.family),
            size: match self.font_size {
                Some(size) if size.is_finite() && size > 0.0 => size,
                Some(size) => {
                    warn_user!("Ignoring font_size {size}: it must be greater than zero");
                    default.size
                }
                None => default.size,
            },
        }
    }
}

fn parse_colors(names: &[String]) -> Vec<Color> {
    names
        .iter()
        .filter_map(|name| {
            let color = Color::from_name(name);
            if color.is_none() {
                warn_user!("Ignoring unknown color '{name}'");
            }
            color
        })
        .collect()
}

/// Parses an xkb keysym name such as `space` or `Escape`, ignoring case, or a
/// single character such as `.`.
fn parse_key(name: &str) -> Option<Keysym> {
    let mut chars = name.chars();
    if let (Some(c), None) = (chars.next(), chars.next()) {
        // Letters are bound without Shift, like the default bindings.
        return Some(Keysym::from_char(c.to_ascii_lowercase()));
    }
    let keysym = xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE);
    (keysym != Keysym::NoSymbol).then_some(keysym)
}

/// Reads the configuration file, falling back to the defaults if it is missing
/// or invalid.
pub(crate) fn load(path: Option<&Path>) -> Config {
    let (path, required) = match path {
        Some(path) => (path.to_path_buf(), true),
        None => match default_path() {
            Some(path) => (path, false),
            None => {
                log::warn!("Neither XDG_CONFIG_HOME nor HOME is set, using the default settings");
                return Config::default();
            }
        },
    };

    let contents = match std::fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {
            log::debug!("No configuration file at {}", path.display());
            return Config::default();
        }
        Err(e) => {
            warn_user!("Failed to read {}: {e}", path.display());
            return Config::default();
        }
    };

    match toml::from_str(&contents) {
        Ok(config) => {
            log::info!("Loaded the configuration from {}", path.display());
            config
        }
        Err(e) => {
            // The full error spans several lines, which reads badly in a
            // notification.
            log::debug!("{e}");
            warn_user!("Failed to parse {}: {}", path.display(), e.message());
            Config::default()
        }
    }
}

fn default_path() -> Option<PathBuf> {
    let dir = match std::env::var_os("XDG_CONFIG_HOME") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => PathBuf::from(std::env::var_os("HOME")?).join(".config"),
    };
    Some(dir.join("waydoodle").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions::Action, canvas::Color, waydoodle::Tool};

    fn parse(contents: &str) -> Config {
        toml::from_str(contents).expect("Failed to parse")
    }

    #[test]
    fn pad_support_is_unset_unless_configured() {
        assert_eq!(Config::default().pad.enabled, None);
        assert_eq!(parse("").pad.enabled, None);
        assert_eq!(parse("[pad]\nenabled = false\n").pad.enabled, Some(false));
        assert_eq!(parse("[pad]\nenabled = true\n").pad.enabled, Some(true));
    }

    #[test]
    fn default_accels_are_used_when_no_buttons_are_configured() {
        assert_eq!(
            parse("[pad]\nenabled = true\n").pad.accels(),
            GlobalAccels::default()
        );
    }

    #[test]
    fn configured_buttons_override_the_defaults() {
        let config = parse("[pad.buttons]\n0 = \"undo\"\n7 = \"pen-#ff8800\"\n");
        let accels = config.pad.accels();
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(0)),
            Some(GlobalAction::Overlay(Action::Undo))
        );
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(7)),
            Some(GlobalAction::Overlay(Action::SetTool(Tool::Pen(
                Color::from_name("#ff8800").unwrap()
            ))))
        );
        // Not configured, so the default is kept.
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(1)),
            Some(GlobalAction::CloseOverlay)
        );
    }

    #[test]
    fn none_removes_a_default_binding() {
        let accels = parse("[pad.buttons]\n1 = \"none\"\n").pad.accels();
        assert_eq!(accels.get(GlobalTrigger::PadButton(1)), None);
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(0)),
            Some(GlobalAction::ToggleOverlay)
        );
    }

    #[test]
    fn unknown_action_names_are_skipped() {
        let config = parse("[pad.buttons]\n0 = \"nonsense\"\n1 = \"undo\"\n");
        let accels = config.pad.accels();
        // The typo is ignored, leaving the default in place.
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(0)),
            Some(GlobalAction::ToggleOverlay)
        );
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(1)),
            Some(GlobalAction::Overlay(Action::Undo))
        );
    }

    fn keys(contents: &str) -> Keybindings {
        parse(contents).keys.keybindings()
    }

    #[test]
    fn configured_keys_override_the_defaults() {
        let keys = keys("[keys.always]\nx = \"pen-red\"\nu = \"clear\"\n");
        let red = Some(Action::SetTool(Tool::Pen(Color::RED)));
        assert_eq!(keys.action(Keysym::x, false), red);
        assert_eq!(keys.action(Keysym::u, false), Some(Action::Clear));
        // Not configured, so the default is kept.
        assert_eq!(keys.action(Keysym::r, false), red);
    }

    #[test]
    fn none_removes_a_default_key() {
        let keys = keys("[keys.always]\nr = \"none\"\n");
        assert_eq!(keys.action(Keysym::r, false), None);
        assert_eq!(
            keys.action(Keysym::g, false),
            Some(Action::SetTool(Tool::Pen(Color::GREEN)))
        );
    }

    #[test]
    fn keys_are_configured_per_mode() {
        let keys = keys("[keys.menu_open]\nq = \"menu-close\"\n");
        assert_eq!(keys.action(Keysym::q, true), Some(Action::CloseContextMenu));
        assert_eq!(keys.action(Keysym::q, false), None);
    }

    #[test]
    fn key_names_are_xkb_names_or_characters() {
        let keys = keys("[keys.always]\nESCAPE = \"undo\"\n\".\" = \"clear\"\nR = \"eraser\"\n");
        // Case is ignored.
        assert_eq!(keys.action(Keysym::Escape, false), Some(Action::Undo));
        assert_eq!(keys.action(Keysym::period, false), Some(Action::Clear));
        // Letters are bound without Shift.
        assert_eq!(
            keys.action(Keysym::r, false),
            Some(Action::SetTool(Tool::Eraser))
        );
    }

    #[test]
    fn unknown_keys_are_skipped() {
        let keys = keys("[keys.always]\nnonsense = \"undo\"\nx = \"clear\"\n");
        assert_eq!(keys.action(Keysym::x, false), Some(Action::Clear));
        assert_eq!(keys.action(Keysym::u, false), Some(Action::Undo));
    }

    #[test]
    fn default_palette_is_used_when_no_menu_is_configured() {
        assert_eq!(parse("").menu.palette(), Palette::default());
    }

    #[test]
    fn configured_colors_replace_their_default_list() {
        let palette = parse("[menu]\npens = [\"blue\", \"#ff8800\", \"chartreuse\"]\n")
            .menu
            .palette();
        // The unknown color is skipped.
        assert_eq!(
            palette.pens,
            [Color::BLUE, Color::from_name("#ff8800").unwrap()]
        );
        assert_eq!(palette.backgrounds, Palette::default().backgrounds);
    }

    #[test]
    fn drawing_defaults_are_configurable() {
        let settings = parse("[drawing]\npen = \"blue\"\nbackground = \"black\"\npen_radius = 3\n")
            .overlay_settings();
        assert_eq!(settings.pen, Color::BLUE);
        assert_eq!(settings.background, Color::BLACK);
        // An integer is accepted where a float is expected.
        assert_eq!(settings.pen_radius, 3.0);
    }

    #[test]
    fn invalid_drawing_defaults_are_ignored() {
        let default = OverlaySettings::default();
        for contents in [
            "[drawing]\npen_radius = 0\n",
            "[drawing]\npen_radius = -2.5\n",
            "[drawing]\npen_radius = nan\n",
        ] {
            assert_eq!(
                parse(contents).overlay_settings().pen_radius,
                default.pen_radius
            );
        }
        let settings = parse("[drawing]\npen = \"chartreuse\"\n").overlay_settings();
        assert_eq!(settings.pen, default.pen);
    }

    #[test]
    fn menu_font_is_configurable() {
        let font = parse("[menu]\nfont = \"Serif\"\nfont_size = 20\n")
            .menu
            .font();
        assert_eq!(
            font,
            MenuFont {
                family: "Serif".to_string(),
                size: 20.0
            }
        );
        assert_eq!(
            parse("[menu]\nfont_size = 0\n").menu.font().size,
            MenuFont::default().size
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(toml::from_str::<Config>("[pad]\nenabeld = true\n").is_err());
        assert!(toml::from_str::<Config>("[nonsense]\n").is_err());
    }
}
