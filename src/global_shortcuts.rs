use async_channel::Sender;
use futures_lite::StreamExt;

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};

pub async fn listen_shortcut(toggle_tx: Sender<()>) {
    let shortcuts = GlobalShortcuts::new()
        .await
        .expect("Failed to create GlobalShortcuts proxy");
    eprintln!(
        "[shortcuts] Portal proxy created (version {})",
        shortcuts.version()
    );

    let session = shortcuts
        .create_session(Default::default())
        .await
        .expect("Failed to create shortcuts session");
    eprintln!("[shortcuts] Session created");

    // Subscribe to the Activated signal *before* binding, so we don't miss any.
    let mut activated = shortcuts
        .receive_activated()
        .await
        .expect("Failed to listen for shortcut activations");
    eprintln!("[shortcuts] Listening for activations");

    let request = shortcuts
        .bind_shortcuts(
            &session,
            &[NewShortcut::new("toggle", "Toggle drawing overlay").preferred_trigger("F9")],
            None,
            Default::default(),
        )
        .await
        .expect("Failed to send bind_shortcuts request");

    match request.response() {
        Ok(bound) => {
            eprintln!("[shortcuts] Bound shortcuts:");
            for s in bound.shortcuts() {
                eprintln!("  - id={:?} trigger={:?}", s.id(), s.trigger_description());
            }
        }
        Err(e) => {
            eprintln!("[shortcuts] bind_shortcuts failed: {e}");
            eprintln!(
                "[shortcuts] Your compositor may not fully support the GlobalShortcuts portal."
            );
            eprintln!("[shortcuts] Falling back to tray-only toggle (no global shortcut).");
            return;
        }
    }

    while let Some(event) = activated.next().await {
        eprintln!(
            "[shortcuts] Shortcut activated: id={:?}",
            event.shortcut_id()
        );
        if event.shortcut_id() == "toggle" {
            let _ = toggle_tx.try_send(());
        }
    }
}
