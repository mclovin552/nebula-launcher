use crate::http::client;
use crate::paths;
use crate::util::{now_millis, now_secs, Ctx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";
const MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

fn random_bytes(n: usize) -> Vec<u8> {
    #[cfg(unix)]
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read as _;
        let mut buf = vec![0u8; n];
        if f.read_exact(&mut buf).is_ok() {
            return buf;
        }
    }
    let mut seed = (now_millis() as u64 ^ (std::process::id() as u64) << 21) | 1;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        out.push((seed & 0xff) as u8);
    }
    out
}

fn random_token() -> String {
    let raw = random_bytes(48);
    let mut out = String::with_capacity(64);
    for b in raw {
        out.push(std::char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
        out.push(std::char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
    }
    out
}

fn pkce_challenge(verifier: &str) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(verifier.as_bytes());
    let mut out = String::with_capacity(44);
    for b in digest {
        out.push(std::char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
        out.push(std::char::from_digit((b & 0x0f) as u32, 16).unwrap_or('0'));
    }
    out
}

#[derive(Serialize)]
pub struct OAuthStart {
    pub authorize_url: String,
}

struct OAuthFlow {
    client_id: String,
    redirect_uri: String,
    code_verifier: String,
    state: String,
    code: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
    state_seen: Arc<Mutex<Option<String>>>,
    done: Arc<AtomicBool>,
}

static OAUTH: Mutex<Option<OAuthFlow>> = Mutex::new(None);

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push_str("%20"),
            _ => {
                out.push('%');
                out.push_str(&format!("{:02X}", b));
            }
        }
    }
    out
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next().unwrap_or(b'0');
            let h2 = bytes.next().unwrap_or(b'0');
            let hex = [h1 as char, h2 as char].iter().collect::<String>();
            let v = u8::from_str_radix(&hex, 16).unwrap_or(0);
            out.push(v as char);
        } else if b == b'+' {
            out.push(' ');
        } else {
            out.push(b as char);
        }
    }
    out
}

type ServerHandles = (
    u16,
    Arc<Mutex<Option<String>>>,
    Arc<Mutex<Option<String>>>,
    Arc<Mutex<Option<String>>>,
    Arc<AtomicBool>,
);

