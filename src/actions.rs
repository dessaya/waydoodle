use std::collections::HashMap;
use std::fmt;

use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::{canvas::Color, waydoodle::Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    SetTool(Tool),
    Clear,
    SetBackground(Color),
    Undo,
    OpenContextMenu,
    CloseContextMenu,
    Focus(FocusDirection),
    ApplyMenuSelection,
    HideOverlay,
}

/// Something that can be triggered while the overlay is absent or unfocused.
/// Unlike an [`Action`], it can create or destroy the overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GlobalAction {
    Overlay(Action),
    ToggleOverlay,
    CloseOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum GlobalTrigger {
    PadButton(u32),
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SetTool(Tool::Eraser) => f.write_str("eraser"),
            Self::SetTool(Tool::Pen(color)) => write!(f, "pen-{color}"),
            Self::SetBackground(color) => write!(f, "background-{color}"),
            Self::Clear => f.write_str("clear"),
            Self::Undo => f.write_str("undo"),
            Self::OpenContextMenu => f.write_str("menu-open"),
            Self::CloseContextMenu => f.write_str("menu-close"),
            Self::ApplyMenuSelection => f.write_str("menu-apply"),
            Self::Focus(FocusDirection::Up) => f.write_str("menu-up"),
            Self::Focus(FocusDirection::Down) => f.write_str("menu-down"),
            Self::Focus(FocusDirection::Left) => f.write_str("menu-left"),
            Self::Focus(FocusDirection::Right) => f.write_str("menu-right"),
            Self::HideOverlay => f.write_str("hide-overlay"),
        }
    }
}

impl Action {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        if let Some(color) = name.strip_prefix("pen-") {
            return Some(Self::SetTool(Tool::Pen(Color::from_name(color)?)));
        }
        if let Some(color) = name.strip_prefix("background-") {
            return Some(Self::SetBackground(Color::from_name(color)?));
        }
        Some(match name {
            "eraser" => Self::SetTool(Tool::Eraser),
            "clear" => Self::Clear,
            "undo" => Self::Undo,
            "menu-open" => Self::OpenContextMenu,
            "menu-close" => Self::CloseContextMenu,
            "menu-apply" => Self::ApplyMenuSelection,
            "menu-up" => Self::Focus(FocusDirection::Up),
            "menu-down" => Self::Focus(FocusDirection::Down),
            "menu-left" => Self::Focus(FocusDirection::Left),
            "menu-right" => Self::Focus(FocusDirection::Right),
            "hide-overlay" => Self::HideOverlay,
            _ => return None,
        })
    }
}

impl fmt::Display for GlobalAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ToggleOverlay => f.write_str("toggle-overlay"),
            Self::CloseOverlay => f.write_str("close-overlay"),
            Self::Overlay(action) => action.fmt(f),
        }
    }
}

impl GlobalAction {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "toggle-overlay" => Some(Self::ToggleOverlay),
            "close-overlay" => Some(Self::CloseOverlay),
            _ => Action::from_name(name).map(Self::Overlay),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Keyboard accels, active while the overlay is focused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Keybindings {
    always: Vec<(Keysym, Action)>,
    menu_closed: Vec<(Keysym, Action)>,
    menu_open: Vec<(Keysym, Action)>,
}

impl Keybindings {
    pub(crate) fn action(&self, keysym: Keysym, menu_open: bool) -> Option<Action> {
        let modal = if menu_open {
            &self.menu_open
        } else {
            &self.menu_closed
        };
        self.always
            .iter()
            .chain(modal)
            .find(|(key, _)| *key == keysym)
            .map(|(_, action)| *action)
    }

    /// The key that triggers `action` while the context menu is closed, for
    /// showing next to it in the menu.
    pub(crate) fn key(&self, action: Action) -> Option<Keysym> {
        self.always
            .iter()
            .chain(&self.menu_closed)
            .find(|(_, a)| *a == action)
            .map(|(key, _)| *key)
    }

