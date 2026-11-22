use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use tracing::warn;

use crate::models::{ContentItem, RecommendationRule, UserAccount};

/// Common Turkish and English military, academic, and corporate abbreviations that contain periods
/// and must NOT be treated as sentence boundaries during 2-sentence paywall truncation.
const PROTECTED_ABBREVIATIONS: &[&str] = &[
    "K.K.K.",
    "Dz.K.K.",
    "Hv.K.K.",
    "J.G.K.",
    "S.G.K.",
    "T.S.K.",
    "M.S.B.",
    "S.S.B.",
    "A.Ş.",
    "A.S.",
    "Ltd.",
    "Şti.",
    "Sti.",
    "Prof.",
    "Doç.",
    "Dr.",
    "Müh.",
    "Org.",
    "Korg.",
    "Tümam.",
    "Tümg.",
    "Tuğg.",
    "Tuğa.",
    "Alb.",
    "Yb.",
    "Bnb.",
    "Yzb.",
    "Ütğm.",
    "Tğm.",
    "Gen.",
    "Col.",
    "Maj.",
    "Capt.",
    "Inc.",
    "Corp.",
    "U.S.",
    "vb.",
    "vs.",
    "No.",
    "Say.",
];

/// Returns `true` when a `ContentItem` is publicly visible:
/// editorial status is `"published"` (or legacy empty) AND `published_at` is not in the future.
pub fn is_publicly_visible(item: &ContentItem) -> bool {
    if !(item.status.is_empty() || item.status == "published") {
        return false;
    }
    let raw = item.published_at.trim();
    if raw.is_empty() {
        return true;
    }
    let now_tr = Utc::now() + chrono::Duration::hours(3);
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return dt.with_timezone(&Utc) <= Utc::now();
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M"))
        .or_else(|_| NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M"))
    {
        return ndt <= now_tr.naive_utc();
    }
    if let Ok(d) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return d <= now_tr.date_naive();
    }
    true
}

/// Parses flexible `subscription_expires_at` formats (`%Y-%m-%d`, RFC3339, `%Y-%m-%d %H:%M:%S`,
/// `%Y-%m-%dT%H:%M:%S`, `%Y-%m-%d %H:%M`) and returns `true` if the timestamp has expired.
/// Date-only strings like `"2030-12-31"` remain valid through the end of that UTC calendar day.
fn is_subscription_expired(expires_str: &str, user_id: i64) -> bool {
    let s = expires_str.trim();
    if s.is_empty() {
        return false;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return dt.with_timezone(&Utc) < Utc::now();
    }
    if let Ok(ndt) = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M"))
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M"))
    {
        return ndt.and_utc() < Utc::now();
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return d < Utc::now().date_naive();
    }
    let prefix = crate::services::ad_service::char_prefix(s, 10);
    if prefix.chars().count() >= 10 {
        if let Ok(d) = NaiveDate::parse_from_str(prefix, "%Y-%m-%d") {
            return d < Utc::now().date_naive();
        }
    }

    warn!(
        "user_has_subscriber_access: unparseable subscription_expires_at '{}' for user id={}; treating as expired",
        s, user_id
    );
    true
}

