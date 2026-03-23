use std::os::unix::io::{AsFd, AsRawFd, FromRawFd};

use wayland_client::{
    Connection, Dispatch, QueueHandle,
    protocol::{wl_buffer, wl_pointer, wl_shm, wl_shm_pool, wl_surface},
};
use wayland_cursor::CursorTheme;
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2;

use crate::canvas::Tool;

/// Trait alias for the `Dispatch` bounds needed to create Wayland buffers.
pub trait WaylandState:
    Dispatch<wl_buffer::WlBuffer, ()> + Dispatch<wl_shm_pool::WlShmPool, ()> + 'static
{
}
impl<T: Dispatch<wl_buffer::WlBuffer, ()> + Dispatch<wl_shm_pool::WlShmPool, ()> + 'static>
    WaylandState for T
{
}

pub struct CursorManager {
    cursor_theme: CursorTheme,
    eraser_cursor_buffer: wl_buffer::WlBuffer,

    pointer: Option<wl_pointer::WlPointer>,
    pointer_cursor_surface: wl_surface::WlSurface,
    pointer_serial: Option<u32>,

    tablet_tool: Option<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    tablet_cursor_surface: wl_surface::WlSurface,
    tablet_serial: Option<u32>,
}

impl CursorManager {
    pub fn new<D: WaylandState>(
        conn: &Connection,
        shm: &wl_shm::WlShm,
        pointer_cursor_surface: wl_surface::WlSurface,
        tablet_cursor_surface: wl_surface::WlSurface,
        qh: &QueueHandle<D>,
    ) -> Self {
        let cursor_theme =
            CursorTheme::load(conn, shm.clone(), 24).expect("failed to load cursor theme");
        let eraser_cursor_buffer = create_eraser_cursor(shm, qh);
        Self {
            cursor_theme,
            eraser_cursor_buffer,
            pointer: None,
            pointer_cursor_surface,
            pointer_serial: None,
            tablet_tool: None,
            tablet_cursor_surface,
            tablet_serial: None,
        }
    }

    pub fn has_pointer(&self) -> bool {
        self.pointer.is_some()
    }

    pub fn set_pointer(&mut self, pointer: wl_pointer::WlPointer) {
        self.pointer = Some(pointer);
    }

    pub fn pointer_enter(&mut self, serial: u32, pointer: &wl_pointer::WlPointer, tool: Tool) {
        self.pointer_serial = Some(serial);
        let cursor_surface = self.pointer_cursor_surface.clone();
        let pointer = pointer.clone();
        self.set_cursor_for_tool(&cursor_surface, tool, |surface, hx, hy| {
            pointer.set_cursor(serial, Some(surface), hx, hy);
        });
    }

    pub fn tablet_proximity_in(
        &mut self,
        serial: u32,
        tablet_tool: &zwp_tablet_tool_v2::ZwpTabletToolV2,
        tool: Tool,
    ) {
        self.tablet_serial = Some(serial);
        self.tablet_tool = Some(tablet_tool.clone());
        let cursor_surface = self.tablet_cursor_surface.clone();
        let tablet_tool = tablet_tool.clone();
        self.set_cursor_for_tool(&cursor_surface, tool, |surface, hx, hy| {
            tablet_tool.set_cursor(serial, Some(surface), hx, hy);
        });
    }

    pub fn refresh(&mut self, tool: Tool) {
        self.refresh_pointer(tool);
        self.refresh_tablet(tool);
    }

    fn refresh_pointer(&mut self, tool: Tool) {
        if let (Some(serial), Some(pointer)) = (self.pointer_serial, self.pointer.clone()) {
            let cursor_surface = self.pointer_cursor_surface.clone();
            self.set_cursor_for_tool(&cursor_surface, tool, |surface, hx, hy| {
                pointer.set_cursor(serial, Some(surface), hx, hy);
            });
        }
    }

    fn refresh_tablet(&mut self, tool: Tool) {
        if let (Some(serial), Some(ref tablet_tool)) =
            (self.tablet_serial, self.tablet_tool.clone())
        {
            let cursor_surface = self.tablet_cursor_surface.clone();
            let tablet_tool = tablet_tool.clone();
            self.set_cursor_for_tool(&cursor_surface, tool, |surface, hx, hy| {
                tablet_tool.set_cursor(serial, Some(surface), hx, hy);
            });
        }
    }

    fn set_cursor_for_tool(
        &mut self,
        cursor_surface: &wl_surface::WlSurface,
        tool: Tool,
        set_cursor: impl FnOnce(&wl_surface::WlSurface, i32, i32),
    ) {
        if tool == Tool::Eraser {
            cursor_surface.attach(Some(&self.eraser_cursor_buffer), 0, 0);
            cursor_surface.commit();
            let hotspot = ERASER_CURSOR_RADIUS as i32;
            set_cursor(cursor_surface, hotspot, hotspot);
        } else if let Some(cursor) = self.cursor_theme.get_cursor("crosshair") {
            let image = &cursor[0];
            let (hotspot_x, hotspot_y) = image.hotspot();
            cursor_surface.attach(Some(image), 0, 0);
            cursor_surface.commit();
            set_cursor(cursor_surface, hotspot_x as i32, hotspot_y as i32);
        }
    }
}

const ERASER_CURSOR_RADIUS: u32 = 10;

/// Draw a hollow circle cursor for the eraser tool.
fn create_eraser_cursor<D: WaylandState>(
    shm: &wl_shm::WlShm,
    qh: &QueueHandle<D>,
) -> wl_buffer::WlBuffer {
    let radius = ERASER_CURSOR_RADIUS;
    let size = radius * 2 + 1;
    let radius = radius as f64;
    let center = radius;
    let byte_size = (size * size * 4) as usize;

    let file = create_shm_file(byte_size);
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            byte_size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_fd().as_raw_fd(),
            0,
        )
    };
    assert_ne!(ptr, libc::MAP_FAILED);

    let pixels = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, (size * size) as usize) };

    // Draw an antialiased hollow circle.
    let ring_outer = radius + 0.5;
    let ring_inner = radius - 1.5;
    for y in 0..size {
        for x in 0..size {
            let dx = x as f64 - center;
            let dy = y as f64 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let alpha = if dist <= ring_inner {
                0.0
            } else if dist <= ring_inner + 1.0 {
                dist - ring_inner
            } else if dist <= ring_outer - 1.0 {
                1.0
            } else if dist <= ring_outer {
                ring_outer - dist
            } else {
                0.0
            };
            let a = (alpha * 255.0).round() as u32;
            // White circle outline with computed alpha, pre-multiplied ARGB.
            pixels[(y * size + x) as usize] = (a << 24) | (a << 16) | (a << 8) | a;
        }
    }

    unsafe { libc::munmap(ptr as *mut libc::c_void, byte_size) };

    let pool = shm.create_pool(file.as_fd(), byte_size as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        size as i32,
        size as i32,
        (size * 4) as i32,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );

    pool.destroy();
    buffer
}

fn create_shm_file(size: usize) -> std::fs::File {
    let name = std::ffi::CString::new("waydoodle-cursor-shm").unwrap();
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    assert!(fd >= 0, "memfd_create failed");

    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    file.set_len(size as u64).expect("ftruncate failed");
    file
}
