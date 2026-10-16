use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Serialize)]
pub struct ServerStatus {
    pub online: bool,
    pub motd: String,
    pub players_online: u64,
    pub players_max: u64,
    pub version: String,
    pub ping_ms: u64,
}

fn varint(mut v: u32) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v == 0 {
            out.push(b);
            break;
        }
        out.push(b | 0x80);
    }
    out
}

async fn read_byte<S: AsyncReadExt + Unpin>(r: &mut S) -> Result<u8, String> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).await.map_err(|e| e.to_string())?;
    Ok(b[0])
}

async fn read_varint<S: AsyncReadExt + Unpin>(r: &mut S) -> Result<u32, String> {
    let mut num: u32 = 0;
    for i in 0..5 {
        let b = read_byte(r).await?;
        num |= ((b & 0x7F) as u32) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(num);
        }
    }
    Err("varint demasiado largo".to_string())
}

fn motd_text(v: Option<&Value>) -> String {
    fn walk(v: &Value, out: &mut String) {
        match v {
            Value::String(s) => out.push_str(s),
            Value::Object(o) => {
                if let Some(Value::String(t)) = o.get("text") {
                    out.push_str(t);
                }
                if let Some(Value::Array(ex)) = o.get("extra") {
                    for e in ex {
                        walk(e, out);
                    }
                }
            }
            Value::Array(a) => {
                for e in a {
                    walk(e, out);
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    if let Some(v) = v {
        walk(v, &mut out);
    }
    let mut clean = String::new();
    let mut it = out.chars();
    while let Some(c) = it.next() {
        if c == '\u{00a7}' {
            let _ = it.next();
            continue;
        }
        clean.push(if c == '\n' { ' ' } else { c });
    }
    clean.trim().to_string()
}

pub async fn ping(address: &str) -> Result<ServerStatus, String> {
    let address = address.trim();
    let (host, port) = match address.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && p.parse::<u16>().is_ok() => {
            (h.to_string(), p.parse::<u16>().unwrap_or(25565))
        }
        _ => (address.to_string(), 25565u16),
    };
    let started = Instant::now();

    let fut = async {
        let mut stream = TcpStream::connect((host.as_str(), port))
            .await
            .map_err(|e| format!("{host}:{port}: {e}"))?;

        let mut body = Vec::new();
        body.extend(varint(0));
        body.extend(varint(0));
        let hb = host.as_bytes();
        body.extend(varint(hb.len() as u32));
        body.extend_from_slice(hb);
        body.extend(port.to_be_bytes());
        body.extend(varint(1));
        let mut packet = varint(body.len() as u32);
        packet.extend(body);
        stream.write_all(&packet).await.map_err(|e| e.to_string())?;

        stream.write_all(&[0x01, 0x00]).await.map_err(|e| e.to_string())?;

        let _len = read_varint(&mut stream).await?;
        let id = read_varint(&mut stream).await?;
        if id != 0 {
            return Err("Respuesta de estado inesperada".to_string());
        }
        let slen = read_varint(&mut stream).await?;
        if slen > 8 * 1024 * 1024 {
            return Err("Respuesta demasiado grande".to_string());
        }
        let mut buf = vec![0u8; slen as usize];
        stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        Ok::<String, String>(String::from_utf8_lossy(&buf).to_string())
    };

    let raw = tokio::time::timeout(Duration::from_secs(6), fut)
        .await
        .map_err(|_| "Tiempo de espera agotado".to_string())??;
    let ping_ms = started.elapsed().as_millis() as u64;

    let v: Value = serde_json::from_str(&raw).map_err(|_| "Respuesta JSON inválida".to_string())?;
    Ok(ServerStatus {
        online: true,
        motd: motd_text(v.get("description")),
        players_online: v.pointer("/players/online").and_then(|x| x.as_u64()).unwrap_or(0),
        players_max: v.pointer("/players/max").and_then(|x| x.as_u64()).unwrap_or(0),
        version: v
            .pointer("/version/name")
            .and_then(|x| x.as_str())
            .unwrap_or("?")
            .to_string(),
        ping_ms,
    })
}

pub async fn status(address: &str) -> ServerStatus {
    match ping(address).await {
        Ok(s) => s,
        Err(_) => ServerStatus {
            online: false,
            motd: String::new(),
            players_online: 0,
            players_max: 0,
            version: String::new(),
            ping_ms: 0,
        },
    }
}
