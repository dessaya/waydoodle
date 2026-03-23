use std::os::unix::io::{AsFd, AsRawFd};

use wayland_client::protocol::wl_surface;

struct SendPtr(*mut u8);
unsafe impl Send for SendPtr {}

struct InputState {
    pressed: bool,
    x: f64,
    y: f64,
    prev: Option<(f64, f64)>,
}

impl InputState {
    fn new() -> Self {
        Self {
            pressed: false,
            x: 0.0,
            y: 0.0,
            prev: None,
        }
    }

    fn reset(&mut self) {
        self.pressed = false;
        self.prev = None;
    }
}

pub struct Canvas {
    shm_ptr: SendPtr,
    width: u32,
    height: u32,
    strokes: Vec<(f64, f64, f64, f64)>,
    pointer: InputState,
    tablet: InputState,
}

impl Canvas {
    pub fn new() -> Self {
        Self {
            shm_ptr: SendPtr(std::ptr::null_mut()),
            width: 0,
            height: 0,
            strokes: Vec::new(),
            pointer: InputState::new(),
            tablet: InputState::new(),
        }
    }

    fn shm_size(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }

    /// Attach a new shared-memory buffer to the canvas, filling it with the
    /// background color and replaying any existing strokes.
    pub fn attach(&mut self, file: &impl AsFd, width: u32, height: u32) {
        let size = (width as usize) * (height as usize) * 4;

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_fd().as_raw_fd(),
                0,
            )
        };
        assert_ne!(ptr, libc::MAP_FAILED);

        // Fill with transparent black (ARGB8888).
        let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, size / 4) };
        for pixel in slice.iter_mut() {
            *pixel = 0x00_00_00_00;
        }

        self.unmap();

        self.shm_ptr = SendPtr(ptr as *mut u8);
        self.width = width;
        self.height = height;

        // Replay existing strokes into the new buffer.
        for &(x0, y0, x1, y1) in &self.strokes {
            draw_line_on_buffer(self.shm_ptr.0, width, height, x0, y0, x1, y1);
        }
    }

    /// Unmap the current pixel buffer (if any).
    pub fn unmap(&mut self) {
        if !self.shm_ptr.0.is_null() {
            unsafe { libc::munmap(self.shm_ptr.0 as *mut libc::c_void, self.shm_size()) };
            self.shm_ptr = SendPtr(std::ptr::null_mut());
        }
    }

    /// Reset all drawing state and clear stored strokes.
    pub fn clear(&mut self) {
        self.strokes.clear();
        self.reset_input();
    }

    /// Reset transient input state without clearing strokes.
    pub fn reset_input(&mut self) {
        self.pointer.reset();
        self.tablet.reset();
    }

    /// Record a line segment, draw it into the pixel buffer, and
    /// damage + commit the surface.
    pub fn draw_stroke(
        &mut self,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        wl_surface: &wl_surface::WlSurface,
    ) {
        self.strokes.push((x0, y0, x1, y1));
        if self.shm_ptr.0.is_null() {
            return;
        }
        draw_line_on_buffer(self.shm_ptr.0, self.width, self.height, x0, y0, x1, y1);
        wl_surface.damage_buffer(0, 0, self.width as i32, self.height as i32);
        wl_surface.commit();
    }

    pub fn pointer_enter(&mut self, x: f64, y: f64) {
        self.pointer.x = x;
        self.pointer.y = y;
    }

    pub fn pointer_leave(&mut self) {
        self.pointer.reset();
    }

    pub fn pointer_motion(&mut self, x: f64, y: f64) {
        self.pointer.x = x;
        self.pointer.y = y;
    }

    pub fn pointer_button(&mut self, pressed: bool) {
        self.pointer.pressed = pressed;
        if pressed {
            self.pointer.prev = Some((self.pointer.x, self.pointer.y));
        } else {
            self.pointer.prev = None;
        }
    }

    pub fn pointer_frame(&mut self, wl_surface: &wl_surface::WlSurface) {
        if self.pointer.pressed {
            let x = self.pointer.x;
            let y = self.pointer.y;
            if let Some((px, py)) = self.pointer.prev {
                self.draw_stroke(px, py, x, y, wl_surface);
            }
            self.pointer.prev = Some((x, y));
        }
    }

    pub fn tablet_proximity_out(&mut self) {
        self.tablet.reset();
    }

    pub fn tablet_down(&mut self) {
        self.tablet.pressed = true;
        self.tablet.prev = Some((self.tablet.x, self.tablet.y));
    }

    pub fn tablet_up(&mut self) {
        self.tablet.reset();
    }

    pub fn tablet_motion(&mut self, x: f64, y: f64) {
        self.tablet.x = x;
        self.tablet.y = y;
    }

    pub fn tablet_frame(&mut self, wl_surface: &wl_surface::WlSurface) {
        if self.tablet.pressed {
            let x = self.tablet.x;
            let y = self.tablet.y;
            if let Some((px, py)) = self.tablet.prev {
                self.draw_stroke(px, py, x, y, wl_surface);
            }
            self.tablet.prev = Some((x, y));
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.unmap();
    }
}

fn draw_line_on_buffer(ptr: *mut u8, width: u32, height: u32, x0: f64, y0: f64, x1: f64, y1: f64) {
    let pixels =
        unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, (width * height) as usize) };
    let brush_radius: i32 = 1;
    let color: u32 = 0xFF_FF_00_00; // opaque red (ARGB8888)

    let mut ix0 = x0 as i64;
    let mut iy0 = y0 as i64;
    let ix1 = x1 as i64;
    let iy1 = y1 as i64;
    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx: i64 = if ix0 < ix1 { 1 } else { -1 };
    let sy: i64 = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Draw a filled circle at (ix0, iy0).
        for by in -brush_radius..=brush_radius {
            for bx in -brush_radius..=brush_radius {
                if bx * bx + by * by <= brush_radius * brush_radius {
                    let px = ix0 as i32 + bx;
                    let py = iy0 as i32 + by;
                    if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                        pixels[py as usize * width as usize + px as usize] = color;
                    }
                }
            }
        }

        if ix0 == ix1 && iy0 == iy1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            ix0 += sx;
        }
        if e2 <= dx {
            err += dx;
            iy0 += sy;
        }
    }
}
