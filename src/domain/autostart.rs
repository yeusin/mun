#[cfg(target_os = "linux")]
fn autostart_path() -> std::path::PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        format!("{home}/.config")
    });
    std::path::PathBuf::from(config_home).join("autostart/mun.desktop")
}

#[cfg(target_os = "macos")]
fn autostart_path() -> std::path::PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    std::path::PathBuf::from(format!(
        "{home}/Library/LaunchAgents/com.example.mun.plist"
    ))
}

#[cfg(target_os = "linux")]
pub fn set_autostart(enabled: bool) {
    let path = autostart_path();
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exe = match std::env::current_exe() {
            Ok(e) => e.display().to_string(),
            Err(_) => return,
        };
        let desktop_entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Mun\n\
             Exec={exe}\n\
             Hidden=false\n\
             NoDisplay=false\n\
             X-GNOME-Autostart-enabled=true\n"
        );
        let _ = std::fs::write(&path, desktop_entry);
    } else {
        let _ = std::fs::remove_file(&path);
    }
}

#[cfg(target_os = "macos")]
pub fn set_autostart(enabled: bool) {
    let path = autostart_path();
    if enabled {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let exe = match std::env::current_exe() {
            Ok(e) => e.display().to_string(),
            Err(_) => return,
        };
        let plist_content = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
               <key>Label</key><string>com.example.mun</string>\n\
               <key>ProgramArguments</key>\n\
               <array>\n\
                 <string>{exe}</string>\n\
               </array>\n\
               <key>RunAtLoad</key><true/>\n\
               <key>KeepAlive</key><false/>\n\
             </dict>\n\
             </plist>\n"
        );
        let _ = std::fs::write(&path, plist_content);
        let _ = std::process::Command::new("launchctl")
            .args(["load", "-w", &path.display().to_string()])
            .output();
    } else {
        let _ = std::process::Command::new("launchctl")
            .args(["unload", &path.display().to_string()])
            .output();
        let _ = std::fs::remove_file(&path);
    }
}

pub fn is_autostart_enabled() -> bool {
    autostart_path().exists()
}
