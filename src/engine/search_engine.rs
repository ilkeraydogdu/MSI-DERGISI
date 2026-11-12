use crate::models::{
    CategoryNode, ContentItem, FacetBucket, MagazineIssue, SearchFacetResponse, SearchFilterQuery,
    SsbInterestTag,
};
use std::time::Instant;

pub struct CentralSearchEngine;

impl CentralSearchEngine {
    fn fold_turkish_ascii(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        for ch in input.chars() {
            match ch {
                'İ' | 'I' | 'ı' | 'i' => out.push('i'),
                'Ş' | 'ş' => out.push('s'),
                'Ğ' | 'ğ' => out.push('g'),
                'Ü' | 'ü' => out.push('u'),
                'Ö' | 'ö' => out.push('o'),
                'Ç' | 'ç' => out.push('c'),
                _ => {
                    for lower in ch.to_lowercase() {
                        out.push(lower);
                    }
                }
            }
        }
        out
    }

    pub fn item_matches_category(item: &ContentItem, cat_slug: &str) -> bool {
        let slug = cat_slug.trim();
        if slug.is_empty() || slug.eq_ignore_ascii_case("all") {
            return true;
        }
        if item.content_type.eq_ignore_ascii_case(slug) {
            return true;
        }
        if item
            .placements
            .iter()
            .any(|p| p.eq_ignore_ascii_case(slug))
        {
            return true;
        }
        if slug.eq_ignore_ascii_case("gunluk-haberler") && item.source_group == "gunluk-haber" {
            return true;
        }
        if slug.eq_ignore_ascii_case("dergi-icerikleri") && item.source_group == "dergi-icerigi" {
            return true;
        }
        if slug.eq_ignore_ascii_case("multimedya")
            && (item.content_type == "video" || item.video_url.is_some())
        {
            return true;
        }
        false
    }

    fn item_matches_tokens(item: &ContentItem, tokens: &[String], folded_tokens: &[String]) -> bool {
        if tokens.is_empty() {
            return true;
        }

        let raw_haystack = format!(
            "{} {} {} {} {} {} {} {} {}",
            item.title_tr,
            item.title_en,
            item.summary_tr,
            item.summary_en,
            item.body_tr,
            item.body_en,
            item.author_name,
            item.placements.join(" "),
            item.ssb_tags.join(" ")
        )
        .to_lowercase();

        let folded_haystack = Self::fold_turkish_ascii(&raw_haystack);

        for (tok, f_tok) in tokens.iter().zip(folded_tokens.iter()) {
            if !raw_haystack.contains(tok.as_str()) && !folded_haystack.contains(f_tok.as_str()) {
                return false;
            }
        }
        true
    }

    /// Executes multi-dimensional faceted search over the Single Source of Truth content pool with microsecond telemetry.
    pub fn execute_faceted_search(
        query: &SearchFilterQuery,
        all_contents: &[ContentItem],
        categories: &[CategoryNode],
        ssb_tags: &[SsbInterestTag],
        issues: &[MagazineIssue],
    ) -> SearchFacetResponse {
        let start = Instant::now();

        let raw_q = query.q.as_deref().unwrap_or("").trim();
        let tokens: Vec<String> = raw_q
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_lowercase())
            .collect();
        let folded_tokens: Vec<String> = tokens
            .iter()
            .map(|t| Self::fold_turkish_ascii(t))
            .collect();

