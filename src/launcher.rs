use eframe::egui;
use global_hotkey::{GlobalHotKeyManager, GlobalHotKeyEvent, hotkey::{HotKey, Modifiers, Code}};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use crate::app_scanner::{AppInfo, scan_apps};
use crate::config::{Config, ConfigKey, LauncherHistory};
#[cfg(target_os = "linux")]
use ksni::{Tray, Icon};
#[cfg(target_os = "linux")]
use ksni::menu::{MenuItem, StandardItem};
#[cfg(target_os = "linux")]
use ksni::blocking::{TrayMethods, Handle};
use ab_glyph::{FontRef, Glyph, PxScale, Font};
use image::{RgbaImage, Rgba};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};

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
            let manager = GlobalHotKeyManager::new().expect("Failed to initialize GlobalHotKeyManager");
            
            let mut hotkeys = Vec::new();
            // Register hotkeys from config
            let (launcher_id, launcher_hk) = register_config_hotkey(&manager, &config.launcher_hotkey);
            hotkeys.push(launcher_hk);

            let mut tiling_ids = std::collections::HashMap::new();
            for (action, key) in &config.window_actions {
                let (id, hk) = register_config_hotkey(&manager, key);
                tiling_ids.insert(id, action.clone());
                hotkeys.push(hk);
            }

            let (tx, rx) = channel();
            
            #[cfg(target_os = "linux")]
            let _tray_handle = setup_tray(tx);

            Box::new(MunLauncher {
                state: Arc::new(Mutex::new(SharedState {
                    config,
                    manager,
                    launcher_id,
                    tiling_ids,
                    hotkeys,
                })),
                history,
                search_query: String::new(),
                current_query: String::new(),
                is_visible: false,
                show_settings: Arc::new(Mutex::new(false)),
                recording_action: Arc::new(Mutex::new(None)),
                apps: scan_apps(),
                results: Vec::new(),
                selected_idx: 0,
                matcher: SkimMatcherV2::default(),
                #[cfg(target_os = "linux")]
                _tray_handle,
                tray_rx: rx,
                initialized: false,
            })
        }),
    )
}

struct SharedState {
    config: Config,
    manager: GlobalHotKeyManager,
    launcher_id: u32,
    tiling_ids: std::collections::HashMap<u32, String>,
    hotkeys: Vec<HotKey>,
}

struct MunLauncher {
    state: Arc<Mutex<SharedState>>,
    history: LauncherHistory,
    search_query: String,
    current_query: String, // query used for last search
    is_visible: bool,
    show_settings: Arc<Mutex<bool>>,
    recording_action: Arc<Mutex<Option<String>>>,
    apps: Vec<AppInfo>,
    results: Vec<SearchResult>,
    selected_idx: usize,
    matcher: SkimMatcherV2,
    #[cfg(target_os = "linux")]
    _tray_handle: Handle<MunTray>,
    tray_rx: Receiver<TrayEvent>,
    initialized: bool,
}

#[derive(Clone, Debug)]
struct SearchResult {
    name: String,
    exec: String,
    score: i64,
    history_score: u32,
    kind: ResultKind,
}

#[derive(Clone, Debug, PartialEq)]
enum ResultKind {
    Application,
    WebSearch,
}

