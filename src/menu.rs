use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::{canvas::Color, waydoodle::Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    SetTool(Tool),
    Clear,
    SetBackground(Color),
    Undo,
    HideOverlay,
}

pub(crate) struct MenuItem {
    pub action: KeyAction,
    pub keysym: Keysym,
    pub key_label: &'static str,
    pub desc: &'static str,
}

impl MenuItem {
    pub(crate) fn swatch(&self) -> Option<Color> {
        match self.action {
            KeyAction::SetTool(Tool::Pen(color)) => Some(color),
            KeyAction::SetBackground(color) => Some(color),
            _ => None,
        }
    }
}

pub(crate) const MENU: &[MenuItem] = &[
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::RED)),
        keysym: Keysym::r,
        key_label: "R",
        desc: "Red pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::GREEN)),
        keysym: Keysym::g,
        key_label: "G",
        desc: "Green pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::BLUE)),
        keysym: Keysym::b,
        key_label: "B",
        desc: "Blue pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::YELLOW)),
        keysym: Keysym::y,
        key_label: "Y",
        desc: "Yellow pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::MAGENTA)),
        keysym: Keysym::m,
        key_label: "M",
        desc: "Magenta pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Pen(Color::CYAN)),
        keysym: Keysym::n,
        key_label: "N",
        desc: "Cyan pen",
    },
    MenuItem {
        action: KeyAction::SetTool(Tool::Eraser),
        keysym: Keysym::e,
        key_label: "E",
        desc: "Eraser",
    },
    MenuItem {
        action: KeyAction::Clear,
        keysym: Keysym::c,
        key_label: "C",
        desc: "Clear screen",
    },
    MenuItem {
        action: KeyAction::SetBackground(Color::BLACK),
        keysym: Keysym::period,
        key_label: ".",
        desc: "Black background",
    },
    MenuItem {
        action: KeyAction::SetBackground(Color::WHITE),
        keysym: Keysym::comma,
        key_label: ",",
        desc: "White background",
    },
    MenuItem {
        action: KeyAction::SetBackground(Color::TRANSPARENT),
        keysym: Keysym::slash,
        key_label: "/",
        desc: "Transparent background",
    },
    MenuItem {
        action: KeyAction::Undo,
        keysym: Keysym::u,
        key_label: "U",
        desc: "Undo",
    },
    MenuItem {
        action: KeyAction::HideOverlay,
        keysym: Keysym::Escape,
        key_label: "Esc",
        desc: "Hide overlay",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swatch_returns_some_for_pen_entries() {
        for info in MENU {
            if let KeyAction::SetTool(Tool::Pen(color)) = info.action {
                assert_eq!(info.swatch(), Some(color));
            }
        }
    }

    #[test]
    fn swatch_returns_none_for_non_pen_entries() {
        for info in MENU {
            match info.action {
                KeyAction::SetTool(Tool::Pen(_)) | KeyAction::SetBackground(_) => {}
                _ => {
                    assert_eq!(info.swatch(), None);
                }
            }
        }
    }

    #[test]
    fn all_keys_contains_expected_keysyms() {
        let expected = [
            Keysym::r,
            Keysym::g,
            Keysym::b,
            Keysym::y,
            Keysym::m,
            Keysym::n,
            Keysym::e,
            Keysym::c,
            Keysym::period,
            Keysym::comma,
            Keysym::slash,
            Keysym::u,
            Keysym::Escape,
        ];
        for ks in &expected {
            assert!(
                MENU.iter().any(|i| i.keysym == *ks),
                "MENU missing keysym {:?}",
                ks
            );
        }
        assert_eq!(MENU.len(), expected.len());
    }

    #[test]
    fn all_keys_maps_keysym_to_correct_action() {
        let cases: &[(Keysym, KeyAction)] = &[
            (Keysym::r, KeyAction::SetTool(Tool::Pen(Color::RED))),
            (Keysym::g, KeyAction::SetTool(Tool::Pen(Color::GREEN))),
            (Keysym::b, KeyAction::SetTool(Tool::Pen(Color::BLUE))),
            (Keysym::y, KeyAction::SetTool(Tool::Pen(Color::YELLOW))),
            (Keysym::m, KeyAction::SetTool(Tool::Pen(Color::MAGENTA))),
            (Keysym::n, KeyAction::SetTool(Tool::Pen(Color::CYAN))),
            (Keysym::e, KeyAction::SetTool(Tool::Eraser)),
            (Keysym::c, KeyAction::Clear),
            (Keysym::period, KeyAction::SetBackground(Color::BLACK)),
            (Keysym::comma, KeyAction::SetBackground(Color::WHITE)),
            (Keysym::slash, KeyAction::SetBackground(Color::TRANSPARENT)),
            (Keysym::u, KeyAction::Undo),
            (Keysym::Escape, KeyAction::HideOverlay),
        ];
        for (keysym, expected_action) in cases {
            let info = MENU.iter().find(|i| i.keysym == *keysym).unwrap();
            assert_eq!(
                info.action, *expected_action,
                "wrong action for {:?}",
                keysym
            );
        }
    }
}
