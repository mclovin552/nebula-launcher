use crate::loaders;
use crate::modmeta;
use crate::mojang;
use crate::pack;
use crate::paths;
use crate::util::{now_millis, now_secs, safe_name, slug, Ctx};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use serde_json::Value;
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub name: String,
    pub address: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    #[serde(default)]
    pub loader_version: Option<String>,
    pub version_id: String,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub jvm_args: Option<String>,
    #[serde(default)]
    pub env_vars: Option<String>,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub playtime: u64,
    #[serde(default)]
    pub servers: Vec<Server>,
    pub created: u64,
    #[serde(default)]
    pub last_played: Option<u64>,
    #[serde(default)]
    pub accent: u8,
    #[serde(skip)]
    pub icon: Option<String>,
}

#[derive(Deserialize)]
pub struct Patch {
    pub name: String,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub jvm_args: Option<String>,
    pub env_vars: Option<String>,
    pub group: Option<String>,
    pub accent: Option<u8>,
}

#[derive(Serialize)]
pub struct ContentItem {
    pub name: String,
    pub enabled: bool,
    pub size: u64,
    pub is_dir: bool,
    pub title: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}

#[derive(Serialize)]
pub struct CrashInfo {
    pub report: Option<String>,
    pub log_tail: String,
}

pub fn dir(id: &str) -> PathBuf {
    paths::instances().join(id)
}

pub fn game_dir(id: &str) -> PathBuf {
    dir(id).join("minecraft")
}

pub fn save(inst: &Instance) -> Result<(), String> {
    std::fs::create_dir_all(dir(&inst.id)).ctx()?;
    let data = serde_json::to_vec_pretty(inst).ctx()?;
    std::fs::write(dir(&inst.id).join("instance.json"), data).ctx()
}

pub fn get(id: &str) -> Result<Instance, String> {
    if !safe_name(id) {
        return Err("Identificador de instancia inválido".to_string());
    }
    let data = std::fs::read(dir(id).join("instance.json"))
        .map_err(|_| "La instancia no existe".to_string())?;
    serde_json::from_slice(&data)
        .map(|mut inst: Instance| {
            inst.icon = load_icon_at(&dir(id));
            inst
        })
        .ctx()
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn load_icon_at(d: &Path) -> Option<String> {
    let bytes = std::fs::read(d.join("icon.png")).ok()?;
    if bytes.len() < 8 || bytes[..8] != PNG_MAGIC || bytes.len() > 1_000_000 {
        return None;
    }
    Some(format!("data:image/png;base64,{}", modmeta::b64(&bytes)))
}

pub fn icon_path(id: &str) -> PathBuf {
    dir(id).join("icon.png")
}

pub fn set_icon(id: &str, src: &Path) -> Result<(), String> {
    get(id)?;
    let bytes = std::fs::read(src).map_err(|_| "No se pudo leer el archivo".to_string())?;
    if bytes.len() > 1_000_000 {
        return Err("El icono debe pesar menos de 1 MB".to_string());
    }
    if bytes.len() < 8 || bytes[..8] != PNG_MAGIC {
        return Err("El icono debe ser un archivo PNG".to_string());
    }
    save_icon_bytes(id, &bytes)
}

pub fn remove_icon(id: &str) -> Result<(), String> {
    get(id)?;
    let _ = std::fs::remove_file(icon_path(id));
    Ok(())
}

fn save_icon_bytes(id: &str, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir(id)).ctx()?;
    std::fs::write(icon_path(id), bytes).ctx()
}

pub fn list() -> Result<Vec<Instance>, String> {
    paths::ensure()?;
    let mut out: Vec<Instance> = Vec::new();
    for e in std::fs::read_dir(paths::instances()).ctx()?.flatten() {
        if let Ok(data) = std::fs::read(e.path().join("instance.json")) {
            if let Ok(mut inst) = serde_json::from_slice::<Instance>(&data) {
                inst.icon = load_icon_at(&e.path());
                out.push(inst);
            }
        }
    }
    out.sort_by(|a, b| {
        let ka = a.last_played.unwrap_or(a.created);
        let kb = b.last_played.unwrap_or(b.created);
        kb.cmp(&ka)
    });
    Ok(out)
}

