use cairo::{Context, ImageSurface, ImageSurfaceData, RectangleInt};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { a: 255, r, g, b }
    }

    pub const RED: Color = Self::rgb(255, 0, 0);
    pub const GREEN: Color = Self::rgb(0, 255, 0);
    pub const BLUE: Color = Self::rgb(0, 0, 255);
    pub const YELLOW: Color = Self::rgb(255, 255, 0);
    pub const MAGENTA: Color = Self::rgb(255, 0, 255);
    pub const CYAN: Color = Self::rgb(0, 255, 255);
    pub const BLACK: Color = Self::rgb(0, 0, 0);
    pub const WHITE: Color = Self::rgb(255, 255, 255);
    pub const TRANSPARENT: Color = Self {
        a: 0,
        r: 0,
        g: 0,
        b: 0,
    };
}

pub(crate) struct Canvas {
    pub(crate) surface: ImageSurface,
}

impl Canvas {
    pub fn new(width: i32, height: i32) -> Self {
        let surface = ImageSurface::create(cairo::Format::ARgb32, width, height)
            .expect("failed to create canvas surface");
        Self { surface }
    }

    pub fn width(&self) -> i32 {
        self.surface.width()
    }

    pub fn height(&self) -> i32 {
        self.surface.height()
    }

    #[cfg(test)]
    pub fn pixel_at(&mut self, x: i32, y: i32) -> Color {
        let stride = self.surface.stride();
        let data = self.surface.data().expect("failed to get surface data");
        let offset = ((y * stride) + (x * 4)) as usize;
        // data is stored as ARGB but in native endianness
        let value = u32::from_ne_bytes(
            data[offset..offset + 4]
                .try_into()
                .expect("failed to read pixel data"),
        );
        Color {
            a: ((value >> 24) & 0xFF) as u8,
            r: ((value >> 16) & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: (value & 0xFF) as u8,
        }
    }

    fn set_source_rgba(ctx: &Context, color: Color) {
        ctx.set_source_rgba(
            color.r as f64 / 255.0,
            color.g as f64 / 255.0,
            color.b as f64 / 255.0,
            color.a as f64 / 255.0,
        );
        ctx.set_operator(cairo::Operator::Source);
    }

    pub fn clear(&mut self) -> RectangleInt {
        let ctx = Context::new(&self.surface).expect("failed to create canvas context");
        Self::set_source_rgba(&ctx, Color::TRANSPARENT);
        ctx.paint().expect("failed to fill canvas");
        RectangleInt::new(0, 0, self.width(), self.height())
    }

    pub fn fill(&mut self, color: Color) -> RectangleInt {
        let ctx = Context::new(&self.surface).expect("failed to create canvas context");
        Self::set_source_rgba(&ctx, color);
        ctx.paint().expect("failed to fill canvas");
        RectangleInt::new(0, 0, self.width(), self.height())
    }

    fn extents_to_rect(&self, extents: (f64, f64, f64, f64)) -> RectangleInt {
        let (x1, y1, x2, y2) = extents;
        RectangleInt::new(
            (x1.floor() as i32).max(0),
            (y1.floor() as i32).max(0),
            ((x2 - x1).ceil() as i32).min(self.width()),
            ((y2 - y1).ceil() as i32).min(self.height()),
        )
    }

    pub fn draw_circle(&mut self, center: Point, radius: f64, color: Color) -> RectangleInt {
        let ctx = Context::new(&self.surface).expect("failed to create canvas context");
        Self::set_source_rgba(&ctx, color);
        ctx.arc(center.x, center.y, radius, 0.0, std::f64::consts::TAU);
        let extents = ctx.fill_extents().expect("failed to get circle extents");
        ctx.fill().expect("failed to fill circle");
        self.extents_to_rect(extents)
    }

    pub fn draw_line(&mut self, from: Point, to: Point, radius: f64, color: Color) -> RectangleInt {
        if from == to {
            return self.draw_circle(from, radius, color);
        }
        let ctx = Context::new(&self.surface).expect("failed to create canvas context");
        Self::set_source_rgba(&ctx, color);
        ctx.set_line_width(radius * 2.0);
        ctx.set_line_cap(cairo::LineCap::Round);
        ctx.new_path();
        ctx.move_to(from.x, from.y);
        ctx.line_to(to.x, to.y);
        let extents = ctx.stroke_extents().expect("failed to get line extents");
        ctx.stroke().expect("failed to stroke line");
        self.extents_to_rect(extents)
    }

    pub(crate) fn surface_data(&'_ mut self) -> ImageSurfaceData<'_> {
        self.surface.data().expect("failed to get surface data")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------
    // Canvas::clear
    // -------------------------------------------------------

    #[test]
    fn clear_fills_buffer_with_zeros_and_returns_full_rect() {
        let w = 10;
        let h = 8;
        let mut canvas = Canvas::new(w, h);

        canvas.fill(Color::RED);

        let damage = canvas.clear();
        assert_eq!(damage.x(), 0);
        assert_eq!(damage.y(), 0);
        assert_eq!(damage.width(), w);
        assert_eq!(damage.height(), h);

        for y in 0..h {
            for x in 0..w {
                assert_eq!(canvas.pixel_at(x, y), Color::TRANSPARENT);
            }
        }
    }

    // -------------------------------------------------------
    // Canvas::fill
    // -------------------------------------------------------

    #[test]
    fn fill_sets_all_pixels_to_given_color_and_returns_full_rect() {
        let w = 10;
        let h = 8;
        let mut canvas = Canvas::new(w, h);

        let damage = canvas.fill(Color::RED);
        assert_eq!(damage.x(), 0);
        assert_eq!(damage.y(), 0);
        assert_eq!(damage.width(), w);
        assert_eq!(damage.height(), h);

        for y in 0..h {
            for x in 0..w {
                assert_eq!(canvas.pixel_at(x, y), Color::RED);
            }
        }
    }

    // -------------------------------------------------------
    // Canvas::draw_circle
    // -------------------------------------------------------

    #[test]
    fn draw_circle_center_pixel_is_filled() {
        let w = 20;
        let h = 20;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 10.0, y: 10.0 };
        let radius = 5.0;
        canvas.draw_circle(center, radius, Color::RED);

        assert_eq!(canvas.pixel_at(10, 10), Color::RED);
    }

    #[test]
    fn draw_circle_pixels_at_cardinal_offsets_within_radius_are_filled() {
        let w = 30;
        let h = 30;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 15.0, y: 15.0 };
        let radius = 5.0;
        canvas.draw_circle(center, radius, Color::RED);

        // Points along axes at distance 2 from center — well within radius 5 — should be fully covered.
        assert_eq!(canvas.pixel_at(15, 13), Color::RED); // 2 pixels above center
        assert_eq!(canvas.pixel_at(15, 17), Color::RED); // 2 pixels below center
        assert_eq!(canvas.pixel_at(13, 15), Color::RED); // 2 pixels left
        assert_eq!(canvas.pixel_at(17, 15), Color::RED); // 2 pixels right
    }

    #[test]
    fn draw_circle_pixels_well_outside_radius_are_transparent() {
        let w = 30;
        let h = 30;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 15.0, y: 15.0 };
        let radius = 5.0;
        canvas.draw_circle(center, radius, Color::RED);

        assert_eq!(canvas.pixel_at(0, 0), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(15, 0), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(29, 29), Color::TRANSPARENT);
    }

