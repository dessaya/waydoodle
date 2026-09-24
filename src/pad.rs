//! Tablet pad buttons, read directly from the kernel.
//!
//! The Wayland tablet protocol can't be used for this: compositors based on
//! smithay (such as niri) don't implement `zwp_tablet_pad_v2`, and even where
//! it is implemented, pad events are only delivered while the pad is focused
//! on one of our surfaces.
//!
//! Nothing here panics: a tablet pad is optional, so any failure is logged and
//! the pad is simply ignored.

use std::collections::HashMap;
use std::io;
use std::os::fd::{AsFd, BorrowedFd};
use std::path::{Path, PathBuf};

use calloop::generic::Generic;
use calloop::{Interest, LoopHandle, Mode, PostAction, RegistrationToken};
use evdev::{Device, EventSummary, KeyCode};

use crate::notify::warn_user;

/// Implemented by the application, so that this module doesn't need to know
/// anything about it.
pub(crate) trait PadHost: Sized + 'static {
    fn loop_handle(&self) -> LoopHandle<'static, Self>;
    fn pads(&mut self) -> &mut Pads;
    fn on_pad_button(&mut self, button: u32);
}

/// The tablet pads we are listening to.
#[derive(Default)]
pub(crate) struct Pads(HashMap<PathBuf, RegistrationToken>);

/// Starts listening to the buttons of every tablet pad, now and as they get
/// plugged in.
pub(crate) fn listen<H: PadHost>(host: &mut H) {
    match monitor() {
        Ok(monitor) => watch_device_changes(host, monitor),
        Err(e) => log::warn!("Failed to watch for tablet pads: {e}"),
    }
    let devnodes = enumerate();
    if devnodes.is_empty() {
        log::debug!("No tablet pads found");
    }
    for devnode in devnodes {
        add(host, &devnode);
    }
}

fn watch_device_changes<H: PadHost>(host: &mut H, monitor: udev::MonitorSocket) {
    let source = Generic::new(monitor, Interest::READ, Mode::Level);
    let r = host
        .loop_handle()
        .insert_source(source, |_readiness, monitor, host: &mut H| {
            let changes: Vec<_> = monitor
                .iter()
                .map(|event| {
                    let device = event.device();
                    (
                        event.event_type(),
                        device.devnode().map(Path::to_path_buf),
                        is_pad(&device),
                    )
                })
                .collect();
            for (event_type, devnode, is_pad) in changes {
                let Some(devnode) = devnode else {
                    continue;
                };
                match event_type {
                    udev::EventType::Add if is_pad => add(host, &devnode),
                    // A removed device no longer reports its properties, so
                    // just check whether we were listening to it.
                    udev::EventType::Remove => remove(host, &devnode),
                    _ => {}
                }
            }
            Ok(PostAction::Continue)
        });
    if let Err(e) = r {
        log::warn!("Failed to watch for tablet pads: {e}");
    }
}

fn add<H: PadHost>(host: &mut H, devnode: &Path) {
    if host.pads().0.contains_key(devnode) {
        return;
    }
    let Some(pad) = Pad::open(devnode) else {
        return;
    };
    let path = devnode.to_path_buf();
    let source = Generic::new(pad, Interest::READ, Mode::Level);
    let r = host
        .loop_handle()
        .insert_source(source, move |_readiness, pad, host: &mut H| {
            // Safety: the pad is only read from, never dropped.
            let pad = unsafe { pad.get_mut() };
            match pad.read_pressed_buttons() {
                Ok(buttons) => {
                    for button in buttons {
                        host.on_pad_button(button);
                    }
                    Ok(PostAction::Continue)
                }
                Err(e) => {
                    log::info!("Tablet pad {} is gone: {e}", path.display());
                    host.pads().0.remove(&path);
                    Ok(PostAction::Remove)
                }
            }
        });
    match r {
        Ok(token) => {
            host.pads().0.insert(devnode.to_path_buf(), token);
        }
        Err(e) => log::warn!("Failed to listen to {}: {e}", devnode.display()),
    }
}

fn remove<H: PadHost>(host: &mut H, devnode: &Path) {
    if let Some(token) = host.pads().0.remove(devnode) {
        log::info!("Tablet pad {} disconnected", devnode.display());
        host.loop_handle().remove(token);
    }
}

/// Key code ranges used by tablet pad buttons: `BTN_0`-`BTN_9`,
/// `BTN_BASE`-`BTN_BASE6` and `BTN_A`-`BTN_THUMBR`.
fn is_pad_button(code: KeyCode) -> bool {
    matches!(code.0, 0x100..=0x109 | 0x126..=0x12b | 0x130..=0x13f)
}