/// Evaluates whether the given `UserAccount` has an active subscriber-level entitlement:
/// 1. Editorial & administrative staff roles always have full access (`return true` first).
/// 2. If `subscription_expires_at` is set and non-empty, expired dates immediately deny subscriber access.
/// 3. Verified public-sector personnel (`is_gov_verified`) or active subscriber roles/tiers receive access.
pub fn user_has_subscriber_access(user: &UserAccount) -> bool {
    let role_lower = user.role.trim().to_lowercase();

    // 1. Staff & editorial roles always retain full access regardless of subscription expiry
    if matches!(
        role_lower.as_str(),
        "site_operator"
            | "admin"
            | "super_admin"
            | "operator"
            | "editor"
            | "editor_in_chief"
            | "managing_editor"
            | "content_editor"
            | "content_publisher"
    ) {
        return true;
    }

    // 2. Enforce subscription expiration date across all date/datetime formats
    if let Some(ref expires_str) = user.subscription_expires_at {
        if !expires_str.trim().is_empty() && is_subscription_expired(expires_str, user.id) {
            return false;
        }
    }

    // 3. Verified .gov.tr / .tsk.tr public-sector subscribers
    if user.is_gov_verified {
        return true;
    }

    if matches!(
        role_lower.as_str(),
        "subscriber"
            | "subscriber_public"
            | "subscriber_corporate"
            | "subscriber_digital"
            | "subscriber_print"
            | "subscriber_print_digital"
            | "corporate"
    ) {
        return true;
    }

    let tier_lower = user.subscription_tier.trim().to_lowercase();
    if matches!(
        tier_lower.as_str(),
        "" | "free" | "standart kayıtlı okur" | "akademik / öğrenci okur" | "akademik okur"
    ) {
        return false;
    }

    tier_lower.contains("abone")
        || tier_lower.contains("subscriber")
        || tier_lower.contains("kamu")
        || tier_lower.contains("kurumsal")
        || tier_lower.contains("e-dergi")
        || tier_lower.contains("basili")
        || tier_lower.contains("basılı")
        || tier_lower.contains("vip")
        || tier_lower.contains("premium")
}

/// Applies the 3-tier paywall visibility rule to a `ContentItem`:
/// - `visibility_level == 0` (or `is_sponsored == true`): Open to all readers (`is_locked = false`).
/// - `visibility_level == 1`: Open to registered users and subscribers (`user.is_some()`).
/// - `visibility_level >= 2`: Open only to active subscribers OR readers who purchased single-article access (`purchased_codes`).
///
/// When locked, truncates `body_tr` / `body_en` to **strictly the first 2 sentences** wrapped in valid HTML
/// and returns `(rendered_html, true, lock_reason)`.
pub fn apply_paywall_view(
    item: &ContentItem,
    user: Option<&UserAccount>,
    purchased_codes: &[String],
    lang: &str,
) -> (String, bool, String) {
    let is_en = lang.eq_ignore_ascii_case("en");
    let full_body = if is_en && !item.body_en.trim().is_empty() {
        item.body_en.clone()
    } else {
        item.body_tr.clone()
    };

    // Sponsored content or visibility level 0 is open to all visitors
    if item.visibility_level <= 0 || item.is_sponsored {
        return (full_body, false, String::new());
    }

    // Check if single article was explicitly purchased by this reader
    let id_code = format!("ARTICLE-{}", item.id);
    let id_raw = item.id.to_string();
    let has_purchased_single = purchased_codes.iter().any(|code| {
        code.eq_ignore_ascii_case(&item.slug)
            || code.eq_ignore_ascii_case(&id_code)
            || code == &id_raw
    });

    if has_purchased_single {
        return (full_body, false, String::new());
    }

    // Level 1: Any logged-in registered user or subscriber can view
    if item.visibility_level == 1 {
        if user.is_some() {
            return (full_body, false, String::new());
        }
        let preview_html = truncate_to_two_sentences_html(&full_body);
        let reason = if is_en {
            "This article is available exclusively to registered readers and subscribers. Please sign in or create a free account to continue reading.".to_string()
        } else {
            "Bu içerik kayıtlı kullanıcılar ve aboneler tarafından görüntülenebilir. Tamamını okumak için lütfen ücretsiz üye girişi yapınız.".to_string()
        };
        return (preview_html, true, reason);
    }

    // Level 2+: Only active subscribers or single-article buyers
    if let Some(u) = user {
        if user_has_subscriber_access(u) {
            return (full_body, false, String::new());
        }
    }

    let preview_html = truncate_to_two_sentences_html(&full_body);
    let reason = if is_en {
        format!(
            "Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({:.2} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.",
            item.single_price_try
        )
    } else {
        format!(
            "Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({:.2} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)",
            item.single_price_try
        )
    };

    (preview_html, true, reason)
}

