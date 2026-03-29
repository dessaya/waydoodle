use calloop::channel::Sender;
use ksni::{MenuItem, Tray, menu::StandardItem};

#[derive(Debug, Clone)]
pub(crate) enum TrayEvent {
    ToggleOverlay,
    CloseOverlay,
    Quit,
}

pub(crate) struct WaydoodleTray {
    sender: Sender<TrayEvent>,
}

impl WaydoodleTray {
    pub(crate) fn new(sender: Sender<TrayEvent>) -> Self {
        Self { sender }
    }

    fn send(&self, ev: TrayEvent) {
        let r = self.sender.send(ev);
        if let Err(e) = r {
            log::error!("Failed to send event from tray: {e}");
        }
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
        vec![
            StandardItem {
                label: "Toggle overlay".into(),
                activate: Box::new(move |t: &mut WaydoodleTray| {
                    t.send(TrayEvent::ToggleOverlay);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Close overlay".into(),
                activate: Box::new(move |t: &mut WaydoodleTray| {
                    t.send(TrayEvent::CloseOverlay);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |t: &mut WaydoodleTray| {
                    t.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
