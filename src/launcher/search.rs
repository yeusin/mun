use crate::config::LauncherHistory;
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
}

pub struct SearchState {
    pub apps: Arc<Mutex<Vec<AppInfo>>>,
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
            results: Vec::new(),
            selected_idx: 0,
            search_query: String::new(),
            current_query: String::new(),
            matcher: SkimMatcherV2::default(),
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

        new_results.sort_by(|a, b| {
            b.history_score
                .cmp(&a.history_score)
                .then_with(|| b.score.cmp(&a.score))
        });

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

        if history_score > 0 {
            new_results.sort_by(|a, b| {
                b.history_score
                    .cmp(&a.history_score)
                    .then_with(|| b.score.cmp(&a.score))
            });
        }

        new_results.truncate(10_000);

        self.results = new_results;
        self.selected_idx = 0;
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
            history.record(&self.current_query, &result.exec);

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
                ResultKind::WebSearch => {
                    let url = result.exec.clone();
                    browser.open_url(&url);
                }
            }
        }
    }
}
