use crate::domain::TrayEvent;
use std::sync::mpsc::Sender;

pub trait SystemTray: Send + 'static {
    type Handle: Send + 'static;
    fn setup(tx: Sender<TrayEvent>) -> Self::Handle;
}
