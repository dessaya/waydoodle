use smithay_client_toolkit::seat::keyboard::Keysym;

use crate::canvas::{Canvas, Point, Rect};

pub(crate) struct PointerState {
    pub pos: Point,
    pub pressed: bool,
}

impl PointerState {
    pub fn new() -> Self {
        Self {
            pos: Point { x: 0.0, y: 0.0 },
            pressed: false,
        }
    }
}

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

const ZERO_RECT: Rect = Rect {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
};

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

    fn on_pointer_enter(&mut self, pointer: &mut PointerState, pos: Point) {
        pointer.pos = pos;
    }

    fn on_pointer_leave(&mut self, pointer: &mut PointerState) {
        pointer.pressed = false;
    }

    fn on_pointer_press(&mut self, pointer: &mut PointerState, pos: Point) -> Rect {
        pointer.pressed = true;
        pointer.pos = pos;
        let radius = self.current_tool().brush_radius();
        let pixel = self.current_tool().pixel_color();
        self.back_canvas()
            .map(|mut c| c.draw_circle(pos, radius, pixel))
            .unwrap_or(ZERO_RECT)
    }

    fn on_pointer_release(&mut self, pointer: &mut PointerState) {
        pointer.pressed = false;
    }

    fn on_pointer_motion(&mut self, pointer: &mut PointerState, pos: Point) -> Option<Rect> {
        let prev = pointer.pos;
        let pressed = pointer.pressed;
        pointer.pos = pos;
        if pressed {
            let radius = self.current_tool().brush_radius();
            let pixel = self.current_tool().pixel_color();
            Some(
                self.back_canvas()
                    .map(|mut c| c.draw_line(prev, pos, radius, pixel))
                    .unwrap_or(ZERO_RECT),
            )
        } else {
            None
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::{Canvas, Point};

    const TEST_WIDTH: u32 = 64;
    const TEST_HEIGHT: u32 = 64;

    struct MockOverlay {
        buf: Vec<u8>,
        width: u32,
        height: u32,
        tool: Tool,
        help: bool,
    }

    impl MockOverlay {
        fn new(width: u32, height: u32) -> Self {
            Self {
                buf: vec![0u8; (width * height * 4) as usize],
                width,
                height,
                tool: DEFAULT_TOOL,
                help: false,
            }
        }
    }

    impl OverlayCanvas for MockOverlay {
        fn back_canvas(&mut self) -> Option<Canvas<'_>> {
            Some(Canvas {
                buf: &mut self.buf,
                width: self.width,
                height: self.height,
            })
        }
    }

    impl OverlayTool for MockOverlay {
        fn current_tool(&self) -> Tool {
            self.tool
        }

        fn set_current_tool(&mut self, tool: Tool) {
            self.tool = tool;
        }
    }

    impl OverlayHelp for MockOverlay {
        fn show_help(&self) -> bool {
            self.help
        }

        fn set_show_help(&mut self, show: bool) {
            self.help = show;
        }
    }

    struct MockApp {
        overlay: Option<Option<MockOverlay>>,
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
                overlay: Some(Some(MockOverlay::new(TEST_WIDTH, TEST_HEIGHT))),
            }
        }
    }

    impl App<MockOverlay> for MockApp {
        fn create_overlay(&mut self) {
            self.overlay = Some(Some(MockOverlay::new(TEST_WIDTH, TEST_HEIGHT)));
        }

        fn destroy_overlay(&mut self) {
            self.overlay = None;
        }

        fn get_overlay(&self) -> Option<Option<&MockOverlay>> {
            match &self.overlay {
                None => None,
                Some(None) => Some(None),
                Some(Some(o)) => Some(Some(o)),
            }
        }
    }

    fn pixel_at(buf: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
        let offset = (y as usize * width as usize + x as usize) * 4;
        [
            buf[offset],
            buf[offset + 1],
            buf[offset + 2],
            buf[offset + 3],
        ]
    }

    // ── Tool tests ──

    #[test]
    fn pen_brush_radius_is_1_5() {
        assert_eq!(Tool::Pen(RED).brush_radius(), 1.5);
    }

    #[test]
    fn eraser_brush_radius_is_10() {
        assert_eq!(Tool::Eraser.brush_radius(), 10.0);
    }

    #[test]
    fn pen_cursor_shape_is_crosshair() {
        assert_eq!(Tool::Pen(RED).cursor_shape(), CursorShape::Crosshair);
    }

    #[test]
    fn eraser_cursor_shape_is_circle() {
        assert_eq!(Tool::Eraser.cursor_shape(), CursorShape::Circle);
    }

    #[test]
    fn pen_pixel_color_returns_its_color() {
        assert_eq!(Tool::Pen(RED).pixel_color(), RED);
        assert_eq!(Tool::Pen(BLUE).pixel_color(), BLUE);
        assert_eq!(Tool::Pen(GREEN).pixel_color(), GREEN);
    }

    #[test]
    fn eraser_pixel_color_returns_transparent() {
        assert_eq!(Tool::Eraser.pixel_color(), [0, 0, 0, 0]);
    }

    #[test]
    fn default_tool_is_red_pen() {
        assert_eq!(DEFAULT_TOOL, Tool::Pen(RED));
    }

    // ── ToolInfo::swatch tests ──

    #[test]
    fn swatch_returns_some_for_pen_entries() {
        for info in ALL_KEYS {
            match info.action {
                KeyAction::SetTool(Tool::Pen(color)) => {
                    assert_eq!(info.swatch(), Some(u32::from_le_bytes(color)));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn swatch_returns_none_for_non_pen_entries() {
        for info in ALL_KEYS {
            match info.action {
                KeyAction::SetTool(Tool::Pen(_)) => {}
                _ => {
                    assert_eq!(info.swatch(), None);
                }
            }
        }
    }

    // ── ALL_KEYS tests ──

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
            (Keysym::r, KeyAction::SetTool(Tool::Pen(RED))),
            (Keysym::g, KeyAction::SetTool(Tool::Pen(GREEN))),
            (Keysym::b, KeyAction::SetTool(Tool::Pen(BLUE))),
            (Keysym::y, KeyAction::SetTool(Tool::Pen(YELLOW))),
            (Keysym::m, KeyAction::SetTool(Tool::Pen(MAGENTA))),
            (Keysym::n, KeyAction::SetTool(Tool::Pen(CYAN))),
            (Keysym::e, KeyAction::SetTool(Tool::Eraser)),
            (Keysym::c, KeyAction::Clear),
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

    // ── Overlay::on_key_pressed tests ──

    #[test]
    fn on_key_pressed_r_sets_red_pen() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.tool = Tool::Eraser;
        let (keep, redraw) = overlay.on_key_pressed(Keysym::r);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.tool, Tool::Pen(RED));
    }

    #[test]
    fn on_key_pressed_e_sets_eraser() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let (keep, redraw) = overlay.on_key_pressed(Keysym::e);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.tool, Tool::Eraser);
    }

    #[test]
    fn on_key_pressed_c_clears_canvas() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        // Draw something first so the buffer is non-zero
        overlay.buf[0..4].copy_from_slice(&RED);
        overlay.buf[100..104].copy_from_slice(&BLUE);

        let (keep, redraw) = overlay.on_key_pressed(Keysym::c);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn on_key_pressed_f1_toggles_help() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        assert!(!overlay.help);

        let (keep, redraw) = overlay.on_key_pressed(Keysym::F1);
        assert!(keep);
        assert!(redraw);
        assert!(overlay.help);
    }

    #[test]
    fn on_key_pressed_escape_returns_hide() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let (keep, redraw) = overlay.on_key_pressed(Keysym::Escape);
        assert!(!keep);
        assert!(!redraw);
    }

    #[test]
    fn on_key_pressed_unbound_key_changes_nothing() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let original_tool = overlay.tool;
        let original_help = overlay.help;

        let (keep, redraw) = overlay.on_key_pressed(Keysym::z);
        assert!(keep);
        assert!(!redraw);
        assert_eq!(overlay.tool, original_tool);
        assert_eq!(overlay.help, original_help);
    }

    // ── Overlay::on_pointer_motion tests ──

    #[test]
    fn on_pointer_motion_with_pen_draws_pixels() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.tool = Tool::Pen(RED);
        let mut ptr = PointerState::new();
        overlay.on_pointer_press(&mut ptr, Point { x: 10.0, y: 10.0 });

        let damage = overlay.on_pointer_motion(&mut ptr, Point { x: 20.0, y: 10.0 });
        let damage = damage.expect("expected damage from motion while pressed");

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        // Check that at least some pixels along the path are red
        let mut found_red = false;
        for x in 10..=20 {
            if pixel_at(&overlay.buf, TEST_WIDTH, x, 10) == RED {
                found_red = true;
                break;
            }
        }
        assert!(found_red, "expected red pixels along the drag path");
    }

    #[test]
    fn on_pointer_motion_while_not_pressed_returns_none() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let mut ptr = PointerState::new();

        let damage = overlay.on_pointer_motion(&mut ptr, Point { x: 20.0, y: 10.0 });
        assert!(damage.is_none());
    }

    #[test]
    fn on_pointer_motion_with_eraser_clears_pixels() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        // First paint a horizontal stripe with red
        overlay.tool = Tool::Pen(RED);
        let mut ptr = PointerState::new();
        overlay.on_pointer_press(&mut ptr, Point { x: 10.0, y: 30.0 });
        overlay.on_pointer_motion(&mut ptr, Point { x: 30.0, y: 30.0 });
        overlay.on_pointer_release(&mut ptr);

        // Now erase along the same path
        overlay.tool = Tool::Eraser;
        overlay.on_pointer_press(&mut ptr, Point { x: 10.0, y: 30.0 });
        let damage = overlay.on_pointer_motion(&mut ptr, Point { x: 30.0, y: 30.0 });
        let damage = damage.expect("expected damage from eraser motion");

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        // The center of the eraser path should be transparent
        let center_pixel = pixel_at(&overlay.buf, TEST_WIDTH, 20, 30);
        assert_eq!(center_pixel, [0, 0, 0, 0]);
    }

    // ── Overlay::on_pointer_press tests ──

    #[test]
    fn on_pointer_press_draws_circle_at_point() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.tool = Tool::Pen(GREEN);
        let mut ptr = PointerState::new();

        let damage = overlay.on_pointer_press(&mut ptr, Point { x: 32.0, y: 32.0 });

        assert!(damage.width > 0);
        assert!(damage.height > 0);

        // The center pixel should be green
        let p = pixel_at(&overlay.buf, TEST_WIDTH, 32, 32);
        assert_eq!(p, GREEN);
    }

    // ── Overlay pointer state tests ──

    #[test]
    fn on_pointer_leave_resets_pressed() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let mut ptr = PointerState::new();
        overlay.on_pointer_press(&mut ptr, Point { x: 10.0, y: 10.0 });
        assert!(ptr.pressed);
        overlay.on_pointer_leave(&mut ptr);
        assert!(!ptr.pressed);
    }

    #[test]
    fn on_pointer_release_resets_pressed() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let mut ptr = PointerState::new();
        overlay.on_pointer_press(&mut ptr, Point { x: 10.0, y: 10.0 });
        assert!(ptr.pressed);
        overlay.on_pointer_release(&mut ptr);
        assert!(!ptr.pressed);
    }

    #[test]
    fn on_pointer_enter_updates_pos() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        let mut ptr = PointerState::new();
        overlay.on_pointer_enter(&mut ptr, Point { x: 42.0, y: 17.0 });
        assert_eq!(ptr.pos.x, 42.0);
        assert_eq!(ptr.pos.y, 17.0);
    }

    // ── Overlay::on_size_changed tests ──

    #[test]
    fn on_size_changed_clears_canvas_and_returns_full_rect() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        // Paint something
        overlay.buf[0..4].copy_from_slice(&RED);

        let rect = overlay.on_size_changed();
        let rect = rect.unwrap();
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.width, TEST_WIDTH as i32);
        assert_eq!(rect.height, TEST_HEIGHT as i32);
        assert!(overlay.buf.iter().all(|&b| b == 0));
    }

    // ── OverlayHelp::on_toggle_help tests ──

    #[test]
    fn toggle_help_false_to_true() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        assert!(!overlay.show_help());
        overlay.on_toggle_help();
        assert!(overlay.show_help());
    }

    #[test]
    fn toggle_help_true_to_false() {
        let mut overlay = MockOverlay::new(TEST_WIDTH, TEST_HEIGHT);
        overlay.help = true;
        assert!(overlay.show_help());
        overlay.on_toggle_help();
        assert!(!overlay.show_help());
    }

    // ── App::on_toggle_overlay tests ──

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
}
