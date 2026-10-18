#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod accounts;
mod agent;
mod discord;
mod http;
mod instances;
mod java;
mod launch;
mod loaders;
mod modmeta;
mod modrinth;
mod mojang;
mod pack;
mod paths;
mod serverping;
mod settings;
mod skins;
mod state;
mod util;
mod wardrobe;

use serde::Serialize;
use serde_json::{json, Value};
use state::AppState;
use tauri::{AppHandle, State};
use util::{emit_progress, Ctx};

#[derive(Serialize)]
struct VersionItem {
    id: String,
    kind: String,
    release_time: String,
}

#[derive(Serialize)]
struct VersionList {
    latest_release: String,
    latest_snapshot: String,
    versions: Vec<VersionItem>,
}

#[derive(Serialize)]
struct SystemInfo {
    total_memory_mb: u64,
    root: String,
}

#[tauri::command]
async fn versions() -> Result<VersionList, String> {
    let m = mojang::manifest().await?;
    Ok(VersionList {
        latest_release: m.latest.release,
        latest_snapshot: m.latest.snapshot,
        versions: m
            .versions
            .into_iter()
            .map(|e| VersionItem {
                id: e.id,
                kind: e.kind,
                release_time: e.release_time,
            })
            .collect(),
    })
}

#[tauri::command]
async fn loader_versions(kind: String, mc_version: String) -> Result<Vec<String>, String> {
    loaders::versions(&kind, &mc_version).await
}

#[tauri::command]
fn list_instances() -> Result<Vec<instances::Instance>, String> {
    instances::list()
}

#[tauri::command]
async fn create_instance(
    app: AppHandle,
    name: String,
    mc_version: String,
    loader: String,
    loader_version: Option<String>,
) -> Result<instances::Instance, String> {
    http::reset_cancel();
    instances::create(Some(app), name, mc_version, loader, loader_version).await
}

#[tauri::command]
async fn duplicate_instance(id: String) -> Result<instances::Instance, String> {
    instances::duplicate(&id).await
}

#[tauri::command]
fn set_servers(id: String, servers: Vec<instances::Server>) -> Result<instances::Instance, String> {
    instances::set_servers(&id, servers)
}

#[tauri::command]
async fn backup_worlds(id: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || instances::backup_worlds(&id))
        .await
        .ctx()?
}

#[tauri::command]
fn restore_backup(id: String, name: String) -> Result<(), String> {
    instances::restore_backup(&id, &name)
}

#[tauri::command]
async fn list_screenshots(id: String) -> Result<Vec<instances::ScreenItem>, String> {
    instances::list_screenshots(&id).await
}

#[tauri::command]
async fn screenshot_to_icon(id: String, name: String) -> Result<(), String> {
    instances::screenshot_to_icon(&id, &name).await
}

#[tauri::command]
async fn refresh_modpack(app: AppHandle, id: String) -> Result<(), String> {
    modrinth::refresh_modpack(app, &id).await
}

async fn java_major_for(id: String) -> Result<u64, String> {
    let inst = instances::get(&id)?;
    let vid = loaders::ensure(&inst, None).await?;
    let (v, _) = mojang::resolve(&vid).await?;
    Ok(mojang::java_major(&v))
}

#[tauri::command]
async fn java_needed(id: String) -> Result<u64, String> {
    java_major_for(id).await
}

#[tauri::command]
async fn java_status(id: String) -> Result<Value, String> {
    let major = java_major_for(id.clone()).await?;
    let inst = instances::get(&id)?;
    let override_path = inst
        .java_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| Some(settings::load().java_path.clone()));
    let ready = java::status(major, override_path).await;
    Ok(json!({ "major": major, "ready": ready }))
}

#[tauri::command]
async fn install_java(id: String) -> Result<String, String> {
    let major = java_major_for(id.clone()).await?;
    let inst = instances::get(&id)?;
    let override_path = inst
        .java_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| Some(settings::load().java_path.clone()));
    let bin = java::ensure(major, override_path, |_, _| {}).await?;
    Ok(bin.to_string_lossy().to_string())
}

