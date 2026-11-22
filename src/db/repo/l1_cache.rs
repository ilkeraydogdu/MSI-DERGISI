#[allow(unused_imports)]
use super::*;

impl Database {
    fn refresh_l1_cache_inner(&self) {
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

    pub fn refresh_l1_cache(&self) {
        let _refresh_guard = self.l1_refresh_lock.lock().unwrap_or_else(|e| e.into_inner());
        self.refresh_l1_cache_inner();
    }

    /// Selectively reload only the named L1 cache sections.
    /// Supported section names: "categories", "ssb_tags", "contents", "issues",
    /// "ads", "site_settings", "notifications", "seo_redirects",
    /// "subscription_plans", "commerce_products", "users",
    /// "institution_categories", "recommendation_rules".
    pub fn refresh_l1_sections(&self, sections: &[&str]) {
        let _refresh_guard = self.l1_refresh_lock.lock().unwrap_or_else(|e| e.into_inner());
        let cache_empty = self
            .l1_cache
            .read()
            .map(|g| g.is_none())
            .unwrap_or(true);
        if cache_empty {
            self.refresh_l1_cache_inner();
            return;
        }

        let mut categories = None;
        let mut ssb_tags = None;
        let mut contents = None;
        let mut issues = None;
        let mut ads = None;
        let mut site_settings = None;
        let mut notifications = None;
        let mut seo_redirects = None;
        let mut subscription_plans = None;
        let mut commerce_products = None;
        let mut users = None;
        let mut institution_categories = None;
        let mut recommendation_rules = None;

        for section in sections {
            match *section {
                "categories" => categories = Some(self.load_categories_db()),
                "ssb_tags" => ssb_tags = Some(self.load_ssb_tags_db()),
                "contents" => {
                    let loaded_contents = self.load_contents_db();
                    let mut loaded_issues = self.load_issues_db();
                    Self::enrich_issue_spreads_from_contents(&mut loaded_issues, &loaded_contents);
                    contents = Some(loaded_contents);
                    issues = Some(loaded_issues);
                }
                "issues" => {
                    let mut loaded_issues = self.load_issues_db();
                    if let Some(ref c) = contents {
                        Self::enrich_issue_spreads_from_contents(&mut loaded_issues, c);
                    } else if let Ok(guard) = self.l1_cache.read() {
                        if let Some(ref snap) = *guard {
                            Self::enrich_issue_spreads_from_contents(&mut loaded_issues, &snap.contents);
                        }
                    }
                    issues = Some(loaded_issues);
                }
                "ads" => ads = Some(self.load_ads_db()),
                "site_settings" => site_settings = Some(self.load_site_settings_db()),
                "notifications" => notifications = Some(self.load_notifications_db()),
                "seo_redirects" => seo_redirects = Some(self.load_seo_redirects_db()),
                "subscription_plans" => {
                    subscription_plans = Some(self.load_subscription_plans_db())
                }
                "commerce_products" => commerce_products = Some(self.load_commerce_products_db()),
                "users" => users = Some(self.load_users_db()),
                "institution_categories" => {
                    institution_categories = Some(self.load_institution_categories_db(false))
                }
                "recommendation_rules" => {
                    recommendation_rules = Some(self.load_recommendation_rules_db())
                }
                _ => {}
            }
        }

        if let Ok(mut guard) = self.l1_cache.write() {
            if let Some(ref mut snap) = *guard {
                if let Some(v) = categories {
                    snap.categories = v;
                }
                if let Some(v) = ssb_tags {
                    snap.ssb_tags = v;
                }
                if let Some(v) = contents {
                    snap.contents = v;
                }
                if let Some(v) = issues {
                    snap.issues = v;
                }
                if let Some(v) = ads {
                    snap.ads = v;
                }
                if let Some(v) = site_settings {
                    snap.site_settings = v;
                }
                if let Some(v) = notifications {
                    snap.notifications = v;
                }
                if let Some(v) = seo_redirects {
                    snap.seo_redirects = v;
                }
                if let Some(v) = subscription_plans {
                    snap.subscription_plans = v;
                }
                if let Some(v) = commerce_products {
                    snap.commerce_products = v;
                }
                if let Some(v) = users {
                    snap.users = v;
                }
                if let Some(v) = institution_categories {
                    snap.institution_categories = v;
                }
                if let Some(v) = recommendation_rules {
                    snap.recommendation_rules = v;
                }
            }
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
