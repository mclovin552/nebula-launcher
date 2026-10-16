use crate::http::{self, Job};
use crate::instances::Instance;
use crate::java;
use crate::mojang;
use crate::paths;
use crate::util::{emit_progress, Ctx};
use serde_json::Value;
use std::collections::HashSet;
use tauri::AppHandle;

const FORGE_META: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
const NEOFORGE_META: &str = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

fn meta_base(kind: &str) -> Result<&'static str, String> {
    match kind {
        "fabric" => Ok("https://meta.fabricmc.net/v2/versions/loader"),
        "quilt" => Ok("https://meta.quiltmc.org/v3/versions/loader"),
        _ => Err(format!("Cargador no soportado: {kind}")),
    }
}

pub fn parse_maven_versions(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in xml.split("<version>").skip(1) {
        if let Some(end) = part.find("</version>") {
            out.push(part[..end].trim().to_string());
        }
    }
    out.reverse();
    out
}

pub fn neoforge_prefix(mc: &str) -> Option<String> {
    let mut parts = mc.split('.');
    if parts.next()? != "1" {
        return None;
    }
    let minor: u32 = parts.next()?.parse().ok()?;
    let patch: u32 = parts.next().map(|p| p.parse().unwrap_or(0)).unwrap_or(0);
    Some(format!("{minor}.{patch}."))
}

