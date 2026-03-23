use std::os::unix::io::{AsFd, AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;

use wayland_client::event_created_child;
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, delegate_noop,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer, wl_callback, wl_compositor, wl_output, wl_pointer, wl_registry, wl_seat, wl_shm,
        wl_shm_pool, wl_surface,
    },
};
use wayland_cursor::CursorTheme;
use wayland_protocols::wp::tablet::zv2::client::{
    zwp_tablet_manager_v2, zwp_tablet_pad_dial_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
    zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
    zwp_tablet_v2,
};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

/// Handle to a running Wayland surface that can be toggled on/off.
///
/// The Wayland connection lives on a dedicated thread. The public API (e.g. the
/// [`toggle`](Self::toggle) method) sends messages through a channel to that
/// thread.
pub struct SurfaceHandle {
    event_fd: Arc<OwnedFd>,
}

impl SurfaceHandle {
    /// Connect to the Wayland display, bind required globals, and spawn the
    /// event-loop thread. Because initialization happens on the calling
    /// thread, any failure (missing display, missing globals, …) surfaces
    /// immediately.
    pub fn new() -> Self {
        let efd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) };
        assert!(efd >= 0, "eventfd creation failed");
        let event_fd = Arc::new(unsafe { OwnedFd::from_raw_fd(efd) });
        let thread_fd = Arc::clone(&event_fd);

        let conn = Connection::connect_to_env().expect("Failed to connect to Wayland display");

        let (globals, event_queue): (GlobalList, EventQueue<State>) =
            registry_queue_init(&conn).expect("Failed to retrieve Wayland globals");
        let qh = event_queue.handle();

        let compositor: wl_compositor::WlCompositor = globals
            .bind(&qh, 4..=6, ())
            .expect("wl_compositor not available");
        let shm: wl_shm::WlShm = globals.bind(&qh, 1..=1, ()).expect("wl_shm not available");
        let wm_base: xdg_wm_base::XdgWmBase = globals
            .bind(&qh, 2..=6, ())
            .expect("xdg_wm_base not available");
        let seat: wl_seat::WlSeat = globals.bind(&qh, 1..=9, ()).expect("wl_seat not available");

        let cursor_theme =
            CursorTheme::load(&conn, shm.clone(), 24).expect("failed to load cursor theme");
        let pointer_cursor_surface = compositor.create_surface(&qh, ());
        let tablet_cursor_surface = compositor.create_surface(&qh, ());

        let tablet_manager = globals
            .bind::<zwp_tablet_manager_v2::ZwpTabletManagerV2, _, _>(&qh, 1..=1, ())
            .expect("zwp_tablet_manager_v2 not available");
        let _tablet_seat = tablet_manager.get_tablet_seat(&seat, &qh, ());

        let state = State::new(
            compositor,
            shm,
            wm_base,
            cursor_theme,
            pointer_cursor_surface,
            tablet_cursor_surface,
        );

        std::thread::Builder::new()
            .name("wayland-surface".into())
            .spawn(move || run(conn, event_queue, state, thread_fd))
            .expect("Failed to spawn Wayland surface thread");

        Self { event_fd }
    }

    /// Toggle the overlay surface (show ↔ hide).
    pub fn toggle(&self) {
        let buf: u64 = 1;
        unsafe {
            libc::write(
                self.event_fd.as_raw_fd(),
                &buf as *const u64 as *const libc::c_void,
                std::mem::size_of::<u64>(),
            );
        }
    }
}

struct State {
    compositor: wl_compositor::WlCompositor,
    shm: wl_shm::WlShm,
    wm_base: xdg_wm_base::XdgWmBase,
    pointer: Option<wl_pointer::WlPointer>,
    cursor_theme: CursorTheme,
    pointer_cursor_surface: wl_surface::WlSurface,
    tablet_cursor_surface: wl_surface::WlSurface,
    // Active surface objects (None when hidden)
    surface_objects: Option<SurfaceObjects>,
    // We only track width/height so we can create a correctly sized buffer
    configured: bool,
    width: u32,
    height: u32,
}

struct SurfaceObjects {
    wl_surface: wl_surface::WlSurface,
    xdg_surface: xdg_surface::XdgSurface,
    xdg_toplevel: xdg_toplevel::XdgToplevel,
    buffer: Option<wl_buffer::WlBuffer>,
    pool: Option<wl_shm_pool::WlShmPool>,
}

impl State {
    fn new(
        compositor: wl_compositor::WlCompositor,
        shm: wl_shm::WlShm,
        wm_base: xdg_wm_base::XdgWmBase,
        cursor_theme: CursorTheme,
        pointer_cursor_surface: wl_surface::WlSurface,
        tablet_cursor_surface: wl_surface::WlSurface,
    ) -> Self {
        Self {
            compositor,
            shm,
            wm_base,
            pointer: None,
            cursor_theme,
            pointer_cursor_surface,
            tablet_cursor_surface,
            surface_objects: None,
            configured: false,
            width: 0,
            height: 0,
        }
    }

