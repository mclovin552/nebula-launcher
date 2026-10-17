use std::path::PathBuf;
use std::sync::OnceLock;

fn portable_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    if dir.join("portable.txt").is_file() {
        Some(dir.join("nebula-data"))
    } else {
        None
    }
}

fn root_cached() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        if let Some(p) = portable_root() {
            return p;
        }
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nebula-launcher")
    })
    .clone()
}

pub fn root() -> PathBuf {
    root_cached()
}

pub fn is_portable() -> bool {
    portable_root().is_some()
}

pub fn versions() -> PathBuf {
    root().join("versions")
}

pub fn libraries() -> PathBuf {
    root().join("libraries")
}

pub fn assets() -> PathBuf {
    root().join("assets")
}

pub fn instances() -> PathBuf {
    root().join("instances")
}

pub fn runtimes() -> PathBuf {
    root().join("runtimes")
}

pub fn cache() -> PathBuf {
    root().join("cache")
}

pub fn ensure() -> Result<(), String> {
    for d in [
        root(),
        versions(),
        libraries(),
        assets(),
        instances(),
        runtimes(),
        cache(),
    ] {
        std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    }
    Ok(())
}