pub async fn versions(kind: &str, mc: &str) -> Result<Vec<String>, String> {
    match kind {
        "fabric" | "quilt" => {
            let base = meta_base(kind)?;
            let list: Vec<Value> = http::get_json(&format!("{base}/{mc}")).await?;
            Ok(list
                .iter()
                .filter_map(|x| {
                    x.get("loader")
                        .and_then(|l| l.get("version"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect())
        }
        "forge" => {
            let bytes = http::get_bytes(FORGE_META).await?;
            let prefix = format!("{mc}-");
            Ok(parse_maven_versions(&String::from_utf8_lossy(&bytes))
                .into_iter()
                .filter(|v| v.starts_with(&prefix))
                .collect())
        }
        "neoforge" => {
            let Some(prefix) = neoforge_prefix(mc) else {
                return Ok(Vec::new());
            };
            let bytes = http::get_bytes(NEOFORGE_META).await?;
            Ok(parse_maven_versions(&String::from_utf8_lossy(&bytes))
                .into_iter()
                .filter(|v| v.starts_with(&prefix))
                .collect())
        }
        _ => Err(format!("Cargador no soportado: {kind}")),
    }
}

fn pick_default(all: &[String]) -> Option<String> {
    all.iter()
        .find(|v| !v.contains('-') && !v.contains("beta") && !v.contains("alpha") && !v.contains("pre"))
        .or_else(|| all.first())
        .cloned()
}

fn version_dirs() -> HashSet<String> {
    let mut out = HashSet::new();
    if let Ok(rd) = std::fs::read_dir(paths::versions()) {
        for e in rd.flatten() {
            out.insert(e.file_name().to_string_lossy().to_string());
        }
    }
    out
}

fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(n);
    lines[start..].join("\n")
}

async fn install_with_installer(
    app: Option<AppHandle>,
    kind: &str,
    mc: &str,
    chosen: String,
) -> Result<(String, String), String> {
    let (url, guess) = if kind == "forge" {
        let rest = chosen.strip_prefix(&format!("{mc}-")).unwrap_or(&chosen).to_string();
        (
            format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{chosen}/forge-{chosen}-installer.jar"),
            format!("{mc}-forge-{rest}"),
        )
    } else {
        (
            format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{chosen}/neoforge-{chosen}-installer.jar"),
            format!("neoforge-{chosen}"),
        )
    };
    let label = if kind == "forge" { "Instalando Forge" } else { "Instalando NeoForge" };
    if let Some(a) = &app {
        emit_progress(a, "loader", label, 0, 0);
    }
    let (vjson, jar_id) = mojang::resolve(mc).await?;
    let mut base_jobs: Vec<Job> = Vec::new();
    if let Some(j) = mojang::client_job(&vjson, &jar_id) {
        base_jobs.push(j);
    }
    http::download_all(base_jobs, 4, |_, _| {}).await?;
    let installer = paths::cache().join(format!("{kind}-{chosen}-installer.jar"));
    http::download(&Job {
        url,
        path: installer.clone(),
        sha1: None,
        size: 0,
    })
    .await?;
    let profiles = paths::root().join("launcher_profiles.json");
    if !profiles.exists() {
        tokio::fs::write(&profiles, br#"{"profiles":{},"settings":{},"version":3}"#)
            .await
            .ctx()?;
    }
    let before = version_dirs();
    let java_bin = java::ensure(mojang::java_major(&vjson), None, |_, _| {}).await?;
    let output = tokio::process::Command::new(&java_bin)
        .arg("-jar")
        .arg(&installer)
        .arg("--installClient")
        .arg(paths::root())
        .current_dir(paths::root())
        .output()
        .await
        .map_err(|e| format!("No se pudo ejecutar el instalador: {e}"))?;
    if !output.status.success() {
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let _ = tokio::fs::remove_file(&installer).await;
        return Err(format!("El instalador de {kind} falló:\n{}", tail(&text, 12)));
    }
    let after = version_dirs();
    let created: Vec<String> = after
        .difference(&before)
        .filter(|n| n.to_lowercase().contains("forge"))
        .cloned()
        .collect();
    let id = match created.first() {
        Some(n) => n.clone(),
        None => {
            let guess_json = paths::versions().join(&guess).join(format!("{guess}.json"));
            if guess_json.is_file() {
                guess
            } else {
                return Err(format!("El instalador terminó pero no se encontró la versión de {kind}"));
            }
        }
    };
    Ok((id, chosen))
}

pub async fn install_profile(
    app: Option<AppHandle>,
    kind: &str,
    mc: &str,
    loader_version: Option<String>,
) -> Result<(String, String), String> {
    let chosen = match loader_version.filter(|v| !v.is_empty()) {
        Some(v) => v,
        None => {
            let all = versions(kind, mc).await?;
            pick_default(&all).ok_or_else(|| format!("No hay {kind} disponible para Minecraft {mc}"))?
        }
    };
    if kind == "forge" || kind == "neoforge" {
        return install_with_installer(app, kind, mc, chosen).await;
    }
    let base = meta_base(kind)?;
    let bytes = http::get_bytes(&format!("{base}/{mc}/{chosen}/profile/json")).await?;
    let parsed: Value = serde_json::from_slice(&bytes).ctx()?;
    let id = parsed
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or("El perfil del cargador no tiene id")?
        .to_string();
    let dir = paths::versions().join(&id);
    tokio::fs::create_dir_all(&dir).await.ctx()?;
    tokio::fs::write(dir.join(format!("{id}.json")), &bytes).await.ctx()?;
    Ok((id, chosen))
}

pub async fn ensure(inst: &Instance, app: Option<AppHandle>) -> Result<String, String> {
    if inst.loader == "vanilla" {
        return Ok(inst.mc_version.clone());
    }
    let path = paths::versions()
        .join(&inst.version_id)
        .join(format!("{}.json", inst.version_id));
    if path.is_file() {
        return Ok(inst.version_id.clone());
    }
    let (id, _) = install_profile(app, &inst.loader, &inst.mc_version, inst.loader_version.clone()).await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_maven_metadata_newest_first() {
        let xml = "<metadata><versioning><latest>3</latest><versions><version>1</version><version>2</version><version>3</version></versions></versioning></metadata>";
        assert_eq!(parse_maven_versions(xml), vec!["3", "2", "1"]);
    }

    #[test]
    fn neoforge_prefix_follows_minecraft_version() {
        assert_eq!(neoforge_prefix("1.21.1").as_deref(), Some("21.1."));
        assert_eq!(neoforge_prefix("1.21").as_deref(), Some("21.0."));
        assert_eq!(neoforge_prefix("1.20.4").as_deref(), Some("20.4."));
        assert_eq!(neoforge_prefix("26.1"), None);
    }
}
