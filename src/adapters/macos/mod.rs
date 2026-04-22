mod app_scanner;
mod bookmark;
mod browser;
mod system_tray;
mod window_manager;

use crate::ports::Platform;
use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyAccessory};
use cocoa::base::YES;
use cocoa::base::nil;

pub struct MacOSPlatform;

pub fn configure_as_accessory_app() {
    unsafe {
        let app = NSApp();
        if app != nil {
            app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
        }
    }
}

pub fn activate_app() {
    unsafe {
        let app = NSApp();
        if app != nil {
            app.activateIgnoringOtherApps_(YES);
        }
    }
}

impl Platform for MacOSPlatform {
    type Scanner = app_scanner::MacOSAppScanner;
    type WinMgr = window_manager::MacOSWindowManager;
    type Browser = browser::OpenCommandBrowser;
    type Tray = system_tray::MenuBarSystemTray;
    type TrayHandle = system_tray::MenuBarTrayHandle;
    type Bookmarks = bookmark::MacOSBookmarkScanner;

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

    fn create_bookmark_scanner() -> Self::Bookmarks {
        bookmark::MacOSBookmarkScanner
    }
}
