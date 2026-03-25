//! Wayland view layer.
//!
//! This module implements the GUI using smithay-client-toolkit (SCTK). It owns
//! the Wayland connection, event loop, SHM pool, XDG window, and input devices.
//! The view drives the [`model::Waydoodle`] model and interprets the
//! [`model::Command`] values it returns.

mod commands;
mod cursors;
mod drawing;
mod handlers;
mod help;
mod init;
mod render;
mod tablet;

use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::xdg::{XdgShell, window::Window},
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use wayland_client::protocol::{wl_keyboard, wl_pointer};

use crate::{model::Waydoodle, tray::WaydoodleTray};
use cursors::Cursors;
use render::DirtyRect;
use tablet::TabletState;

pub(crate) struct PointerState {
    pub wl_pointer: wl_pointer::WlPointer,
    pub enter_serial: u32,
    pub pos: (f64, f64),
    pub pressed: bool,
}

struct WaylandState {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub xdg_shell: XdgShell,
    pub shm: Shm,
}

pub(crate) struct ViewOverlay {
    pub window: Window,
    pub pool: SlotPool,
    pub buffers: [Buffer; 2],
    /// Index of the buffer currently attached to (owned by) the compositor.
    pub front: usize,
    pub width: u32,
    pub height: u32,
    pub pending_damage: Option<DirtyRect>,
    pub frame_requested: bool,
}

pub(crate) enum OverlayState {
    Pending(Window),
    Ready(ViewOverlay),
}

impl OverlayState {
    pub fn window(&self) -> &Window {
        match self {
            OverlayState::Pending(w) => w,
            OverlayState::Ready(o) => &o.window,
        }
    }
}

pub struct View {
    wayland: WaylandState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    cursors: Cursors,
    tablet: Option<TabletState>,
    overlay: Option<OverlayState>,
    pointer: Option<PointerState>,
    tray_handle: Option<ksni::blocking::Handle<WaydoodleTray>>,
    loop_handle: calloop::LoopHandle<'static, Self>,
    model: Waydoodle,
}
