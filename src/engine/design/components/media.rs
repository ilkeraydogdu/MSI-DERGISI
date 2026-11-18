use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::{OnceLock, RwLock};

static IMAGE_DIM_CACHE: OnceLock<RwLock<HashMap<String, Option<(u32, u32)>>>> = OnceLock::new();

/// Fast memoized WebP header dimension reader (`VP8 `, `VP8L`, `VP8X`) for zero-CLS smart framing.
pub fn image_dimensions(rel_path: &str) -> Option<(u32, u32)> {
    let clean = rel_path.trim().trim_start_matches('/');
    if clean.is_empty() || clean.starts_with("http://") || clean.starts_with("https://") {
        return None;
    }
    let cache = IMAGE_DIM_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(guard) = cache.read() {
        if let Some(&cached) = guard.get(clean) {
            return cached;
        }
    }
    let static_base = std::env::var("STATIC_DIR").unwrap_or_else(|_| "public".to_string());
    let candidate_direct = Path::new(clean);
    let candidate_public = Path::new("public").join(clean);
    let candidate_static = Path::new(&static_base).join(clean);
    let parsed = read_webp_header(candidate_direct)
        .or_else(|| read_webp_header(&candidate_static))
        .or_else(|| read_webp_header(&candidate_public));
    if let Ok(mut guard) = cache.write() {
        guard.insert(clean.to_string(), parsed);
    }
    parsed
}

fn read_webp_header(path: &Path) -> Option<(u32, u32)> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 32];
    let n = file.read(&mut buf).ok()?;
    if n < 30 || &buf[0..4] != b"RIFF" || &buf[8..12] != b"WEBP" {
        return None;
    }
    match &buf[12..16] {
        b"VP8 " => {
            let w = (u16::from_le_bytes([buf[26], buf[27]]) & 0x3fff) as u32;
            let h = (u16::from_le_bytes([buf[28], buf[29]]) & 0x3fff) as u32;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        b"VP8L" => {
            let bits = u32::from_le_bytes([buf[21], buf[22], buf[23], buf[24]]);
            let w = (bits & 0x3fff) + 1;
            let h = ((bits >> 14) & 0x3fff) + 1;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        b"VP8X" => {
            let w = (buf[24] as u32 | ((buf[25] as u32) << 8) | ((buf[26] as u32) << 16)) + 1;
            let h = (buf[27] as u32 | ((buf[28] as u32) << 8) | ((buf[29] as u32) << 16)) + 1;
            if w > 0 && h > 0 { Some((w, h)) } else { None }
        }
        _ => None,
    }
}

