use smithay_client_toolkit::{
    compositor::CompositorState,
    seat::pointer::cursor_shape::CursorShapeManager,
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

use super::View;
use crate::model::{CursorShape, ERASER_RADIUS};

pub(crate) struct TabletCursorState {
    cursor_surface: wl_surface::WlSurface,
    cursor_theme: CursorTheme,
    eraser_cursor: CursorSurface,
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
        let eraser_cursor = CursorSurface::from_rgba(
            include_bytes!("../../assets/eraser_cursor.rgba"),
            (ERASER_RADIUS as i32) * 2 + 1,
            (ERASER_RADIUS as i32) * 2 + 1,
            compositor,
            shm,
            qh,
        );
        Self {
            cursor_surface,
            cursor_theme,
            eraser_cursor,
        }
    }
}

pub(crate) struct Cursors {
    pub shape_manager: CursorShapeManager,
    eraser: CursorSurface,
}

impl Cursors {
    pub(super) fn new(
        compositor: &CompositorState,
        shm: &Shm,
        globals: &wayland_client::globals::GlobalList,
        qh: &QueueHandle<View>,
    ) -> Self {
        let shape_manager =
            CursorShapeManager::bind(globals, qh).expect("cursor shape manager not available");
        let eraser = CursorSurface::from_rgba(
            include_bytes!("../../assets/eraser_cursor.rgba"),
            (ERASER_RADIUS as i32) * 2 + 1,
            (ERASER_RADIUS as i32) * 2 + 1,
            compositor,
            shm,
            qh,
        );
        Self {
            shape_manager,
            eraser,
        }
    }
}

struct CursorSurface {
    surface: wl_surface::WlSurface,
    hotspot_x: i32,
    hotspot_y: i32,
    // Kept alive so the backing SHM memory isn't freed while the surface references it.
    _buffer: Buffer,
    _pool: SlotPool,
}

impl CursorSurface {
    /// Creates a cursor surface from raw RGBA pixel data.
    ///
    /// The hotspot is placed at the center of the image. `rgba` must contain
    /// exactly `width * height * 4` bytes in RGBA order.
    fn from_rgba(
        rgba: &[u8],
        width: i32,
        height: i32,
        compositor: &CompositorState,
        shm: &Shm,
        qh: &QueueHandle<View>,
    ) -> Self {
        let stride = width * 4;
        let size = (width * height * 4) as usize;
        assert_eq!(
            rgba.len(),
            size,
            "RGBA data length must match width * height * 4"
        );

        let mut pool = SlotPool::new(size, shm).expect("Failed to create cursor SHM pool");
        let (buffer, canvas) = pool
            .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
            .expect("Failed to create cursor buffer");

        // The input is RGBA; Wayland expects ARGB in native byte order.
        // Convert each pixel: RGBA → ARGB stored as little-endian u32.
        for (src, dst) in rgba.chunks_exact(4).zip(canvas.chunks_exact_mut(4)) {
            let [r, g, b, a] = [src[0], src[1], src[2], src[3]];
            let pixel = u32::from_be_bytes([a, r, g, b]);
            dst.copy_from_slice(&pixel.to_le_bytes());
        }

        let surface = compositor.create_surface(qh);
        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, width, height);
        surface.commit();

        Self {
            surface,
            _buffer: buffer,
            _pool: pool,
            hotspot_x: width / 2,
            hotspot_y: height / 2,
        }
    }
}

impl View {
    pub(super) fn apply_cursor(&mut self, shape: CursorShape, qh: &QueueHandle<Self>) {
        self.set_pointer_cursor(shape, qh);
        self.set_tablet_cursor(shape);
    }

    fn set_pointer_cursor(&self, shape: CursorShape, qh: &QueueHandle<Self>) {
        let ptr = match self.pointer.as_ref() {
            Some(p) => p,
            None => return,
        };

        match shape {
            CursorShape::Crosshair => {
                let device = self
                    .cursors
                    .shape_manager
                    .get_shape_device(&ptr.wl_pointer, qh);
                device.set_shape(
                    ptr.enter_serial,
                    wp_cursor_shape_device_v1::Shape::Crosshair,
                );
                device.destroy();
            }
            CursorShape::Circle => {
                let cursor = &self.cursors.eraser;
                ptr.wl_pointer.set_cursor(
                    ptr.enter_serial,
                    Some(&cursor.surface),
                    cursor.hotspot_x,
                    cursor.hotspot_y,
                );
            }
        }
    }

    fn set_tablet_cursor(&mut self, shape: CursorShape) {
        let tablet = match self.tablet.as_mut() {
            Some(t) => t,
            None => return,
        };
        let active = match tablet.active_tool.as_ref() {
            Some(a) => a,
            None => return,
        };

        match shape {
            CursorShape::Crosshair => {
                if let Some(cursor) = tablet.cursor.cursor_theme.get_cursor("crosshair") {
                    let image = &cursor[0];
                    let (hotspot_x, hotspot_y) = image.hotspot();
                    tablet.cursor.cursor_surface.attach(Some(image), 0, 0);
                    tablet.cursor.cursor_surface.commit();
                    active.tool.set_cursor(
                        active.serial,
                        Some(&tablet.cursor.cursor_surface),
                        hotspot_x as i32,
                        hotspot_y as i32,
                    );
                }
            }
            CursorShape::Circle => {
                let cursor = &tablet.cursor.eraser_cursor;
                active.tool.set_cursor(
                    active.serial,
                    Some(&cursor.surface),
                    cursor.hotspot_x,
                    cursor.hotspot_y,
                );
            }
        }
    }
}
