use calloop::channel::Sender;
use ksni::{MenuItem, Tray, menu::StandardItem};

#[derive(Debug, Clone)]
pub enum TrayEvent {
    ToggleOverlay,
    Quit,
}

pub struct WaydoodleTray {
    sender: Sender<TrayEvent>,
}

impl WaydoodleTray {
    pub fn new(sender: Sender<TrayEvent>) -> Self {
        Self { sender }
    }
}

impl Tray for WaydoodleTray {
    fn id(&self) -> String {
        "waydoodle".into()
    }

    fn icon_name(&self) -> String {
        "waydoodle".into()
    }

    fn title(&self) -> String {
        "Waydoodle".into()
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let sender_toggle = self.sender.clone();
        let sender_quit = self.sender.clone();
        vec![
            StandardItem {
                label: "Toggle overlay".into(),
                activate: Box::new(move |_| {
                    let _ = sender_toggle.send(TrayEvent::ToggleOverlay);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_| {
                    let _ = sender_quit.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