pub async fn create(
    app: Option<AppHandle>,
    name: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<Instance, String> {
    paths::ensure()?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Escribe un nombre para la instancia".to_string());
    }
    let manifest = mojang::manifest().await?;
    if !manifest.versions.iter().any(|v| v.id == mc_version) {
        return Err(format!("Minecraft {mc_version} no existe"));
    }
    let (version_id, loader_version) = match loader.as_str() {
        "vanilla" => (mc_version.clone(), None),
        "fabric" | "quilt" | "forge" | "neoforge" => {
            let (id, lv) = loaders::install_profile(app, &loader, &mc_version, loader_version).await?;
            (id, Some(lv))
        }
        other => return Err(format!("Cargador no soportado: {other}")),
    };
    let id = format!("{}-{}", slug(&name), now_millis() % 1_000_000);
    let inst = Instance {
        id: id.clone(),
        name,
        mc_version,
        loader,
        loader_version,
        version_id,
        memory_mb: None,
        java_path: None,
        jvm_args: None,
        env_vars: None,
        group: String::new(),
        playtime: 0,
        servers: Vec::new(),
        created: now_secs(),
        last_played: None,
        accent: (now_millis() % 6) as u8,
        icon: None,
    };
    for sub in ["mods", "resourcepacks", "shaderpacks", "saves"] {
        std::fs::create_dir_all(game_dir(&id).join(sub)).ctx()?;
    }
    save(&inst)?;
    Ok(inst)
}

pub fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).ctx()?;
    for e in std::fs::read_dir(src).ctx()?.flatten() {
        let from = e.path();
        let to = dst.join(e.file_name());
        let Ok(ft) = e.file_type() else {
            continue;
        };
        if ft.is_dir() {
            copy_dir(&from, &to)?;
        } else if ft.is_file() {
            std::fs::copy(&from, &to).ctx()?;
        }
    }
    Ok(())
}

pub async fn duplicate(id: &str) -> Result<Instance, String> {
    let source = get(id)?;
    let src_dir = dir(id);
    let mut inst = source.clone();
    inst.name = format!("{} (copia)", source.name);
    inst.id = format!("{}-{}", slug(&inst.name), now_millis() % 1_000_000);
    inst.created = now_secs();
    inst.last_played = None;
    inst.playtime = 0;
    let dst_dir = dir(&inst.id);
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        copy_dir(&src_dir, &dst_dir)?;
        let _ = std::fs::remove_dir_all(dst_dir.join("natives"));
        let _ = std::fs::remove_dir_all(dst_dir.join("backups"));
        if let Some(icon) = source.icon.as_deref() {
            let bytes = strip_data_url(icon);
            if !bytes.is_empty() {
                let _ = std::fs::write(dst_dir.join("icon.png"), &bytes);
            }
        }
        Ok(())
    })
    .await
    .ctx()??;
    save(&inst)?;
    Ok(inst)
}

fn strip_data_url(data_url: &str) -> Vec<u8> {
    let Some(rest) = data_url.strip_prefix("data:image/png;base64,") else {
        return Vec::new();
    };
    b64_decode(rest)
}

fn b64_decode(s: &str) -> Vec<u8> {
    let val = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => 0,
        }
    };
    let data: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut out = Vec::with_capacity(data.len() * 3 / 4);
    for chunk in data.chunks(4) {
        let n = chunk
            .iter()
            .fold(0u32, |acc, b| (acc << 6) | val(*b));
        let rem = chunk.len();
        if rem > 1 {
            out.push((n >> 16) as u8);
        }
        if rem > 2 {
            out.push((n >> 8) as u8);
        }
        if rem > 3 {
            out.push(n as u8);
        }
    }
    out
}

