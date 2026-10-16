use crate::http::{self, Job};
use crate::paths;
use crate::util::Ctx;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const MANIFEST_URL: &str = "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json";
const RESOURCES_URL: &str = "https://resources.download.minecraft.net";
const LIBRARIES_URL: &str = "https://libraries.minecraft.net/";

#[derive(Serialize, Deserialize, Clone)]
pub struct Latest {
    pub release: String,
    pub snapshot: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    #[serde(default)]
    pub sha1: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Manifest {
    pub latest: Latest,
    pub versions: Vec<Entry>,
}

pub async fn manifest() -> Result<Manifest, String> {
    let cache = paths::cache().join("version_manifest_v2.json");
    match http::get_json::<Manifest>(MANIFEST_URL).await {
        Ok(m) => {
            if let Ok(bytes) = serde_json::to_vec(&m) {
                let _ = tokio::fs::create_dir_all(paths::cache()).await;
                let _ = tokio::fs::write(&cache, bytes).await;
            }
            Ok(m)
        }
        Err(e) => {
            let data = tokio::fs::read(&cache).await.map_err(|_| e.clone())?;
            serde_json::from_slice(&data).map_err(|_| e)
        }
    }
}

pub async fn version_json(id: &str) -> Result<Value, String> {
    let path = paths::versions().join(id).join(format!("{id}.json"));
    if let Ok(data) = tokio::fs::read(&path).await {
        if let Ok(v) = serde_json::from_slice::<Value>(&data) {
            return Ok(v);
        }
    }
    let m = manifest().await?;
    let entry = m
        .versions
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("La versión {id} no existe en el manifiesto de Mojang"))?;
    let bytes = http::get_bytes(&entry.url).await?;
    if !entry.sha1.is_empty() && !http::sha1_of_bytes(&bytes).eq_ignore_ascii_case(&entry.sha1) {
        return Err(format!("El JSON de la versión {id} no coincide con su SHA-1"));
    }
    let v: Value = serde_json::from_slice(&bytes).ctx()?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.ctx()?;
    }
    tokio::fs::write(&path, &bytes).await.ctx()?;
    Ok(v)
}

fn merge(base: Value, child: Value) -> Value {
    let mut base_map = match base {
        Value::Object(m) => m,
        _ => return child,
    };
    let child_map = match child {
        Value::Object(m) => m,
        _ => return Value::Object(base_map),
    };
    for (k, v) in child_map {
        match k.as_str() {
            "inheritsFrom" => {}
            "libraries" => {
                let mut libs = match v {
                    Value::Array(a) => a,
                    _ => Vec::new(),
                };
                if let Some(Value::Array(p)) = base_map.get("libraries") {
                    libs.extend(p.iter().cloned());
                }
                base_map.insert(k.clone(), Value::Array(libs));
            }
            "arguments" => {
                let mut out: Map<String, Value> = match base_map.get("arguments") {
                    Some(Value::Object(o)) => o.clone(),
                    _ => Map::new(),
                };
                if let Value::Object(co) = &v {
                    for key in ["game", "jvm"] {
                        let mut arr = match out.get(key) {
                            Some(Value::Array(a)) => a.clone(),
                            _ => Vec::new(),
                        };
                        if let Some(Value::Array(ca)) = co.get(key) {
                            arr.extend(ca.iter().cloned());
                        }
                        out.insert(key.to_string(), Value::Array(arr));
                    }
                }
                base_map.insert(k.clone(), Value::Object(out));
            }
            _ => {
                base_map.insert(k.clone(), v);
            }
        }
    }
    Value::Object(base_map)
}

pub async fn resolve(id: &str) -> Result<(Value, String), String> {
    let mut chain: Vec<Value> = Vec::new();
    let mut current = id.to_string();
    for _ in 0..6 {
        let v = version_json(&current).await?;
        let parent = v
            .get("inheritsFrom")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        chain.push(v);
        match parent {
            Some(p) => current = p,
            None => break,
        }
    }
    let jar_id = chain
        .last()
        .and_then(|v| v.get("id"))
        .and_then(|x| x.as_str())
        .unwrap_or(id)
        .to_string();
    let mut iter = chain.into_iter().rev();
    let mut merged = iter.next().ok_or("La versión está vacía")?;
    for child in iter {
        merged = merge(merged, child);
    }
    Ok((merged, jar_id))
}