    pub(crate) fn always(&self) -> &[(Keysym, Action)] {
        &self.always
    }
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            always: DEFAULT_ACCELS_ALWAYS.to_vec(),
            menu_closed: DEFAULT_ACCELS_MENU_CLOSED.to_vec(),
            menu_open: DEFAULT_ACCELS_MENU_OPEN.to_vec(),
        }
    }
}

const DEFAULT_ACCELS_ALWAYS: &[(Keysym, Action)] = &[
    (Keysym::r, Action::SetTool(Tool::Pen(Color::RED))),
    (Keysym::g, Action::SetTool(Tool::Pen(Color::GREEN))),
    (Keysym::b, Action::SetTool(Tool::Pen(Color::BLUE))),
    (Keysym::y, Action::SetTool(Tool::Pen(Color::YELLOW))),
    (Keysym::m, Action::SetTool(Tool::Pen(Color::MAGENTA))),
    (Keysym::n, Action::SetTool(Tool::Pen(Color::CYAN))),
    (Keysym::e, Action::SetTool(Tool::Eraser)),
    (Keysym::period, Action::SetBackground(Color::BLACK)),
    (Keysym::comma, Action::SetBackground(Color::WHITE)),
    (Keysym::slash, Action::SetBackground(Color::TRANSPARENT)),
    (Keysym::c, Action::Clear),
    (Keysym::u, Action::Undo),
];

const DEFAULT_ACCELS_MENU_CLOSED: &[(Keysym, Action)] = &[
    (Keysym::space, Action::OpenContextMenu),
    (Keysym::Escape, Action::HideOverlay),
];

const DEFAULT_ACCELS_MENU_OPEN: &[(Keysym, Action)] = &[
    (Keysym::space, Action::CloseContextMenu),
    (Keysym::Escape, Action::CloseContextMenu),
    (Keysym::Right, Action::Focus(FocusDirection::Right)),
    (Keysym::Left, Action::Focus(FocusDirection::Left)),
    (Keysym::Down, Action::Focus(FocusDirection::Down)),
    (Keysym::Up, Action::Focus(FocusDirection::Up)),
    (Keysym::Return, Action::ApplyMenuSelection),
];

const DEFAULT_GLOBAL_ACCELS: &[(GlobalTrigger, GlobalAction)] = &[
    (GlobalTrigger::PadButton(0), GlobalAction::ToggleOverlay),
    (GlobalTrigger::PadButton(1), GlobalAction::CloseOverlay),
    (
        GlobalTrigger::PadButton(2),
        GlobalAction::Overlay(Action::SetTool(Tool::Eraser)),
    ),
    (
        GlobalTrigger::PadButton(3),
        GlobalAction::Overlay(Action::SetTool(Tool::Pen(Color::RED))),
    ),
    (
        GlobalTrigger::PadButton(4),
        GlobalAction::Overlay(Action::SetTool(Tool::Pen(Color::GREEN))),
    ),
    (
        GlobalTrigger::PadButton(5),
        GlobalAction::Overlay(Action::SetTool(Tool::Pen(Color::MAGENTA))),
    ),
];

/// Accels that are active even when the overlay is absent or unfocused. Owned
/// rather than const, so that they can later be loaded from a config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GlobalAccels(HashMap<GlobalTrigger, GlobalAction>);

impl GlobalAccels {
    pub(crate) fn get(&self, trigger: GlobalTrigger) -> Option<GlobalAction> {
        self.0.get(&trigger).copied()
    }

    pub(crate) fn bind(&mut self, trigger: GlobalTrigger, action: GlobalAction) {
        self.0.insert(trigger, action);
    }

    pub(crate) fn unbind(&mut self, trigger: GlobalTrigger) {
        self.0.remove(&trigger);
    }
}

impl Default for GlobalAccels {
    fn default() -> Self {
        DEFAULT_GLOBAL_ACCELS.iter().copied().collect()
    }
}

