use crate::canvas::{Canvas, Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Magenta,
    Cyan,
}

impl Color {
    pub fn to_argb(self: Color) -> [u8; 4] {
        let (r, g, b) = match self {
            Color::Red => (255, 0, 0),
            Color::Green => (0, 255, 0),
            Color::Blue => (0, 0, 255),
            Color::Yellow => (255, 255, 0),
            Color::Magenta => (255, 0, 255),
            Color::Cyan => (0, 255, 255),
        };
        u32::from_be_bytes([255, r, g, b]).to_ne_bytes()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pen(Color),
    Eraser,
}

impl Tool {
    pub fn brush_radius(self: Tool) -> f64 {
        match self {
            Tool::Pen(_) => 1.5,
            Tool::Eraser => 10.0,
        }
    }

    pub fn cursor_shape(self: Tool) -> CursorShape {
        match self {
            Tool::Pen(_) => CursorShape::Crosshair,
            Tool::Eraser => CursorShape::Circle,
        }
    }

    pub fn pixel_color(self: Tool) -> [u8; 4] {
        match self {
            Tool::Pen(color) => color.to_argb(),
            Tool::Eraser => [0, 0, 0, 0],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Crosshair,
    Circle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    R,
    G,
    B,
    Y,
    M,
    N,
    E,
    C,
    ESC,
    F1,
}

pub trait Overlay {
    fn back_canvas(&mut self) -> Option<Canvas<'_>>;

    fn current_tool(&self) -> Tool;
    fn set_current_tool(&mut self, tool: Tool);

    fn show_help(&self) -> bool;
    fn set_show_help(&mut self, show: bool);

    fn on_toggle_help(&mut self) {
        self.set_show_help(!self.show_help());
    }

    // Returns true if the overlay should remain active after this key press,
    // false if it should be closed.
    fn on_key_pressed(&mut self, key: Key) -> bool {
        let new_tool = match key {
            Key::R => Some(Tool::Pen(Color::Red)),
            Key::G => Some(Tool::Pen(Color::Green)),
            Key::B => Some(Tool::Pen(Color::Blue)),
            Key::Y => Some(Tool::Pen(Color::Yellow)),
            Key::M => Some(Tool::Pen(Color::Magenta)),
            Key::N => Some(Tool::Pen(Color::Cyan)),
            Key::E => Some(Tool::Eraser),
            Key::C => {
                if let Some(mut canvas) = self.back_canvas() {
                    canvas.clear();
                }
                None
            }
            Key::ESC => return false,
            Key::F1 => {
                self.on_toggle_help();
                None
            }
        };
        if let Some(tool) = new_tool {
            self.set_current_tool(tool);
        }
        true
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

    fn on_size_changed(&mut self) -> Rect {
        self.back_canvas().map(|mut c| c.clear()).unwrap_or(Rect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        })
    }
}

pub trait App<O>
where
    O: Overlay,
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
