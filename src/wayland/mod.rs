//! Wayland view layer.
//!
//! This module implements the GUI using smithay-client-toolkit (SCTK). It owns
//! the Wayland connection, event loop, SHM pool, XDG window, and input devices.

mod app;
mod cursors;
mod handlers;
mod overlay;
mod render;
mod tablet;

use cursors::Cursors;
use ksni::blocking::Handle;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::xdg::{XdgShell, window::Window},
    shm::Shm,
};
use tablet::TabletState;
use wayland_client::protocol::{wl_keyboard, wl_pointer};

use crate::{tray::WaydoodleTray, wayland::overlay::Overlay};

struct PointerState {
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

/// When we create an overlay, we first create the XDG window and enter the
/// Pending state. This allows the compositor to set up the surface and send us
/// the configure event with the initial size before we create the SHM buffers.
/// Once we receive the configure event, we create the buffers and transition to
/// the Ready state.
enum OverlayState {
    Pending(Window),
    Ready(Overlay),
}

impl OverlayState {
    fn window(&self) -> &Window {
        match self {
            OverlayState::Pending(w) => w,
            OverlayState::Ready(o) => &o.window,
        }
    }
}

pub(crate) struct App {
    wayland: WaylandState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    cursors: Cursors,
    tablet: Option<TabletState>,
    overlay: Option<OverlayState>,
    pointer: Option<PointerState>,
    tray_handle: Option<Handle<WaydoodleTray>>,
    loop_handle: calloop::LoopHandle<'static, Self>,
    queue_handle: wayland_client::QueueHandle<Self>,
}
