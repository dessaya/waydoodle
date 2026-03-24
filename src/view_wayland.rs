//! Wayland view layer.
//!
//! This module implements the GUI using smithay-client-toolkit (SCTK). It owns
//! the Wayland connection, event loop, SHM pool, XDG window, and input devices.
//! The view drives the [`model::Waydoodle`] model and interprets the
//! [`model::Command`] values it returns.

use std::time::Duration;

use calloop::signals::{Signal, Signals};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    reexports::calloop::{self, EventLoop},
    reexports::calloop_wayland_source::WaylandSource,
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        xdg::{
            XdgShell,
            window::{Window, WindowConfigure, WindowDecorations, WindowHandler},
        },
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

use crate::model::{BrushStyle, Color, Command, Point, Tool, Waydoodle};

const LEFT_BUTTON: u32 = 0x110;

pub struct View {
    // Wayland state
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    compositor_state: CompositorState,
    xdg_shell: XdgShell,
    shm: Shm,

    // Input devices
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,

    // Overlay window state
    window: Option<Window>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
    width: u32,
    height: u32,
    first_configure: bool,
    dirty: bool,

    // Pointer tracking
    pointer_pos: (f64, f64),
    pointer_pressed: bool,

    // Application model
    model: Waydoodle,

    // Event loop
    loop_handle: calloop::LoopHandle<'static, Self>,
    exit: bool,
}

impl View {
    pub fn run() {
        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
        let (globals, event_queue) =
            registry_queue_init(&conn).expect("Failed to initialize registry");
        let qh = event_queue.handle();

        let mut event_loop: EventLoop<View> =
            EventLoop::try_new().expect("Failed to create event loop");
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("Failed to insert Wayland source");

        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");

        let mut view = View {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            compositor_state,
            xdg_shell,
            shm,

            keyboard: None,
            pointer: None,

            window: None,
            pool: None,
            buffer: None,
            width: 0,
            height: 0,
            first_configure: false,
            dirty: false,

            pointer_pos: (0.0, 0.0),
            pointer_pressed: false,

            model: Waydoodle::new(),

            loop_handle: event_loop.handle(),
            exit: false,
        };

        // Register SIGUSR1 handler to toggle the overlay.
        let sigusr1 = Signals::new(&[Signal::SIGUSR1]).expect("Failed to register SIGUSR1");
        let qh_clone = qh.clone();
        event_loop
            .handle()
            .insert_source(sigusr1, move |_, _, view| {
                let cmds = view.model.toggle_overlay();
                view.dispatch_commands(&qh_clone, cmds);
            })
            .expect("Failed to insert signal source");

        loop {
            event_loop
                .dispatch(Duration::from_millis(16), &mut view)
                .expect("Event loop dispatch failed");

            if view.exit {
                break;
            }
        }
    }

    fn dispatch_commands(&mut self, qh: &QueueHandle<Self>, cmds: Vec<Command>) {
        for cmd in cmds {
            self.dispatch_command(qh, cmd);
        }
    }

    fn dispatch_command(&mut self, qh: &QueueHandle<Self>, cmd: Command) {
        match cmd {
            Command::ShowOverlay => self.show_overlay(qh),
            Command::HideOverlay => self.hide_overlay(),
            Command::SetCrosshairCursor => {
                // TODO: set cursor shape via wp_cursor_shape or themed pointer
                log::debug!("SetCrosshairCursor");
            }
            Command::SetCircleCursor => {
                // TODO: set custom cursor (hollow circle via SHM surface)
                log::debug!("SetCircleCursor");
            }
            Command::DrawLine {
                style,
                radius,
                from,
                to,
            } => self.render_line(style, radius, from, to),
            Command::DrawDot {
                style,
                radius,
                center,
            } => self.render_dot(style, radius, center),
            Command::ClearBuffer => self.clear_buffer(),
        }

        if self.dirty {
            self.draw_frame(qh);
        }
    }

    fn show_overlay(&mut self, qh: &QueueHandle<Self>) {
        if self.window.is_some() {
            self.hide_overlay();
        }

        let surface = self.compositor_state.create_surface(qh);
        let window = self
            .xdg_shell
            .create_window(surface, WindowDecorations::None, qh);

        window.set_title("Waydoodle");
        window.set_app_id("io.github.dessaya.waydoodle");
        window.set_maximized();
        window.commit();

        self.window = Some(window);
        self.first_configure = true;
        self.buffer = None;
        self.dirty = true;
    }

