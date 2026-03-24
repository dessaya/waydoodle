use smithay_client_toolkit::{
    compositor::CompositorState,
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_shm, wl_surface},
};
use wayland_cursor::CursorTheme;
use wayland_protocols::wp::cursor_shape::v1::client::wp_cursor_shape_device_v1;

use crate::model::{Command, ERASER_RADIUS};

use super::View;

pub(super) struct Cursor {
    surface: wl_surface::WlSurface,
    _buffer: Buffer,
    _pool: SlotPool,
    hotspot_x: i32,
    hotspot_y: i32,
}

pub(super) struct TabletCursorState {
    cursor_surface: wl_surface::WlSurface,
    cursor_theme: CursorTheme,
    eraser_cursor: Option<Cursor>,
}

impl TabletCursorState {
    pub(super) fn new(
        conn: &Connection,
        compositor: &CompositorState,
        shm: &Shm,
        qh: &QueueHandle<View>,
    ) -> Self {
        let cursor_surface = compositor.create_surface(qh);
        let cursor_theme =
            CursorTheme::load(conn, shm.wl_shm().clone(), 24).expect("Failed to load cursor theme");
        Self {
            cursor_surface,
            cursor_theme,
            eraser_cursor: None,
        }
    }
}

impl View {
    pub(super) fn set_crosshair_cursor(&self, qh: &QueueHandle<Self>) {
        if let (Some(pointer), Some(manager)) =
            (self.pointer.as_ref(), self.cursor_shape_manager.as_ref())
        {
            let device = manager.get_shape_device(pointer, qh);
            device.set_shape(
                self.pointer_enter_serial,
                wp_cursor_shape_device_v1::Shape::Crosshair,
            );
            device.destroy();
        }
    }

    pub(super) fn set_circle_cursor(&mut self, qh: &QueueHandle<Self>) {
        let pointer = match self.pointer.as_ref() {
            Some(p) => p,
            None => return,
        };

        if self.eraser_cursor.is_none() {
            self.eraser_cursor = Some(Self::create_eraser_cursor(
                &self.compositor_state,
                &self.shm,
                qh,
            ));
        }

        let cursor = self.eraser_cursor.as_ref().unwrap();
        pointer.set_cursor(
            self.pointer_enter_serial,
            Some(&cursor.surface),
            cursor.hotspot_x,
            cursor.hotspot_y,
        );
    }

    pub(super) fn set_tablet_crosshair_cursor(&mut self) {
        let tool = match self.tablet_tool.as_ref() {
            Some(t) => t,
            None => return,
        };
        let tablet_cursor = match self.tablet_cursor.as_mut() {
            Some(tc) => tc,
            None => return,
        };

        if let Some(cursor) = tablet_cursor.cursor_theme.get_cursor("crosshair") {
            let image = &cursor[0];
            let (hotspot_x, hotspot_y) = image.hotspot();
            tablet_cursor.cursor_surface.attach(Some(image), 0, 0);
            tablet_cursor.cursor_surface.commit();
            tool.set_cursor(
                self.tablet_tool_serial,
                Some(&tablet_cursor.cursor_surface),
                hotspot_x as i32,
                hotspot_y as i32,
            );
        }
    }

    pub(super) fn set_tablet_circle_cursor(&mut self, qh: &QueueHandle<Self>) {
        let tool = match self.tablet_tool.as_ref() {
            Some(t) => t.clone(),
            None => return,
        };
        let tablet_cursor = match self.tablet_cursor.as_mut() {
            Some(tc) => tc,
            None => return,
        };

        if tablet_cursor.eraser_cursor.is_none() {
            tablet_cursor.eraser_cursor = Some(Self::create_eraser_cursor(
                &self.compositor_state,
                &self.shm,
                qh,
            ));
        }

        let cursor = tablet_cursor.eraser_cursor.as_ref().unwrap();
        tool.set_cursor(
            self.tablet_tool_serial,
            Some(&cursor.surface),
            cursor.hotspot_x,
            cursor.hotspot_y,
        );
    }

    fn create_eraser_cursor(
        compositor: &CompositorState,
        shm: &Shm,
        qh: &QueueHandle<View>,
    ) -> Cursor {
        const CURSOR_RGBA: &[u8] = include_bytes!("../../assets/eraser_cursor.rgba");
        const CURSOR_SIZE: i32 = (ERASER_RADIUS as i32) * 2 + 1;
        const CURSOR_STRIDE: i32 = CURSOR_SIZE * 4;

        let mut pool =
            SlotPool::new(CURSOR_RGBA.len(), shm).expect("Failed to create cursor SHM pool");
        let (buffer, canvas) = pool
            .create_buffer(
                CURSOR_SIZE,
                CURSOR_SIZE,
                CURSOR_STRIDE,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create cursor buffer");

        // The embedded image is RGBA; Wayland expects ARGB in native byte order.
        // Convert each pixel: RGBA → ARGB stored as little-endian u32.
        for (rgba, argb) in CURSOR_RGBA.chunks_exact(4).zip(canvas.chunks_exact_mut(4)) {
            let [r, g, b, a] = [rgba[0], rgba[1], rgba[2], rgba[3]];
            let pixel = u32::from_be_bytes([a, r, g, b]);
            argb.copy_from_slice(&pixel.to_le_bytes());
        }

        let surface = compositor.create_surface(qh);
        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, CURSOR_SIZE, CURSOR_SIZE);
        surface.commit();

        Cursor {
            surface,
            _buffer: buffer,
            _pool: pool,
            hotspot_x: CURSOR_SIZE / 2,
            hotspot_y: CURSOR_SIZE / 2,
        }
    }

    pub(super) fn apply_current_cursor(&mut self, qh: &QueueHandle<Self>) {
        if let Some(overlay) = self.model.overlay.as_ref() {
            let cmd = overlay.cursor_command();
            match cmd {
                Command::SetCrosshairCursor => self.set_crosshair_cursor(qh),
                Command::SetCircleCursor => self.set_circle_cursor(qh),
                _ => {}
            }
        }
    }

    pub(super) fn apply_tablet_cursor(&mut self, qh: &QueueHandle<Self>) {
        if self.tablet_tool.is_none() {
            return;
        }

        if let Some(overlay) = self.model.overlay.as_ref() {
            let cmd = overlay.cursor_command();
            match cmd {
                Command::SetCrosshairCursor => self.set_tablet_crosshair_cursor(),
                Command::SetCircleCursor => self.set_tablet_circle_cursor(qh),
                _ => {}
            }
        }
    }
}
