mod hotkey;
mod icon_cache;
mod search;
mod settings;

use eframe::egui;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

use crate::config::{Config, LauncherHistory};
use crate::domain::{TrayEvent, WindowAction};
use crate::ports::{AppScanner, BookmarkScanner, Platform, WindowManager};

pub use search::ResultKind;

pub fn run<P: Platform>() -> eframe::Result<()> {
    let config = Config::load();
    let history = LauncherHistory::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 80.0])
            .with_decorations(false)
            .with_always_on_top()
            .with_transparent(true)
            .with_visible(false)
            .with_resizable(false),
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

            let _tray_handle = P::setup_tray(tx);

            let initial_apps = P::create_scanner().scan_apps();
            let mut search_state = search::SearchState::new(initial_apps);
            let apps = search_state.apps.clone();
            search::SearchState::start_background_rescan(|| P::create_scanner().scan_apps(), apps);

            let bookmarks = P::create_bookmark_scanner().scan_bookmarks();
            search_state.set_bookmarks(bookmarks);

            let icon_cache = icon_cache::IconCache::new(28);

            Ok(Box::new(MunLauncher::<P> {
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
                _tray_handle,
                tray_rx: rx,
                initialized: false,
                icon_cache,
                had_focus: false,
                needs_centering: false,
            needs_scroll_reset: false,
            pending_show: false,
            }))
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

struct MunLauncher<P: Platform> {
    state: Arc<Mutex<SharedState>>,
    history: LauncherHistory,
    search: search::SearchState,
    is_visible: bool,
    show_settings: Arc<Mutex<bool>>,
    recording_action: Arc<Mutex<Option<String>>>,
    _tray_handle: P::TrayHandle,
    tray_rx: Receiver<TrayEvent>,
    initialized: bool,
    icon_cache: icon_cache::IconCache,
    had_focus: bool,
    needs_centering: bool,
    needs_scroll_reset: bool,
    pending_show: bool,
}

impl<P: Platform> eframe::App for MunLauncher<P> {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.initialized = true;
        }

        if self.pending_show {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            self.needs_centering = true;
            self.pending_show = false;
        }

        if self.needs_centering && self.is_visible {
            if self.center_on_screen(ctx) {
                self.needs_centering = false;
            }
        }

        if self.is_visible {
            let has_focus = ctx.input(|i| i.focused);
            if !has_focus && self.had_focus {
                self.hide_launcher(ctx);
            }
            self.had_focus = has_focus;
        }

        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                let state = self.state.lock().unwrap();
                if event.id == state.launcher_id {
                    drop(state);
                    self.toggle_launcher(ctx);
                } else if let Some(action_str) = state.tiling_ids.get(&event.id).cloned() {
                    let action = match action_str.as_str() {
                        "LeftHalf" => Some(WindowAction::LeftHalf),
                        "RightHalf" => Some(WindowAction::RightHalf),
                        "TopHalf" => Some(WindowAction::TopHalf),
                        "BottomHalf" => Some(WindowAction::BottomHalf),

                        "TopLeft" => Some(WindowAction::TopLeft),
                        "TopRight" => Some(WindowAction::TopRight),
                        "BottomLeft" => Some(WindowAction::BottomLeft),
                        "BottomRight" => Some(WindowAction::BottomRight),

                        "TopLeftSixth" => Some(WindowAction::TopLeftSixth),
                        "TopCenterSixth" => Some(WindowAction::TopCenterSixth),
                        "TopRightSixth" => Some(WindowAction::TopRightSixth),
                        "BottomLeftSixth" => Some(WindowAction::BottomLeftSixth),
                        "BottomCenterSixth" => Some(WindowAction::BottomCenterSixth),
                        "BottomRightSixth" => Some(WindowAction::BottomRightSixth),

                        "Maximize" => Some(WindowAction::Maximize),
                        "Center" => Some(WindowAction::Center),
                        _ => None,
                    };
                    if let Some(action) = action {
                        std::thread::spawn(move || {
                            P::create_window_manager().perform_action(action);
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

        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.is_visible {
            return;
        }

        let ctx = ui.ctx().clone();
        let available_height = ui.available_height();

        let mut visuals = egui::Visuals::dark();
        visuals.window_corner_radius = egui::CornerRadius::same(14);
        ctx.set_visuals(visuals);

        let panel_frame = egui::Frame::new()
            .fill(egui::Color32::from_black_alpha(230))
            .corner_radius(egui::CornerRadius::same(14))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .inner_margin(egui::Margin::same(12));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show_inside(ui, |ui| {
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
                        if ui.input(|i| i.key_pressed(egui::Key::PageDown)) {
                            self.search.selected_idx =
                                (self.search.selected_idx + 8).min(self.search.results.len() - 1);
                            ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::PageDown)
                            });
                        }
                        if ui.input(|i| i.key_pressed(egui::Key::PageUp)) {
                            self.search.selected_idx = self.search.selected_idx.saturating_sub(8);
                            ui.input_mut(|i| {
                                i.consume_key(egui::Modifiers::NONE, egui::Key::PageUp)
                            });
                        }
                    }

                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.search
                            .execute_selected(&mut self.history, &P::create_browser());
                        self.hide_launcher(&ctx);
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        if self.search.search_query.is_empty() {
                            self.hide_launcher(&ctx);
                        } else {
                            self.search.search_query.clear();
                            self.search.update_search(&self.history);
                            self.needs_scroll_reset = true;
                        }
                        ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                    }

                    ui.horizontal(|ui| {
                        draw_search_icon(ui);
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut self.search.search_query)
                                .hint_text(egui::RichText::new("Search apps or type web search...").size(22.0))
                                .font(egui::FontId::proportional(22.0))
                                .frame(egui::Frame::NONE)
                                .desired_width(f32::INFINITY)
                                .text_color(egui::Color32::WHITE),
                        );

                        if response.changed() {
                            self.search.update_search(&self.history);
                            self.needs_scroll_reset = true;
                        }

                        response.request_focus();
                    });

                    let size_ok = (available_height - self.desired_height()).abs() < 2.0;

                    if !self.search.results.is_empty() && size_ok {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let mut scroll_area = egui::ScrollArea::vertical()
                            .max_height(358.0)
                            .auto_shrink([false, false]);
                        if self.needs_scroll_reset {
                            scroll_area = scroll_area.scroll_offset(egui::Vec2::ZERO);
                            self.needs_scroll_reset = false;
                        }
                        scroll_area.show(ui, |ui| {
                                for (idx, result) in self.search.results.iter().enumerate() {
                                    let is_selected = idx == self.search.selected_idx;
                                    let mut frame = egui::Frame::new()
                                        .inner_margin(egui::Margin::symmetric(14, 8))
                                        .corner_radius(egui::CornerRadius::same(8));

                                    if is_selected {
                                        frame = frame.fill(egui::Color32::from_rgba_unmultiplied(
                                            64, 128, 242, 210,
                                        ));
                                    }

                                    let inner = frame.show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if let Some(texture) =
                                                self.icon_cache.get(&ctx, &result.icon)
                                            {
                                                let img = egui::Image::new(&texture)
                                                    .fit_to_exact_size(egui::vec2(28.0, 28.0));
                                                ui.add(img);
                                            } else {
                                                draw_default_icon(ui);
                                            }
                                            let name_text = highlighted_name(
                                                &result.name,
                                                &result.matched_indices,
                                            );
                                            ui.label(name_text);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    let kind_text = match result.kind {
                                                        ResultKind::Application => "App",
                                                        ResultKind::WebSearch => "Web",
                                                        ResultKind::Bookmark => "Bookmark",
                                                        ResultKind::Calculator => "Calc",
                                                        ResultKind::Url => "URL",
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

                                    if is_selected && idx > 0 {
                                        ui.scroll_to_rect(
                                            inner.response.rect,
                                            Some(egui::Align::Center),
                                        );
                                    }
                                }
                            });
                    }

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("↑↓ Navigate    ↵ Open    esc Dismiss    = Calc")
                                .size(10.0)
                                .color(egui::Color32::from_gray(100)),
                        );
                    });
                });
            });

        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            650.0,
            self.desired_height(),
        )));
    }
}

