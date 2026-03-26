use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_pointer,
    delegate_registry, delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{BTN_LEFT, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
        xdg::window::{Window, WindowConfigure, WindowHandler},
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, Proxy, QueueHandle,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
};

use crate::{
    canvas::{Point, Rect},
    waydoodle::{self, App as _, DEFAULT_TOOL, Overlay as _, OverlayTool as _},
    wayland::{App, Overlay},
};

use super::{OverlayState, cursors::TabletCursorState, tablet::TabletState};

impl CompositorHandler for App {
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
        if let Some(OverlayState::Ready(o)) = self.overlay.as_mut() {
            o.on_frame_callback(qh)
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

impl OutputHandler for App {
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

impl App {
    fn configure(&mut self, width: u32, height: u32, qh: &QueueHandle<Self>) {
        log::debug!(
            "Received configure event for overlay window: {}x{}",
            width,
            height,
        );
        match &mut self.overlay {
            Some(OverlayState::Pending(_)) => {
                // Transition Pending → Ready: take the Window out and build the full Overlay.
                let Some(OverlayState::Pending(window)) = self.overlay.take() else {
                    unreachable!();
                };
                let canvas_buf = vec![0u8; width as usize * height as usize * 4];
                let (pool, buffers) =
                    Self::create_overlay_pool_and_buffers(&self.wayland.shm, width, height);
                let mut overlay = Overlay {
                    window,
                    width,
                    height,
                    canvas_buf,
                    pool,
                    buffers,
                    stale: [None, None],
                    pending_damage: None,
                    frame_requested: false,
                    tool: DEFAULT_TOOL,
                    help: false,
                    history: Vec::new(),
                };
                overlay.mark_dirty(qh, Rect::new(width, height));
                self.overlay = Some(OverlayState::Ready(overlay));
            }
            Some(OverlayState::Ready(overlay)) => {
                if width != overlay.width || height != overlay.height {
                    log::debug!(
                        "Overlay window resized to {}x{} -- recreating SHM buffers",
                        width,
                        height
                    );
                    let canvas_buf = vec![0u8; width as usize * height as usize * 4];
                    let (pool, buffers) =
                        Self::create_overlay_pool_and_buffers(&self.wayland.shm, width, height);
                    overlay.canvas_buf = canvas_buf;
                    overlay.pool = pool;
                    overlay.buffers = buffers;
                    overlay.stale = [None, None];
                    overlay.width = width;
                    overlay.height = height;
                    if let Some(damage) = overlay.on_size_changed() {
                        overlay.mark_dirty(qh, damage);
                    }
                }
            }
            None => {}
        }
    }
}

impl WindowHandler for App {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _window: &Window,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let (Some(w), Some(h)) = configure.new_size else {
            return;
        };
        let (width, height) = (w.get(), h.get());
        self.configure(width, height, qh);
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.wayland.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && !self.keyboards.iter().any(|k| k.seat == seat) {
            let wl_keyboard = self
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
            self.keyboards.push(super::KeyboardState {
                seat: seat.clone(),
                wl_keyboard,
            });
        }

        if capability == Capability::Pointer && !self.pointers.iter().any(|p| p.seat == seat) {
            let wl_pointer = self
                .wayland
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to get pointer");

            self.pointers.push(super::PointerState {
                seat: seat.clone(),
                wl_pointer,
                enter_serial: 0,
                model: waydoodle::PointerState::new(),
            });
        }

        if let Some(ref manager) = self.tablet_manager
            && !self.tablets.iter().any(|t| t.wl_seat == seat)
        {
            let tablet_seat = manager.get_tablet_seat(&seat, qh, ());
            let cursor =
                TabletCursorState::new(conn, &self.wayland.compositor_state, &self.wayland.shm, qh);
            self.tablets.push(TabletState {
                wl_seat: seat.clone(),
                _seat: tablet_seat,
                cursor,
                active_tool: None,
                model: waydoodle::PointerState::new(),
            });
            log::info!("Tablet seat created for seat");
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            self.keyboards.retain(|k| {
                if k.seat == seat {
                    k.wl_keyboard.release();
                    false
                } else {
                    true
                }
            });
        }
        if capability == Capability::Pointer {
            self.pointers.retain(|p| {
                if p.seat == seat {
                    p.wl_pointer.release();
                    false
                } else {
                    true
                }
            });
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.keyboards.retain(|k| {
            if k.seat == seat {
                k.wl_keyboard.release();
                false
            } else {
                true
            }
        });
        self.pointers.retain(|p| {
            if p.seat == seat {
                p.wl_pointer.release();
                false
            } else {
                true
            }
        });
        self.tablets.retain(|t| t.wl_seat != seat);
    }
}

impl KeyboardHandler for App {
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
        let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() else {
            return;
        };
        let (keep_open, redraw) = overlay.on_key_pressed(event.keysym);
        let shape = overlay.current_tool().cursor_shape();
        if redraw {
            overlay.mark_dirty(qh, Rect::new(overlay.width, overlay.height));
        }
        if !keep_open {
            self.on_toggle_overlay();
        } else {
            self.apply_cursor(shape, qh);
        }
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

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        let window_surface_id = match self.overlay.as_ref() {
            Some(s) => s.window().wl_surface().id(),
            None => return,
        };

        for event in events {
            if event.surface.id() != window_surface_id {
                continue;
            }

            match event.kind {
                PointerEventKind::Enter { serial } => {
                    let Some(ptr) = self.pointers.iter_mut().find(|p| &p.wl_pointer == pointer)
                    else {
                        continue;
                    };
                    ptr.enter_serial = serial;
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        let pos = Point {
                            x: event.position.0,
                            y: event.position.1,
                        };
                        overlay.on_pointer_enter(&mut ptr.model, pos);
                        let shape = overlay.current_tool().cursor_shape();
                        self.apply_cursor(shape, qh);
                    }
                }
                PointerEventKind::Leave { .. } => {
                    let Some(ptr) = self.pointers.iter_mut().find(|p| &p.wl_pointer == pointer)
                    else {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        overlay.on_pointer_leave(&mut ptr.model);
                    }
                }
                PointerEventKind::Motion { .. } => {
                    let Some(ptr) = self.pointers.iter_mut().find(|p| &p.wl_pointer == pointer)
                    else {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        let pos = Point {
                            x: event.position.0,
                            y: event.position.1,
                        };
                        if let Some(damage) = overlay.on_pointer_motion(&mut ptr.model, pos) {
                            overlay.mark_dirty(qh, damage);
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if button == BTN_LEFT {
                        let Some(ptr) = self.pointers.iter_mut().find(|p| &p.wl_pointer == pointer)
                        else {
                            continue;
                        };
                        if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                            let pos = Point {
                                x: event.position.0,
                                y: event.position.1,
                            };
                            let damage = overlay.on_pointer_press(&mut ptr.model, pos);
                            overlay.mark_dirty(qh, damage);
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if button == BTN_LEFT {
                        let Some(ptr) = self.pointers.iter_mut().find(|p| &p.wl_pointer == pointer)
                        else {
                            continue;
                        };
                        if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                            overlay.on_pointer_release(&mut ptr.model);
                        }
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.wayland.shm
    }
}

impl LayerShellHandler for App {
    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        self.configure(width, height, qh);
    }

    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_keyboard!(App);
delegate_pointer!(App);
delegate_xdg_shell!(App);
delegate_xdg_window!(App);
delegate_registry!(App);
delegate_layer!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.wayland.registry_state
    }
    registry_handlers![OutputState, SeatState];
}