impl eframe::App for MunLauncher {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Force initial hide if not yet initialized
        if !self.initialized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.initialized = true;
        }

        // Handle global hotkeys - process all queued events
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                let state = self.state.lock().unwrap();
                if event.id == state.launcher_id {
                    drop(state);
                    self.toggle_launcher(ctx);
                } else if let Some(action_str) = state.tiling_ids.get(&event.id).cloned() {
                    println!("Hotkey: Window tiling action pressed: {}", action_str);
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
                        "TopCenterSixth" => Some(crate::window_manager::WindowAction::TopCenterSixth),
                        "TopRightSixth" => Some(crate::window_manager::WindowAction::TopRightSixth),
                        "BottomLeftSixth" => Some(crate::window_manager::WindowAction::BottomLeftSixth),
                        "BottomCenterSixth" => Some(crate::window_manager::WindowAction::BottomCenterSixth),
                        "BottomRightSixth" => Some(crate::window_manager::WindowAction::BottomRightSixth),
                        
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

        // Handle tray events
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
            self.show_settings_viewport(ctx);
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(50));

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
                        // Handle key navigation BEFORE the text edit to consume events
                        if !self.results.is_empty() {
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                                self.selected_idx = (self.selected_idx + 1) % self.results.len();
                                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                                self.selected_idx = if self.selected_idx == 0 {
                                    self.results.len() - 1
                                } else {
                                    self.selected_idx - 1
                                };
                                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                            }
                        }

                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.execute_selected(ctx);
                            ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.hide_launcher(ctx);
                            ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                        }

                        // Search field with Alfred-like styling
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search_query)
                                .hint_text("Search apps or type web search...")
                                .font(egui::FontId::proportional(22.0))
                                .frame(false)
                                .desired_width(f32::INFINITY)
                                .text_color(egui::Color32::WHITE)
                        );
                        
                        if response.changed() {
                            self.update_search();
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Results list
                        let mut clicked_idx = None;
                        egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                            for (idx, result) in self.results.iter().enumerate() {
                                let is_selected = idx == self.selected_idx;
                                let mut frame = egui::Frame::none()
                                    .inner_margin(egui::Margin::symmetric(14.0, 8.0))
                                    .rounding(8.0);
                                
                                if is_selected {
                                    frame = frame.fill(egui::Color32::from_rgba_unmultiplied(64, 128, 242, 210));
                                }

                                frame.show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new(&result.name).size(16.0).color(egui::Color32::WHITE));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            let kind_text = match result.kind {
                                                ResultKind::Application => "App",
                                                ResultKind::WebSearch => "Web",
                                            };
                                            ui.label(egui::RichText::new(kind_text).size(11.0).color(egui::Color32::from_gray(140)));
                                        });
                                    });
                                });
                                
                                if ui.input(|i| i.pointer.any_click()) && ui.rect_contains_pointer(ui.max_rect()) {
                                    clicked_idx = Some(idx);
                                }
                            }
                        });

                        if let Some(idx) = clicked_idx {
                            self.selected_idx = idx;
                            self.execute_selected(ctx);
                        }

                        // Footer hint
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("↑↓ Navigate    ↵ Open    esc Dismiss").size(10.0).color(egui::Color32::from_gray(100)));
                        });
                        
                        response.request_focus();
                    });
                });
        }
    }
}

impl MunLauncher {
    fn update_search(&mut self) {
        let mut new_results = Vec::new();
        let query = self.search_query.trim().to_lowercase();
        self.current_query = query.clone();

        if query.is_empty() {
            self.results = Vec::new();
            self.selected_idx = 0;
            return;
        }

        let state = self.state.lock().unwrap();
        // 1. Check apps
        for app in &self.apps {
            if let Some(score) = self.matcher.fuzzy_match(&app.name, &query) {
                let history_score = self.history.get_score(&query, &app.exec);
                new_results.push(SearchResult {
                    name: app.name.clone(),
                    exec: app.exec.clone(),
                    score,
                    history_score,
                    kind: ResultKind::Application,
                });
            }
        }
        drop(state);

        // Sort by history count first, then by fuzzy score
        new_results.sort_by(|a, b| {
            b.history_score.cmp(&a.history_score)
                .then_with(|| b.score.cmp(&a.score))
        });

        // 2. Fallback: Web search (always at bottom)
        let web_exec = format!("https://www.google.com/search?q={}", urlencoding::encode(&self.search_query));
        let history_score = self.history.get_score(&query, &web_exec);
        new_results.push(SearchResult {
            name: format!("Search Google for \"{}\"", self.search_query),
            exec: web_exec,
            score: -100,
            history_score,
            kind: ResultKind::WebSearch,
        });

        // Re-sort after adding web search if it has history
        if history_score > 0 {
            new_results.sort_by(|a, b| {
                b.history_score.cmp(&a.history_score)
                    .then_with(|| b.score.cmp(&a.score))
            });
        }

        // Limit to 10 results
        new_results.truncate(10);

        self.results = new_results;
        self.selected_idx = 0;
    }

