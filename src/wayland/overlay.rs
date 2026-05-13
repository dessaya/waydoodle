use cairo::RectangleInt;
use smithay_client_toolkit::{
    compositor::Region,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
        xdg::window::{Window, WindowDecorations},
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::protocol::wl_surface::WlSurface;

use crate::{
    waydoodle::{self},
    wayland::{App, OverlayState},
};

pub enum WaylandWindow {
    XdgWindow(Window),
    LayerSurface(LayerSurface),
}

impl WaylandWindow {
    pub(crate) fn wl_surface(&self) -> &WlSurface {
        match self {
            WaylandWindow::XdgWindow(w) => w.wl_surface(),
            WaylandWindow::LayerSurface(l) => l.wl_surface(),
        }
    }

    pub(crate) fn commit(&self) {
        match self {
            WaylandWindow::XdgWindow(w) => w.commit(),
            WaylandWindow::LayerSurface(l) => l.commit(),
        }
    }
}

pub(super) struct Overlay {
    pub window: WaylandWindow,

    /// SHM pool and two presentation buffers. At frame time we pick whichever
    /// buffer the compositor has released, copy the dirty region from the
    /// off-screen canvas into it, attach, and commit.
    pub pool: SlotPool,
    pub buffers: [Buffer; 2],

    /// Per-buffer tracking of regions that are out of date compared to
    /// `canvas_buf`. When we present using one buffer, the *other* buffer
    /// accumulates the damage as stale. Next time we use that buffer we
    /// must copy its stale region in addition to the current frame's damage.
    pub stale: [cairo::Region; 2],

    pub pending_damage: cairo::Region,
    pub frame_requested: bool,

    pub has_focus: bool,

    pub state: waydoodle::OverlayState,
}

impl Overlay {
    pub fn width(&self) -> i32 {
        self.state.canvas.width()
    }

    pub fn height(&self) -> i32 {
        self.state.canvas.height()
    }
}

impl App {
    pub(super) fn create_overlay_pool_and_buffers(
        shm: &Shm,
        width: i32,
        height: i32,
    ) -> (SlotPool, [Buffer; 2]) {
        log::debug!(
            "Creating SHM slot pool and buffers for overlay ({}x{})",
            width,
            height
        );
        let size = width as usize * height as usize * 4 * 2;
        let mut pool = SlotPool::new(size, shm).expect("Failed to create SHM slot pool");
        let buffers = Overlay::create_buffers(&mut pool, width, height);
        (pool, buffers)
    }
}

impl waydoodle::App<Overlay> for App {
    fn create_overlay(&mut self) {
        debug_assert!(
            self.overlay.is_none(),
            "create_overlay called while overlay already exists"
        );

        let wl_surface = self
            .wayland
            .compositor_state
            .create_surface(&self.queue_handle);

        // Set an empty opaque region so the compositor knows our surface is
        // fully transparent and must composite windows behind it correctly.
        if let Ok(empty_region) = Region::new(&self.wayland.compositor_state) {
            wl_surface.set_opaque_region(Some(empty_region.wl_region()));
        }

        const WINDOW_ID: &str = "waydoodle";
        let xdg_window_or_layer_surface = match &self.wayland.layer_shell {
            Some(layer_shell) => {
                let layer_surface = layer_shell.create_layer_surface(
                    &self.queue_handle,
                    wl_surface,
                    Layer::Overlay,
                    Some(WINDOW_ID),
                    None,
                );
                layer_surface.set_anchor(Anchor::all());
                layer_surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                layer_surface.set_size(0, 0); // Use full screen size
                layer_surface.set_exclusive_zone(-1);
                layer_surface.commit();
                WaylandWindow::LayerSurface(layer_surface)
            }
            None => {
                let window = self.wayland.xdg_shell.create_window(
                    wl_surface,
                    WindowDecorations::None,
                    &self.queue_handle,
                );
                window.set_title("Waydoodle");
                window.set_app_id(WINDOW_ID);
                window.set_maximized();
                window.commit();
                log::debug!(
                    "Created overlay widow -- waiting for configure event to create buffers"
                );
                WaylandWindow::XdgWindow(window)
            }
        };
        self.overlay = Some(OverlayState::Pending(xdg_window_or_layer_surface));
    }

    fn destroy_overlay(&mut self) {
        self.overlay = None;
    }

    fn toggle_focus_or_destroy_overlay(&mut self) {
        let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() else {
            return;
        };
        match &overlay.window {
            WaylandWindow::XdgWindow(_) => {
                self.destroy_overlay();
            }
            WaylandWindow::LayerSurface(l) => {
                let s = l.wl_surface();
                if overlay.has_focus {
                    // default (empty) region means the surface receives input events across its entire area
                    let r = Region::new(&self.wayland.compositor_state)
                        .expect("Failed to create input region for unfocusing overlay");
                    s.set_input_region(Some(r.wl_region()));
                    l.set_keyboard_interactivity(KeyboardInteractivity::None);
                } else {
                    // NULL region means the surface receives no input events
                    s.set_input_region(None);
                    l.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
                }
                s.commit();
                overlay.has_focus = !overlay.has_focus;
            }
        }
    }

    fn get_overlay(&self) -> Option<Option<&Overlay>> {
        match &self.overlay {
            Some(OverlayState::Ready(overlay)) => Some(Some(overlay)),
            Some(OverlayState::Pending(_)) => Some(None),
            None => None,
        }
    }
}

impl App {
    pub(crate) fn on_configure(&mut self, width: u32, height: u32) {
        let (width, height) = (width as i32, height as i32);
        match &mut self.overlay {
            Some(OverlayState::Pending(_)) => {
                // Transition Pending → Ready: take the Window out and build the full Overlay.
                let Some(OverlayState::Pending(window)) = self.overlay.take() else {
                    unreachable!();
                };
                let (pool, buffers) =
                    Self::create_overlay_pool_and_buffers(&self.wayland.shm, width, height);
                let mut overlay = Overlay {
                    window,
                    pool,
                    buffers,
                    stale: [cairo::Region::create(), cairo::Region::create()],
                    pending_damage: cairo::Region::create(),
                    frame_requested: false,
                    has_focus: true,
                    state: waydoodle::OverlayState::new(width, height)
                        .expect("Failed to create overlay state"),
                };
                overlay.mark_dirty(&self.queue_handle, RectangleInt::new(0, 0, width, height));
                self.overlay = Some(OverlayState::Ready(overlay));
            }
            Some(OverlayState::Ready(overlay))
                if width != overlay.width() || height != overlay.height() =>
            {
                log::debug!(
                    "Overlay window resized to {}x{} -- recreating SHM buffers",
                    width,
                    height
                );
                let damage = overlay.state.resize(width, height);
                let (pool, buffers) =
                    Self::create_overlay_pool_and_buffers(&self.wayland.shm, width, height);
                overlay.pool = pool;
                overlay.buffers = buffers;
                overlay.stale = [cairo::Region::create(), cairo::Region::create()];
                overlay.mark_dirty(&self.queue_handle, damage);
            }
            _ => {}
        }
    }
}
