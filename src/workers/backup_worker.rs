use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use chrono::{NaiveTime, Timelike, Utc};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use hmac::{Hmac, Mac};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::config::AppConfig;
use crate::controllers::AppState;
use crate::db::Database;

type HmacSha256 = Hmac<Sha256>;

const ENVELOPE_MAGIC: &[u8; 8] = b"MSIBKP01";
const ENVELOPE_HEADER_LEN: usize = 12;
const ENVELOPE_NONCE_LEN: usize = 12;
const KIND_FULL_DB: u8 = 1;
const KIND_PAGE_DELTA: u8 = 2;

const DELTA_MAGIC: &[u8; 9] = b"MSIDELTA1";
const DEFAULT_PAGE_SIZE: usize = 4096;

#[derive(Clone, Debug, Serialize)]
pub struct BackupResult {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub integrity_check: String,
    pub pruned_count: usize,
    pub encrypted_archive_path: Option<String>,
    pub encrypted_sha256: Option<String>,
    pub s3_uploaded: bool,
    pub s3_key: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IncrementalBackupResult {
    pub kind: String,
    pub archive_filename: String,
    pub archive_path: String,
    pub encrypted_sha256: String,
    pub target_db_sha256: String,
    pub changed_pages: usize,
    pub total_pages: usize,
    pub s3_uploaded: bool,
    pub s3_key: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct OffsiteState {
    last_base_filename: Option<String>,
    last_base_sha256: Option<String>,
    last_snapshot_sha256: Option<String>,
    updated_at: Option<String>,
}

fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn sha256_hex_bytes(data: &[u8]) -> String {
    hex::encode(sha256_bytes(data))
}

/// Derives a 256-bit AES-GCM key from `BACKUP_ENCRYPTION_KEY` (64-hex, 32-byte base64,
/// or arbitrary passphrase) or falls back to domain-separated `SESSION_SECRET`.
pub fn derive_backup_key(configured_key: &str, session_secret: &str) -> [u8; 32] {
    let trimmed = configured_key.trim();
    if trimmed.len() == 64 {
        if let Ok(raw) = hex::decode(trimmed) {
            if raw.len() == 32 {
                let mut out = [0u8; 32];
                out.copy_from_slice(&raw);
                return out;
            }
        }
    }
    if !trimmed.is_empty() {
        if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(trimmed) {
            if raw.len() == 32 {
                let mut out = [0u8; 32];
                out.copy_from_slice(&raw);
                return out;
            }
        }
        let mut hasher = Sha256::new();
        hasher.update(b"MSI-BACKUP-KEY-V1:");
        hasher.update(trimmed.as_bytes());
        return hasher.finalize().into();
    }

    warn!("BACKUP_ENCRYPTION_KEY is not configured; deriving backup key from SESSION_SECRET fallback");
    let mut hasher = Sha256::new();
    hasher.update(b"MSI-BACKUP-FALLBACK-V1:");
    hasher.update(session_secret.as_bytes());
    hasher.finalize().into()
}

fn gzip_compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .map_err(|e| format!("Gzip compression write error: {}", e))?;
    encoder
        .finish()
        .map_err(|e| format!("Gzip compression finish error: {}", e))
}

fn gzip_decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| format!("Gzip decompression error: {}", e))?;
    Ok(out)
}

fn encrypt_envelope(kind: u8, plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let compressed = gzip_compress(plaintext)?;
    let mut header = [0u8; ENVELOPE_HEADER_LEN];
    header[..8].copy_from_slice(ENVELOPE_MAGIC);
    header[8] = kind;

    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; ENVELOPE_NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| "SystemRandom nonce generation failed".to_string())?;

    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| "Invalid AES-256-GCM key".to_string())?;
    let aead_key = LessSafeKey::new(unbound);

    let mut in_out = compressed;
    aead_key
        .seal_in_place_append_tag(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::from(&header),
            &mut in_out,
        )
        .map_err(|_| "AES-256-GCM encryption failed".to_string())?;

    let mut envelope = Vec::with_capacity(ENVELOPE_HEADER_LEN + ENVELOPE_NONCE_LEN + in_out.len());
    envelope.extend_from_slice(&header);
    envelope.extend_from_slice(&nonce_bytes);
    envelope.extend_from_slice(&in_out);
    Ok(envelope)
}