fn arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    }
}

pub fn features() -> HashMap<&'static str, bool> {
    features_for(false)
}

pub fn features_for(quick_multiplayer: bool) -> HashMap<&'static str, bool> {
    let mut f = HashMap::new();
    f.insert("is_demo_user", false);
    f.insert("has_custom_resolution", true);
    f.insert("has_quick_plays_support", quick_multiplayer);
    f.insert("is_quick_play_singleplayer", false);
    f.insert("is_quick_play_multiplayer", quick_multiplayer);
    f.insert("is_quick_play_realms", false);
    f
}

fn rule_matches(rule: &Value, feats: &HashMap<&'static str, bool>) -> bool {
    if let Some(os) = rule.get("os") {
        if let Some(n) = os.get("name").and_then(|x| x.as_str()) {
            if n != "linux" {
                return false;
            }
        }
        if let Some(a) = os.get("arch").and_then(|x| x.as_str()) {
            if a != arch() {
                return false;
            }
        }
    }
    if let Some(Value::Object(f)) = rule.get("features") {
        for (k, v) in f {
            let want = v.as_bool().unwrap_or(false);
            let have = feats.get(k.as_str()).copied().unwrap_or(false);
            if want != have {
                return false;
            }
        }
    }
    true
}

pub fn allowed(rules: Option<&Value>, feats: &HashMap<&'static str, bool>) -> bool {
    let Some(Value::Array(arr)) = rules else {
        return true;
    };
    if arr.is_empty() {
        return true;
    }
    let mut ok = false;
    for r in arr {
        if rule_matches(r, feats) {
            ok = r.get("action").and_then(|a| a.as_str()) == Some("allow");
        }
    }
    ok
}

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn u64_field(v: &Value, k: &str) -> u64 {
    v.get(k).and_then(|x| x.as_u64()).unwrap_or(0)
}

fn maven_path(name: &str) -> Option<String> {
    let (coords, ext) = match name.split_once('@') {
        Some((c, e)) => (c, e),
        None => (name, "jar"),
    };
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let art = parts[1];
    let ver = parts[2];
    let file = if parts.len() > 3 {
        format!("{art}-{ver}-{}.{ext}", parts[3])
    } else {
        format!("{art}-{ver}.{ext}")
    };
    Some(format!("{group}/{art}/{ver}/{file}"))
}

fn ga_key(name: &str) -> String {
    let p: Vec<&str> = name.split(':').collect();
    if p.len() >= 4 {
        format!("{}:{}:{}", p[0], p[1], p[3])
    } else if p.len() >= 2 {
        format!("{}:{}", p[0], p[1])
    } else {
        name.to_string()
    }
}

pub struct LibraryPlan {
    pub jobs: Vec<Job>,
    pub classpath: Vec<PathBuf>,
    pub natives: Vec<PathBuf>,
}

fn artifact_job(art: &Value) -> Option<(Job, PathBuf)> {
    let path = art.get("path").and_then(|p| p.as_str())?;
    let dest = paths::libraries().join(path);
    let url = art.get("url").and_then(|u| u.as_str()).unwrap_or("");
    let job = Job {
        url: url.to_string(),
        path: dest.clone(),
        sha1: str_field(art, "sha1"),
        size: u64_field(art, "size"),
    };
    Some((job, dest))
}

