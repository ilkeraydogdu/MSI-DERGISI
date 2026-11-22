#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn refresh_l1_cache(&self) {
        let start = Instant::now();
        let categories = self.load_categories_db();
        let ssb_tags = self.load_ssb_tags_db();
        let contents = self.load_contents_db();
        let mut issues = self.load_issues_db();
        Self::enrich_issue_spreads_from_contents(&mut issues, &contents);
        let ads = self.load_ads_db();
        let site_settings = self.load_site_settings_db();
        let notifications = self.load_notifications_db();
        let seo_redirects = self.load_seo_redirects_db();
        let subscription_plans = self.load_subscription_plans_db();
        let commerce_products = self.load_commerce_products_db();
        let users = self.load_users_db();
        let institution_categories = self.load_institution_categories_db(false);
        let recommendation_rules = self.load_recommendation_rules_db();
        let elapsed_us = start.elapsed().as_micros().max(1);

        if let Ok(mut guard) = self.l1_cache.write() {
            *guard = Some(L1CacheSnapshot {
                categories,
                ssb_tags,
                contents,
                issues,
                ads,
                site_settings,
                notifications,
                seo_redirects,
                subscription_plans,
                commerce_products,
                users,
                institution_categories,
                recommendation_rules,
                last_refreshed_us: elapsed_us,
            });
        }
    }

    /// Selectively reload only the named L1 cache sections.
    /// Supported section names: "categories", "ssb_tags", "contents", "issues",
    /// "ads", "site_settings", "notifications", "seo_redirects",
    /// "subscription_plans", "commerce_products", "users",
    /// "institution_categories", "recommendation_rules".
    pub fn refresh_l1_sections(&self, sections: &[&str]) {
        // Clone current snapshot; if none exists, do a full refresh instead.
        let mut snap = match self.l1_cache.read() {
            Ok(guard) => match guard.as_ref() {
                Some(s) => s.clone(),
                None => {
                    drop(guard);
                    self.refresh_l1_cache();
                    return;
                }
            },
            Err(_) => {
                self.refresh_l1_cache();
                return;
            }
        };

        for section in sections {
            match *section {
                "categories" => snap.categories = self.load_categories_db(),
                "ssb_tags" => snap.ssb_tags = self.load_ssb_tags_db(),
                "contents" => {
                    snap.contents = self.load_contents_db();
                    // Issues depend on contents for spread enrichment
                    let mut issues = self.load_issues_db();
                    Self::enrich_issue_spreads_from_contents(&mut issues, &snap.contents);
                    snap.issues = issues;
                }
                "issues" => {
                    let mut issues = self.load_issues_db();
                    Self::enrich_issue_spreads_from_contents(&mut issues, &snap.contents);
                    snap.issues = issues;
                }
                "ads" => snap.ads = self.load_ads_db(),
                "site_settings" => snap.site_settings = self.load_site_settings_db(),
                "notifications" => snap.notifications = self.load_notifications_db(),
                "seo_redirects" => snap.seo_redirects = self.load_seo_redirects_db(),
                "subscription_plans" => snap.subscription_plans = self.load_subscription_plans_db(),
                "commerce_products" => snap.commerce_products = self.load_commerce_products_db(),
                "users" => snap.users = self.load_users_db(),
                "institution_categories" => {
                    snap.institution_categories = self.load_institution_categories_db(false)
                }
                "recommendation_rules" => {
                    snap.recommendation_rules = self.load_recommendation_rules_db()
                }
                _ => {}
            }
        }

        if let Ok(mut guard) = self.l1_cache.write() {
            *guard = Some(snap);
        }

        // Invalidate L2 content query cache when contents change
        if sections.iter().any(|s| *s == "contents") {
            crate::engine::content_query_engine::CentralContentQueryEngine::invalidate_cache();
        }
    }

    pub(super) fn enrich_issue_spreads_from_contents(
        issues: &mut [MagazineIssue],
        contents: &[ContentItem],
    ) {
        for issue in issues.iter_mut() {
            if issue.spreads.is_empty() {
                issue.spreads = contents
                    .iter()
                    .filter(|c| c.issue_number == Some(issue.issue_number))
                    .take(crate::db::constants::PDF_SPREADS_LIMIT)
                    .enumerate()
                    .map(|(idx, c)| {
                        let start_p = crate::db::constants::PDF_SPREAD_START_PAGE + idx * crate::db::constants::PDF_SPREAD_PAGE_INCREMENT;
                        let end_p = start_p + 6;
                        MagazinePageSpread {
                            page_no: format!("S. {:02}-{:02}", start_p, end_p),
                            title_tr: c.title_tr.clone(),
                            title_en: if c.title_en.is_empty() {
                                c.title_tr.clone()
                            } else {
                                c.title_en.clone()
                            },
                            category_badge: c.content_type.replace('-', " ").to_uppercase(),
                            image_url: c.cover_image.clone(),
                            excerpt_tr: c.summary_tr.clone(),
                            excerpt_en: if c.summary_en.is_empty() {
                                c.summary_tr.clone()
                            } else {
                                c.summary_en.clone()
                            },
                            linked_content_slug: Some(c.slug.clone()),
                        }
                    })
                    .collect();
            }
        }
    }

}
