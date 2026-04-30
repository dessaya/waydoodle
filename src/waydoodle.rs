use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::canvas::{Canvas, Color, Point, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeyAction {
    SetTool(Tool),
    Clear,
    SetBackground(Color),
    Undo,
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
    pub(crate) fn swatch(&self) -> Option<Color> {
        match self.action {
            KeyAction::SetTool(Tool::Pen(color)) => Some(color),
            KeyAction::SetBackground(color) => Some(color),
            _ => None,
        }
    }
}

pub(crate) const ALL_KEYS: &[ToolInfo] = &[
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::RED)),
        keysym: Keysym::r,
        key_label: "R",
        desc: "Red pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::GREEN)),
        keysym: Keysym::g,
        key_label: "G",
        desc: "Green pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::BLUE)),
        keysym: Keysym::b,
        key_label: "B",
        desc: "Blue pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::YELLOW)),
        keysym: Keysym::y,
        key_label: "Y",
        desc: "Yellow pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::MAGENTA)),
        keysym: Keysym::m,
        key_label: "M",
        desc: "Magenta pen",
    },
    ToolInfo {
        action: KeyAction::SetTool(Tool::Pen(Color::CYAN)),
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
        action: KeyAction::SetBackground(Color::BLACK),
        keysym: Keysym::period,
        key_label: ".",
        desc: "Black background",
    },
    ToolInfo {
        action: KeyAction::SetBackground(Color::WHITE),
        keysym: Keysym::comma,
        key_label: ",",
        desc: "White background",
    },
    ToolInfo {
        action: KeyAction::SetBackground(Color::TRANSPARENT),
        keysym: Keysym::slash,
        key_label: "/",
        desc: "Transparent background",
    },
    ToolInfo {
        action: KeyAction::Undo,
        keysym: Keysym::u,
        key_label: "U",
        desc: "Undo",
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Pen(Color),
    Eraser,
}

impl Default for Tool {
    fn default() -> Self {
        Tool::Pen(Color::RED)
    }
}

impl Tool {
    pub(crate) fn brush_radius(self) -> f64 {
        match self {
            Tool::Pen(_) => 1.5,
            Tool::Eraser => 10.0,
        }
    }

    pub(crate) fn pixel_color(self, background_color: Color) -> Color {
        match self {
            Tool::Pen(color) => color,
            Tool::Eraser => background_color,
        }
    }

    fn cursor_shape(self) -> CursorShape {
        match self {
            Tool::Pen(_) => CursorShape::Crosshair,
            Tool::Eraser => CursorShape::Circle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorShape {
    Crosshair,
    Circle,
}

pub(crate) struct Stroke {
    pub color: Color,
    pub brush_radius: f64,
    pub points: Vec<Point>,
}

pub(crate) enum HistoryItem {
    Stroke(Stroke),
    Clear,
    SetBackground(Color),
}

pub(crate) struct OverlayState {
    pub canvas: Canvas,
    pub current_stroke: Option<Stroke>,
    pub background_color: Color,
    pub primary_tool: Tool,
    pub override_tool: Option<Tool>,
    pub show_help: bool,
    pub history: Vec<HistoryItem>,
}

impl OverlayState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            canvas: Canvas::new(width, height),
            current_stroke: None,
            background_color: Color::TRANSPARENT,
            primary_tool: Tool::default(),
            override_tool: None,
            show_help: false,
            history: Vec::new(),
        }
    }

    pub fn current_tool(&self) -> Tool {
        self.override_tool.unwrap_or(self.primary_tool)
    }

