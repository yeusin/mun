use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

const MAX_HISTORY_QUERIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub launcher_hotkey: ConfigKey,
    pub window_actions: HashMap<String, ConfigKey>,
    #[serde(default)]
    pub launch_at_login: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigKey {
    pub modifiers: Vec<String>,
    pub key: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut window_actions = HashMap::new();
        let mods = vec!["Alt".to_string(), "Ctrl".to_string()];

        window_actions.insert(
            "LeftHalf".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Left".to_string(),
            },
        );
        window_actions.insert(
            "RightHalf".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Right".to_string(),
            },
        );
        window_actions.insert(
            "TopHalf".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Up".to_string(),
            },
        );
        window_actions.insert(
            "BottomHalf".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Down".to_string(),
            },
        );

        window_actions.insert(
            "TopLeft".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "D1".to_string(),
            },
        );
        window_actions.insert(
            "TopRight".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "D2".to_string(),
            },
        );
        window_actions.insert(
            "BottomLeft".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "D3".to_string(),
            },
        );
        window_actions.insert(
            "BottomRight".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "D4".to_string(),
            },
        );

        window_actions.insert(
            "TopLeftSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Q".to_string(),
            },
        );
        window_actions.insert(
            "TopCenterSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "W".to_string(),
            },
        );
        window_actions.insert(
            "TopRightSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "E".to_string(),
            },
        );
        window_actions.insert(
            "BottomLeftSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "A".to_string(),
            },
        );
        window_actions.insert(
            "BottomCenterSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "S".to_string(),
            },
        );
        window_actions.insert(
            "BottomRightSixth".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "D".to_string(),
            },
        );

        window_actions.insert(
            "Maximize".to_string(),
            ConfigKey {
                modifiers: mods.clone(),
                key: "Enter".to_string(),
            },
        );
        window_actions.insert(
            "Center".to_string(),
            ConfigKey {
                modifiers: mods,
                key: "C".to_string(),
            },
        );

        Self {
            launcher_hotkey: ConfigKey {
                modifiers: vec!["Ctrl".to_string()],
                key: "Space".to_string(),
            },
            window_actions,
            launch_at_login: false,
        }
    }
}

impl Config {
    fn base_config_dir() -> PathBuf {
        let proj_dirs = ProjectDirs::from("", "", "mun").expect("Failed to get config directory");
        let config_dir = proj_dirs.config_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir).ok();
        config_dir
    }

    pub fn config_path() -> PathBuf {
        Self::base_config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<Config>(&data) {
                Ok(config) => {
                    log::info!("Loaded config from {}", path.display());
                    config
                }
                Err(e) => {
                    log::warn!("Failed to parse config ({}), using defaults", e);
                    let config = Config::default();
                    config.save();
                    config
                }
            }
        } else {
            log::info!("No config found, creating default at {}", path.display());
            let config = Self::default();
            config.save();
            config
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(data) = serde_json::to_string_pretty(self)
            && let Err(e) = std::fs::write(&path, &data)
        {
            log::error!("Failed to save config to {}: {}", path.display(), e);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LauncherHistory {
    pub usage: HashMap<String, HashMap<String, u32>>,
}

impl LauncherHistory {
    pub fn history_path() -> PathBuf {
        Config::base_config_dir().join("history.json")
    }

    pub fn load() -> Self {
        let path = Self::history_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn record(&mut self, query: &str, exec: &str) {
        let query = query.trim().to_lowercase();
        let exec_counts = self.usage.entry(query).or_default();
        let count = exec_counts.entry(exec.to_string()).or_insert(0);
        *count += 1;
        self.evict_if_needed();
        self.save_async();
    }

    pub fn get_score(&self, query: &str, exec: &str) -> u32 {
        let query = query.trim().to_lowercase();
        self.usage
            .get(&query)
            .and_then(|exec_counts| exec_counts.get(exec))
            .copied()
            .unwrap_or(0)
    }

    pub fn save_async(&self) {
        let path = Self::history_path();
        let data = serde_json::to_string_pretty(self).ok();
        std::thread::spawn(move || {
            if let Some(data) = data {
                std::fs::write(&path, data).ok();
            }
        });
    }

    pub fn top_execs_overall(&self, limit: usize) -> Vec<(String, u32)> {
        let mut totals: HashMap<String, u32> = HashMap::new();
        for execs in self.usage.values() {
            for (exec, count) in execs {
                *totals.entry(exec.clone()).or_insert(0) += count;
            }
        }
        let mut ranked: Vec<(String, u32)> = totals.into_iter().collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        ranked.truncate(limit);
        ranked
    }

    fn evict_if_needed(&mut self) {
        if self.usage.len() <= MAX_HISTORY_QUERIES {
            return;
        }

        let mut total_counts: Vec<(String, u32)> = self
            .usage
            .iter()
            .map(|(query, execs)| (query.clone(), execs.values().sum()))
            .collect();
        total_counts.sort_by_key(|(_, count)| *count);

        let to_remove = self.usage.len() - MAX_HISTORY_QUERIES;
        for (query, _) in total_counts.into_iter().take(to_remove) {
            self.usage.remove(&query);
        }
    }
}