fn decrypt_envelope(envelope: &[u8], key: &[u8; 32]) -> Result<(u8, Vec<u8>), String> {
    let min_len = ENVELOPE_HEADER_LEN + ENVELOPE_NONCE_LEN + 16;
    if envelope.len() < min_len {
        return Err("Encrypted backup envelope is truncated".to_string());
    }
    let header = &envelope[..ENVELOPE_HEADER_LEN];
    if &header[..8] != ENVELOPE_MAGIC {
        return Err("Invalid encrypted backup magic header".to_string());
    }
    let kind = header[8];
    if kind != KIND_FULL_DB && kind != KIND_PAGE_DELTA {
        return Err(format!("Unsupported backup envelope kind: {}", kind));
    }

    let mut nonce_bytes = [0u8; ENVELOPE_NONCE_LEN];
    nonce_bytes.copy_from_slice(&envelope[ENVELOPE_HEADER_LEN..ENVELOPE_HEADER_LEN + ENVELOPE_NONCE_LEN]);
    let mut ciphertext = envelope[ENVELOPE_HEADER_LEN + ENVELOPE_NONCE_LEN..].to_vec();

    let unbound = UnboundKey::new(&AES_256_GCM, key)
        .map_err(|_| "Invalid AES-256-GCM key".to_string())?;
    let aead_key = LessSafeKey::new(unbound);

    let decrypted_compressed = aead_key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce_bytes),
            Aad::from(header),
            &mut ciphertext,
        )
        .map_err(|_| "Backup decryption or authentication failed (wrong key or tampered archive)".to_string())?;

    let plain = gzip_decompress(decrypted_compressed)?;
    Ok((kind, plain))
}

fn encode_page_delta(
    base_bytes: &[u8],
    target_bytes: &[u8],
    page_size: usize,
) -> Result<(Vec<u8>, usize, usize), String> {
    let ps = if page_size == 0 { DEFAULT_PAGE_SIZE } else { page_size };
    let base_sha = sha256_bytes(base_bytes);
    let target_sha = sha256_bytes(target_bytes);
    let total_pages = target_bytes.len().div_ceil(ps);

    let mut changed: Vec<(u32, &[u8])> = Vec::new();
    for (idx, target_page) in target_bytes.chunks(ps).enumerate() {
        let start = idx * ps;
        let end = start + target_page.len();
        let matches_base = base_bytes
            .get(start..end)
            .map(|b_slice| b_slice == target_page)
            .unwrap_or(false);
        if !matches_base {
            changed.push((idx as u32, target_page));
        }
    }

    let changed_count = changed.len();
    let mut payload = Vec::with_capacity(9 + 32 + 32 + 8 + 4 + 4 + changed_count * (4 + ps));
    payload.extend_from_slice(DELTA_MAGIC);
    payload.extend_from_slice(&base_sha);
    payload.extend_from_slice(&target_sha);
    payload.extend_from_slice(&(target_bytes.len() as u64).to_le_bytes());
    payload.extend_from_slice(&(ps as u32).to_le_bytes());
    payload.extend_from_slice(&(changed_count as u32).to_le_bytes());

    for (page_idx, page_bytes) in changed {
        payload.extend_from_slice(&page_idx.to_le_bytes());
        payload.extend_from_slice(page_bytes);
    }

    Ok((payload, changed_count, total_pages))
}

