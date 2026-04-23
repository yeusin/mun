use super::hotkey;
use super::SharedState;
use crate::config::{Config, ConfigKey};
use crate::domain::autostart;
use eframe::egui;
use std::sync::{Arc, Mutex};

pub fn show_settings_viewport(
    ctx: &egui::Context,
    state_arc: &Arc<Mutex<SharedState>>,
    show_settings_arc: &Arc<Mutex<bool>>,
    recording_action_arc: &Arc<Mutex<Option<String>>>,
) {
    let state_arc = Arc::clone(state_arc);
    let show_settings_arc = Arc::clone(show_settings_arc);
    let recording_action_arc = Arc::clone(recording_action_arc);

    ctx.show_viewport_deferred(
        egui::ViewportId::from_hash_of("mun_settings"),
        egui::ViewportBuilder::default()
            .with_title("Mun Settings")
            .with_inner_size([450.0, 600.0])
            .with_decorations(true),
        move |ui, class| {
            let ctx = ui.ctx().clone();
            let mut visuals = egui::Visuals::dark();
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 30);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 45);
            ctx.set_visuals(visuals);

            if matches!(
                class,
                egui::ViewportClass::Deferred | egui::ViewportClass::Immediate
            ) {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    ui.add_space(10.0);
                    ui.heading(egui::RichText::new("Mun Preferences").size(24.0).strong());
                    ui.add_space(15.0);

                    egui::ScrollArea::vertical()
                        .max_height(460.0)
                        .show(ui, |ui| {
                            let state = state_arc.lock().unwrap();

                            ui.label(
                                egui::RichText::new("GENERAL")
                                    .color(egui::Color32::from_rgb(100, 150, 255))
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            hotkey_row_ui(
                                ui,
                                "Launcher Toggle",
                                "launcher",
                                &state.config,
                                &recording_action_arc,
                            );

                            ui.add_space(15.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(
                                egui::RichText::new("WINDOW TILING")
                                    .color(egui::Color32::from_rgb(100, 150, 255))
                                    .strong(),
                            );
                            ui.add_space(8.0);

                            ui.label(
                                egui::RichText::new("Halves")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                            for action in ["LeftHalf", "RightHalf", "TopHalf", "BottomHalf"] {
                                hotkey_row_ui(
                                    ui,
                                    action,
                                    action,
                                    &state.config,
                                    &recording_action_arc,
                                );
                            }

                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Quarters")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                            for action in ["TopLeft", "TopRight", "BottomLeft", "BottomRight"] {
                                hotkey_row_ui(
                                    ui,
                                    action,
                                    action,
                                    &state.config,
                                    &recording_action_arc,
                                );
                            }

                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Sixths")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                            for action in [
                                "TopLeftSixth",
                                "TopCenterSixth",
                                "TopRightSixth",
                                "BottomLeftSixth",
                                "BottomCenterSixth",
                                "BottomRightSixth",
                            ] {
                                hotkey_row_ui(
                                    ui,
                                    action,
                                    action,
                                    &state.config,
                                    &recording_action_arc,
                                );
                            }

                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Other")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                            for action in ["Maximize", "Center"] {
                                hotkey_row_ui(
                                    ui,
                                    action,
                                    action,
                                    &state.config,
                                    &recording_action_arc,
                                );
                            }

                            let launch_at_login = state.config.launch_at_login;
                            drop(state);

                            ui.add_space(15.0);
                            ui.separator();
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new("SYSTEM")
                                    .color(egui::Color32::from_rgb(100, 150, 255))
                                    .strong(),
                            );
                            ui.add_space(4.0);
                            let mut launch_at_login = launch_at_login;
                            if ui.checkbox(&mut launch_at_login, "Launch at Login").changed() {
                                let mut state = state_arc.lock().unwrap();
                                state.config.launch_at_login = launch_at_login;
                                state.config.save();
                                autostart::set_autostart(launch_at_login);
                            }
                        });

                    ui.add_space(20.0);
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            let config_path = Config::config_path();
                            let display = config_path.display();
                            ui.label(
                                egui::RichText::new(format!("Config: {}", display))
                                    .size(10.0)
                                    .color(egui::Color32::GRAY),
                            );
                        });
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add(egui::Button::new("Close").min_size(egui::vec2(80.0, 24.0)))
                                .clicked()
                            {
                                *show_settings_arc.lock().unwrap() = false;
                            }
                            if ui
                                .add(
                                    egui::Button::new("Reset Defaults")
                                        .min_size(egui::vec2(120.0, 24.0)),
                                )
                                .clicked()
                            {
                                let mut state = state_arc.lock().unwrap();
                                state.config = Config::default();
                                state.config.save();
                                hotkey::apply_config_internal(&mut state);
                            }
                        });
                    });
                });

                let mut rec_action = recording_action_arc.lock().unwrap();
                if let Some(action) = rec_action.clone() {
                    let mut recorded = None;
                    ctx.input(|i| {
                        for event in &i.events {
                            if let egui::Event::Key {
                                key,
                                pressed: true,
                                repeat: false,
                                modifiers,
                                ..
                            } = event
                            {
                                let key_str = format!("{:?}", key);
                                if is_modifier_only_key(&key_str) {
                                    continue;
                                }

                                recorded = Some(ConfigKey {
                                    modifiers: config_modifiers(*modifiers),
                                    key: key_str,
                                });
                                break;
                            }
                        }
                    });

                    if let Some(new_key) = recorded {
                        let mut state = state_arc.lock().unwrap();
                        if action == "launcher" {
                            state.config.launcher_hotkey = new_key;
                        } else {
                            state.config.window_actions.insert(action, new_key);
                        }
                        state.config.save();
                        hotkey::apply_config_internal(&mut state);
                        *rec_action = None;
                    }
                }

                if ctx.input(|i| i.viewport().close_requested()) {
                    *show_settings_arc.lock().unwrap() = false;
                }
            }
        },
    );
}

