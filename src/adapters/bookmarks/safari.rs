use crate::domain::AppInfo;

pub fn scan_safari_bookmarks() -> Vec<AppInfo> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let path = format!("{home}/Library/Safari/Bookmarks.plist");
    let Ok(data) = std::fs::read(&path) else {
        return Vec::new();
    };
    let Ok(value) = plist::from_bytes::<plist::Value>(&data) else {
        return Vec::new();
    };
    let mut bookmarks = Vec::new();
    walk_plist(&value, &mut bookmarks);
    bookmarks
}

fn walk_plist(node: &plist::Value, out: &mut Vec<AppInfo>) {
    let Some(dict) = node.as_dictionary() else {
        return;
    };

    if let Some(url) = dict.get("URLString").and_then(|v| v.as_string()) {
        if let Some(title) = dict.get("Title").and_then(|v| v.as_string()) {
            if !title.is_empty() && !url.is_empty() {
                out.push(AppInfo {
                    name: title.to_string(),
                    exec: url.to_string(),
                    icon: None,
                });
            }
        }
    }

    if let Some(children) = dict.get("Children").and_then(|v| v.as_array()) {
        for child in children {
            walk_plist(child, out);
        }
    }
}