#[tauri::command]
async fn java_scan() -> Result<Vec<Value>, String> {
    let list = java::scan().await;
    Ok(list
        .into_iter()
        .map(|(path, major)| json!({ "path": path, "major": major }))
        .collect())
}

#[tauri::command]
async fn reorder_instances(id: String, other: String) -> Result<(), String> {
    instances::reorder(&id, &other)
}

#[tauri::command]
async fn skin_lookup(name: String) -> Result<Value, String> {
    wardrobe::lookup_skin(&name).await
}

#[tauri::command]
fn app_info() -> Value {
    serde_json::json!({
        "name": "Nebula Launcher",
        "version": env!("CARGO_PKG_VERSION"),
        "repo": "https://github.com/mclovin552/nebula-launcher",
        "issues": "https://github.com/mclovin552/nebula-launcher/issues",
        "portable": paths::is_portable()
    })
}

#[tauri::command]
fn import_files(id: String, kind: String, paths: Vec<String>) -> Result<usize, String> {
    instances::import_files(&id, &kind, paths)
}

#[tauri::command]
fn crash_report(id: String) -> Result<instances::CrashInfo, String> {
    instances::crash_info(&id)
}

#[tauri::command]
async fn check_mod_updates(id: String) -> Result<Vec<modrinth::UpdateItem>, String> {
    modrinth::check_updates(&id).await
}

#[tauri::command]
async fn apply_mod_update(
    id: String,
    old_name: String,
    version_id: String,
) -> Result<String, String> {
    modrinth::apply_update(&id, &old_name, &version_id).await
}

#[tauri::command]
async fn export_modpack(id: String, dest: String) -> Result<(), String> {
    modrinth::export_mrpack(&id, dest).await
}

#[tauri::command]
async fn import_modpack_file(app: AppHandle, path: String) -> Result<instances::Instance, String> {
    http::reset_cancel();
    let concurrency = settings::load().concurrency;
    modrinth::install_from_pack(
        app,
        std::path::PathBuf::from(path),
        String::new(),
        concurrency,
    )
    .await
}

#[tauri::command]
async fn change_skin(account_id: String, path: String, variant: String) -> Result<(), String> {
    accounts::change_skin(&account_id, &path, &variant).await
}

#[tauri::command]
fn cancel_downloads() {
    http::request_cancel();
}

#[tauri::command]
async fn optimize_instance(app: AppHandle, id: String) -> Result<Vec<String>, String> {
    modrinth::optimize_instance(app, &id).await
}

#[tauri::command]
async fn project_details(project_id: String) -> Result<Value, String> {
    modrinth::project_details(&project_id).await
}

#[derive(Serialize)]
struct VerifyReport {
    checked: u64,
    repaired: u64,
}

#[tauri::command]
async fn verify_instance(app: AppHandle, id: String) -> Result<VerifyReport, String> {
    http::reset_cancel();
    let inst = instances::get(&id)?;
    emit_progress(&app, "verify", "Preparando versión", 0, 0);
    let version_id = loaders::ensure(&inst, Some(app.clone())).await?;
    let (vjson, jar_id) = mojang::resolve(&version_id).await?;
    let lib_plan = mojang::plan_libraries(&vjson);
    let mut jobs = lib_plan.jobs.clone();
    if let Some(j) = mojang::client_job(&vjson, &jar_id) {
        jobs.push(j);
    }
    if let Some((j, _)) = mojang::logging(&vjson) {
        jobs.push(j.clone());
    }
    emit_progress(&app, "verify", "Leyendo índice de recursos", 0, 0);
    let assets = mojang::plan_assets(&vjson).await?;
    jobs.extend(assets.jobs.clone());
    let total = jobs.len() as u64;
    let app2 = app.clone();
    let mut broken = Vec::new();
    for (i, j) in jobs.iter().enumerate() {
        if !http::check_job(j).await {
            broken.push(j.clone());
        }
        emit_progress(&app2, "verify", "Verificando archivos", i as u64 + 1, total);
    }
    let repaired = broken.len() as u64;
    if repaired > 0 {
        let app3 = app.clone();
        http::download_all(broken, settings::load().concurrency, move |d, t| {
            emit_progress(&app3, "verify", "Reparando archivos", d, t)
        })
        .await?;
    }
    Ok(VerifyReport {
        checked: total,
        repaired,
    })
}

