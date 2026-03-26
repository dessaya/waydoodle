use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop, event_created_child};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
    zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
    zwp_tablet_v2,
};

use crate::{
    canvas::Point,
    waydoodle::{self, Overlay as _, OverlayTool as _},
    wayland::App,
};

use super::{OverlayState, cursors::TabletCursorState};

pub(super) struct ActiveTabletTool {
    pub tool: zwp_tablet_tool_v2::ZwpTabletToolV2,
    pub serial: u32,
}

pub(super) struct TabletState {
    pub wl_seat: wl_seat::WlSeat,
    /// Kept alive so the compositor continues sending tablet events for this seat.
    pub _seat: zwp_tablet_seat_v2::ZwpTabletSeatV2,
    pub cursor: TabletCursorState,
    pub active_tool: Option<ActiveTabletTool>,
    pub model: waydoodle::PointerState,
}

delegate_noop!(App: ignore zwp_tablet_manager_v2::ZwpTabletManagerV2);

impl Dispatch<zwp_tablet_seat_v2::ZwpTabletSeatV2, ()> for App {
    event_created_child!(App, zwp_tablet_seat_v2::ZwpTabletSeatV2, [
        zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (zwp_tablet_v2::ZwpTabletV2, ()),
        zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (zwp_tablet_tool_v2::ZwpTabletToolV2, ()),
        zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (zwp_tablet_pad_v2::ZwpTabletPadV2, ()),
    ]);

    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
        _event: zwp_tablet_seat_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_tablet_tool_v2::ZwpTabletToolV2, ()> for App {
    fn event(
        state: &mut Self,
        tool: &zwp_tablet_tool_v2::ZwpTabletToolV2,
        event: zwp_tablet_tool_v2::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_tablet_tool_v2::Event::ProximityIn {
                serial,
                tablet: _,
                surface: _,
            } => {
                let Some(tablet) = state.tablets.iter_mut().find(|t| t.active_tool.is_none())
                else {
                    return;
                };
                tablet.active_tool = Some(ActiveTabletTool {
                    tool: tool.clone(),
                    serial,
                });
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    overlay.on_pointer_enter(&mut tablet.model, Point { x: 0.0, y: 0.0 });
                    let shape = overlay.current_tool().cursor_shape();
                    state.apply_cursor(shape, qh);
                }
            }
            zwp_tablet_tool_v2::Event::ProximityOut => {
                let Some(tablet) = state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                else {
                    return;
                };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    overlay.on_pointer_leave(&mut tablet.model);
                }
                tablet.active_tool = None;
            }
            zwp_tablet_tool_v2::Event::Down { .. } => {
                let Some(tablet) = state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                else {
                    return;
                };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    let pos = tablet.model.pos;
                    let damage = overlay.on_pointer_press(&mut tablet.model, pos);
                    overlay.mark_dirty(qh, damage);
                }
            }
            zwp_tablet_tool_v2::Event::Up => {
                let Some(tablet) = state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                else {
                    return;
                };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    overlay.on_pointer_release(&mut tablet.model);
                }
            }
            zwp_tablet_tool_v2::Event::Motion { x, y } => {
                let Some(tablet) = state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                else {
                    return;
                };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut()
                    && let Some(damage) =
                        overlay.on_pointer_motion(&mut tablet.model, Point { x, y })
                {
                    overlay.mark_dirty(qh, damage);
                }
            }
            zwp_tablet_tool_v2::Event::Pressure { .. } => {}
            zwp_tablet_tool_v2::Event::Frame { .. } => {}
            zwp_tablet_tool_v2::Event::Removed => {
                for tablet in &mut state.tablets {
                    if tablet.active_tool.as_ref().is_some_and(|a| &a.tool == tool) {
                        tablet.active_tool = None;
                    }
                }
                tool.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_tablet_v2::ZwpTabletV2, ()> for App {
    fn event(
        _state: &mut Self,
        tablet: &zwp_tablet_v2::ZwpTabletV2,
        event: zwp_tablet_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwp_tablet_v2::Event::Removed = event {
            tablet.destroy();
        }
    }
}

impl Dispatch<zwp_tablet_pad_v2::ZwpTabletPadV2, ()> for App {
    event_created_child!(App, zwp_tablet_pad_v2::ZwpTabletPadV2, [
        zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()),
    ]);

    fn event(
        _state: &mut Self,
        pad: &zwp_tablet_pad_v2::ZwpTabletPadV2,
        event: zwp_tablet_pad_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwp_tablet_pad_v2::Event::Removed = event {
            pad.destroy();
        }
    }
}

impl Dispatch<zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()> for App {
    event_created_child!(App, zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, [
        zwp_tablet_pad_group_v2::EVT_RING_OPCODE => (zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, ()),
        zwp_tablet_pad_group_v2::EVT_STRIP_OPCODE => (zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2, ()),
    ]);

    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2,
        _event: zwp_tablet_pad_group_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(App: ignore zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2);
delegate_noop!(App: ignore zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2);