    fn hide_overlay(&mut self) {
        if let Some(window) = self.window.take() {
            let surface = window.wl_surface().clone();
            drop(window);
            surface.destroy();
        }
        self.buffer = None;
        self.pool = None;
        self.width = 0;
        self.height = 0;
    }

    fn ensure_pool(&mut self) {
        if self.pool.is_none() && self.width > 0 && self.height > 0 {
            let size = self.width as usize * self.height as usize * 4;
            self.pool =
                Some(SlotPool::new(size, &self.shm).expect("Failed to create SHM slot pool"));
        }
    }

    fn canvas_mut(&mut self) -> Option<&mut [u8]> {
        let buffer = self.buffer.as_ref()?;
        let pool = self.pool.as_mut()?;
        pool.canvas(buffer)
    }

    fn draw_frame(&mut self, qh: &QueueHandle<Self>) {
        if self.window.is_none() || self.width == 0 || self.height == 0 {
            return;
        }

        self.ensure_pool();

        let width = self.width;
        let height = self.height;
        let stride = width as i32 * 4;
        let pool = self.pool.as_mut().unwrap();

        let buffer = self.buffer.get_or_insert_with(|| {
            let (buf, canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("Failed to create SHM buffer");
            // Start fully transparent
            canvas.fill(0);
            buf
        });

        if pool.canvas(buffer).is_none() {
            // Compositor hasn't released previous buffer; create a new one.
            let (new_buffer, canvas) = pool
                .create_buffer(
                    width as i32,
                    height as i32,
                    stride,
                    wl_shm::Format::Argb8888,
                )
                .expect("Failed to create SHM buffer");
            canvas.fill(0);
            *buffer = new_buffer;
        }

        let window = self.window.as_ref().unwrap();
        let surface = window.wl_surface();

        surface.damage_buffer(0, 0, width as i32, height as i32);
        surface.frame(qh, surface.clone());
        buffer.attach_to(surface).expect("Failed to attach buffer");
        window.commit();

        self.dirty = false;
    }

    fn clear_buffer(&mut self) {
        if let Some(canvas) = self.canvas_mut() {
            canvas.fill(0);
            self.dirty = true;
        }
    }

    fn color_to_argb_le(color: Color) -> [u8; 4] {
        let argb: u32 = match color {
            Color::Red => 0xFFFF0000,
            Color::Green => 0xFF00FF00,
            Color::Blue => 0xFF0000FF,
            Color::Yellow => 0xFFFFFF00,
        };
        argb.to_le_bytes()
    }

    fn set_pixel(canvas: &mut [u8], width: u32, height: u32, x: i32, y: i32, pixel: [u8; 4]) {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let offset = (y as usize * width as usize + x as usize) * 4;
        canvas[offset..offset + 4].copy_from_slice(&pixel);
    }

    fn fill_circle(
        canvas: &mut [u8],
        width: u32,
        height: u32,
        cx: f64,
        cy: f64,
        radius: f64,
        pixel: [u8; 4],
    ) {
        let r = radius.ceil() as i32;
        let cx_i = cx.round() as i32;
        let cy_i = cy.round() as i32;
        let r_sq = radius * radius;
        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = (dx as f64 - (cx - cx_i as f64)).powi(2)
                    + (dy as f64 - (cy - cy_i as f64)).powi(2);
                if dist_sq <= r_sq {
                    Self::set_pixel(canvas, width, height, cx_i + dx, cy_i + dy, pixel);
                }
            }
        }
    }

    fn stroke(
        canvas: &mut [u8],
        width: u32,
        height: u32,
        x0: f64,
        y0: f64,
        x1: f64,
        y1: f64,
        radius: f64,
        pixel: [u8; 4],
    ) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let dist = dx.hypot(dy);
        let steps = (dist / 0.5).ceil().max(1.0) as usize;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let x = x0 + dx * t;
            let y = y0 + dy * t;
            Self::fill_circle(canvas, width, height, x, y, radius, pixel);
        }
    }

    fn brush_pixel(style: BrushStyle) -> [u8; 4] {
        match style {
            BrushStyle::Draw(color) => Self::color_to_argb_le(color),
            BrushStyle::Erase => [0, 0, 0, 0],
        }
    }

    fn render_line(&mut self, style: BrushStyle, radius: f64, from: Point, to: Point) {
        let width = self.width;
        let height = self.height;
        if let Some(canvas) = self.canvas_mut() {
            let pixel = Self::brush_pixel(style);
            Self::stroke(
                canvas, width, height, from.x, from.y, to.x, to.y, radius, pixel,
            );
            self.dirty = true;
        }
    }

    fn render_dot(&mut self, style: BrushStyle, radius: f64, center: Point) {
        let width = self.width;
        let height = self.height;
        if let Some(canvas) = self.canvas_mut() {
            let pixel = Self::brush_pixel(style);
            Self::fill_circle(canvas, width, height, center.x, center.y, radius, pixel);
            self.dirty = true;
        }
    }

    fn handle_key(&mut self, qh: &QueueHandle<Self>, keysym: Keysym) {
        let overlay = match self.model.overlay.as_mut() {
            Some(o) => o,
            None => return,
        };

        let cmd = match keysym {
            Keysym::r | Keysym::R => Some(overlay.set_tool(Tool::Pen(Color::Red))),
            Keysym::g | Keysym::G => Some(overlay.set_tool(Tool::Pen(Color::Green))),
            Keysym::b | Keysym::B => Some(overlay.set_tool(Tool::Pen(Color::Blue))),
            Keysym::y | Keysym::Y => Some(overlay.set_tool(Tool::Pen(Color::Yellow))),
            Keysym::e | Keysym::E => Some(overlay.set_tool(Tool::Eraser)),
            Keysym::c | Keysym::C => Some(overlay.clear()),
            _ => None,
        };

        if let Some(cmd) = cmd {
            self.dispatch_command(qh, cmd);
        }
    }
}

