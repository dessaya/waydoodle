use std::os::unix::io::{AsFd, AsRawFd};

use wayland_client::protocol::wl_surface;

struct SendPtr(*mut u8);
unsafe impl Send for SendPtr {}

#[derive(Clone, Copy, PartialEq)]
struct Coord {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    Red,
    Green,
    Blue,
    Yellow,
}

impl Color {
    fn to_argb(self) -> u32 {
        match self {
            Color::Red => 0xFF_FF_00_00,
            Color::Green => 0xFF_00_FF_00,
            Color::Blue => 0xFF_00_00_FF,
            Color::Yellow => 0xFF_FF_FF_00,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Pen(Color),
    Eraser,
}

impl Tool {
    fn argb(self) -> u32 {
        match self {
            Tool::Pen(c) => c.to_argb(),
            Tool::Eraser => 0x00_00_00_00,
        }
    }

    fn radius(self) -> i32 {
        match self {
            Tool::Pen(_) => 1,
            Tool::Eraser => 10,
        }
    }
}

struct InputState {
    pressed: bool,
    pos: Coord,
    prev: Option<Coord>,
}

impl InputState {
    fn new() -> Self {
        Self {
            pressed: false,
            pos: Coord { x: 0.0, y: 0.0 },
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
    strokes: Vec<(Coord, Coord, Tool)>,
    pointer: InputState,
    tablet: InputState,
    tool: Tool,
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
            tool: Tool::Pen(Color::Red),
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
        for &(from, to, tool) in &self.strokes {
            draw_line_on_buffer(self.shm_ptr.0, width, height, from, to, tool);
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

    /// Clear strokes and redraw the buffer (fill with transparent), then
    /// damage + commit the surface so the change is visible immediately.
    pub fn clear_and_redraw(&mut self, wl_surface: &wl_surface::WlSurface) {
        self.strokes.clear();
        self.reset_input();
        if !self.shm_ptr.0.is_null() {
            let count = (self.width as usize) * (self.height as usize);
            let pixels =
                unsafe { std::slice::from_raw_parts_mut(self.shm_ptr.0 as *mut u32, count) };
            for pixel in pixels.iter_mut() {
                *pixel = 0x00_00_00_00;
            }
            wl_surface.damage_buffer(0, 0, self.width as i32, self.height as i32);
            wl_surface.commit();
        }
    }

    /// Reset transient input state without clearing strokes.
    pub fn reset_input(&mut self) {
        self.pointer.reset();
        self.tablet.reset();
    }

    pub fn tool(&self) -> Tool {
        self.tool
    }

    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool;
    }

    pub fn set_eraser(&mut self) {
        self.tool = Tool::Eraser;
    }

    /// Record a line segment, draw it into the pixel buffer, and
    /// damage + commit the surface.
    fn draw_stroke(&mut self, from: Coord, to: Coord, wl_surface: &wl_surface::WlSurface) {
        let tool = self.tool;
        self.strokes.push((from, to, tool));
        if self.shm_ptr.0.is_null() {
            return;
        }
        draw_line_on_buffer(self.shm_ptr.0, self.width, self.height, from, to, tool);
        wl_surface.damage_buffer(0, 0, self.width as i32, self.height as i32);
        wl_surface.commit();
    }

    pub fn pointer_enter(&mut self, x: f64, y: f64) {
        self.pointer.pos = Coord { x, y };
    }

    pub fn pointer_leave(&mut self) {
        self.pointer.reset();
    }

    pub fn pointer_motion(&mut self, x: f64, y: f64) {
        self.pointer.pos = Coord { x, y };
    }

    pub fn pointer_button(&mut self, pressed: bool) {
        self.pointer.pressed = pressed;
        if pressed {
            self.pointer.prev = Some(self.pointer.pos);
        } else {
            self.pointer.prev = None;
        }
    }

    pub fn pointer_frame(&mut self, wl_surface: &wl_surface::WlSurface) {
        if self.pointer.pressed {
            let pos = self.pointer.pos;
            if let Some(prev) = self.pointer.prev
                && prev != pos
            {
                self.draw_stroke(prev, pos, wl_surface);
            }
            self.pointer.prev = Some(pos);
        }
    }

    pub fn tablet_proximity_out(&mut self) {
        self.tablet.reset();
    }

    pub fn tablet_down(&mut self) {
        self.tablet.pressed = true;
        self.tablet.prev = Some(self.tablet.pos);
    }

    pub fn tablet_up(&mut self) {
        self.tablet.reset();
    }

    pub fn tablet_motion(&mut self, x: f64, y: f64) {
        self.tablet.pos = Coord { x, y };
    }

    pub fn tablet_frame(&mut self, wl_surface: &wl_surface::WlSurface) {
        if self.tablet.pressed {
            let pos = self.tablet.pos;
            if let Some(prev) = self.tablet.prev
                && prev != pos
            {
                self.draw_stroke(prev, pos, wl_surface);
            }
            self.tablet.prev = Some(pos);
        }
    }
}

impl Drop for Canvas {
    fn drop(&mut self) {
        self.unmap();
    }
}

fn draw_line_on_buffer(ptr: *mut u8, width: u32, height: u32, from: Coord, to: Coord, tool: Tool) {
    let pixels =
        unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, (width * height) as usize) };
    let color = tool.argb();
    let radius = tool.radius();

    let mut ix0 = from.x as i64;
    let mut iy0 = from.y as i64;
    let ix1 = to.x as i64;
    let iy1 = to.y as i64;
    let dx = (ix1 - ix0).abs();
    let dy = -(iy1 - iy0).abs();
    let sx: i64 = if ix0 < ix1 { 1 } else { -1 };
    let sy: i64 = if iy0 < iy1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        // Draw a filled circle at (ix0, iy0).
        for by in -radius..=radius {
            for bx in -radius..=radius {
                if bx * bx + by * by <= radius * radius {
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
