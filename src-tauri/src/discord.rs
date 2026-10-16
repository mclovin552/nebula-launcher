use serde_json::json;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

static DONE: AtomicBool = AtomicBool::new(false);
static RUNNING: AtomicBool = AtomicBool::new(false);

fn app_id() -> String {
    let s = crate::settings::load().discord_app_id.trim().to_string();
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) {
        s
    } else {
        String::new()
    }
}

#[cfg(unix)]
fn socket_path() -> Option<std::path::PathBuf> {
    let base = std::env::var("XDG_RUNTIME_DIR").ok().or_else(|| {
        let status = std::fs::read_to_string("/proc/self/status").ok()?;
        let uid = status
            .lines()
            .find_map(|l| {
                let rest = l.strip_prefix("Uid:")?;
                rest.split_whitespace().next().map(|s| s.to_string())
            })
            .unwrap_or_else(|| "1000".to_string());
        Some(format!("/run/user/{uid}"))
    })?;
    for n in 0..10 {
        let p = std::path::PathBuf::from(format!("{base}/discord-ipc-{n}"));
        if p.exists() {
            return Some(p);
        }
    }
    None
}

fn nonce() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 * 1_000_000 + d.as_secs() % 1_000_000)
        .unwrap_or(0);
    format!("nebula-{n}")
}

#[cfg(unix)]
fn write_frame(socket: &mut UnixStream, op: u32, payload: &[u8]) -> std::io::Result<()> {
    let mut buf = Vec::with_capacity(8 + payload.len());
    buf.extend(op.to_le_bytes());
    buf.extend((payload.len() as u32).to_le_bytes());
    buf.extend_from_slice(payload);
    socket.write_all(&buf)
}

#[cfg(unix)]
fn drain(socket: &UnixStream) {
    if let Ok(mut clone) = socket.try_clone() {
        clone.set_nonblocking(true).ok();
        let mut byte = [0u8; 4096];
        loop {
            match std::io::Read::read(&mut clone, &mut byte) {
                Ok(n) if n > 0 => continue,
                _ => break,
            }
        }
    }
}

#[cfg(unix)]
fn send_activity(app_id: &str, activity_value: serde_json::Value) -> std::io::Result<()> {
    let path = socket_path().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Discord no está en ejecución")
    })?;
    let mut socket = UnixStream::connect(&path)?;

    let hello = json!({"v": 1, "client_id": app_id}).to_string();
    write_frame(&mut socket, 0, hello.as_bytes())?;
    use std::io::Read;
    let mut buf = [0u8; 4096];
    let _ = socket.read(&mut buf);
    drain(&socket);

    let payload = json!({
        "cmd": "SET_ACTIVITY",
        "args": { "pid": std::process::id(), "activity": activity_value },
        "nonce": nonce()
    })
    .to_string();
    write_frame(&mut socket, 1, payload.as_bytes())?;
    socket.flush()
}

#[cfg(not(unix))]
fn send_activity(_app_id: &str, _activity_value: serde_json::Value) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "sin soporte",
    ))
}

pub fn start(details: String, state: String) {
    let id = app_id();
    if id.is_empty() {
        return;
    }
    if RUNNING.load(Ordering::SeqCst) {
        stop();
        std::thread::sleep(Duration::from_millis(300));
    }
    DONE.store(false, Ordering::SeqCst);
    let start_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::thread::spawn(move || {
        RUNNING.store(true, Ordering::SeqCst);
        let activity = json!({
            "details": details,
            "state": state,
            "timestamps": { "start": start_secs },
            "instance": true
        });
        let mut connected = false;
        for _ in 0..5 {
            if DONE.load(Ordering::SeqCst) {
                break;
            }
            if send_activity(&id, activity.clone()).is_ok() {
                connected = true;
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        if !connected {
            RUNNING.store(false, Ordering::SeqCst);
            DONE.store(false, Ordering::SeqCst);
            return;
        }
        let mut elapsed = 0u64;
        while !DONE.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(15));
            if send_activity(&id, activity.clone()).is_err() {
                break;
            }
            elapsed += 15;
            if elapsed > 8 * 60 * 60 {
                break;
            }
        }
        RUNNING.store(false, Ordering::SeqCst);
        DONE.store(false, Ordering::SeqCst);
    });
}

pub fn stop() {
    DONE.store(true, Ordering::SeqCst);
}