fn apply_page_delta(base_bytes: &[u8], delta: &[u8]) -> Result<Vec<u8>, String> {
    let header_len = 9 + 32 + 32 + 8 + 4 + 4;
    if delta.len() < header_len {
        return Err("Incremental delta payload is truncated".to_string());
    }
    if &delta[..9] != DELTA_MAGIC {
        return Err("Invalid incremental delta magic header".to_string());
    }

    let expected_base_sha = &delta[9..41];
    let expected_target_sha = &delta[41..73];
    let target_len = u64::from_le_bytes(delta[73..81].try_into().unwrap()) as usize;
    let page_size = u32::from_le_bytes(delta[81..85].try_into().unwrap()) as usize;
    let changed_count = u32::from_le_bytes(delta[85..89].try_into().unwrap()) as usize;

    if page_size == 0 {
        return Err("Invalid page size 0 in delta payload".to_string());
    }
    if sha256_bytes(base_bytes) != expected_base_sha {
        return Err("Base snapshot SHA-256 does not match incremental delta base_sha256".to_string());
    }

    let mut restored = vec![0u8; target_len];
    let copy_init = base_bytes.len().min(target_len);
    restored[..copy_init].copy_from_slice(&base_bytes[..copy_init]);

    let mut cursor = header_len;
    for _ in 0..changed_count {
        if cursor + 4 > delta.len() {
            return Err("Truncated page index in delta payload".to_string());
        }
        let page_idx = u32::from_le_bytes(delta[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        let offset = page_idx
            .checked_mul(page_size)
            .ok_or_else(|| "Page offset overflow in delta".to_string())?;
        if offset >= target_len {
            return Err("Out-of-bounds page index in delta payload".to_string());
        }
        let expected_chunk_len = (target_len - offset).min(page_size);
        if cursor + expected_chunk_len > delta.len() {
            return Err("Truncated page bytes in delta payload".to_string());
        }
        restored[offset..offset + expected_chunk_len]
            .copy_from_slice(&delta[cursor..cursor + expected_chunk_len]);
        cursor += expected_chunk_len;
    }

    if sha256_bytes(&restored) != expected_target_sha {
        return Err("Restored snapshot SHA-256 verification failed".to_string());
    }

    Ok(restored)
}

/// Restores and verifies an encrypted full (`kind=1`) or incremental delta (`kind=2`) backup archive
/// to `out_db_path`, checking AES-256-GCM authenticity and `PRAGMA integrity_check == "ok"`.
pub fn restore_encrypted_backup(
    base_enc: Option<&Path>,
    snapshot_enc: &Path,
    key: &[u8; 32],
    out_db_path: &Path,
) -> Result<String, String> {
    let snap_bytes = std::fs::read(snapshot_enc)
        .map_err(|e| format!("Encrypted archive read failed ({:?}): {}", snapshot_enc, e))?;
    let (kind, plain) = decrypt_envelope(&snap_bytes, key)?;

    let db_bytes = match kind {
        KIND_FULL_DB => plain,
        KIND_PAGE_DELTA => {
            let base_path = base_enc.ok_or_else(|| {
                "Incremental delta restore requires base encrypted archive".to_string()
            })?;
            let base_raw = std::fs::read(base_path)
                .map_err(|e| format!("Base encrypted archive read failed ({:?}): {}", base_path, e))?;
            let (base_kind, base_plain) = decrypt_envelope(&base_raw, key)?;
            if base_kind != KIND_FULL_DB {
                return Err("Base archive must be a full database backup (kind=1)".to_string());
            }
            apply_page_delta(&base_plain, &plain)?
        }
        other => return Err(format!("Unsupported backup kind: {}", other)),
    };

    if let Some(parent) = out_db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Restore target directory creation failed: {}", e))?;
        }
    }
    std::fs::write(out_db_path, &db_bytes)
        .map_err(|e| format!("Failed writing restored DB ({:?}): {}", out_db_path, e))?;

    let conn = rusqlite::Connection::open(out_db_path)
        .map_err(|e| format!("Restored DB open failed: {}", e))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .map_err(|e| format!("Restored DB PRAGMA integrity_check failed: {}", e))?;
    if !integrity.trim().eq_ignore_ascii_case("ok") {
        return Err(format!("Restored DB integrity check failed: {}", integrity));
    }

    Ok(sha256_hex_bytes(&db_bytes))
}

