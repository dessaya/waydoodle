use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_output, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{BTN_LEFT, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        xdg::window::{Window, WindowConfigure, WindowHandler},
    },
    shm::{
        Shm, ShmHandler,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
};

use super::{DirtyRect, OverlayState, View, ViewOverlay};
use crate::model::Point;

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
        self.on_frame_callback(qh);
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
        &mut self.wayland.output_state
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

impl View {
    pub(super) fn create_overlay_pool_and_buffer(
        shm: &Shm,
        width: u32,
        height: u32,
    ) -> (SlotPool, Buffer) {
        let stride = width as i32 * 4;
        let size = width as usize * height as usize * 4;
        let mut pool = SlotPool::new(size, shm).expect("Failed to create SHM slot pool");
        let (buffer, canvas) = pool
            .create_buffer(
                width as i32,
                height as i32,
                stride,
                wl_shm::Format::Argb8888,
            )
            .expect("Failed to create SHM buffer");
        canvas.fill(0);
        (pool, buffer)
    }
}

impl WindowHandler for View {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let state = match self.overlay.take() {
            Some(s) => s,
            None => return,
        };

        match state {
            OverlayState::Pending(window) => {
                let width = configure.new_size.0.map(|v| v.get()).unwrap_or(1);
                let height = configure.new_size.1.map(|v| v.get()).unwrap_or(1);
                let (pool, buffer) =
                    Self::create_overlay_pool_and_buffer(&self.wayland.shm, width, height);
                self.overlay = Some(OverlayState::Ready(ViewOverlay {
                    window,
                    pool,
                    buffer,
                    width,
                    height,
                    pending_damage: None,
                    frame_requested: false,
                }));
                self.mark_dirty(qh, DirtyRect::full(width, height));
            }
            OverlayState::Ready(mut overlay) => {
                let new_width = configure
                    .new_size
                    .0
                    .map(|v| v.get())
                    .unwrap_or(overlay.width);
                let new_height = configure
                    .new_size
                    .1
                    .map(|v| v.get())
                    .unwrap_or(overlay.height);

                if new_width != overlay.width || new_height != overlay.height {
                    let (pool, buffer) = Self::create_overlay_pool_and_buffer(
                        &self.wayland.shm,
                        new_width,
                        new_height,
                    );
                    overlay.pool = pool;
                    overlay.buffer = buffer;
                    overlay.width = new_width;
                    overlay.height = new_height;

                    if let Some(cmd) = self.model.reset_overlay() {
                        self.dispatch_command(qh, cmd);
                    }
                }

                self.overlay = Some(OverlayState::Ready(overlay));
                self.mark_dirty(qh, DirtyRect::full(new_width, new_height));
            }
        }
    }
}

impl SeatHandler for View {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.wayland.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if let Some(ref mut tablet) = self.tablet
            && tablet.seat.is_none()
        {
            let tablet_seat = tablet.manager.get_tablet_seat(&seat, qh, ());
            tablet.seat = Some(tablet_seat);
            log::info!("Tablet seat created for seat");
        }

        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .wayland
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
            let wl_pointer = self
                .wayland
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to get pointer");

            self.pointer = Some(super::PointerState {
                wl_pointer,
                enter_serial: 0,
                pos: (0.0, 0.0),
                pressed: false,
            });
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(kbd) = self.keyboard.take()
        {
            kbd.release();
        }
        if capability == Capability::Pointer
            && let Some(ptr) = self.pointer.take()
        {
            ptr.wl_pointer.release();
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
        let window_surface = match self.overlay.as_ref() {
            Some(s) => s.window().wl_surface().clone(),
            None => return,
        };

        for event in events {
            if event.surface != window_surface {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { serial } => {
                    let Some(ptr) = self.pointer.as_mut() else {
                        continue;
                    };
                    ptr.enter_serial = serial;
                    ptr.pos = event.position;
                    if let Some(overlay) = self.model.overlay.as_ref() {
                        let shape = overlay.cursor_shape();
                        self.apply_cursor(shape, qh);
                    }
                }
                PointerEventKind::Leave { .. } => {
                    let Some(ptr) = self.pointer.as_mut() else {
                        continue;
                    };
                    ptr.pressed = false;
                }
                PointerEventKind::Motion { .. } => {
                    let Some(ptr) = self.pointer.as_mut() else {
                        continue;
                    };
                    let prev = ptr.pos;
                    ptr.pos = event.position;

                    if ptr.pressed
                        && let Some(overlay) = self.model.overlay.as_ref()
                    {
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
                PointerEventKind::Press { button, .. } => {
                    if button == BTN_LEFT {
                        let Some(ptr) = self.pointer.as_mut() else {
                            continue;
                        };
                        ptr.pressed = true;
                        ptr.pos = event.position;

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
                    if button == BTN_LEFT {
                        let Some(ptr) = self.pointer.as_mut() else {
                            continue;
                        };
                        ptr.pressed = false;
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl ShmHandler for View {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.wayland.shm
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
        &mut self.wayland.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
