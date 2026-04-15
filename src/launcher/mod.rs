mod hotkey;
pub mod icon;
mod search;
mod settings;
mod tray;

use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use crate::config::{Config, LauncherHistory};

pub use search::ResultKind;
pub use tray::TrayEvent;

pub fn run() -> eframe::Result<()> {
    let config = Config::load();
    let history = LauncherHistory::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 480.0])
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_visible(false),
        ..Default::default()
    };

    eframe::run_native(
        "Mun Launcher",
        options,
        Box::new(|_cc| {
            let manager =
                GlobalHotKeyManager::new().expect("Failed to initialize GlobalHotKeyManager");

            let mut hotkeys = Vec::new();
            let (launcher_id, launcher_hk) =
                hotkey::register_config_hotkey(&manager, &config.launcher_hotkey);
            hotkeys.push(launcher_hk);

            let mut tiling_ids = std::collections::HashMap::new();
            for (action, key) in &config.window_actions {
                let (id, hk) = hotkey::register_config_hotkey(&manager, key);
                tiling_ids.insert(id, action.clone());
                hotkeys.push(hk);
            }

            let (tx, rx) = channel();

            #[cfg(target_os = "linux")]
            let _tray_handle = tray::setup_tray(tx);

            let search_state = search::SearchState::new();
            search::SearchState::start_background_rescan(search_state.apps.clone());

            Box::new(MunLauncher {
                state: Arc::new(Mutex::new(SharedState {
                    config,
                    manager,
                    launcher_id,
                    tiling_ids,
                    hotkeys,
                })),
                history,
                search: search_state,
                is_visible: false,
                show_settings: Arc::new(Mutex::new(false)),
                recording_action: Arc::new(Mutex::new(None)),
                #[cfg(target_os = "linux")]
                _tray_handle,
                tray_rx: rx,
                initialized: false,
            })
        }),
    )
}

pub struct SharedState {
    pub config: Config,
    pub manager: GlobalHotKeyManager,
    pub launcher_id: u32,
    pub tiling_ids: std::collections::HashMap<u32, String>,
    pub hotkeys: Vec<HotKey>,
}

struct MunLauncher {
    state: Arc<Mutex<SharedState>>,
    history: LauncherHistory,
    search: search::SearchState,
    is_visible: bool,
    show_settings: Arc<Mutex<bool>>,
    recording_action: Arc<Mutex<Option<String>>>,
    #[cfg(target_os = "linux")]
    _tray_handle: ksni::blocking::Handle<tray::MunTray>,
    tray_rx: Receiver<TrayEvent>,
    initialized: bool,
}