pub fn plan_libraries(v: &Value) -> LibraryPlan {
    let feats = features();
    let mut plan = LibraryPlan {
        jobs: Vec::new(),
        classpath: Vec::new(),
        natives: Vec::new(),
    };
    let mut seen: HashSet<String> = HashSet::new();
    let libs = v
        .get("libraries")
        .and_then(|l| l.as_array())
        .cloned()
        .unwrap_or_default();
    for lib in libs.iter() {
        if !allowed(lib.get("rules"), &feats) {
            continue;
        }
        let name = lib.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let has_natives = lib.get("natives").is_some();
        let artifact = lib.pointer("/downloads/artifact");
        if (!has_natives || artifact.is_some()) && seen.insert(ga_key(name)) {
            if let Some(art) = artifact {
                if let Some((job, dest)) = artifact_job(art) {
                    if !job.url.is_empty() {
                        plan.jobs.push(job);
                    }
                    plan.classpath.push(dest.clone());
                    if name.contains(":natives-") {
                        plan.natives.push(dest);
                    }
                }
            } else if let Some(rel) = maven_path(name) {
                let base = lib
                    .get("url")
                    .and_then(|u| u.as_str())
                    .filter(|u| !u.is_empty())
                    .unwrap_or(LIBRARIES_URL);
                let base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                let dest = paths::libraries().join(&rel);
                plan.jobs.push(Job {
                    url: format!("{base}{rel}"),
                    path: dest.clone(),
                    sha1: str_field(lib, "sha1"),
                    size: u64_field(lib, "size"),
                });
                plan.classpath.push(dest);
            }
        }
        if let Some(cls) = lib
            .get("natives")
            .and_then(|n| n.get("linux"))
            .and_then(|c| c.as_str())
        {
            let cls = cls.replace("${arch}", "64");
            let classifier = lib
                .get("downloads")
                .and_then(|d| d.get("classifiers"))
                .and_then(|c| c.get(cls.as_str()));
            if let Some(art) = classifier {
                if let Some((job, dest)) = artifact_job(art) {
                    if !job.url.is_empty() {
                        plan.jobs.push(job);
                    }
                    plan.natives.push(dest);
                }
            }
        }
    }
    plan
}

pub fn jar_path(jar_id: &str) -> PathBuf {
    paths::versions().join(jar_id).join(format!("{jar_id}.jar"))
}

pub fn client_job(v: &Value, jar_id: &str) -> Option<Job> {
    let c = v.pointer("/downloads/client")?;
    Some(Job {
        url: c.get("url")?.as_str()?.to_string(),
        path: jar_path(jar_id),
        sha1: str_field(c, "sha1"),
        size: u64_field(c, "size"),
    })
}

pub fn logging(v: &Value) -> Option<(Job, String)> {
    let client = v.pointer("/logging/client")?;
    let file = client.get("file")?;
    let id = file.get("id")?.as_str()?;
    let job = Job {
        url: file.get("url")?.as_str()?.to_string(),
        path: paths::assets().join("log_configs").join(id),
        sha1: str_field(file, "sha1"),
        size: u64_field(file, "size"),
    };
    let arg = client.get("argument")?.as_str()?.to_string();
    Some((job, arg))
}

pub fn java_major(v: &Value) -> u64 {
    v.pointer("/javaVersion/majorVersion")
        .and_then(|m| m.as_u64())
        .unwrap_or(8)
}

pub struct AssetPlan {
    pub jobs: Vec<Job>,
    pub index_id: String,
}

pub async fn plan_assets(v: &Value) -> Result<AssetPlan, String> {
    let index = v.get("assetIndex").ok_or("La versión no define assetIndex")?;
    let id = index
        .get("id")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("assets").and_then(|x| x.as_str()))
        .ok_or("assetIndex sin id")?
        .to_string();
    let url = index
        .get("url")
        .and_then(|x| x.as_str())
        .ok_or("assetIndex sin url")?
        .to_string();
    let idx_path = paths::assets().join("indexes").join(format!("{id}.json"));
    http::download(&Job {
        url,
        path: idx_path.clone(),
        sha1: str_field(index, "sha1"),
        size: u64_field(index, "size"),
    })
    .await?;
    let data = tokio::fs::read(&idx_path).await.ctx()?;
    let parsed: Value = serde_json::from_slice(&data).ctx()?;
    let mut jobs = Vec::new();
    if let Some(objects) = parsed.get("objects").and_then(|o| o.as_object()) {
        for (_, obj) in objects {
            let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) else {
                continue;
            };
            if hash.len() < 2 {
                continue;
            }
            let sub = &hash[..2];
            jobs.push(Job {
                url: format!("{RESOURCES_URL}/{sub}/{hash}"),
                path: paths::assets().join("objects").join(sub).join(hash),
                sha1: Some(hash.to_string()),
                size: u64_field(obj, "size"),
            });
        }
    }
    Ok(AssetPlan { jobs, index_id: id })
}

