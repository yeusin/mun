use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppInfo {
    pub name: String,
    pub exec: String,
}

pub fn scan_apps() -> Vec<AppInfo> {
    let mut apps = Vec::new();

    let user_dir = std::env::var("HOME")
        .map(|h| format!("{}/.local/share/applications", h))
        .unwrap_or_else(|_| String::from("/usr/share/applications"));

    let dirs = ["/usr/share/applications", &user_dir];

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

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.to_lowercase() == b.name.to_lowercase());

    apps
}

fn parse_desktop_file(path: PathBuf) -> Option<AppInfo> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut in_desktop_entry = false;
    let mut no_display = false;
    let mut hidden = false;

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

        if let Some(value) = line.strip_prefix("Name=") {
            name = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("Exec=") {
            let clean_exec = strip_desktop_placeholders(value);
            exec = Some(clean_exec);
        } else if let Some(value) = line.strip_prefix("NoDisplay=") {
            no_display = value.trim().eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("Hidden=") {
            hidden = value.trim().eq_ignore_ascii_case("true");
        }
    }

    if no_display || hidden {
        return None;
    }

    match (name, exec) {
        (Some(n), Some(e)) => Some(AppInfo { name: n, exec: e }),
        _ => None,
    }
}

fn strip_desktop_placeholders(exec: &str) -> String {
    let mut result = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let next = chars.peek();
            if next == Some(&'%') {
                result.push('%');
                chars.next();
            } else {
                // Skip %f, %F, %u, %U, %d, %D, %n, %N, %i, %c, %k, %v, %m and any other %X
                chars.next();
            }
        } else {
            result.push(c);
        }
    }

    result.trim().to_string()
}