impl eframe::App for MunLauncher {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.initialized = true;
        }

        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                let state = self.state.lock().unwrap();
                if event.id == state.launcher_id {
                    drop(state);
                    self.toggle_launcher(ctx);
                } else if let Some(action_str) = state.tiling_ids.get(&event.id).cloned() {
                    let action = match action_str.as_str() {
                        "LeftHalf" => Some(crate::window_manager::WindowAction::LeftHalf),
                        "RightHalf" => Some(crate::window_manager::WindowAction::RightHalf),
                        "TopHalf" => Some(crate::window_manager::WindowAction::TopHalf),
                        "BottomHalf" => Some(crate::window_manager::WindowAction::BottomHalf),

                        "TopLeft" => Some(crate::window_manager::WindowAction::TopLeft),
                        "TopRight" => Some(crate::window_manager::WindowAction::TopRight),
                        "BottomLeft" => Some(crate::window_manager::WindowAction::BottomLeft),
                        "BottomRight" => Some(crate::window_manager::WindowAction::BottomRight),

                        "TopLeftSixth" => Some(crate::window_manager::WindowAction::TopLeftSixth),
                        "TopCenterSixth" => {
                            Some(crate::window_manager::WindowAction::TopCenterSixth)
                        }
                        "TopRightSixth" => Some(crate::window_manager::WindowAction::TopRightSixth),
                        "BottomLeftSixth" => {
                            Some(crate::window_manager::WindowAction::BottomLeftSixth)
                        }
                        "BottomCenterSixth" => {
                            Some(crate::window_manager::WindowAction::BottomCenterSixth)
                        }
                        "BottomRightSixth" => {
                            Some(crate::window_manager::WindowAction::BottomRightSixth)
                        }

                        "Maximize" => Some(crate::window_manager::WindowAction::Maximize),
                        "Center" => Some(crate::window_manager::WindowAction::Center),
                        _ => None,
                    };
                    if let Some(action) = action {
                        std::thread::spawn(move || {
                            crate::window_manager::perform_action(action);
                        });
                    }
                }
            }
        }

        while let Ok(event) = self.tray_rx.try_recv() {
            match event {
                TrayEvent::Toggle => self.toggle_launcher(ctx),
                TrayEvent::Settings => {
                    let mut show = self.show_settings.lock().unwrap();
                    *show = true;
                }
                TrayEvent::Quit => std::process::exit(0),
            }
        }

        let show_settings = *self.show_settings.lock().unwrap();
        if show_settings {
            settings::show_settings_viewport(
                ctx,
                &self.state,
                &self.show_settings,
                &self.recording_action,
            );
        }

        if self.is_visible {
            let mut visuals = egui::Visuals::dark();
            visuals.window_rounding = 14.0.into();
            ctx.set_visuals(visuals);

            let panel_frame = egui::Frame::none()
                .fill(egui::Color32::from_black_alpha(230))
                .rounding(14.0)
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
                .inner_margin(egui::Margin::same(12.0));

            egui::CentralPanel::default()
                .frame(panel_frame)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        if !self.search.results.is_empty() {
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                self.search.selected_idx =
                                    (self.search.selected_idx + 1) % self.search.results.len();
                                ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown)
                                });
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                self.search.selected_idx = if self.search.selected_idx == 0 {
                                    self.search.results.len() - 1
                                } else {
                                    self.search.selected_idx - 1
                                };
                                ui.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)
                                });
                            }
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.search.execute_selected(&mut self.history);
                            self.hide_launcher(ctx);
                            ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                            });
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.hide_launcher(ctx);
                            ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                            });
                        }

                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search.search_query)
                                .hint_text("Search apps or type web search...")
                                .font(egui::FontId::proportional(22.0))
                                .frame(false)
                                .desired_width(f32::INFINITY)
                                .text_color(egui::Color32::WHITE),
                        );

                        if response.changed() {
                            self.search.update_search(&self.history);
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let mut clicked_idx = None;
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                for (idx, result) in self.search.results.iter().enumerate() {
                                    let is_selected = idx == self.search.selected_idx;
                                    let mut frame = egui::Frame::none()
                                        .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                                        .rounding(8.0);

                                    if is_selected {
                                        frame = frame.fill(egui::Color32::from_rgba_unmultiplied(
                                            64, 128, 242, 210,
                                        ));
                                    }

                                    frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(&result.name)
                                                    .size(16.0)
                                                    .color(egui::Color32::WHITE),
                                            );
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let kind_text = match result.kind {
                                                        ResultKind::Application => "App",
                                                        ResultKind::WebSearch => "Web",
                                                    };
                                                    ui.label(
                                                        egui::RichText::new(kind_text)
                                                            .size(11.0)
                                                            .color(egui::Color32::from_gray(140)),
                                                    );
                                                },
                                            );
                                        });
                                    });

                                    if ui.input(|i| i.pointer.any_click())
                                        && ui.rect_contains_pointer(ui.max_rect())
                                    {
                                        clicked_idx = Some(idx);
                                    }
                                }
                            });

                        if let Some(idx) = clicked_idx {
                            self.search.selected_idx = idx;
                            self.search.execute_selected(&mut self.history);
                            self.hide_launcher(ctx);
                        }

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("↑↓ Navigate    ↵ Open    esc Dismiss")
                                    .size(10.0)
                                    .color(egui::Color32::from_gray(100)),
                            );
                        });

                        response.request_focus();
                    });
                });
        }
    }
}

impl MunLauncher {
    fn hide_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = false;
        self.search.search_query.clear();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn toggle_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = !self.is_visible;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.is_visible));
        if self.is_visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.search.update_search(&self.history);
        } else {
            self.search.search_query.clear();
        }
    }
}