impl FromIterator<(GlobalTrigger, GlobalAction)> for GlobalAccels {
    fn from_iter<I: IntoIterator<Item = (GlobalTrigger, GlobalAction)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_accels_cover_the_first_six_pad_buttons() {
        use GlobalTrigger::PadButton;

        let accels = GlobalAccels::default();
        assert_eq!(accels.get(PadButton(0)), Some(GlobalAction::ToggleOverlay));
        assert_eq!(accels.get(PadButton(1)), Some(GlobalAction::CloseOverlay));
        assert_eq!(
            accels.get(PadButton(2)),
            Some(GlobalAction::Overlay(Action::SetTool(Tool::Eraser)))
        );
        assert_eq!(
            accels.get(PadButton(3)),
            Some(GlobalAction::Overlay(Action::SetTool(Tool::Pen(
                Color::RED
            ))))
        );
        assert_eq!(
            accels.get(PadButton(4)),
            Some(GlobalAction::Overlay(Action::SetTool(Tool::Pen(
                Color::GREEN
            ))))
        );
        assert_eq!(
            accels.get(PadButton(5)),
            Some(GlobalAction::Overlay(Action::SetTool(Tool::Pen(
                Color::MAGENTA
            ))))
        );
    }

    #[test]
    fn action_names_round_trip() {
        let all = [
            Action::SetTool(Tool::Eraser),
            Action::SetTool(Tool::Pen(Color::RED)),
            Action::SetBackground(Color::TRANSPARENT),
            Action::Clear,
            Action::Undo,
            Action::OpenContextMenu,
            Action::CloseContextMenu,
            Action::ApplyMenuSelection,
            Action::Focus(FocusDirection::Up),
            Action::Focus(FocusDirection::Down),
            Action::Focus(FocusDirection::Left),
            Action::Focus(FocusDirection::Right),
            Action::HideOverlay,
        ];
        for action in all {
            assert_eq!(Action::from_name(&action.to_string()), Some(action));
        }
    }

    #[test]
    fn global_action_names_round_trip() {
        for (_, action) in DEFAULT_GLOBAL_ACCELS {
            assert_eq!(GlobalAction::from_name(&action.to_string()), Some(*action));
        }
        for action in [GlobalAction::ToggleOverlay, GlobalAction::CloseOverlay] {
            assert_eq!(GlobalAction::from_name(&action.to_string()), Some(action));
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        for name in ["", "pen", "pen-", "pen-chartreuse", "toggle", "menu"] {
            assert_eq!(GlobalAction::from_name(name), None, "{name}");
        }
    }

    #[test]
    fn keybindings_depend_on_whether_the_menu_is_open() {
        let keys = Keybindings::default();
        assert_eq!(
            keys.action(Keysym::space, false),
            Some(Action::OpenContextMenu)
        );
        assert_eq!(
            keys.action(Keysym::space, true),
            Some(Action::CloseContextMenu)
        );
        assert_eq!(keys.action(Keysym::u, false), Some(Action::Undo));
        assert_eq!(keys.action(Keysym::u, true), Some(Action::Undo));
        assert_eq!(keys.action(Keysym::Up, false), None);
    }

    #[test]
    fn keybindings_find_the_key_shown_in_the_menu() {
        let keys = Keybindings::default();
        assert_eq!(keys.key(Action::SetTool(Tool::Eraser)), Some(Keysym::e));
        assert_eq!(keys.key(Action::HideOverlay), Some(Keysym::Escape));
        // Only bound while the menu is open, so there is nothing to show.
        assert_eq!(keys.key(Action::CloseContextMenu), None);
    }

    #[test]
    fn unbound_pad_buttons_have_no_action() {
        let accels = GlobalAccels::default();
        for button in [6, 7, 8, 42] {
            assert_eq!(accels.get(GlobalTrigger::PadButton(button)), None);
        }
    }
}
