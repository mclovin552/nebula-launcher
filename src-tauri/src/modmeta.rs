use serde_json::Value;
use std::io::Read;
use std::path::Path;

pub struct ModMeta {
    pub title: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
}

fn read_entry(zip: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let mut e = zip.by_name(name).ok()?;
    let mut buf = Vec::new();
    e.read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

fn icon(zip: &mut zip::ZipArchive<std::fs::File>, path: &str) -> Option<String> {
    let clean = path.trim_start_matches('/');
    let bytes = read_entry(zip, clean)?;
    if bytes.is_empty() || bytes.len() > 80_000 {
        return None;
    }
    Some(format!("data:image/png;base64,{}", b64(&bytes)))
}

fn text(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(|x| x.as_str()).map(|s| s.to_string())
}

pub fn toml_value(text: &str, key: &str) -> Option<String> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let t = line.trim();
        let Some(rest) = t.strip_prefix(key) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(val) = rest.strip_prefix('=') else {
            continue;
        };
        let val = val.trim();
        if val.starts_with("'''") || val.starts_with("\"\"\"") {
            let inner = val.trim_start_matches("'''").trim_start_matches("\"\"\"").trim();
            if !inner.is_empty() {
                return Some(
                    inner
                        .trim_end_matches("'''")
                        .trim_end_matches("\"\"\"")
                        .trim()
                        .to_string(),
                );
            }
            for next in lines.by_ref() {
                let n = next.trim();
                if !n.is_empty() {
                    return Some(
                        n.trim_end_matches("'''")
                            .trim_end_matches("\"\"\"")
                            .to_string(),
                    );
                }
            }
            return None;
        }
        let v = val.trim_matches('"').trim_matches('\'').to_string();
        if v.is_empty() {
            return None;
        }
        return Some(v);
    }
    None
}

pub fn read(path: &Path) -> Option<ModMeta> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    if let Some(raw) = read_entry(&mut zip, "fabric.mod.json") {
        if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
            let icon_path = match v.get("icon") {
                Some(Value::String(s)) => Some(s.clone()),
                Some(Value::Object(o)) => o
                    .values()
                    .filter_map(|x| x.as_str())
                    .last()
                    .map(|s| s.to_string()),
                _ => None,
            };
            let icon = icon_path.and_then(|p| icon(&mut zip, &p));
            return Some(ModMeta {
                title: text(&v, "name").or_else(|| text(&v, "id")),
                version: text(&v, "version"),
                description: text(&v, "description"),
                icon,
            });
        }
    }
    if let Some(raw) = read_entry(&mut zip, "quilt.mod.json") {
        if let Ok(v) = serde_json::from_slice::<Value>(&raw) {
            let loader = v.get("quilt_loader").cloned().unwrap_or(Value::Null);
            let meta = loader.get("metadata").cloned().unwrap_or(Value::Null);
            let icon_path = meta.get("icon").and_then(|i| i.as_str()).map(|s| s.to_string());
            let icon = icon_path.and_then(|p| icon(&mut zip, &p));
            return Some(ModMeta {
                title: text(&meta, "name").or_else(|| text(&loader, "id")),
                version: text(&loader, "version"),
                description: text(&meta, "description"),
                icon,
            });
        }
    }
    for name in ["META-INF/neoforge.mods.toml", "META-INF/mods.toml"] {
        if let Some(raw) = read_entry(&mut zip, name) {
            let body = String::from_utf8_lossy(&raw).to_string();
            let logo = toml_value(&body, "logoFile");
            let icon = logo.and_then(|p| icon(&mut zip, &p));
            return Some(ModMeta {
                title: toml_value(&body, "displayName"),
                version: toml_value(&body, "version").filter(|v| !v.contains("${")),
                description: toml_value(&body, "description"),
                icon,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_reference() {
        assert_eq!(b64(b"Man"), "TWFu");
        assert_eq!(b64(b"Ma"), "TWE=");
        assert_eq!(b64(b"M"), "TQ==");
    }

    #[test]
    fn toml_reads_simple_and_multiline() {
        let t = "displayName=\"Sodium\"\nversion=\"1.2.3\"\ndescription='''\nFast renderer\n'''\n";
        assert_eq!(toml_value(t, "displayName").as_deref(), Some("Sodium"));
        assert_eq!(toml_value(t, "version").as_deref(), Some("1.2.3"));
        assert_eq!(toml_value(t, "description").as_deref(), Some("Fast renderer"));
        assert_eq!(toml_value("versionRange=\"[1,2)\"\n", "version"), None);
    }
}
