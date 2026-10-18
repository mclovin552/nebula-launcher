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
    if let Some(uuid) = url_or_path.strip_prefix("crafty:") {
        return crafty_texture(uuid).await;
    }
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

const UA_CRAFTY: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36";

#[derive(Clone, Serialize)]
pub struct CraftyItem {
    pub uuid: String,
    pub hash: String,
    pub title: String,
    pub model: String,
    pub players: i64,
    pub render: String,
}

#[derive(Clone)]
enum Dv {
    Num(i64),
    Str(String),
    Bool(bool),
    Null,
    Arr(Vec<usize>),
    Obj(Vec<(String, usize)>),
}

fn build(v: &serde_json::Value) -> Result<Vec<Dv>, String> {
    let items = match v {
        serde_json::Value::Array(a) => a.clone(),
        _ => return Err("Formato inesperado del catálogo de crafty.gg".to_string()),
    };
    let mut nodes = Vec::new();
    for it in &items {
        let d = match it {
            serde_json::Value::String(s) => Dv::Str(s.clone()),
            serde_json::Value::Bool(b) => Dv::Bool(*b),
            serde_json::Value::Null => Dv::Null,
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(k) => Dv::Num(k),
                None => Dv::Num(0),
            },
            serde_json::Value::Array(a) => {
                let refs = a
                    .iter()
                    .map(|x| x.as_u64().map(|k| k as usize).unwrap_or(usize::MAX))
                    .collect();
                Dv::Arr(refs)
            }
            serde_json::Value::Object(m) => {
                let props = m
                    .iter()
                    .map(|(k, x)| {
                        (
                            k.clone(),
                            x.as_u64().map(|n| n as usize).unwrap_or(usize::MAX),
                        )
                    })
                    .collect();
                Dv::Obj(props)
            }
        };
        nodes.push(d);
    }
    Ok(nodes)
}

