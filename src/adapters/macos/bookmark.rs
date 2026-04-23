use crate::adapters::bookmarks::{chromium, firefox, safari};
use crate::domain::AppInfo;
use crate::ports::BookmarkScanner;

pub struct MacOSBookmarkScanner;

impl BookmarkScanner for MacOSBookmarkScanner {
    fn scan_bookmarks(&self) -> Vec<AppInfo> {
        let mut bookmarks = Vec::new();
        bookmarks.extend(chromium::scan_chromium_bookmarks());
        bookmarks.extend(firefox::scan_firefox_bookmarks());
        bookmarks.extend(safari::scan_safari_bookmarks());
        bookmarks.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        if !bookmarks.is_empty() {
            log::info!("Scanned {} browser bookmarks", bookmarks.len());
        }
        bookmarks
    }
}