impl CompositorHandler for View {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        if self.dirty {
            self.draw_frame(qh);
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for View {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for View {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let new_width = configure.new_size.0.map(|v| v.get()).unwrap_or(self.width);
        let new_height = configure.new_size.1.map(|v| v.get()).unwrap_or(self.height);

        if new_width != self.width || new_height != self.height {
            self.width = new_width;
            self.height = new_height;
            self.buffer = None;
            self.pool = None;
        }

        if self.first_configure {
            self.first_configure = false;
        }

        self.dirty = true;
        self.draw_frame(qh);
    }
}

impl SeatHandler for View {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .seat_state
                .get_keyboard_with_repeat(
                    qh,
                    &seat,
                    None,
                    self.loop_handle.clone(),
                    Box::new(|_state, _wl_kbd, _event| {}),
                )
                .expect("Failed to get keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to get pointer");
            self.pointer = Some(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            if let Some(kbd) = self.keyboard.take() {
                kbd.release();
            }
        }
        if capability == Capability::Pointer {
            if let Some(ptr) = self.pointer.take() {
                ptr.release();
            }
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
    }
}

impl KeyboardHandler for View {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.handle_key(qh, event.keysym);
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl PointerHandler for View {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        let window_surface = match self.window.as_ref() {
            Some(w) => w.wl_surface().clone(),
            None => return,
        };

        for event in events {
            if event.surface != window_surface {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { .. } => {
                    self.pointer_pos = event.position;
                }
                PointerEventKind::Leave { .. } => {
                    self.pointer_pressed = false;
                }
                PointerEventKind::Motion { .. } => {
                    let prev = self.pointer_pos;
                    self.pointer_pos = event.position;

                    if self.pointer_pressed {
                        if let Some(overlay) = self.model.overlay.as_ref() {
                            let from = Point {
                                x: prev.0,
                                y: prev.1,
                            };
                            let to = Point {
                                x: event.position.0,
                                y: event.position.1,
                            };
                            let cmd = overlay.draw(from, to);
                            self.dispatch_command(qh, cmd);
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == LEFT_BUTTON {
                        self.pointer_pressed = true;
                        self.pointer_pos = event.position;

                        if let Some(overlay) = self.model.overlay.as_ref() {
                            let center = Point {
                                x: event.position.0,
                                y: event.position.1,
                            };
                            let cmd = overlay.draw_dot(center);
                            self.dispatch_command(qh, cmd);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == LEFT_BUTTON {
                        self.pointer_pressed = false;
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl ShmHandler for View {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(View);
delegate_output!(View);
delegate_shm!(View);
delegate_seat!(View);
delegate_keyboard!(View);
delegate_pointer!(View);
delegate_xdg_shell!(View);
delegate_xdg_window!(View);
delegate_registry!(View);

impl ProvidesRegistryState for View {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
