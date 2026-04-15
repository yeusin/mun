use crate::domain::TrayEvent;
use crate::ports::SystemTray;

use std::sync::mpsc::Sender;

pub struct NoOpSystemTray;

impl SystemTray for NoOpSystemTray {
    type Handle = ();

    fn setup(_tx: Sender<TrayEvent>) -> Self::Handle {}
}
