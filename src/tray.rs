use async_channel::Sender;
use ksni::TrayMethods;

#[derive(Debug)]
pub struct MyTray {
    pub toggle_tx: Sender<()>,
}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn icon_name(&self) -> String {
        "waydoodle".into()
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

pub async fn spawn_tray(toggle_tx: Sender<()>) -> ksni::Handle<MyTray> {
    let tray = MyTray { toggle_tx };
    tray.spawn().await.unwrap()
}
