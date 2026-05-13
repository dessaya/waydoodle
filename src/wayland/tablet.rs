use wayland_client::protocol::wl_seat;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop, event_created_child};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
    zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2,
    zwp_tablet_tool_v2::{self, ButtonState},
    zwp_tablet_v2,
};

use crate::waydoodle::InputButton;
use crate::{canvas::Point, wayland::App};

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
    pub pos: Point,
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
                    let shape = overlay.state.on_pointer_enter();
                    state.apply_cursor(shape);
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
                    overlay.state.on_pointer_leave();
                }
                tablet.active_tool = None;
            }
            zwp_tablet_tool_v2::Event::Button {
                state: btn_state,
                button: btn,
                ..
            } => {
                let Some(tablet) = state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                else {
                    return;
                };
                let pressed = btn_state
                    .into_result()
                    .is_ok_and(|s| s == ButtonState::Pressed);
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    // from linux/input-event-codes.h
                    const BTN_STYLUS: u32 = 0x14b;
                    let input_btn = if btn == BTN_STYLUS {
                        InputButton::Tertiary
                    } else {
                        InputButton::Secondary
                    };
                    let (keep_open, redraw, cursor_shape) = if pressed {
                        overlay
                            .state
                            .on_pointer_button_pressed(tablet.pos, input_btn)
                    } else {
                        overlay
                            .state
                            .on_pointer_button_released(tablet.pos, input_btn)
                    };
                    state.handle_overlay_event_result(keep_open, redraw, cursor_shape);
                }
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
                    let damage = overlay.state.begin_stroke(tablet.pos);
                    overlay.mark_dirty(qh, damage);
                }
            }
            zwp_tablet_tool_v2::Event::Up => {
                if state
                    .tablets
                    .iter_mut()
                    .find(|t| t.active_tool.as_ref().is_some_and(|a| &a.tool == tool))
                    .is_none()
                {
                    return;
                };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut() {
                    overlay.state.end_stroke();
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
                tablet.pos = Point { x, y };
                if let Some(OverlayState::Ready(overlay)) = state.overlay.as_mut()
                    && let Some(damage) = overlay.state.on_pointer_motion(tablet.pos)
                {
                    overlay.mark_dirty(qh, damage);
                }
            }
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
