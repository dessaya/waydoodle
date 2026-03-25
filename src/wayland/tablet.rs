use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop, event_created_child};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
    zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
    zwp_tablet_v2,
};

use crate::{
    canvas::Point,
    waydoodle::{Overlay as _, OverlayTool as _},
    wayland::App,
};

use super::{OverlayState, cursors::TabletCursorState};

pub(super) struct ActiveTabletTool {
    pub tool: zwp_tablet_tool_v2::ZwpTabletToolV2,
    pub serial: u32,
}

pub(super) struct TabletState {
    pub manager: zwp_tablet_manager_v2::ZwpTabletManagerV2,
    pub seat: Option<zwp_tablet_seat_v2::ZwpTabletSeatV2>,
    pub cursor: TabletCursorState,
    pub active_tool: Option<ActiveTabletTool>,
    pub pos: (f64, f64),
    pub pressed: bool,
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
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                tablet.active_tool = Some(ActiveTabletTool {
                    tool: tool.clone(),
                    serial,
                });
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_ref() {
                    let shape = overlay.current_tool().cursor_shape();
                    state.apply_cursor(shape, qh);
                }
            }
            zwp_tablet_tool_v2::Event::ProximityOut => {
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                tablet.pressed = false;
            }
            zwp_tablet_tool_v2::Event::Down { .. } => {
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                let pos = tablet.pos;
                tablet.pressed = true;

                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    let center = Point { x: pos.0, y: pos.1 };
                    let damage = overlay.on_press(center);
                    overlay.mark_dirty(qh, damage);
                }
            }
            zwp_tablet_tool_v2::Event::Up => {
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                tablet.pressed = false;
            }
            zwp_tablet_tool_v2::Event::Motion { x, y } => {
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                let prev = tablet.pos;
                let pressed = tablet.pressed;
                tablet.pos = (x, y);

                if pressed {
                    if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                        let from = Point {
                            x: prev.0,
                            y: prev.1,
                        };
                        let to = Point { x, y };
                        let damage = overlay.on_drag(from, to);
                        overlay.mark_dirty(qh, damage);
                    }
                }
            }
            zwp_tablet_tool_v2::Event::Pressure { .. } => {}
            zwp_tablet_tool_v2::Event::Frame { .. } => {}
            zwp_tablet_tool_v2::Event::Removed => {
                let Some(tablet) = state.tablet.as_mut() else {
                    return;
                };
                if tablet.active_tool.as_ref().is_some_and(|a| &a.tool == tool) {
                    tablet.active_tool = None;
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
