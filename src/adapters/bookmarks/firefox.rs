use crate::domain::AppInfo;

#[cfg(target_os = "linux")]
fn firefox_profile_dirs() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    vec![format!("{home}/.mozilla/firefox")]
}

#[cfg(target_os = "macos")]
fn firefox_profile_dirs() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    vec![format!("{home}/Library/Application Support/Firefox/Profiles")]
}

pub fn scan_firefox_bookmarks() -> Vec<AppInfo> {
    let mut bookmarks = Vec::new();
    for dir in firefox_profile_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.contains(".default") {
                continue;
            }
            let db_path = entry.path().join("places.sqlite");
            if db_path.exists() {
                bookmarks.extend(read_firefox_db(&db_path));
            }
        }
    }
    bookmarks
}

fn read_firefox_db(path: &std::path::Path) -> Vec<AppInfo> {
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return Vec::new();
    };

    let Ok(mut stmt) = conn.prepare(
        "SELECT b.title, p.url \
         FROM moz_bookmarks b \
         JOIN moz_places p ON b.fk = p.id \
         WHERE b.type = 1 AND b.title IS NOT NULL AND p.url IS NOT NULL",
    ) else {
        return Vec::new();
    };

    let rows = stmt.query_map([], |row| {
        let title: String = row.get(0)?;
        let url: String = row.get(1)?;
        Ok(AppInfo {
            name: title,
            exec: url,
            icon: None,
        })
    });

    match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}