    fn is_visible(&self) -> bool {
        self.surface_objects.is_some()
    }

    fn show(&mut self, qh: &QueueHandle<Self>) {
        if self.is_visible() {
            return; // already visible
        }

        let wl_surface = self.compositor.create_surface(qh, ());
        let xdg_surface = self.wm_base.get_xdg_surface(&wl_surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());

        xdg_toplevel.set_title("Waydoodle".into());
        xdg_toplevel.set_app_id("waydoodle".into());
        xdg_toplevel.set_maximized();

        // Commit to trigger the initial configure.
        wl_surface.commit();

        self.configured = false;
        self.surface_objects = Some(SurfaceObjects {
            wl_surface,
            xdg_surface,
            xdg_toplevel,
            buffer: None,
            pool: None,
        });
    }

    fn hide(&mut self) {
        if let Some(objs) = self.surface_objects.take() {
            if let Some(buf) = objs.buffer {
                buf.destroy();
            }
            if let Some(pool) = objs.pool {
                pool.destroy();
            }
            objs.xdg_toplevel.destroy();
            objs.xdg_surface.destroy();
            objs.wl_surface.destroy();
            self.configured = false;
        }
    }

    fn attach_buffer(&mut self, qh: &QueueHandle<Self>) {
        let objs = match self.surface_objects.as_mut() {
            Some(o) => o,
            None => return,
        };

        let width = if self.width > 0 { self.width } else { 100 };
        let height = if self.height > 0 { self.height } else { 100 };
        let stride = width * 4;
        let size = (stride * height) as usize;

        // Create an anonymous shared‑memory file.
        let file = create_shm_file(size);

        // Fill with semi‑transparent black (ARGB8888: 0x80000000).
        {
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    size,
                    libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    file.as_fd().as_raw_fd(),
                    0,
                )
            };
            assert_ne!(ptr, libc::MAP_FAILED);
            let slice = unsafe { std::slice::from_raw_parts_mut(ptr as *mut u32, size / 4) };
            for pixel in slice.iter_mut() {
                // Semi-transparent black — lets the desktop show through.
                *pixel = 0x80_00_00_00;
            }
            unsafe {
                libc::munmap(ptr, size);
            }
        }

        // Destroy old buffer/pool if any.
        if let Some(buf) = objs.buffer.take() {
            buf.destroy();
        }
        if let Some(pool) = objs.pool.take() {
            pool.destroy();
        }

        let pool = self.shm.create_pool(file.as_fd(), size as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            width as i32,
            height as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );

        objs.wl_surface.attach(Some(&buffer), 0, 0);
        objs.wl_surface.commit();

        objs.buffer = Some(buffer);
        objs.pool = Some(pool);
    }

    /// Load the crosshair cursor from the theme and attach it to the shared
    /// cursor surface, then invoke `set_cursor` with the surface and hotspot.
    fn set_crosshair_cursor(
        &mut self,
        cursor_surface: &wl_surface::WlSurface,
        set_cursor: impl FnOnce(&wl_surface::WlSurface, i32, i32),
    ) {
        if let Some(cursor) = self.cursor_theme.get_cursor("crosshair") {
            let image = &cursor[0];
            let (hotspot_x, hotspot_y) = image.hotspot();
            cursor_surface.attach(Some(&image), 0, 0);
            cursor_surface.commit();
            set_cursor(cursor_surface, hotspot_x as i32, hotspot_y as i32);
        }
    }
}

fn create_shm_file(size: usize) -> std::fs::File {
    // memfd_create is the modern, race-free way.
    let name = std::ffi::CString::new("waydoodle-shm").unwrap();
    let fd = unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC) };
    assert!(fd >= 0, "memfd_create failed");

    let file = unsafe { std::fs::File::from_raw_fd(fd) };
    file.set_len(size as u64).expect("ftruncate failed");
    file
}

delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_buffer::WlBuffer);
delegate_noop!(State: ignore wl_callback::WlCallback);
delegate_noop!(State: ignore wl_output::WlOutput);

// wl_registry — handled by GlobalList internally, but we still need to provide an impl.
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for State {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        _event: wl_registry::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // Handled by GlobalList internally.
    }
}

// wl_seat — track pointer capability and bind if available.
impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: cap_flags,
        } = event
        {
            let has_pointer = cap_flags
                .into_result()
                .map(|c| c.contains(wl_seat::Capability::Pointer))
                .unwrap_or(false);

            if has_pointer && state.pointer.is_none() {
                state.pointer = Some(seat.get_pointer(qh, ()));
            }
        }
    }
}

// wl_pointer — set crosshair cursor on pointer enter.
impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        state: &mut Self,
        pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_pointer::Event::Enter {
            serial,
            surface: _,
            surface_x: _,
            surface_y: _,
        } = event
        {
            let cursor_surface = state.pointer_cursor_surface.clone();
            let pointer = pointer.clone();
            state.set_crosshair_cursor(&cursor_surface, |surface, hx, hy| {
                pointer.set_cursor(serial, Some(surface), hx, hy);
            });
        }
    }
}

