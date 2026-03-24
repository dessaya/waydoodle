//! Pure application model for Waydoodle.
//!
//! The model holds the application state (current tool, whether the overlay is
//! active) but knows nothing about the GUI framework, pixel formats, or
//! rendering. Instead of mutating a buffer directly, every method returns
//! [`Command`] values that describe *what* the view should do. The view
//! interprets these commands using whatever rendering backend it has (e.g.
//! Wayland shared-memory buffers). This keeps the model fully testable without
//! any graphical dependencies and makes it trivial to swap out the view layer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
    Magenta,
    Cyan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Pen(Color),
    Eraser,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BrushStyle {
    Draw(Color),
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Crosshair,
    Circle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    ShowOverlay,
    HideOverlay,
    SetCursorShape(CursorShape),
    DrawLine {
        style: BrushStyle,
        radius: f64,
        from: Point,
        to: Point,
    },
    DrawDot {
        style: BrushStyle,
        radius: f64,
        center: Point,
    },
    ClearBuffer,
}

pub const PEN_RADIUS: f64 = 1.5;
pub const ERASER_RADIUS: f64 = 10.0;

pub struct Overlay {
    pub current_tool: Tool,
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            current_tool: Tool::Pen(Color::Red),
        }
    }

    pub fn set_tool(&mut self, tool: Tool) -> Command {
        self.current_tool = tool;
        Command::SetCursorShape(self.cursor_shape())
    }

    pub fn cursor_shape(&self) -> CursorShape {
        match self.current_tool {
            Tool::Pen(_) => CursorShape::Crosshair,
            Tool::Eraser => CursorShape::Circle,
        }
    }

    pub fn clear(&self) -> Command {
        Command::ClearBuffer
    }

    fn brush_style(&self) -> BrushStyle {
        match self.current_tool {
            Tool::Pen(color) => BrushStyle::Draw(color),
            Tool::Eraser => BrushStyle::Erase,
        }
    }

    fn brush_radius(&self) -> f64 {
        match self.current_tool {
            Tool::Pen(_) => PEN_RADIUS,
            Tool::Eraser => ERASER_RADIUS,
        }
    }

    pub fn draw(&self, from: Point, to: Point) -> Command {
        Command::DrawLine {
            style: self.brush_style(),
            radius: self.brush_radius(),
            from,
            to,
        }
    }

    pub fn draw_dot(&self, center: Point) -> Command {
        Command::DrawDot {
            style: self.brush_style(),
            radius: self.brush_radius(),
            center,
        }
    }
}

pub struct Waydoodle {
    pub overlay: Option<Overlay>,
}

impl Waydoodle {
    pub fn new() -> Self {
        Self { overlay: None }
    }

    pub fn toggle_overlay(&mut self) -> Command {
        if self.overlay.is_some() {
            self.hide_overlay()
        } else {
            self.show_overlay()
        }
    }

    pub fn show_overlay(&mut self) -> Command {
        self.overlay = Some(Overlay::new());
        Command::ShowOverlay
    }

    pub fn hide_overlay(&mut self) -> Command {
        self.overlay = None;
        Command::HideOverlay
    }

