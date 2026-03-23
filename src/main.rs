mod canvas;
mod global_shortcut;
mod surface;
mod tray;

use async_channel;
use macro_rules_attribute::apply;
use smol_macros::main;

use surface::SurfaceHandle;

#[apply(main!)]
async fn main() {
    let (toggle_tx, toggle_rx) = async_channel::bounded(1);
    let _ = tray::spawn_tray(toggle_tx.clone()).await;
    smol::spawn(global_shortcut::listen(toggle_tx)).detach();
    let surface = SurfaceHandle::new();
    loop {
        let _ = toggle_rx.recv().await;
        surface.toggle();
    }
}
