use super::super::config::Section;
use super::labels::localized;
use super::PageData;
use super::filters::is_published_content;
use crate::models::ContentItem;
use std::collections::HashMap;

#[cfg(test)]
pub(super) fn select<'a>(s: &Section, data: &'a PageData<'_>) -> Vec<&'a ContentItem> {
    select_for_page(s, data, &mut std::collections::HashSet::new())
}

pub fn select_for_page<'a>(
    s: &Section,
    data: &'a PageData<'_>,
    seen_ids: &mut std::collections::HashSet<i64>,
) -> Vec<&'a ContentItem> {
    let mut base_items: Vec<_> = data
        .contents
        .iter()
        .filter(|c| {
            is_published_content(c)
                && (s.category == "yeni-sayi-kapak" || c.content_type != "yeni-sayi-kapak")
                && (s.source == "all"
                    || s.source == "magazine"
                        && (c.source_group == "dergi-icerigi"
                            || (data.lang == "en" && c.source_group == "msi-tdr"))
                    || s.source == "news" && c.source_group != "dergi-icerigi")
                && (s.category.is_empty()
                    || c.content_type == s.category
                    || c.placements.contains(&s.category)
                    || c.ssb_tags.contains(&s.category))
                && (data.lang != "tr" || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
        })
        .collect();
    if data.lang == "en" {
        base_items.retain(|c| super::is_en_content_ready(c));
    }
    let mut items = base_items;
    items.sort_by(|a, b| {
        b.published_at
            .cmp(&a.published_at)
            .then_with(|| {
                if data.lang == "en" {
                    b.source_group
                        .eq_ignore_ascii_case("msi-tdr")
                        .cmp(&a.source_group.eq_ignore_ascii_case("msi-tdr"))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then(b.id.cmp(&a.id))
    });
    if data.lang == "en" {
        // Prefer msi-tdr rows when identical English titles exist alongside bilingual TR rows
        let mut seen_titles: HashMap<String, usize> = HashMap::new();
        let mut deduped: Vec<&'a ContentItem> = Vec::with_capacity(items.len());
        for item in items {
            let key: String = localized(data.lang, &item.title_tr, &item.title_en)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if let Some(&idx) = seen_titles.get(&key) {
                if item.source_group.eq_ignore_ascii_case("msi-tdr")
                    && !deduped[idx].source_group.eq_ignore_ascii_case("msi-tdr")
                {
                    deduped[idx] = item;
                }
            } else {
                seen_titles.insert(key, deduped.len());
                deduped.push(item);
            }
        }
        items = deduped;
    }
    let chosen: Vec<&'a ContentItem> = items
        .into_iter()
        .skip(s.offset)
        .filter(|c| !seen_ids.contains(&c.id))
        .take(s.limit)
        .collect();
    for item in &chosen {
        seen_ids.insert(item.id);
    }
    chosen
}

