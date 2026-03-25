use smithay_client_toolkit::shell::{WaylandSurface, xdg::window::WindowDecorations};

use crate::{
    canvas::Canvas,
    waydoodle::{self, Tool},
    wayland::{App, Overlay, OverlayState},
};

impl waydoodle::App<Overlay> for App {
    fn create_overlay(&mut self) {
        debug_assert!(
            self.overlay.is_none(),
            "create_overlay called while overlay already exists"
        );
        let surface = self
            .wayland
            .compositor_state
            .create_surface(&self.queue_handle);
        let window = self.wayland.xdg_shell.create_window(
            surface,
            WindowDecorations::None,
            &self.queue_handle,
        );
        window.set_title("Waydoodle");
        window.set_app_id("io.github.dessaya.waydoodle");
        window.set_maximized();
        window.commit();
        self.overlay = Some(OverlayState::Pending(window));
    }

    fn destroy_overlay(&mut self) {
        if let Some(overlay) = self.overlay.take() {
            let window = match overlay {
                OverlayState::Pending(window) => window,
                OverlayState::Ready(overlay) => overlay.window,
            };
            let surface = window.wl_surface().clone();
            drop(window);
            surface.destroy();
        }
        self.overlay = None;
    }

    fn get_overlay(&self) -> Option<Option<&Overlay>> {
        match &self.overlay {
            Some(OverlayState::Ready(overlay)) => Some(Some(overlay)),
            Some(OverlayState::Pending(_)) => Some(None),
            None => None,
        }
    }
}

impl waydoodle::Overlay for Overlay {
    /// Returns a mutable reference to the back buffer's canvas (the one NOT
    /// held by the compositor). Returns `None` only if the pool is in a bad
    /// state.
    fn back_canvas(&mut self) -> Option<Canvas<'_>> {
        let back = 1 - self.front;
        if let Some(buf) = self.pool.canvas(&self.buffers[back]) {
            return Some(Canvas {
                buf,
                width: self.width,
                height: self.height,
            });
        }
        None
    }

    fn current_tool(&self) -> Tool {
        self.tool
    }

    fn set_current_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }

    fn show_help(&self) -> bool {
        self.help
    }

    fn set_show_help(&mut self, show: bool) {
        self.help = show;
    }
}