fn strg(nodes: &[Dv], i: usize) -> Option<String> {
    match nodes.get(i)? {
        Dv::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn num(nodes: &[Dv], i: usize) -> Option<i64> {
    match nodes.get(i)? {
        Dv::Num(n) => Some(*n),
        _ => None,
    }
}

fn get(nodes: &[Dv], i: usize, key: &str) -> Option<usize> {
    match nodes.get(i)? {
        Dv::Obj(props) => props.iter().find(|(n, _)| n == key).map(|(_, x)| *x),
        _ => None,
    }
}

fn crafty_item(nodes: &[Dv], i: usize) -> Option<CraftyItem> {
    let hash = strg(nodes, get(nodes, i, "hash")?)?;
    let uuid = strg(nodes, get(nodes, i, "id")?)?;
    let slim = get(nodes, i, "slim")
        .and_then(|j| nodes.get(j))
        .is_some_and(|d| matches!(d, Dv::Bool(true)));
    let players = get(nodes, i, "players_count")
        .and_then(|j| num(nodes, j))
        .unwrap_or(0);
    let mut title = String::new();
    if let Some(fp) = get(nodes, i, "first_player") {
        if let Some(n) = get(nodes, fp, "username").and_then(|j| strg(nodes, j)) {
            title = format!("usada por {n}");
        }
    }
    let mut colors: Vec<String> = Vec::new();
    if let Some(carr) = get(nodes, i, "colors") {
        if let Dv::Arr(refs) = nodes.get(carr)? {
            for r in refs.iter().take(3) {
                if let Some(nm) = get(nodes, *r, "name").and_then(|j| strg(nodes, j)) {
                    colors.push(nm);
                }
            }
        }
    }
    let head = if colors.is_empty() {
        "Skin".to_string()
    } else {
        let mut cs = colors[0].chars();
        cs.next()
            .map(|f| f.to_uppercase().collect::<String>())
            .unwrap_or_default()
            + cs.as_str()
    };
    let title = if title.is_empty() {
        head
    } else {
        format!("{head} — {title}")
    };
    Some(CraftyItem {
        render: format!("https://render.crafty.gg/3d/full/{hash}"),
        model: if slim { "slim" } else { "classic" }.to_string(),
        title,
        players,
        uuid,
        hash,
    })
}

pub async fn crafty_list(kind: &str) -> Result<Vec<CraftyItem>, String> {
    let bytes = http::client()
        .get("https://crafty.gg/skins/__data.json")
        .header("User-Agent", UA_CRAFTY)
        .send()
        .await
        .map_err(|e| format!("No hubo conexión con crafty.gg: {e}"))?;
    if !bytes.status().is_success() {
        return Err(format!(
            "crafty.gg respondió el catálogo con HTTP {}",
            bytes.status()
        ));
    }
    let raw: serde_json::Value = bytes
        .json()
        .await
        .map_err(|_| "El catálogo de crafty.gg no es JSON legible".to_string())?;
    let nodes = build(&raw["nodes"][1]["data"])?;
    let mut out: Vec<CraftyItem> = Vec::new();
    let mut group: Option<usize> = None;
    for (i, d) in nodes.iter().enumerate() {
        if let Dv::Obj(props) = d {
            if props.iter().any(|(n, _)| n == "top_viewed")
                && props.iter().any(|(n, _)| n == "most_worn")
                && props.iter().any(|(n, _)| n == "newest")
            {
                group = Some(i);
                break;
            }
        }
    }
    let gi = match group {
        Some(g) => g,
        None => return Err("El catálogo de crafty.gg no trajo listas".to_string()),
    };
    let mut keys: Vec<&str> = match kind {
        "newest" => vec!["newest"],
        "top_viewed" => vec!["top_viewed"],
        _ => vec!["most_worn"],
    };
    keys.extend(["most_worn", "top_viewed", "newest"]);
    let mut have: Vec<String> = Vec::new();
    for key in keys {
        if let Some(Dv::Arr(refs)) = get(&nodes, gi, key).and_then(|j| nodes.get(j).cloned()) {
            for rr in refs {
                if let Some(it) = crafty_item(&nodes, rr) {
                    if !have.contains(&it.uuid) {
                        have.push(it.uuid.clone());
                        out.push(it);
                    }
                }
            }
        }
    }
    Ok(out)
}

pub async fn crafty_texture(uuid: &str) -> Result<Vec<u8>, String> {
    let resp = http::client()
        .get(format!(
            "https://crafty.gg/skins/{}",
            crate::util::url_param(uuid)
        ))
        .header("User-Agent", UA_CRAFTY)
        .header("Accept-Language", "en-US,en;q=0.9")
        .send()
        .await
        .map_err(|e| format!("No hubo conexión con crafty.gg: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!(
            "crafty.gg respondió el detalle con HTTP {}",
            resp.status()
        ));
    }
    let html = resp
        .text()
        .await
        .map_err(|_| "No se pudo leer el detalle de la skin".to_string())?;
    for m in html.match_indices("iVBOR") {
        let start = m.0;
        if let Some(end) = html[start..].find('"') {
            let b64 = &html[start..start + end];
            if b64.len() > 400 {
                use base64::Engine as _;
                if let Ok(data) = base64::engine::general_purpose::STANDARD.decode(b64.as_bytes()) {
                    if data.len() > 400
                        && data.starts_with(&[0x89, b'P', b'N', b'G'])
                        && data.len() < 20_000
                    {
                        if let Some((w, hh)) = png_dims(&data) {
                            if w == 64 && (hh == 32 || hh == 64) {
                                return Ok(data);
                            }
                        }
                    }
                }
            }
        }
    }
    Err("No se encontró la textura de esa skin en crafty.gg".to_string())
}

fn png_dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 {
        return None;
    }
    let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
    Some((w, h))
}

#[tokio::test]
#[ignore]
async fn crafty_works() {
    let l = crafty_list("populares").await.unwrap();
    println!(
        "populares: {} items, primero: {:?}",
        l.len(),
        l.first().map(|i| (&i.title, &i.uuid))
    );
    assert!(!l.is_empty());
    if let Some(it) = l.first() {
        let t = crafty_texture(&it.uuid).await.unwrap();
        println!("textura {} bytes", t.len());
        assert!(valid_png(&t));
    }
    let n = crafty_list("newest").await.unwrap();
    println!("newest: {}", n.len());
    assert!(!n.is_empty());
    let v = crafty_list("top_viewed").await.unwrap();
    println!("vistas: {}", v.len());
    assert!(!v.is_empty());
}