#[tauri::command]
async fn ping_server(address: String) -> Result<serverping::ServerStatus, String> {
    Ok(serverping::status(&address).await)
}

#[tauri::command]
fn set_instance_icon(id: String, path: String) -> Result<(), String> {
    instances::set_icon(&id, &std::path::PathBuf::from(&path))
}

#[tauri::command]
fn remove_instance_icon(id: String) -> Result<(), String> {
    instances::remove_icon(&id)
}

#[tauri::command]
async fn import_instance_zip(app: AppHandle, path: String) -> Result<instances::Instance, String> {
    http::reset_cancel();
    instances::import_multimc(Some(app), std::path::PathBuf::from(path)).await
}

#[tauri::command]
async fn wardrobe_list() -> Result<Vec<wardrobe::SkinEntry>, String> {
    tokio::task::spawn_blocking(wardrobe::list_skins)
        .await
        .ctx()
}

#[tauri::command]
async fn wardrobe_add(source: String, name: String) -> Result<wardrobe::SkinEntry, String> {
    wardrobe::add_skin(source, name).await
}

#[tauri::command]
fn wardrobe_remove(id: String) -> Result<(), String> {
    wardrobe::remove_skin(&id)
}

#[tauri::command]
async fn wardrobe_apply(
    account_id: String,
    skin_id: String,
    variant: String,
) -> Result<(), String> {
    wardrobe::apply_saved_skin(&account_id, &skin_id, &variant).await
}

#[tauri::command]
async fn profile_items(account_id: String) -> Result<wardrobe::ProfileItems, String> {
    wardrobe::profile_items(&account_id).await
}

#[tauri::command]
async fn cape_set(account_id: String, cape_id: Option<String>) -> Result<(), String> {
    wardrobe::set_cape(&account_id, cape_id).await
}

#[tauri::command]
async fn offline_skin_apply(
    app: AppHandle,
    instance_id: String,
    player: String,
    skin_id: String,
) -> Result<(), String> {
    wardrobe::local_skin(app, &instance_id, &player, &skin_id).await
}

#[tauri::command]
async fn offline_cape_apply(
    app: AppHandle,
    instance_id: String,
    player: String,
    path: String,
) -> Result<(), String> {
    wardrobe::local_cape(app, &instance_id, &player, &path).await
}

#[tauri::command]
async fn check_mod_status(id: String) -> Result<modrinth::ModsStatus, String> {
    modrinth::mods_status(&id).await
}

