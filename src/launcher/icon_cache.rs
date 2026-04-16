use eframe::egui;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct IconCache {
    textures: HashMap<String, Option<egui::TextureHandle>>,
    size: u32,
}

impl IconCache {
    pub fn new(size: u32) -> Self {
        Self {
            textures: HashMap::new(),
            size,
        }
    }

    pub fn get(
        &mut self,
        ctx: &egui::Context,
        icon_name: &Option<String>,
    ) -> Option<egui::TextureHandle> {
        let name = icon_name.as_ref()?;
        if let Some(cached) = self.textures.get(name) {
            return cached.clone();
        }
        let handle = self.load_icon(ctx, name);
        self.textures.insert(name.clone(), handle.clone());
        handle
    }

    fn load_icon(&self, ctx: &egui::Context, name: &str) -> Option<egui::TextureHandle> {
        let path = resolve_icon_path(name, self.size)?;
        let bytes = std::fs::read(&path).ok()?;
        let img = image::load_from_memory(&bytes).ok()?;
        let rgba = img.resize_exact(self.size, self.size, image::imageops::FilterType::Lanczos3);
        let rgba_img = rgba.to_rgba8();
        let size = [rgba_img.width() as usize, rgba_img.height() as usize];
        let pixels = rgba_img.as_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, pixels);
        Some(ctx.load_texture(name, color_image, egui::TextureOptions::LINEAR))
    }
}

fn resolve_icon_path(name: &str, size: u32) -> Option<PathBuf> {
    if name.starts_with('/') {
        let p = PathBuf::from(name);
        if p.exists() {
            return Some(p);
        }
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let search_dirs: Vec<PathBuf> = vec![
        PathBuf::from(format!("{home}/.local/share/icons")),
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
    ];

    let sizes: Vec<String> = [
        format!("{}x{}", size, size),
        "48x48".to_string(),
        "64x64".to_string(),
        "128x128".to_string(),
        "256x256".to_string(),
        "32x32".to_string(),
        "scalable".to_string(),
    ]
    .to_vec();

    let extensions = ["png", "svg", "xpm"];

    for base_dir in &search_dirs {
        for sz_str in &sizes {
            for cat in ["apps", "categories", "mimetypes", ""] {
                let mut path = base_dir.clone();
                if let Some(theme) = detect_icon_theme(base_dir) {
                    path.push(theme);
                }
                path.push(sz_str.as_str());
                if !cat.is_empty() {
                    path.push(cat);
                }
                path.push(name);
                for ext in &extensions {
                    let mut p = path.clone();
                    p.set_extension(ext);
                    if p.exists() {
                        return Some(p);
                    }
                }
            }
        }
    }

    let mut path = PathBuf::from("/usr/share/pixmaps");
    path.push(name);
    for ext in &extensions {
        let mut p = path.clone();
        p.set_extension(ext);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn detect_icon_theme(base_dir: &std::path::Path) -> Option<String> {
    let index_path = base_dir.join("hicolor").join("index.theme");
    if index_path.exists() {
        return Some("hicolor".to_string());
    }

    for entry in std::fs::read_dir(base_dir).ok()? {
        let entry = entry.ok()?;
        if entry.file_type().ok()?.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().join("index.theme").exists() {
                return Some(name);
            }
        }
    }
    None
}
