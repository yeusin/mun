use crate::domain::TrayEvent;
use std::sync::mpsc::Sender;

use super::{AppScanner, BookmarkScanner, BrowserLauncher, SystemTray, WindowManager};

pub trait Platform: Send + 'static {
    type Scanner: AppScanner;
    type WinMgr: WindowManager;
    type Browser: BrowserLauncher;
    type Tray: SystemTray;
    type TrayHandle: Send + 'static;
    type Bookmarks: BookmarkScanner;

    fn create_scanner() -> Self::Scanner;
    fn create_window_manager() -> Self::WinMgr;
    fn create_browser() -> Self::Browser;
    fn setup_tray(tx: Sender<TrayEvent>) -> Self::TrayHandle;
    fn create_bookmark_scanner() -> Self::Bookmarks;
}