fn draw_search_icon(ui: &mut egui::Ui) {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
    let center = rect.center();
    let radius = 6.0;
    let color = egui::Color32::from_gray(120);

    ui.painter().circle_stroke(
        egui::pos2(center.x, center.y - 1.0),
        radius,
        egui::Stroke::new(2.0, color),
    );

    let handle_start = egui::pos2(center.x + radius * 0.65, center.y - 1.0 + radius * 0.65);
    let handle_end = egui::pos2(center.x + radius + 4.0, center.y - 1.0 + radius + 4.0);
    ui.painter()
        .line_segment([handle_start, handle_end], egui::Stroke::new(2.5, color));
}

fn draw_default_icon(ui: &mut egui::Ui) {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
    let center = rect.center();
    let color = egui::Color32::from_gray(80);
    let fill = egui::Color32::from_rgba_unmultiplied(80, 80, 80, 60);

    ui.painter().rect_filled(rect.shrink(2.0), 4.0, fill);
    let dot_r = 1.5;
    let offset = 4.0;
    for dx in [-1.0, 1.0] {
        for dy in [-1.0, 1.0] {
            ui.painter().circle_filled(
                egui::pos2(center.x + dx * offset, center.y + dy * offset),
                dot_r,
                color,
            );
        }
    }
}

fn highlighted_name(name: &str, indices: &[usize]) -> egui::WidgetText {
    if indices.is_empty() {
        return egui::RichText::new(name.to_string())
            .size(16.0)
            .color(egui::Color32::WHITE)
            .into();
    }

    let index_set: std::collections::HashSet<usize> = indices.iter().copied().collect();
    let mut layout_job = egui::text::LayoutJob::default();
    for (i, ch) in name.chars().enumerate() {
        let is_match = index_set.contains(&i);
        layout_job.append(
            &ch.to_string(),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::proportional(16.0),
                color: if is_match {
                    egui::Color32::from_rgb(100, 180, 255)
                } else {
                    egui::Color32::WHITE
                },
                underline: if is_match {
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(100, 180, 255))
                } else {
                    egui::Stroke::NONE
                },
                ..Default::default()
            },
        );
    }
    egui::WidgetText::LayoutJob(layout_job.into())
}

impl<P: Platform> MunLauncher<P> {
    fn hide_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = false;
        self.had_focus = false;
        self.needs_centering = false;
        self.pending_show = false;
        self.search.search_query.clear();
        self.search.results.clear();
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
    }

    fn toggle_launcher(&mut self, ctx: &egui::Context) {
        self.is_visible = !self.is_visible;
        if self.is_visible {
            self.search.update_search(&self.history);
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                650.0,
                self.desired_height(),
            )));
            self.pending_show = true;
            self.needs_scroll_reset = true;
            ctx.request_repaint();
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.had_focus = false;
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.search.search_query.clear();
            self.search.results.clear();
            self.had_focus = false;
            self.needs_centering = false;
        }
    }

    fn desired_height(&self) -> f32 {
        if self.search.results.is_empty() {
            106.0
        } else {
            480.0
        }
    }

    fn center_on_screen(&self, ctx: &egui::Context) -> bool {
        if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
            ctx.send_viewport_cmd(cmd);
            true
        } else {
            false
        }
    }
}