    // Returns (keep_open, redraw, cursor)
    pub fn on_key_pressed(&mut self, keysym: Keysym) -> (bool, bool, CursorShape) {
        let Some(info) = ALL_KEYS.iter().find(|i| i.keysym == keysym) else {
            return (true, false, self.current_tool().cursor_shape());
        };
        let (keep_open, redraw) = match info.action {
            KeyAction::SetTool(tool) => {
                self.primary_tool = tool;
                (true, false)
            }
            KeyAction::Clear => {
                self.canvas.clear();
                self.history.push(HistoryItem::Clear);
                (true, true)
            }
            KeyAction::SetBackground(color) => {
                if self.background_color != color {
                    self.history.push(HistoryItem::SetBackground(color));
                    self.background_color = color;
                    self.canvas.fill(self.background_color);
                    (true, true)
                } else {
                    (true, false)
                }
            }
            KeyAction::Undo => {
                if self.history.pop().is_some() {
                    self.canvas.clear();
                    self.replay_history();
                    (true, true)
                } else {
                    (true, false)
                }
            }
            KeyAction::ToggleHelp => {
                self.show_help = !self.show_help;
                (true, true)
            }
            KeyAction::HideOverlay => (false, false),
        };
        (keep_open, redraw, self.current_tool().cursor_shape())
    }

    fn replay_history(&mut self) {
        for item in self.history.iter() {
            match item {
                HistoryItem::Stroke(stroke) => {
                    if let Some(first) = stroke.points.first() {
                        self.canvas
                            .draw_circle(*first, stroke.brush_radius, stroke.color);
                        for pair in stroke.points.windows(2) {
                            self.canvas.draw_line(
                                pair[0],
                                pair[1],
                                stroke.brush_radius,
                                stroke.color,
                            );
                        }
                    }
                }
                HistoryItem::Clear => {
                    self.canvas.clear();
                }
                HistoryItem::SetBackground(to) => {
                    self.canvas.fill(*to);
                }
            }
        }
    }

    pub fn on_pointer_enter(&mut self) -> CursorShape {
        self.current_tool().cursor_shape()
    }

    pub fn on_pointer_leave(&mut self) {
        self.current_stroke = None;
    }

    pub fn on_pointer_button_pressed(&mut self, is_secondary_button: bool) -> CursorShape {
        if self.current_stroke.is_some() {
            return self.current_tool().cursor_shape();
        }
        self.override_tool = if is_secondary_button {
            Some(Tool::Eraser)
        } else {
            None
        };
        self.current_tool().cursor_shape()
    }

    pub fn on_pointer_button_released(&mut self) -> CursorShape {
        self.override_tool = None;
        self.current_tool().cursor_shape()
    }

    pub fn begin_stroke(&mut self, pos: Point) -> Rect {
        if self.current_stroke.is_some() {
            return Rect::zero();
        };
        let tool = self.current_tool();
        let brush_radius = tool.brush_radius();
        let color = tool.pixel_color(self.background_color);
        self.current_stroke = Some(Stroke {
            color,
            brush_radius,
            points: vec![pos],
        });
        self.canvas.draw_circle(pos, brush_radius, color)
    }

    pub fn end_stroke(&mut self) {
        let Some(stroke) = self.current_stroke.take() else {
            return;
        };
        self.history.push(HistoryItem::Stroke(stroke));
    }

    pub fn on_pointer_motion(&mut self, pos: Point) -> Option<Rect> {
        let mut stroke = self.current_stroke.take()?;
        let prev = *stroke.points.last()?;
        stroke.points.push(pos);
        let brush_radius = stroke.brush_radius;
        let color = stroke.color;
        self.current_stroke = Some(stroke);
        Some(self.canvas.draw_line(prev, pos, brush_radius, color))
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Rect {
        self.canvas = Canvas::new(width, height);
        self.history = Vec::new();
        Rect {
            x: 0,
            y: 0,
            width: width as i32,
            height: height as i32,
        }
    }
}

pub(crate) trait App<O> {
    fn create_overlay(&mut self);
    fn destroy_overlay(&mut self);
    fn toggle_focus_or_destroy_overlay(&mut self);

    // Returns Some(Some(overlay)) if the overlay is active, Some(None) if it's
    // in the process of being created, and None if it doesn't exist at all.
    fn get_overlay(&self) -> Option<Option<&O>>;

