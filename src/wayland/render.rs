use smithay_client_toolkit::shell::WaylandSurface;
use wayland_client::QueueHandle;

use super::{OverlayState, View};

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

        let width = overlay.width;
        let height = overlay.height;
        let stride = width as i32 * 4;

        if overlay.pool.canvas(&overlay.buffer).is_none() {
            let (new_buffer, canvas) = overlay
                .pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wayland_client::protocol::wl_shm::Format::Argb8888,
                )
                .expect("Failed to create SHM buffer");
            canvas.fill(0);
            overlay.buffer = new_buffer;
        }

        let surface = overlay.window.wl_surface();
        surface.damage_buffer(damage.x, damage.y, damage.width, damage.height);
        overlay
            .buffer
            .attach_to(surface)
            .expect("Failed to attach buffer");
        overlay.window.commit();
    }
}