fn start_local_server() -> Result<ServerHandles, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("No se pudo abrir el servidor local (localhost): {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let _ = listener.set_nonblocking(true);

    let code: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let state_seen: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let done: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    let c = code.clone();
    let e = error.clone();
    let st = state_seen.clone();
    let d = done.clone();
    std::thread::spawn(move || {
        let _ = listener;
        loop {
            if d.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _addr)) => {
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).unwrap_or(0);
                    if n > 0 {
                        let req = String::from_utf8_lossy(&buf[..n]);
                        if let Some(first_line) = req.lines().next() {
                            let parts: Vec<&str> = first_line.split_whitespace().collect();
                            if parts.len() >= 2 {
                                let target = parts[1];
                                if let Some((path, query)) = target.split_once('?') {
                                    if path == "/callback" {
                                        for kv in query.split('&') {
                                            if let Some((k, v)) = kv.split_once('=') {
                                                match k {
                                                    "code" => {
                                                        *c.lock().unwrap() = Some(url_decode(v));
                                                    }
                                                    "error" => {
                                                        *e.lock().unwrap() = Some(url_decode(v));
                                                    }
                                                    "state" => {
                                                        *st.lock().unwrap() = Some(url_decode(v));
                                                    }
                                                    _ => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    let html = "<!doctype html><html><head><meta charset='utf-8'><title>Nebula - Sesión iniciada</title></head><body style='font-family:sans-serif;text-align:center;padding-top:40px'><h2>¡Sesión de Microsoft iniciada!</h2><p>Ya puedes cerrar esta pestaña y volver al lanzador.</p></body></html>";
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        html.len(),
                        html
                    );
                    let _ = stream.write_all(resp.as_bytes());
                    let _ = stream.flush();
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    d.store(true, Ordering::SeqCst);
                    break;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(200));
                }
                Err(_) => break,
            }
        }
        d.store(true, Ordering::SeqCst);
    });

    Ok((port, code, error, state_seen, done))
}

pub fn cancel_local_login() {
    if let Ok(mut g) = OAUTH.lock() {
        if let Some(f) = g.take() {
            f.done.store(true, Ordering::SeqCst);
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub uuid: String,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub expires_at: u64,
}

#[derive(Serialize, Clone)]
pub struct AccountView {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub uuid: String,
}

#[derive(Serialize)]
pub struct AccountList {
    pub accounts: Vec<AccountView>,
    pub selected: Option<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Store {
    accounts: Vec<Account>,
    selected: Option<String>,
}

fn load_store() -> Store {
    match std::fs::read(paths::root().join("accounts.json")) {
        Ok(data) => serde_json::from_slice(&data).unwrap_or_default(),
        Err(_) => Store::default(),
    }
}

fn save_store(store: &Store) -> Result<(), String> {
    paths::ensure()?;
    let data = serde_json::to_vec_pretty(store).ctx()?;
    let path = paths::root().join("accounts.json");
    std::fs::write(&path, data).ctx()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = &path;
    }
    Ok(())
}

fn view(a: &Account) -> AccountView {
    AccountView {
        id: a.id.clone(),
        kind: a.kind.clone(),
        name: a.name.clone(),
        uuid: a.uuid.clone(),
    }
}

pub fn list() -> AccountList {
    let store = load_store();
    AccountList {
        accounts: store.accounts.iter().map(view).collect(),
        selected: store.selected,
    }
}

pub fn offline_uuid(name: &str) -> String {
    let mut b = md5::compute(format!("OfflinePlayer:{name}").as_bytes()).0;
    b[6] = (b[6] & 0x0f) | 0x30;
    b[8] = (b[8] & 0x3f) | 0x80;
    crate::util::hex(&b)
}

pub fn add_offline(name: &str) -> Result<AccountView, String> {
    let name = name.trim().to_string();
    let valid = name.len() >= 3
        && name.len() <= 16
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if !valid {
        return Err(
            "El nombre debe tener de 3 a 16 caracteres: letras, números o guion bajo".to_string(),
        );
    }
    let mut store = load_store();
    let acc = Account {
        id: format!("offline-{}", now_millis()),
        kind: "offline".to_string(),
        uuid: offline_uuid(&name),
        name,
        access_token: "0".to_string(),
        refresh_token: String::new(),
        expires_at: 0,
    };
    let v = view(&acc);
    store.selected = Some(acc.id.clone());
    store.accounts.push(acc);
    save_store(&store)?;
    Ok(v)
}

pub fn remove(id: &str) -> Result<(), String> {
    let mut store = load_store();
    store.accounts.retain(|a| a.id != id);
    if store.selected.as_deref() == Some(id) {
        store.selected = store.accounts.first().map(|a| a.id.clone());
    }
    save_store(&store)
}

pub fn select(id: &str) -> Result<(), String> {
    let mut store = load_store();
    if !store.accounts.iter().any(|a| a.id == id) {
        return Err("La cuenta no existe".to_string());
    }
    store.selected = Some(id.to_string());
    save_store(&store)
}

pub async fn begin() -> Result<OAuthStart, String> {
    let client_id = MS_CLIENT_ID;

    let (port, code, error, state_seen, done) = start_local_server()?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", port);
    let code_verifier = random_token();
    let challenge = pkce_challenge(&code_verifier);
    let state = random_token();

    *OAUTH.lock().unwrap() = Some(OAuthFlow {
        client_id: client_id.to_string(),
        redirect_uri: redirect_uri.clone(),
        code_verifier,
        state: state.clone(),
        code,
        error,
        state_seen,
        done,
    });

    let authorize_url = format!(
        "https://login.microsoftonline.com/common/oauth2/v2.0/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&prompt=select_account&code_challenge={}&code_challenge_method=S256&state={}",
        url_encode(client_id),
        url_encode(&redirect_uri),
        url_encode(SCOPE),
        url_encode(&challenge),
        url_encode(&state)
    );

    Ok(OAuthStart { authorize_url })
}

fn xerr(code: u64) -> &'static str {
    match code {
        2148916233 => "Esta cuenta Microsoft no tiene perfil de Xbox. Crea uno en xbox.com.",
        2148916235 => "Xbox Live no está disponible en tu país.",
        2148916236 | 2148916237 => "La cuenta requiere verificación de adulto en Xbox.",
        2148916238 => "Cuenta de menor de edad: debe pertenecer a una familia de Microsoft.",
        _ => "Xbox Live rechazó el inicio de sesión.",
    }
}

async fn post_json(url: &str, body: Value) -> Result<(u16, Value), String> {
    let r = client()
        .post(url)
        .header("Accept", "application/json")
        .timeout(Duration::from_secs(40))
        .json(&body)
        .send()
        .await
        .ctx()?;
    let status = r.status().as_u16();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    Ok((status, v))
}

struct Minecraft {
    token: String,
    expires_at: u64,
    uuid: String,
    name: String,
}

async fn minecraft_chain(ms_token: &str) -> Result<Minecraft, String> {
    let (s1, xbl) = post_json(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={ms_token}")
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if s1 != 200 {
        return Err("No se pudo autenticar con Xbox Live".to_string());
    }
    let xbl_token = xbl
        .get("Token")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta de Xbox Live inválida")?
        .to_string();
    let uhs = xbl
        .pointer("/DisplayClaims/xui/0/uhs")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta de Xbox Live sin uhs")?
        .to_string();

    let (s2, xsts) = post_json(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }),
    )
    .await?;
    if s2 != 200 {
        let code = xsts.get("XErr").and_then(|x| x.as_u64()).unwrap_or(0);
        return Err(xerr(code).to_string());
    }
    let xsts_token = xsts
        .get("Token")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta XSTS inválida")?
        .to_string();

    let (s3, mc) = post_json(
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
    )
    .await?;
    if s3 != 200 {
        return Err(format!(
            "Minecraft rechazó el inicio de sesión ({s3}). Si es 403, tu Client ID de Azure aún no está aprobado: solicita la revisión en https://aka.ms/mce-reviewappid"
        ));
    }
    let token = mc
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or("Respuesta de Minecraft inválida")?
        .to_string();
    let expires_in = mc
        .get("expires_in")
        .and_then(|e| e.as_u64())
        .unwrap_or(86400);

    let ent = client()
        .get("https://api.minecraftservices.com/entitlements/mcstore")
        .bearer_auth(&token)
        .timeout(Duration::from_secs(40))
        .send()
        .await
        .ctx()?;
    let owns_game = if ent.status().is_success() {
        let ev: Value = ent.json().await.ctx().unwrap_or(Value::Null);
        ev.get("items")
            .and_then(|i| i.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(true)
    } else {
        true
    };
    if !owns_game {
        return Err(
            "Esta cuenta de Microsoft no tiene Minecraft Java Edition comprado. Cómpralo o activa Game Pass en minecraft.net e inténtalo de nuevo."
                .to_string(),
        );
    }

    let prof = client()
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&token)
        .timeout(Duration::from_secs(40))
        .send()
        .await
        .ctx()?;
    if prof.status().as_u16() == 404 {
        return Err("Esta cuenta no posee Minecraft Java Edition".to_string());
    }
    if !prof.status().is_success() {
        return Err("No se pudo leer el perfil de Minecraft".to_string());
    }
    let pv: Value = prof.json().await.ctx()?;
    let uuid = pv
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or("Perfil sin id")?
        .to_string();
    let name = pv
        .get("name")
        .and_then(|x| x.as_str())
        .ok_or("Perfil sin nombre")?
        .to_string();
    Ok(Minecraft {
        token,
        expires_at: now_secs() + expires_in,
        uuid,
        name,
    })
}

fn upsert(mc: Minecraft, refresh_token: String, select: bool) -> Result<AccountView, String> {
    let mut store = load_store();
    let id = format!("ms-{}", mc.uuid);
    let acc = Account {
        id: id.clone(),
        kind: "microsoft".to_string(),
        name: mc.name,
        uuid: mc.uuid,
        access_token: mc.token,
        refresh_token,
        expires_at: mc.expires_at,
    };
    let v = view(&acc);
    store.accounts.retain(|a| a.id != id);
    store.accounts.push(acc);
    if select {
        store.selected = Some(id);
    }
    save_store(&store)?;
    Ok(v)
}

pub async fn finish() -> Result<AccountView, String> {
    let flow = match OAUTH.lock().unwrap().take() {
        Some(f) => f,
        None => return Err("No hay un inicio de sesión en curso".to_string()),
    };

    let client_id = flow.client_id;
    let redirect_uri = flow.redirect_uri;

    let deadline = now_secs() + 300;
    let mut code: Option<String> = None;
    let mut err: Option<String> = None;
    loop {
        if let Some(c) = flow.code.lock().unwrap().clone() {
            code = Some(c);
            break;
        }
        if let Some(e) = flow.error.lock().unwrap().clone() {
            err = Some(e);
            break;
        }
        if now_secs() > deadline {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    if let Some(e) = err {
        return Err(format!("Microsoft rechazó el inicio de sesión: {e}"));
    }
    let code = code.ok_or("Tiempo de espera agotado. Vuelve a intentar iniciar sesión.")?;

    let seen_state = flow
        .state_seen
        .lock()
        .unwrap()
        .clone()
        .ok_or("La respuesta de Microsoft llegó sin verificación de estado (state)")?;
    if seen_state != flow.state {
        return Err("La respuesta de Microsoft no coincide con la solicitud (state inválido). Vuelve a intentarlo.".to_string());
    }

    let resp = client()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.trim()),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("scope", SCOPE),
            ("code_verifier", flow.code_verifier.as_str()),
        ])
        .send()
        .await
        .ctx()?;
    let status = resp.status();
    let body: Value = resp.json().await.ctx()?;
    if !status.is_success() {
        let desc = body
            .get("error_description")
            .and_then(|d| d.as_str())
            .unwrap_or("error desconocido");
        return Err(format!(
            "No se pudo canjear el código de autorización: {desc}"
        ));
    }
    let at = body
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or("Respuesta sin access_token")?
        .to_string();
    let rt = body
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let mc = minecraft_chain(&at).await?;
    upsert(mc, rt, true)
}

pub async fn resolve_for_launch(requested: Option<String>) -> Result<Account, String> {
    let store = load_store();
    let id = requested
        .or(store.selected.clone())
        .ok_or("Añade o selecciona una cuenta para jugar")?;
    let acc = store
        .accounts
        .iter()
        .find(|a| a.id == id)
        .cloned()
        .ok_or("La cuenta seleccionada no existe")?;
    if acc.kind != "microsoft" || acc.expires_at > now_secs() + 120 {
        return Ok(acc);
    }
    let resp = client()
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", MS_CLIENT_ID),
            ("refresh_token", acc.refresh_token.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .ctx()?;
    let status = resp.status();
    let body: Value = resp.json().await.ctx()?;
    if !status.is_success() {
        return Err(
            "La sesión de Microsoft caducó. Vuelve a iniciar sesión en Cuentas.".to_string(),
        );
    }
    let at = body
        .get("access_token")
        .and_then(|x| x.as_str())
        .ok_or("Respuesta sin access_token")?
        .to_string();
    let rt = body
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .unwrap_or(acc.refresh_token.clone());
    let mc = minecraft_chain(&at).await?;
    let token = mc.token.clone();
    let expires_at = mc.expires_at;
    let name = mc.name.clone();
    let uuid = mc.uuid.clone();
    upsert(mc, rt.clone(), false)?;
    Ok(Account {
        id: acc.id,
        kind: acc.kind,
        name,
        uuid,
        access_token: token,
        refresh_token: rt,
        expires_at,
    })
}

pub async fn change_skin(account_id: &str, path: &str, variant: &str) -> Result<(), String> {
    if variant != "classic" && variant != "slim" {
        return Err("Variante de skin inválida".to_string());
    }
    let acc = resolve_for_launch(Some(account_id.to_string())).await?;
    if acc.kind != "microsoft" {
        return Err("Solo las cuentas Microsoft pueden cambiar de skin".to_string());
    }
    let bytes = tokio::fs::read(path).await.ctx()?;
    if bytes.len() > 1_000_000
        || bytes.len() < 8
        || bytes[..8] != [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
    {
        return Err("La skin debe ser un archivo PNG de menos de 1 MB".to_string());
    }
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name("skin.png")
        .mime_str("image/png")
        .ctx()?;
    let form = reqwest::multipart::Form::new()
        .text("variant", variant.to_string())
        .part("file", part);
    let resp = client()
        .post("https://api.minecraftservices.com/minecraft/profile/skins")
        .bearer_auth(&acc.access_token)
        .multipart(form)
        .timeout(Duration::from_secs(60))
        .send()
        .await
        .ctx()?;
    if !resp.status().is_success() {
        return Err(format!("Minecraft rechazó la skin ({})", resp.status()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_uuid_is_version_three() {
        let u = offline_uuid("Notch");
        assert_eq!(u.len(), 32);
        assert_eq!(&u[12..13], "3");
        assert!(matches!(&u[16..17], "8" | "9" | "a" | "b"));
        assert_eq!(u, offline_uuid("Notch"));
        assert_ne!(u, offline_uuid("Jeb_"));
    }

    #[test]
    fn offline_names_are_validated() {
        assert!(add_offline("ab").is_err());
        assert!(add_offline("nombre con espacios").is_err());
    }
}
