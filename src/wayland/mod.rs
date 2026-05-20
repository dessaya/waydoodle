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
    shell::{wlr_layer::LayerShell, xdg::XdgShell},
    shm::Shm,
};
use tablet::TabletState;
use wayland_client::protocol::{wl_keyboard, wl_pointer, wl_seat};
use wayland_protocols::wp::{
    cursor_shape::v1::client::wp_cursor_shape_device_v1::WpCursorShapeDeviceV1,
    tablet::zv2::client::zwp_tablet_manager_v2,
};

use crate::{
    tray::WaydoodleTray,
    wayland::overlay::{Overlay, WaylandWindow},
};

struct KeyboardState {
    pub seat: wl_seat::WlSeat,
    pub wl_keyboard: wl_keyboard::WlKeyboard,
}

struct PointerState {
    pub seat: wl_seat::WlSeat,
    pub wl_pointer: wl_pointer::WlPointer,
    pub device: WpCursorShapeDeviceV1,
    pub enter_serial: u32,
}

impl Drop for PointerState {
    fn drop(&mut self) {
        self.device.destroy();
    }
}

struct WaylandState {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub compositor_state: CompositorState,
    pub xdg_shell: XdgShell,
    pub layer_shell: Option<LayerShell>,
    pub shm: Shm,
}

/// When we create an overlay, we first create the XDG window and enter the
/// Pending state. This allows the compositor to set up the surface and send us
/// the configure event with the initial size before we create the SHM buffers.
/// Once we receive the configure event, we create the buffers and transition to
/// the Ready state.
#[allow(clippy::large_enum_variant)]
enum OverlayStatus {
    Pending(WaylandWindow),
    Ready(Overlay),
}

impl OverlayStatus {
    fn window(&self) -> &WaylandWindow {
        match self {
            OverlayStatus::Pending(w) => w,
            OverlayStatus::Ready(o) => &o.window,
        }
    }
}

pub(crate) struct App {
    wayland: WaylandState,
    overlay: Option<OverlayStatus>,
    cursors: Cursors,
    keyboards: Vec<KeyboardState>,
    pointers: Vec<PointerState>,
    tablets: Vec<TabletState>,
    tablet_manager: Option<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
    tray_handle: Option<Handle<WaydoodleTray>>,
    loop_handle: calloop::LoopHandle<'static, Self>,
    queue_handle: wayland_client::QueueHandle<Self>,
}
