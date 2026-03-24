//! Wayland view layer.
//!
//! This module implements the GUI using smithay-client-toolkit (SCTK). It owns
//! the Wayland connection, event loop, SHM pool, XDG window, and input devices.
//! The view drives the [`model::Waydoodle`] model and interprets the
//! [`model::Command`] values it returns.

mod commands;
mod cursor;
mod drawing;
mod handlers;
mod init;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::{SeatState, pointer::cursor_shape::CursorShapeManager},
    shell::xdg::XdgShell,
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::protocol::{wl_keyboard, wl_pointer};

use crate::model::Waydoodle;

use smithay_client_toolkit::shell::xdg::window::Window;

pub(crate) const LEFT_BUTTON: u32 = 0x110;

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
    cursor_shape_manager: Option<CursorShapeManager>,
    pointer_enter_serial: u32,
    eraser_cursor: Option<cursor::Cursor>,

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
