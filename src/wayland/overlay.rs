use smithay_client_toolkit::{
    shell::{
        WaylandSurface,
        xdg::window::{Window, WindowDecorations},
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};

use crate::{
    canvas::{Canvas, Point, Rect},
    waydoodle::{self, StrokeItem, Tool},
    wayland::{App, OverlayState},
};

pub(super) struct Overlay {
    pub window: Window,
    pub width: u32,
    pub height: u32,

    // double buffering: we create two SHM buffers and alternate between them.
    // The compositor always holds a reference to the "front" buffer, while we
    // draw into the "back" buffer. When we want to update the display, we
    // attach the back buffer and then swap them.
    pub pool: SlotPool,
    pub buffers: [Buffer; 2],
    /// Index of the buffer currently attached to (owned by) the compositor.
    pub front: usize,

    pub pending_damage: Option<Rect>,
    pub frame_requested: bool,

    /// for OverlayTool
    pub tool: Tool,

    /// for OverlayHelp
    pub help: bool,

    /// for OverlayStrokes
    pub strokes: Vec<StrokeItem>,
    pub current_points: Vec<Point>,
}

impl App {
    pub(super) fn create_overlay_pool_and_buffers(
        shm: &Shm,
        width: u32,
        height: u32,
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
        log::debug!("Created overlay widow -- waiting for configure event to create buffers");
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

impl waydoodle::OverlayCanvas for Overlay {
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
}

impl waydoodle::OverlayTool for Overlay {
    fn current_tool(&self) -> Tool {
        self.tool
    }

    fn set_current_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }
}

impl waydoodle::OverlayHelp for Overlay {
    fn show_help(&self) -> bool {
        self.help
    }

    fn set_show_help(&mut self, show: bool) {
        self.help = show;
    }
}

impl waydoodle::OverlayStrokes for Overlay {
    fn strokes(&self) -> &[StrokeItem] {
        &self.strokes
    }

    fn push_stroke(&mut self, item: StrokeItem) {
        self.strokes.push(item);
    }

    fn pop_stroke(&mut self) -> Option<StrokeItem> {
        self.strokes.pop()
    }

    fn current_points(&mut self) -> &mut Vec<Point> {
        &mut self.current_points
    }
}
