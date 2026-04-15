use crate::ports::BrowserLauncher;

pub struct XdgOpenBrowser;

impl BrowserLauncher for XdgOpenBrowser {
    fn open_url(&self, url: &str) {
        let url = url.to_string();
        std::thread::spawn(move || {
            let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
        });
    }
}
