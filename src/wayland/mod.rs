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
mod tablet;

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
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
};

use crate::model::Waydoodle;
use crate::tray::WaydoodleTray;

use smithay_client_toolkit::shell::xdg::window::Window;

pub(crate) const LEFT_BUTTON: u32 = 0x110;

#[derive(Clone, Copy)]
pub(crate) struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl DirtyRect {
    pub fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width: width as i32,
            height: height as i32,
        }
    }
}

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
    cursor_shape_manager: CursorShapeManager,
    eraser_cursor: cursor::Cursor,
    tablet_cursor: Option<cursor::TabletCursorState>,

    // Tablet input
    tablet_manager: Option<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
    tablet_seat: Option<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    tablet_tool: Option<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    tablet_tool_serial: u32,
    tablet_pos: (f64, f64),
    tablet_pressed: bool,

    // Overlay window state
    window: Option<Window>,
    pool: Option<SlotPool>,
    buffer: Option<Buffer>,
    width: u32,
    height: u32,
    first_configure: bool,

    // Pointer tracking
    pointer_enter_serial: u32,
    pointer_pos: (f64, f64),
    pointer_pressed: bool,

    // Application model
    model: Waydoodle,

    // Tray
    tray_handle: Option<ksni::blocking::Handle<WaydoodleTray>>,

    // Event loop
    loop_handle: calloop::LoopHandle<'static, Self>,
}
