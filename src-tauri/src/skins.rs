use crate::http;
use crate::paths;
use serde::Serialize;
use serde_json::json;
use std::path::PathBuf;

pub fn skinroot() -> PathBuf {
    paths::root().join("skins")
}

pub fn current_skin_path() -> PathBuf {
    skinroot().join("current_skin.png")
}

#[derive(Clone, Serialize)]
pub struct SkindexItem {
    pub slug: String,
    pub title: String,
    pub preview: String,
    pub full: String,
}

const CATEGORIES: [&str; 7] = [
    "people", "games", "movies", "tv", "fantasy", "mobs", "other",
];

fn enc(s: &str) -> String {
    crate::util::url_param(s)
}

async fn fetch_page(url: &str) -> Result<String, String> {
    let bytes = http::get_bytes(url)
        .await
        .map_err(|e| format!("No se pudo abrir el catálogo de skins: {e}"))?;
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn parse_cards(html: &str) -> Vec<SkindexItem> {
    let mut out: Vec<SkindexItem> = Vec::new();
    for block in html.split("<div class=\"card\">").skip(1) {
        let slug = block
            .split("<a class=\"panel-link\" href=\"/")
            .nth(1)
            .and_then(|h| h.split(&['?', '"']).next())
            .unwrap_or("");
        let title = block
            .split("alt=\"")
            .nth(1)
            .and_then(|t| t.split('"').next())
            .filter(|t| t.len() > 1)
            .unwrap_or("");
        if slug.is_empty() || slug.contains("category") {
            continue;
        }
        let title = if title.is_empty() {
            slug.replace(['-', '_'], " ")
        } else {
            title.to_string()
        };
        out.push(SkindexItem {
            preview: format!("https://www.minecraftskins.net/static/small_preview/{slug}.png"),
            full: format!("https://www.minecraftskins.net/static/skin/{slug}.png"),
            slug: slug.to_string(),
            title: title.to_string(),
        });
    }
    out
}

pub async fn skindex_search(query: &str, page: i64) -> Result<Vec<SkindexItem>, String> {
    let p = page.clamp(1, 50);
    let url = if query.trim().is_empty() {
        let cat = CATEGORIES[((p as usize).saturating_sub(1)) % CATEGORIES.len()];
        format!("https://www.minecraftskins.net/category/{cat}/{p}")
    } else {
        format!(
            "https://www.minecraftskins.net/search/{}?page={p}",
            enc(query.trim())
        )
    };
    Ok(parse_cards(&fetch_page(&url).await?))
}

pub async fn download_skin(url_or_path: &str) -> Result<Vec<u8>, String> {
    if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        http::get_bytes(url_or_path)
            .await
            .map_err(|e| format!("No se pudo descargar la skin: {e}"))
    } else {
        tokio::fs::read(url_or_path)
            .await
            .map_err(|e| format!("No se pudo leer el archivo: {e}"))
    }
}

pub fn valid_png(data: &[u8]) -> bool {
    data.len() > 12 && data.starts_with(&[0x89, b'P', b'N', b'G'])
}

pub async fn set_current_skin(bytes: &[u8]) -> Result<PathBuf, String> {
    paths::ensure()?;
    let dir = skinroot();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| e.to_string())?;
    let dest = current_skin_path();
    tokio::fs::write(&dest, bytes)
        .await
        .map_err(|e| format!("No se pudo guardar la skin local: {e}"))?;
    Ok(dest)
}

fn uuid_to_hyphenless(u: &str) -> &str {
    u
}

fn profile_payload(port: u16, uuid: &str, name: &str, slim: bool) -> String {
    let tex = if slim {
        json!({
            "SKIN": {"url": format!("http://127.0.0.1:{port}/current_skin.png"), "metadata": {"model": "slim"}}
        })
    } else {
        json!({
            "SKIN": {"url": format!("http://127.0.0.1:{port}/current_skin.png")}
        })
    };
    use base64::Engine as _;
    let texval = base64::engine::general_purpose::STANDARD.encode(tex.to_string().as_bytes());
    json!({
        "id": uuid_to_hyphenless(uuid),
        "name": name,
        "properties": [{"name": "textures", "value": texval}]
    })
    .to_string()
}

fn root_payload(_port: u16) -> String {
    json!({
        "meta": {
            "serverName": "Nebula skins locales",
            "implementationName": "Nebula local skin feed",
            "implementationVersion": "1.0"
        },
        "skinDomains": ["127.0.0.1", "localhost"],
    })
    .to_string()
}

#[derive(Clone)]
struct Srv {
    port: u16,
    name: String,
    skin: PathBuf,
    slim: bool,
}

async fn handle(mut sock: tokio::net::TcpStream, srv: Srv) -> std::io::Result<()> {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    let mut buf = [0u8; 4096];
    let n = sock.read(&mut buf).await.unwrap_or(0);
    let req = String::from_utf8_lossy(&buf[..n]);
    let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
    let method = req
        .split_whitespace()
        .next()
        .unwrap_or("GET")
        .to_uppercase();
    let base = path.split('?').next().unwrap_or("");
    let (status, ctype, body): (&str, &str, Vec<u8>) =
        if base.ends_with("api/authlib-injector") || base == "/" || base.is_empty() {
            (
                "200 OK",
                "application/json",
                root_payload(srv.port).into_bytes(),
            )
        } else if base.contains("current_skin.png") {
            match tokio::fs::read(&srv.skin).await {
                Ok(b) => ("200 OK", "image/png", b),
                Err(_) => ("404 Not Found", "text/plain", b"sin skin local".to_vec()),
            }
        } else if base.contains("/sessionserver/session/minecraft/profile/") && method == "GET" {
            let uuid = base.rsplit('/').next().unwrap_or("").to_string();
            let clean: String = uuid.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            if clean.len() != 32 {
                let e = json!({"error":"IllegalArgumentException","errorMessage":"bad uuid"})
                    .to_string();
                ("400 Bad Request", "application/json", e.into_bytes())
            } else {
                (
                    "200 OK",
                    "application/json",
                    profile_payload(srv.port, &clean, &srv.name, srv.slim).into_bytes(),
                )
            }
        } else if base.contains("join") || method == "POST" {
            ("204 No Content", "text/plain", Vec::new())
        } else if base.contains("meta") || base.contains("auth") {
            ("404 Not Found", "application/json", b"{}".to_vec())
        } else {
            (
                "200 OK",
                "application/json",
                json!({"status":"ok"}).to_string().into_bytes(),
            )
        };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(&body);
    let _ = sock.write_all(&out).await;
    let _ = sock.flush().await;
    Ok(())
}

pub async fn spawn_server(name: String, skin: PathBuf, slim: bool) -> Result<u16, String> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("No pude abrir el puerto local de skins: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let state = Srv {
        port,
        name,
        skin,
        slim,
    };
    let state2 = state.clone();
    tokio::task::spawn(async move {
        while let Ok((sock, _)) = listener.accept().await {
            let st = state2.clone();
            tokio::task::spawn(handle(sock, st));
        }
    });
    Ok(port)
}
