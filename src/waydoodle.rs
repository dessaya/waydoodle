use cairo::RectangleInt;
use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::{
    actions::{Action, GLOBAL_ACCELS, MENU_ACCELS, NO_MENU_ACCELS},
    canvas::{Canvas, Color, Point},
    ui::UI,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputButton {
    Primary,   // left mouse button or tablet stylus press
    Secondary, // right mouse button or tablet stylus button
    Tertiary,  // middle mouse button or tablet stylus button
}

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

#[derive(Debug)]
pub(crate) struct Stroke {
    pub color: Color,
    pub brush_radius: f64,
    pub points: Vec<Point>,
}

#[derive(Debug)]
pub(crate) enum HistoryItem {
    Stroke(Stroke),
    Clear(Color),
}

pub(crate) struct OverlayState {
    pub canvas: Canvas,
    pub current_stroke: Option<Stroke>,
    pub background_color: Color,
    pub primary_tool: Tool,
    pub override_tool: Option<Tool>,
    pub history: Vec<HistoryItem>,
    pub ui: UI,
}

impl OverlayState {
    pub fn new(width: i32, height: i32) -> Result<Self, cairo::Error> {
        Ok(Self {
            canvas: Canvas::new(width, height),
            current_stroke: None,
            background_color: Color::TRANSPARENT,
            primary_tool: Tool::default(),
            override_tool: None,
            history: Vec::new(),
            ui: UI::new(width, height)?,
        })
    }

    pub fn current_tool(&self) -> Tool {
        self.override_tool.unwrap_or(self.primary_tool)
    }

    // Returns keep_open
    fn apply_action(&mut self, action: Action) -> bool {
        match action {
            Action::SetTool(tool) => {
                self.primary_tool = tool;
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::Clear => {
                self.canvas.fill(self.background_color);
                self.history.push(HistoryItem::Clear(self.background_color));
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::SetBackground(color) => {
                if self.background_color != color {
                    self.background_color = color;
                    self.canvas.fill(self.background_color);
                    self.history.push(HistoryItem::Clear(self.background_color));
                }
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::Undo => {
                if self.history.pop().is_some() {
                    self.canvas.clear();
                    self.replay_history();
                }
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::OpenContextMenu => {
                self.ui.open_context_menu().unwrap();
                true
            }
            Action::CloseContextMenu => {
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::Focus(direction) => {
                self.ui.focus_menu_item(direction).unwrap();
                true
            }
            Action::ApplyMenuSelection => {
                if let Some(action) = self.ui.get_menu_selection() {
                    self.apply_action(action);
                }
                self.ui.close_context_menu().unwrap();
                true
            }
            Action::HideOverlay => false,
        }
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
                HistoryItem::Clear(color) => {
                    self.canvas.fill(*color);
                }
            }
        }
    }

    fn match_accel(&self, keysym: Keysym) -> Option<Action> {
        GLOBAL_ACCELS
            .iter()
            .chain(if self.ui.is_context_menu_open() {
                MENU_ACCELS.iter()
            } else {
                NO_MENU_ACCELS.iter()
            })
            .find_map(|(accel_keysym, action)| {
                if *accel_keysym == keysym {
                    Some(*action)
                } else {
                    None
                }
            })
    }

    // Returns (keep_open, redraw, cursor)
    pub fn on_key_pressed(&mut self, keysym: Keysym) -> (bool, bool, CursorShape) {
        let Some(action) = self.match_accel(keysym) else {
            return (true, false, self.current_tool().cursor_shape());
        };
        let keep_open = self.apply_action(action);
        (keep_open, true, self.current_tool().cursor_shape())
    }

    pub fn on_pointer_enter(&mut self) -> CursorShape {
        self.current_tool().cursor_shape()
    }

    pub fn on_pointer_leave(&mut self) {
        self.current_stroke = None;
    }

    // Returns (keep_open, redraw, cursor)
    pub fn on_pointer_button_pressed(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> (bool, bool, CursorShape) {
        let (action, handled) = self.ui.on_pointer_button_pressed(pos, btn).unwrap();
        if let Some(action) = action {
            let keep_open = self.apply_action(action);
            return (keep_open, true, self.current_tool().cursor_shape());
        }
        if handled {
            return (true, true, self.current_tool().cursor_shape());
        }
        if self.current_stroke.is_some() {
            return (true, false, self.current_tool().cursor_shape());
        }
        self.override_tool = if btn == InputButton::Tertiary {
            Some(Tool::Eraser)
        } else {
            None
        };
        (true, false, self.current_tool().cursor_shape())
    }

    // Returns (keep_open, redraw, cursor)
    pub fn on_pointer_button_released(
        &mut self,
        pos: Point,
        btn: InputButton,
    ) -> (bool, bool, CursorShape) {
        let (action, handled) = self.ui.on_pointer_button_released(pos, btn).unwrap();
        if let Some(action) = action {
            let keep_open = self.apply_action(action);
            return (keep_open, true, self.current_tool().cursor_shape());
        }
        if handled {
            return (true, true, self.current_tool().cursor_shape());
        }
        self.override_tool = None;
        (true, false, self.current_tool().cursor_shape())
    }

    pub fn begin_stroke(&mut self, pos: Point) -> RectangleInt {
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

    pub fn on_pointer_motion(&mut self, pos: Point) -> Option<RectangleInt> {
        let redraw = self.ui.on_pointer_motion(pos).unwrap();
        if redraw {
            return Some(RectangleInt::new(
                0,
                0,
                self.canvas.width(),
                self.canvas.height(),
            ));
        }
        if self.ui.is_context_menu_open() {
            return None;
        }
        let mut stroke = self.current_stroke.take()?;
        let prev = *stroke.points.last()?;
        stroke.points.push(pos);
        let brush_radius = stroke.brush_radius;
        let color = stroke.color;
        self.current_stroke = Some(stroke);
        Some(self.canvas.draw_line(prev, pos, brush_radius, color))
    }

    pub fn resize(&mut self, width: i32, height: i32) -> RectangleInt {
        self.canvas = Canvas::new(width, height);
        self.history = Vec::new();
        RectangleInt::new(0, 0, width, height)
    }
}

pub(crate) enum OverlayStatus {
    None,
    Pending,
    Ready,
}

pub(crate) trait OverlayController {
    fn overlay_status(&self) -> OverlayStatus;
    fn create_overlay(&mut self);
    fn destroy_overlay(&mut self);
    fn toggle_focus_or_destroy_overlay(&mut self);

    fn on_toggle_overlay(&mut self) {
        match self.overlay_status() {
            OverlayStatus::None => self.create_overlay(),
            OverlayStatus::Pending => (),
            OverlayStatus::Ready => self.toggle_focus_or_destroy_overlay(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Point;

    const TEST_WIDTH: i32 = 64;
    const TEST_HEIGHT: i32 = 64;

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
                overlay: Some(Some(OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap())),
            }
        }
    }

    impl OverlayController for MockApp {
        fn overlay_status(&self) -> OverlayStatus {
            match self.overlay {
                None => OverlayStatus::None,
                Some(None) => OverlayStatus::Pending,
                Some(Some(_)) => OverlayStatus::Ready,
            }
        }

        fn create_overlay(&mut self) {
            self.overlay = Some(Some(OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap()));
        }

        fn destroy_overlay(&mut self) {
            self.overlay = None;
        }

        fn toggle_focus_or_destroy_overlay(&mut self) {
            self.destroy_overlay();
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
    fn on_key_pressed_r_sets_red_pen() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Eraser;
        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::r);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert_eq!(overlay.primary_tool, Tool::Pen(Color::RED));
    }

    #[test]
    fn on_key_pressed_e_sets_eraser() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::e);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Circle);
        assert_eq!(overlay.primary_tool, Tool::Eraser);
    }

    fn assert_all_pixels_color(canvas: &mut Canvas, expected: Color) {
        for x in 0..TEST_WIDTH {
            for y in 0..TEST_HEIGHT {
                assert_eq!(
                    canvas.pixel_at(x, y),
                    expected,
                    "expected pixel at ({}, {}) to be {:?}",
                    x,
                    y,
                    expected
                );
            }
        }
    }

    #[test]
    fn on_key_pressed_c_clears_canvas() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.canvas.fill(Color::RED);

        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::c);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert_all_pixels_color(&mut overlay.canvas, Color::TRANSPARENT);
    }

    #[test]
    fn on_key_pressed_escape_returns_hide() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Escape);
        assert!(!keep);
        assert!(redraw);
    }

    #[test]
    fn on_key_pressed_unbound_key_changes_nothing() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        let original_tool = overlay.primary_tool;

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::z);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.primary_tool, original_tool);
    }

    #[test]
    fn on_pointer_motion_with_pen_draws_pixels() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });

        let damage = overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        let damage = damage.expect("expected damage from motion while pressed");

        assert!(damage.width() > 0);
        assert!(damage.height() > 0);

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
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        let damage = overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        assert!(damage.is_none());
    }

    #[test]
    fn on_pointer_motion_with_eraser_clears_pixels() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 30.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 30.0, y: 30.0 }, InputButton::Primary);

        overlay.primary_tool = Tool::Eraser;
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 30.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        let damage = overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        let damage = damage.expect("expected damage from eraser motion");

        assert!(damage.width() > 0);
        assert!(damage.height() > 0);

        let center_pixel = overlay.canvas.pixel_at(20, 30);
        assert_eq!(center_pixel, Color::TRANSPARENT);
    }

    #[test]
    fn middle_mouse_button_uses_eraser_tool() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 30.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 30.0, y: 30.0 }, InputButton::Primary);

        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 30.0 }, InputButton::Tertiary);
        overlay.begin_stroke(Point { x: 10.0, y: 30.0 });
        let damage = overlay.on_pointer_motion(Point { x: 30.0, y: 30.0 });
        let damage = damage.expect("expected damage from right-button motion");
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 30.0, y: 30.0 }, InputButton::Tertiary);

        assert!(damage.width() > 0);
        assert!(damage.height() > 0);

        let center_pixel = overlay.canvas.pixel_at(20, 30);
        assert_eq!(center_pixel, Color::TRANSPARENT);
    }

    #[test]
    fn begin_stroke_draws_circle_at_point() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::GREEN);

        overlay.on_pointer_button_pressed(Point { x: 32.0, y: 32.0 }, InputButton::Primary);
        let damage = overlay.begin_stroke(Point { x: 32.0, y: 32.0 });

        assert!(damage.width() > 0);
        assert!(damage.height() > 0);

        let p = overlay.canvas.pixel_at(32, 32);
        assert_eq!(p, Color::GREEN);
    }

    #[test]
    fn on_pointer_leave_clears_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        assert!(overlay.current_stroke.is_some());
        overlay.on_pointer_leave();
        assert!(overlay.current_stroke.is_none());
    }

    #[test]
    fn end_stroke_clears_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        assert!(overlay.current_stroke.is_some());
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        assert!(overlay.current_stroke.is_none());
    }

    #[test]
    fn on_size_changed_clears_canvas_and_returns_full_rect() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.canvas.fill(Color::RED);

        let rect = overlay.resize(TEST_WIDTH, TEST_HEIGHT);
        assert_eq!(rect.x(), 0);
        assert_eq!(rect.y(), 0);
        assert_eq!(rect.width(), TEST_WIDTH);
        assert_eq!(rect.height(), TEST_HEIGHT);
        assert_all_pixels_color(&mut overlay.canvas, Color::TRANSPARENT);
    }

    #[test]
    fn toggle_overlay_when_none_creates_overlay() {
        let mut app = MockApp::new();
        assert!(app.overlay.is_none());

        app.on_toggle_overlay();

        let overlay = app.overlay;
        assert!(overlay.is_some());
        assert!(overlay.unwrap().is_some());
    }

    #[test]
    fn toggle_overlay_when_pending_does_nothing() {
        let mut app = MockApp::with_pending();
        assert!(matches!(app.overlay_status(), OverlayStatus::Pending));

        app.on_toggle_overlay();

        assert!(matches!(app.overlay_status(), OverlayStatus::Pending));
    }

    #[test]
    fn toggle_overlay_when_ready_destroys_overlay() {
        let mut app = MockApp::with_overlay();
        assert!(matches!(app.overlay_status(), OverlayStatus::Ready));

        app.on_toggle_overlay();

        assert!(matches!(app.overlay_status(), OverlayStatus::None));
    }

    #[test]
    fn on_key_pressed_u_undoes_last_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 20.0, y: 10.0 }, InputButton::Primary);

        assert_eq!(overlay.history.len(), 1);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.history.is_empty());
        assert_all_pixels_color(&mut overlay.canvas, Color::TRANSPARENT);
    }

    #[test]
    fn on_key_pressed_c_then_u_restores_drawing() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 20.0, y: 10.0 }, InputButton::Primary);

        let (keep, redraw, shape) = overlay.on_key_pressed(Keysym::c);
        assert!(keep);
        assert!(redraw);
        assert_eq!(shape, CursorShape::Crosshair);
        assert_all_pixels_color(&mut overlay.canvas, Color::TRANSPARENT);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert_eq!(
            overlay.canvas.pixel_at(15, 10),
            Color::RED,
            "expected red pixel at (15, 10) after undoing clear"
        );
    }

    #[test]
    fn multiple_strokes_undo_in_order() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        overlay.primary_tool = Tool::Pen(Color::RED);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 20.0, y: 10.0 }, InputButton::Primary);
        let after_first = overlay.canvas.surface.data().unwrap().to_vec();

        overlay.primary_tool = Tool::Pen(Color::BLUE);
        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 50.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 50.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 50.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 20.0, y: 50.0 }, InputButton::Primary);

        assert_eq!(overlay.history.len(), 2);

        overlay.on_key_pressed(Keysym::u);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(
            overlay.canvas.surface.data().unwrap().to_vec(),
            after_first.as_slice()
        );

        overlay.on_key_pressed(Keysym::u);
        assert!(overlay.history.is_empty());
        assert_all_pixels_color(&mut overlay.canvas, Color::TRANSPARENT);
    }

    #[test]
    fn begin_stroke_starts_current_stroke() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::GREEN);

        overlay.on_pointer_button_pressed(Point { x: 15.0, y: 25.0 }, InputButton::Primary);
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
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(Point { x: 5.0, y: 5.0 }, InputButton::Primary);
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
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(Point { x: 5.0, y: 5.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 5.0, y: 5.0 });
        overlay.on_pointer_motion(Point { x: 15.0, y: 15.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 15.0, y: 15.0 }, InputButton::Primary);

        assert!(overlay.current_stroke.is_none());
        assert_eq!(overlay.history.len(), 1);
    }

    #[test]
    fn on_size_changed_clears_history() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        overlay.on_pointer_button_released(Point { x: 20.0, y: 10.0 }, InputButton::Primary);

        assert_eq!(overlay.history.len(), 1);

        overlay.resize(TEST_WIDTH, TEST_HEIGHT);

        assert!(overlay.history.is_empty());
    }

    #[test]
    fn on_key_pressed_period_fills_black_background() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::period);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::BLACK);
        assert_eq!(overlay.canvas.pixel_at(50, 40), Color::BLACK);
    }

    #[test]
    fn on_key_pressed_comma_fills_white_background() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::comma);
        assert!(keep);
        assert!(redraw);
        assert_eq!(overlay.history.len(), 1);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::WHITE);
        assert_eq!(overlay.canvas.pixel_at(50, 40), Color::WHITE);
    }

    #[test]
    fn fill_background_then_undo_restores_previous() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();
        overlay.primary_tool = Tool::Pen(Color::RED);

        overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Primary);
        overlay.begin_stroke(Point { x: 10.0, y: 10.0 });
        overlay.on_pointer_motion(Point { x: 20.0, y: 10.0 });
        overlay.end_stroke();
        let buf_after_draw: Vec<u8> = overlay.canvas.surface.data().unwrap().to_vec();

        overlay.on_key_pressed(Keysym::period);
        assert_eq!(overlay.canvas.pixel_at(0, 0), Color::BLACK);

        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::u);
        assert!(keep);
        assert!(redraw);
        assert_eq!(
            overlay.canvas.surface.data().unwrap().to_vec(),
            buf_after_draw.as_slice()
        );
    }

    #[test]
    fn right_mouse_button_opens_context_menu() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        let (keep, redraw, _) =
            overlay.on_pointer_button_pressed(Point { x: 10.0, y: 10.0 }, InputButton::Secondary);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.ui.is_context_menu_open());

        // hover some menu item
        let rect = overlay.ui.context_menu_rect().unwrap();
        let point = Point {
            x: rect.x() as f64 + rect.width() as f64 - 5.0,
            y: rect.y() as f64 + 5.0,
        };
        let damage = overlay.on_pointer_motion(point).unwrap_or_else(|| {
            panic!("expected Some from pointer motion while context menu is open")
        });
        assert!(damage.width() > 0);

        // Release the button.
        // Some op should be triggered, and the menu should close.
        let (keep, redraw, _) = overlay.on_pointer_button_released(point, InputButton::Secondary);
        assert!(keep);
        assert!(redraw);
        assert!(!overlay.ui.is_context_menu_open());
    }

    #[test]
    fn menu_can_be_used_with_keyboard() {
        let mut overlay = OverlayState::new(TEST_WIDTH, TEST_HEIGHT).unwrap();

        // Open the menu with space.
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::space);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.ui.is_context_menu_open());

        // Focus Clear and apply it.
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Up);
        assert!(keep);
        assert!(redraw);
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Up);
        assert!(keep);
        assert!(redraw);
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Up);
        assert!(keep);
        assert!(redraw);
        assert!(matches!(
            overlay.ui.get_menu_selection(),
            Some(Action::Clear)
        ));
        let (keep, redraw, _) = overlay.on_key_pressed(Keysym::Return);
        assert!(keep);
        assert!(redraw);
        assert!(!overlay.ui.is_context_menu_open());
        assert!(matches!(
            overlay.history.last(),
            Some(HistoryItem::Clear(_))
        ));
    }
}