    fn on_toggle_overlay(&mut self) {
        match self.get_overlay() {
            None => self.create_overlay(),
            Some(None) => (),
            Some(Some(_)) => self.toggle_focus_or_destroy_overlay(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    const TEST_WIDTH: u32 = 64;
    const TEST_HEIGHT: u32 = 64;

    struct MockApp {
        overlay: Option<Option<OverlayState>>,
    }

    impl MockApp {
        fn new() -> Self {
            Self { overlay: None }
        }

        fn with_pending() -> Self {
            Self {
                overlay: Some(None),
            }
        }

        fn with_overlay() -> Self {
            Self {
                overlay: Some(Some(OverlayState::new(TEST_WIDTH, TEST_HEIGHT))),
            }
        }
    }

    impl App<OverlayState> for MockApp {
        fn create_overlay(&mut self) {
            self.overlay = Some(Some(OverlayState::new(TEST_WIDTH, TEST_HEIGHT)));
        }

        fn destroy_overlay(&mut self) {
            self.overlay = None;
        }

        fn toggle_focus_or_destroy_overlay(&mut self) {
            self.destroy_overlay();
        }

        fn get_overlay(&self) -> Option<Option<&OverlayState>> {
            self.overlay.as_ref().map(|o| o.as_ref())
        }
    }

    #[test]
    fn pen_brush_radius_is_1_5() {
        assert_eq!(Tool::Pen(Color::RED).brush_radius(), 1.5);
    }

    #[test]
    fn eraser_brush_radius_is_10() {
        assert_eq!(Tool::Eraser.brush_radius(), 10.0);
    }

    #[test]
    fn pen_cursor_shape_is_crosshair() {
        assert_eq!(Tool::Pen(Color::RED).cursor_shape(), CursorShape::Crosshair);
    }

    #[test]
    fn eraser_cursor_shape_is_circle() {
        assert_eq!(Tool::Eraser.cursor_shape(), CursorShape::Circle);
    }

    #[test]
    fn pen_pixel_color_returns_its_color() {
        assert_eq!(Tool::Pen(Color::RED).pixel_color(Color::BLACK), Color::RED);
        assert_eq!(
            Tool::Pen(Color::BLUE).pixel_color(Color::BLACK),
            Color::BLUE
        );
        assert_eq!(
            Tool::Pen(Color::GREEN).pixel_color(Color::BLACK),
            Color::GREEN
        );
    }

    #[test]
    fn eraser_pixel_color_returns_transparent() {
        assert_eq!(Tool::Eraser.pixel_color(Color::BLACK), Color::BLACK);
    }

    #[test]
    fn default_tool_is_red_pen() {
        assert_eq!(Tool::default(), Tool::Pen(Color::RED));
    }

    #[test]
    fn swatch_returns_some_for_pen_entries() {
        for info in ALL_KEYS {
            if let KeyAction::SetTool(Tool::Pen(color)) = info.action {
                assert_eq!(info.swatch(), Some(color));
            }
        }
    }

    #[test]
    fn swatch_returns_none_for_non_pen_entries() {
        for info in ALL_KEYS {
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
            Keysym::F1,
        ];
        for ks in &expected {
            assert!(
                ALL_KEYS.iter().any(|i| i.keysym == *ks),
                "ALL_KEYS missing keysym {:?}",
                ks
            );
        }
        assert_eq!(ALL_KEYS.len(), expected.len());
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
            (Keysym::F1, KeyAction::ToggleHelp),
        ];
        for (keysym, expected_action) in cases {
            let info = ALL_KEYS.iter().find(|i| i.keysym == *keysym).unwrap();
            assert_eq!(
                info.action, *expected_action,
                "wrong action for {:?}",
                keysym
            );
        }
    }

    #[test]
    fn on_key_pressed_r_sets_red_pen() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Eraser;
        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::r);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert_eq!(overlay.primary_tool, Tool::Pen(Color::RED));
    }

