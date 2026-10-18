use crate::http;
use crate::paths;
use serde::Deserialize;
use sha2::Digest;

const LATEST: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";

#[derive(Deserialize)]
struct Artifact {
    version: String,
    download_url: String,
    checksums: Checksums,
}

#[derive(Deserialize)]
struct Checksums {
    sha256: String,
}

pub fn agent_dir() -> std::path::PathBuf {
    paths::root().join("agents")
}

pub async fn ensure_agent() -> Result<std::path::PathBuf, String> {
    paths::ensure()?;
    let dir = agent_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let art: Artifact = http::get_json(LATEST).await?;
    let target = dir.join(format!("authlib-injector-{}.jar", art.version));
    if target.exists() {
        if let Ok(data) = tokio::fs::read(&target).await {
            let mut h = sha2::Sha256::new();
            h.update(&data);
            if format!("{:x}", h.finalize()) == art.checksums.sha256 {
                return Ok(target);
            }
            let _ = tokio::fs::remove_file(&target).await;
        }
    }
    let data = http::get_bytes(&art.download_url)
        .await
        .map_err(|e| format!("No se pudo descargar authlib-injector: {e}"))?;
    let mut h = sha2::Sha256::new();
    h.update(&data);
    if format!("{:x}", h.finalize()) != art.checksums.sha256 {
        tokio::fs::remove_file(&target).await.ok();
        return Err(
            "El jar de authlib-injector descargado no coincide con su checksum".to_string(),
        );
    }
    tokio::fs::write(&target, &data)
        .await
        .map_err(|e| format!("No se pudo guardar el agente: {e}"))?;
    Ok(target)
}
