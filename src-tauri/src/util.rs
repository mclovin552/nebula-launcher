use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

pub trait Ctx<T> {
    fn ctx(self) -> Result<T, String>;
}

impl<T, E: std::fmt::Display> Ctx<T> for Result<T, E> {
    fn ctx(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let t = out.trim_matches('-').to_string();
    if t.is_empty() {
        "instancia".to_string()
    } else {
        t
    }
}

pub fn safe_rel(p: &str) -> Option<PathBuf> {
    let path = Path::new(p);
    if path.is_absolute() {
        return None;
    }
    for c in path.components() {
        match c {
            Component::Normal(_) => {}
            _ => return None,
        }
    }
    Some(path.to_path_buf())
}

pub fn safe_name(n: &str) -> bool {
    !n.is_empty() && !n.contains('/') && !n.contains('\\') && n != ".." && n != "."
}

pub fn split_args(s: &str) -> Vec<String> {
    s.split_whitespace().map(|x| x.to_string()).collect()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Serialize, Clone)]
pub struct Progress {
    pub stage: String,
    pub label: String,
    pub done: u64,
    pub total: u64,
}

pub fn emit_progress(app: &AppHandle, stage: &str, label: &str, done: u64, total: u64) {
    let _ = app.emit(
        "progress",
        Progress {
            stage: stage.to_string(),
            label: label.to_string(),
            done,
            total,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_rel_rejects_escapes() {
        assert!(safe_rel("mods/a.jar").is_some());
        assert!(safe_rel("../etc/passwd").is_none());
        assert!(safe_rel("/etc/passwd").is_none());
        assert!(safe_rel("a/../../b").is_none());
    }

    #[test]
    fn slug_is_filesystem_friendly() {
        assert_eq!(slug("Mi Mundo Épico!"), "mi-mundo-pico");
        assert_eq!(slug("!!!"), "instancia");
    }

    #[test]
    fn safe_name_blocks_paths() {
        assert!(safe_name("a.jar"));
        assert!(!safe_name("../a.jar"));
        assert!(!safe_name("a/b"));
        assert!(!safe_name(""));
    }
}
