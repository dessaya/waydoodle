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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Accels that are active globally, even when the context menu is open.
pub(crate) const GLOBAL_ACCELS: &[(Keysym, Action)] = &[
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

/// Accels that are only active when the context menu is closed.
pub(crate) const NO_MENU_ACCELS: &[(Keysym, Action)] = &[
    (Keysym::space, Action::OpenContextMenu),
    (Keysym::Escape, Action::HideOverlay),
];

/// Accels that are only active when the context menu is open.
pub(crate) const MENU_ACCELS: &[(Keysym, Action)] = &[
    (Keysym::space, Action::CloseContextMenu),
    (Keysym::Escape, Action::CloseContextMenu),
    (Keysym::Right, Action::Focus(FocusDirection::Right)),
    (Keysym::Left, Action::Focus(FocusDirection::Left)),
    (Keysym::Down, Action::Focus(FocusDirection::Down)),
    (Keysym::Up, Action::Focus(FocusDirection::Up)),
    (Keysym::Return, Action::ApplyMenuSelection),
];