/// Returns the device nodes of all connected tablet pads.
fn enumerate() -> Vec<PathBuf> {
    let mut enumerator = match udev::Enumerator::new() {
        Ok(enumerator) => enumerator,
        Err(e) => {
            log::warn!("Failed to enumerate input devices: {e}");
            return Vec::new();
        }
    };
    if let Err(e) = enumerator
        .match_subsystem("input")
        .and_then(|()| enumerator.match_property("ID_INPUT_TABLET_PAD", "1"))
    {
        log::warn!("Failed to filter input devices: {e}");
        return Vec::new();
    }
    match enumerator.scan_devices() {
        Ok(devices) => devices
            .filter(is_pad)
            .filter_map(|device| device.devnode().map(Path::to_path_buf))
            .collect(),
        Err(e) => {
            log::warn!("Failed to scan input devices: {e}");
            Vec::new()
        }
    }
}

/// A socket that reports input devices being plugged and unplugged.
fn monitor() -> io::Result<udev::MonitorSocket> {
    udev::MonitorBuilder::new()?
        .match_subsystem("input")?
        .listen()
}

/// A tablet pad also shows up as a joystick, which we can't read as an input
/// device, so only the event node counts.
fn is_pad(device: &udev::Device) -> bool {
    device
        .property_value("ID_INPUT_TABLET_PAD")
        .is_some_and(|value| value == "1")
        && device.sysname().as_encoded_bytes().starts_with(b"event")
}

struct Pad {
    device: Device,
    /// Button key codes, in ascending order.
    buttons: Vec<KeyCode>,
}

impl Pad {
    /// Opens a tablet pad, or returns `None` if it can't be read or has no
    /// buttons.
    fn open(devnode: &Path) -> Option<Self> {
        let device = match Device::open(devnode) {
            Ok(device) => device,
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {
                warn_user!(
                    "Not allowed to read tablet pad {}: add your user to the \
                     'input' group to use pad buttons",
                    devnode.display()
                );
                return None;
            }
            Err(e) => {
                log::warn!("Failed to open tablet pad {}: {e}", devnode.display());
                return None;
            }
        };
        // Otherwise a read would block the event loop.
        if let Err(e) = device.set_nonblocking(true) {
            log::warn!("Failed to set up tablet pad {}: {e}", devnode.display());
            return None;
        }
        let buttons: Vec<KeyCode> = device
            .supported_keys()
            .into_iter()
            .flatten()
            .filter(|code| is_pad_button(*code))
            .collect();
        if buttons.is_empty() {
            log::debug!("Ignoring {}: it has no pad buttons", devnode.display());
            return None;
        }
        log::info!(
            "Listening to tablet pad {} ({}) with {} buttons",
            device.name().unwrap_or("unnamed"),
            devnode.display(),
            buttons.len(),
        );
        Some(Self { device, buttons })
    }

    /// Reads the pending events, returning the buttons that were pressed. An
    /// error means the device is gone.
    fn read_pressed_buttons(&mut self) -> io::Result<Vec<u32>> {
        let Self { device, buttons } = self;
        let events = match device.fetch_events() {
            Ok(events) => events,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut pressed = Vec::new();
        for event in events {
            // 1 is a press; 0 is a release and 2 is auto-repeat.
            if let EventSummary::Key(_, code, 1) = event.destructure() {
                match button_number(buttons, code) {
                    Some(button) => pressed.push(button),
                    None => log::trace!("Ignoring key {code:?}"),
                }
            }
        }
        Ok(pressed)
    }
}

/// A button is identified by the index of its key code, which is also how
/// libinput numbers pad buttons.
fn button_number(buttons: &[KeyCode], code: KeyCode) -> Option<u32> {
    let index = buttons.iter().position(|button| *button == code)?;
    u32::try_from(index).ok()
}

impl AsFd for Pad {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.device.as_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pad_buttons_are_recognized() {
        for code in [0x100, 0x108, 0x109, 0x126, 0x130, 0x13f] {
            assert!(is_pad_button(KeyCode(code)), "{code:#x}");
        }
        // BTN_LEFT, BTN_RIGHT and BTN_STYLUS are not pad buttons.
        for code in [0x110, 0x111, 0x14b] {
            assert!(!is_pad_button(KeyCode(code)), "{code:#x}");
        }
    }

    #[test]
    fn buttons_are_numbered_by_key_code_order() {
        // A tablet with contiguous key codes, like the Huion H640P.
        let buttons: Vec<KeyCode> = (0x100..=0x108).map(KeyCode).collect();
        assert_eq!(button_number(&buttons, KeyCode(0x100)), Some(0));
        assert_eq!(button_number(&buttons, KeyCode(0x105)), Some(5));
        assert_eq!(button_number(&buttons, KeyCode(0x109)), None);

        // A tablet with gaps between them.
        let buttons = [KeyCode(0x100), KeyCode(0x101), KeyCode(0x130)];
        assert_eq!(button_number(&buttons, KeyCode(0x130)), Some(2));
    }
}