pub fn delete(id: &str) -> Result<(), String> {
    get(id)?;
    std::fs::remove_dir_all(dir(id)).ctx()
}

pub fn update(id: &str, patch: Patch) -> Result<Instance, String> {
    let mut inst = get(id)?;
    let name = patch.name.trim().to_string();
    if !name.is_empty() {
        inst.name = name;
    }
    inst.memory_mb = patch.memory_mb.filter(|m| *m > 0);
    inst.java_path = patch.java_path.filter(|s| !s.trim().is_empty());
    inst.jvm_args = patch.jvm_args.filter(|s| !s.trim().is_empty());
    inst.env_vars = patch.env_vars.filter(|s| !s.trim().is_empty());
    if let Some(g) = patch.group {
        inst.group = g.trim().chars().take(40).collect();
    }
    if let Some(a) = patch.accent {
        inst.accent = a % 6;
    }
    save(&inst)?;
    Ok(inst)
}

pub fn set_servers(id: &str, servers: Vec<Server>) -> Result<Instance, String> {
    let mut inst = get(id)?;
    inst.servers = servers
        .into_iter()
        .filter(|s| !s.name.trim().is_empty() && !s.address.trim().is_empty())
        .take(100)
        .map(|s| Server {
            name: s.name.trim().to_string(),
            address: s.address.trim().to_string(),
        })
        .collect();
    save(&inst)?;
    Ok(inst)
}

pub fn add_playtime(id: &str, seconds: u64) {
    if let Ok(mut inst) = get(id) {
        inst.playtime += seconds;
        let _ = save(&inst);
    }
}

fn content_dir(id: &str, kind: &str) -> Result<PathBuf, String> {
    match kind {
        "mods" | "resourcepacks" | "shaderpacks" | "saves" => Ok(game_dir(id).join(kind)),
        "backups" => Ok(dir(id).join("backups")),
        _ => Err("Tipo de contenido inválido".to_string()),
    }
}

