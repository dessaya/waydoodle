use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_shm;

use crate::canvas::Rect;
use crate::{
    waydoodle::Overlay as _,
    wayland::{App, Overlay},
};

use super::OverlayState;

impl Overlay {
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

impl App {
    pub(super) fn mark_dirty(&mut self, qh: &QueueHandle<Self>, damage: Rect) {
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

        let show_help = overlay.show_help();

        // Copy back-buffer contents into the front buffer so both stay in sync.
        // After this, both buffers contain identical drawing data.
        overlay.sync_buffers();

        // Composite the help panel into the back buffer (which is about to be
        // presented to the compositor). The front buffer retains the clean
        // drawing data, so after swap() the new back buffer (old front) is
        // free of any transient UI — no cleanup needed when help is dismissed.
        if show_help {
            if let Some(mut canvas) = overlay.back_canvas() {
                super::help::render_help(&mut canvas);
            }
        }

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
