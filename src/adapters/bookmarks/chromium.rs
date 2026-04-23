use crate::domain::AppInfo;
use std::fs;

#[cfg(target_os = "linux")]
pub fn chromium_paths() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    vec![
        format!("{home}/.config/google-chrome/Default/Bookmarks"),
        format!("{home}/.config/google-chrome/Profile 1/Bookmarks"),
        format!("{home}/.config/chromium/Default/Bookmarks"),
        format!("{home}/.config/microsoft-edge/Default/Bookmarks"),
        format!("{home}/.config/BraveSoftware/Brave-Browser/Default/Bookmarks"),
    ]
}

#[cfg(target_os = "macos")]
pub fn chromium_paths() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    vec![
        format!("{home}/Library/Application Support/Google/Chrome/Default/Bookmarks"),
        format!("{home}/Library/Application Support/Google/Chrome/Profile 1/Bookmarks"),
        format!("{home}/Library/Application Support/Chromium/Default/Bookmarks"),
        format!("{home}/Library/Application Support/Microsoft Edge/Default/Bookmarks"),
        format!("{home}/Library/Application Support/BraveSoftware/Brave-Browser/Default/Bookmarks"),
    ]
}

pub fn scan_chromium_bookmarks() -> Vec<AppInfo> {
    let paths = chromium_paths();
    let mut bookmarks = Vec::new();
    for path in &paths {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(roots) = parsed.get("roots") {
                    extract_chromium_bookmarks(roots, &mut bookmarks);
                }
            }
        }
    }
    bookmarks
}

fn extract_chromium_bookmarks(node: &serde_json::Value, out: &mut Vec<AppInfo>) {
    if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
        for child in children {
            extract_chromium_bookmarks(child, out);
        }
    }
    if node.get("type").and_then(|t| t.as_str()) == Some("url") {
        let name = node.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let url = node.get("url").and_then(|u| u.as_str()).unwrap_or("");
        if !name.is_empty() && !url.is_empty() {
            out.push(AppInfo {
                name: name.to_string(),
                exec: url.to_string(),
                icon: None,
            });
        }
    }

    if let Some(obj) = node.as_object() {
        for value in obj.values() {
            if value.is_object() && value.get("children").is_some() && value.get("type").and_then(|t| t.as_str()) != Some("url") {
                extract_chromium_bookmarks(value, out);
            }
        }
    }
}