pub async fn link_legacy_assets(index_id: &str, game_dir: &Path) -> Result<PathBuf, String> {
    let idx_path = paths::assets().join("indexes").join(format!("{index_id}.json"));
    let data = tokio::fs::read(&idx_path).await.ctx()?;
    let parsed: Value = serde_json::from_slice(&data).ctx()?;
    let is_virtual = parsed.get("virtual").and_then(|x| x.as_bool()).unwrap_or(false);
    let to_resources = parsed
        .get("map_to_resources")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if !is_virtual && !to_resources {
        return Ok(paths::assets());
    }
    let target = if to_resources {
        game_dir.join("resources")
    } else {
        paths::assets().join("virtual").join(index_id)
    };
    let target_clone = target.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let Some(objects) = parsed.get("objects").and_then(|o| o.as_object()) else {
            return Ok(());
        };
        for (name, obj) in objects {
            let Some(hash) = obj.get("hash").and_then(|h| h.as_str()) else {
                continue;
            };
            if hash.len() < 2 {
                continue;
            }
            let Some(rel) = crate::util::safe_rel(name) else {
                continue;
            };
            let src = paths::assets().join("objects").join(&hash[..2]).join(hash);
            let dst = target_clone.join(rel);
            if dst.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                std::fs::create_dir_all(parent).ctx()?;
            }
            std::fs::copy(&src, &dst).ctx()?;
        }
        Ok(())
    })
    .await
    .ctx()??;
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn maven_paths_are_built_from_coordinates() {
        assert_eq!(
            maven_path("net.fabricmc:fabric-loader:0.15.7").as_deref(),
            Some("net/fabricmc/fabric-loader/0.15.7/fabric-loader-0.15.7.jar")
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-linux").as_deref(),
            Some("org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-linux.jar")
        );
    }

    #[test]
    fn rules_filter_by_os() {
        let feats = features();
        let linux = json!([{ "action": "allow", "os": { "name": "linux" } }]);
        let windows = json!([{ "action": "allow", "os": { "name": "windows" } }]);
        let not_osx = json!([{ "action": "allow" }, { "action": "disallow", "os": { "name": "osx" } }]);
        assert!(allowed(Some(&linux), &feats));
        assert!(!allowed(Some(&windows), &feats));
        assert!(allowed(Some(&not_osx), &feats));
        assert!(allowed(None, &feats));
    }

    #[test]
    fn child_version_overrides_and_extends_parent() {
        let parent = json!({
            "id": "1.20.1",
            "mainClass": "net.minecraft.client.main.Main",
            "libraries": [{ "name": "a:b:1" }],
            "arguments": { "game": ["--a"], "jvm": ["-x"] }
        });
        let child = json!({
            "id": "fabric-loader",
            "inheritsFrom": "1.20.1",
            "mainClass": "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries": [{ "name": "c:d:2" }],
            "arguments": { "game": ["--b"], "jvm": [] }
        });
        let merged = merge(parent, child);
        assert_eq!(merged["mainClass"], "net.fabricmc.loader.impl.launch.knot.KnotClient");
        assert_eq!(merged["libraries"][0]["name"], "c:d:2");
        assert_eq!(merged["libraries"][1]["name"], "a:b:1");
        assert_eq!(merged["arguments"]["game"], json!(["--a", "--b"]));
        assert!(merged.get("inheritsFrom").is_none());
    }

    #[test]
    fn library_plan_dedupes_and_extracts_natives() {
        let v = json!({
            "libraries": [
                { "name": "g:a:2", "downloads": { "artifact": { "path": "g/a/2/a-2.jar", "url": "https://x/a-2.jar", "sha1": "00", "size": 1 } } },
                { "name": "g:a:1", "downloads": { "artifact": { "path": "g/a/1/a-1.jar", "url": "https://x/a-1.jar", "sha1": "00", "size": 1 } } },
                { "name": "org.lwjgl:lwjgl:3:natives-linux", "downloads": { "artifact": { "path": "l/n.jar", "url": "https://x/n.jar", "sha1": "00", "size": 1 } } }
            ]
        });
        let plan = plan_libraries(&v);
        assert_eq!(plan.classpath.len(), 2);
        assert_eq!(plan.natives.len(), 1);
        assert!(plan.classpath[0].ends_with("g/a/2/a-2.jar"));
    }
}
