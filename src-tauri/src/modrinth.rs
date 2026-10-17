use crate::http::{self, client, Job};
use crate::instances;
use crate::loaders;
use crate::pack;
use crate::paths;
use crate::util::{emit_progress, hex, safe_name, safe_rel, Ctx};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha512};
use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;

const API: &str = "https://api.modrinth.com/v2";

#[derive(Serialize)]
pub struct UpdateItem {
    pub file: String,
    pub new_name: String,
    pub new_version: String,
    pub version_id: String,
}

fn text(v: &Value, k: &str) -> String {
    v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

pub fn loader_list(loader: &str) -> Vec<String> {
    if loader == "quilt" {
        vec!["quilt".to_string(), "fabric".to_string()]
    } else {
        vec![loader.to_string()]
    }
}

pub async fn search(
    kind: &str,
    query: &str,
    mc: Option<String>,
    loader: Option<String>,
    sort: &str,
    offset: u32,
) -> Result<Value, String> {
    if !["mod", "modpack", "resourcepack", "shader", "datapack"].contains(&kind) {
        return Err("Tipo de proyecto inválido".to_string());
    }
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{kind}")]];
    if let Some(v) = mc.filter(|s| !s.is_empty()) {
        facets.push(vec![format!("versions:{v}")]);
    }
    if kind == "mod" || kind == "modpack" {
        if let Some(l) = loader.filter(|s| !s.is_empty() && s != "vanilla") {
            facets.push(
                loader_list(&l)
                    .iter()
                    .map(|x| format!("categories:{x}"))
                    .collect(),
            );
        }
    }
    let facets_json = serde_json::to_string(&facets).ctx()?;
    let off = offset.to_string();
    let resp = client()
        .get(format!("{API}/search"))
        .query(&[
            ("query", query),
            ("facets", facets_json.as_str()),
            ("limit", "24"),
            ("offset", off.as_str()),
            ("index", sort),
        ])
        .timeout(Duration::from_secs(40))
        .send()
        .await
        .ctx()?
        .error_for_status()
        .ctx()?;
    let body: Value = resp.json().await.ctx()?;
    let hits = body
        .get("hits")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();
    let items: Vec<Value> = hits
        .iter()
        .map(|h| {
            json!({
                "id": text(h, "project_id"),
                "title": text(h, "title"),
                "description": text(h, "description"),
                "icon": text(h, "icon_url"),
                "author": text(h, "author"),
                "downloads": h.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
                "source": "modrinth"
            })
        })
        .collect();
    Ok(json!({
        "items": items,
        "total": body.get("total_hits").and_then(|t| t.as_u64()).unwrap_or(0)
    }))
}

async fn fetch_versions(
    project: &str,
    mc: Option<&str>,
    loaders: Option<Vec<String>>,
) -> Result<Vec<Value>, String> {
    let mut q: Vec<(String, String)> = Vec::new();
    if let Some(m) = mc {
        q.push((
            "game_versions".to_string(),
            serde_json::to_string(&[m]).ctx()?,
        ));
    }
    if let Some(l) = loaders {
        q.push(("loaders".to_string(), serde_json::to_string(&l).ctx()?));
    }
    let resp = client()
        .get(format!("{API}/project/{project}/version"))
        .query(&q)
        .timeout(Duration::from_secs(40))
        .send()
        .await
        .ctx()?
        .error_for_status()
        .ctx()?;
    resp.json::<Vec<Value>>().await.ctx()
}

fn best(versions: Vec<Value>) -> Option<Value> {
    let release = versions
        .iter()
        .find(|v| v.get("version_type").and_then(|t| t.as_str()) == Some("release"))
        .cloned();
    release.or_else(|| versions.into_iter().next())
}

async fn version_by_id(id: &str) -> Result<Value, String> {
    http::get_json(&format!("{API}/version/{id}")).await
}

fn primary_job(version: &Value, dest: PathBuf) -> Result<(Job, String), String> {
    let files = version
        .get("files")
        .and_then(|f| f.as_array())
        .ok_or("La versión no tiene archivos")?;
    let file = files
        .iter()
        .find(|f| f.get("primary").and_then(|p| p.as_bool()) == Some(true))
        .or_else(|| files.first())
        .ok_or("La versión no tiene archivos")?;
    let filename = text(file, "filename");
    if !safe_name(&filename) {
        return Err("Nombre de archivo inseguro".to_string());
    }
    let job = Job {
        url: text(file, "url"),
        path: dest.join(&filename),
        sha1: file
            .get("hashes")
            .and_then(|h| h.get("sha1"))
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        size: file.get("size").and_then(|s| s.as_u64()).unwrap_or(0),
    };
    Ok((job, filename))
}

async fn resolve_version(
    project: &str,
    mc: &str,
    loaders: Option<Vec<String>>,
    fallback_any: bool,
) -> Result<Value, String> {
    let mut list = fetch_versions(project, Some(mc), loaders).await?;
    if list.is_empty() && fallback_any {
        list = fetch_versions(project, None, None).await?;
    }
    best(list).ok_or_else(|| "No hay una versión compatible con esta instancia".to_string())
}

pub async fn install_project(
    kind: &str,
    project_id: &str,
    instance_id: &str,
) -> Result<Vec<String>, String> {
    http::reset_cancel();
    let inst = instances::get(instance_id)?;
    let (folder, use_loader) = match kind {
        "mod" => ("mods", true),
        "resourcepack" => ("resourcepacks", false),
        "shader" => ("shaderpacks", false),
        _ => return Err("Este tipo de contenido no se instala en una instancia".to_string()),
    };
    if use_loader && inst.loader == "vanilla" {
        return Err("Esta instancia es vanilla. Crea una con Fabric, Quilt, Forge o NeoForge para usar mods.".to_string());
    }
    let dest = instances::game_dir(instance_id).join(folder);
    tokio::fs::create_dir_all(&dest).await.ctx()?;
    let loaders = if use_loader {
        Some(loader_list(&inst.loader))
    } else {
        None
    };
    let mut queue: Vec<(String, Option<String>)> = vec![(project_id.to_string(), None)];
    let mut visited: HashSet<String> = HashSet::new();
    let mut installed: Vec<String> = Vec::new();
    let mut first = true;
    while let Some((pid, vid)) = queue.pop() {
        if installed.len() >= 30 || !visited.insert(pid.clone()) {
            continue;
        }
        let version = match vid {
            Some(v) => version_by_id(&v).await,
            None => resolve_version(&pid, &inst.mc_version, loaders.clone(), !use_loader).await,
        };
        let version = match version {
            Ok(v) => v,
            Err(e) => {
                if first {
                    return Err(e);
                }
                continue;
            }
        };
        first = false;
        let (job, filename) = primary_job(&version, dest.clone())?;
        http::download(&job).await?;
        installed.push(filename);
        if use_loader {
            if let Some(deps) = version.get("dependencies").and_then(|d| d.as_array()) {
                for d in deps {
                    if d.get("dependency_type").and_then(|t| t.as_str()) != Some("required") {
                        continue;
                    }
                    let Some(dep_pid) = d.get("project_id").and_then(|p| p.as_str()) else {
                        continue;
                    };
                    let dep_vid = d
                        .get("version_id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    queue.push((dep_pid.to_string(), dep_vid));
                }
            }
        }
    }
    Ok(installed)
}

const PERFORMANCE_FABRIC: &[&str] = &[
    "sodium",
    "lithium",
    "ferrite-core",
    "entityculling",
    "immediatelyfast",
    "modernfix",
    "krypton",
];
const PERFORMANCE_FORGE: &[&str] = &[
    "embeddium",
    "canary",
    "ferrite-core",
    "modernfix",
    "immediatelyfast",
    "entityculling",
];

pub async fn optimize_instance(app: AppHandle, instance_id: &str) -> Result<Vec<String>, String> {
    http::reset_cancel();
    let inst = instances::get(instance_id)?;
    let slugs = match inst.loader.as_str() {
        "fabric" | "quilt" => PERFORMANCE_FABRIC,
        "forge" | "neoforge" => PERFORMANCE_FORGE,
        "vanilla" => {
            return Err(
                "Esta instancia es vanilla: crea una con Fabric, Quilt, Forge o NeoForge para optimizarla"
                    .to_string(),
            )
        }
        other => return Err(format!("Cargador no soportado: {other}")),
    };
    let mut installed: Vec<String> = Vec::new();
    for (i, slug) in slugs.iter().enumerate() {
        emit_progress(
            &app,
            "modpack",
            "Optimizando instancia",
            i as u64,
            slugs.len() as u64,
        );
        if let Ok(files) = install_project("mod", slug, instance_id).await {
            installed.extend(files);
        }
    }
    if installed.is_empty() {
        return Err(
            "No se encontraron mods de rendimiento compatibles con esta versión".to_string(),
        );
    }
    emit_progress(&app, "modpack", "Instancia optimizada", 1, 1);
    Ok(installed)
}

pub async fn project_details(project_id: &str) -> Result<Value, String> {
    let v: Value = http::get_json(&format!("{API}/project/{project_id}")).await?;
    let gallery = v
        .get("gallery")
        .and_then(|g| g.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|g| g.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()))
        .collect::<Vec<String>>();
    let categories = v
        .get("categories")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();
    Ok(json!({
        "id": text(&v, "id"),
        "slug": text(&v, "slug"),
        "title": text(&v, "title"),
        "description": text(&v, "description"),
        "body": text(&v, "body"),
        "icon": text(&v, "icon_url"),
        "downloads": v.get("downloads").and_then(|d| d.as_u64()).unwrap_or(0),
        "follows": v.get("followers").and_then(|d| d.as_u64()).unwrap_or(0),
        "categories": categories,
        "gallery": gallery,
    }))
}

async fn status_core(instance_id: &str) -> Result<(String, String, Vec<(String, String)>), String> {
    let inst = instances::get(instance_id)?;
    let dir = instances::game_dir(instance_id).join("mods");
    let entries = tokio::task::spawn_blocking(move || -> Vec<(String, String)> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().to_string();
                if !name.ends_with(".jar") {
                    continue;
                }
                if let Some(h) = http::sha1_of_file(&e.path()) {
                    out.push((name, h));
                }
            }
        }
        out
    })
    .await
    .ctx()?;
    Ok((inst.loader.clone(), inst.mc_version.clone(), entries))
}

