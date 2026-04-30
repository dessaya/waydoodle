use bdf_reader::Font;

pub(crate) const GLYPH_W: u32 = 8;
pub(crate) const GLYPH_H: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width: width as i32,
            height: height as i32,
        }
    }

    pub fn zero() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn merge(self, other: Rect) -> Self {
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.width).max(other.x + other.width);
        let y1 = (self.y + self.height).max(other.y + other.height);
        Self {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }
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

    pub fn argb_le(&self) -> [u8; 4] {
        [self.b, self.g, self.r, self.a]
    }

    pub(crate) const fn from_u32(argb: u32) -> Color {
        Color {
            a: ((argb >> 24) & 0xFF) as u8,
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }
}

pub(crate) struct Canvas {
    pub buf: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Self {
        let buf = vec![0u8; (width * height * 4) as usize];
        Self { buf, width, height }
    }

    #[cfg(test)]
    pub fn pixel_at(&self, x: u32, y: u32) -> Color {
        let offset = (y as usize * self.width as usize + x as usize) * 4;
        Color {
            b: self.buf[offset],
            g: self.buf[offset + 1],
            r: self.buf[offset + 2],
            a: self.buf[offset + 3],
        }
    }

    pub fn clear(&mut self) -> Rect {
        self.buf.fill(0);
        Rect::new(self.width, self.height)
    }

