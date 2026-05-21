use std::ops::Deref;

use cairo::{RectangleInt, Region};
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use crate::wayland::{App, Overlay};

impl Overlay {
    /// Create two SHM buffers, filled with transparent black.
    pub(super) fn create_buffers(pool: &mut SlotPool, width: i32, height: i32) -> [Buffer; 2] {
        let stride = width * 4;
        let (buf_a, canvas_a) = pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("Failed to create SHM buffer A");
        canvas_a.fill(0);

        let (buf_b, canvas_b) = pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("Failed to create SHM buffer B");
        canvas_b.fill(0);

        [buf_a, buf_b]
    }

    fn copy_rect(shm_buf: &mut [u8], canvas_buf: &[u8], width: i32, rect: &RectangleInt) {
        let stride = width as usize * 4;
        let w = width;
        let h = (canvas_buf.len() / stride) as i32;

        let x0 = rect.x().max(0) as usize;
        let y0 = rect.y().max(0) as usize;
        let x1 = (rect.x() + rect.width()).min(w).max(0) as usize;
        let y1 = (rect.y() + rect.height()).min(h).max(0) as usize;

        if x0 >= x1 || y0 >= y1 {
            return;
        }

        for y in y0..y1 {
            let start = y * stride + x0 * 4;
            let end = y * stride + x1 * 4;
            shm_buf[start..end].copy_from_slice(&canvas_buf[start..end]);
        }
    }

    pub(super) fn mark_dirty(&mut self, qh: &QueueHandle<App>, damage: RectangleInt) {
        let region = Region::create_rectangle(&damage);
        self.pending_damage
            .union(&region)
            .expect("Failed to union damage region");

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

        let damage = self.pending_damage.copy();
        self.pending_damage = Region::create();

        // The total region we must copy into this buffer: the current frame's
        // damage plus any stale region this buffer accumulated while the other
        // buffer was being presented.
        let copy_rect = self.stale[buf_idx].copy();
        copy_rect
            .union(&damage)
            .expect("Failed to union copy region");
        self.stale[buf_idx] = Region::create();

        // Mark the *other* buffer as stale in the region we're about to present.
        let other = 1 - buf_idx;
        self.stale[other]
            .union(&damage)
            .expect("Failed to union stale region");

        let width = self.state.canvas.width();

        // Copy only the affected rows from the off-screen canvas into the SHM
        // buffer.
        let data = self.state.canvas.surface_data();
        for i in 0..copy_rect.num_rectangles() {
            let rect = copy_rect.rectangle(i);
            Self::copy_rect(shm_buf, data.deref(), width, &rect);
        }

        // if the context menu is open, copy its surface into the shm_surface
        if let Some(menu_rect) = self.state.ui.context_menu_rect() {
            let surface = self
                .state
                .ui
                .surface_data()
                .expect("failed to get surface data");
            Self::copy_rect(shm_buf, surface.deref(), width, &menu_rect);
        }

        let surface = self.window.wl_surface();
        for i in 0..damage.num_rectangles() {
            let rect = damage.rectangle(i);
            surface.damage_buffer(rect.x(), rect.y(), rect.width(), rect.height());
        }
        self.buffers[buf_idx]
            .attach_to(surface)
            .expect("Failed to attach buffer");
        self.window.commit();
    }

    pub(super) fn on_frame_callback(&mut self, qh: &QueueHandle<App>) {
        self.frame_requested = false;

        if !self.pending_damage.is_empty() {
            self.frame_requested = true;
            let surface = self.window.wl_surface();
            surface.frame(qh, surface.clone());
            self.flush_frame();
        }
    }
}
