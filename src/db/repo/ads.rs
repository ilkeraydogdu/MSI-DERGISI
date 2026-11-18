#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_all_ads(&self) -> Vec<AdCampaign> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.ads.clone();
            }
        }
        self.load_ads_db()
    }

    pub(super) fn load_ads_db(&self) -> Vec<AdCampaign> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                    target_categories_json, target_ssb_tags_json, target_institutions_json,
                    impressions_count, clicks_count, is_active
             FROM ad_campaigns ORDER BY id ASC",
            [],
            |r| {
                let cat_raw: String = r.get(17)?;
                let ssb_raw: String = r.get(18)?;
                let inst_raw: String = r.get(19)?;
                Ok(AdCampaign {
                    id: r.get(0)?,
                    slot_code: r.get(1)?,
                    badge_label: r.get(2)?,
                    dimensions: r.get(3)?,
                    sponsor_name: r.get(4)?,
                    sponsor_type: r.get(5)?,
                    pill_style: r.get(6)?,
                    title_tr: r.get(7)?,
                    title_en: r.get(8)?,
                    desc_tr: r.get(9)?,
                    desc_en: r.get(10)?,
                    target_url: r.get(11)?,
                    image_url: r.get(12)?,
                    budget_score: r.get(13)?,
                    is_default_fallback: r.get::<_, i32>(14)? != 0,
                    is_floating_popup: r.get::<_, i32>(15)? != 0,
                    is_newsletter_ad: r.get::<_, i32>(16)? != 0,
                    target_categories: serde_json::from_str(&cat_raw).unwrap_or_default(),
                    target_ssb_tags: serde_json::from_str(&ssb_raw).unwrap_or_default(),
                    target_institutions: serde_json::from_str(&inst_raw).unwrap_or_default(),
                    impressions_count: r.get(20)?,
                    clicks_count: r.get(21)?,
                    is_active: r.get::<_, i32>(22)? != 0,
                })
            },
        )
    }

    pub fn upsert_ad(&self, ad: &AdCampaign) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let cat_json =
            serde_json::to_string(&ad.target_categories).unwrap_or_else(|_| "[]".to_string());
        let ssb_json =
            serde_json::to_string(&ad.target_ssb_tags).unwrap_or_else(|_| "[]".to_string());
        let inst_json =
            serde_json::to_string(&ad.target_institutions).unwrap_or_else(|_| "[]".to_string());

        let result_id = if ad.id > 0 {
            conn.execute(
                "UPDATE ad_campaigns SET
                    slot_code = ?1, badge_label = ?2, dimensions = ?3, sponsor_name = ?4,
                    sponsor_type = ?5, pill_style = ?6, title_tr = ?7, title_en = ?8,
                    desc_tr = ?9, desc_en = ?10, target_url = ?11, image_url = ?12,
                    budget_score = ?13, is_default_fallback = ?14, is_floating_popup = ?15,
                    is_newsletter_ad = ?16, target_categories_json = ?17, target_ssb_tags_json = ?18,
                    target_institutions_json = ?19, is_active = ?20
                 WHERE id = ?21",
                params![
                    ad.slot_code,
                    ad.badge_label,
                    ad.dimensions,
                    ad.sponsor_name,
                    ad.sponsor_type,
                    ad.pill_style,
                    ad.title_tr,
                    ad.title_en,
                    ad.desc_tr,
                    ad.desc_en,
                    ad.target_url,
                    ad.image_url,
                    ad.budget_score,
                    if ad.is_default_fallback { 1 } else { 0 },
                    if ad.is_floating_popup { 1 } else { 0 },
                    if ad.is_newsletter_ad { 1 } else { 0 },
                    cat_json,
                    ssb_json,
                    inst_json,
                    if ad.is_active { 1 } else { 0 },
                    ad.id
                ],
            )?;
            ad.id
        } else {
            conn.execute(
                "INSERT INTO ad_campaigns (
                    slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
                    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
                    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
                    target_categories_json, target_ssb_tags_json, target_institutions_json,
                    impressions_count, clicks_count, is_active
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16,
                    ?17, ?18, ?19,
                    ?20, ?21, ?22
                )",
                params![
                    ad.slot_code,
                    ad.badge_label,
                    ad.dimensions,
                    ad.sponsor_name,
                    ad.sponsor_type,
                    ad.pill_style,
                    ad.title_tr,
                    ad.title_en,
                    ad.desc_tr,
                    ad.desc_en,
                    ad.target_url,
                    ad.image_url,
                    ad.budget_score,
                    if ad.is_default_fallback { 1 } else { 0 },
                    if ad.is_floating_popup { 1 } else { 0 },
                    if ad.is_newsletter_ad { 1 } else { 0 },
                    cat_json,
                    ssb_json,
                    inst_json,
                    ad.impressions_count.max(1),
                    ad.clicks_count,
                    if ad.is_active { 1 } else { 0 }
                ],
            )?;
            conn.last_insert_rowid()
        };
        drop(conn);
        self.refresh_l1_sections(&["ads"]);
        Ok(result_id)
    }

    pub fn delete_ad_campaign(&self, id: i64) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute("DELETE FROM ad_campaigns WHERE id = ?1", params![id])?;
        drop(conn);
        self.refresh_l1_sections(&["ads"]);
        Ok(())
    }

    pub fn increment_ad_metric(&self, ad_id: i64, is_click: bool) {
        if let Ok(conn) = self.open_conn() {
            if is_click {
                let _ = conn.execute(
                    "UPDATE ad_campaigns SET clicks_count = clicks_count + 1 WHERE id = ?1",
                    params![ad_id],
                );
            } else {
                let _ = conn.execute(
                    "UPDATE ad_campaigns SET impressions_count = impressions_count + 1 WHERE id = ?1",
                    params![ad_id],
                );
            }
        }
        if let Ok(mut guard) = self.l1_cache.write() {
            if let Some(ref mut snap) = *guard {
                if let Some(ad) = snap.ads.iter_mut().find(|a| a.id == ad_id) {
                    if is_click {
                        ad.clicks_count += 1;
                    } else {
                        ad.impressions_count += 1;
                    }
                    return;
                }
            }
        }
        self.refresh_l1_cache();
    }

    pub fn get_recommendation_rules(&self) -> Vec<RecommendationRule> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority, is_active
             FROM recommendation_rules ORDER BY priority DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(RecommendationRule {
                id: r.get(0)?,
                rule_name: r.get(1)?,
                source_tag_or_cat: r.get(2)?,
                target_institution: r.get(3)?,
                recommended_content_slug: r.get(4)?,
                priority: r.get(5)?,
                is_active: r.get::<_, i32>(6)? != 0,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

}
