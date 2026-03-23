mod global_shortcuts;
mod tray;

use async_channel;
use futures_lite::future::zip;
use macro_rules_attribute::apply;
use smol_macros::main;

#[apply(main!)]
async fn main() {
    let (toggle_tx, toggle_rx) = async_channel::bounded(1);

    let _ = tray::spawn_tray(toggle_tx.clone()).await;

    // Run the shortcut listener concurrently with the tray menu toggle receiver.
    let shortcut_fut = global_shortcuts::listen_shortcut(toggle_tx);
    let toggle_loop = async {
        loop {
            let _ = toggle_rx.recv().await;
            println!("Toggle!");
        }
    };
    zip(shortcut_fut, toggle_loop).await;
}