    pub fn fill(&mut self, color: Color) -> Rect {
        let argb = color.argb_le();
        for chunk in self.buf.chunks_exact_mut(4) {
            chunk.copy_from_slice(&argb);
        }
        Rect::new(self.width, self.height)
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: Color) {
        let width = self.width;
        let height = self.height;
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let offset = (y as usize * width as usize + x as usize) * 4;
        self.buf[offset..offset + 4].copy_from_slice(&color.argb_le());
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) -> Rect {
        let width = self.width;
        let height = self.height;

        let x0 = x;
        let y0 = y;
        let x1 = x + w as i32;
        let y1 = y + h as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                self.set_pixel(x, y, color);
            }
        }

        let x = x0.max(0);
        let y = y0.max(0);
        let x1 = x1.min(width as i32);
        let y1 = y1.min(height as i32);
        Rect {
            x,
            y,
            width: (x1 - x).max(0),
            height: (y1 - y).max(0),
        }
    }

    pub fn draw_circle(&mut self, center: Point, radius: f64, color: Color) -> Rect {
        let width = self.width;
        let height = self.height;

        let r = radius.ceil() as i32;
        let cx_i = center.x.round() as i32;
        let cy_i = center.y.round() as i32;
        let r_sq = radius * radius;
        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = (dx as f64 - (center.x - cx_i as f64)).powi(2)
                    + (dy as f64 - (center.y - cy_i as f64)).powi(2);
                if dist_sq <= r_sq {
                    self.set_pixel(cx_i + dx, cy_i + dy, color);
                }
            }
        }

        let x = (center.x - radius).floor().max(0.0) as i32;
        let y = (center.y - radius).floor().max(0.0) as i32;
        let x1 = (center.x + radius).ceil().min(width as f64) as i32;
        let y1 = (center.y + radius).ceil().min(height as f64) as i32;
        Rect {
            x,
            y,
            width: (x1 - x).max(0),
            height: (y1 - y).max(0),
        }
    }

    pub fn draw_border(&mut self, x: i32, y: i32, w: u32, h: u32, color: Color) {
        self.draw_rect(x, y, w, 1, color);
        self.draw_rect(x, y + h as i32 - 1, w, 1, color);
        self.draw_rect(x, y, 1, h, color);
        self.draw_rect(x + w as i32 - 1, y, 1, h, color);
    }

    pub fn draw_line(&mut self, from: Point, to: Point, radius: f64, color: Color) -> Rect {
        let width = self.width;
        let height = self.height;

        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dist = dx.hypot(dy);
        let steps = (dist / 0.5).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let center = Point {
                x: from.x + dx * t,
                y: from.y + dy * t,
            };
            self.draw_circle(center, radius, color);
        }

        let min_x = from.x.min(to.x) - radius;
        let min_y = from.y.min(to.y) - radius;
        let max_x = from.x.max(to.x) + radius;
        let max_y = from.y.max(to.y) + radius;
        let x = min_x.floor().max(0.0) as i32;
        let y = min_y.floor().max(0.0) as i32;
        let x1 = max_x.ceil().min(width as f64) as i32;
        let y1 = max_y.ceil().min(height as f64) as i32;
        Rect {
            x,
            y,
            width: (x1 - x).max(0),
            height: (y1 - y).max(0),
        }
    }

    pub fn draw_text(&mut self, font: &Font, text: &str, x: i32, y: i32, color: Color) {
        let font_bb = font.bounding_box();
        let mut cursor_x = x;
        for ch in text.chars() {
            if let Some(glyph) = font.glyph(ch) {
                let bb = glyph.bounding_box();
                let bitmap = glyph.bitmap();
                for row in 0..bb.height as usize {
                    for col in 0..bb.width as usize {
                        if bitmap.get(col, row).unwrap_or(false) {
                            let px = cursor_x + bb.offset_x + col as i32;
                            let py = y
                                + (font_bb.height as i32 - bb.offset_y - bb.height as i32)
                                + row as i32;
                            self.set_pixel(px, py, color);
                        }
                    }
                }
                let advance = glyph
                    .dwidth()
                    .map(|(dx, _)| dx as i32)
                    .unwrap_or(GLYPH_W as i32);
                cursor_x += advance;
            } else {
                cursor_x += GLYPH_W as i32;
            }
        }
    }

    pub fn text_width(text: &str) -> u32 {
        text.len() as u32 * GLYPH_W
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------
    // Rect::new
    // -------------------------------------------------------

    #[test]
    fn rect_new_sets_origin_and_dimensions() {
        let r = Rect::new(100, 200);
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.width, 100);
        assert_eq!(r.height, 200);
    }

    #[test]
    fn rect_new_zero_size() {
        let r = Rect::new(0, 0);
        assert_eq!(r.x, 0);
        assert_eq!(r.y, 0);
        assert_eq!(r.width, 0);
        assert_eq!(r.height, 0);
    }

    // -------------------------------------------------------
    // Rect::merge
    // -------------------------------------------------------

    #[test]
    fn rect_merge_overlapping() {
        let a = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        };
        let b = Rect {
            x: 5,
            y: 5,
            width: 10,
            height: 10,
        };
        let m = a.merge(b);
        assert_eq!(m.x, 0);
        assert_eq!(m.y, 0);
        assert_eq!(m.width, 15);
        assert_eq!(m.height, 15);
    }

    #[test]
    fn rect_merge_non_overlapping() {
        let a = Rect {
            x: 0,
            y: 0,
            width: 5,
            height: 5,
        };
        let b = Rect {
            x: 20,
            y: 30,
            width: 10,
            height: 10,
        };
        let m = a.merge(b);
        assert_eq!(m.x, 0);
        assert_eq!(m.y, 0);
        assert_eq!(m.width, 30);
        assert_eq!(m.height, 40);
    }

    #[test]
    fn rect_merge_one_contains_the_other() {
        let outer = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let inner = Rect {
            x: 10,
            y: 10,
            width: 20,
            height: 20,
        };
        let m = outer.merge(inner);
        assert_eq!(m.x, 0);
        assert_eq!(m.y, 0);
        assert_eq!(m.width, 100);
        assert_eq!(m.height, 100);

        // Merge in the other direction should give the same result.
        let m2 = inner.merge(outer);
        assert_eq!(m2.x, 0);
        assert_eq!(m2.y, 0);
        assert_eq!(m2.width, 100);
        assert_eq!(m2.height, 100);
    }

    #[test]
    fn rect_merge_with_negative_coords() {
        let a = Rect {
            x: -5,
            y: -5,
            width: 10,
            height: 10,
        };
        let b = Rect {
            x: 3,
            y: 3,
            width: 10,
            height: 10,
        };
        let m = a.merge(b);
        assert_eq!(m.x, -5);
        assert_eq!(m.y, -5);
        assert_eq!(m.width, 18);
        assert_eq!(m.height, 18);
    }

    // -------------------------------------------------------
    // Canvas::clear
    // -------------------------------------------------------

    #[test]
    fn clear_fills_buffer_with_zeros_and_returns_full_rect() {
        let w = 10;
        let h = 8;
        let mut canvas = Canvas::new(w, h);

        // Dirty some pixels first.
        canvas.set_pixel(3, 4, Color::RED);
        canvas.set_pixel(0, 0, Color::BLUE);

        let damage = canvas.clear();
        assert_eq!(damage.x, 0);
        assert_eq!(damage.y, 0);
        assert_eq!(damage.width, w as i32);
        assert_eq!(damage.height, h as i32);

        // Every byte must be zero.
        assert!(canvas.buf.iter().all(|&b| b == 0));
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
        assert_eq!(damage.x, 0);
        assert_eq!(damage.y, 0);
        assert_eq!(damage.width, w as i32);
        assert_eq!(damage.height, h as i32);

        // Every color must be RED.
        for y in 0..h {
            for x in 0..w {
                assert_eq!(canvas.pixel_at(x, y), Color::RED);
            }
        }
    }

    // -------------------------------------------------------
    // Canvas::set_pixel (tested indirectly via draw_rect)
    // -------------------------------------------------------

    #[test]
    fn set_pixel_ignores_negative_x() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set_pixel(-1, 0, Color::RED);
        // Buffer must still be all zeros.
        assert!(canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_pixel_ignores_negative_y() {
        let mut canvas = Canvas::new(4, 4);
        canvas.set_pixel(0, -1, Color::RED);
        assert!(canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_pixel_ignores_x_beyond_width() {
        let w = 4;
        let mut canvas = Canvas::new(w, 4);
        canvas.set_pixel(w as i32, 0, Color::RED);
        assert!(canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_pixel_ignores_y_beyond_height() {
        let h = 4;
        let mut canvas = Canvas::new(4, h);
        canvas.set_pixel(0, h as i32, Color::RED);
        assert!(canvas.buf.iter().all(|&b| b == 0));
    }

    #[test]
    fn set_pixel_valid_writes_correct_bytes() {
        let w = 4;
        let mut canvas = Canvas::new(w, 4);
        canvas.set_pixel(2, 3, Color::RED);
        assert_eq!(canvas.pixel_at(2, 3), Color::RED);
        // Neighbouring color must be untouched.
        assert_eq!(canvas.pixel_at(1, 3), Color::TRANSPARENT);
    }

    // -------------------------------------------------------
    // Canvas::draw_rect
    // -------------------------------------------------------

    #[test]
    fn draw_rect_fills_pixels_and_returns_damage() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        let damage = canvas.draw_rect(2, 3, 4, 5, Color::RED);

        // Damage rect should match the drawn area.
        assert_eq!(damage.x, 2);
        assert_eq!(damage.y, 3);
        assert_eq!(damage.width, 4);
        assert_eq!(damage.height, 5);

        // Pixels inside the rect should be RED.
        for y in 3..8 {
            for x in 2..6 {
                assert_eq!(
                    canvas.pixel_at(x, y),
                    Color::RED,
                    "color ({x}, {y}) should be RED"
                );
            }
        }

        // A color just outside should be transparent.
        assert_eq!(canvas.pixel_at(1, 3), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(6, 3), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(2, 2), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(2, 8), Color::TRANSPARENT);
    }

    #[test]
    fn draw_rect_partially_out_of_bounds_clips_damage() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        // Rect extends beyond the right and bottom edges.
        let damage = canvas.draw_rect(8, 7, 5, 6, Color::BLUE);

        // Damage should be clipped to canvas bounds.
        assert_eq!(damage.x, 8);
        assert_eq!(damage.y, 7);
        assert_eq!(damage.width, 2); // 10 - 8
        assert_eq!(damage.height, 3); // 10 - 7

        // Pixels within canvas bounds should be written.
        assert_eq!(canvas.pixel_at(8, 7), Color::BLUE);
        assert_eq!(canvas.pixel_at(9, 9), Color::BLUE);
    }

    #[test]
    fn draw_rect_negative_origin_clips_damage() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        let damage = canvas.draw_rect(-2, -3, 5, 6, Color::RED);

        // Damage rect clipped to canvas: starts at (0,0), extends to (3,3).
        assert_eq!(damage.x, 0);
        assert_eq!(damage.y, 0);
        assert_eq!(damage.width, 3); // (-2 + 5) = 3
        assert_eq!(damage.height, 3); // (-3 + 6) = 3

        // The visible portion should have RED pixels.
        assert_eq!(canvas.pixel_at(0, 0), Color::RED);
        assert_eq!(canvas.pixel_at(2, 2), Color::RED);
        // Just outside the rect.
        assert_eq!(canvas.pixel_at(3, 0), Color::TRANSPARENT);
    }

    #[test]
    fn draw_rect_fully_out_of_bounds_returns_zero_damage() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        let damage = canvas.draw_rect(-10, -10, 5, 5, Color::RED);
        assert_eq!(damage.width, 0);
        assert_eq!(damage.height, 0);

        // Nothing should have been written.
        assert!(canvas.buf.iter().all(|&b| b == 0));
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

        // The center color must be filled.
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

        // Points along axes at distance < radius should be filled.
        assert_eq!(canvas.pixel_at(15, 11), Color::RED); // 4 pixels above center
        assert_eq!(canvas.pixel_at(15, 19), Color::RED); // 4 pixels below center
        assert_eq!(canvas.pixel_at(11, 15), Color::RED); // 4 pixels left
        assert_eq!(canvas.pixel_at(19, 15), Color::RED); // 4 pixels right
    }

    #[test]
    fn draw_circle_pixels_well_outside_radius_are_transparent() {
        let w = 30;
        let h = 30;
        let mut canvas = Canvas::new(w, h);

        let center = Point { x: 15.0, y: 15.0 };
        let radius = 5.0;
        canvas.draw_circle(center, radius, Color::RED);

        // Pixels well outside the radius should be untouched.
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

        // Damage rect should encompass the circle's bounding box.
        assert!(damage.x <= 10);
        assert!(damage.y <= 10);
        assert!(damage.x + damage.width >= 20);
        assert!(damage.y + damage.height >= 20);
    }

    #[test]
    fn draw_circle_clipped_at_edge() {
        let w = 10;
        let h = 10;
        let mut canvas = Canvas::new(w, h);

        // Circle centered near the edge — should not panic and damage is clipped.
        let center = Point { x: 1.0, y: 1.0 };
        let radius = 5.0;
        let damage = canvas.draw_circle(center, radius, Color::BLUE);

        assert!(damage.x >= 0);
        assert!(damage.y >= 0);
        assert!(damage.x + damage.width <= w as i32);
        assert!(damage.y + damage.height <= h as i32);

        // Center color should still be written.
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

        // Every color along the horizontal midline between from.x and to.x should be filled.
        for x in 5..=25 {
            assert_eq!(
                canvas.pixel_at(x, 5),
                Color::RED,
                "color ({x}, 5) on line should be RED"
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
                "color (5, {y}) on line should be BLUE"
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

        // Damage must contain the full bounding box of the line + radius.
        assert!(damage.x <= 3, "damage.x={} should be <= 3", damage.x);
        assert!(damage.y <= 8, "damage.y={} should be <= 8", damage.y);
        assert!(
            damage.x + damage.width >= 32,
            "damage right edge {} should be >= 32",
            damage.x + damage.width
        );
        assert!(
            damage.y + damage.height >= 27,
            "damage bottom edge {} should be >= 27",
            damage.y + damage.height
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

        // Should behave like a circle at the point.
        assert_eq!(canvas.pixel_at(10, 10), Color::RED);
        assert!(damage.width > 0);
        assert!(damage.height > 0);
    }

    // -------------------------------------------------------
    // Canvas::draw_border
    // -------------------------------------------------------

    #[test]
    fn draw_border_draws_one_pixel_edges() {
        let w = 20;
        let h = 20;
        let mut canvas = Canvas::new(w, h);

        let bx = 3;
        let by = 4;
        let bw = 10u32;
        let bh = 8u32;
        canvas.draw_border(bx, by, bw, bh, Color::RED);

        // Top edge.
        for x in bx..(bx + bw as i32) {
            assert_eq!(
                canvas.pixel_at(x as u32, by as u32),
                Color::RED,
                "top edge color ({x}, {by}) should be RED"
            );
        }

        // Bottom edge.
        let bottom_y = by + bh as i32 - 1;
        for x in bx..(bx + bw as i32) {
            assert_eq!(
                canvas.pixel_at(x as u32, bottom_y as u32),
                Color::RED,
                "bottom edge color ({x}, {bottom_y}) should be RED"
            );
        }

        // Left edge.
        for y in by..(by + bh as i32) {
            assert_eq!(
                canvas.pixel_at(bx as u32, y as u32),
                Color::RED,
                "left edge color ({bx}, {y}) should be RED"
            );
        }

        // Right edge.
        let right_x = bx + bw as i32 - 1;
        for y in by..(by + bh as i32) {
            assert_eq!(
                canvas.pixel_at(right_x as u32, y as u32),
                Color::RED,
                "right edge color ({right_x}, {y}) should be RED"
            );
        }
    }

    #[test]
    fn draw_border_interior_is_not_filled() {
        let w = 20;
        let h = 20;
        let mut canvas = Canvas::new(w, h);

        let bx = 2;
        let by = 2;
        let bw = 10u32;
        let bh = 10u32;
        canvas.draw_border(bx, by, bw, bh, Color::RED);

        // Interior pixels (more than 1 color away from edges) should remain transparent.
        for y in (by + 1)..(by + bh as i32 - 1) {
            for x in (bx + 1)..(bx + bw as i32 - 1) {
                assert_eq!(
                    canvas.pixel_at(x as u32, y as u32),
                    Color::TRANSPARENT,
                    "interior color ({x}, {y}) should be transparent"
                );
            }
        }
    }

    #[test]
    fn draw_border_outside_is_not_filled() {
        let w = 20;
        let h = 20;
        let mut canvas = Canvas::new(w, h);

        canvas.draw_border(5, 5, 6, 6, Color::RED);

        // color just outside each edge.
        assert_eq!(canvas.pixel_at(4, 5), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(11, 5), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(5, 4), Color::TRANSPARENT);
        assert_eq!(canvas.pixel_at(5, 11), Color::TRANSPARENT);
    }

    // -------------------------------------------------------
    // Canvas::text_width
    // -------------------------------------------------------

    #[test]
    fn text_width_empty_string() {
        assert_eq!(Canvas::text_width(""), 0);
    }

    #[test]
    fn text_width_single_char() {
        assert_eq!(Canvas::text_width("A"), GLYPH_W);
    }

    #[test]
    fn text_width_multi_char() {
        let text = "Hello";
        assert_eq!(Canvas::text_width(text), text.len() as u32 * GLYPH_W);
    }

    #[test]
    fn text_width_with_spaces() {
        let text = "a b c";
        assert_eq!(Canvas::text_width(text), 5 * GLYPH_W);
    }
}