#[tauri::command]
async fn self_update(app: AppHandle) -> Result<String, String> {
    if !cfg!(windows) && !cfg!(target_os = "macos") {
        return Err("La actualización con un clic funciona en Windows y macOS. En Linux, descarga el AppImage nuevo desde Releases.".to_string());
    }
    let repo = settings::load().update_repo;
    let repo = repo.trim().to_string();
    let repo = if repo.is_empty() {
        "mclovin552/nebula-launcher".to_string()
    } else {
        repo
    };
    let repo_valid = repo.split('/').count() == 2
        && repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '.');
    if !repo_valid {
        return Err("Repositorio de actualizaciones inválido (usa usuario/repo)".to_string());
    }
    let v: Value = http::get_json(&format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ))
    .await?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if tag.is_empty() || tag == env!("CARGO_PKG_VERSION") {
        return Err("Ya estás en la última versión".to_string());
    }
    let asset = v
        .get("assets")
        .and_then(|a| a.as_array())
        .and_then(|a| {
            a.iter().find(|x| {
                x.get("name")
                    .and_then(|n| n.as_str())
                    .map(|n| {
                        if cfg!(windows) {
                            n.ends_with("_x64-setup.exe")
                        } else {
                            n.ends_with(".dmg")
                        }
                    })
                    .unwrap_or(false)
            })
        })
        .ok_or("No hay instalador disponible todavía")?;
    let url = asset
        .get("browser_download_url")
        .and_then(|u| u.as_str())
        .ok_or("URL inválida")?;
    if cfg!(windows) {
        let bytes = http::get_bytes(url).await?;
        let tmp =
            std::env::temp_dir().join(format!("nebula-setup-{}.exe", env!("CARGO_PKG_VERSION")));
        tokio::fs::write(&tmp, &bytes)
            .await
            .map_err(|e| e.to_string())?;
        let mut cmd = std::process::Command::new(&tmp);
        cmd.arg("/S");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.spawn().map_err(|e| e.to_string())?;
        app.exit(0);
        return Ok("Instalando la nueva versión…".to_string());
    }
    let tmp = std::env::temp_dir().join(format!(
        "Nebula-{}-installer.dmg",
        env!("CARGO_PKG_VERSION")
    ));
    let _ = tokio::fs::remove_file(&tmp).await;
    http::download(&http::Job {
        url: url.to_string(),
        path: tmp.clone(),
        sha1: None,
        size: 0,
    })
    .await?;
    std::process::Command::new("open")
        .arg(&tmp)
        .spawn()
        .map_err(|e| e.to_string())?;
    app.exit(0);
    Ok("Descargué el .dmg: arrastrala a Aplicaciones para actualizar".to_string())
}

#[derive(Serialize)]
struct UpdateInfo {
    available: bool,
    version: String,
    url: String,
}

#[tauri::command]
async fn skin_gallery(query: String, mode: String, page: i64) -> Result<Vec<Value>, String> {
    if mode == "populares" || mode == "newest" || mode == "top_viewed" {
        let items = crate::skins::crafty_list(&mode).await?;
        return Ok(items
            .into_iter()
            .map(|it| {
                json!({
                    "title": it.title,
                    "preview": format!("{}?width=170&height=250&x=-30&z=50", it.render),
                    "full": format!("crafty:{}:{}", it.uuid, it.model),
                    "model": it.model,
                    "crafty": true,
                })
            })
            .collect());
    }
    let items = crate::skins::skindex_search(&query, page).await?;
    if !items.is_empty() {
        return Ok(items
            .into_iter()
            .map(|it| {
                json!({
                    "title": it.title,
                    "preview": it.preview,
                    "full": it.full,
                    "model": "classic",
                    "crafty": false,
                })
            })
            .collect());
    }
    if let Some(ely) = crate::skins::ely_skin(&query).await {
        return Ok(vec![json!({
            "title": format!("{} — skin pública de Ely.by", ely.0),
            "preview": ely.1,
            "full": ely.1,
            "model": "classic",
            "crafty": false,
        })]);
    }
    Ok(Vec::new())
}

#[tauri::command]
async fn skin_preview(file: String) -> Result<String, String> {
    if file.contains("..") || file.contains('/') {
        return Err("Archivo no válido".to_string());
    }
    let path = crate::skins::skinroot().join("historia").join(&file);
    if !path.exists() {
        return Err("Esa skin no existe".to_string());
    }
    let raw = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("No se pudo leer la skin: {e}"))?;
    use base64::Engine as _;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(raw)
    ))
}

