use crate::http;
use crate::paths;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Digest;

const AUTH: &str = "https://authserver.ely.by";
const SKINS: &str = "https://skinsystem.ely.by";
const LATEST: &str = "https://authlib-injector.yushi.moe/artifact/latest.json";
pub const AGENT_ROOT: &str = "https://authserver.ely.by/api/authlib-injector";

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

fn token_file() -> std::path::PathBuf {
    paths::root().join("ely_client_token.txt")
}

fn client_token() -> String {
    if let Ok(t) = std::fs::read_to_string(token_file()) {
        let t = t.trim().to_string();
        if t.len() >= 16 {
            return t;
        }
    }
    let t = crate::accounts::offline_uuid(&format!(
        "nebula-ely-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    ));
    let _ = paths::ensure();
    let _ = std::fs::write(token_file(), &t);
    t
}

fn ely_error(status: u16, body: &Value) -> String {
    let msg = body
        .get("errorMessage")
        .and_then(|x| x.as_str())
        .unwrap_or("Error desconocido de Ely.by");
    format!("Ely.by ({status}): {msg}")
}

async fn auth_post(path: &str, payload: Value) -> Result<Value, String> {
    let resp = http::client()
        .post(format!("{AUTH}{path}"))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("No hubo conexión con Ely.by: {e}"))?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .map_err(|_| format!("Ely.by respondió algo ilegible (HTTP {status})"))?;
    if !status.is_success() {
        return Err(ely_error(status.as_u16(), &body));
    }
    Ok(body)
}

pub async fn login(username: &str, password: &str) -> Result<(Value, String), String> {
    let ct = client_token();
    let resp = match auth_post(
        "/auth/authenticate",
        json!({
            "username": username.trim(),
            "password": password,
            "clientToken": ct,
            "requestUser": true,
        }),
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.contains("two factor") {
                "Tu cuenta Ely.by tiene 2FA activado: escribe tu contraseña y el código de 6 \
                 dígitos juntos, así: contrasena:123456"
                    .to_string()
            } else {
                e
            };
            return Err(msg);
        }
    };
    let prof = resp
        .get("selectedProfile")
        .or_else(|| resp.get("availableProfiles").and_then(|a| a.get(0)))
        .ok_or("Ely.by no devolvió el perfil del jugador")?;
    let token = resp
        .get("accessToken")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta de Ely.by sin accessToken")?
        .to_string();
    Ok((prof.clone(), token))
}

pub async fn validate(access_token: &str) -> bool {
    auth_post("/auth/validate", json!({ "accessToken": access_token }))
        .await
        .is_ok()
}

pub async fn refresh(access_token: &str) -> Result<(String, Value), String> {
    let resp = auth_post(
        "/auth/refresh",
        json!({
            "accessToken": access_token,
            "clientToken": client_token(),
        }),
    )
    .await?;
    let token = resp
        .get("accessToken")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta de Ely.by sin accessToken")?
        .to_string();
    let prof = resp.get("selectedProfile").cloned().unwrap_or(Value::Null);
    Ok((token, prof))
}

pub fn agent_dir() -> std::path::PathBuf {
    paths::root().join("agents")
}

pub async fn ensure_agent() -> Result<(std::path::PathBuf, String), String> {
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
                return Ok((target, art.version));
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
    Ok((target, art.version))
}

pub async fn lookup(nick: &str) -> Result<Value, String> {
    let prof: Value = http::get_json(&format!("{SKINS}/profile/{nick}?unsigned=false")).await?;
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
        .ok_or("Ese jugador no tiene skin en Ely.by")?;
    crate::wardrobe::parse_profile_textures(&prof, value, nick, "ely_by")
}
