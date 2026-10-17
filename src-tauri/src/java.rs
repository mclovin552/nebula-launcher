use crate::http::{self, Job};
use crate::paths;
use crate::util::Ctx;
use std::path::{Path, PathBuf};

async fn major_of(bin: &Path) -> Option<u64> {
    let out = tokio::process::Command::new(bin)
        .arg("-version")
        .output()
        .await
        .ok()?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    let ver = &rest[..end];
    let mut parts = ver.split(['.', '-', '_', '+']);
    let first: u64 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn find_bin(dir: &Path) -> Option<PathBuf> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let direct = dir.join("bin").join(exe);
    if direct.is_file() {
        return Some(direct);
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let candidate = e.path().join("bin").join(exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn managed_dir(major: u64) -> PathBuf {
    paths::runtimes().join(format!("java-{major}"))
}

fn system_candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if cfg!(windows) {
        let bases = [
            r"C:\Program Files\Java",
            r"C:\Program Files (x86)\Java",
            r"C:\Program Files\Eclipse Adoptium",
            r"C:\Program Files\Microsoft",
        ];
        for base in bases {
            if let Ok(entries) = std::fs::read_dir(base) {
                for e in entries.flatten() {
                    let bin = e.path().join("bin").join("java.exe");
                    if bin.is_file() {
                        out.push(bin);
                    }
                }
            }
        }
        if let Ok(jh) = std::env::var("JAVA_HOME") {
            let bin = PathBuf::from(&jh).join("bin").join("java.exe");
            if bin.is_file() {
                out.push(bin);
            }
        }
    } else {
        for base in ["/usr/lib/jvm", "/usr/lib64/jvm", "/usr/java", "/opt/java"] {
            if let Ok(entries) = std::fs::read_dir(base) {
                for e in entries.flatten() {
                    let bin = e.path().join("bin").join("java");
                    if bin.is_file() {
                        out.push(bin);
                    }
                }
            }
        }
    }
    out.push(PathBuf::from(if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }));
    out
}

fn adoptium_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "x64"
    }
}

pub async fn ensure<F>(
    major: u64,
    override_path: Option<String>,
    progress: F,
) -> Result<PathBuf, String>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    if let Some(p) = override_path.filter(|p| !p.trim().is_empty()) {
        let path = PathBuf::from(p.trim());
        if path.is_file() {
            return Ok(path);
        }
        return Err(format!(
            "La ruta de Java configurada no existe: {}",
            path.display()
        ));
    }
    let managed = managed_dir(major);
    if let Some(bin) = find_bin(&managed) {
        return Ok(bin);
    }
    let mut fallback: Option<PathBuf> = None;
    for bin in system_candidates() {
        if let Some(m) = major_of(&bin).await {
            if m == major {
                return Ok(bin);
            }
            if major >= 17 && m > major && fallback.is_none() {
                fallback = Some(bin);
            }
        }
    }
    if let Some(bin) = fallback {
        return Ok(bin);
    }
    tokio::fs::create_dir_all(paths::cache()).await.ctx()?;
    let os = if cfg!(windows) { "windows" } else { "linux" };
    let (archive_name, is_zip) = if cfg!(windows) {
        (format!("java-{major}.zip"), true)
    } else {
        (format!("java-{major}.tar.gz"), false)
    };
    let archive = paths::cache().join(&archive_name);
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major}/ga/{os}/{}/jre/hotspot/normal/eclipse",
        adoptium_arch()
    );
    let _ = tokio::fs::remove_file(&archive).await;
    http::download_all(
        vec![Job {
            url,
            path: archive.clone(),
            sha1: None,
            size: 0,
        }],
        1,
        progress,
    )
    .await?;
    let dest = managed.clone();
    let archive_clone = archive.clone();
    let is_zip_clone = is_zip;
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let _ = std::fs::remove_dir_all(&dest);
        std::fs::create_dir_all(&dest).ctx()?;
        if is_zip_clone {
            let file = std::fs::File::open(&archive_clone).ctx()?;
            let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
            zip.extract(&dest).map_err(|e| e.to_string())
        } else {
            let file = std::fs::File::open(&archive_clone).ctx()?;
            let gz = flate2::read::GzDecoder::new(file);
            tar::Archive::new(gz).unpack(&dest).ctx()
        }
    })
    .await
    .ctx()??;
    let _ = tokio::fs::remove_file(&archive).await;
    find_bin(&managed).ok_or_else(|| format!("No se pudo instalar Java {major}"))
}