#[tauri::command]
async fn skin_history_remove(file: String) -> Result<(), String> {
    if file.contains("..") || file.contains('/') {
        return Err("Archivo no válido".to_string());
    }
    let root = crate::skins::skinroot();
    let _ = tokio::fs::remove_file(root.join("historia").join(&file)).await;
    let mut v = tokio::fs::read_to_string(root.join("historia.json"))
        .await
        .ok()
        .and_then(|txt| serde_json::from_str::<Vec<serde_json::Value>>(&txt).ok())
        .unwrap_or_default();
    v.retain(|it| it.get("file").and_then(|x| x.as_str()) != Some(file.as_str()));
    tokio::fs::write(root.join("historia.json"), json!(v).to_string())
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn skin_history() -> Vec<crate::skins::HistoryItem> {
    crate::skins::history().await
}

#[tauri::command]
async fn skin_history_apply(account_id: String, file: String) -> Result<String, String> {
    let hist = crate::skins::skinroot().join("historia");
    if file.contains("..") || file.contains('/') {
        return Err("Archivo no válido".to_string());
    }
    let path = hist.join(&file);
    if !path.exists() {
        return Err("Esa skin ya no está en el historial".to_string());
    }
    skin_apply(account_id, path.to_string_lossy().to_string()).await
}

#[tauri::command]
async fn skin_profile(account_id: String) -> Result<Value, String> {
    let acc = crate::accounts::resolve_for_launch(Some(account_id)).await?;
    let crafty = format!(
        "https://render.crafty.gg/3d/full/{}",
        crate::util::url_param(&acc.name)
    );
    let local = crate::skins::current_skin_path().exists();
    Ok(json!({
        "kind": acc.kind,
        "name": acc.name,
        "crafty": crafty,
        "local_skin": local,
    }))
}

#[tauri::command]
async fn skin_apply(account_id: String, source: String) -> Result<String, String> {
    let acc = crate::accounts::resolve_for_launch(Some(account_id)).await?;
    let source2: String;
    let model: String;
    if let Some(trimmed) = source.strip_prefix("crafty:") {
        let rest: Vec<&str> = trimmed.split(':').collect();
        source2 = format!("crafty:{}", rest.first().unwrap_or(&""));
        model = rest
            .get(1)
            .map(|m| m.to_string())
            .unwrap_or_else(|| "classic".to_string());
    } else {
        source2 = source;
        model = "classic".to_string();
    }
    let bytes = crate::skins::download_skin(&source2).await?;
    if !crate::skins::valid_png(&bytes) {
        return Err("Ese enlace no devuelve una skin PNG válida".to_string());
    }
    crate::skins::record_history(&bytes).await;
    if acc.kind == "microsoft" {
        let entry =
            crate::wardrobe::add_skin_from_bytes(&bytes, format!("{}-skin", acc.name)).await?;
        crate::wardrobe::apply_saved_skin(&acc.id, &entry.id, &model).await?;
        Ok("Skin subida a tu cuenta premium: entra al juego y la verás".to_string())
    } else {
        crate::skins::copy_for_local_skin_mods(&acc.name, &bytes).await;
        let dest = crate::skins::set_current_skin(&bytes).await?;
        Ok(format!(
            "Skin guardada como tu skin local ({}). Al pulsar Jugar se inyecta en el juego.",
            dest.to_string_lossy()
        ))
    }
}

#[tauri::command]
async fn check_launcher_update() -> Result<UpdateInfo, String> {
    let repo = settings::load().update_repo;
    let repo = repo.trim().to_string();
    let repo = if repo.is_empty() {
        "mclovin552/nebula-launcher".to_string()
    } else {
        repo
    };
    let valid = repo.split('/').count() == 2
        && repo
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '.');
    if !valid {
        return Ok(UpdateInfo {
            available: false,
            version: String::new(),
            url: String::new(),
        });
    }
    let v: Value = http::get_json(&format!(
        "https://api.github.com/repos/{repo}/releases/latest"
    ))
    .await?;
    let tag = v
        .get("tag_name")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    let url = v
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or("")
        .to_string();
    let newer = !tag.is_empty() && version_newer(&tag, env!("CARGO_PKG_VERSION"));
    Ok(UpdateInfo {
        available: newer,
        version: tag,
        url,
    })
}

fn version_newer(candidate: &str, current: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .map(|p| p.parse().unwrap_or(0))
            .collect()
    };
    parse(candidate) > parse(current)
}

#[tauri::command]
fn delete_instance(id: String) -> Result<(), String> {
    instances::delete(&id)
}