    fn execute_selected(&mut self, ctx: &egui::Context) {
        if let Some(result) = self.results.get(self.selected_idx) {
            // Record usage in history for the current query
            self.history.record(&self.current_query, &result.exec);

            match result.kind {
                ResultKind::Application => {
                    let cmd = result.exec.clone();
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn();
                    });
                }
                ResultKind::WebSearch => {
                    let url = result.exec.clone();
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    });
                }
            }
        }
        self.hide_launcher(ctx);
    }

    fn hide_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = false;
        self.search_query.clear();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn toggle_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = !self.is_visible;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.is_visible));
        if self.is_visible {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.update_search();
        } else {
            self.search_query.clear();
        }
    }

    fn show_settings_viewport(&mut self, ctx: &egui::Context) {
        let state_arc = Arc::clone(&self.state);
        let show_settings_arc = Arc::clone(&self.show_settings);
        let recording_action_arc = Arc::clone(&self.recording_action);

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("mun_settings"),
            egui::ViewportBuilder::default()
                .with_title("Mun Settings")
                .with_inner_size([450.0, 600.0])
                .with_decorations(true),
            move |ctx, class| {
                let mut visuals = egui::Visuals::dark();
                visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(30, 30, 30);
                visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 45, 45);
                ctx.set_visuals(visuals);

                if class == egui::ViewportClass::Immediate {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add_space(10.0);
                        ui.heading(egui::RichText::new("Mun Preferences").size(24.0).strong());
                        ui.add_space(15.0);

                        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                            let mut state = state_arc.lock().unwrap();
                            
                            ui.label(egui::RichText::new("GENERAL").color(egui::Color32::from_rgb(100, 150, 255)).strong());
                            ui.add_space(4.0);
                            hotkey_row_ui(ui, "Launcher Toggle", "launcher", &mut state, &recording_action_arc);
                            
                            ui.add_space(15.0);
                            ui.separator();
                            ui.add_space(10.0);

                            ui.label(egui::RichText::new("WINDOW TILING").color(egui::Color32::from_rgb(100, 150, 255)).strong());
                            ui.add_space(8.0);
                            
                            ui.label(egui::RichText::new("Halves").italics().color(egui::Color32::GRAY));
                            for action in ["LeftHalf", "RightHalf", "TopHalf", "BottomHalf"] {
                                hotkey_row_ui(ui, action, action, &mut state, &recording_action_arc);
                            }

                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Quarters").italics().color(egui::Color32::GRAY));
                            for action in ["TopLeft", "TopRight", "BottomLeft", "BottomRight"] {
                                hotkey_row_ui(ui, action, action, &mut state, &recording_action_arc);
                            }

                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Sixths").italics().color(egui::Color32::GRAY));
                            for action in ["TopLeftSixth", "TopCenterSixth", "TopRightSixth", "BottomLeftSixth", "BottomCenterSixth", "BottomRightSixth"] {
                                hotkey_row_ui(ui, action, action, &mut state, &recording_action_arc);
                            }

                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Other").italics().color(egui::Color32::GRAY));
                            for action in ["Maximize", "Center"] {
                                hotkey_row_ui(ui, action, action, &mut state, &recording_action_arc);
                            }
                        });

                        ui.add_space(20.0);
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("Config: ~/.config/mun/config.json").size(10.0).color(egui::Color32::GRAY));
                            });
                            ui.add_space(5.0);
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new("Close").min_size(egui::vec2(80.0, 24.0))).clicked() {
                                    *show_settings_arc.lock().unwrap() = false;
                                }
                                if ui.add(egui::Button::new("Reset Defaults").min_size(egui::vec2(120.0, 24.0))).clicked() {
                                    let mut state = state_arc.lock().unwrap();
                                    state.config = Config::default();
                                    state.config.save();
                                    apply_config_internal(&mut state);
                                }
                            });
                        });
                    });

                    // Handle recording inside the viewport
                    let mut rec_action = recording_action_arc.lock().unwrap();
                    if let Some(action) = rec_action.clone() {
                        let mut recorded = None;
                        ctx.input(|i| {
                            for key in egui::Key::ALL {
                                if i.key_pressed(*key) {
                                    let key_str = format!("{:?}", key);
                                    if !["Alt", "Ctrl", "Shift", "Command", "MacCmd"].contains(&key_str.as_str()) {
                                        let mut modifiers = Vec::new();
                                        // Specific Linux fix: Often Ctrl is reported with MacCmd/Command flag true in some contexts
                                        // We ensure we only record what's actually pressed.
                                        if i.modifiers.alt { modifiers.push("Alt".to_string()); }
                                        if i.modifiers.ctrl { modifiers.push("Ctrl".to_string()); }
                                        if i.modifiers.shift { modifiers.push("Shift".to_string()); }
                                        // Only add Meta if it's NOT just a side-effect of Ctrl/Alt
                                        if (i.modifiers.mac_cmd || i.modifiers.command) && !i.modifiers.ctrl {
                                            modifiers.push("Meta".to_string()); 
                                        }
                                        
                                        recorded = Some(ConfigKey { modifiers, key: key_str });
                                        break;
                                    }
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
                            apply_config_internal(&mut state);
                            *rec_action = None;
                        }
                    }

                    if ctx.input(|i| i.viewport().close_requested()) {
                        *show_settings_arc.lock().unwrap() = false;
                    }
                }
            }
        );
    }
}

