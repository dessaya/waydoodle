use super::render::DirtyRect;
use super::{OverlayState, View};
use crate::model::{BrushStyle, Color, Point};

impl View {
    pub(super) fn clear_buffer(&mut self) -> Option<DirtyRect> {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return None,
        };
        let width = overlay.width;
        let height = overlay.height;
        let canvas = overlay.back_canvas()?;
        canvas.fill(0);
        Some(DirtyRect::full(width, height))
    }

    fn color_to_argb_le(color: Color) -> [u8; 4] {
        let argb: u32 = match color {
            Color::Red => 0xFFFF0000,
            Color::Green => 0xFF00FF00,
            Color::Blue => 0xFF0000FF,
            Color::Yellow => 0xFFFFFF00,
            Color::Magenta => 0xFFFF00FF,
            Color::Cyan => 0xFF00FFFF,
        };
        argb.to_le_bytes()
    }

    fn set_pixel(canvas: &mut [u8], width: u32, height: u32, x: i32, y: i32, pixel: [u8; 4]) {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let offset = (y as usize * width as usize + x as usize) * 4;
        canvas[offset..offset + 4].copy_from_slice(&pixel);
    }

    fn fill_circle(
        canvas: &mut [u8],
        width: u32,
        height: u32,
        center: Point,
        radius: f64,
        pixel: [u8; 4],
    ) -> DirtyRect {
        let r = radius.ceil() as i32;
        let cx_i = center.x.round() as i32;
        let cy_i = center.y.round() as i32;
        let r_sq = radius * radius;
        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = (dx as f64 - (center.x - cx_i as f64)).powi(2)
                    + (dy as f64 - (center.y - cy_i as f64)).powi(2);
                if dist_sq <= r_sq {
                    Self::set_pixel(canvas, width, height, cx_i + dx, cy_i + dy, pixel);
                }
            }
        }

        let x = (center.x - radius).floor().max(0.0) as i32;
        let y = (center.y - radius).floor().max(0.0) as i32;
        let x1 = (center.x + radius).ceil().min(width as f64) as i32;
        let y1 = (center.y + radius).ceil().min(height as f64) as i32;
        DirtyRect {
            x,
            y,
            width: (x1 - x).max(0),
            height: (y1 - y).max(0),
        }
    }

    fn stroke(
        canvas: &mut [u8],
        width: u32,
        height: u32,
        from: Point,
        to: Point,
        radius: f64,
        pixel: [u8; 4],
    ) -> DirtyRect {
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
            Self::fill_circle(canvas, width, height, center, radius, pixel);
        }

        let min_x = from.x.min(to.x) - radius;
        let min_y = from.y.min(to.y) - radius;
        let max_x = from.x.max(to.x) + radius;
        let max_y = from.y.max(to.y) + radius;
        let x = min_x.floor().max(0.0) as i32;
        let y = min_y.floor().max(0.0) as i32;
        let x1 = max_x.ceil().min(width as f64) as i32;
        let y1 = max_y.ceil().min(height as f64) as i32;
        DirtyRect {
            x,
            y,
            width: (x1 - x).max(0),
            height: (y1 - y).max(0),
        }
    }

    fn brush_pixel(style: BrushStyle) -> [u8; 4] {
        match style {
            BrushStyle::Draw(color) => Self::color_to_argb_le(color),
            BrushStyle::Erase => [0, 0, 0, 0],
        }
    }

    pub(super) fn draw_line(
        &mut self,
        style: BrushStyle,
        radius: f64,
        from: Point,
        to: Point,
    ) -> Option<DirtyRect> {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return None,
        };
        let width = overlay.width;
        let height = overlay.height;
        let canvas = overlay.back_canvas()?;
        let pixel = Self::brush_pixel(style);
        Some(Self::stroke(canvas, width, height, from, to, radius, pixel))
    }

    pub(super) fn draw_dot(
        &mut self,
        style: BrushStyle,
        radius: f64,
        center: Point,
    ) -> Option<DirtyRect> {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return None,
        };
        let width = overlay.width;
        let height = overlay.height;
        let canvas = overlay.back_canvas()?;
        let pixel = Self::brush_pixel(style);
        Some(Self::fill_circle(
            canvas, width, height, center, radius, pixel,
        ))
    }
}
