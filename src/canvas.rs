use bdf_reader::Font;

pub const GLYPH_W: u32 = 8;
pub const GLYPH_H: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy)]
pub struct Rect {
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

pub struct Canvas<'a> {
    pub buf: &'a mut [u8],
    pub width: u32,
    pub height: u32,
}

impl<'a> Canvas<'a> {
    pub fn clear(&mut self) -> Rect {
        self.buf.fill(0);
        Rect::new(self.width, self.height)
    }

    pub fn set_pixel(&mut self, x: i32, y: i32, pixel: [u8; 4]) {
        let width = self.width;
        let height = self.height;
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let offset = (y as usize * width as usize + x as usize) * 4;
        self.buf[offset..offset + 4].copy_from_slice(&pixel);
    }

    pub fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, pixel: [u8; 4]) -> Rect {
        let width = self.width;
        let height = self.height;

        let x0 = x;
        let y0 = y;
        let x1 = x + w as i32;
        let y1 = y + h as i32;

        for y in y0..y1 {
            for x in x0..x1 {
                self.set_pixel(x, y, pixel);
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

    pub fn draw_circle(&mut self, center: Point, radius: f64, pixel: [u8; 4]) -> Rect {
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
                    self.set_pixel(cx_i + dx, cy_i + dy, pixel);
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

    pub fn draw_border(&mut self, x: i32, y: i32, w: u32, h: u32, color: [u8; 4]) {
        self.draw_rect(x, y, w, 1, color);
        self.draw_rect(x, y + h as i32 - 1, w, 1, color);
        self.draw_rect(x, y, 1, h, color);
        self.draw_rect(x + w as i32 - 1, y, 1, h, color);
    }

    pub fn draw_line(&mut self, from: Point, to: Point, radius: f64, pixel: [u8; 4]) -> Rect {
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
            self.draw_circle(center, radius, pixel);
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

    pub fn draw_text(&mut self, font: &Font, text: &str, x: i32, y: i32, color: [u8; 4]) {
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