    #[test]
    fn on_key_pressed_e_sets_eraser() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::e);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(shape, CursorShape::Circle);
        assert_eq!(overlay.primary_tool, Tool::Eraser);
    }

    #[test]
    fn on_key_pressed_c_clears_canvas() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.canvas.buf[0..4].copy_from_slice(&Color::RED.argb_le());
        overlay.canvas.buf[100..104].copy_from_slice(&Color::BLUE.argb_le());

        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::c);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert!(overlay.canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn on_key_pressed_f1_toggles_help() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        assert!(!overlay.show_help);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::F1);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.show_help);
    }

    #[test]
    fn on_key_pressed_escape_returns_hide() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Escape);
        assert!(!keep);
        assert!(!redraw);
    }

    #[test]
    fn on_key_pressed_unbound_key_changes_nothing() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        let original_tool = overlay.primary_tool;
        let original_help = overlay.show_help;

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::z);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.primary_tool, original_tool);
        assert_eq!(overlay.show_help, original_help);
    }

    #[test]
    fn on_pointer_motion_with_pen_draws_pixels() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });

        let damage = overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        let damage = damage.expect("expected damage from motion while pressed");

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        let mut found_red = false;
        for x in 10..=20 {
            if overlay.canvas.pixel_at(x, 10) == Color::RED {
                found_red = true;
                break;
            }
        }
        assert!(found_red, "expected red pixels along the drag path");
    }

    #[test]
    fn on_pointer_motion_while_not_pressed_returns_none() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);

        let damage = overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        assert!(damage.is_none());
    }

    #[test]
    fn on_pointer_motion_with_eraser_clears_pixels() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        overlay.primary_tool = Tool::Eraser;
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        let damage = overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        let damage = damage.expect("expected damage from eraser motion");

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        let center_pixel = overlay.canvas.pixel_at(20, 30);
        assert_eq!(center_pixel, Color::TRANSPARENT);
    }

    #[test]
    fn right_mouse_button_uses_eraser_tool() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        overlay.on_pointer_button_pressed(true);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        let damage = overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        let damage = damage.expect("expected damage from right-button motion");
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        let center_pixel = overlay.canvas.pixel_at(20, 30);
        assert_eq!(center_pixel, Color::TRANSPARENT);
    }

    #[test]
    fn begin_stroke_draws_circle_at_point() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::GREEN);

        overlay.on_pointer_button_pressed(false);
        let damage = overlay.begin_stroke(Point { x: 32.0, y: 32.0 });

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        let p = overlay.canvas.pixel_at(32, 32);
        assert_eq!(p, Color::GREEN);
    }

    #[test]
    fn on_pointer_leave_clears_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        assert!(overlay.current_stroke.is_some());
        overlay.on_pointer_leave();
        assert!(overlay.current_stroke.is_none());
    }

    #[test]
    fn end_stroke_clears_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        assert!(overlay.current_stroke.is_some());
        overlay.end_stroke();
        overlay.on_pointer_button_released();
        assert!(overlay.current_stroke.is_none());
    }

    #[test]
    fn on_size_changed_clears_canvas_and_returns_full_rect() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.canvas.buf[0..4].copy_from_slice(&Color::RED.argb_le());

        let rect = overlay.resize(TEST_WIDTH, TEST_HEIGHT);
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, TEST_WIDTH as i32);
        assert_eq!(rect.height, TEST_HEIGHT as i32);
        assert!(overlay.canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn toggle_help_false_to_true() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        assert!(!overlay.show_help);
        overlay.on_key_pressed(Keysym::F1);
        assert!(overlay.show_help);
    }

    #[test]
    fn toggle_help_true_to_false() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.show_help = true;
        assert!(overlay.show_help);
        overlay.on_key_pressed(Keysym::F1);
        assert!(!overlay.show_help);
    }

    #[test]
    fn toggle_overlay_when_none_creates_overlay() {
        let mut app = MockApp::new();
        assert!(app.get_overlay().is_none());

        app.on_toggle_overlay();

        let overlay = app.get_overlay();
        assert!(overlay.is_some());
        assert!(overlay.unwrap().is_some());
    }

    #[test]
    fn toggle_overlay_when_pending_does_nothing() {
        let mut app = MockApp::with_pending();
        assert!(matches!(app.get_overlay(), Some(None)));

        app.on_toggle_overlay();

        assert!(matches!(app.get_overlay(), Some(None)));
    }

    #[test]
    fn toggle_overlay_when_ready_destroys_overlay() {
        let mut app = MockApp::with_overlay();
        assert!(matches!(app.get_overlay(), Some(Some(_))));

        app.on_toggle_overlay();

        assert!(app.get_overlay().is_none());
    }

    #[test]
    fn on_key_pressed_u_undoes_last_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        assert_eq!(overlay.history.len(), 1);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.history.is_empty());
        assert!(overlay.canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn on_key_pressed_u_with_empty_strokes_is_noop() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        let buf_before: Vec<u8> = overlay.canvas.buf.clone();

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.canvas.buf, buf_before);
    }

    #[test]
    fn on_key_pressed_c_then_u_restores_drawing() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        let buf_after_draw: Vec<u8> = overlay.canvas.buf.clone();

        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::c);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert!(overlay.canvas.buf.iter().all(|&b| b == 0));

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.canvas.buf, buf_after_draw);
    }

    #[test]
    fn multiple_strokes_undo_in_order() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);

        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();
        let buf_after_first: Vec<u8> = overlay.canvas.buf.clone();

        overlay.primary_tool = Tool::Pen(Color::BLUE);
        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 50.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 50.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        assert_eq!(overlay.history.len(), 2);

        overlay.on_key_pressed(Keysym::u);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(overlay.canvas.buf, buf_after_first);

        overlay.on_key_pressed(Keysym::u);
        assert!(overlay.history.is_empty());
        assert!(overlay.canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn begin_stroke_starts_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::GREEN);

        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 15.0, y: 25.0 });

        let stroke = overlay
            .current_stroke
            .expect("stroke should be in progress");
        assert_eq!(stroke.points.len(), 1);
        assert_eq!(stroke.points[0].x, 15.0);
        assert_eq!(stroke.points[0].y, 25.0);
    }

    #[test]
    fn on_pointer_motion_appends_to_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 5.0, y: 5.0 });
        overlay.on_pointer_motion(Point { x: 15.0, y: 15.0 });

        let stroke = overlay
            .current_stroke
            .expect("stroke should be in progress");
        assert_eq!(stroke.points.len(), 2);
        assert_eq!(stroke.points[1].x, 15.0);
        assert_eq!(stroke.points[1].y, 15.0);
    }

    #[test]
    fn end_stroke_finalizes_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 5.0, y: 5.0 });
        overlay.on_pointer_motion(Point { x: 15.0, y: 15.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        assert!(overlay.current_stroke.is_none());
        assert_eq!(overlay.history.len(), 1);
    }

    #[test]
    fn on_size_changed_clears_history() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released();

        assert_eq!(overlay.history.len(), 1);

        overlay.resize(TEST_WIDTH, TEST_HEIGHT);

        assert!(overlay.history.is_empty());
    }

    #[test]
    fn on_key_pressed_period_fills_black_background() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::period);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::BLACK);
        assert_eq!(overlay.canvas.pixel_at(50, 40), Color::BLACK);
    }

    #[test]
    fn on_key_pressed_comma_fills_white_background() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::comma);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::WHITE);
        assert_eq!(overlay.canvas.pixel_at(50, 40), Color::WHITE);
    }

    #[test]
    fn fill_background_then_undo_restores_previous() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(false);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        let buf_after_draw: Vec<u8> = overlay.canvas.buf.clone();

        overlay.on_key_pressed(Keysym::period);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::BLACK);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.canvas.buf, buf_after_draw);
    }

    #[test]
    fn swatch_returns_some_for_fill_background_entries() {
        for info in ALL_KEYS {
            if let KeyAction::SetBackground(_) = info.action {
                assert!(
                    info.swatch().is_some(),
                    "swatch() should return Some for FillBackground key '{}'",
                    info.key_label,
                );
            }
        }
    }
}