#[tauri::command]
fn update_instance(id: String, patch: instances::Patch) -> Result<instances::Instance, String> {
    instances::update(&id, patch)
}

#[tauri::command]
async fn list_content(id: String, kind: String) -> Result<Vec<instances::ContentItem>, String> {
    tokio::task::spawn_blocking(move || instances::list_content(&id, &kind))
        .await
        .ctx()?
}

#[tauri::command]
fn toggle_content(id: String, kind: String, name: String) -> Result<(), String> {
    instances::toggle_content(&id, &kind, &name)
}

#[tauri::command]
fn remove_content(id: String, kind: String, name: String) -> Result<(), String> {
    instances::remove_content(&id, &kind, &name)
}

#[tauri::command]
fn open_folder(id: Option<String>, sub: Option<String>) -> Result<(), String> {
    let mut path = match &id {
        Some(i) => {
            instances::get(i)?;
            instances::game_dir(i)
        }
        None => paths::root(),
    };
    if let Some(s) = sub.filter(|s| util::safe_name(s)) {
        path = path.join(s);
    }
    std::fs::create_dir_all(&path).ctx()?;
    let opener = if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("No se pudo abrir el explorador de archivos: {e}"))?;
    Ok(())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Solo se pueden abrir enlaces https".to_string());
    }
    if cfg!(windows) {
        std::process::Command::new("explorer")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el navegador: {e}"))?;
    } else {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("No se pudo abrir el navegador: {e}"))?;
    }
    Ok(())
}

#[tauri::command]
fn accounts_list() -> accounts::AccountList {
    accounts::list()
}

#[tauri::command]
fn add_offline(name: String) -> Result<accounts::AccountView, String> {
    accounts::add_offline(&name)
}

#[tauri::command]
fn remove_account(id: String) -> Result<(), String> {
    accounts::remove(&id)
}

#[tauri::command]
fn select_account(id: String) -> Result<(), String> {
    accounts::select(&id)
}

#[tauri::command]
async fn ms_begin() -> Result<accounts::OAuthStart, String> {
    accounts::cancel_local_login();
    accounts::begin().await
}

#[tauri::command]
async fn ms_finish() -> Result<accounts::AccountView, String> {
    accounts::finish().await
}

#[tauri::command]
fn ms_cancel() {
    accounts::cancel_local_login();
}

#[tauri::command]
fn settings_get() -> settings::Settings {
    settings::load()
}

#[tauri::command]
fn settings_set(settings: settings::Settings) -> Result<(), String> {
    settings::save(&settings)
}

#[tauri::command]
fn system_info() -> SystemInfo {
    SystemInfo {
        total_memory_mb: settings::total_memory_mb(),
        root: paths::root().to_string_lossy().to_string(),
    }
}

#[tauri::command]
async fn launch(
    app: AppHandle,
    state: State<'_, AppState>,
    instance_id: String,
    account_id: Option<String>,
    server: Option<String>,
) -> Result<(), String> {
    let running = state.running.clone();
    launch::run(app, running, instance_id, account_id, server).await
}

#[tauri::command]
fn stop(state: State<'_, AppState>, instance_id: String) -> Result<(), String> {
    launch::stop(&state.running, &instance_id)
}

