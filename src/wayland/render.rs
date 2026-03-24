use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use super::{OverlayState, View, ViewOverlay};

#[derive(Clone, Copy)]
pub(crate) struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl DirtyRect {
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width: width as i32,
            height: height as i32,
        }
    }

    pub fn merge(self, other: DirtyRect) -> Self {
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

impl ViewOverlay {
    /// Returns a mutable reference to the back buffer's canvas (the one NOT
    /// held by the compositor). Returns `None` only if the pool is in a bad
    /// state.
    pub fn back_canvas(&mut self) -> Option<&mut [u8]> {
        let back = 1 - self.front;
        self.pool.canvas(&self.buffers[back])
    }

    /// Returns the index of the back buffer.
    pub fn back_index(&self) -> usize {
        1 - self.front
    }

    /// Swap front and back: the back buffer (which we just
    /// attached) becomes the new front.
    pub fn swap(&mut self) {
        self.front = 1 - self.front;
    }

    /// Copy the entire back-buffer contents into the front buffer so the two
    /// stay in sync after a swap. Call this *before* `swap()`.
    pub fn sync_buffers(&mut self) {
        // We need both canvases at the same time, but `pool.canvas` borrows
        // mutably. Instead we use the raw pool slice and buffer offsets.
        // SlotPool doesn't expose raw offsets, so we copy via a temp vec.
        let back = 1 - self.front;
        let front = self.front;

        // Try to get the back canvas and copy its contents
        if let Some(src) = self.pool.canvas(&self.buffers[back]) {
            let data = src.to_vec();
            if let Some(dst) = self.pool.canvas(&self.buffers[front]) {
                dst[..data.len()].copy_from_slice(&data);
            }
        }
    }

    /// Create both SHM buffers, filling them with transparent black.
    pub fn create_buffers(pool: &mut SlotPool, width: u32, height: u32) -> [Buffer; 2] {
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
}

impl View {
    pub(super) fn mark_dirty(&mut self, qh: &QueueHandle<Self>, damage: DirtyRect) {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return,
        };

        overlay.pending_damage = Some(match overlay.pending_damage {
            Some(existing) => existing.merge(damage),
            None => damage,
        });

        if !overlay.frame_requested {
            overlay.frame_requested = true;
            let surface = overlay.window.wl_surface();
            surface.frame(qh, surface.clone());
            self.flush_frame();
        }
    }

    pub(super) fn on_frame_callback(&mut self, qh: &QueueHandle<Self>) {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return,
        };

        overlay.frame_requested = false;

        if overlay.pending_damage.is_some() {
            overlay.frame_requested = true;
            let surface = overlay.window.wl_surface();
            surface.frame(qh, surface.clone());
            self.flush_frame();
        }
    }

    fn flush_frame(&mut self) {
        let overlay = match self.overlay.as_mut() {
            Some(OverlayState::Ready(o)) => o,
            _ => return,
        };

        let damage = match overlay.pending_damage.take() {
            Some(d) => d,
            None => return,
        };

        // Copy back-buffer contents into the front buffer so both stay in sync,
        // then present the back buffer and swap indices.
        overlay.sync_buffers();

        let back = overlay.back_index();
        let surface = overlay.window.wl_surface();
        surface.damage_buffer(damage.x, damage.y, damage.width, damage.height);
        overlay.buffers[back]
            .attach_to(surface)
            .expect("Failed to attach buffer");
        overlay.window.commit();

        overlay.swap();
    }
}