pub fn list_content(id: &str, kind: &str) -> Result<Vec<ContentItem>, String> {
    get(id)?;
    let base = content_dir(id, kind)?;
    std::fs::create_dir_all(&base).ctx()?;
    let mut out = Vec::new();
    for e in std::fs::read_dir(&base).ctx()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if name.ends_with(".part") || name.starts_with('.') {
            continue;
        }
        let Ok(meta) = e.metadata() else {
            continue;
        };
        let info = if kind == "mods" && meta.is_file() {
            modmeta::read(&e.path())
        } else {
            None
        };
        out.push(ContentItem {
            enabled: !name.ends_with(".disabled"),
            name,
            size: if meta.is_file() { meta.len() } else { 0 },
            is_dir: meta.is_dir(),
            title: info.as_ref().and_then(|m| m.title.clone()),
            version: info.as_ref().and_then(|m| m.version.clone()),
            description: info.as_ref().and_then(|m| m.description.clone()),
            icon: info.and_then(|m| m.icon),
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn toggle_content(id: &str, kind: &str, name: &str) -> Result<(), String> {
    if !safe_name(name) {
        return Err("Nombre inválido".to_string());
    }
    let base = content_dir(id, kind)?;
    let from = base.join(name);
    let to = match name.strip_suffix(".disabled") {
        Some(original) => base.join(original),
        None => base.join(format!("{name}.disabled")),
    };
    std::fs::rename(from, to).ctx()
}

pub fn remove_content(id: &str, kind: &str, name: &str) -> Result<(), String> {
    if !safe_name(name) {
        return Err("Nombre inválido".to_string());
    }
    let target = content_dir(id, kind)?.join(name);
    if target.is_dir() {
        std::fs::remove_dir_all(target).ctx()
    } else {
        std::fs::remove_file(target).ctx()
    }
}

pub fn import_files(id: &str, kind: &str, files: Vec<String>) -> Result<usize, String> {
    get(id)?;
    let ext = match kind {
        "mods" => "jar",
        "resourcepacks" | "shaderpacks" => "zip",
        _ => return Err("No se pueden importar archivos en esta sección".to_string()),
    };
    let dest = content_dir(id, kind)?;
    std::fs::create_dir_all(&dest).ctx()?;
    let mut count = 0;
    for f in files {
        let src = PathBuf::from(&f);
        let Some(name) = src.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if !src.is_file() || !safe_name(&name) || !name.to_lowercase().ends_with(&format!(".{ext}")) {
            continue;
        }
        std::fs::copy(&src, dest.join(&name)).ctx()?;
        count += 1;
    }
    if count == 0 {
        return Err(format!("No se encontró ningún archivo .{ext} válido"));
    }
    Ok(count)
}

fn add_dir(zw: &mut zip::ZipWriter<std::fs::File>, base: &Path, cur: &Path) -> Result<(), String> {
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for e in std::fs::read_dir(cur).ctx()?.flatten() {
        let p = e.path();
        let rel = p.strip_prefix(base).ctx()?.to_string_lossy().replace('\\', "/");
        if p.is_dir() {
            zw.add_directory(format!("{rel}/"), opts).ctx()?;
            add_dir(zw, base, &p)?;
        } else if p.is_file() {
            zw.start_file(rel, opts).ctx()?;
            let mut f = std::fs::File::open(&p).ctx()?;
            std::io::copy(&mut f, zw).ctx()?;
        }
    }
    Ok(())
}

pub fn backup_worlds(id: &str) -> Result<String, String> {
    get(id)?;
    let saves = game_dir(id).join("saves");
    std::fs::create_dir_all(&saves).ctx()?;
    let backups = dir(id).join("backups");
    std::fs::create_dir_all(&backups).ctx()?;
    let name = format!("mundos-{}.zip", now_secs());
    let file = std::fs::File::create(backups.join(&name)).ctx()?;
    let mut zw = zip::ZipWriter::new(file);
    add_dir(&mut zw, &saves, &saves)?;
    zw.finish().ctx()?;
    Ok(name)
}

pub fn restore_backup(id: &str, name: &str) -> Result<(), String> {
    get(id)?;
    if !safe_name(name) {
        return Err("Nombre inválido".to_string());
    }
    let archive = dir(id).join("backups").join(name);
    let saves = game_dir(id).join("saves");
    std::fs::create_dir_all(&saves).ctx()?;
    pack::extract_prefixed(&archive, &[String::new()], &saves)
}

fn tail_bytes(bytes: &[u8], max: usize) -> String {
    let text = String::from_utf8_lossy(bytes).to_string();
    let start = text.len().saturating_sub(max);
    let start = (start..text.len())
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(0);
    text[start..].to_string()
}

pub fn crash_info(id: &str) -> Result<CrashInfo, String> {
    get(id)?;
    let game = game_dir(id);
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    if let Ok(rd) = std::fs::read_dir(game.join("crash-reports")) {
        for e in rd.flatten() {
            if let Ok(modified) = e.metadata().and_then(|m| m.modified()) {
                if newest.as_ref().map(|(t, _)| modified > *t).unwrap_or(true) {
                    newest = Some((modified, e.path()));
                }
            }
        }
    }
    let report = newest.and_then(|(_, p)| std::fs::read(p).ok()).map(|b| tail_bytes(&b, 200_000));
    let log_tail = std::fs::read(game.join("logs").join("latest.log"))
        .map(|b| {
            let text = tail_bytes(&b, 60_000);
            let lines: Vec<&str> = text.lines().collect();
            lines[lines.len().saturating_sub(150)..].join("\n")
        })
        .unwrap_or_default();
    Ok(CrashInfo { report, log_tail })
}

struct PrismPack {
    root: String,
    mc: String,
    loader: String,
    loader_version: Option<String>,
    name: String,
}

fn zip_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut e = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    e.read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub async fn import_multimc(app: Option<AppHandle>, path: PathBuf) -> Result<Instance, String> {
    let scan = path.clone();
    let pack = tokio::task::spawn_blocking(move || -> Result<PrismPack, String> {
        let f = std::fs::File::open(&scan).ctx()?;
        let mut zip = zip::ZipArchive::new(f).ctx()?;
        let mut best: Option<(usize, String)> = None;
        for i in 0..zip.len() {
            let name = match zip.by_index(i) {
                Ok(e) => e.name().to_string(),
                Err(_) => continue,
            };
            if let Some(prefix) = name.strip_suffix("mmc-pack.json") {
                let depth = prefix.matches('/').count();
                if best.as_ref().map(|(d, _)| depth < *d).unwrap_or(true) {
                    best = Some((depth, name));
                }
            }
        }
        let entry_name = best
            .ok_or("El .zip no parece una instancia de Prism/MultiMC (falta mmc-pack.json)")?
            .1;
        let root = entry_name.strip_suffix("mmc-pack.json").unwrap_or("").to_string();

        let index: Value = {
            let b = zip_entry(&mut zip, &entry_name).ok_or("mmc-pack.json no se puede leer")?;
            serde_json::from_slice(&b).ctx()?
        };

        let comps = index
            .get("components")
            .and_then(|c| c.as_array())
            .cloned()
            .unwrap_or_default();
        let find = |uid: &str| -> Option<String> {
            comps
                .iter()
                .find(|c| c.get("uid").and_then(|u| u.as_str()) == Some(uid))
                .and_then(|c| c.get("version").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        };
        let mc = find("net.minecraft").ok_or("La instancia no indica la versión de Minecraft")?;
        let (loader, loader_version) = find("net.fabricmc.fabric-loader")
            .map(|v| ("fabric".to_string(), Some(v)))
            .or_else(|| find("org.quiltmc.quilt-loader").map(|v| ("quilt".to_string(), Some(v))))
            .or_else(|| find("net.neoforged").map(|v| ("neoforge".to_string(), Some(v))))
            .or_else(|| find("net.minecraftforge").map(|v| ("forge".to_string(), Some(format!("{mc}-{v}")))))
            .unwrap_or(("vanilla".to_string(), None));

        let cfg_name = zip_entry(&mut zip, &format!("{root}instance.cfg"))
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|txt| {
                txt.lines()
                    .find(|l| l.starts_with("name="))
                    .map(|l| l[5..].trim().to_string())
            })
            .filter(|n| !n.is_empty());
        let name = cfg_name.unwrap_or_else(|| {
            root.trim_end_matches('/')
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or("Importada")
                .to_string()
        });
        Ok(PrismPack { root, mc, loader, loader_version, name })
    })
    .await
    .ctx()??;

    let inst = create(app.clone(), pack.name, pack.mc, pack.loader, pack.loader_version).await?;
    let game = game_dir(&inst.id);
    let prefixes = vec![
        format!("{}minecraft/", pack.root),
        format!("{}.minecraft/", pack.root),
    ];
    let scan_prefixes = prefixes.clone();
    let zpath = path.clone();
    let extracted = tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
        let f = std::fs::File::open(&zpath).ctx()?;
        let mut zip = zip::ZipArchive::new(f).ctx()?;
        let mut found: Option<String> = None;
        for i in 0..zip.len() {
            let name = zip.by_index(i).map(|e| e.name().to_string()).unwrap_or_default();
            for p in &scan_prefixes {
                if name.starts_with(p.as_str()) {
                    found = Some(name);
                    break;
                }
            }
            if found.is_some() {
                break;
            }
        }
        Ok(found)
    })
    .await
    .ctx()??;
    if extracted.is_none() {
        let _ = delete(&inst.id);
        return Err("El zip no contiene una carpeta .minecraft/ con contenido".to_string());
    }
    tokio::task::spawn_blocking(move || pack::extract_prefixed(&path, &prefixes, &game))
        .await
        .ctx()??;
    Ok(inst)
}
