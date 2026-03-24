use wayland_client::event_created_child;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
    zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
    zwp_tablet_v2,
};

use crate::model::Point;

use super::View;

impl Dispatch<zwp_tablet_manager_v2::ZwpTabletManagerV2, ()> for View {
    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_manager_v2::ZwpTabletManagerV2,
        _event: zwp_tablet_manager_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_tablet_seat_v2::ZwpTabletSeatV2, ()> for View {
    event_created_child!(View, zwp_tablet_seat_v2::ZwpTabletSeatV2, [
        zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (zwp_tablet_v2::ZwpTabletV2, ()),
        zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (zwp_tablet_tool_v2::ZwpTabletToolV2, ()),
        zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (zwp_tablet_pad_v2::ZwpTabletPadV2, ()),
    ]);

    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
        event: zwp_tablet_seat_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwp_tablet_seat_v2::Event::ToolAdded { id: _ } => {
                log::info!("Tablet tool added");
            }
            zwp_tablet_seat_v2::Event::TabletAdded { id: _ } => {
                log::info!("Tablet added");
            }
            zwp_tablet_seat_v2::Event::PadAdded { id: _ } => {
                log::info!("Tablet pad added");
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_tablet_tool_v2::ZwpTabletToolV2, ()> for View {
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
                state.tablet_tool_serial = serial;
                state.tablet_tool = Some(tool.clone());
                state.apply_tablet_cursor(qh);
            }
            zwp_tablet_tool_v2::Event::ProximityOut => {
                state.tablet_pressed = false;
            }
            zwp_tablet_tool_v2::Event::Down { .. } => {
                state.tablet_pressed = true;

                if let Some(overlay) = state.model.overlay.as_ref() {
                    let center = Point {
                        x: state.tablet_pos.0,
                        y: state.tablet_pos.1,
                    };
                    let cmd = overlay.draw_dot(center);
                    state.dispatch_command(qh, cmd);
                }
            }
            zwp_tablet_tool_v2::Event::Up => {
                state.tablet_pressed = false;
            }
            zwp_tablet_tool_v2::Event::Motion { x, y } => {
                let prev = state.tablet_pos;
                state.tablet_pos = (x, y);

                if state.tablet_pressed {
                    if let Some(overlay) = state.model.overlay.as_ref() {
                        let from = Point {
                            x: prev.0,
                            y: prev.1,
                        };
                        let to = Point { x, y };
                        let cmd = overlay.draw(from, to);
                        state.dispatch_command(qh, cmd);
                    }
                }
            }
            zwp_tablet_tool_v2::Event::Pressure { .. } => {}
            zwp_tablet_tool_v2::Event::Frame { .. } => {}
            zwp_tablet_tool_v2::Event::Removed => {
                if state.tablet_tool.as_ref().is_some_and(|t| t == tool) {
                    state.tablet_tool = None;
                }
                tool.destroy();
            }
            _ => {}
        }
    }
}

impl Dispatch<zwp_tablet_v2::ZwpTabletV2, ()> for View {
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

impl Dispatch<zwp_tablet_pad_v2::ZwpTabletPadV2, ()> for View {
    event_created_child!(View, zwp_tablet_pad_v2::ZwpTabletPadV2, [
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

impl Dispatch<zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()> for View {
    event_created_child!(View, zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, [
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

impl Dispatch<zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, ()> for View {
    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
        _event: zwp_tablet_pad_ring_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2, ()> for View {
    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
        _event: zwp_tablet_pad_strip_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
