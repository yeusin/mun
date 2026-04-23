use crate::domain::TrayEvent;
use crate::ports::SystemTray;

use ab_glyph::{Font, FontRef, Glyph, point};
use std::fs;
use std::sync::mpsc::Sender;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct MenuBarSystemTray;

pub struct MenuBarTrayHandle {
    _tray_icon: TrayIcon,
}

unsafe impl Send for MenuBarTrayHandle {}

impl SystemTray for MenuBarSystemTray {
    type Handle = MenuBarTrayHandle;

    fn setup(tx: Sender<TrayEvent>) -> Self::Handle {
        let toggle_id = MenuId::new("toggle");
        let settings_id = MenuId::new("settings");
        let autostart_id = MenuId::new("autostart");
        let quit_id = MenuId::new("quit");

        let autostart_label = if crate::domain::autostart::is_autostart_enabled() {
            "✓ Launch at Login"
        } else {
            "Launch at Login"
        };

        let toggle = MenuItem::with_id(toggle_id.clone(), "Show Launcher", true, None);
        let settings = MenuItem::with_id(settings_id.clone(), "Settings", true, None);
        let autostart =
            MenuItem::with_id(autostart_id.clone(), autostart_label, true, None);
        let quit = MenuItem::with_id(quit_id.clone(), "Quit", true, None);

        let submenu = Submenu::with_items(
            "Mun",
            true,
            &[
                &toggle,
                &settings,
                &autostart,
                &PredefinedMenuItem::separator(),
                &quit,
            ],
        )
        .expect("failed to build tray submenu");

        let menu = Menu::new();
        menu.append(&submenu)
            .expect("failed to append tray submenu");

        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let tray_event = if event.id == toggle_id {
                Some(TrayEvent::Toggle)
            } else if event.id == settings_id {
                Some(TrayEvent::Settings)
            } else if event.id == autostart_id {
                Some(TrayEvent::ToggleAutostart)
            } else if event.id == quit_id {
                Some(TrayEvent::Quit)
            } else {
                None
            };

            if let Some(event) = tray_event
                && let Err(err) = tx.send(event)
            {
                log::error!("Failed to send tray event: {}", err);
            }
        }));

        let tray_icon = TrayIconBuilder::new()
            .with_icon(tray_icon())
            .with_icon_as_template(true)
            .with_tooltip("Mun")
            .with_menu(Box::new(menu))
            .build()
            .expect("failed to create tray icon");

        MenuBarTrayHandle {
            _tray_icon: tray_icon,
        }
    }
}

fn tray_icon() -> Icon {
    render_glyph_icon('문').unwrap_or_else(fallback_tray_icon)
}

fn render_glyph_icon(ch: char) -> Option<Icon> {
    const SIZE: u32 = 18;
    const SCALE: f32 = 20.0;
    const X_OFFSET: f32 = 5.0;
    const Y_OFFSET: f32 = 0.5;
    const FONT_CANDIDATES: &[(&str, &[u32])] = &[
        ("/System/Library/Fonts/Supplemental/AppleGothic.ttf", &[0]),
        ("/System/Library/Fonts/Supplemental/AppleMyungjo.ttf", &[0]),
        ("/System/Library/Fonts/AppleSDGothicNeo.ttc", &[0, 1, 2, 3, 4]),
    ];

    for (path, indices) in FONT_CANDIDATES {
        let Ok(data) = fs::read(path) else {
            continue;
        };
        for index in *indices {
            let Ok(font) = FontRef::try_from_slice_and_index(&data, *index) else {
                continue;
            };
            if font.glyph_id(ch).0 == 0 {
                continue;
            }

            let Some(glyph) = centered_glyph(&font, ch, SCALE, SIZE as f32, X_OFFSET, Y_OFFSET) else {
                continue;
            };
            let Some(outlined) = font.outline_glyph(glyph) else {
                continue;
            };
            let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];
            outlined.draw(|x, y, coverage| {
                if x >= SIZE || y >= SIZE {
                    return;
                }
                let alpha = (coverage * 255.0).round() as u8;
                let idx = ((y * SIZE + x) * 4) as usize;
                rgba[idx] = 255;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 255;
                rgba[idx + 3] = rgba[idx + 3].max(alpha);
            });
            return Icon::from_rgba(rgba, SIZE, SIZE).ok();
        }
    }

    None
}

fn centered_glyph(
    font: &FontRef<'_>,
    ch: char,
    scale: f32,
    canvas_size: f32,
    x_offset: f32,
    y_offset: f32,
) -> Option<Glyph> {
    let glyph_id = font.glyph_id(ch);
    let trial = glyph_id.with_scale_and_position(scale, point(0.0, 0.0));
    let bounds = font.outline_glyph(trial)?.px_bounds();
    let x = -bounds.min.x + (canvas_size - bounds.width()) / 2.0 + x_offset;
    let y = -bounds.min.y + (canvas_size - bounds.height()) / 2.0 + y_offset;
    Some(glyph_id.with_scale_and_position(scale, point(x, y)))
}

fn fallback_tray_icon() -> Icon {
    const SIZE: u32 = 18;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let on =
                ((2..=4).contains(&x) && (2..=15).contains(&y)) ||
                ((12..=14).contains(&x) && (2..=15).contains(&y)) ||
                ((2..=14).contains(&x) && (2..=4).contains(&y)) ||
                ((2..=14).contains(&x) && (13..=15).contains(&y)) ||
                ((6..=10).contains(&x) && (6..=11).contains(&y)) ||
                ((6..=12).contains(&x) && (6..=8).contains(&y)) ||
                ((10..=12).contains(&x) && (6..=11).contains(&y));
            let alpha = if on { 255 } else { 0 };
            rgba.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }

    Icon::from_rgba(rgba, SIZE, SIZE).expect("failed to create tray icon image")
}
