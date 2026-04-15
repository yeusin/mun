use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AppInfo {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
}

#[cfg(target_os = "linux")]
pub fn scan_apps() -> Vec<AppInfo> {
    let mut apps = BTreeMap::new();

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
                        let key = app.name.to_lowercase();
                        apps.entry(key).or_insert(app);
                    }
                }
            }
        }
    }

    apps.into_values().collect()
}

#[cfg(target_os = "macos")]
pub fn scan_apps() -> Vec<AppInfo> {
    let mut apps = BTreeMap::new();

    let user_apps = std::env::var("HOME")
        .map(|h| format!("{}/Applications", h))
        .unwrap_or_else(|_| String::from("/Applications"));

    let dirs = ["/Applications", &user_apps, "/System/Applications"];

    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("app") {
                    if let Some(app) = parse_macos_app(&path) {
                        let key = app.name.to_lowercase();
                        apps.entry(key).or_insert(app);
                    }
                }
            }
        }
    }

    apps.into_values().collect()
}

#[cfg(target_os = "macos")]
fn parse_macos_app(path: &PathBuf) -> Option<AppInfo> {
    let plist_path = path.join("Contents").join("Info.plist");
    let content = fs::read_to_string(&plist_path).ok()?;

    let name = extract_plist_value(&content, "CFBundleDisplayName")
        .or_else(|| extract_plist_value(&content, "CFBundleName"))
        .or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        })?;

    let exec_name =
        extract_plist_value(&content, "CFBundleIdentifier").unwrap_or_else(|| name.clone());

    let icon = extract_plist_value(&content, "CFBundleIconFile")
        .or_else(|| extract_plist_value(&content, "CFBundleIconName"));

    Some(AppInfo {
        name,
        exec: format!("open -b {}", exec_name),
        icon,
    })
}

#[cfg(target_os = "macos")]
fn extract_plist_value(plist: &str, key: &str) -> Option<String> {
    let search = format!("<key>{}</key>", key);
    let start = plist.find(&search)?;
    let after_key = &plist[start + search.len()..];

    let tag_start = after_key.find('<')?;
    let tag_end = after_key.find('>')?;
    let tag = &after_key[tag_start..=tag_end];

    let value_start = tag_end + 1;
    let value_end = after_key[value_start..].find('<')?;
    let value = after_key[value_start..value_start + value_end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(target_os = "linux")]
fn parse_desktop_file(path: PathBuf) -> Option<AppInfo> {
    let content = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut app_type = None;
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
        } else if let Some(value) = line.strip_prefix("Icon=") {
            icon = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Type=") {
            app_type = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("NoDisplay=") {
            no_display = value.trim().eq_ignore_ascii_case("true");
        } else if let Some(value) = line.strip_prefix("Hidden=") {
            hidden = value.trim().eq_ignore_ascii_case("true");
        }
    }

    if no_display || hidden {
        return None;
    }

    if let Some(ref t) = app_type {
        if t != "Application" {
            return None;
        }
    }

    match (name, exec) {
        (Some(n), Some(e)) => Some(AppInfo {
            name: n,
            exec: e,
            icon,
        }),
        _ => None,
    }
}

#[cfg(target_os = "linux")]
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
                chars.next();
            }
        } else {
            result.push(c);
        }
    }

    result.trim().to_string()
}