    /// Reset the overlay to its default state (e.g. after a resolution change).
    /// Returns a cursor-shape command so the view can update accordingly.
    /// No-op if the overlay is not active.
    pub fn reset_overlay(&mut self) -> Option<Command> {
        self.overlay.as_ref()?;
        self.overlay = Some(Overlay::new());
        Some(Command::SetCursorShape(
            self.overlay.as_ref().unwrap().cursor_shape(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn overlay_defaults_to_red_pen() {
        let mut app = Waydoodle::new();
        app.show_overlay();
        assert_eq!(
            app.overlay.as_ref().unwrap().current_tool,
            Tool::Pen(Color::Red)
        );
    }

    #[test]
    fn set_tool_returns_cursor_command() {
        let mut overlay = Overlay::new();

        let cmd = overlay.set_tool(Tool::Pen(Color::Blue));
        assert_eq!(cmd, Command::SetCursorShape(CursorShape::Crosshair));
        assert_eq!(overlay.current_tool, Tool::Pen(Color::Blue));

        let cmd = overlay.set_tool(Tool::Eraser);
        assert_eq!(cmd, Command::SetCursorShape(CursorShape::Circle));
        assert_eq!(overlay.current_tool, Tool::Eraser);

        let cmd = overlay.set_tool(Tool::Pen(Color::Green));
        assert_eq!(cmd, Command::SetCursorShape(CursorShape::Crosshair));
        assert_eq!(overlay.current_tool, Tool::Pen(Color::Green));
    }

    #[test]
    fn cursor_shape_reflects_tool() {
        let mut overlay = Overlay::new();
        assert_eq!(overlay.cursor_shape(), CursorShape::Crosshair);

        overlay.current_tool = Tool::Eraser;
        assert_eq!(overlay.cursor_shape(), CursorShape::Circle);

        overlay.current_tool = Tool::Pen(Color::Yellow);
        assert_eq!(overlay.cursor_shape(), CursorShape::Crosshair);
    }

    #[test]
    fn clear_returns_clear_command() {
        let overlay = Overlay::new();
        assert_eq!(overlay.clear(), Command::ClearBuffer);
    }

    #[test]
    fn draw_dot_pen() {
        let overlay = Overlay::new();
        let cmd = overlay.draw_dot(pt(10.0, 20.0));
        assert_eq!(
            cmd,
            Command::DrawDot {
                style: BrushStyle::Draw(Color::Red),
                radius: PEN_RADIUS,
                center: pt(10.0, 20.0),
            }
        );
    }

    #[test]
    fn draw_dot_eraser() {
        let mut overlay = Overlay::new();
        overlay.current_tool = Tool::Eraser;
        let cmd = overlay.draw_dot(pt(5.0, 5.0));
        assert_eq!(
            cmd,
            Command::DrawDot {
                style: BrushStyle::Erase,
                radius: ERASER_RADIUS,
                center: pt(5.0, 5.0),
            }
        );
    }

    #[test]
    fn draw_line_pen() {
        let mut overlay = Overlay::new();
        overlay.current_tool = Tool::Pen(Color::Blue);
        let cmd = overlay.draw(pt(10.0, 50.0), pt(20.0, 50.0));
        assert_eq!(
            cmd,
            Command::DrawLine {
                style: BrushStyle::Draw(Color::Blue),
                radius: PEN_RADIUS,
                from: pt(10.0, 50.0),
                to: pt(20.0, 50.0),
            }
        );
    }

    #[test]
    fn draw_line_eraser() {
        let mut overlay = Overlay::new();
        overlay.current_tool = Tool::Eraser;
        let cmd = overlay.draw(pt(0.0, 0.0), pt(5.0, 5.0));
        assert_eq!(
            cmd,
            Command::DrawLine {
                style: BrushStyle::Erase,
                radius: ERASER_RADIUS,
                from: pt(0.0, 0.0),
                to: pt(5.0, 5.0),
            }
        );
    }

    #[test]
    fn draw_line_all_colors() {
        let mut overlay = Overlay::new();
        for color in [
            Color::Red,
            Color::Green,
            Color::Blue,
            Color::Yellow,
            Color::Magenta,
            Color::Cyan,
        ] {
            overlay.current_tool = Tool::Pen(color);
            let cmd = overlay.draw(pt(0.0, 0.0), pt(1.0, 1.0));
            assert_eq!(
                cmd,
                Command::DrawLine {
                    style: BrushStyle::Draw(color),
                    radius: PEN_RADIUS,
                    from: pt(0.0, 0.0),
                    to: pt(1.0, 1.0),
                }
            );
        }
    }

    #[test]
    fn waydoodle_starts_with_no_overlay() {
        let app = Waydoodle::new();
        assert!(app.overlay.is_none());
    }

    #[test]
    fn toggle_overlay_on() {
        let mut app = Waydoodle::new();
        let cmd = app.toggle_overlay();
        assert!(app.overlay.is_some());
        assert_eq!(cmd, Command::ShowOverlay);
    }

    #[test]
    fn toggle_overlay_off() {
        let mut app = Waydoodle::new();
        app.toggle_overlay();
        let cmd = app.toggle_overlay();
        assert!(app.overlay.is_none());
        assert_eq!(cmd, Command::HideOverlay);
    }

    #[test]
    fn show_overlay_returns_show() {
        let mut app = Waydoodle::new();
        let cmd = app.show_overlay();
        assert!(app.overlay.is_some());
        assert_eq!(cmd, Command::ShowOverlay);
    }

    #[test]
    fn show_overlay_resets_state() {
        let mut app = Waydoodle::new();
        app.show_overlay();
        app.overlay.as_mut().unwrap().current_tool = Tool::Eraser;

        let cmd = app.show_overlay();
        assert_eq!(
            app.overlay.as_ref().unwrap().current_tool,
            Tool::Pen(Color::Red)
        );
        assert_eq!(cmd, Command::ShowOverlay);
    }

    #[test]
    fn hide_overlay_returns_hide() {
        let mut app = Waydoodle::new();
        app.show_overlay();
        let cmd = app.hide_overlay();
        assert!(app.overlay.is_none());
        assert_eq!(cmd, Command::HideOverlay);
    }

    #[test]
    fn hide_overlay_when_already_hidden() {
        let mut app = Waydoodle::new();
        let cmd = app.hide_overlay();
        assert!(app.overlay.is_none());
        assert_eq!(cmd, Command::HideOverlay);
    }

    #[test]
    fn reset_overlay_resets_tool_to_default() {
        let mut app = Waydoodle::new();
        app.show_overlay();
        app.overlay.as_mut().unwrap().current_tool = Tool::Eraser;

        let cmd = app.reset_overlay();
        assert_eq!(cmd, Some(Command::SetCursorShape(CursorShape::Crosshair)));
        assert_eq!(
            app.overlay.as_ref().unwrap().current_tool,
            Tool::Pen(Color::Red)
        );
    }

    #[test]
    fn reset_overlay_noop_when_hidden() {
        let mut app = Waydoodle::new();
        let cmd = app.reset_overlay();
        assert_eq!(cmd, None);
        assert!(app.overlay.is_none());
    }

    #[test]
    fn full_interaction_scenario() {
        let mut app = Waydoodle::new();

        let cmd = app.toggle_overlay();
        assert_eq!(cmd, Command::ShowOverlay);

        let cmd = app.overlay.as_ref().unwrap().draw_dot(pt(100.0, 200.0));
        assert_eq!(
            cmd,
            Command::DrawDot {
                style: BrushStyle::Draw(Color::Red),
                radius: PEN_RADIUS,
                center: pt(100.0, 200.0),
            }
        );

        let cmd = app
            .overlay
            .as_mut()
            .unwrap()
            .set_tool(Tool::Pen(Color::Blue));
        assert_eq!(cmd, Command::SetCursorShape(CursorShape::Crosshair));

        let cmd = app
            .overlay
            .as_ref()
            .unwrap()
            .draw(pt(100.0, 200.0), pt(150.0, 200.0));
        assert_eq!(
            cmd,
            Command::DrawLine {
                style: BrushStyle::Draw(Color::Blue),
                radius: PEN_RADIUS,
                from: pt(100.0, 200.0),
                to: pt(150.0, 200.0),
            }
        );

        let cmd = app.overlay.as_mut().unwrap().set_tool(Tool::Eraser);
        assert_eq!(cmd, Command::SetCursorShape(CursorShape::Circle));

        let cmd = app.overlay.as_ref().unwrap().draw_dot(pt(120.0, 200.0));
        assert_eq!(
            cmd,
            Command::DrawDot {
                style: BrushStyle::Erase,
                radius: ERASER_RADIUS,
                center: pt(120.0, 200.0),
            }
        );

        let cmd = app.overlay.as_ref().unwrap().clear();
        assert_eq!(cmd, Command::ClearBuffer);

        let cmd = app.toggle_overlay();
        assert_eq!(cmd, Command::HideOverlay);
    }
}
