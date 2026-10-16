use crate::paths;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub min_memory: u32,
    pub max_memory: u32,
    pub java_path: String,
    pub jvm_args: String,
    pub wrapper: String,
    pub ms_client_id: String,
    pub width: u32,
    pub height: u32,
    pub concurrency: usize,
    pub show_snapshots: bool,
    pub env_vars: String,
    pub pre_launch: String,
    pub post_exit: String,
    pub language: String,
    pub update_repo: String,
    pub close_to_tray: bool,
    pub auto_backup_minutes: u32,
    pub theme: String,
    pub discord_app_id: String,
}

pub const DEFAULT_MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

pub fn total_memory_mb() -> u64 {
    let Ok(text) = std::fs::read_to_string("/proc/meminfo") else {
        return 8192;
    };
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest
                .split_whitespace()
                .next()
                .and_then(|n| n.parse().ok())
                .unwrap_or(8_388_608);
            return kb / 1024;
        }
    }
    8192
}

impl Default for Settings {
    fn default() -> Self {
        let half = (total_memory_mb() / 2).clamp(2048, 8192) as u32;
        Settings {
            min_memory: 1024,
            max_memory: half / 256 * 256,
            java_path: String::new(),
            jvm_args: String::new(),
            wrapper: String::new(),
            ms_client_id: std::env::var("NEBULA_MS_CLIENT_ID")
                .unwrap_or_else(|_| DEFAULT_MS_CLIENT_ID.to_string()),
            width: 1280,
            height: 720,
            concurrency: 32,
            show_snapshots: false,
            env_vars: String::new(),
            pre_launch: String::new(),
            post_exit: String::new(),
            language: "es".to_string(),
            update_repo: String::new(),
            close_to_tray: true,
            auto_backup_minutes: 0,
            theme: "dark".to_string(),
            discord_app_id: std::env::var("NEBULA_DISCORD_APP_ID").unwrap_or_default(),
        }
    }
}

pub fn load() -> Settings {
    let path = paths::root().join("settings.json");
    match std::fs::read(&path) {
        Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
        Err(_) => Settings::default(),
    }
}

pub fn save(settings: &Settings) -> Result<(), String> {
    paths::ensure()?;
    let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(paths::root().join("settings.json"), data).map_err(|e| e.to_string())
}
