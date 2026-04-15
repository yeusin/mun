use crate::domain::AppInfo;

pub trait AppScanner: Send + Sync + 'static {
    fn scan_apps(&self) -> Vec<AppInfo>;
}