fn s3_is_configured(config: &AppConfig) -> bool {
    !config.s3_backup_endpoint.trim().is_empty()
        && !config.s3_backup_bucket.trim().is_empty()
        && !config.s3_backup_access_key.trim().is_empty()
        && !config.s3_backup_secret_key.trim().is_empty()
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts arbitrary key lengths");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// Uploads an encrypted backup archive to an S3-compatible endpoint using AWS SigV4 `PUT`.
pub async fn upload_to_s3_sigv4(
    config: &AppConfig,
    object_name: &str,
    body: Vec<u8>,
) -> Result<String, String> {
    let endpoint = config.s3_backup_endpoint.trim().trim_end_matches('/');
    let bucket = config.s3_backup_bucket.trim().trim_matches('/');
    let prefix = config.s3_backup_prefix.trim().trim_matches('/');
    let region = if config.s3_backup_region.trim().is_empty() {
        "eu-central-1"
    } else {
        config.s3_backup_region.trim()
    };

    let object_key = if prefix.is_empty() {
        object_name.trim_start_matches('/').to_string()
    } else {
        format!("{}/{}", prefix, object_name.trim_start_matches('/'))
    };

    let url = format!("{}/{}/{}", endpoint, bucket, object_key);
    let without_scheme = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        .ok_or_else(|| format!("Invalid S3_BACKUP_ENDPOINT URL scheme: {}", endpoint))?;
    let (host, endpoint_base_path) = match without_scheme.split_once('/') {
        Some((h, rest)) => (h, format!("/{}", rest.trim_matches('/'))),
        None => (without_scheme, String::new()),
    };
    let canonical_uri = if endpoint_base_path.is_empty() || endpoint_base_path == "/" {
        format!("/{}/{}", bucket, object_key)
    } else {
        format!("{}/{}/{}", endpoint_base_path, bucket, object_key)
    };

    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let payload_hash = sha256_hex_bytes(&body);

    let canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, amz_date
    );
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = format!(
        "PUT\n{}\n\n{}\n{}\n{}",
        canonical_uri, canonical_headers, signed_headers, payload_hash
    );
    let canonical_request_hash = sha256_hex_bytes(canonical_request.as_bytes());

    let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date, credential_scope, canonical_request_hash
    );

    let k_date = hmac_sha256(
        format!("AWS4{}", config.s3_backup_secret_key.trim()).as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, b"s3");
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        config.s3_backup_access_key.trim(),
        credential_scope,
        signed_headers,
        signature
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed building S3 HTTP client: {}", e))?;

    let resp = client
        .put(&url)
        .header("content-type", "application/octet-stream")
        .header("x-amz-date", &amz_date)
        .header("x-amz-content-sha256", &payload_hash)
        .header("authorization", authorization)
        .body(body)
        .send()
        .await
        .map_err(|e| format!("S3 PUT request failed ({}): {}", url, e))?;

    if !resp.status().is_success() {
        let st = resp.status();
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("S3 PUT returned HTTP {}: {}", st, err_body));
    }

    Ok(object_key)
}

fn load_offsite_state(archive_dir: &str) -> OffsiteState {
    let state_path = format!("{}/offsite_state.json", archive_dir.trim_end_matches('/'));
    std::fs::read_to_string(&state_path)
        .ok()
        .and_then(|s| serde_json::from_str::<OffsiteState>(&s).ok())
        .unwrap_or_default()
}

fn save_offsite_state(archive_dir: &str, state: &OffsiteState) -> Result<(), String> {
    let state_path = format!("{}/offsite_state.json", archive_dir.trim_end_matches('/'));
    let json_txt =
        serde_json::to_string_pretty(state).map_err(|e| format!("State serialize error: {}", e))?;
    std::fs::write(&state_path, json_txt)
        .map_err(|e| format!("Failed writing offsite_state.json: {}", e))
}

fn snapshot_sqlite_pages(db: &Database, out_tmp_path: &str) -> Result<(Vec<u8>, usize), String> {
    let src_conn = db
        .open_conn()
        .map_err(|e| format!("Kaynak DB bağlantı hatası: {}", e))?;
    if Path::new(out_tmp_path).exists() {
        let _ = std::fs::remove_file(out_tmp_path);
    }
    let mut dst_conn = rusqlite::Connection::open(out_tmp_path)
        .map_err(|e| format!("Snapshot hedef DB açılamadı: {}", e))?;
    {
        let backup = rusqlite::backup::Backup::new(&src_conn, &mut dst_conn)
            .map_err(|e| format!("SQLite online backup başlatılamadı: {}", e))?;
        backup
            .run_to_completion(128, Duration::from_millis(5), None)
            .map_err(|e| format!("SQLite page-level backup başarısız: {}", e))?;
    }
    let integrity: String = dst_conn
        .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
        .map_err(|e| format!("Snapshot integrity_check başarısız: {}", e))?;
    if !integrity.trim().eq_ignore_ascii_case("ok") {
        drop(dst_conn);
        let _ = std::fs::remove_file(out_tmp_path);
        return Err(format!("Snapshot integrity_check failed: {}", integrity));
    }
    let page_size: i64 = dst_conn
        .query_row("PRAGMA page_size;", [], |r| r.get(0))
        .unwrap_or(DEFAULT_PAGE_SIZE as i64);
    drop(dst_conn);
    drop(src_conn);

    let bytes = std::fs::read(out_tmp_path)
        .map_err(|e| format!("Snapshot bytes read failed: {}", e))?;
    Ok((bytes, page_size.max(512) as usize))
}

