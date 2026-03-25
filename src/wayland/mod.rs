//! Wayland view layer.
//!
//! This module implements the GUI using smithay-client-toolkit (SCTK). It owns
//! the Wayland connection, event loop, SHM pool, XDG window, and input devices.

pub mod app;
mod cursors;
mod handlers;

mod overlay;
mod render;
mod tablet;

use ksni::blocking::Handle;
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

use crate::{canvas::Rect, tray::WaydoodleTray, waydoodle::Tool};
use cursors::Cursors;
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

pub struct Overlay {
    pub window: Window,
    pub pool: SlotPool,
    pub buffers: [Buffer; 2],
    /// Index of the buffer currently attached to (owned by) the compositor.
    pub front: usize,
    pub width: u32,
    pub height: u32,
    pub pending_damage: Option<Rect>,
    pub frame_requested: bool,
    pub tool: Tool,
    pub help: bool,
}

pub enum OverlayState {
    Pending(Window),
    Ready(Overlay),
}

impl OverlayState {
    pub fn window(&self) -> &Window {
        match self {
            OverlayState::Pending(w) => w,
            OverlayState::Ready(o) => &o.window,
        }
    }
}

pub struct App {
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
