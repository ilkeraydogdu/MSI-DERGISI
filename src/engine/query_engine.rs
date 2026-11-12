use crate::db::{Database, L1CacheSnapshot};
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub struct QueryTelemetry<T> {
    pub data: T,
    pub latency_us: u128,
    pub cache_hit: bool,
}

/// Central Query Engine (`CentralQueryEngine`)
/// Provides a single, unified, sub-millisecond query gateway backed by SQLite WAL + L1 Read-Through Memory Cache.
pub struct CentralQueryEngine;

impl CentralQueryEngine {
    /// Returns a complete read-consistent snapshot of all core platform entities in < 10 microseconds.
    pub fn snapshot(db: &Database) -> QueryTelemetry<L1CacheSnapshot> {
        let start = Instant::now();
        if let Ok(guard) = db.l1_cache.read() {
            if let Some(ref snap) = *guard {
                let cloned = snap.clone();
                let latency_us = start.elapsed().as_micros().max(1);
                return QueryTelemetry {
                    data: cloned,
                    latency_us,
                    cache_hit: true,
                };
            }
        }
        db.refresh_l1_cache();
        let snap = db
            .l1_cache
            .read()
            .ok()
            .and_then(|g| g.clone())
            .unwrap_or_default();
        let latency_us = start.elapsed().as_micros().max(1);
        QueryTelemetry {
            data: snap,
            latency_us,
            cache_hit: false,
        }
    }

    /// Executes any read closure against the centralized `Database` while measuring exact microsecond latency.
    #[allow(dead_code)]
    pub fn execute_timed<F, T>(db: &Database, op: F) -> QueryTelemetry<T>
    where
        F: FnOnce(&Database) -> T,
    {
        let start = Instant::now();
        let cache_hit = db
            .l1_cache
            .read()
            .map(|g| g.is_some())
            .unwrap_or(false);
        let data = op(db);
        let latency_us = start.elapsed().as_micros().max(1);
        QueryTelemetry {
            data,
            latency_us,
            cache_hit,
        }
    }
}
