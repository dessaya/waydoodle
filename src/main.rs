mod global_shortcuts;
mod surface;
mod tray;

use async_channel;
use futures_lite::future::zip;
use macro_rules_attribute::apply;
use smol_macros::main;

use surface::SurfaceHandle;

#[apply(main!)]
async fn main() {
    let (toggle_tx, toggle_rx) = async_channel::bounded(1);

    let _ = tray::spawn_tray(toggle_tx.clone()).await;

    let surface = SurfaceHandle::new();

    // Run the shortcut listener concurrently with the tray menu toggle receiver.
    let shortcut_fut = global_shortcuts::listen_shortcut(toggle_tx);
    let toggle_loop = async {
        loop {
            let _ = toggle_rx.recv().await;
            surface.toggle();
        }
    };
    zip(shortcut_fut, toggle_loop).await;
}
