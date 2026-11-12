use crate::db::Database;
use crate::engine::query_engine::CentralQueryEngine;
use crate::engine::search_engine::CentralSearchEngine;
use crate::models::{ContentItem, ContentQueryResult, SearchFilterQuery};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Thread-safe in-memory cache for frequent query results.
static QUERY_CACHE: once_cell::sync::Lazy<Arc<RwLock<HashMap<u64, CachedResult>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

const CACHE_MAX_ENTRIES: usize = 256;
const DEFAULT_PER_PAGE: usize = 20;
const MAX_PER_PAGE: usize = 100;

#[derive(Clone)]
struct CachedResult {
    result: ContentQueryResult,
    cached_at: Instant,
}

/// Central Content Query Engine — single entry point for ALL content search,
/// filtering, pagination, and sorting across the entire platform.
///
/// Replaces every scattered implementation in web_controller, api_controller,
/// and admin_view with one unified, cached, zero-duplication engine.
pub struct CentralContentQueryEngine;

impl CentralContentQueryEngine {
    /// Unified query entry point. Call this from ANY controller.
    ///
    /// # Parameters
    /// - `db`: Database reference (used to obtain L1 snapshot via CentralQueryEngine)
    /// - `query`: Unified filter/search/pagination/sort criteria
    ///
    /// # Returns
    /// `ContentQueryResult` with paginated items, metadata, and telemetry.
    pub fn execute(db: &Database, query: &SearchFilterQuery) -> ContentQueryResult {
        let start = Instant::now();

        // Normalize pagination params
        let per_page = query
            .per_page
            .unwrap_or(DEFAULT_PER_PAGE)
            .clamp(1, MAX_PER_PAGE);
        let page = query.page.unwrap_or(1).max(1);

        // Check cache
        let cache_key = Self::compute_cache_key(query, per_page, page);
        if let Ok(guard) = QUERY_CACHE.read() {
            if let Some(cached) = guard.get(&cache_key) {
                // Cache valid for 5 seconds
                if cached.cached_at.elapsed().as_secs() < 5 {
                    let mut result = cached.result.clone();
                    result.cache_hit = true;
                    result.execution_time_us = start.elapsed().as_micros().max(1);
                    return result;
                }
            }
        }

        // Get snapshot via CentralQueryEngine (L1 cache)
        let snap = CentralQueryEngine::snapshot(db);

        // Build full SearchFilterQuery for faceted search (reuse existing engine)
        let facet_query = SearchFilterQuery {
            q: query.q.clone(),
            category: query.category.clone(),
            ssb_tag: query.ssb_tag.clone(),
            content_type: query.content_type.clone(),
            source_group: query.source_group.clone(),
            issue_number: query.issue_number,
            visibility_level: query.visibility_level,
            is_sponsored: query.is_sponsored,
            sort_by: query.sort_by.clone(),
            lang: query.lang.clone(),
            limit: None, // No limit at facet stage; we paginate after
            page: None,
            per_page: None,
        };

        // Execute faceted search (existing CentralSearchEngine handles matching + sorting)
        let facet_res = CentralSearchEngine::execute_faceted_search(
            &facet_query,
            &snap.data.contents,
            &snap.data.categories,
            &snap.data.ssb_tags,
            &snap.data.issues,
            &snap.data.site_settings,
        );

        let total_count = facet_res.total_matches;
        let total_pages = (total_count.saturating_add(per_page - 1) / per_page).max(1);
        let clamped_page = page.min(total_pages);
        let start_idx = (clamped_page - 1) * per_page;
        let page_items: Vec<ContentItem> = facet_res
            .items
            .iter()
            .skip(start_idx)
            .take(per_page)
            .cloned()
            .collect();

        let execution_time_us = start.elapsed().as_micros().max(1);

        let result = ContentQueryResult {
            items: page_items,
            total_count,
            current_page: clamped_page,
            per_page,
            total_pages,
            filters_applied: query.clone(),
            execution_time_us,
            cache_hit: false,
        };

        // Store in cache (evict oldest if full)
        if let Ok(mut guard) = QUERY_CACHE.write() {
            if guard.len() >= CACHE_MAX_ENTRIES {
                // Simple eviction: clear half the cache
                let keys_to_remove: Vec<u64> = guard.keys().take(CACHE_MAX_ENTRIES / 2).copied().collect();
                for k in keys_to_remove {
                    guard.remove(&k);
                }
            }
            guard.insert(
                cache_key,
                CachedResult {
                    result: result.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        result
    }

    /// Convenience: build a SearchFilterQuery from raw HashMap params (e.g., Axum Query extractors).
    pub fn from_params(params: &HashMap<String, String>) -> SearchFilterQuery {
        SearchFilterQuery {
            q: params.get("q").filter(|s| !s.trim().is_empty()).cloned(),
            category: params
                .get("category")
                .filter(|s| !s.trim().is_empty())
                .cloned(),
            ssb_tag: params
                .get("ssb_tag")
                .or_else(|| params.get("ssb"))
                .filter(|s| !s.trim().is_empty())
                .cloned(),
            content_type: params
                .get("content_type")
                .filter(|s| !s.trim().is_empty())
                .cloned(),
            source_group: params
                .get("source_group")
                .filter(|s| !s.trim().is_empty())
                .cloned(),
            issue_number: params.get("issue").and_then(|s| s.parse::<i32>().ok()),
            visibility_level: params
                .get("visibility_max")
                .and_then(|s| s.parse::<i32>().ok()),
            is_sponsored: params
                .get("is_sponsored")
                .and_then(|s| s.parse::<bool>().ok()),
            sort_by: params.get("sort").cloned(),
            lang: params.get("lang").cloned(),
            limit: params.get("limit").and_then(|s| s.parse::<usize>().ok()),
            page: params.get("page").and_then(|s| s.parse::<usize>().ok()),
            per_page: params.get("per_page").and_then(|s| s.parse::<usize>().ok()),
        }
    }

    /// Compute a deterministic cache key from query parameters.
    fn compute_cache_key(query: &SearchFilterQuery, per_page: usize, page: usize) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.q.hash(&mut hasher);
        query.category.hash(&mut hasher);
        query.ssb_tag.hash(&mut hasher);
        query.content_type.hash(&mut hasher);
        query.source_group.hash(&mut hasher);
        query.issue_number.hash(&mut hasher);
        query.visibility_level.hash(&mut hasher);
        query.is_sponsored.hash(&mut hasher);
        query.sort_by.hash(&mut hasher);
        query.lang.hash(&mut hasher);
        per_page.hash(&mut hasher);
        page.hash(&mut hasher);
        hasher.finish()
    }

    /// Invalidate the query cache (call after content mutations).
    #[allow(dead_code)]
    pub fn invalidate_cache() {
        if let Ok(mut guard) = QUERY_CACHE.write() {
            guard.clear();
        }
    }
}