fn turkish_lowercase(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            'İ' => out.push('i'),
            'I' => out.push('ı'),
            _ => {
                for lc in c.to_lowercase() {
                    out.push(lc);
                }
            }
        }
    }
    out
}

pub fn fold_eq_tr(a: &str, b: &str) -> bool {
    let a_t = a.trim();
    let b_t = b.trim();
    turkish_lowercase(a_t) == turkish_lowercase(b_t)
        || a_t.to_lowercase() == b_t.to_lowercase()
}

/// Enforces `B13` institution-exclusive content gating (`target_institutions_json`).
/// Returns `true` when `item.target_institutions` is empty OR when `user` is staff
/// OR belongs to one of the targeted institutions / categories.
pub fn user_matches_target_institutions(
    item: &ContentItem,
    user: Option<&UserAccount>,
    db: &crate::db::Database,
) -> bool {
    if item.target_institutions.is_empty() {
        return true;
    }
    let Some(u) = user else {
        return false;
    };
    if crate::extractors::operator_user::is_staff_role(db, &u.role) {
        return true;
    }
    let u_cat = u.organization_category.trim();
    let u_name = u.organization_name.trim();
    let u_role = u.role.trim();
    let is_verified_public = u.is_gov_verified || fold_eq_tr(u_role, "subscriber_public");
    let is_verified_corp = fold_eq_tr(u_role, "subscriber_corporate");
    let categories = db.list_institution_categories(false);

    let matches_category = |candidate: &str, cat: &crate::models::InstitutionCategory| -> bool {
        !candidate.is_empty()
            && (fold_eq_tr(candidate, &cat.code)
                || fold_eq_tr(candidate, &cat.legacy_value)
                || fold_eq_tr(candidate, &cat.label_tr)
                || fold_eq_tr(candidate, &cat.label_en))
    };

    for raw_target in &item.target_institutions {
        let t = raw_target.trim();
        if t.is_empty() {
            continue;
        }
        if t == "*" || fold_eq_tr(t, "all") {
            return true;
        }
        let matched_cat = categories.iter().find(|c| {
            fold_eq_tr(&c.code, t)
                || fold_eq_tr(&c.legacy_value, t)
                || fold_eq_tr(&c.label_tr, t)
                || fold_eq_tr(&c.label_en, t)
        });
        if let Some(cat) = matched_cat {
            let eligible = if cat.is_public_sector {
                is_verified_public
            } else if cat.requires_verification {
                is_verified_public || is_verified_corp
            } else {
                true
            };
            if eligible && (matches_category(u_cat, cat) || matches_category(u_name, cat)) {
                return true;
            }
        } else {
            let t_low = turkish_lowercase(t);
            if t_low.contains("kamu") || t_low.contains("tsk") || fold_eq_tr(t, "public_sector") {
                if is_verified_public {
                    return true;
                }
            } else if (is_verified_public || is_verified_corp)
                && ((!u_cat.is_empty() && fold_eq_tr(u_cat, t))
                    || (!u_name.is_empty() && fold_eq_tr(u_name, t)))
            {
                return true;
            }
        }
    }
    false
}

/// Returns `true` when `item` is marked sponsored or has an `advertorial`/`advertorials` content type.
pub fn is_sponsored_or_advertorial(item: &ContentItem) -> bool {
    let ct = item.content_type.trim();
    item.is_sponsored
        || ct.eq_ignore_ascii_case("advertorial")
        || ct.eq_ignore_ascii_case("advertorials")
}

/// Selects related articles for the article detail view.
///
/// **STRICT COMPETITOR PROTECTION RULE**:
/// If `current` is sponsored or advertorial, immediately returns an empty `Vec::new()`
/// ("Sponsorlu içerik gösterilirken içerik önerisi yapılmaz").
pub fn select_related_articles_authorized(
    current: &ContentItem,
    all_contents: &[ContentItem],
    rules: &[RecommendationRule],
    user: Option<&UserAccount>,
    db: &crate::db::Database,
    limit: usize,
) -> Vec<ContentItem> {
    let allowed: Vec<ContentItem> = all_contents
        .iter()
        .filter(|c| user_matches_target_institutions(c, user, db))
        .cloned()
        .collect();
    select_related_articles(current, &allowed, rules, user, limit)
}