fn is_modifier_only_key(key: &str) -> bool {
    matches!(key, "Alt" | "Ctrl" | "Shift" | "Command" | "MacCmd")
}

fn config_modifiers(modifiers: egui::Modifiers) -> Vec<String> {
    let mut recorded = Vec::new();
    if modifiers.alt {
        recorded.push("Alt".to_string());
    }
    if modifiers.ctrl {
        recorded.push("Ctrl".to_string());
    }
    if modifiers.shift {
        recorded.push("Shift".to_string());
    }
    if modifiers.mac_cmd {
        recorded.push("Meta".to_string());
    }
    recorded
}

fn hotkey_row_ui(
    ui: &mut egui::Ui,
    label: &str,
    action_id: &str,
    config: &Config,
    recording_action_arc: &Arc<Mutex<Option<String>>>,
) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        ui.label(egui::RichText::new(format!("{}:", label)).color(egui::Color32::from_gray(200)));

        let key_cfg = if action_id == "launcher" {
            &config.launcher_hotkey
        } else {
            config.window_actions.get(action_id).unwrap()
        };

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let current_rec = recording_action_arc.lock().unwrap();
            let is_recording = current_rec.as_deref() == Some(action_id);

            let btn_text = if is_recording {
                "Recording...".to_string()
            } else {
                let mods = key_cfg.modifiers.join("+");
                if mods.is_empty() {
                    key_cfg.key.clone()
                } else {
                    format!("{}+{}", mods, key_cfg.key)
                }
            };

            let mut btn = egui::Button::new(egui::RichText::new(btn_text).strong())
                .min_size(egui::vec2(120.0, 24.0))
                .corner_radius(egui::CornerRadius::same(6));

            if is_recording {
                btn = btn
                    .fill(egui::Color32::from_rgb(200, 60, 60))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::WHITE));
            } else {
                btn = btn.fill(egui::Color32::from_gray(45));
            }

            if ui.add(btn).clicked() {
                drop(current_rec);
                *recording_action_arc.lock().unwrap() = Some(action_id.to_string());
            }
        });
    });
}
