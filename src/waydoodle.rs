use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::canvas::{Canvas, Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    SetTool(Tool),
    Clear,
    ToggleHelp,
    HideOverlay,
}

pub(crate) struct ToolInfo {
    pub action: KeyAction,
    pub keysym: Keysym,
    pub key_label: &'static str,
    pub desc: &'static str,
}

impl ToolInfo {
    pub(crate) fn swatch(&self) -> Option<u32> {
        match self.action {
            KeyAction::SetTool(Tool::Pen(color)) => Some(u32::from_le_bytes(color)),
            _ => None,
        }
    }
}

const fn argb(r: u8, g: u8, b: u8) -> [u8; 4] {
    u32::from_be_bytes([255, r, g, b]).to_ne_bytes()
}

const RED: [u8; 4] = argb(255, 0, 0);
const GREEN: [u8; 4] = argb(0, 255, 0);
const BLUE: [u8; 4] = argb(0, 0, 255);
const YELLOW: [u8; 4] = argb(255, 255, 0);
const MAGENTA: [u8; 4] = argb(255, 0, 255);
const CYAN: [u8; 4] = argb(0, 255, 255);

pub(crate) const ALL_KEYS: &[ToolInfo] = &[
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(RED)),
        keysym: Keysym::r,
        key_label: "R",
        desc: "Red pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(GREEN)),
        keysym: Keysym::g,
        key_label: "G",
        desc: "Green pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(BLUE)),
        keysym: Keysym::b,
        key_label: "B",
        desc: "Blue pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(YELLOW)),
        keysym: Keysym::y,
        key_label: "Y",
        desc: "Yellow pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(MAGENTA)),
        keysym: Keysym::m,
        key_label: "M",
        desc: "Magenta pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(CYAN)),
        keysym: Keysym::n,
        key_label: "N",
        desc: "Cyan pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Eraser),
        keysym: Keysym::e,
        key_label: "E",
        desc: "Eraser",
    },
    ToolInfo {
        action: KeyAction::Clear,
        keysym: Keysym::c,
        key_label: "C",
        desc: "Clear screen",
    },
    ToolInfo {
        action: KeyAction::HideOverlay,
        keysym: Keysym::Escape,
        key_label: "Esc",
        desc: "Hide overlay",
    },
    ToolInfo {
        action: KeyAction::ToggleHelp,
        keysym: Keysym::F1,
        key_label: "F1",
        desc: "Toggle this help",
    },
];

pub(crate) const DEFAULT_TOOL: Tool = Tool::Pen(RED);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Pen([u8; 4]),
    Eraser,
}

impl Tool {
    pub(crate) fn brush_radius(self) -> f64 {
        match self {
            Tool::Pen(_) => 1.5,
            Tool::Eraser => 10.0,
        }
    }

    pub(crate) fn cursor_shape(self) -> CursorShape {
        match self {
            Tool::Pen(_) => CursorShape::Crosshair,
            Tool::Eraser => CursorShape::Circle,
        }
    }

    pub(crate) fn pixel_color(self) -> [u8; 4] {
        match self {
            Tool::Pen(color) => color,
            Tool::Eraser => [0, 0, 0, 0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorShape {
    Crosshair,
    Circle,
}

pub(crate) trait OverlayCanvas {
    fn back_canvas(&mut self) -> Option<Canvas<'_>>;
}

pub(crate) trait OverlayTool {
    fn current_tool(&self) -> Tool;
    fn set_current_tool(&mut self, tool: Tool);
}

pub(crate) trait OverlayHelp {
    fn show_help(&self) -> bool;
    fn set_show_help(&mut self, show: bool);

    fn on_toggle_help(&mut self) {
        self.set_show_help(!self.show_help());
    }
}

pub(crate) trait Overlay: OverlayCanvas + OverlayTool + OverlayHelp {
    // Returns (keep_open, redraw)
    fn on_key_pressed(&mut self, keysym: Keysym) -> (bool, bool) {
        let Some(info) = ALL_KEYS.iter().find(|i| i.keysym == keysym) else {
            return (true, false);
        };
        match info.action {
            KeyAction::SetTool(tool) => {
                self.set_current_tool(tool);
                (true, false)
            }
            KeyAction::Clear => {
                if let Some(mut canvas) = self.back_canvas() {
                    canvas.clear();
                }
                (true, true)
            }
            KeyAction::ToggleHelp => {
                self.on_toggle_help();
                (true, true)
            }
            KeyAction::HideOverlay => (false, false),
        }
    }

    fn on_drag(&mut self, from: Point, to: Point) -> Rect {
        let radius = self.current_tool().brush_radius();
        let pixel = self.current_tool().pixel_color();
        self.back_canvas()
            .map(|mut c| c.draw_line(from, to, radius, pixel))
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            })
    }

    fn on_press(&mut self, center: Point) -> Rect {
        let radius = self.current_tool().brush_radius();
        let pixel = self.current_tool().pixel_color();
        self.back_canvas()
            .map(|mut c| c.draw_circle(center, radius, pixel))
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            })
    }

    fn on_size_changed(&mut self) -> Option<Rect> {
        if let Some(mut canvas) = self.back_canvas() {
            Some(canvas.clear())
        } else {
            None
        }
    }
}

impl<T: OverlayCanvas + OverlayTool + OverlayHelp> Overlay for T {}

pub(crate) trait App<O>
where
    O: OverlayCanvas + OverlayTool + OverlayHelp,
{
    fn create_overlay(&mut self);
    fn destroy_overlay(&mut self);
    // Returns Some(Some(overlay)) if the overlay is active, Some(None) if it's
    // in the process of being created, and None if it doesn't exist at all.
    fn get_overlay(&self) -> Option<Option<&O>>;

    fn on_toggle_overlay(&mut self) {
        match self.get_overlay() {
            None => self.create_overlay(),
            Some(None) => (),
            Some(Some(_)) => self.destroy_overlay(),
        }
    }
}
