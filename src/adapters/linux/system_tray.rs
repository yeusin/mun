use crate::domain::TrayEvent;
use crate::ports::SystemTray;

use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, Tray};
use std::sync::mpsc::Sender;

pub struct KsniSystemTray;

impl SystemTray for KsniSystemTray {
    type Handle = Handle<MunTray>;

    fn setup(tx: Sender<TrayEvent>) -> Self::Handle {
        let icon_data = super::icon::render_icon_text("문");
        let tray = MunTray {
            sender: tx,
            icon_data,
        };
        tray.spawn().expect("Failed to spawn tray")
    }
}

pub struct MunTray {
    pub sender: Sender<TrayEvent>,
    pub icon_data: Vec<u8>,
}

impl Tray for MunTray {
    fn id(&self) -> String {
        "mun-launcher".into()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: 32,
            height: 32,
            data: self.icon_data.clone(),
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx = self.sender.clone();
        let tx_settings = self.sender.clone();
        let tx_autostart = self.sender.clone();
        let tx_quit = self.sender.clone();
        vec![
            StandardItem {
                label: "Show/Hide Launcher".into(),
                activate: Box::new(move |_| {
                    let _ = tx.send(TrayEvent::Toggle);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Settings".into(),
                activate: Box::new(move |_| {
                    let _ = tx_settings.send(TrayEvent::Settings);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: if crate::domain::autostart::is_autostart_enabled() {
                    "✓ Launch at Login".into()
                } else {
                    "Launch at Login".into()
                },
                activate: Box::new(move |_| {
                    let _ = tx_autostart.send(TrayEvent::ToggleAutostart);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_| {
                    let _ = tx_quit.send(TrayEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(TrayEvent::Toggle);
    }
}
