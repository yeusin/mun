use crate::domain::TrayEvent;
use std::sync::mpsc::Sender;

use super::{AppScanner, BrowserLauncher, SystemTray, WindowManager};

pub trait Platform: Send + 'static {
    type Scanner: AppScanner;
    type WinMgr: WindowManager;
    type Browser: BrowserLauncher;
    type Tray: SystemTray;
    type TrayHandle: Send + 'static;

    fn create_scanner() -> Self::Scanner;
    fn create_window_manager() -> Self::WinMgr;
    fn create_browser() -> Self::Browser;
    fn setup_tray(tx: Sender<TrayEvent>) -> Self::TrayHandle;
}
