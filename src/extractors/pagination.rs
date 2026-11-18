use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use serde::Deserialize;

/// Standardized pagination parameters extracted from query string.
/// Defaults: page=1, per_page=25, max per_page=100.
#[derive(Clone, Debug, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn default_page() -> u32 {
    1
}
fn default_per_page() -> u32 {
    25
}

impl PaginationParams {
    /// Computed offset for SQL LIMIT/OFFSET queries.
    pub fn offset(&self) -> u32 {
        (self.page.max(1) - 1) * self.effective_per_page()
    }

    /// Clamped per_page value (1..=100).
    pub fn effective_per_page(&self) -> u32 {
        self.per_page.clamp(1, 100)
    }

    /// Build a meta object for API responses.
    pub fn meta(&self, total_count: u64) -> serde_json::Value {
        let per_page = self.effective_per_page();
        let total_pages = if total_count == 0 {
            0
        } else {
            ((total_count as f64) / (per_page as f64)).ceil() as u64
        };
        serde_json::json!({
            "page": self.page.max(1),
            "per_page": per_page,
            "total_count": total_count,
            "total_pages": total_pages
        })
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for PaginationParams
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or("");
        let mut params: PaginationParams =
            serde_urlencoded::from_str(query).unwrap_or(PaginationParams {
                page: 1,
                per_page: 25,
            });
        params.page = params.page.max(1);
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meta_clamps_zero_page_to_one() {
        let p = PaginationParams {
            page: 0,
            per_page: 25,
        };
        let meta = p.meta(60);
        assert_eq!(meta["page"], 1);
        assert_eq!(meta["per_page"], 25);
        assert_eq!(meta["total_count"], 60);
        assert_eq!(meta["total_pages"], 3);
        assert_eq!(p.offset(), 0);
    }
}
