use crate::ports::BrowserLauncher;

pub struct OpenCommandBrowser;

impl BrowserLauncher for OpenCommandBrowser {
    fn open_url(&self, url: &str) {
        let url = url.to_string();
        std::thread::spawn(move || {
            let _ = std::process::Command::new("open").arg(&url).spawn();
        });
    }
}
