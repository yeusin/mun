use crate::config::LauncherHistory;
use crate::domain::calculator;
use crate::domain::AppInfo;
use crate::ports::BrowserLauncher;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub exec: String,
    #[allow(dead_code)]
    pub icon: Option<String>,
    pub score: i64,
    pub history_score: u32,
    pub kind: ResultKind,
    pub matched_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultKind {
    Application,
    WebSearch,
    Bookmark,
    Calculator,
    Url,
}

pub struct SearchState {
    pub apps: Arc<Mutex<Vec<AppInfo>>>,
    pub bookmarks: Arc<Mutex<Vec<AppInfo>>>,
    pub results: Vec<SearchResult>,
    pub selected_idx: usize,
    pub search_query: String,
    pub current_query: String,
    matcher: SkimMatcherV2,
}

impl SearchState {
    pub fn new(apps: Vec<AppInfo>) -> Self {
        Self {
            apps: Arc::new(Mutex::new(apps)),
            bookmarks: Arc::new(Mutex::new(Vec::new())),
            results: Vec::new(),
            selected_idx: 0,
            search_query: String::new(),
            current_query: String::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn set_bookmarks(&mut self, bookmarks: Vec<AppInfo>) {
        if let Ok(mut locked) = self.bookmarks.lock() {
            *locked = bookmarks;
        }
    }

    pub fn start_background_rescan<F>(scan_fn: F, apps: Arc<Mutex<Vec<AppInfo>>>)
    where
        F: Fn() -> Vec<AppInfo> + Send + 'static,
    {
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(30));
            let scanned = scan_fn();
            if let Ok(mut locked) = apps.lock() {
                *locked = scanned;
            }
        });
    }

    pub fn update_search(&mut self, history: &LauncherHistory) {
        let mut new_results = Vec::new();
        let query = self.search_query.trim().to_lowercase();
        self.current_query = query.clone();

        if query.is_empty() {
            self.results = self.build_recents(history);
            self.selected_idx = 0;
            return;
        }

        if query.starts_with('=') {
            self.results = self.build_calculator_results(&query[1..]);
            self.selected_idx = 0;
            return;
        }

        let apps = self.apps.lock().unwrap();
        for app in apps.iter() {
            if let Some((score, indices)) = self.matcher.fuzzy_indices(&app.name, &query) {
                let history_score = history.get_score(&query, &app.exec);
                new_results.push(SearchResult {
                    name: app.name.clone(),
                    exec: app.exec.clone(),
                    icon: app.icon.clone(),
                    score,
                    history_score,
                    kind: ResultKind::Application,
                    matched_indices: indices,
                });
            }
        }

        let bookmarks = self.bookmarks.lock().unwrap();
        for bm in bookmarks.iter() {
            if let Some((score, indices)) = self.matcher.fuzzy_indices(&bm.name, &query) {
                let history_score = history.get_score(&query, &bm.exec);
                new_results.push(SearchResult {
                    name: bm.name.clone(),
                    exec: bm.exec.clone(),
                    icon: bm.icon.clone(),
                    score,
                    history_score,
                    kind: ResultKind::Bookmark,
                    matched_indices: indices,
                });
            }
        }

        if is_domain(&self.search_query) {
            let url = if self.search_query.contains("://") {
                self.search_query.clone()
            } else {
                format!("https://{}", self.search_query.trim())
            };
            let history_score = history.get_score(&query, &url);
            new_results.push(SearchResult {
                name: format!("Open \"{}\"", self.search_query.trim()),
                exec: url,
                icon: None,
                score: -50,
                history_score,
                kind: ResultKind::Url,
                matched_indices: Vec::new(),
            });
        }

        let web_exec = format!(
            "https://www.google.com/search?q={}",
            urlencoding::encode(&self.search_query)
        );
        let history_score = history.get_score(&query, &web_exec);
        new_results.push(SearchResult {
            name: format!("Search Google for \"{}\"", self.search_query),
            exec: web_exec,
            icon: None,
            score: -100,
            history_score,
            kind: ResultKind::WebSearch,
            matched_indices: Vec::new(),
        });

        new_results.sort_by(|a, b| {
            b.history_score
                .cmp(&a.history_score)
                .then_with(|| b.score.cmp(&a.score))
        });

        new_results.truncate(10_000);

        self.results = new_results;
        self.selected_idx = 0;
    }

    fn build_calculator_results(&self, expr: &str) -> Vec<SearchResult> {
        let expr = expr.trim();
        if expr.is_empty() {
            return vec![SearchResult {
                name: "Calculator — type an expression after =".to_string(),
                exec: "calc:".to_string(),
                icon: None,
                score: 0,
                history_score: 0,
                kind: ResultKind::Calculator,
                matched_indices: Vec::new(),
            }];
        }
        match calculator::evaluate(expr) {
            Some(value) => {
                let formatted = calculator::format_result(value);
                vec![SearchResult {
                    name: format!("= {}", formatted),
                    exec: format!("calc:{}", formatted),
                    icon: None,
                    score: 0,
                    history_score: 0,
                    kind: ResultKind::Calculator,
                    matched_indices: Vec::new(),
                }]
            }
            None => vec![SearchResult {
                name: "Calculator (Error)".to_string(),
                exec: "calc:error".to_string(),
                icon: None,
                score: 0,
                history_score: 0,
                kind: ResultKind::Calculator,
                matched_indices: Vec::new(),
            }],
        }
    }

    fn build_recents(&self, history: &LauncherHistory) -> Vec<SearchResult> {
        let top = history.top_execs_overall(50);
        let apps = self.apps.lock().unwrap();
        let mut results = Vec::new();
        for (exec, count) in &top {
            if let Some(app) = apps.iter().find(|a| a.exec == *exec) {
                results.push(SearchResult {
                    name: app.name.clone(),
                    exec: app.exec.clone(),
                    icon: app.icon.clone(),
                    score: 0,
                    history_score: *count,
                    kind: ResultKind::Application,
                    matched_indices: Vec::new(),
                });
            }
        }
        results
    }

    pub fn execute_selected(&self, history: &mut LauncherHistory, browser: &impl BrowserLauncher) {
        if let Some(result) = self.results.get(self.selected_idx) {
            log::info!("Executing: {} ({:?})", result.name, result.kind);

            if result.kind != ResultKind::Calculator {
                history.record(&self.current_query, &result.exec);
            }

            match result.kind {
                ResultKind::Application => {
                    let cmd = result.exec.clone();
                    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(&cmd)
                            .current_dir(&home)
                            .spawn();
                    });
                }
                ResultKind::WebSearch | ResultKind::Url | ResultKind::Bookmark => {
                    let url = result.exec.clone();
                    browser.open_url(&url);
                }
                ResultKind::Calculator => {
                    if let Some(value) = result.exec.strip_prefix("calc:") {
                        if value != "error" && !value.is_empty() {
                            let val = value.to_string();
                            std::thread::spawn(move || {
                                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                    if let Err(e) = clipboard.set_text(&val) {
                                        log::error!("Failed to copy to clipboard: {}", e);
                                    }
                                }
                            });
                            log::info!("Copied to clipboard: {}", value);
                        }
                    }
                }
            }
        }
    }
}

fn is_domain(query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() || query.contains(' ') {
        return false;
    }
    if query.starts_with("http://") || query.starts_with("https://") {
        return true;
    }
    if !query.contains('.') {
        return false;
    }
    let parts: Vec<&str> = query.rsplit('.').collect();
    parts.first().map_or(false, |tld| tld.len() >= 2)
}