#[tauri::command]
fn running(state: State<'_, AppState>) -> Vec<String> {
    state
        .running
        .lock()
        .map(|r| r.keys().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
async fn search(
    kind: String,
    query: String,
    mc_version: Option<String>,
    loader: Option<String>,
    sort: String,
    offset: u32,
) -> Result<Value, String> {
    modrinth::search(&kind, &query, mc_version, loader, &sort, offset).await
}

#[tauri::command]
async fn install_project(
    kind: String,
    project_id: String,
    instance_id: String,
) -> Result<Vec<String>, String> {
    modrinth::install_project(&kind, &project_id, &instance_id).await
}

#[tauri::command]
async fn install_modpack(
    app: AppHandle,
    project_id: String,
    title: String,
    version_id: Option<String>,
) -> Result<instances::Instance, String> {
    let concurrency = settings::load().concurrency;
    modrinth::install_modpack(app, project_id, title, version_id, concurrency).await
}

#[tauri::command]
async fn modpack_versions(project_id: String) -> Result<Vec<Value>, String> {
    modrinth::modpack_versions(&project_id).await
}

#[tauri::command]
async fn patch_notes() -> Result<Vec<Value>, String> {
    let v: Value =
        http::get_json("https://launchercontent.mojang.com/v2/javaPatchNotes.json").await?;
    let entries = v
        .get("entries")
        .and_then(|e| e.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(entries
        .into_iter()
        .take(8)
        .map(|e| {
            let image = e.pointer("/image/url").and_then(|u| u.as_str()).map(|u| {
                if u.starts_with("http") {
                    u.to_string()
                } else if u.starts_with("/v2/") {
                    format!("https://launchercontent.mojang.com{u}")
                } else {
                    format!("https://launchercontent.mojang.com/v2{u}")
                }
            });
            json!({
                "title": e.get("title"),
                "version": e.get("version"),
                "kind": e.get("type"),
                "date": e.get("date"),
                "summary": e.get("shortText"),
                "image": image
            })
        })
        .collect())
}

fn main() {
    #[cfg(target_os = "linux")]
    std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    let _ = paths::ensure();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            use tauri::menu::{Menu, MenuItem};
            if let (Ok(menu), Some(icon)) = (
                Menu::with_items(
                    app,
                    &[
                        &MenuItem::with_id(app, "show", "Mostrar Nebula", true, None::<&str>)?,
                        &MenuItem::with_id(app, "quit", "Salir", true, None::<&str>)?,
                    ],
                ),
                app.default_window_icon(),
            ) {
                let _ = tauri::tray::TrayIconBuilder::with_id("nebula-tray")
                    .menu(&menu)
                    .tooltip("Nebula Launcher")
                    .icon(icon.clone())
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => {
                            use tauri::Manager;
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.unminimize();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click { .. } = event {
                            use tauri::Manager;
                            if let Some(w) = tray.app_handle().get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app);
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if settings::load().close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            versions,
            loader_versions,
            list_instances,
            create_instance,
            duplicate_instance,
            delete_instance,
            update_instance,
            set_servers,
            backup_worlds,
            restore_backup,
            list_screenshots,
            screenshot_to_icon,
            refresh_modpack,
            java_needed,
            java_status,
            install_java,
            java_scan,
            skin_gallery,
            skin_profile,
            skin_history,
            skin_history_apply,
            skin_history_remove,
            skin_preview,
            skin_apply,
            app_info,
            reorder_instances,
            skin_lookup,
            import_files,
            crash_report,
            check_mod_updates,
            check_mod_status,
            self_update,
            apply_mod_update,
            export_modpack,
            import_modpack_file,
            change_skin,
            cancel_downloads,
            check_launcher_update,
            list_content,
            toggle_content,
            remove_content,
            open_folder,
            open_url,
            accounts_list,
            add_offline,
            remove_account,
            select_account,
            ms_begin,
            ms_finish,
            ms_cancel,
            settings_get,
            settings_set,
            system_info,
            launch,
            stop,
            running,
            search,
            install_project,
            install_modpack,
            modpack_versions,
            patch_notes,
            optimize_instance,
            project_details,
            verify_instance,
            ping_server,
            set_instance_icon,
            remove_instance_icon,
            import_instance_zip,
            wardrobe_list,
            wardrobe_add,
            wardrobe_remove,
            wardrobe_apply,
            profile_items,
            cape_set,
            offline_skin_apply,
            offline_cape_apply
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar Nebula Launcher");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison_is_numeric() {
        assert!(version_newer("1.10.0", "1.9.9"));
        assert!(version_newer("2.0", "1.9.9"));
        assert!(!version_newer("1.0.0", "1.0.0"));
        assert!(!version_newer("0.9.0", "1.0.0"));
    }
}
