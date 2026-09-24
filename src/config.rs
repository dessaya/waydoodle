//! Optional configuration file, read from `$XDG_CONFIG_HOME/waydoodle/config.toml`.
//!
//! A missing or broken configuration is never fatal: it is reported and the
//! defaults are used instead.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::actions::{GlobalAccels, GlobalAction, GlobalTrigger};
use crate::notify::warn_user;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct Config {
    pub pad: PadConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct PadConfig {
    /// `None` when the user didn't say either way, in which case pad support
    /// is on, but quietly.
    pub enabled: Option<bool>,
    /// Action names by button number. Replaces the defaults when present.
    buttons: Option<HashMap<u32, String>>,
}

impl PadConfig {
    /// The configured bindings, ignoring (and reporting) unknown action names.
    pub fn accels(&self) -> GlobalAccels {
        let Some(buttons) = &self.buttons else {
            return GlobalAccels::default();
        };
        buttons
            .iter()
            .filter_map(|(button, name)| match GlobalAction::from_name(name) {
                Some(action) => Some((GlobalTrigger::PadButton(*button), action)),
                None => {
                    warn_user!("Ignoring unknown action '{name}' bound to pad button {button}");
                    None
                }
            })
            .collect()
    }
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
    fn configured_buttons_replace_the_defaults() {
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
        // Button 1 is close-overlay by default, but the table was replaced.
        assert_eq!(accels.get(GlobalTrigger::PadButton(1)), None);
    }

    #[test]
    fn unknown_action_names_are_skipped() {
        let config = parse("[pad.buttons]\n0 = \"nonsense\"\n1 = \"undo\"\n");
        let accels = config.pad.accels();
        assert_eq!(accels.get(GlobalTrigger::PadButton(0)), None);
        assert_eq!(
            accels.get(GlobalTrigger::PadButton(1)),
            Some(GlobalAction::Overlay(Action::Undo))
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(toml::from_str::<Config>("[pad]\nenabeld = true\n").is_err());
        assert!(toml::from_str::<Config>("[nonsense]\n").is_err());
    }
}