pub fn prune_expired_offsite_archives(
    archive_dir: &str,
    retention_days: u32,
    preserve_base_filename: Option<&str>,
) -> usize {
    let Ok(entries) = std::fs::read_dir(archive_dir) else {
        return 0;
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days as u64 * 86_400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut pruned = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !fname.ends_with(".enc") {
            continue;
        }
        if preserve_base_filename == Some(fname) {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff && std::fs::remove_file(&path).is_ok() {
            pruned += 1;
            let sidecar = format!("{}.json", path.to_string_lossy());
            if Path::new(&sidecar).exists() {
                let _ = std::fs::remove_file(&sidecar);
            }
        }
    }
    pruned
}

/// Performs a WAL-safe SQLite Online Backup (`VACUUM INTO`), verifies `PRAGMA integrity_check == "ok"`,
/// computes SHA-256 checksum, writes `.db.json` metadata sidecar, and prunes expired
/// automated backups older than `retention_days` (preserving `pre_deploy_*`).
pub async fn perform_sqlite_backup(
    db: &Database,
    backup_dir: &str,
    retention_days: u32,
    prefix: &str,
    created_by: &str,
) -> Result<BackupResult, String> {
    tokio::fs::create_dir_all(backup_dir)
        .await
        .map_err(|e| format!("Yedek dizini oluşturulamadı ({}): {}", backup_dir, e))?;

    let stamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{}_{}.db", prefix.trim(), stamp);
    let tmp_path = format!("{}/{}.tmp", backup_dir.trim_end_matches('/'), filename);
    let final_path = format!("{}/{}", backup_dir.trim_end_matches('/'), filename);
    let source_db_path = db.db_path.clone();
    let pool = db.pool.clone();

    let tmp_clone = tmp_path.clone();
    let final_clone = final_path.clone();
    let (size_bytes, sha256, integrity_check) = tokio::task::spawn_blocking(move || {
        let src_conn = pool.get().map_err(|e| format!("Kaynak DB bağlantı hatası: {}", e))?;
        if Path::new(&tmp_clone).exists() {
            let _ = std::fs::remove_file(&tmp_clone);
        }
        src_conn
            .execute("VACUUM INTO ?1", rusqlite::params![&tmp_clone])
            .map_err(|e| format!("SQLite online backup (VACUUM INTO) başarısız: {}", e))?;
        drop(src_conn);

        let dst_conn = rusqlite::Connection::open(&tmp_clone)
            .map_err(|e| format!("Hedef yedek DB açılamadı: {}", e))?;
        let integrity: String = dst_conn
            .query_row("PRAGMA integrity_check;", [], |r| r.get(0))
            .map_err(|e| format!("PRAGMA integrity_check başarısız: {}", e))?;
        if !integrity.trim().eq_ignore_ascii_case("ok") {
            let _ = std::fs::remove_file(&tmp_clone);
            return Err(format!("Yedek bütünlük doğrulaması başarısız: {}", integrity));
        }
        drop(dst_conn);

        let bytes = std::fs::read(&tmp_clone)
            .map_err(|e| format!("Yedek dosyası okunamadı: {}", e))?;
        let sha_hex = sha256_hex_bytes(&bytes);
        let len = bytes.len() as u64;

        std::fs::rename(&tmp_clone, &final_clone)
            .map_err(|e| format!("Yedek dosyası taşınamadı: {}", e))?;

        Ok::<(u64, String, String), String>((len, sha_hex, integrity))
    })
    .await
    .map_err(|e| format!("Yedekleme görevi hatası: {}", e))??;

    let meta_json = serde_json::json!({
        "filename": filename,
        "source_db": source_db_path,
        "created_at": Utc::now().to_rfc3339(),
        "size_bytes": size_bytes,
        "sha256": sha256,
        "integrity_check": integrity_check,
        "created_by": created_by,
    });
    let meta_path = format!("{}.json", final_path);
    let _ = tokio::fs::write(
        &meta_path,
        serde_json::to_string_pretty(&meta_json).unwrap_or_else(|_| meta_json.to_string()),
    )
    .await;

    let pruned_count = prune_expired_backups(backup_dir, retention_days);

    Ok(BackupResult {
        filename,
        path: final_path,
        size_bytes,
        sha256,
        integrity_check,
        pruned_count,
        encrypted_archive_path: None,
        encrypted_sha256: None,
        s3_uploaded: false,
        s3_key: None,
    })
}

/// Performs the standard local online backup PLUS creates an AES-256-GCM encrypted off-site
/// full archive (`MSIBKP01`), updates the page-aligned `offsite_base.db` reference for future
/// `RPO < 1h` incremental snapshots, and optionally replicates to S3 via AWS SigV4 `PUT`.
pub async fn perform_full_backup_with_offsite(
    db: &Database,
    config: &AppConfig,
    prefix: &str,
    created_by: &str,
) -> Result<BackupResult, String> {
    let backup_dir = if !config.backup_dir.trim().is_empty() {
        config.backup_dir.clone()
    } else {
        format!(
            "{}/backups",
            Path::new(&config.sqlite_db_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| "storage/db".to_string())
        )
    };
    let archive_dir = if !config.backup_archive_dir.trim().is_empty() {
        config.backup_archive_dir.clone()
    } else {
        format!("{}/offsite", backup_dir.trim_end_matches('/'))
    };

    let mut res = perform_sqlite_backup(
        db,
        &backup_dir,
        config.backup_retention_days,
        prefix,
        created_by,
    )
    .await?;

    tokio::fs::create_dir_all(&archive_dir)
        .await
        .map_err(|e| format!("Off-site yedek dizini oluşturulamadı ({}): {}", archive_dir, e))?;

    let db_clone = db.clone();
    let archive_clone = archive_dir.clone();
    let key = derive_backup_key(&config.backup_encryption_key, &config.session_secret);
    let created_by_owned = created_by.to_string();

    let (enc_filename, enc_path, enc_sha, enc_bytes) = tokio::task::spawn_blocking(move || {
        let base_tmp = format!("{}/.offsite_base.tmp.db", archive_clone.trim_end_matches('/'));
        let base_final = format!("{}/offsite_base.db", archive_clone.trim_end_matches('/'));
        let (snap_bytes, _page_size) = snapshot_sqlite_pages(&db_clone, &base_tmp)?;
        let snap_sha = sha256_hex_bytes(&snap_bytes);

        std::fs::rename(&base_tmp, &base_final)
            .map_err(|e| format!("Base snapshot move failed: {}", e))?;

        let enc_bytes = encrypt_envelope(KIND_FULL_DB, &snap_bytes, &key)?;
        let enc_sha = sha256_hex_bytes(&enc_bytes);
        let stamp = Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
        let enc_filename = format!("msi_full_{}.db.gz.enc", stamp);
        let enc_path = format!("{}/{}", archive_clone.trim_end_matches('/'), enc_filename);
        std::fs::write(&enc_path, &enc_bytes)
            .map_err(|e| format!("Failed writing encrypted full backup: {}", e))?;

        let manifest = serde_json::json!({
            "kind": "full_db",
            "archive_filename": enc_filename,
            "encrypted_sha256": enc_sha,
            "plain_db_sha256": snap_sha,
            "plain_db_bytes": snap_bytes.len(),
            "encrypted_bytes": enc_bytes.len(),
            "created_at": Utc::now().to_rfc3339(),
            "created_by": created_by_owned,
        });
        let _ = std::fs::write(
            format!("{}.json", enc_path),
            serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| manifest.to_string()),
        );

        let state = OffsiteState {
            last_base_filename: Some(enc_filename.clone()),
            last_base_sha256: Some(snap_sha.clone()),
            last_snapshot_sha256: Some(snap_sha),
            updated_at: Some(Utc::now().to_rfc3339()),
        };
        save_offsite_state(&archive_clone, &state)?;

        Ok::<(String, String, String, Vec<u8>), String>((enc_filename, enc_path, enc_sha, enc_bytes))
    })
    .await
    .map_err(|e| format!("Encrypted off-site archive task error: {}", e))??;

    prune_expired_offsite_archives(
        &archive_dir,
        config.backup_retention_days,
        Some(&enc_filename),
    );

    res.encrypted_archive_path = Some(enc_path);
    res.encrypted_sha256 = Some(enc_sha);

    if s3_is_configured(config) {
        match upload_to_s3_sigv4(config, &enc_filename, enc_bytes).await {
            Ok(key) => {
                res.s3_uploaded = true;
                res.s3_key = Some(key);
            }
            Err(e) => {
                warn!("S3 off-site upload failed (local encrypted archive preserved): {}", e);
            }
        }
    }

    Ok(res)
}

