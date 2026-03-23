use async_channel::Sender;
use async_signal::{Signal, Signals};
use futures_lite::StreamExt;

pub async fn listen(toggle_tx: Sender<()>) {
    let mut signals = Signals::new([Signal::Usr1]).expect("failed to register SIGUSR1 handler");
    while signals.next().await.is_some() {
        let _ = toggle_tx.try_send(());
    }
}
