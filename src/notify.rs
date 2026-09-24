//! Desktop notifications for problems the user can do something about.
//!
//! Waydoodle is usually started from an application menu, where nobody sees
//! its log output.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use zbus::zvariant::Value;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, &Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
}

/// Logs a warning and shows it as a desktop notification. Use it for problems
/// the user can do something about, and `log::debug!` for the details.
macro_rules! warn_user {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        log::warn!("{message}");
        crate::notify::show(&message);
    }};
}

pub(crate) use warn_user;

static ENABLED: AtomicBool = AtomicBool::new(true);

/// Turns notifications on or off. `warn_user!` still logs when they are off.
pub(crate) fn set_enabled(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

/// Implementation detail of `warn_user!`; call that instead.
pub(crate) fn show(message: &str) {
    if !ENABLED.load(Ordering::Relaxed) {
        return;
    }
    if let Err(e) = try_show(message) {
        log::debug!("Failed to show a notification: {e}");
    }
}

fn try_show(message: &str) -> zbus::Result<()> {
    let connection = zbus::blocking::Connection::session()?;
    let proxy = NotificationsProxyBlocking::new(&connection)?;
    proxy.notify(
        "waydoodle",
        0,
        "waydoodle",
        "Waydoodle",
        message,
        &[],
        HashMap::new(),
        -1,
    )?;
    Ok(())
}
