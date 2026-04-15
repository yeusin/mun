use ab_glyph::{Font, FontRef, Glyph, PxScale};
use image::{Rgba, RgbaImage};

pub fn render_icon_text(text: &str) -> Vec<u8> {
    let size = 32;
    let mut image = RgbaImage::new(size, size);

    let font_paths = [
        "/usr/share/fonts/truetype/baekmuk/batang.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Medium.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    let font_data = font_paths
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .unwrap_or_else(|| {
            let mut data = Vec::new();
            for _ in 0..size * size {
                data.extend_from_slice(&[255, 255, 255, 255]);
            }
            data
        });

    if font_data.len() == (size * size * 4) as usize {
        return font_data;
    }

    let font = FontRef::try_from_slice(&font_data).expect("Failed to load font");
    let scale = PxScale::from(26.0);
    let glyph: Glyph = font
        .glyph_id(text.chars().next().unwrap())
        .with_scale_and_position(scale, ab_glyph::point(3.0, 26.0));

    if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();
        outlined.draw(|x, y, v| {
            let px = x + bounds.min.x as u32;
            let py = y + bounds.min.y as u32;
            if px < size && py < size {
                image.put_pixel(px, py, Rgba([(v * 255.0) as u8, 255, 255, 255]));
            }
        });
    }

    image.into_raw()
}
