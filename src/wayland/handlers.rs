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
        pointer::{
            BTN_LEFT, BTN_MIDDLE, BTN_RIGHT, PointerEvent, PointerEventKind, PointerHandler,
        },
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

use crate::{canvas::Point, waydoodle::InputButton, wayland::App};

use super::{OverlayState, PointerState, cursors::TabletCursorState, tablet::TabletState};

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
    fn configure(&mut self, width: u32, height: u32, _: &QueueHandle<Self>) {
        log::debug!(
            "Received configure event for overlay window: {}x{}",
            width,
            height,
        );
        self.on_configure(width, height);
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
            let device = self.cursors.shape_manager.get_shape_device(&wl_pointer, qh);
            self.pointers.push(PointerState {
                seat: seat.clone(),
                wl_pointer,
                device,
                enter_serial: 0,
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
                pos: Point { x: 0.0, y: 0.0 },
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
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() else {
            return;
        };
        let (keep_open, redraw, shape) = overlay.state.on_key_pressed(event.keysym);
        self.handle_overlay_event_result(keep_open, redraw, shape);
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
                        let shape = overlay.state.on_pointer_enter();
                        self.apply_cursor(shape);
                    }
                }
                PointerEventKind::Leave { .. } => {
                    if self
                        .pointers
                        .iter_mut()
                        .find(|p| &p.wl_pointer == pointer)
                        .is_none()
                    {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        overlay.state.on_pointer_leave();
                    }
                }
                PointerEventKind::Motion { .. } => {
                    if self
                        .pointers
                        .iter_mut()
                        .find(|p| &p.wl_pointer == pointer)
                        .is_none()
                    {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        let pos = Point {
                            x: event.position.0,
                            y: event.position.1,
                        };
                        if let Some(damage) = overlay.state.on_pointer_motion(pos) {
                            overlay.mark_dirty(qh, damage);
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    if self
                        .pointers
                        .iter_mut()
                        .find(|p| &p.wl_pointer == pointer)
                        .is_none()
                    {
                        continue;
                    };
                    let Some((input_btn, pos)) = input_btn_pos(button, event) else {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        let (keep_open, redraw, cursor_shape) =
                            overlay.state.on_pointer_button_pressed(pos, input_btn);
                        if keep_open && !redraw && input_btn != InputButton::Secondary {
                            let damage = overlay.state.begin_stroke(pos);
                            overlay.mark_dirty(qh, damage);
                        }
                        self.handle_overlay_event_result(keep_open, redraw, cursor_shape);
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    if self
                        .pointers
                        .iter_mut()
                        .find(|p| &p.wl_pointer == pointer)
                        .is_none()
                    {
                        continue;
                    };
                    let Some((input_btn, pos)) = input_btn_pos(button, event) else {
                        continue;
                    };
                    if let Some(OverlayState::Ready(overlay)) = self.overlay.as_mut() {
                        let (keep_open, redraw, cursor_shape) =
                            overlay.state.on_pointer_button_released(pos, input_btn);
                        if keep_open && !redraw && input_btn != InputButton::Secondary {
                            overlay.state.end_stroke();
                        }
                        self.handle_overlay_event_result(keep_open, redraw, cursor_shape);
                    }
                }
                PointerEventKind::Axis { .. } => {}
            }
        }
    }
}

fn input_btn_pos(button: u32, event: &PointerEvent) -> Option<(InputButton, Point)> {
    Some((
        match button {
            BTN_LEFT => InputButton::Primary,
            BTN_RIGHT => InputButton::Secondary,
            BTN_MIDDLE => InputButton::Tertiary,
            _ => return None,
        },
        Point {
            x: event.position.0,
            y: event.position.1,
        },
    ))
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
