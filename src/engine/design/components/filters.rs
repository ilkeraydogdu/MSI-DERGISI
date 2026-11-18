use super::labels::category_display_label;
use crate::models::{ContentItem, MagazineIssue, MagazinePageSpread};

#[inline]
pub fn is_published_content(c: &ContentItem) -> bool {
    crate::services::content_service::is_publicly_visible(c)
}

pub fn is_redundant_excerpt(title: &str, excerpt: &str) -> bool {
    let t = title.trim().to_lowercase();
    let e = excerpt.trim().to_lowercase();
    e.is_empty()
        || e == t
        || (t.len() >= 18
            && e.starts_with(
                &t[..t
                    .char_indices()
                    .nth(18)
                    .map(|(i, _)| i)
                    .unwrap_or(t.len())],
            ))
        || e.contains("yeni sayısının içerikleri:")
}

/// Enriches an issue's spreads with non-duplicate excerpts and real SQLite `dergi-icerigi`
/// dossier records belonging to that issue when `spreads` has fewer than 4 entries.
pub fn enrich_issue_spreads(
    issue: &MagazineIssue,
    contents: &[ContentItem],
) -> Vec<MagazinePageSpread> {
    let limit = crate::db::constants::COMPONENTS_PREVIEW_SPREADS_LIMIT;
    let mut out = Vec::new();
    let mut used_slugs = std::collections::HashSet::new();

    for raw_sp in &issue.spreads {
        let mut sp = raw_sp.clone();
        if let Some(slug) = sp.linked_content_slug.as_deref() {
            if !slug.is_empty() {
                used_slugs.insert(slug.to_string());
            }
        }
        if is_redundant_excerpt(&sp.title_tr, &sp.excerpt_tr) {
            let linked_summary = sp
                .linked_content_slug
                .as_deref()
                .and_then(|slug| {
                    contents
                        .iter()
                        .find(|c| is_published_content(c) && c.slug == slug)
                })
                .map(|c| c.summary_tr.trim())
                .filter(|sum| !is_redundant_excerpt(&sp.title_tr, sum));

            let matched_editorial = if linked_summary.is_none() && sp.title_tr.contains("ARES") {
                contents
                    .iter()
                    .find(|c| {
                        is_published_content(c)
                            && c.source_group == "dergi-icerigi"
                            && c.title_tr.contains("ARES")
                            && !c.summary_tr.trim().is_empty()
                    })
                    .map(|c| (c.slug.as_str(), c.title_tr.trim(), c.summary_tr.trim()))
            } else {
                None
            };

            if let Some(sum) = linked_summary {
                sp.excerpt_tr = sum.to_string();
            } else if let Some((slug, full_title, sum)) = matched_editorial {
                if sp.title_tr.ends_with("...") {
                    sp.title_tr = full_title.to_string();
                }
                sp.excerpt_tr = sum.to_string();
                sp.linked_content_slug = Some(slug.to_string());
                used_slugs.insert(slug.to_string());
            } else if !is_redundant_excerpt(&sp.title_tr, &issue.summary_tr) {
                sp.excerpt_tr = issue.summary_tr.trim().to_string();
            } else {
                sp.excerpt_tr.clear();
            }
        }
        if is_redundant_excerpt(&sp.title_en, &sp.excerpt_en) {
            sp.excerpt_en.clear();
        }
        out.push(sp);
    }

    if out.len() < limit {
        let mut candidates: Vec<&ContentItem> = contents
            .iter()
            .filter(|c| {
                is_published_content(c)
                    && c.source_group == "dergi-icerigi"
                    && c.content_type != "yeni-sayi-kapak"
                    && !used_slugs.contains(&c.slug)
                    && (c.issue_number == Some(issue.issue_number)
                        || (c.issue_number.is_none()
                            && c.published_at.starts_with(&issue.year.to_string())))
            })
            .collect();
        candidates.sort_by(|a, b| b.published_at.cmp(&a.published_at).then(b.id.cmp(&a.id)));

        for c in candidates {
            if out.len() >= limit {
                break;
            }
            used_slugs.insert(c.slug.clone());
            let exc_tr = if is_redundant_excerpt(&c.title_tr, &c.summary_tr) {
                String::new()
            } else {
                c.summary_tr.trim().to_string()
            };
            let exc_en = if !c.summary_en.trim().is_empty()
                && !is_redundant_excerpt(&c.title_en, &c.summary_en)
            {
                c.summary_en.trim().to_string()
            } else {
                String::new()
            };
            out.push(MagazinePageSpread {
                page_no: format!("DOSYA {:02}", out.len() + 1),
                title_tr: c.title_tr.clone(),
                title_en: if c.title_en.is_empty() {
                    c.title_tr.clone()
                } else {
                    c.title_en.clone()
                },
                category_badge: category_display_label(&c.content_type, "tr").to_string(),
                image_url: c.cover_image.clone(),
                excerpt_tr: exc_tr,
                excerpt_en: exc_en,
                linked_content_slug: Some(c.slug.clone()),
            });
        }
    }

    out
}

