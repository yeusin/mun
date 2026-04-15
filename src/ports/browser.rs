pub trait BrowserLauncher: Send + Sync + 'static {
    fn open_url(&self, url: &str);
}