/// Performs a 4KB page-level incremental snapshot (`RPO < 1h`) using `rusqlite::backup::Backup`,
/// compresses & encrypts the delta (`MSIDELTA1` inside `MSIBKP01`) or full snapshot if base is
/// absent / >60% changed, persists locally under `backup_archive_dir`, and uploads to S3 if configured.
/// Returns `Ok(None)` if the database has not changed since the last off-site snapshot.
pub async fn perform_incremental_snapshot(
    db: &Database,
    config: &AppConfig,
) -> Result<Option<IncrementalBackupResult>, String> {
    let archive_dir = if !config.backup_archive_dir.trim().is_empty() {
        config.backup_archive_dir.clone()
    } else {
        format!("{}/offsite", config.backup_dir.trim_end_matches('/'))
    };
    tokio::fs::create_dir_all(&archive_dir)
        .await
        .map_err(|e| format!("Off-site yedek dizini oluşturulamadı ({}): {}", archive_dir, e))?;

    let db_clone = db.clone();
    let archive_clone = archive_dir.clone();
    let key = derive_backup_key(&config.backup_encryption_key, &config.session_secret);
    let retention_days = config.backup_retention_days;

    let work = tokio::task::spawn_blocking(move || {
        let tmp_snap = format!("{}/.incr_snapshot.tmp.db", archive_clone.trim_end_matches('/'));
        let (target_bytes, page_size) = snapshot_sqlite_pages(&db_clone, &tmp_snap)?;
        let target_sha = sha256_hex_bytes(&target_bytes);

        let mut state = load_offsite_state(&archive_clone);
        if state.last_snapshot_sha256.as_deref() == Some(target_sha.as_str()) {
            let _ = std::fs::remove_file(&tmp_snap);
            return Ok::<Option<(IncrementalBackupResult, Vec<u8>)>, String>(None);
        }

        let base_path = format!("{}/offsite_base.db", archive_clone.trim_end_matches('/'));
        let base_bytes_opt = if Path::new(&base_path).exists() && state.last_base_filename.is_some() {
            std::fs::read(&base_path).ok()
        } else {
            None
        };

        let stamp = Utc::now().format("%Y%m%d_%H%M%S_%3f").to_string();
        let total_pages = target_bytes.len().div_ceil(page_size.max(512));

        let (kind_str, enc_filename, enc_bytes, changed_pages) = if let Some(base_bytes) = base_bytes_opt {
            let (delta_payload, changed_count, _) =
                encode_page_delta(&base_bytes, &target_bytes, page_size)?;
            if changed_count * page_size <= (target_bytes.len() * 60 / 100) {
                let enc = encrypt_envelope(KIND_PAGE_DELTA, &delta_payload, &key)?;
                let fname = format!("msi_incr_{}.delta.gz.enc", stamp);
                let _ = std::fs::remove_file(&tmp_snap);
                ("incremental_delta".to_string(), fname, enc, changed_count)
            } else {
                let enc = encrypt_envelope(KIND_FULL_DB, &target_bytes, &key)?;
                let fname = format!("msi_full_{}.db.gz.enc", stamp);
                std::fs::rename(&tmp_snap, &base_path)
                    .map_err(|e| format!("Failed refreshing offsite base DB: {}", e))?;
                state.last_base_filename = Some(fname.clone());
                state.last_base_sha256 = Some(target_sha.clone());
                ("full_snapshot".to_string(), fname, enc, total_pages)
            }
        } else {
            let enc = encrypt_envelope(KIND_FULL_DB, &target_bytes, &key)?;
            let fname = format!("msi_full_{}.db.gz.enc", stamp);
            std::fs::rename(&tmp_snap, &base_path)
                .map_err(|e| format!("Failed creating offsite base DB: {}", e))?;
            state.last_base_filename = Some(fname.clone());
            state.last_base_sha256 = Some(target_sha.clone());
            ("full_snapshot".to_string(), fname, enc, total_pages)
        };

        let enc_sha = sha256_hex_bytes(&enc_bytes);
        let enc_path = format!("{}/{}", archive_clone.trim_end_matches('/'), enc_filename);
        std::fs::write(&enc_path, &enc_bytes)
            .map_err(|e| format!("Failed writing encrypted incremental archive: {}", e))?;

        let manifest = serde_json::json!({
            "kind": kind_str,
            "archive_filename": enc_filename,
            "base_archive": state.last_base_filename,
            "base_sha256": state.last_base_sha256,
            "target_db_sha256": target_sha,
            "encrypted_sha256": enc_sha,
            "changed_pages": changed_pages,
            "total_pages": total_pages,
            "created_at": Utc::now().to_rfc3339(),
        });
        let _ = std::fs::write(
            format!("{}.json", enc_path),
            serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| manifest.to_string()),
        );

        state.last_snapshot_sha256 = Some(target_sha.clone());
        state.updated_at = Some(Utc::now().to_rfc3339());
        save_offsite_state(&archive_clone, &state)?;
        prune_expired_offsite_archives(
            &archive_clone,
            retention_days,
            state.last_base_filename.as_deref(),
        );

        Ok(Some((
            IncrementalBackupResult {
                kind: kind_str,
                archive_filename: enc_filename,
                archive_path: enc_path,
                encrypted_sha256: enc_sha,
                target_db_sha256: target_sha,
                changed_pages,
                total_pages,
                s3_uploaded: false,
                s3_key: None,
            },
            enc_bytes,
        )))
    })
    .await
    .map_err(|e| format!("Incremental snapshot task error: {}", e))??;

    let Some((mut result, enc_bytes)) = work else {
        return Ok(None);
    };

    if s3_is_configured(config) {
        match upload_to_s3_sigv4(config, &result.archive_filename, enc_bytes).await {
            Ok(key) => {
                result.s3_uploaded = true;
                result.s3_key = Some(key);
            }
            Err(e) => {
                warn!("S3 incremental upload failed (local encrypted archive preserved): {}", e);
            }
        }
    }

    Ok(Some(result))
}

