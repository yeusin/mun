use crate::domain::AppInfo;

pub trait BookmarkScanner: Send + Sync + 'static {
    fn scan_bookmarks(&self) -> Vec<AppInfo>;
}
