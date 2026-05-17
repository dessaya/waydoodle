use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::{canvas::Color, waydoodle::Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    SetTool(Tool),
    Clear,
    SetBackground(Color),
    Undo,
    ToggleContextMenu,
    HideOverlay,
}

pub(crate) struct Action {
    pub op: Op,
    pub accel: Keysym,
    pub accel_label: &'static str,
    pub desc: &'static str,
}

impl Action {
    pub(crate) fn swatch(&self) -> Option<Color> {
        match self.op {
            Op::SetTool(Tool::Pen(color)) => Some(color),
            Op::SetBackground(color) => Some(color),
            _ => None,
        }
    }
}

pub(crate) enum MenuComponent {
    Category {
        name: &'static str,
        items: &'static [Action],
    },
    Item(Action),
}

pub(crate) const CONTEXT_MENU: &[MenuComponent] = &[
    MenuComponent::Category {
        name: "Pen",
        items: &[
            Action {
                op: Op::SetTool(Tool::Pen(Color::RED)),
                accel: Keysym::r,
                accel_label: "R",
                desc: "Red pen",
            },
            Action {
                op: Op::SetTool(Tool::Pen(Color::GREEN)),
                accel: Keysym::g,
                accel_label: "G",
                desc: "Green pen",
            },
            Action {
                op: Op::SetTool(Tool::Pen(Color::BLUE)),
                accel: Keysym::b,
                accel_label: "B",
                desc: "Blue pen",
            },
            Action {
                op: Op::SetTool(Tool::Pen(Color::YELLOW)),
                accel: Keysym::y,
                accel_label: "Y",
                desc: "Yellow pen",
            },
            Action {
                op: Op::SetTool(Tool::Pen(Color::MAGENTA)),
                accel: Keysym::m,
                accel_label: "M",
                desc: "Magenta pen",
            },
            Action {
                op: Op::SetTool(Tool::Pen(Color::CYAN)),
                accel: Keysym::n,
                accel_label: "N",
                desc: "Cyan pen",
            },
        ],
    },
    MenuComponent::Item(Action {
        op: Op::SetTool(Tool::Eraser),
        accel: Keysym::e,
        accel_label: "E",
        desc: "Eraser",
    }),
    MenuComponent::Category {
        name: "Background",
        items: &[
            Action {
                op: Op::SetBackground(Color::BLACK),
                accel: Keysym::period,
                accel_label: ".",
                desc: "Black background",
            },
            Action {
                op: Op::SetBackground(Color::WHITE),
                accel: Keysym::comma,
                accel_label: ",",
                desc: "White background",
            },
            Action {
                op: Op::SetBackground(Color::TRANSPARENT),
                accel: Keysym::slash,
                accel_label: "/",
                desc: "Transparent background",
            },
        ],
    },
    MenuComponent::Item(Action {
        op: Op::Clear,
        accel: Keysym::c,
        accel_label: "C",
        desc: "Clear screen",
    }),
    MenuComponent::Item(Action {
        op: Op::Undo,
        accel: Keysym::u,
        accel_label: "U",
        desc: "Undo",
    }),
    MenuComponent::Item(Action {
        op: Op::HideOverlay,
        accel: Keysym::Escape,
        accel_label: "Esc",
        desc: "Hide overlay",
    }),
];

pub(crate) const ACCEL_ONLY: &[Action] = &[Action {
    op: Op::ToggleContextMenu,
    accel: Keysym::space,
    accel_label: "Space",
    desc: "Toggle context menu",
}];

pub(crate) fn menu_actions() -> impl Iterator<Item = &'static Action> {
    CONTEXT_MENU.iter().flat_map(|component| match component {
        MenuComponent::Category { items, .. } => items.iter(),
        MenuComponent::Item(action) => std::slice::from_ref(action).iter(),
    })
}

pub(crate) fn all_actions() -> impl Iterator<Item = &'static Action> {
    menu_actions().chain(ACCEL_ONLY.iter())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn swatch_returns_some_for_pen_entries() {
        for info in menu_actions() {
            if let Op::SetTool(Tool::Pen(color)) = info.op {
                assert_eq!(info.swatch(), Some(color));
            }
        }
    }

    #[test]
    fn swatch_returns_none_for_non_pen_entries() {
        for info in menu_actions() {
            match info.op {
                Op::SetTool(Tool::Pen(_)) | Op::SetBackground(_) => {}
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
            Keysym::space,
        ];
        for ks in &expected {
            assert!(
                all_actions().any(|i| i.accel == *ks),
                "MENU missing keysym {:?}",
                ks
            );
        }
        assert_eq!(all_actions().count(), expected.len());
    }

    #[test]
    fn all_keys_maps_keysym_to_correct_action() {
        let cases: &[(Keysym, Op)] = &[
            (Keysym::r, Op::SetTool(Tool::Pen(Color::RED))),
            (Keysym::g, Op::SetTool(Tool::Pen(Color::GREEN))),
            (Keysym::b, Op::SetTool(Tool::Pen(Color::BLUE))),
            (Keysym::y, Op::SetTool(Tool::Pen(Color::YELLOW))),
            (Keysym::m, Op::SetTool(Tool::Pen(Color::MAGENTA))),
            (Keysym::n, Op::SetTool(Tool::Pen(Color::CYAN))),
            (Keysym::e, Op::SetTool(Tool::Eraser)),
            (Keysym::c, Op::Clear),
            (Keysym::period, Op::SetBackground(Color::BLACK)),
            (Keysym::comma, Op::SetBackground(Color::WHITE)),
            (Keysym::slash, Op::SetBackground(Color::TRANSPARENT)),
            (Keysym::u, Op::Undo),
            (Keysym::Escape, Op::HideOverlay),
            (Keysym::space, Op::ToggleContextMenu),
        ];
        for (keysym, expected_action) in cases {
            let info = all_actions().find(|i| i.accel == *keysym).unwrap();
            assert_eq!(info.op, *expected_action, "wrong action for {:?}", keysym);
        }
    }

    #[test]
    fn swatch_returns_some_for_fill_background_entries() {
        for info in menu_actions() {
            if let Op::SetBackground(_) = info.op {
                assert!(
                    info.swatch().is_some(),
                    "swatch() should return Some for FillBackground key '{}'",
                    info.accel_label,
                );
            }
        }
    }
}
