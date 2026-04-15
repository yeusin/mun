mod app_scanner;
mod browser;
mod system_tray;
mod window_manager;

use crate::ports::Platform;

pub struct MacOSPlatform;

impl Platform for MacOSPlatform {
    type Scanner = app_scanner::MacOSAppScanner;
    type WinMgr = window_manager::MacOSWindowManager;
    type Browser = browser::OpenCommandBrowser;
    type Tray = system_tray::NoOpSystemTray;
    type TrayHandle = ();

    fn create_scanner() -> Self::Scanner {
        app_scanner::MacOSAppScanner
    }

    fn create_window_manager() -> Self::WinMgr {
        window_manager::MacOSWindowManager
    }

    fn create_browser() -> Self::Browser {
        browser::OpenCommandBrowser
    }

    fn setup_tray(tx: std::sync::mpsc::Sender<crate::domain::TrayEvent>) -> Self::TrayHandle {
        <Self::Tray as crate::ports::SystemTray>::setup(tx)
    }
}
