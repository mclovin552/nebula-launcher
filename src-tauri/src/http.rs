use crate::util::{hex, Ctx};
use futures::StreamExt;
use serde::de::DeserializeOwned;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::io::AsyncWriteExt;

pub fn client() -> &'static reqwest::Client {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    C.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("NebulaLauncher/1.0.0 (linux)")
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(900))
            .pool_max_idle_per_host(64)
            .build()
            .expect("cliente http")
    })
}

pub async fn get_json<T: DeserializeOwned>(url: &str) -> Result<T, String> {
    let resp = client()
        .get(url)
        .timeout(Duration::from_secs(40))
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    let resp = resp.error_for_status().map_err(|e| format!("{url}: {e}"))?;
    resp.json::<T>().await.map_err(|e| format!("{url}: {e}"))
}

pub async fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = client()
        .get(url)
        .timeout(Duration::from_secs(120))
        .send()
        .await
        .map_err(|e| format!("{url}: {e}"))?;
    let resp = resp.error_for_status().map_err(|e| format!("{url}: {e}"))?;
    let bytes = resp.bytes().await.map_err(|e| format!("{url}: {e}"))?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("El archivo descargado es demasiado grande".to_string());
    }
    Ok(bytes.to_vec())
}

pub fn sha1_of_bytes(data: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(data);
    hex(&h.finalize())
}

pub fn sha1_of_file(path: &Path) -> Option<String> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut h = Sha1::new();
    std::io::copy(&mut f, &mut h).ok()?;
    Some(hex(&h.finalize()))
}

#[derive(Clone)]
pub struct Job {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: u64,
}

pub async fn check_job(job: &Job) -> bool {
    let Ok(meta) = tokio::fs::metadata(&job.path).await else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    if job.size > 0 {
        return meta.len() == job.size;
    }
    match &job.sha1 {
        Some(expected) => {
            let p = job.path.clone();
            let e = expected.clone();
            tokio::task::spawn_blocking(move || {
                sha1_of_file(&p)
                    .map(|h| h.eq_ignore_ascii_case(&e))
                    .unwrap_or(false)
            })
            .await
            .unwrap_or(false)
        }
        None => meta.len() > 0,
    }
}

pub static CANCEL: AtomicBool = AtomicBool::new(false);
pub const CANCELLED: &str = "Descarga cancelada";

pub fn reset_cancel() {
    CANCEL.store(false, Ordering::SeqCst);
}

pub fn request_cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}

fn cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

async fn partial_state(tmp: &Path) -> Option<(Sha1, u64)> {
    let t = tmp.to_path_buf();
    tokio::task::spawn_blocking(move || -> Option<(Sha1, u64)> {
        let mut f = std::fs::File::open(&t).ok()?;
        let mut h = Sha1::new();
        let len = std::io::copy(&mut f, &mut h).ok()?;
        Some((h, len))
    })
    .await
    .ok()
    .flatten()
}

async fn fetch_to(job: &Job) -> Result<(), String> {
    if let Some(parent) = job.path.parent() {
        tokio::fs::create_dir_all(parent).await.ctx()?;
    }
    let tmp = PathBuf::from(format!("{}.part", job.path.display()));
    let mut hasher = Sha1::new();
    let mut existing: u64 = 0;
    if tokio::fs::metadata(&tmp)
        .await
        .map(|m| m.len() > 0)
        .unwrap_or(false)
    {
        if let Some((h, len)) = partial_state(&tmp).await {
            hasher = h;
            existing = len;
        }
    }
    let mut req = client().get(&job.url);
    if existing > 0 {
        req = req.header("Range", format!("bytes={existing}-"));
    }
    let resp = req.send().await.ctx()?;
    let status = resp.status().as_u16();
    if status == 416 {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err("descarga parcial inválida".to_string());
    }
    let resp = resp.error_for_status().ctx()?;
    let resumed = existing > 0 && status == 206;
    if !resumed {
        hasher = Sha1::new();
    }
    let mut file = if resumed {
        tokio::fs::OpenOptions::new()
            .append(true)
            .open(&tmp)
            .await
            .ctx()?
    } else {
        tokio::fs::File::create(&tmp).await.ctx()?
    };
    let mut stream = resp.bytes_stream();
    let mut written: u64 = existing;
    const MAX_DOWNLOAD: u64 = 2 * 1024 * 1024 * 1024;
    while let Some(chunk) = stream.next().await {
        if cancelled() {
            let _ = file.flush().await;
            return Err(CANCELLED.to_string());
        }
        let chunk = chunk.ctx()?;
        hasher.update(&chunk);
        written += chunk.len() as u64;
        if (job.size > 0 && written > job.size) || written > MAX_DOWNLOAD {
            let _ = file.flush().await;
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("El archivo supera el tamaño esperado".to_string());
        }
        file.write_all(&chunk).await.ctx()?;
    }
    file.flush().await.ctx()?;
    drop(file);
    if let Some(expected) = &job.sha1 {
        let got = hex(&hasher.finalize());
        if !got.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err("suma de verificación incorrecta".to_string());
        }
    }
    tokio::fs::rename(&tmp, &job.path).await.ctx()
}

pub async fn download(job: &Job) -> Result<(), String> {
    if cancelled() {
        return Err(CANCELLED.to_string());
    }
    if check_job(job).await {
        return Ok(());
    }
    let mut last = String::new();
    for attempt in 0..4u64 {
        if cancelled() {
            return Err(CANCELLED.to_string());
        }
        match fetch_to(job).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e == CANCELLED {
                    return Err(e);
                }
                last = e;
                tokio::time::sleep(Duration::from_millis(400 * (attempt + 1))).await;
            }
        }
    }
    Err(format!("{}: {}", job.url, last))
}

pub async fn download_all<F>(jobs: Vec<Job>, concurrency: usize, progress: F) -> Result<(), String>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let mut seen = std::collections::HashSet::new();
    let jobs: Vec<Job> = jobs
        .into_iter()
        .filter(|j| seen.insert(j.path.clone()))
        .collect();
    let total = jobs.len() as u64;
    let done = Arc::new(AtomicU64::new(0));
    let progress = Arc::new(progress);
    (*progress)(0, total);
    let mut stream = futures::stream::iter(jobs)
        .map(|job| {
            let done = done.clone();
            let progress = progress.clone();
            async move {
                let r = download(&job).await;
                let n = done.fetch_add(1, Ordering::SeqCst) + 1;
                (*progress)(n, total);
                r
            }
        })
        .buffer_unordered(concurrency.max(1));
    let mut first_err: Option<String> = None;
    while let Some(r) = stream.next().await {
        if let Err(e) = r {
            if first_err.is_none() {
                first_err = Some(e);
            }
        }
    }
    match first_err {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
