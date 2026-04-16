use crate::domain::AppInfo;
use crate::ports::BookmarkScanner;

pub struct MacOSBookmarkScanner;

impl BookmarkScanner for MacOSBookmarkScanner {
    fn scan_bookmarks(&self) -> Vec<AppInfo> {
        Vec::new()
    }
}
