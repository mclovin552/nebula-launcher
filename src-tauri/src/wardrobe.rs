use crate::accounts;
use crate::http;
use crate::modmeta::b64;
use crate::paths;
use crate::util::{now_millis, Ctx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;

const PROFILE: &str = "https://api.minecraftservices.com/minecraft";

fn skins_dir() -> PathBuf {
    paths::root().join("skins")
}

fn index_path() -> PathBuf {
    skins_dir().join("skins.json")
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SkinRecord {
    pub id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct SkinEntry {
    pub id: String,
    pub name: String,
    pub data: String,
}

#[derive(Serialize)]
pub struct CapeEntry {
    pub id: String,
    pub alias: String,
    pub active: bool,
    pub url: String,
}

#[derive(Serialize)]
pub struct ProfileItems {
    pub skins: Vec<CapeEntry>,
    pub capes: Vec<CapeEntry>,
}

const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

fn png_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || bytes[..8] != PNG_MAGIC {
        return None;
    }
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    Some((w, h))
}

fn load_index() -> Vec<SkinRecord> {
    std::fs::read(index_path())
        .ok()
        .and_then(|d| serde_json::from_slice(&d).ok())
        .unwrap_or_default()
}

fn save_index(v: &[SkinRecord]) -> Result<(), String> {
    std::fs::create_dir_all(skins_dir()).ctx()?;
    let data = serde_json::to_vec_pretty(v).ctx()?;
    std::fs::write(index_path(), data).ctx()
}

pub fn skin_path(id: &str) -> PathBuf {
    skins_dir().join(format!("{id}.png"))
}

fn save_skin_bytes(name: &str, bytes: Vec<u8>) -> Result<SkinEntry, String> {
    let Some((w, h)) = png_size(&bytes) else {
        return Err("El archivo debe ser una imagen PNG de skin".to_string());
    };
    if w != 64 || (h != 32 && h != 64) {
        return Err(format!("La skin debe medir 64x32 o 64x64 y mide {w}x{h}"));
    }
    if bytes.len() > 1_000_000 {
        return Err("La skin debe pesar menos de 1 MB".to_string());
    }
    let index = load_index();
    let id = format!("skin-{}", now_millis());
    std::fs::create_dir_all(skins_dir()).ctx()?;
    std::fs::write(skin_path(&id), &bytes).ctx()?;
    let mut index = index;
    index.insert(
        0,
        SkinRecord {
            id: id.clone(),
            name: name.to_string(),
        },
    );
    save_index(&index)?;
    Ok(SkinEntry {
        id,
        name: name.to_string(),
        data: format!("data:image/png;base64,{}", b64(&bytes)),
    })
}

pub async fn add_skin(source: String, name: String) -> Result<SkinEntry, String> {
    let name = name.trim().chars().take(40).collect::<String>();
    let name = if name.is_empty() {
        "Skin".to_string()
    } else {
        name
    };
    if source.starts_with("https://") {
        let bytes = http::get_bytes(&source)
            .await
            .map_err(|e| format!("No se pudo descargar la skin: {e}"))?;
        save_skin_bytes(&name, bytes)
    } else {
        let bytes = tokio::fs::read(&source)
            .await
            .map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
        save_skin_bytes(&name, bytes)
    }
}

pub async fn add_skin_from_bytes(bytes: &[u8], name: String) -> Result<SkinEntry, String> {
    let name = name.replace(' ', "_");
    save_skin_bytes(&name, bytes.to_vec())
}

pub fn remove_skin(id: &str) -> Result<(), String> {
    let before = load_index();
    let after: Vec<SkinRecord> = before.iter().filter(|s| s.id != id).cloned().collect();
    if after.len() == before.len() {
        return Err("Esa skin no está en el armario".to_string());
    }
    save_index(&after)?;
    let _ = std::fs::remove_file(skin_path(id));
    Ok(())
}

pub fn list_skins() -> Vec<SkinEntry> {
    load_index()
        .into_iter()
        .filter_map(|r| {
            let bytes = std::fs::read(skin_path(&r.id)).ok()?;
            Some(SkinEntry {
                id: r.id,
                name: r.name,
                data: format!("data:image/png;base64,{}", b64(&bytes)),
            })
        })
        .collect()
}

async fn profile_json(account_id: &str) -> Result<Value, String> {
    let acc = accounts::resolve_for_launch(Some(account_id.to_string())).await?;
    if acc.kind != "microsoft" {
        return Err("Solo las cuentas Microsoft tienen perfil de skins y capas".to_string());
    }
    let resp = http::client()
        .get(format!("{PROFILE}/profile"))
        .bearer_auth(&acc.access_token)
        .send()
        .await
        .ctx()?;
    if !resp.status().is_success() {
        return Err(format!(
            "Minecraft no respondió el perfil ({})",
            resp.status()
        ));
    }
    resp.json().await.ctx()
}

pub async fn profile_items(account_id: &str) -> Result<ProfileItems, String> {
    let v = profile_json(account_id).await?;
    let map = |arr: Option<Vec<Value>>| -> Vec<CapeEntry> {
        arr.unwrap_or_default()
            .iter()
            .filter_map(|e| {
                Some(CapeEntry {
                    id: e.get("id").and_then(|x| x.as_str())?.to_string(),
                    alias: e
                        .get("alias")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                    active: e.get("state").and_then(|x| x.as_str()) == Some("ACTIVE"),
                    url: e
                        .get("url")
                        .and_then(|x| x.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
            })
            .collect()
    };
    Ok(ProfileItems {
        skins: map(v.get("skins").and_then(|s| s.as_array()).cloned()),
        capes: map(v.get("capes").and_then(|c| c.as_array()).cloned()),
    })
}

pub async fn set_cape(account_id: &str, cape_id: Option<String>) -> Result<(), String> {
    let acc = accounts::resolve_for_launch(Some(account_id.to_string())).await?;
    if acc.kind != "microsoft" {
        return Err("Solo las cuentas Microsoft tienen capas".to_string());
    }
    let cape = match cape_id {
        Some(c) if !c.trim().is_empty() => json!(c.trim()),
        _ => Value::Null,
    };
    let resp = http::client()
        .put(format!("{PROFILE}/profile/capes/active"))
        .bearer_auth(&acc.access_token)
        .json(&json!({ "capeId": cape }))
        .send()
        .await
        .ctx()?;
    if !resp.status().is_success() {
        return Err(format!("Minecraft rechazó la capa ({})", resp.status()));
    }
    Ok(())
}

pub async fn apply_saved_skin(
    account_id: &str,
    skin_id: &str,
    variant: &str,
) -> Result<(), String> {
    let bytes = std::fs::read(skin_path(skin_id))
        .map_err(|_| "Esa skin ya no está en el armario".to_string())?;
    let path = skins_dir().join(format!("tmp-{}", now_millis()));
    tokio::fs::write(&path, bytes).await.ctx()?;
    let result = accounts::change_skin(account_id, &path.to_string_lossy(), variant).await;
    let _ = tokio::fs::remove_file(&path).await;
    result
}

fn valid_player(p: &str) -> bool {
    !p.is_empty()
        && p.chars().count() <= 16
        && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub async fn ensure_csl(instance_id: &str) -> Result<(), String> {
    let inst = crate::instances::get(instance_id)?;
    if inst.loader == "vanilla" {
        return Err(
            "La skin local necesita una instancia con Fabric, Forge, NeoForge o Quilt".to_string(),
        );
    }
    let game = crate::instances::game_dir(instance_id);
    let mut installed = false;
    if let Ok(rd) = std::fs::read_dir(game.join("mods")) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_lowercase();
            if n.starts_with("customskinloader") {
                installed = true;
                break;
            }
        }
    }
    if !installed {
        crate::modrinth::install_project("mod", "customskinloader", instance_id)
            .await
            .map_err(|e| format!("No se pudo instalar CustomSkinLoader: {e}"))?;
    }
    write_csl_config(&game)
}

fn write_csl_config(game: &std::path::Path) -> Result<(), String> {
    let dir = game.join("CustomSkinLoader");
    std::fs::create_dir_all(&dir).ctx()?;
    let cfg = serde_json::json!({
        "version": "14.15",
        "buildNumber": 0,
        "enableTransparentSkin": true,
        "forceLoadAllTextures": true,
        "enableCape": true,
        "threadPoolSize": 8,
        "cacheExpiry": 30,
        "loadlist": [
            {
                "name": "LocalSkin",
                "type": "Legacy",
                "checkPNG": false,
                "model": "auto",
                "skin": "LocalSkin/skins/{USERNAME}.png",
                "cape": "LocalSkin/capes/{USERNAME}.png",
                "elytra": "LocalSkin/elytras/{USERNAME}.png"
            },
            {
                "name": "Mojang",
                "type": "MojangAPI",
                "apiRoot": "https://api.mojang.com/",
                "sessionRoot": "https://sessionserver.mojang.com/session/minecraft/profile/"
            }
        ]
    });
    std::fs::write(
        dir.join("CustomSkinLoader.json"),
        serde_json::to_string_pretty(&cfg).ctx()?,
    )
    .ctx()
}

pub async fn local_skin(instance_id: &str, player: &str, skin_id: &str) -> Result<(), String> {
    if !valid_player(player) {
        return Err("Nombre de jugador no válido".to_string());
    }
    let bytes = tokio::fs::read(skin_path(skin_id))
        .await
        .map_err(|_| "Esa skin ya no está en el armario".to_string())?;
    ensure_csl(instance_id).await?;
    let dest = crate::instances::game_dir(instance_id)
        .join("CustomSkinLoader")
        .join("LocalSkin")
        .join("skins");
    std::fs::create_dir_all(&dest).ctx()?;
    std::fs::write(dest.join(format!("{player}.png")), bytes).ctx()
}

pub async fn local_cape(instance_id: &str, player: &str, source: &str) -> Result<(), String> {
    if !valid_player(player) {
        return Err("Nombre de jugador no válido".to_string());
    }
    let bytes = if source.starts_with("https://") {
        http::get_bytes(source)
            .await
            .map_err(|e| format!("No se pudo descargar la capa: {e}"))?
    } else {
        tokio::fs::read(source)
            .await
            .map_err(|e| format!("No se pudo leer el archivo: {e}"))?
    };
    if bytes.len() < 8 || bytes[..8] != PNG_MAGIC {
        return Err("La capa debe ser un archivo PNG".to_string());
    }
    ensure_csl(instance_id).await?;
    let dest = crate::instances::game_dir(instance_id)
        .join("CustomSkinLoader")
        .join("LocalSkin")
        .join("capes");
    std::fs::create_dir_all(&dest).ctx()?;
    std::fs::write(dest.join(format!("{player}.png")), bytes).ctx()
}

pub fn parse_profile_textures(
    prof: &Value,
    value: &str,
    fallback_name: &str,
    source: &str,
) -> Result<Value, String> {
    use base64::Engine as _;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(value.as_bytes())
        .map_err(|e| format!("Textura del servidor ilegible ({e})"))?;
    let text_v = String::from_utf8_lossy(&decoded).to_string();
    let tex: Value = serde_json::from_str(&text_v).map_err(|e| e.to_string())?;
    let sk = tex.get("textures").and_then(|t| t.get("SKIN"));
    let url = sk
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .ok_or("Ese jugador no tiene skin publicada")?
        .to_string()
        .replace(
            "http://textures.minecraft.net",
            "https://textures.minecraft.net",
        );
    let cape = tex
        .get("textures")
        .and_then(|t| t.get("CAPE"))
        .and_then(|s| s.get("url"))
        .and_then(|u| u.as_str())
        .map(|s| {
            s.replace(
                "http://textures.minecraft.net",
                "https://textures.minecraft.net",
            )
        });
    let model = sk
        .and_then(|s| s.get("metadata"))
        .and_then(|m| m.get("model"))
        .and_then(|x| x.as_str())
        .unwrap_or("classic")
        .to_string();
    let name2 = prof
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(fallback_name)
        .to_string();
    Ok(json!({
        "name": name2,
        "skin": url,
        "cape": cape,
        "model": model,
        "source": source,
    }))
}

pub async fn lookup_skin(name: &str) -> Result<Value, String> {
    let name = name.trim();
    if !(3..=20).contains(&name.len())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err("Escribe un nick válido de Minecraft (3-20 letras, números o _ )".to_string());
    }
    mojang_lookup(name).await
}

async fn mojang_lookup(name: &str) -> Result<Value, String> {
    let up: Value = http::get_json(&format!(
        "https://api.mojang.com/users/profiles/minecraft/{name}"
    ))
    .await?;
    let id = up
        .get("id")
        .and_then(|i| i.as_str())
        .ok_or("No existe un usuario con ese nick")?
        .to_string();
    let prof: Value = http::get_json(&format!(
        "https://sessionserver.mojang.com/session/minecraft/profile/{id}"
    ))
    .await?;
    let prop = prof
        .get("properties")
        .and_then(|p| p.as_array())
        .and_then(|a| {
            a.iter()
                .find(|x| x.get("name").and_then(|n| n.as_str()) == Some("textures"))
        });
    let value = prop
        .and_then(|p| p.get("value"))
        .and_then(|v| v.as_str())
        .ok_or("Ese jugador no tiene texturas de skin publicadas")?;
    parse_profile_textures(&prof, value, name, "mojang")
}
