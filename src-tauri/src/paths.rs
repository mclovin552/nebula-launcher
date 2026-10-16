use std::path::PathBuf;

pub fn root() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nebula-launcher")
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