async fn version_map(
    loader: &str,
    mc_version: &str,
    entries: &[(String, String)],
) -> Result<Value, String> {
    if entries.is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    let hashes: Vec<String> = entries.iter().map(|e| e.1.clone()).collect();
    let body = json!({
        "hashes": hashes,
        "algorithm": "sha1",
        "loaders": loader_list(loader),
        "game_versions": [mc_version]
    });
    let resp = client()
        .post(format!("{API}/version_files/update"))
        .json(&body)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .ctx()?
        .error_for_status()
        .ctx()?;
    resp.json().await.ctx()
}

pub async fn check_updates(instance_id: &str) -> Result<Vec<UpdateItem>, String> {
    let (loader, mc_version, entries) = status_core(instance_id).await?;
    if loader == "vanilla" || entries.is_empty() {
        return Ok(Vec::new());
    }
    let map = version_map(&loader, &mc_version, &entries).await?;
    let mut out = Vec::new();
    for (name, hash) in entries {
        let Some(v) = map.get(hash.as_str()) else {
            continue;
        };
        let Some(files) = v.get("files").and_then(|f| f.as_array()) else {
            continue;
        };
        let Some(file) = files
            .iter()
            .find(|f| f.get("primary").and_then(|p| p.as_bool()) == Some(true))
            .or_else(|| files.first())
        else {
            continue;
        };
        let new_hash = file
            .get("hashes")
            .and_then(|h| h.get("sha1"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        if new_hash.is_empty() || new_hash.eq_ignore_ascii_case(&hash) {
            continue;
        }
        out.push(UpdateItem {
            file: name,
            new_name: text(file, "filename"),
            new_version: text(v, "version_number"),
            version_id: text(v, "id"),
        });
    }
    Ok(out)
}

#[derive(Serialize)]
pub struct ModStatusItem {
    pub file: String,
    pub state: String,
    pub new_version: String,
    pub version_id: String,
}

#[derive(Serialize)]
pub struct ModsStatus {
    pub items: Vec<ModStatusItem>,
    pub ok: usize,
    pub updated: usize,
    pub unknown: usize,
}

pub async fn mods_status(instance_id: &str) -> Result<ModsStatus, String> {
    let (loader, mc_version, entries) = status_core(instance_id).await?;
    if entries.is_empty() {
        return Ok(ModsStatus {
            items: Vec::new(),
            ok: 0,
            updated: 0,
            unknown: 0,
        });
    }
    let map = if loader == "vanilla" {
        Value::Object(Default::default())
    } else {
        version_map(&loader, &mc_version, &entries).await?
    };
    let mut items = Vec::new();
    let mut ok = 0usize;
    let mut updated = 0usize;
    let mut unknown = 0usize;
    for (name, hash) in entries {
        let Some(v) = map.get(hash.as_str()) else {
            unknown += 1;
            items.push(ModStatusItem {
                file: name,
                state: "unknown".into(),
                new_version: String::new(),
                version_id: String::new(),
            });
            continue;
        };
        let files = v.get("files").and_then(|f| f.as_array());
        let new_hash = files
            .and_then(|f| f.first())
            .and_then(|f| f.get("hashes"))
            .and_then(|h| h.get("sha1"))
            .and_then(|s| s.as_str())
            .unwrap_or("");
        let has_new = !new_hash.is_empty() && !new_hash.eq_ignore_ascii_case(&hash);
        if has_new {
            updated += 1;
            items.push(ModStatusItem {
                file: name,
                state: "update".into(),
                new_version: text(v, "version_number"),
                version_id: text(v, "id"),
            });
        } else {
            ok += 1;
            items.push(ModStatusItem {
                file: name,
                state: "ok".into(),
                new_version: String::new(),
                version_id: String::new(),
            });
        }
    }
    Ok(ModsStatus {
        items,
        ok,
        updated,
        unknown,
    })
}

pub async fn apply_update(
    instance_id: &str,
    old_name: &str,
    version_id: &str,
) -> Result<String, String> {
    http::reset_cancel();
    instances::get(instance_id)?;
    if !safe_name(old_name) {
        return Err("Nombre inválido".to_string());
    }
    let version = version_by_id(version_id).await?;
    let dest = instances::game_dir(instance_id).join("mods");
    let (job, filename) = primary_job(&version, dest.clone())?;
    http::download(&job).await?;
    if filename != old_name {
        let _ = tokio::fs::remove_file(dest.join(old_name)).await;
    }
    Ok(filename)
}

fn host_allowed(url: &str) -> bool {
    [
        "https://cdn.modrinth.com/",
        "https://github.com/",
        "https://raw.githubusercontent.com/",
        "https://gitlab.com/",
        "https://cdn.technicpack.net/",
        "https://technic.s3.amazonaws.com/",
        "https://s3.amazonaws.com/",
    ]
    .iter()
    .any(|p| url.starts_with(p))
}

pub async fn install_modpack(
    app: AppHandle,
    project_id: String,
    title: String,
    concurrency: usize,
) -> Result<instances::Instance, String> {
    http::reset_cancel();
    emit_progress(&app, "modpack", "Buscando versión del modpack", 0, 0);
    let versions = fetch_versions(&project_id, None, None).await?;
    let version = best(versions).ok_or("El modpack no tiene versiones")?;
    paths::ensure()?;
    let (job, _) = primary_job(&version, paths::cache())?;
    emit_progress(&app, "modpack", "Descargando modpack", 0, 1);
    http::download(&job).await?;
    install_from_pack(app, job.path, title, concurrency).await
}

pub async fn install_from_pack(
    app: AppHandle,
    pack_path: PathBuf,
    title: String,
    concurrency: usize,
) -> Result<instances::Instance, String> {
    let index = read_mrpack_index(pack_path.clone()).await?;

    let deps = index.get("dependencies").cloned().unwrap_or(Value::Null);
    let mc = deps
        .get("minecraft")
        .and_then(|m| m.as_str())
        .ok_or("El modpack no indica versión de Minecraft")?
        .to_string();
    let dep = |k: &str| deps.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let (loader, loader_version) = if let Some(v) = dep("fabric-loader") {
        ("fabric".to_string(), Some(v))
    } else if let Some(v) = dep("quilt-loader") {
        ("quilt".to_string(), Some(v))
    } else if let Some(v) = dep("neoforge") {
        ("neoforge".to_string(), Some(v))
    } else if let Some(v) = dep("forge") {
        ("forge".to_string(), Some(format!("{mc}-{v}")))
    } else {
        ("vanilla".to_string(), None)
    };

    emit_progress(&app, "modpack", "Creando instancia", 0, 0);
    let name = if title.trim().is_empty() {
        text(&index, "name")
    } else {
        title
    };
    let inst = instances::create(Some(app.clone()), name, mc, loader, loader_version).await?;
    let result = finish_modpack(&app, &inst, &index, pack_path, concurrency).await;
    if let Err(e) = result {
        let _ = instances::delete(&inst.id);
        return Err(e);
    }
    let _ = loaders::ensure(&inst, None).await;
    emit_progress(&app, "modpack", "Modpack instalado", 1, 1);
    Ok(inst)
}

async fn read_mrpack_index(pack_path: PathBuf) -> Result<Value, String> {
    tokio::task::spawn_blocking(move || -> Result<Value, String> {
        let file = std::fs::File::open(&pack_path).ctx()?;
        let mut zip = zip::ZipArchive::new(file).ctx()?;
        let mut entry = zip
            .by_name("modrinth.index.json")
            .map_err(|_| "El archivo no es un modpack de Modrinth (.mrpack)".to_string())?;
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut entry, &mut buf).ctx()?;
        serde_json::from_str(&buf).ctx()
    })
    .await
    .ctx()?
}

async fn finish_modpack(
    app: &AppHandle,
    inst: &instances::Instance,
    index: &Value,
    pack_path: PathBuf,
    concurrency: usize,
) -> Result<(), String> {
    let game = instances::game_dir(&inst.id);
    let mut jobs: Vec<Job> = Vec::new();
    if let Some(files) = index.get("files").and_then(|f| f.as_array()) {
        for f in files {
            let client_env = f
                .get("env")
                .and_then(|e| e.get("client"))
                .and_then(|c| c.as_str())
                .unwrap_or("required");
            if client_env == "unsupported" {
                continue;
            }
            let Some(rel) = f.get("path").and_then(|p| p.as_str()).and_then(safe_rel) else {
                continue;
            };
            let Some(url) = f.get("downloads").and_then(|d| d.as_array()).and_then(|d| {
                d.iter()
                    .filter_map(|u| u.as_str())
                    .find(|u| host_allowed(u))
            }) else {
                continue;
            };
            jobs.push(Job {
                url: url.to_string(),
                path: game.join(rel),
                sha1: f
                    .get("hashes")
                    .and_then(|h| h.get("sha1"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string()),
                size: f.get("fileSize").and_then(|s| s.as_u64()).unwrap_or(0),
            });
        }
    }
    let app_progress = app.clone();
    http::download_all(jobs, concurrency, move |d, t| {
        emit_progress(
            &app_progress,
            "modpack",
            "Descargando contenido del modpack",
            d,
            t,
        )
    })
    .await?;
    let prefixes = vec!["overrides/".to_string(), "client-overrides/".to_string()];
    tokio::task::spawn_blocking(move || pack::extract_prefixed(&pack_path, &prefixes, &game))
        .await
        .ctx()??;
    Ok(())
}

struct Exported {
    rel: String,
    path: PathBuf,
    sha1: String,
    sha512: String,
    size: u64,
    client_only: bool,
}

pub async fn export_mrpack(instance_id: &str, dest: String) -> Result<(), String> {
    let inst = instances::get(instance_id)?;
    let game = instances::game_dir(instance_id);
    let game_scan = game.clone();
    let files: Vec<Exported> = tokio::task::spawn_blocking(move || -> Vec<Exported> {
        let mut out = Vec::new();
        for folder in ["mods", "resourcepacks", "shaderpacks"] {
            let base = game_scan.join(folder);
            if let Ok(rd) = std::fs::read_dir(&base) {
                for e in rd.flatten() {
                    let p = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    if !p.is_file() || name.ends_with(".disabled") || name.ends_with(".part") {
                        continue;
                    }
                    let Ok(bytes) = std::fs::read(&p) else {
                        continue;
                    };
                    let mut h = Sha512::new();
                    h.update(&bytes);
                    out.push(Exported {
                        rel: format!("{folder}/{name}"),
                        path: p,
                        sha1: http::sha1_of_bytes(&bytes),
                        sha512: hex(&h.finalize()),
                        size: bytes.len() as u64,
                        client_only: folder != "mods",
                    });
                }
            }
        }
        out
    })
    .await
    .ctx()?;

    let mut urls: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if !files.is_empty() {
        let hashes: Vec<String> = files.iter().map(|f| f.sha1.clone()).collect();
        let resp = client()
            .post(format!("{API}/version_files"))
            .json(&json!({ "hashes": hashes, "algorithm": "sha1" }))
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .ctx()?;
        if resp.status().is_success() {
            let map: Value = resp.json().await.ctx()?;
            if let Some(obj) = map.as_object() {
                for (hash, version) in obj {
                    let found = version
                        .get("files")
                        .and_then(|f| f.as_array())
                        .and_then(|fs| {
                            fs.iter().find(|f| {
                                f.get("hashes")
                                    .and_then(|h| h.get("sha1"))
                                    .and_then(|s| s.as_str())
                                    == Some(hash.as_str())
                            })
                        })
                        .and_then(|f| f.get("url"))
                        .and_then(|u| u.as_str());
                    if let Some(u) = found {
                        urls.insert(hash.clone(), u.to_string());
                    }
                }
            }
        }
    }

    let mut index_files: Vec<Value> = Vec::new();
    let mut overrides: Vec<(String, PathBuf)> = Vec::new();
    for f in &files {
        let server_env = if f.client_only {
            "unsupported"
        } else {
            "required"
        };
        match urls.get(&f.sha1) {
            Some(u) => index_files.push(json!({
                "path": f.rel,
                "hashes": { "sha1": f.sha1, "sha512": f.sha512 },
                "env": { "client": "required", "server": server_env },
                "downloads": [u],
                "fileSize": f.size
            })),
            None => overrides.push((format!("overrides/{}", f.rel), f.path.clone())),
        }
    }
    let config_dir = game.join("config");
    for p in pack::collect_files(&config_dir) {
        if let Ok(rel) = p.strip_prefix(&game) {
            overrides.push((
                format!("overrides/{}", rel.to_string_lossy().replace('\\', "/")),
                p.clone(),
            ));
        }
    }
    let options = game.join("options.txt");
    if options.is_file() {
        overrides.push(("overrides/options.txt".to_string(), options));
    }

    let mut dependencies = serde_json::Map::new();
    dependencies.insert(
        "minecraft".to_string(),
        Value::String(inst.mc_version.clone()),
    );
    let lv = inst.loader_version.clone().unwrap_or_default();
    match inst.loader.as_str() {
        "fabric" => {
            dependencies.insert("fabric-loader".to_string(), Value::String(lv));
        }
        "quilt" => {
            dependencies.insert("quilt-loader".to_string(), Value::String(lv));
        }
        "neoforge" => {
            dependencies.insert("neoforge".to_string(), Value::String(lv));
        }
        "forge" => {
            let short = lv
                .strip_prefix(&format!("{}-", inst.mc_version))
                .unwrap_or(&lv)
                .to_string();
            dependencies.insert("forge".to_string(), Value::String(short));
        }
        _ => {}
    }
    let index = json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": "1.0.0",
        "name": inst.name,
        "files": index_files,
        "dependencies": Value::Object(dependencies)
    });
    let index_bytes = serde_json::to_vec_pretty(&index).ctx()?;
    let dest_path = if dest.to_lowercase().ends_with(".mrpack") {
        dest
    } else {
        format!("{dest}.mrpack")
    };
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::create(&dest_path).ctx()?;
        let mut zw = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("modrinth.index.json", opts).ctx()?;
        zw.write_all(&index_bytes).ctx()?;
        for (name, path) in overrides {
            zw.start_file(name, opts).ctx()?;
            let mut f = std::fs::File::open(&path).ctx()?;
            std::io::copy(&mut f, &mut zw).ctx()?;
        }
        zw.finish().ctx()?;
        Ok(())
    })
    .await
    .ctx()??;
    Ok(())
}

pub async fn refresh_modpack(app: AppHandle, instance_id: &str) -> Result<(), String> {
    http::reset_cancel();
    let inst = instances::get(instance_id)?;
    let pid = inst
        .pack_project
        .clone()
        .ok_or("Esta instancia no guarda el modpack de Modrinth de origen")?;
    emit_progress(&app, "modpack", "Buscando versión nueva del modpack", 0, 0);
    let versions = fetch_versions(&pid, None, None).await?;
    let version = best(versions).ok_or("El modpack no tiene versiones")?;
    paths::ensure()?;
    let (job, _) = primary_job(&version, paths::cache())?;
    emit_progress(&app, "modpack", "Descargando modpack", 0, 1);
    http::download(&job).await?;
    let index = read_mrpack_index(job.path.clone()).await?;
    let concurrency = crate::settings::load().concurrency;
    emit_progress(&app, "modpack", "Descargando contenido del modpack", 0, 0);
    finish_modpack(&app, &inst, &index, job.path, concurrency).await?;
    let _ = loaders::ensure(&inst, None).await;
    emit_progress(&app, "modpack", "Modpack actualizado", 1, 1);
    Ok(())
}
