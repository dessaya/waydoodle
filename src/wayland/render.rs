use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use crate::canvas::{Canvas, Rect};
use crate::wayland::{App, Overlay};

impl Overlay {
    /// Create two SHM buffers, filled with transparent black.
    pub(super) fn create_buffers(pool: &mut SlotPool, width: u32, height: u32) -> [Buffer; 2] {
        let stride = width as i32 * 4;
        let (buf_a, canvas_a) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create SHM buffer A");
        canvas_a.fill(0);

        let (buf_b, canvas_b) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create SHM buffer B");
        canvas_b.fill(0);

        [buf_a, buf_b]
    }

    /// Copy rows covered by `rect` from `canvas_buf` into `shm_buf`.
    /// Both slices have identical layout (width × height × 4 bytes, row-major).
    fn copy_rect(shm_buf: &mut [u8], canvas_buf: &[u8], width: u32, rect: &Rect) {
        let stride = width as usize * 4;
        let w = width as i32;
        let h = (canvas_buf.len() / stride) as i32;

        // Clamp the rect to the buffer bounds.
        let x0 = rect.x.max(0) as usize;
        let y0 = rect.y.max(0) as usize;
        let x1 = (rect.x + rect.width).min(w).max(0) as usize;
        let y1 = (rect.y + rect.height).min(h).max(0) as usize;

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        let byte_start = x0 * 4;
        let byte_end = x1 * 4;

        for y in y0..y1 {
            let row_offset = y * stride;
            let src = &canvas_buf[row_offset + byte_start..row_offset + byte_end];
            shm_buf[row_offset + byte_start..row_offset + byte_end].copy_from_slice(src);
        }
    }

    pub(super) fn mark_dirty(&mut self, qh: &QueueHandle<App>, damage: Rect) {
        self.pending_damage = Some(match self.pending_damage {
            Some(existing) => existing.merge(damage),
            None => damage,
        });

        if !self.frame_requested {
            self.frame_requested = true;
            let surface = self.window.wl_surface();
            surface.frame(qh, surface.clone());
            self.flush_frame();
        }
    }

    fn flush_frame(&mut self) {
        // Find a free SHM buffer. Try both; at least one should be available.
        let (buf_idx, shm_buf) = {
            if let Some(shm_buf) = self.pool.canvas(&self.buffers[0]) {
                (0, shm_buf)
            } else if let Some(shm_buf) = self.pool.canvas(&self.buffers[1]) {
                (1, shm_buf)
            } else {
                log::debug!("flush_frame: both SHM buffers held by compositor, deferring");
                return;
            }
        };

        let damage = match self.pending_damage.take() {
            Some(d) => d,
            None => return,
        };

        // The total region we must copy into this buffer: the current frame's
        // damage plus any stale region this buffer accumulated while the other
        // buffer was being presented.
        let copy_rect = match self.stale[buf_idx] {
            Some(stale) => stale.merge(damage),
            None => damage,
        };
        self.stale[buf_idx] = None;

        // Mark the *other* buffer as stale in the region we're about to present.
        let other = 1 - buf_idx;
        self.stale[other] = Some(match self.stale[other] {
            Some(existing) => existing.merge(damage),
            None => damage,
        });

        // Copy only the affected rows from the off-screen canvas into the SHM
        // buffer.
        Self::copy_rect(
            shm_buf,
            &self.state.canvas.buf,
            self.state.canvas.width,
            &copy_rect,
        );

        // Composite the help panel directly into the SHM buffer (transient UI
        // that should not persist in the off-screen canvas).
        if self.state.show_help {
            let mut canvas = Canvas::new(self.width(), self.height());
            crate::help::render_help(&mut canvas);
        }

        let surface = self.window.wl_surface();
        surface.damage_buffer(damage.x, damage.y, damage.width, damage.height);
        self.buffers[buf_idx]
            .attach_to(surface)
            .expect("Failed to attach buffer");
        self.window.commit();
    }

    pub(super) fn on_frame_callback(&mut self, qh: &QueueHandle<App>) {
        self.frame_requested = false;

        if self.pending_damage.is_some() {
            self.frame_requested = true;
            let surface = self.window.wl_surface();
            surface.frame(qh, surface.clone());
            self.flush_frame();
        }
    }
}
