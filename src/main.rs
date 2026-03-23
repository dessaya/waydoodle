use async_channel::{self, Sender};
use macro_rules_attribute::apply;
use smol_macros::main;

use ksni::TrayMethods;

#[derive(Debug)]
struct MyTray {
    toggle_tx: Sender<()>,
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn icon_name(&self) -> String {
        "input-tablet".into()
    }
    fn title(&self) -> String {
        "Waydoodle".into()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Show/Hide".into(),
                activate: Box::new(|this: &mut Self| {
                    let _ = this.toggle_tx.try_send(());
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Exit".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[apply(main!)]
async fn main() {
    let (toggle_tx, toggle_rx) = async_channel::bounded(1);

    let tray = MyTray { toggle_tx };
    _ = tray.spawn().await.unwrap();

    loop {
        let _ = toggle_rx.recv().await;
        println!("Toggle!");
    }
}