fn hotkey_row_ui(ui: &mut egui::Ui, label: &str, action_id: &str, state: &mut SharedState, recording_action_arc: &Arc<Mutex<Option<String>>>) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.set_height(28.0);
        ui.label(egui::RichText::new(format!("{}:", label)).color(egui::Color32::from_gray(200)));
        
        let key_cfg = if action_id == "launcher" {
            &state.config.launcher_hotkey
        } else {
            state.config.window_actions.get(action_id).unwrap()
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
                .rounding(6.0);
            
            if is_recording {
                btn = btn.fill(egui::Color32::from_rgb(200, 60, 60))
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

fn apply_config_internal(state: &mut SharedState) {
    // Unregister all
    for hk in state.hotkeys.drain(..) {
        let _ = state.manager.unregister(hk);
    }
    state.tiling_ids.clear();

    // Re-register
    let (l_id, l_hk) = register_config_hotkey(&state.manager, &state.config.launcher_hotkey);
    state.launcher_id = l_id;
    state.hotkeys.push(l_hk);

    for (action, key) in &state.config.window_actions {
        let (id, hk) = register_config_hotkey(&state.manager, key);
        state.tiling_ids.insert(id, action.clone());
        state.hotkeys.push(hk);
    }
}

fn register_config_hotkey(manager: &GlobalHotKeyManager, config_key: &ConfigKey) -> (u32, HotKey) {
    let mut modifiers = Modifiers::empty();
    for m in &config_key.modifiers {
        match m.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "meta" | "super" | "command" | "win" => modifiers |= Modifiers::META,
            _ => {}
        }
    }

    let code = str_to_code(&config_key.key);
    let hotkey = HotKey::new(Some(modifiers), code);
    if let Err(e) = manager.register(hotkey) {
        eprintln!("Failed to register hotkey {:?}: {:?}", hotkey, e);
    }
    (hotkey.id(), hotkey)
}

fn str_to_code(s: &str) -> Code {
    match s.to_lowercase().as_str() {
        "space" => Code::Space,
        "arrowleft" | "left" => Code::ArrowLeft,
        "arrowright" | "right" => Code::ArrowRight,
        "arrowup" | "up" => Code::ArrowUp,
        "arrowdown" | "down" => Code::ArrowDown,
        "keya" | "a" => Code::KeyA, "keyb" | "b" => Code::KeyB, "keyc" | "c" => Code::KeyC,
        "keyd" | "d" => Code::KeyD, "keye" | "e" => Code::KeyE, "keyf" | "f" => Code::KeyF,
        "keyg" | "g" => Code::KeyG, "keyh" | "h" => Code::KeyH, "keyi" | "i" => Code::KeyI,
        "keyj" | "j" => Code::KeyJ, "keyk" | "k" => Code::KeyK, "keyl" | "l" => Code::KeyL,
        "keym" | "m" => Code::KeyM, "keyn" | "n" => Code::KeyN, "keyo" | "o" => Code::KeyO,
        "keyp" | "p" => Code::KeyP, "keyq" | "q" => Code::KeyQ, "keyr" | "r" => Code::KeyR,
        "keys" | "s" => Code::KeyS, "keyt" | "t" => Code::KeyT, "keyu" | "u" => Code::KeyU,
        "keyv" | "v" => Code::KeyV, "keyw" | "w" => Code::KeyW, "keyx" | "x" => Code::KeyX,
        "keyy" | "y" => Code::KeyY, "keyz" | "z" => Code::KeyZ,
        "num1" | "d1" | "1" => Code::Digit1, "num2" | "d2" | "2" => Code::Digit2,
        "num3" | "d3" | "3" => Code::Digit3, "num4" | "d4" | "4" => Code::Digit4,
        "num5" | "d5" | "5" => Code::Digit5, "num6" | "d6" | "6" => Code::Digit6,
        "num7" | "d7" | "7" => Code::Digit7, "num8" | "d8" | "8" => Code::Digit8,
        "num9" | "d9" | "9" => Code::Digit9, "num0" | "d0" | "0" => Code::Digit0,
        "enter" => Code::Enter,
        "escape" => Code::Escape,
        "tab" => Code::Tab,
        "backspace" => Code::Backspace,
        "insert" => Code::Insert,
        "delete" => Code::Delete,
        "home" => Code::Home,
        "end" => Code::End,
        "pageup" => Code::PageUp,
        "pagedown" => Code::PageDown,
        "f1" => Code::F1, "f2" => Code::F2, "f3" => Code::F3, "f4" => Code::F4,
        "f5" => Code::F5, "f6" => Code::F6, "f7" => Code::F7, "f8" => Code::F8,
        "f9" => Code::F9, "f10" => Code::F10, "f11" => Code::F11, "f12" => Code::F12,
        _ => Code::KeyA,
    }
}

enum TrayEvent {
    Toggle,
    Settings,
    Quit,
}

#[cfg(target_os = "linux")]
struct MunTray {
    sender: Sender<TrayEvent>,
    icon_data: Vec<u8>,
}

#[cfg(target_os = "linux")]
impl Tray for MunTray {
    fn id(&self) -> String { "mun-launcher".into() }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![Icon {
            width: 32,
            height: 32,
            data: self.icon_data.clone(),
        }]
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let tx = self.sender.clone();
        let tx_settings = self.sender.clone();
        let tx_quit = self.sender.clone();
        vec![
            StandardItem {
                label: "Show/Hide Launcher".into(),
                activate: Box::new(move |_| { let _ = tx.send(TrayEvent::Toggle); }),
                ..Default::default()
            }.into(),
            StandardItem {
                label: "Settings".into(),
                activate: Box::new(move |_| { let _ = tx_settings.send(TrayEvent::Settings); }),
                ..Default::default()
            }.into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(move |_| { let _ = tx_quit.send(TrayEvent::Quit); }),
                ..Default::default()
            }.into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.sender.send(TrayEvent::Toggle);
    }
}

