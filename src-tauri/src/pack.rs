use crate::util::{safe_rel, Ctx};
use std::path::Path;

pub fn extract_prefixed(zip_path: &Path, prefixes: &[String], dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).ctx()?;
    let mut zip = zip::ZipArchive::new(file).ctx()?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).ctx()?;
        let name = entry.name().to_string();
        if name.starts_with("__MACOSX") {
            continue;
        }
        let mut rel: Option<String> = None;
        for p in prefixes {
            if let Some(r) = name.strip_prefix(p.as_str()) {
                rel = Some(r.to_string());
                break;
            }
        }
        let Some(rel) = rel else {
            continue;
        };
        if rel.is_empty() {
            continue;
        }
        let Some(rel_path) = safe_rel(&rel) else {
            continue;
        };
        let out = dest.join(rel_path);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).ctx()?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).ctx()?;
            }
            let mut o = std::fs::File::create(&out).ctx()?;
            std::io::copy(&mut entry, &mut o).ctx()?;
        }
    }
    Ok(())
}

pub fn collect_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(collect_files(&p));
            } else if p.is_file() {
                out.push(p);
            }
        }
    }
    out
}
