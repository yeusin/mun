use crate::app_scanner::{scan_apps, AppInfo};
use crate::config::LauncherHistory;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Clone, Debug)]
pub struct SearchResult {
    pub name: String,
    pub exec: String,
    pub score: i64,
    pub history_score: u32,
    pub kind: ResultKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultKind {
    Application,
    WebSearch,
}

pub struct SearchState {
    pub apps: Vec<AppInfo>,
    pub results: Vec<SearchResult>,
    pub selected_idx: usize,
    pub search_query: String,
    pub current_query: String,
    matcher: SkimMatcherV2,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            apps: scan_apps(),
            results: Vec::new(),
            selected_idx: 0,
            search_query: String::new(),
            current_query: String::new(),
            matcher: SkimMatcherV2::default(),
        }
    }

    pub fn rescan_apps(&mut self) {
        self.apps = scan_apps();
    }

    pub fn update_search(&mut self, history: &LauncherHistory) {
        let mut new_results = Vec::new();
        let query = self.search_query.trim().to_lowercase();
        self.current_query = query.clone();

        if query.is_empty() {
            self.results = Vec::new();
            self.selected_idx = 0;
            return;
        }

        for app in &self.apps {
            if let Some(score) = self.matcher.fuzzy_match(&app.name, &query) {
                let history_score = history.get_score(&query, &app.exec);
                new_results.push(SearchResult {
                    name: app.name.clone(),
                    exec: app.exec.clone(),
                    score,
                    history_score,
                    kind: ResultKind::Application,
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
            score: -100,
            history_score,
            kind: ResultKind::WebSearch,
        });

        if history_score > 0 {
            new_results.sort_by(|a, b| {
                b.history_score
                    .cmp(&a.history_score)
                    .then_with(|| b.score.cmp(&a.score))
            });
        }

        new_results.truncate(10);

        self.results = new_results;
        self.selected_idx = 0;
    }

    pub fn execute_selected(&self, history: &mut LauncherHistory) {
        if let Some(result) = self.results.get(self.selected_idx) {
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
                    #[cfg(target_os = "linux")]
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    });
                    #[cfg(target_os = "macos")]
                    std::thread::spawn(move || {
                        let _ = std::process::Command::new("open").arg(&url).spawn();
                    });
                }
            }
        }
    }
}
