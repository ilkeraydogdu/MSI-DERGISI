#[allow(unused_imports)]
use super::*;

impl Database {
    /// Sanitize a user search query into a safe FTS5 MATCH expression.
    /// Splits on whitespace, strips unsafe characters (keeping alphanumeric,
    /// underscore, and Turkish letters), wraps each token in quotes with '*'
    /// prefix-match suffix, and joins with OR. Returns empty string if no
    /// valid tokens remain.
    #[cfg(test)]
    pub(super) fn sanitize_fts_query(query: &str) -> String {
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|tok| {
                tok.chars()
                    .filter(|c| {
                        c.is_ascii_alphanumeric()
                            || *c == '_'
                            || matches!(*c,
                                'ç' | 'Ç' | 'ğ' | 'Ğ' | 'ı' | 'İ' |
                                'ö' | 'Ö' | 'ş' | 'Ş' | 'ü' | 'Ü' |
                                'â' | 'Â' | 'ê' | 'Ê' | 'î' | 'Î' |
                                'ô' | 'Ô' | 'û' | 'Û')
                    })
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"*", t))
            .collect();
        if tokens.is_empty() {
            String::new()
        } else {
            tokens.join(" OR ")
        }
    }

    pub fn search_contents(&self, query: &str) -> Vec<ContentItem> {
        let q = query.trim();
        if q.is_empty() {
            return self.get_all_contents();
        }

        // Try AND first for precision, fall back to OR
        let fts_and = Self::sanitize_fts_query_with_join(q, " AND ");
        let fts_or = Self::sanitize_fts_query_with_join(q, " OR ");

        let mut fts_ids: Vec<i64> = Vec::new();

        // Attempt AND query first
        if !fts_and.is_empty() {
            let sql = format!(
                "SELECT c.id FROM contents_fts f JOIN contents c ON c.id = f.content_id \
                 WHERE contents_fts MATCH ?1 ORDER BY rank LIMIT {}",
                crate::db::constants::SEARCH_MAX_RESULTS
            );
            let ids: Vec<i64> = crate::db::query_helpers::query_all(
                self, &sql, params![fts_and.as_str()], |r| r.get(0),
            );
            if !ids.is_empty() {
                fts_ids = ids;
            }
        }

        // Fall back to OR if AND yielded nothing
        if fts_ids.is_empty() && !fts_or.is_empty() {
            let sql = format!(
                "SELECT c.id FROM contents_fts f JOIN contents c ON c.id = f.content_id \
                 WHERE contents_fts MATCH ?1 ORDER BY rank LIMIT {}",
                crate::db::constants::SEARCH_MAX_RESULTS
            );
            fts_ids = crate::db::query_helpers::query_all(
                self, &sql, params![fts_or.as_str()], |r| r.get(0),
            );
        }

        // Hydrate from L1 cache (preserves placements/ssb_tags)
        let all_contents = self.get_all_contents();
        let q_lower = q.to_lowercase();

        // Collect FTS-matched items first (in rank order)
        let mut result: Vec<ContentItem> = Vec::new();
        let mut seen_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();

        for id in &fts_ids {
            if let Some(item) = all_contents.iter().find(|c| c.id == *id) {
                result.push(item.clone());
                seen_ids.insert(*id);
            }
        }

        // Union with legacy in-memory matches (author, ssb_tags, placements, slug)
        for item in &all_contents {
            if seen_ids.contains(&item.id) {
                continue;
            }
            if item.author_name.to_lowercase().contains(&q_lower)
                || item.ssb_tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
                || item.placements.iter().any(|p| p.to_lowercase().contains(&q_lower))
                || item.slug.to_lowercase().contains(&q_lower)
            {
                result.push(item.clone());
                seen_ids.insert(item.id);
            }
        }

        // If FTS returned nothing and legacy also nothing, fall back to full list
        if result.is_empty() {
            return all_contents;
        }

        result
    }

    /// Build an FTS query with a specific join operator (AND or OR).
    pub(super) fn sanitize_fts_query_with_join(query: &str, joiner: &str) -> String {
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|tok| {
                tok.chars()
                    .filter(|c| {
                        c.is_ascii_alphanumeric()
                            || *c == '_'
                            || matches!(*c,
                                'ç' | 'Ç' | 'ğ' | 'Ğ' | 'ı' | 'İ' |
                                'ö' | 'Ö' | 'ş' | 'Ş' | 'ü' | 'Ü' |
                                'â' | 'Â' | 'ê' | 'Ê' | 'î' | 'Î' |
                                'ô' | 'Ô' | 'û' | 'Û')
                    })
                    .collect::<String>()
            })
            .filter(|t| !t.is_empty())
            .map(|t| format!("\"{}\"*", t))
            .collect();
        if tokens.is_empty() {
            String::new()
        } else {
            tokens.join(joiner)
        }
    }

}

#[cfg(test)]
mod fts_tests {
    use super::*;

    #[test]
    fn test_sanitize_fts_query_basic() {
        let result = Database::sanitize_fts_query("hello world");
        assert_eq!(result, "\"hello\"* OR \"world\"*");
    }

    #[test]
    fn test_sanitize_fts_query_turkish_chars() {
        let result = Database::sanitize_fts_query("uçak gemisi");
        assert_eq!(result, "\"uçak\"* OR \"gemisi\"*");
    }

    #[test]
    fn test_sanitize_fts_query_strips_punctuation() {
        let result = Database::sanitize_fts_query("hello! @world# test...");
        assert_eq!(result, "\"hello\"* OR \"world\"* OR \"test\"*");
    }

    #[test]
    fn test_sanitize_fts_query_quotes_removed() {
        let result = Database::sanitize_fts_query("\"quoted\" 'single'");
        assert_eq!(result, "\"quoted\"* OR \"single\"*");
    }

    #[test]
    fn test_sanitize_fts_query_empty_input() {
        assert_eq!(Database::sanitize_fts_query(""), "");
        assert_eq!(Database::sanitize_fts_query("   "), "");
    }

    #[test]
    fn test_sanitize_fts_query_only_special_chars() {
        assert_eq!(Database::sanitize_fts_query("!@#$%^&*()"), "");
    }

    #[test]
    fn test_sanitize_fts_query_with_join_and() {
        let result = Database::sanitize_fts_query_with_join("foo bar baz", " AND ");
        assert_eq!(result, "\"foo\"* AND \"bar\"* AND \"baz\"*");
    }

    #[test]
    fn test_sanitize_fts_query_underscore_preserved() {
        let result = Database::sanitize_fts_query("my_test query");
        assert_eq!(result, "\"my_test\"* OR \"query\"*");
    }
}