pub fn prune_expired_backups(backup_dir: &str, retention_days: u32) -> usize {
    let Ok(entries) = std::fs::read_dir(backup_dir) else {
        return 0;
    };
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(retention_days as u64 * 86_400))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut pruned = 0usize;
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if fname.starts_with("pre_deploy_") || !fname.ends_with(".db") || !fname.starts_with("msi_backup_") {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff && std::fs::remove_file(&path).is_ok() {
            pruned += 1;
            let sidecar = format!("{}.json", path.to_string_lossy());
            if Path::new(&sidecar).exists() {
                let _ = std::fs::remove_file(&sidecar);
            }
        }
    }
    pruned
}

fn duration_until_next_utc_hour(target_hour: u32, target_min: u32) -> Duration {
    let now = Utc::now();
    let current_secs = now.num_seconds_from_midnight() as i64;
    let target_time = NaiveTime::from_hms_opt(target_hour, target_min, 0)
        .unwrap_or_else(|| NaiveTime::from_hms_opt(2, 0, 0).unwrap());
    let target_secs = target_time.num_seconds_from_midnight() as i64;
    let delta = if target_secs > current_secs {
        target_secs - current_secs
    } else {
        86_400 - current_secs + target_secs
    };
    Duration::from_secs(delta.max(60) as u64)
}