pub fn select_related_articles(
    current: &ContentItem,
    all_contents: &[ContentItem],
    rules: &[RecommendationRule],
    user: Option<&UserAccount>,
    limit: usize,
) -> Vec<ContentItem> {
    if is_sponsored_or_advertorial(current) {
        return Vec::new();
    }

    let target_limit = limit.max(3);
    let user_institution = user
        .map(|u| u.organization_category.as_str())
        .unwrap_or("");

    let mut scored_candidates: Vec<(i64, i64, ContentItem)> = all_contents
        .iter()
        .filter(|candidate| {
            is_publicly_visible(candidate)
                && !is_sponsored_or_advertorial(candidate)
                && candidate.id != current.id
                && candidate.slug != current.slug
        })
        .map(|candidate| {
            let mut score: i64 = 0;

            // 1. Operator RecommendationRule matches (Institution + SSB Tag / Category)
            for rule in rules.iter().filter(|r| r.is_active) {
                if rule.recommended_content_slug == candidate.slug {
                    let source_matches = rule.source_tag_or_cat == "*"
                        || rule.source_tag_or_cat.eq_ignore_ascii_case("all")
                        || current
                            .ssb_tags
                            .iter()
                            .any(|t| t.eq_ignore_ascii_case(&rule.source_tag_or_cat))
                        || current
                            .placements
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&rule.source_tag_or_cat))
                        || current
                            .content_type
                            .eq_ignore_ascii_case(&rule.source_tag_or_cat);

                    let inst_matches = rule.target_institution.is_empty()
                        || rule.target_institution == "*"
                        || fold_eq_tr(&rule.target_institution, "all")
                        || (!user_institution.is_empty()
                            && fold_eq_tr(&rule.target_institution, user_institution));

                    if source_matches && inst_matches {
                        score += 120 + (rule.priority as i64 * 15);
                        if !user_institution.is_empty()
                            && fold_eq_tr(&rule.target_institution, user_institution)
                        {
                            score += 60;
                        }
                    }
                }
            }

            // 2. Shared SSB Catalog Interest Tags with current article
            for tag in &current.ssb_tags {
                if candidate
                    .ssb_tags
                    .iter()
                    .any(|ct| ct.eq_ignore_ascii_case(tag))
                {
                    score += 35;
                }
            }

            // 3. Reader's personal SSB interests match
            if let Some(u) = user {
                for interest in &u.ssb_interests {
                    if candidate
                        .ssb_tags
                        .iter()
                        .any(|ct| ct.eq_ignore_ascii_case(interest))
                    {
                        score += 25;
                    }
                }
            }

            // 4. Shared site placement categories
            for placement in &current.placements {
                if candidate
                    .placements
                    .iter()
                    .any(|cp| cp.eq_ignore_ascii_case(placement))
                {
                    score += 15;
                }
            }

            // 5. Same content_type or magazine issue
            if candidate.content_type == current.content_type {
                score += crate::db::constants::SEARCH_SCORE_CATEGORY_MATCH;
            }
            if current.issue_number.is_some() && candidate.issue_number == current.issue_number {
                score += crate::db::constants::SEARCH_SCORE_CATEGORY_MATCH;
            }

            // Prefer editorial content over sponsored advertorials in organic recommendations
            if !candidate.is_sponsored {
                score += 5;
            }

            (score, candidate.view_count, candidate.clone())
        })
        .collect();

    scored_candidates.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| b.2.id.cmp(&a.2.id))
    });

    scored_candidates
        .into_iter()
        .take(target_limit)
        .map(|(_, _, item)| item)
        .collect()
}

