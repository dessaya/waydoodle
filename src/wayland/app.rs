use calloop::{
    channel::Event,
    signals::{Signal, Signals},
};
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    reexports::calloop::EventLoop,
    reexports::calloop_wayland_source::WaylandSource,
    registry::RegistryState,
    seat::SeatState,
    shell::{wlr_layer::LayerShell, xdg::XdgShell},
    shm::Shm,
};
use wayland_client::{Connection, globals::registry_queue_init};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2;

use super::{WaylandState, cursors::Cursors};
use crate::{
    tray::{TrayEvent, WaydoodleTray},
    waydoodle::App as _,
    wayland::App,
};

impl App {
    pub(crate) fn run() {
        // Block signals before spawning any background threads (e.g. the tray)
        // so they inherit the blocked mask. Signals::new() calls sigprocmask.
        let signals =
            Signals::new(&[Signal::SIGUSR1, Signal::SIGUSR2]).expect("Failed to register signals");

        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland compositor");
        let (globals, event_queue) =
            registry_queue_init(&conn).expect("Failed to initialize registry");
        let qh = event_queue.handle();

        let mut event_loop: EventLoop<App> =
            EventLoop::try_new().expect("Failed to create event loop");
        let loop_handle = event_loop.handle();

        WaylandSource::new(conn.clone(), event_queue)
            .insert(loop_handle.clone())
            .expect("Failed to insert Wayland source");

        let compositor_state =
            CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
        let xdg_shell = XdgShell::bind(&globals, &qh).expect("xdg_wm_base not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
        let layer_shell = match LayerShell::bind(&globals, &qh) {
            Ok(layer_shell) => Some(layer_shell),
            Err(err) => {
                log::warn!("Layer shell not available: {err}");
                None
            }
        };

        let tablet_manager = globals
            .bind::<zwp_tablet_manager_v2::ZwpTabletManagerV2, _, _>(&qh, 1..=1, ())
            .ok();
        if tablet_manager.is_some() {
            log::info!("Tablet manager bound");
        } else {
            log::info!("Tablet manager not available (tablet input will be unavailable)");
        }

        // Set up the calloop channel for tray events.
        let (tray_sender, tray_channel) = calloop::channel::channel::<TrayEvent>();
        let tray_sender_signals = tray_sender.clone();

        // Spawn the tray service on a background thread.
        let tray = WaydoodleTray::new(tray_sender);
        let tray_handle = match ksni::blocking::TrayMethods::spawn(tray) {
            Ok(handle) => {
                log::info!("Tray service started");
                Some(handle)
            }
            Err(err) => {
                log::warn!("Failed to start tray service: {err}");
                None
            }
        };

        let cursors = Cursors::new(&compositor_state, &shm, &globals, &qh);

        let mut app = Self {
            wayland: WaylandState {
                registry_state: RegistryState::new(&globals),
                seat_state: SeatState::new(&globals, &qh),
                output_state: OutputState::new(&globals, &qh),
                compositor_state,
                xdg_shell,
                layer_shell,
                shm,
            },
            keyboards: Vec::new(),
            pointers: Vec::new(),
            tablets: Vec::new(),
            tablet_manager,
            cursors,
            overlay: None,
            tray_handle,
            loop_handle: event_loop.handle(),
            queue_handle: qh,
        };

        // Register the signal source for toggling the overlay.
        {
            event_loop
                .handle()
                .insert_source(signals, move |ev, _, _| {
                    let r = match ev.signal() {
                        Signal::SIGUSR1 => tray_sender_signals.send(TrayEvent::ToggleOverlay),
                        Signal::SIGUSR2 => tray_sender_signals.send(TrayEvent::CloseOverlay),
                        _ => Ok(()),
                    };
                    if let Err(e) = r {
                        log::error!("Failed to send event from signal handler: {e}");
                    }
                })
                .expect("Failed to insert signal source");
        }

        // Register the tray event channel.
        {
            let loop_signal = event_loop.get_signal();
            event_loop
                .handle()
                .insert_source(tray_channel, move |event, _, app| {
                    let Event::Msg(tray_event) = event else {
                        return;
                    };
                    match tray_event {
                        TrayEvent::ToggleOverlay => {
                            app.on_toggle_overlay();
                        }
                        TrayEvent::CloseOverlay => {
                            app.destroy_overlay();
                        }
                        TrayEvent::Quit => {
                            loop_signal.stop();
                        }
                    }
                })
                .expect("Failed to insert tray event source");
        }

        event_loop
            .run(None, &mut app, |_| {})
            .expect("Event loop failed");

        // Shut down the tray service on exit.
        if let Some(handle) = app.tray_handle.take() {
            handle.shutdown();
        }
    }
}
