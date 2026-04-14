use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppInfo {
    pub name: String,
    pub exec: String,
}

pub fn scan_apps() -> Vec<AppInfo> {
    let mut apps = Vec::new();
    let dirs = [
        "/usr/share/applications",
        "/home/ykim/.local/share/applications", // hardcoded for now or use dirs crate
    ];

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("desktop") {
                    if let Some(app) = parse_desktop_file(path) {
                        apps.push(app);
                    }
                }
            }
        }
    }
    
    // De-duplicate by name
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());
    
    apps
}

fn parse_desktop_file(path: PathBuf) -> Option<AppInfo> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "[Desktop Entry]" {
            in_desktop_entry = true;
            continue;
        }
        if line.starts_with('[') {
            in_desktop_entry = false;
            continue;
        }
        if !in_desktop_entry {
            continue;
        }

        if line.starts_with("Name=") {
            name = Some(line[5..].to_string());
        } else if line.starts_with("Exec=") {
            // Strip placeholders like %u, %f, %U, %F
            let full_exec = &line[5..];
            let clean_exec = full_exec.split(" %").next().unwrap_or(full_exec).to_string();
            exec = Some(clean_exec);
        }
    }

    match (name, exec) {
        (Some(n), Some(e)) => Some(AppInfo { name: n, exec: e }),
        _ => None,
    }
}
