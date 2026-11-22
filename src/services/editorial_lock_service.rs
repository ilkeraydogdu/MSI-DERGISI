//! In-memory editorial article lock registry (`LOCK_TTL_MS = 45_000`),
//! partitioned by `db_path` so concurrent integration tests remain isolated.

use crate::editorial::common::ArticleLock;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

pub const LOCK_TTL_MS: i64 = 45_000;

static LOCKS: Lazy<RwLock<HashMap<String, HashMap<String, ArticleLock>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn prune_expired(bucket: &mut HashMap<String, ArticleLock>, cutoff: i64) {
    bucket.retain(|_, lock| lock.updated_at_ms >= cutoff);
}

/// Lists all active non-expired article locks for a given database instance.
pub fn list_locks(db_path: &str) -> Vec<ArticleLock> {
    let cutoff = now_ms().saturating_sub(LOCK_TTL_MS);
    let mut guard = LOCKS.write().unwrap_or_else(|p| p.into_inner());
    let Some(bucket) = guard.get_mut(db_path) else {
        return Vec::new();
    };
    prune_expired(bucket, cutoff);
    bucket.values().cloned().collect()
}

/// Attempts to acquire or refresh a lock for `article_id` by `(user_id, username)`.
/// Returns `Ok(lock)` when granted/refreshed, or `Err(conflicting_lock)` if held by another user.
pub fn acquire_lock(
    db_path: &str,
    article_id: &str,
    user_id: i32,
    username: &str,
) -> Result<ArticleLock, ArticleLock> {
    let now = now_ms();
    let cutoff = now.saturating_sub(LOCK_TTL_MS);
    let clean_id = article_id.trim().to_string();

    let mut guard = LOCKS.write().unwrap_or_else(|p| p.into_inner());
    let bucket = guard.entry(db_path.to_string()).or_default();
    prune_expired(bucket, cutoff);

    if let Some(existing) = bucket.get(&clean_id) {
        if existing.user_id != user_id {
            return Err(existing.clone());
        }
    }

    let next = ArticleLock {
        article_id: clean_id.clone(),
        user_id,
        username: username.to_string(),
        updated_at_ms: now,
    };
    bucket.insert(clean_id, next.clone());
    Ok(next)
}

/// Releases an article lock if held by `user_id` or expired. Returns `Err(holder)` if held by another active editor.
pub fn release_lock(
    db_path: &str,
    article_id: &str,
    user_id: i32,
) -> Result<(), ArticleLock> {
    let cutoff = now_ms().saturating_sub(LOCK_TTL_MS);
    let clean_id = article_id.trim();

    let mut guard = LOCKS.write().unwrap_or_else(|p| p.into_inner());
    let Some(bucket) = guard.get_mut(db_path) else {
        return Ok(());
    };
    prune_expired(bucket, cutoff);

    if let Some(existing) = bucket.get(clean_id) {
        if existing.user_id != user_id {
            return Err(existing.clone());
        }
    }
    bucket.remove(clean_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_per_db_lock_ownership_and_release() {
        let db_a = "mem://lock-test-a";
        let db_b = "mem://lock-test-b";

        let l1 = acquire_lock(db_a, "42", 1, "Editor One").expect("first lock succeeds");
        assert_eq!(l1.user_id, 1);

        // Same user refreshes lock cleanly
        assert!(acquire_lock(db_a, "42", 1, "Editor One").is_ok());

        // Different user on same db is rejected
        let conflict = acquire_lock(db_a, "42", 2, "Editor Two").unwrap_err();
        assert_eq!(conflict.user_id, 1);

        // Isolated db_path is unaffected
        assert!(acquire_lock(db_b, "42", 2, "Editor Two").is_ok());

        // Non-owner cannot release
        assert!(release_lock(db_a, "42", 2).is_err());

        // Owner releases cleanly, freeing for Editor Two
        assert!(release_lock(db_a, "42", 1).is_ok());
        assert!(acquire_lock(db_a, "42", 2, "Editor Two").is_ok());
        let _ = release_lock(db_a, "42", 2);
        let _ = release_lock(db_b, "42", 2);
    }
}