    #[test]
    fn draw_circle_returns_correct_damage_rect() {
        let w = 30;
        let h = 30;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 15.0, y: 15.0 };
        let radius = 5.0;
        let damage = canvas.draw_circle(center, radius, Color::RED);

        assert!(damage.x() <= 10);
        assert!(damage.y() <= 10);
        assert!(damage.x() + damage.width() >= 20);
        assert!(damage.y() + damage.height() >= 20);
    }

    #[test]
    fn draw_circle_clipped_at_edge() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 1.0, y: 1.0 };
        let radius = 5.0;
        let damage = canvas.draw_circle(center, radius, Color::BLUE);

        assert!(damage.x() >= 0);
        assert!(damage.y() >= 0);
        assert!(damage.x() + damage.width() <= w);
        assert!(damage.y() + damage.height() <= h);

        assert_eq!(canvas.pixel_at(1, 1), Color::BLUE);
    }

    // -------------------------------------------------------
    // Canvas::draw_line
    // -------------------------------------------------------

    #[test]
    fn draw_line_horizontal_fills_pixels_along_path() {
        let w = 30;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        let from = Point { x: 5.0, y: 5.0 };
        let to = Point { x: 25.0, y: 5.0 };
        let radius = 1.5;
        canvas.draw_line(from, to, radius, Color::RED);

        for x in 5..=25 {
            assert_eq!(
                canvas.pixel_at(x, 5),
                Color::RED,
                "pixel ({x}, 5) on line should be RED"
            );
        }
    }

    #[test]
    fn draw_line_vertical_fills_pixels_along_path() {
        let w = 10;
        let h = 30;
        let mut canvas = Canvas::new(w, h);

        let from = Point { x: 5.0, y: 3.0 };
        let to = Point { x: 5.0, y: 20.0 };
        let radius = 1.5;
        canvas.draw_line(from, to, radius, Color::BLUE);

        for y in 3..=20 {
            assert_eq!(
                canvas.pixel_at(5, y),
                Color::BLUE,
                "pixel (5, {y}) on line should be BLUE"
            );
        }
    }

    #[test]
    fn draw_line_returns_correct_damage_rect() {
        let w = 40;
        let h = 40;
        let mut canvas = Canvas::new(w, h);

        let from = Point { x: 5.0, y: 10.0 };
        let to = Point { x: 30.0, y: 25.0 };
        let radius = 2.0;
        let damage = canvas.draw_line(from, to, radius, Color::RED);

        assert!(damage.x() <= 3, "damage.x={} should be <= 3", damage.x());
        assert!(damage.y() <= 8, "damage.y={} should be <= 8", damage.y());
        assert!(
            damage.x() + damage.width() >= 32,
            "damage right edge {} should be >= 32",
            damage.x() + damage.width()
        );
        assert!(
            damage.y() + damage.height() >= 27,
            "damage bottom edge {} should be >= 27",
            damage.y() + damage.height()
        );
    }

    #[test]
    fn draw_line_single_point() {
        let w = 20;
        let h = 20;
        let mut canvas = Canvas::new(w, h);

        let p = Point { x: 10.0, y: 10.0 };
        let radius = 3.0;
        let damage = canvas.draw_line(p, p, radius, Color::RED);

        assert_eq!(canvas.pixel_at(10, 10), Color::RED);
        assert!(damage.width() > 0);
        assert!(damage.height() > 0);
    }
}