/// Extracts strictly the first 2 sentences from HTML or plain text and returns clean, valid HTML.
pub fn truncate_to_two_sentences_html(raw_html: &str) -> String {
    let plain = strip_html_tags(raw_html);
    let cleaned = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    if cleaned.is_empty() {
        return "<p class=\"paywall-excerpt\"></p>".to_string();
    }

    // Temporarily mask dots inside known military/corporate abbreviations
    let mut masked = cleaned;
    for abbr in PROTECTED_ABBREVIATIONS {
        let safe_token = abbr.replace('.', "\u{0007}");
        masked = masked.replace(abbr, &safe_token);
    }

    let chars: Vec<char> = masked.chars().collect();
    let mut sentence_count = 0usize;
    let mut end_char_idx = chars.len();

    for i in 0..chars.len() {
        let c = chars[i];
        if c == '.' || c == '!' || c == '?' {
            let prev_is_digit = i > 0 && chars[i - 1].is_ascii_digit();
            let next_is_digit = i + 1 < chars.len() && chars[i + 1].is_ascii_digit();
            if prev_is_digit && next_is_digit {
                continue;
            }

            let at_boundary = i + 1 == chars.len()
                || chars[i + 1].is_whitespace()
                || chars[i + 1] == '"'
                || chars[i + 1] == '\''
                || chars[i + 1] == '”';

            if at_boundary {
                sentence_count += 1;
                if sentence_count >= 2 {
                    end_char_idx = i + 1;
                    break;
                }
            }
        }
    }

    let excerpt: String = chars[..end_char_idx]
        .iter()
        .collect::<String>()
        .replace('\u{0007}', ".");

    format!("<p class=\"paywall-excerpt\">{}</p>", excerpt.trim())
}

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_user(role: &str, tier: &str, expires: Option<&str>) -> UserAccount {
        UserAccount {
            id: 1,
            email: "okur@aselsan.com.tr".to_string(),
            full_name: "Test Okur".to_string(),
            role: role.to_string(),
            organization_category: "Sektör Kuruluşları".to_string(),
            organization_name: "ASELSAN".to_string(),
            job_title: "Mühendis".to_string(),
            is_gov_verified: false,
            subscription_tier: tier.to_string(),
            subscription_expires_at: expires.map(|s| s.to_string()),
            reminders_halted: false,
            kvkk_consent_at: "2026-01-01".to_string(),
            kvkk_consent_ip: "127.0.0.1".to_string(),
            kvkk_consent_hash: "KVKK".to_string(),
            email_consent: true,
            ssb_interests: vec![],
            created_at: "2026-01-01".to_string(),
            email_verified: true,
            marketing_emails_opted_in: true,
            totp_enabled: false,
            tos_accepted_at: "2026-01-01T00:00:00Z".to_string(),
            tos_version: "2026-01".to_string(),
        }
    }

    #[test]
    fn date_only_future_expiry_allows_subscriber_and_past_denies() {
        let active = make_user("subscriber_digital", "Dijital E-Dergi Abonesi", Some("2030-12-31"));
        assert!(user_has_subscriber_access(&active));

        let expired = make_user("subscriber_digital", "Dijital E-Dergi Abonesi", Some("2020-01-01"));
        assert!(!user_has_subscriber_access(&expired));
    }

    #[test]
    fn staff_roles_retain_access_regardless_of_expiry() {
        let staff = make_user("editor_in_chief", "free", Some("2020-01-01"));
        assert!(user_has_subscriber_access(&staff));

        let op = make_user("site_operator", "Kurumsal Tam Yetki", Some("2030-12-31"));
        assert!(user_has_subscriber_access(&op));
    }

    #[test]
    fn target_institutions_enforces_specific_agency_boundaries_and_preserves_umbrellas() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir()
            .join(format!("msi_cs_inst_{}_{}.db", std::process::id(), ts))
            .to_string_lossy()
            .to_string();
        let db = crate::db::Database::new(&db_path).expect("test db");

        let mut navy_user = make_user("subscriber_public", "Kamu & TSK", Some("2030-12-31"));
        navy_user.is_gov_verified = true;
        navy_user.organization_category = "Dz.K.K.lığı".to_string();
        navy_user.organization_name = "Deniz Kuvvetleri Komutanlığı".to_string();

        let mut airforce_user_legacy = navy_user.clone();
        airforce_user_legacy.organization_category = "Hv.K.K.lığı".to_string();
        airforce_user_legacy.organization_name = "Hava Kuvvetleri Komutanlığı".to_string();

        let mut airforce_user_code = navy_user.clone();
        airforce_user_code.organization_category = "hvkk".to_string();

        let hvkk_item = ContentItem {
            id: 1,
            slug: "hvkk-only".to_string(),
            target_institutions: vec!["Hv.K.K.lığı".to_string()],
            ..ContentItem::default()
        };
        let diger_kamu_item = ContentItem {
            id: 2,
            slug: "diger-kamu-only".to_string(),
            target_institutions: vec!["Diğer Kamu Kurumları".to_string()],
            ..ContentItem::default()
        };
        let umbrella_kamu_tsk = ContentItem {
            id: 3,
            slug: "kamu-tsk-umbrella".to_string(),
            target_institutions: vec!["Kamu & TSK".to_string()],
            ..ContentItem::default()
        };
        let umbrella_kamu_ssb = ContentItem {
            id: 4,
            slug: "kamu-tsb-ssb-umbrella".to_string(),
            target_institutions: vec!["Kamu / TSK / SSB".to_string()],
            ..ContentItem::default()
        };
        let umbrella_public_sector = ContentItem {
            id: 5,
            slug: "public-sector-umbrella".to_string(),
            target_institutions: vec!["public_sector".to_string()],
            ..ContentItem::default()
        };

        assert!(
            !user_matches_target_institutions(&hvkk_item, Some(&navy_user), &db),
            "Dz.K.K.lığı .gov.tr user must NOT bypass Hv.K.K.lığı-only content"
        );
        assert!(
            !user_matches_target_institutions(&diger_kamu_item, Some(&navy_user), &db),
            "Dz.K.K.lığı .gov.tr user must NOT bypass Diğer Kamu Kurumları-only content"
        );
        assert!(user_matches_target_institutions(&hvkk_item, Some(&airforce_user_legacy), &db));
        assert!(user_matches_target_institutions(&hvkk_item, Some(&airforce_user_code), &db));
        assert!(user_matches_target_institutions(&umbrella_kamu_tsk, Some(&navy_user), &db));
        assert!(user_matches_target_institutions(&umbrella_kamu_ssb, Some(&navy_user), &db));
        assert!(user_matches_target_institutions(&umbrella_public_sector, Some(&navy_user), &db));

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn target_institutions_turkish_uppercase_preserves_agency_isolation_and_allows_legitimate_users() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir()
            .join(format!("msi_cs_tr_case_{}_{}.db", std::process::id(), ts))
            .to_string_lossy()
            .to_string();
        let db = crate::db::Database::new(&db_path).expect("test db");

        let mut navy_user = make_user("subscriber_public", "Kamu & TSK", Some("2030-12-31"));
        navy_user.is_gov_verified = true;
        navy_user.organization_category = "Dz.K.K.lığı".to_string();
        navy_user.organization_name = "Deniz Kuvvetleri Komutanlığı".to_string();

        let mut airforce_user = navy_user.clone();
        airforce_user.organization_category = "Hv.K.K.lığı".to_string();
        airforce_user.organization_name = "Hava Kuvvetleri Komutanlığı".to_string();

        let mut airforce_user_tr_upper = navy_user.clone();
        airforce_user_tr_upper.organization_category = "HV.K.K.LIĞI".to_string();

        let mut other_public_user = navy_user.clone();
        other_public_user.organization_category = "Diğer Kamu Kurumları".to_string();
        other_public_user.organization_name = "T.C. Ulaştırma Bakanlığı".to_string();

        let mut mit_user = navy_user.clone();
        mit_user.organization_category = "mit".to_string();

        let diger_kamu_upper_item = ContentItem {
            id: 101,
            slug: "diger-kamu-upper".to_string(),
            target_institutions: vec!["DİĞER KAMU KURUMLARI".to_string()],
            ..ContentItem::default()
        };
        let hvkk_upper_padded_item = ContentItem {
            id: 102,
            slug: "hvkk-upper-padded".to_string(),
            target_institutions: vec!["  HV.K.K.LIĞI  ".to_string()],
            ..ContentItem::default()
        };
        let hvkk_canonical_item = ContentItem {
            id: 103,
            slug: "hvkk-canonical".to_string(),
            target_institutions: vec!["Hv.K.K.lığı".to_string()],
            ..ContentItem::default()
        };
        let mit_upper_item = ContentItem {
            id: 104,
            slug: "mit-upper".to_string(),
            target_institutions: vec!["MİT".to_string()],
            ..ContentItem::default()
        };
        let other_public_en_upper_item = ContentItem {
            id: 105,
            slug: "other-public-en-upper".to_string(),
            target_institutions: vec!["OTHER PUBLIC INSTITUTIONS".to_string()],
            ..ContentItem::default()
        };

        // Cross-agency denial: Dz.K.K.lığı user MUST be denied for "DİĞER KAMU KURUMLARI"
        assert!(
            !user_matches_target_institutions(&diger_kamu_upper_item, Some(&navy_user), &db),
            "Dz.K.K.lığı .gov.tr user must NOT bypass DİĞER KAMU KURUMLARI-only content"
        );
        assert!(
            user_matches_target_institutions(&diger_kamu_upper_item, Some(&other_public_user), &db),
            "Diğer Kamu Kurumları user must match DİĞER KAMU KURUMLARI"
        );

        // Legitimate user allowance with Turkish uppercase & padding
        assert!(
            user_matches_target_institutions(&hvkk_upper_padded_item, Some(&airforce_user), &db),
            "Hv.K.K.lığı user must match padded uppercase '  HV.K.K.LIĞI  '"
        );
        assert!(
            !user_matches_target_institutions(&hvkk_upper_padded_item, Some(&navy_user), &db),
            "Dz.K.K.lığı user must NOT match '  HV.K.K.LIĞI  '"
        );
        assert!(
            user_matches_target_institutions(&hvkk_canonical_item, Some(&airforce_user_tr_upper), &db),
            "User with organization_category='HV.K.K.LIĞI' must match 'Hv.K.K.lığı'"
        );

        // MİT & English label uppercase parity
        assert!(user_matches_target_institutions(&mit_upper_item, Some(&mit_user), &db));
        assert!(!user_matches_target_institutions(&mit_upper_item, Some(&navy_user), &db));
        assert!(user_matches_target_institutions(&other_public_en_upper_item, Some(&other_public_user), &db));
        assert!(!user_matches_target_institutions(&other_public_en_upper_item, Some(&navy_user), &db));

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }

    #[test]
    fn select_related_articles_suppresses_and_excludes_advertorials_even_if_is_sponsored_false() {
        let advertorial_plural = ContentItem {
            id: 10,
            slug: "adv-plural".to_string(),
            content_type: "AdVeRtOrIaLs".to_string(),
            is_sponsored: false,
            status: "published".to_string(),
            ..ContentItem::default()
        };
        let regular = ContentItem {
            id: 11,
            slug: "regular-news".to_string(),
            content_type: "haberler".to_string(),
            is_sponsored: false,
            status: "published".to_string(),
            ..ContentItem::default()
        };
        let organic_candidate = ContentItem {
            id: 12,
            slug: "organic-cand".to_string(),
            content_type: "haberler".to_string(),
            is_sponsored: false,
            status: "published".to_string(),
            ..ContentItem::default()
        };

        let all = vec![advertorial_plural.clone(), regular.clone(), organic_candidate.clone()];
        assert!(select_related_articles(&advertorial_plural, &all, &[], None, 3).is_empty());

        let related_for_regular = select_related_articles(&regular, &all, &[], None, 3);
        assert!(related_for_regular.iter().any(|c| c.slug == "organic-cand"));
        assert!(
            !related_for_regular.iter().any(|c| c.slug == "adv-plural"),
            "advertorials candidate must be excluded from organic related recommendations"
        );
    }
}
