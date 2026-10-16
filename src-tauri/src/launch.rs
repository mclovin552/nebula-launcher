use crate::accounts;
use crate::http;
use crate::instances;
use crate::java;
use crate::loaders;
use crate::mojang;
use crate::paths;
use crate::settings;
use crate::util::{emit_progress, now_secs, split_args, Ctx};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Serialize, Clone)]
struct LogLine {
    instance: String,
    line: String,
}

#[derive(Serialize, Clone)]
struct ExitEvent {
    instance: String,
    code: Option<i32>,
}

fn collect(args: Option<&Value>, feats: &HashMap<&'static str, bool>) -> Vec<String> {
    let mut out = Vec::new();
    for a in args.and_then(|a| a.as_array()).into_iter().flatten() {
        match a {
            Value::String(s) => out.push(s.clone()),
            Value::Object(_) => {
                if !mojang::allowed(a.get("rules"), feats) {
                    continue;
                }
                match a.get("value") {
                    Some(Value::String(s)) => out.push(s.clone()),
                    Some(Value::Array(vs)) => {
                        for v in vs {
                            if let Some(s) = v.as_str() {
                                out.push(s.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

fn subst(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        out = out.replace(&format!("${{{}}}", k), v);
    }
    out
}

fn extract_natives(jars: Vec<PathBuf>, dest: PathBuf) -> Result<(), String> {
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).ctx()?;
    for jar in jars {
        let Ok(file) = std::fs::File::open(&jar) else {
            continue;
        };
        let Ok(mut zip) = zip::ZipArchive::new(file) else {
            continue;
        };
        for i in 0..zip.len() {
            let Ok(mut entry) = zip.by_index(i) else {
                continue;
            };
            if entry.is_dir() {
                continue;
            }
            let name = entry.name().to_string();
            if name.starts_with("META-INF") {
                continue;
            }
            let Some(fname) = Path::new(&name)
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
            else {
                continue;
            };
            if !fname.contains(".so") {
                continue;
            }
            if let Ok(mut out) = std::fs::File::create(dest.join(&fname)) {
                let _ = std::io::copy(&mut entry, &mut out);
            }
        }
    }
    Ok(())
}

pub fn parse_env(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            let key = k.trim();
            if !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                out.push((key.to_string(), v.trim().to_string()));
            }
        }
    }
    out
}

async fn run_shell(script: &str, cwd: &Path, env: &[(String, String)]) -> Result<(), String> {
    let mut c = tokio::process::Command::new("sh");
    c.arg("-c").arg(script).current_dir(cwd);
    for (k, v) in env {
        c.env(k, v);
    }
    let status = c.status().await.ctx()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("El comando terminó con {status}"))
    }
}

pub async fn run(
    app: AppHandle,
    running: Arc<Mutex<HashMap<String, u32>>>,
    instance_id: String,
    account_id: Option<String>,
    server: Option<String>,
) -> Result<(), String> {
    paths::ensure()?;
    http::reset_cancel();
    if running.lock().map(|r| r.contains_key(&instance_id)).unwrap_or(false) {
        return Err("La instancia ya está en ejecución".to_string());
    }
    let cfg = settings::load();
    let mut inst = instances::get(&instance_id)?;
    emit_progress(&app, "prepare", "Comprobando cuenta", 0, 0);
    let account = accounts::resolve_for_launch(account_id).await?;

    emit_progress(&app, "prepare", "Preparando versión", 0, 0);
    let version_id = loaders::ensure(&inst, Some(app.clone())).await?;
    let (vjson, jar_id) = mojang::resolve(&version_id).await?;
    let game_dir = instances::game_dir(&inst.id);
    tokio::fs::create_dir_all(&game_dir).await.ctx()?;

    let lib_plan = mojang::plan_libraries(&vjson);
    let mut jobs = lib_plan.jobs.clone();
    if let Some(j) = mojang::client_job(&vjson, &jar_id) {
        jobs.push(j);
    }
    let log_cfg = mojang::logging(&vjson);
    if let Some((j, _)) = &log_cfg {
        jobs.push(j.clone());
    }
    emit_progress(&app, "prepare", "Leyendo índice de recursos", 0, 0);
    let assets = mojang::plan_assets(&vjson).await?;
    jobs.extend(assets.jobs.clone());

    let app_dl = app.clone();
    http::download_all(jobs, cfg.concurrency, move |d, t| {
        emit_progress(&app_dl, "download", "Descargando archivos del juego", d, t)
    })
    .await?;

    let assets_root = {
        let linked = mojang::link_legacy_assets(&assets.index_id, &game_dir).await?;
        if linked == paths::assets() {
            paths::assets()
        } else {
            linked
        }
    };
    let game_assets = assets_root.clone();
    let assets_dir = paths::assets();

    let natives_dir = instances::dir(&inst.id).join("natives");
    let natives_jars = lib_plan.natives.clone();
    let natives_target = natives_dir.clone();
    tokio::task::spawn_blocking(move || extract_natives(natives_jars, natives_target))
        .await
        .ctx()??;

    let major = mojang::java_major(&vjson);
    emit_progress(&app, "java", "Preparando Java", 0, 0);
    let app_java = app.clone();
    let java_override = inst
        .java_path
        .clone()
        .filter(|p| !p.trim().is_empty())
        .or_else(|| Some(cfg.java_path.clone()));
    let java_bin = java::ensure(major, java_override, move |d, t| {
        emit_progress(&app_java, "java", &format!("Descargando Java {major}"), d, t)
    })
    .await?;

    let sep = ":";
    let mut cp: Vec<String> = lib_plan
        .classpath
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();
    cp.push(mojang::jar_path(&jar_id).to_string_lossy().to_string());
    let classpath = cp.join(sep);

    let max_mem = inst.memory_mb.unwrap_or(cfg.max_memory);
    let min_mem = cfg.min_memory.min(max_mem);
    let is_ms = account.kind == "microsoft";

    let mut vars: HashMap<&str, String> = HashMap::new();
    vars.insert("auth_player_name", account.name.clone());
    vars.insert("version_name", version_id.clone());
    vars.insert("game_directory", game_dir.to_string_lossy().to_string());
    vars.insert("assets_root", assets_dir.to_string_lossy().to_string());
    vars.insert("assets_index_name", assets.index_id.clone());
    vars.insert("auth_uuid", account.uuid.clone());
    vars.insert("auth_access_token", account.access_token.clone());
    vars.insert("auth_session", format!("token:{}:{}", account.access_token, account.uuid));
    vars.insert("clientid", "0".to_string());
    vars.insert("auth_xuid", "0".to_string());
    let user_type = if is_ms { "msa" } else { "legacy" };
    vars.insert("user_type", user_type.to_string());
    vars.insert("user_properties", "{}".to_string());
    vars.insert(
        "version_type",
        vjson.get("type").and_then(|t| t.as_str()).unwrap_or("release").to_string(),
    );
    vars.insert("resolution_width", cfg.width.to_string());
    vars.insert("resolution_height", cfg.height.to_string());
    vars.insert("natives_directory", natives_dir.to_string_lossy().to_string());
    vars.insert("launcher_name", "NebulaLauncher".to_string());
    vars.insert("launcher_version", "1.0.0".to_string());
    vars.insert("classpath", classpath);
    vars.insert("classpath_separator", sep.to_string());
    vars.insert("library_directory", paths::libraries().to_string_lossy().to_string());
    vars.insert("game_assets", game_assets.to_string_lossy().to_string());

    let server = server.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let supports_quick = vjson
        .get("arguments")
        .map(|a| a.to_string().contains("quickPlayMultiplayer"))
        .unwrap_or(false);
    let quick = server.is_some() && supports_quick;
    let feats = mojang::features_for(quick);
    if quick {
        let qp = game_dir.join("quickPlay");
        let _ = std::fs::create_dir_all(&qp);
        vars.insert("quickPlayPath", qp.join("log.json").to_string_lossy().to_string());
        vars.insert("quickPlayMultiplayer", server.clone().unwrap_or_default());
    }
    let mut jvm: Vec<String> = vec![format!("-Xms{min_mem}M"), format!("-Xmx{max_mem}M")];
    jvm.extend(split_args(&cfg.jvm_args));
    if let Some(extra) = &inst.jvm_args {
        jvm.extend(split_args(extra));
    }
    let mut game_args: Vec<String>;
    if let Some(args) = vjson.get("arguments") {
        jvm.extend(collect(args.get("jvm"), &feats));
        game_args = collect(args.get("game"), &feats);
    } else {
        jvm.push("-Djava.library.path=${natives_directory}".to_string());
        jvm.push("-Dminecraft.launcher.brand=${launcher_name}".to_string());
        jvm.push("-Dminecraft.launcher.version=${launcher_version}".to_string());
        jvm.push("-cp".to_string());
        jvm.push("${classpath}".to_string());
        game_args = vjson
            .get("minecraftArguments")
            .and_then(|a| a.as_str())
            .map(split_args)
            .unwrap_or_default();
    }
    if let Some((job, arg)) = &log_cfg {
        jvm.push(arg.replace("${path}", &job.path.to_string_lossy()));
    }
    let main_class = vjson
        .get("mainClass")
        .and_then(|m| m.as_str())
        .ok_or("La versión no define mainClass")?
        .to_string();

    let mut full: Vec<String> = Vec::new();
    full.extend(jvm.iter().map(|a| subst(a, &vars)));
    full.push(main_class);
    game_args = game_args.iter().map(|a| subst(a, &vars)).collect();
    full.extend(game_args);
    if let Some(addr) = &server {
        if !quick {
            let (host, port) = match addr.rsplit_once(':') {
                Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h.to_string(), p.to_string()),
                _ => (addr.clone(), "25565".to_string()),
            };
            full.push("--server".to_string());
            full.push(host);
            full.push("--port".to_string());
            full.push(port);
        }
    }

    let wrapper = split_args(&cfg.wrapper);
    let mut cmd = if let Some((first, rest)) = wrapper.split_first() {
        let mut c = tokio::process::Command::new(first);
        c.args(rest).arg(&java_bin);
        c
    } else {
        tokio::process::Command::new(&java_bin)
    };
    let env_text = format!("{}\n{}", cfg.env_vars, inst.env_vars.clone().unwrap_or_default());
    let env_pairs = parse_env(&env_text);
    for (k, v) in &env_pairs {
        cmd.env(k, v);
    }
    cmd.args(&full)
        .current_dir(&game_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    emit_progress(&app, "launch", "Iniciando Minecraft", 0, 0);
    if !cfg.pre_launch.trim().is_empty() {
        let _ = run_shell(&cfg.pre_launch, &game_dir, &env_pairs).await;
    }
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("No se pudo iniciar Java ({}): {e}", java_bin.display()))?;
    let pid = child.id().unwrap_or(0);
    if let Ok(mut r) = running.lock() {
        r.insert(instance_id.clone(), pid);
    }
    crate::discord::start(
        format!("Jugando a {}", inst.name),
        format!("Minecraft {} · Nebula Launcher", inst.mc_version),
    );
    inst.last_played = Some(now_secs());
    let _ = instances::save(&inst);

    if let Some(out) = child.stdout.take() {
        let app_out = app.clone();
        let id = instance_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(out).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_out.emit("game-log", LogLine { instance: id.clone(), line });
            }
        });
    }
    if let Some(err) = child.stderr.take() {
        let app_err = app.clone();
        let id = instance_id.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(err).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = app_err.emit("game-log", LogLine { instance: id.clone(), line });
            }
        });
    }
    let app_wait = app.clone();
    let id_wait = instance_id.clone();
    let post_exit = cfg.post_exit.clone();
    let post_dir = game_dir.clone();
    let post_env = env_pairs.clone();
    let started = std::time::Instant::now();
    tokio::spawn(async move {
        let status = child.wait().await;
        if let Ok(mut r) = running.lock() {
            r.remove(&id_wait);
        }
        crate::discord::stop();
        instances::add_playtime(&id_wait, started.elapsed().as_secs());
        if !post_exit.trim().is_empty() {
            let _ = run_shell(&post_exit, &post_dir, &post_env).await;
        }
        let code = status.ok().and_then(|s| s.code());
        let _ = app_wait.emit("game-exit", ExitEvent { instance: id_wait, code });
    });
    emit_progress(&app, "running", "Minecraft en ejecución", 1, 1);
    Ok(())
}

pub fn stop(running: &Arc<Mutex<HashMap<String, u32>>>, instance_id: &str) -> Result<(), String> {
    let pid = running
        .lock()
        .ok()
        .and_then(|r| r.get(instance_id).copied())
        .ok_or("La instancia no está en ejecución")?;
    std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()
        .ctx()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_parser_skips_invalid_lines() {
        let parsed = parse_env("# nota\nMESA_GLTHREAD=true\n\nmalo clave=1\n__GL_THREADED_OPTIMIZATIONS = 1\n");
        assert_eq!(
            parsed,
            vec![
                ("MESA_GLTHREAD".to_string(), "true".to_string()),
                ("__GL_THREADED_OPTIMIZATIONS".to_string(), "1".to_string())
            ]
        );
    }

    #[test]
    fn placeholders_are_replaced() {
        let mut vars: HashMap<&str, String> = HashMap::new();
        vars.insert("auth_player_name", "Steve".to_string());
        assert_eq!(subst("--username ${auth_player_name}", &vars), "--username Steve");
    }
}