        let cat_filter = query
            .category
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));
        let ssb_filter = query
            .ssb_tag
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));
        let type_filter = query
            .content_type
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));
        let source_filter = query
            .source_group
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("all"));
        let issue_filter = query.issue_number.filter(|&n| n > 0);
        let vis_filter = query.visibility_level.filter(|&v| v >= 0);
        let sponsored_filter = query.is_sponsored;

        // Helper closure to evaluate item against active filters with optional dimension skip for disjunctive facets
        let matches_with_skip = |item: &ContentItem, skip_dim: &str| -> bool {
            if !Self::item_matches_tokens(item, &tokens, &folded_tokens) {
                return false;
            }
            if skip_dim != "category" {
                if let Some(cat) = cat_filter {
                    if !Self::item_matches_category(item, cat) {
                        return false;
                    }
                }
            }
            if skip_dim != "ssb" {
                if let Some(tag) = ssb_filter {
                    if !item.ssb_tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                        return false;
                    }
                }
            }
            if skip_dim != "content_type" {
                if let Some(ct) = type_filter {
                    if !item.content_type.eq_ignore_ascii_case(ct)
                        && !item.placements.iter().any(|p| p.eq_ignore_ascii_case(ct))
                    {
                        return false;
                    }
                }
            }
            if skip_dim != "source" {
                if let Some(sg) = source_filter {
                    if !item.source_group.eq_ignore_ascii_case(sg) {
                        return false;
                    }
                }
            }
            if skip_dim != "issue" {
                if let Some(iss_no) = issue_filter {
                    if item.issue_number != Some(iss_no) {
                        return false;
                    }
                }
            }
            if let Some(vis) = vis_filter {
                if item.visibility_level != vis {
                    return false;
                }
            }
            if let Some(sp) = sponsored_filter {
                if item.is_sponsored != sp {
                    return false;
                }
            }
            true
        };

        // 1. Filter matching items
        let mut matched_items: Vec<ContentItem> = all_contents
            .iter()
            .filter(|item| matches_with_skip(item, ""))
            .cloned()
            .collect();

        // 2. Sort matched items
        let sort_mode = query.sort_by.as_deref().unwrap_or("newest");
        match sort_mode {
            "most_read" | "views" => {
                matched_items.sort_by(|a, b| {
                    b.view_count
                        .cmp(&a.view_count)
                        .then_with(|| b.id.cmp(&a.id))
                });
            }
            "true_read_rank" | "true_reads" | "engagement" => {
                matched_items.sort_by(|a, b| {
                    b.true_read_count
                        .cmp(&a.true_read_count)
                        .then_with(|| b.view_count.cmp(&a.view_count))
                });
            }
            "read_time" | "in_depth" => {
                matched_items.sort_by(|a, b| {
                    b.read_time_min
                        .cmp(&a.read_time_min)
                        .then_with(|| b.true_read_count.cmp(&a.true_read_count))
                });
            }
            _ => {
                matched_items.sort_by(|a, b| {
                    b.published_at
                        .cmp(&a.published_at)
                        .then_with(|| b.id.cmp(&a.id))
                });
            }
        }

        let total_matches = matched_items.len();
        if let Some(limit) = query.limit {
            if limit > 0 && matched_items.len() > limit {
                matched_items.truncate(limit);
            }
        }

        // 3. Compute exact live facet counts
        // Category Facets
        let category_facets: Vec<FacetBucket> = categories
            .iter()
            .filter(|c| c.group_type == "icerikler" || c.group_type == "gruplandirilmis")
            .map(|c| {
                let count = all_contents
                    .iter()
                    .filter(|item| {
                        matches_with_skip(item, "category")
                            && Self::item_matches_category(item, &c.slug)
                    })
                    .count();
                FacetBucket {
                    code: c.slug.clone(),
                    label_tr: c.name_tr.clone(),
                    label_en: c.name_en.clone(),
                    count,
                }
            })
            .collect();

        // SSB Catalog Interest Tag Facets
        let ssb_facets: Vec<FacetBucket> = ssb_tags
            .iter()
            .map(|tag| {
                let count = all_contents
                    .iter()
                    .filter(|item| {
                        matches_with_skip(item, "ssb")
                            && item
                                .ssb_tags
                                .iter()
                                .any(|t| t.eq_ignore_ascii_case(&tag.code))
                    })
                    .count();
                FacetBucket {
                    code: tag.code.clone(),
                    label_tr: tag.name_tr.clone(),
                    label_en: tag.name_en.clone(),
                    count,
                }
            })
            .collect();

        // Magazine Issue Facets
        let issue_facets: Vec<FacetBucket> = issues
            .iter()
            .map(|iss| {
                let count = all_contents
                    .iter()
                    .filter(|item| {
                        matches_with_skip(item, "issue")
                            && item.issue_number == Some(iss.issue_number)
                    })
                    .count();
                FacetBucket {
                    code: iss.issue_number.to_string(),
                    label_tr: format!("Sayı {} ({} {})", iss.issue_number, iss.publish_month_tr, iss.year),
                    label_en: format!("Issue {} ({} {})", iss.issue_number, iss.publish_month_en, iss.year),
                    count,
                }
            })
            .collect();

        // Source Group Facets
        let source_defs = [
            (
                "dergi-icerigi",
                "Dergi İçerikleri (MSI Dergisi / TDR)",
                "Magazine Issues (MSI Dergisi / TDR)",
            ),
            (
                "gunluk-haber",
                "Günlük Haberler (savunmahaber.com)",
                "Daily Defense News (savunmahaber.com)",
            ),
            (
                "sadece-online",
                "Sadece Online Özel Dosya",
                "Online Exclusive Analysis",
            ),
        ];
        let source_facets: Vec<FacetBucket> = source_defs
            .iter()
            .map(|(code, tr, en)| {
                let count = all_contents
                    .iter()
                    .filter(|item| {
                        matches_with_skip(item, "source")
                            && item.source_group.eq_ignore_ascii_case(code)
                    })
                    .count();
                FacetBucket {
                    code: (*code).to_string(),
                    label_tr: (*tr).to_string(),
                    label_en: (*en).to_string(),
                    count,
                }
            })
            .collect();

        let execution_time_us = start.elapsed().as_micros().max(1);

        SearchFacetResponse {
            query: query.clone(),
            total_matches,
            execution_time_us,
            items: matched_items,
            category_facets,
            ssb_facets,
            issue_facets,
            source_facets,
        }
    }
}

pub fn execute_faceted_search(
    query: &SearchFilterQuery,
    all_contents: &[ContentItem],
    categories: &[CategoryNode],
    ssb_tags: &[SsbInterestTag],
    issues: &[MagazineIssue],
) -> SearchFacetResponse {
    CentralSearchEngine::execute_faceted_search(query, all_contents, categories, ssb_tags, issues)
}