pub fn spawn_backup_worker(state: AppState, cancel: CancellationToken) {
    if !state.config.backup_enabled {
        return;
    }
    tokio::spawn(async move {
        let incr_secs = state.config.backup_incremental_interval_secs.clamp(60, 3540);
        let mut incr_ticker = tokio::time::interval(Duration::from_secs(incr_secs));
        incr_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Consume the immediate t=0 tick so incremental snapshots fire every `incr_secs` (RPO < 1h).
        incr_ticker.tick().await;

        let mut next_daily_sleep = Box::pin(tokio::time::sleep(duration_until_next_utc_hour(2, 0)));

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("DRP backup worker shutting down gracefully");
                    return;
                }
                _ = &mut next_daily_sleep => {
                    match perform_full_backup_with_offsite(
                        &state.db,
                        &state.config,
                        "msi_backup",
                        "backup_worker",
                    )
                    .await
                    {
                        Ok(res) => {
                            info!(
                                "DRP full backup completed: {} ({} bytes, sha256={}, offsite={:?}, s3={})",
                                res.filename, res.size_bytes, res.sha256, res.encrypted_archive_path, res.s3_uploaded
                            );
                        }
                        Err(e) => {
                            warn!("DRP full backup worker error: {}", e);
                        }
                    }
                    next_daily_sleep = Box::pin(tokio::time::sleep(duration_until_next_utc_hour(2, 0)));
                }
                _ = incr_ticker.tick() => {
                    match perform_incremental_snapshot(&state.db, &state.config).await {
                        Ok(Some(res)) => {
                            info!(
                                "DRP incremental snapshot completed: {} ({}, {}/{} pages, s3={})",
                                res.archive_filename, res.kind, res.changed_pages, res.total_pages, res.s3_uploaded
                            );
                        }
                        Ok(None) => {}
                        Err(e) => {
                            warn!("DRP incremental snapshot error: {}", e);
                        }
                    }
                }
            }
        }
    });
}
