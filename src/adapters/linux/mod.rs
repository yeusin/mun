mod app_scanner;
mod browser;
mod icon;
mod system_tray;
mod window_manager;

use crate::ports::Platform;

pub struct LinuxPlatform;

impl Platform for LinuxPlatform {
    type Scanner = app_scanner::LinuxAppScanner;
    type WinMgr = window_manager::LinuxWindowManager;
    type Browser = browser::XdgOpenBrowser;
    type Tray = system_tray::KsniSystemTray;
    type TrayHandle = ksni::blocking::Handle<system_tray::MunTray>;

    fn create_scanner() -> Self::Scanner {
        app_scanner::LinuxAppScanner
    }

    fn create_window_manager() -> Self::WinMgr {
        window_manager::LinuxWindowManager
    }

    fn create_browser() -> Self::Browser {
        browser::XdgOpenBrowser
    }

    fn setup_tray(tx: std::sync::mpsc::Sender<crate::domain::TrayEvent>) -> Self::TrayHandle {
        <Self::Tray as crate::ports::SystemTray>::setup(tx)
    }
}
