use serde::{Deserialize, Serialize};

use super::content::ContentItem;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchFilterQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub ssb_tag: Option<String>,
    pub content_type: Option<String>,
    pub source_group: Option<String>,
    pub issue_number: Option<i32>,
    pub visibility_level: Option<i32>,
    pub is_sponsored: Option<bool>,
    pub sort_by: Option<String>,
    pub lang: Option<String>,
    pub limit: Option<usize>,
    #[serde(default)]
    pub page: Option<usize>,
    #[serde(default)]
    pub per_page: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacetBucket {
    pub code: String,
    pub label_tr: String,
    pub label_en: String,
    pub count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchFacetResponse {
    pub query: SearchFilterQuery,
    pub total_matches: usize,
    pub execution_time_us: u128,
    pub items: Vec<ContentItem>,
    pub category_facets: Vec<FacetBucket>,
    pub ssb_facets: Vec<FacetBucket>,
    pub issue_facets: Vec<FacetBucket>,
    pub source_facets: Vec<FacetBucket>,
}