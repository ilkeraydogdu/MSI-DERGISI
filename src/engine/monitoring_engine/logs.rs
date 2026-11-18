use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use super::types::*;

// ─── Log Parsers ────────────────────────────────────────────────────────

/// Parses an Nginx combined log timestamp (e.g. `17/Nov/2026:19:30:14 +0300`) into `DateTime<Utc>`.
static NGINX_ACCESS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^(\S+)\s+\S+\s+\S+\s+\[([^\]]+)\]\s+"(\S+)\s+(\S+)\s+[^"]*"\s+(\d{3})\s+(\d+)\s+"[^"]*"\s+"([^"]*)""#,
    )
    .expect("valid static nginx access log regex")
});

static FAIL2BAN_BAN_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2}).*?\[([^\]]+)\]\s+(Ban|Found|Unban)\s+(\S+)"#,
    )
    .expect("valid static fail2ban regex")
});

pub(super) static LEGACY_GMT_DATE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"([A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}:\d{2}\s+\d{4})\s+GMT")
        .expect("valid static GMT date regex")
});


pub fn parse_nginx_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    let trimmed = ts.trim();
    if let Ok(dt) = DateTime::parse_from_str(trimmed, "%d/%b/%Y:%H:%M:%S %z") {
        return Some(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    None
}

/// Parse last `max_lines` lines of an Nginx/Plesk access log file.
pub fn parse_nginx_access_log_tail(path: &str, max_lines: usize) -> Vec<NginxAccessEntry> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let file_size = metadata.len();
    if file_size == 0 {
        return Vec::new();
    }

    let read_size = std::cmp::min(file_size, (max_lines as u64) * 512);
    let mut file = file;
    let _ = file.seek(SeekFrom::End(-(read_size as i64)));

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = lines.len().saturating_sub(max_lines);
    let mut entries = Vec::with_capacity(lines.len() - start);

    for line in &lines[start..] {
        if let Some(caps) = NGINX_ACCESS_RE.captures(line) {
            entries.push(NginxAccessEntry {
                ip: caps[1].to_string(),
                timestamp: caps[2].to_string(),
                method: caps[3].to_string(),
                path: caps[4].to_string(),
                status: caps[5].parse().unwrap_or(0),
                bytes: caps[6].parse().unwrap_or(0),
                user_agent: caps[7].to_string(),
            });
        }
    }
    entries
}

/// Parse fail2ban log tail for `Ban` / `Found` / `Unban` events.
pub fn parse_fail2ban_log_tail(path: &str, max_lines: usize) -> Vec<Fail2banEvent> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let metadata = match file.metadata() {
        Ok(m) => m,
        Err(_) => return Vec::new(),
    };

    let file_size = metadata.len();
    if file_size == 0 {
        return Vec::new();
    }

    let read_size = std::cmp::min(file_size, (max_lines as u64) * 300);
    let mut file = file;
    let _ = file.seek(SeekFrom::End(-(read_size as i64)));

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    let start = lines.len().saturating_sub(max_lines);
    let mut events = Vec::new();

    for line in &lines[start..] {
        if let Some(caps) = FAIL2BAN_BAN_RE.captures(line) {
            events.push(Fail2banEvent {
                jail: caps[2].to_string(),
                ip: caps[4].to_string(),
                action: caps[3].to_string(),
                timestamp: caps[1].to_string(),
            });
        }
    }
    events
}

// ─── Vhost Discovery ────────────────────────────────────────────────────

pub(super) fn pick_best_log_path(candidates: &[String]) -> String {
    // Prefer an existing non-empty log file first (e.g. Plesk's active proxy_access_ssl_log)
    for c in candidates {
        if let Ok(meta) = fs::metadata(c) {
            if meta.is_file() && meta.len() > 0 {
                return c.clone();
            }
        }
    }
    for c in candidates {
        if Path::new(c).is_file() {
            return c.clone();
        }
    }
    String::new()
}