#[cfg(target_os = "linux")]
fn setup_tray(tx: Sender<TrayEvent>) -> Handle<MunTray> {
    let icon_data = render_icon_text("문");
    let tray = MunTray {
        sender: tx,
        icon_data,
    };
    tray.spawn().expect("Failed to spawn tray")
}

fn render_icon_text(text: &str) -> Vec<u8> {
    let size = 32;
    let mut image = RgbaImage::new(size, size);

    // Common Linux font paths
    let font_paths = [
        "/usr/share/fonts/truetype/baekmuk/batang.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Medium.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    let font_data = font_paths.iter()
        .find_map(|path| std::fs::read(path).ok())
        .unwrap_or_else(|| {
            // Fallback: draw a simple square if no font found
            let mut data = Vec::new();
            for _ in 0..size * size {
                // ARGB big-endian
                data.extend_from_slice(&[255, 255, 255, 255]);
            }
            data
        });
    
    if font_data.len() == (size * size * 4) as usize {
        return font_data;
    }

    let font = FontRef::try_from_slice(&font_data).expect("Failed to load font");
    let scale = PxScale::from(26.0);
    let glyph: Glyph = font.glyph_id(text.chars().next().unwrap()).with_scale_and_position(scale, ab_glyph::point(3.0, 26.0));

    if let Some(outlined) = font.outline_glyph(glyph) {
        let bounds = outlined.px_bounds();
        outlined.draw(|x, y, v| {
            let px = x + bounds.min.x as u32;
            let py = y + bounds.min.y as u32;
            if px < size && py < size {
                // ksni expects ARGB format for the pixmap data (big-endian/network order)
                // byte 0: A, byte 1: R, byte 2: G, byte 3: B
                image.put_pixel(px, py, Rgba([(v * 255.0) as u8, 255, 255, 255]));
            }
        });
    }

    image.into_raw()
}
