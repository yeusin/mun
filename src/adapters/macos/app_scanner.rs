use crate::domain::AppInfo;
use crate::ports::AppScanner;

use std::collections::BTreeMap;
use std::path::PathBuf;

pub struct MacOSAppScanner;

impl AppScanner for MacOSAppScanner {
    fn scan_apps(&self) -> Vec<AppInfo> {
        let mut apps = BTreeMap::new();

        let user_apps = std::env::var("HOME")
            .map(|h| format!("{}/Applications", h))
            .unwrap_or_else(|_| String::from("/Applications"));

        let dirs = [
            "/Applications",
            &user_apps,
            "/System/Applications",
            "/System/Library/CoreServices",
        ];

        for dir in dirs {
            scan_app_dir(&PathBuf::from(dir), &mut apps);
        }

        apps.into_values().collect()
    }
}

fn scan_app_dir(dir: &PathBuf, apps: &mut BTreeMap<String, AppInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("app") {
            if let Some(app) = parse_macos_app(&path) {
                let key = app.name.to_lowercase();
                apps.entry(key).or_insert(app);
            }
            continue;
        }

        if path.is_dir() {
            scan_app_dir(&path, apps);
        }
    }
}

fn parse_macos_app(path: &PathBuf) -> Option<AppInfo> {
    let plist_path = path.join("Contents").join("Info.plist");
    let content = std::fs::read_to_string(&plist_path).ok()?;

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

fn extract_plist_value(plist: &str, key: &str) -> Option<String> {
    let search = format!("<key>{}</key>", key);
    let start = plist.find(&search)?;
    let after_key = &plist[start + search.len()..];

    let tag_start = after_key.find('<')?;
    let tag_end = after_key.find('>')?;
    let _tag = &after_key[tag_start..=tag_end];

    let value_start = tag_end + 1;
    let value_end = after_key[value_start..].find('<')?;
    let value = after_key[value_start..value_start + value_end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
