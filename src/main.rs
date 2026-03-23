use macro_rules_attribute::apply;
use smol_macros::main;

use ksni::TrayMethods; // provides the spawn method

#[derive(Debug)]
struct MyTray {}

impl ksni::Tray for MyTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }
    fn icon_name(&self) -> String {
        "view-fullscreen".into()
    }
    fn title(&self) -> String {
        "Waydoodle".into()
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "Show/Hide".into(),
                activate: Box::new(|_| std::process::exit(0)),
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
    let tray = MyTray {};
    _ = tray.spawn().await.unwrap();
    std::future::pending().await
}
