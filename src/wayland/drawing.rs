use smithay_client_toolkit::{shell::WaylandSurface, shm::slot::SlotPool};
use wayland_client::{QueueHandle, protocol::wl_shm};

use crate::model::{BrushStyle, Color, Point};

use super::View;

impl View {
    pub(super) fn ensure_pool(&mut self) {
        if self.pool.is_none() && self.width > 0 && self.height > 0 {
            let size = self.width as usize * self.height as usize * 4;
            self.pool =
                Some(SlotPool::new(size, &self.shm).expect("Failed to create SHM slot pool"));
        }
    }

    pub(super) fn canvas_mut(&mut self) -> Option<&mut [u8]> {
        let buffer = self.buffer.as_ref()?;
        let pool = self.pool.as_mut()?;
        pool.canvas(buffer)
    }

    pub(super) fn draw_frame(&mut self, qh: &QueueHandle<Self>) {
        if self.window.is_none() || self.width == 0 || self.height == 0 {
            return;
        }

        self.ensure_pool();

        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;
        let pool = self.pool.as_mut().unwrap();

        let buffer = self.buffer.get_or_insert_with(|| {
            let (buf, canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("Failed to create SHM buffer");
            canvas.fill(0);
            buf
        });

        if pool.canvas(buffer).is_none() {
            let (new_buffer, canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("Failed to create SHM buffer");
            canvas.fill(0);
            *buffer = new_buffer;
        }

        let window = self.window.as_ref().unwrap();
        let surface = window.wl_surface();

        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.frame(qh, surface.clone());
        buffer.attach_to(surface).expect("Failed to attach buffer");
        window.commit();

        self.dirty = false;
    }

    pub(super) fn clear_buffer(&mut self) {
        if let Some(canvas) = self.canvas_mut() {
            canvas.fill(0);
            self.dirty = true;
        }
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
        cx: f64,
        cy: f64,
        radius: f64,
        pixel: [u8; 4],
    ) {
        let r = radius.ceil() as i32;
        let cx_i = cx.round() as i32;
        let cy_i = cy.round() as i32;
        let r_sq = radius * radius;
        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = (dx as f64 - (cx - cx_i as f64)).powi(2)
                    + (dy as f64 - (cy - cy_i as f64)).powi(2);
                if dist_sq <= r_sq {
                    Self::set_pixel(canvas, width, height, cx_i + dx, cy_i + dy, pixel);
                }
            }
        }
    }

    fn stroke(
        canvas: &mut [u8],
        width: u32,
        height: u32,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        radius: f64,
        pixel: [u8; 4],
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = dx.hypot(dy);
        let steps = (dist / 0.5).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = x0 + dx * t;
            let y = y0 + dy * t;
            Self::fill_circle(canvas, width, height, x, y, radius, pixel);
        }
    }

    fn brush_pixel(style: BrushStyle) -> [u8; 4] {
        match style {
            BrushStyle::Draw(color) => Self::color_to_argb_le(color),
            BrushStyle::Erase => [0, 0, 0, 0],
        }
    }

    pub(super) fn render_line(&mut self, style: BrushStyle, radius: f64, from: Point, to: Point) {
        let width = self.width;
        let height = self.height;
        if let Some(canvas) = self.canvas_mut() {
            let pixel = Self::brush_pixel(style);
            Self::stroke(
                canvas, width, height, from.x, from.y, to.x, to.y, radius, pixel,
            );
            self.dirty = true;
        }
    }

    pub(super) fn render_dot(&mut self, style: BrushStyle, radius: f64, center: Point) {
        let width = self.width;
        let height = self.height;
        if let Some(canvas) = self.canvas_mut() {
            let pixel = Self::brush_pixel(style);
            Self::fill_circle(canvas, width, height, center.x, center.y, radius, pixel);
            self.dirty = true;
        }
    }
}