delegate_noop!(State: ignore zwp_tablet_manager_v2::ZwpTabletManagerV2);
delegate_noop!(State: ignore zwp_tablet_v2::ZwpTabletV2);

impl Dispatch<zwp_tablet_pad_v2::ZwpTabletPadV2, ()> for State {
    event_created_child!(State, zwp_tablet_pad_v2::ZwpTabletPadV2, [
        zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, Default::default()),
    ]);
    fn event(
        _: &mut State,
        _: &zwp_tablet_pad_v2::ZwpTabletPadV2,
        _: <zwp_tablet_pad_v2::ZwpTabletPadV2 as wayland_client::Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
    }
}

delegate_noop!(State: ignore zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2);
delegate_noop!(State: ignore zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2);
delegate_noop!(State: ignore zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2);
delegate_noop!(State: ignore zwp_tablet_pad_dial_v2::ZwpTabletPadDialV2);

impl Dispatch<zwp_tablet_seat_v2::ZwpTabletSeatV2, ()> for State {
    event_created_child!(State, zwp_tablet_seat_v2::ZwpTabletSeatV2, [
        zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (zwp_tablet_v2::ZwpTabletV2, Default::default()),
        zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (zwp_tablet_tool_v2::ZwpTabletToolV2, Default::default()),
        zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (zwp_tablet_pad_v2::ZwpTabletPadV2, Default::default())
    ]);
    fn event(
        _state: &mut Self,
        _proxy: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
        _event: <zwp_tablet_seat_v2::ZwpTabletSeatV2 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
    }
}

// zwp_tablet_tool_v2 — set crosshair cursor on proximity-in.
impl Dispatch<zwp_tablet_tool_v2::ZwpTabletToolV2, ()> for State {
    fn event(
        state: &mut Self,
        tool: &zwp_tablet_tool_v2::ZwpTabletToolV2,
        event: zwp_tablet_tool_v2::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let zwp_tablet_tool_v2::Event::ProximityIn {
            serial,
            tablet: _,
            surface: _,
        } = event
        {
            let cursor_surface = state.tablet_cursor_surface.clone();
            let tool = tool.clone();
            state.set_crosshair_cursor(&cursor_surface, |surface, hx, hy| {
                tool.set_cursor(serial, Some(surface), hx, hy);
            });
        }
    }
}

// xdg_wm_base — must reply to ping.
impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _state: &mut Self,
        proxy: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            proxy.pong(serial);
        }
    }
}

// xdg_surface — handle configure acks.
impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        proxy: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _data: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            proxy.ack_configure(serial);
            if !state.configured {
                state.configured = true;
                state.attach_buffer(qh);
            }
        }
    }
}

// xdg_toplevel — handle configure (size) and close.
impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _proxy: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure {
                width,
                height,
                states: _,
            } => {
                if width > 0 && height > 0 {
                    state.width = width as u32;
                    state.height = height as u32;
                    // Mark as needing a new buffer on the next xdg_surface configure.
                    state.configured = false;
                }
            }
            xdg_toplevel::Event::Close => {
                state.hide();
            }
            _ => {}
        }
    }
}

fn run(
    conn: Connection,
    mut event_queue: EventQueue<State>,
    mut state: State,
    event_fd: Arc<OwnedFd>,
) {
    let qh = event_queue.handle();
    let wayland_fd = conn.prepare_read().unwrap().connection_fd().as_raw_fd();
    let event_raw_fd = event_fd.as_raw_fd();

    eprintln!("[surface] Wayland thread running");

    loop {
        if conn.flush().is_err() {
            eprintln!("[surface] Wayland connection lost");
            break;
        }

        // Block until either the Wayland socket or the eventfd is readable.
        let mut fds = [
            libc::pollfd {
                fd: wayland_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: event_raw_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        unsafe { libc::poll(fds.as_mut_ptr(), 2, -1) };

        // If the Wayland socket is readable, read incoming events.
        if (fds[0].revents & libc::POLLIN) != 0 {
            if let Some(guard) = conn.prepare_read() {
                let _ = guard.read();
            }
        } else {
            // No Wayland data — still need to drop the read guard.
            if let Some(guard) = conn.prepare_read() {
                drop(guard);
            }
        }

        // If the eventfd is readable, consume it and toggle.
        if (fds[1].revents & libc::POLLIN) != 0 {
            let mut buf: u64 = 0;
            unsafe {
                libc::read(
                    event_raw_fd,
                    &mut buf as *mut u64 as *mut libc::c_void,
                    std::mem::size_of::<u64>(),
                );
            }
            if state.is_visible() {
                eprintln!("[surface] Hiding overlay");
                state.hide();
            } else {
                eprintln!("[surface] Showing overlay");
                state.show(&qh);
            }
        }

        event_queue
            .dispatch_pending(&mut state)
            .expect("Wayland dispatch failed");
    }
}
