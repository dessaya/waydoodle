use std::time::Duration;

use calloop::signals::{Signal, Signals};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    reexports::calloop::EventLoop,
    reexports::calloop_wayland_source::WaylandSource,
    seat::{
        SeatState,
        pointer::cursor_shape::CursorShapeManager,
    },
    shell::xdg::XdgShell,
    shm::Shm,
};
use wayland_client::{
    Connection,
    globals::registry_queue_init,
};

use crate::model::Waydoodle;

use super::View;

impl View {
    pub fn run() {
        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
        let (globals, event_queue) =
            registry_queue_init(&conn).expect("Failed to initialize registry");
        let qh = event_queue.handle();

        let mut event_loop: EventLoop<View> =
            EventLoop::try_new().expect("Failed to create event loop");
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("Failed to insert Wayland source");

        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");

        let mut view = View {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state: OutputState::new(&globals, &qh),
            compositor_state,
            xdg_shell,
            shm,

            keyboard: None,
            pointer: None,
            cursor_shape_manager: CursorShapeManager::bind(&globals, &qh).ok(),
            pointer_enter_serial: 0,
            eraser_cursor: None,

            window: None,
            pool: None,
            buffer: None,
            width: 0,
            height: 0,
            first_configure: false,
            dirty: false,

            pointer_pos: (0.0, 0.0),
            pointer_pressed: false,

            model: Waydoodle::new(),

            loop_handle: event_loop.handle(),
            exit: false,
        };

        // Register SIGUSR1 handler to toggle the overlay.
        let sigusr1 = Signals::new(&[Signal::SIGUSR1]).expect("Failed to register SIGUSR1");
        let qh_clone = qh.clone();
        event_loop
            .handle()
            .insert_source(sigusr1, move |_, _, view| {
                let cmd = view.model.toggle_overlay();
                view.dispatch_command(&qh_clone, cmd);
            })
            .expect("Failed to insert signal source");

        loop {
            event_loop
                .dispatch(Duration::from_millis(16), &mut view)
                .expect("Event loop dispatch failed");

            if view.exit {
                break;
            }
        }
    }
}
