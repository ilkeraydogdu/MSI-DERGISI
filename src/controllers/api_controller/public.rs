use super::*;

// ============================================================================

pub async fn health_check(State(state): State<AppState>, headers: HeaderMap) -> Json<Value> {
    let contents = state.db.get_all_contents();
    let categories = state.db.get_all_categories();
    let ssb_tags = state.db.get_all_ssb_tags();
    let issues = state.db.get_all_issues();
    let ads = state.db.get_all_ads();
    let session_exists = auth_service::session_user(&state, &headers).is_some();

    Json(json!({
        "status": "ok",
        "platform": state.config.app_name,
        "env": state.config.app_env,
        "public_url": state.config.public_url,
        "authenticated": session_exists,
        "engine": "Rust Axum Fullstack Engine v2.6",
        "domains": {
            "primary": state.config.primary_domain,
            "news_portal": state.config.news_domain,
            "bilingual_locales": ["tr", "en"]
        },
        "counts": {
            "contents": contents.iter().filter(|c| content_service::is_publicly_visible(c)).count(),
            "categories": categories.len(),
            "ssb_tags": ssb_tags.len(),
            "magazine_issues": issues.len(),
            "ad_campaigns": ads.iter().filter(|a| a.is_active).count()
        }
    }))
}

pub async fn list_contents(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<ContentItem>> {
    let criteria = CentralContentQueryEngine::from_params(&params);
    let query_result = CentralContentQueryEngine::execute(&state.db, &criteria);
    let mut items = query_result.items;

    let user_opt = auth_service::session_user(&state, &headers);
    let purchased = user_opt
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();

    for item in &mut items {
        let (_, is_locked, _) =
            content_service::apply_paywall_view(item, user_opt.as_ref(), &purchased, "tr");
        if is_locked {
            item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
            item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
        }
    }

    Json(items)
}

pub async fn get_content_detail(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let lang = params.get("lang").map(|s| s.as_str()).unwrap_or("tr");
    let Some(mut item) = state.db.get_content_by_slug(&slug) else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::CONTENT_NOT_FOUND,
        )
        .into_response();
    };

    let user_opt = auth_service::session_user(&state, &headers);
    let is_staff = user_opt
        .as_ref()
        .map(|u| is_staff_role(&state.db, &u.role))
        .unwrap_or(false);

    if !content_service::is_publicly_visible(&item) && !is_staff {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::CONTENT_NOT_FOUND,
        )
        .into_response();
    }

    let purchased = user_opt
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();

    let (rendered_body, is_locked, lock_reason) =
        content_service::apply_paywall_view(&item, user_opt.as_ref(), &purchased, lang);

    if is_locked {
        item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
        item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
    }

    let all_contents = state.db.get_all_contents();
    let rules = state.db.get_recommendation_rules();
    let related = content_service::select_related_articles(
        &item,
        &all_contents,
        &rules,
        user_opt.as_ref(),
        3,
    );

    let all_ads = state.db.get_all_ads();
    let context_ads = ad_service::select_ads_for_context(
        &all_ads,
        Some(&item.content_type),
        Some(&item),
        user_opt.as_ref(),
    );

    Json(json!({
        "content": item,
        "paywall": {
            "is_locked": is_locked,
            "lock_reason": lock_reason,
            "rendered_body": rendered_body,
            "competitor_protection_active": item.is_sponsored
        },
        "related_articles": related,
        "context_ads": context_ads
    }))
    .into_response()
}

pub async fn list_categories(State(state): State<AppState>) -> Json<Vec<CategoryNode>> {
    Json(state.db.get_all_categories())
}

pub async fn list_ssb_tags(State(state): State<AppState>) -> Json<Vec<SsbInterestTag>> {
    Json(state.db.get_all_ssb_tags())
}

pub async fn list_issues(State(state): State<AppState>) -> Json<Value> {
    Json(json!(state.db.get_all_issues()))
}

pub async fn list_ads(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Json<Vec<AdCampaign>> {
    let all_ads = state.db.get_all_ads();
    let cat = params.get("category").map(|s| s.as_str());
    let content_opt = params
        .get("content_slug")
        .and_then(|slug| state.db.get_content_by_slug(slug));
    let user_opt = auth_service::session_user(&state, &headers);

    if cat.is_some() || content_opt.is_some() || user_opt.is_some() {
        Json(ad_service::select_ads_for_context(
            &all_ads,
            cat,
            content_opt.as_ref(),
            user_opt.as_ref(),
        ))
    } else {
        Json(all_ads.into_iter().filter(|a| a.is_active).collect())
    }
}

pub async fn download_edergi_pdf(
    State(state): State<AppState>,
    Path(issue_no): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let accepts_html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html"))
        .unwrap_or(false);

    let lang = api_response::resolve_lang(&params);
    let Some(user) = auth_service::session_user(&state, &headers) else {
        if accepts_html {
            return Redirect::to("/hesabim?next=/e-dergi").into_response();
        }
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            msg_keys::AUTH_REQUIRED_PDF,
        )
        .into_response();
    };

    if !content_service::user_has_subscriber_access(&user) {
        if accepts_html {
            return Redirect::to("/abonelik-ve-satis").into_response();
        }
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::FORBIDDEN,
            error_codes::FORBIDDEN,
            msg_keys::SUBSCRIBER_ACCESS_REQUIRED,
        )
        .into_response();
    }

    if !(user.email_verified || user.role == "site_operator" || user.role == "super_admin") {
        if accepts_html {
            return Redirect::to("/hesabim?verify_email=1").into_response();
        }
        return ApiResponse::raw(
            StatusCode::FORBIDDEN,
            json!({
                "status": "error",
                "error_code": error_codes::FORBIDDEN,
                "message": if lang.eq_ignore_ascii_case("en") {
                    "Please verify your email address before downloading E-Dergi PDF issues."
                } else {
                    "E-Dergi PDF indirebilmek için lütfen e-posta adresinizi doğrulayın."
                },
                "data": { "email_verified": false }
            }),
        )
        .into_response();
    }

    let requested_pub_type = params
        .get("publication_type")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let issue_opt = if let Some(pub_type) = requested_pub_type {
        state
            .db
            .get_all_issues()
            .into_iter()
            .find(|i| {
                i.issue_number == issue_no && i.publication_type.eq_ignore_ascii_case(pub_type)
            })
            .or_else(|| state.db.get_issue_by_number(issue_no))
    } else {
        state.db.get_issue_by_number(issue_no)
    };

    let Some(issue) = issue_opt else {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::ISSUE_NOT_FOUND,
        )
        .into_response();
    };

    let pages = params.get("pages").map(|s| s.as_str());
    let page_range = edergi_pdf_service::parse_page_range(pages);
    let client_ip = extract_client_ip(&headers);

    let org_cat = user.organization_category.trim();
    let tier = user.subscription_tier.trim();
    let license_label = match (org_cat.is_empty(), tier.is_empty()) {
        (false, false) => {
            if user.is_gov_verified {
                format!("{} Kamu Kurumsal Lisansi - {}", org_cat, tier)
            } else {
                format!("{} - {}", org_cat, tier)
            }
        }
        (false, true) => {
            if user.is_gov_verified {
                format!("{} Kamu Kurumsal Lisansi", org_cat)
            } else {
                org_cat.to_string()
            }
        }
        (true, false) => tier.to_string(),
        (true, true) => {
            if user.is_gov_verified {
                "Kamu Kurumsal E-Dergi Lisansi".to_string()
            } else {
                "Abone E-Dergi Lisansi".to_string()
            }
        }
    };

    let now = chrono::Utc::now();
    let now_ts = now.to_rfc3339();
    let watermark_code = edergi_pdf_service::generate_drm_watermark_code(
        issue.issue_number,
        &user.full_name,
        &user.organization_name,
        &client_ip,
        &now_ts,
        &state.config.watermark_secret,
    );

    let uploaded_pdf_bytes: Option<Vec<u8>> = issue
        .uploaded_pdf_file
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|raw| FsPath::new(raw).file_name())
        .and_then(|name| {
            let candidate = FsPath::new(&state.config.edergi_dir).join(name);
            std::fs::read(&candidate).ok()
        })
        .filter(|bytes| bytes.starts_with(b"%PDF-"));

    let pdf_bytes = match uploaded_pdf_bytes {
        Some(raw_pdf) => {
            let utc_date = now.format("%Y-%m-%d %H:%M UTC").to_string();
            let full_name = if user.full_name.trim().is_empty() {
                "Kayitli Okur"
            } else {
                user.full_name.trim()
            };
            let org_name = user.organization_name.trim();
            let org_display = match (org_name.is_empty(), org_cat.is_empty()) {
                (false, false) if !org_name.eq_ignore_ascii_case(org_cat) => {
                    format!("{} ({})", org_name, org_cat)
                }
                (false, _) => org_name.to_string(),
                (true, false) => org_cat.to_string(),
                (true, true) => "Kurumsal Paydas".to_string(),
            };
            let watermark_text = format!(
                "{} | {} | {} | {} | IP:{} | {}",
                full_name, org_display, license_label, utc_date, client_ip, watermark_code
            );
            edergi_pdf_service::apply_visible_watermark_and_slice(
                &raw_pdf,
                &watermark_text,
                &watermark_code,
                page_range,
            )
        }
        None => edergi_pdf_service::generate_watermarked_pdf(
            &issue,
            &user.full_name,
            &user.organization_name,
            &license_label,
            &client_ip,
            pages,
            &state.config.watermark_secret,
        ),
    };

    let page_suffix = pages.unwrap_or("all").replace(['/', '\\', ' '], "_");
    let prefix = if issue.publication_type.eq_ignore_ascii_case("msi_tdr") {
        "MSI_TDR_Issue"
    } else {
        "MSI_Dergisi_Sayi"
    };
    let filename = format!("{}_{}_Watermark_{}.pdf", prefix, issue.issue_number, page_suffix);

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/pdf".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("inline; filename=\"{}\"", filename),
            ),
            (
                header::HeaderName::from_static("x-msi-drm-watermark"),
                watermark_code,
            ),
        ],
        pdf_bytes,
    )
        .into_response()
}

#[cfg(test)]
mod edergi_download_tests {
    use crate::{
        config::AppConfig,
        controllers::AppState,
        db::Database,
        engine::translation_engine::TranslationCache,
        models::MagazineIssue,
        routes,
        services::{auth_service, edergi_pdf_service},
    };
    use axum::{
        body::{to_bytes, Body},
        http::{header, Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn test_state(label: &str) -> (AppState, String, String) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = format!("/tmp/msi_pub_pdf_{}_{}_{}.db", label, std::process::id(), ts);
        let edergi_dir = format!("/tmp/msi_pub_edergi_{}_{}_{}", label, std::process::id(), ts);
        let _ = std::fs::create_dir_all(&edergi_dir);
        let mut cfg = AppConfig::from_env();
        cfg.app_env = "test".into();
        cfg.sqlite_db_path = db_path.clone();
        cfg.edergi_dir = edergi_dir.clone();
        cfg.session_secret = "test-session-secret".into();
        cfg.watermark_secret = "test-watermark-secret".into();
        cfg.monitoring_enabled = false;
        let db = Database::new(&db_path).expect("test db");
        let translations = TranslationCache::new();
        translations.load(db.load_all_translations());
        let state = AppState {
            config: Arc::new(cfg),
            db,
            translations,
            monitoring_cache: None,
        };
        (state, db_path, edergi_dir)
    }

    fn make_session_cookie(
        state: &AppState,
        email: &str,
        full_name: &str,
        role: &str,
        org_cat: &str,
        org_name: &str,
        tier: &str,
        is_gov: bool,
        email_verified: bool,
    ) -> String {
        let hash = auth_service::hash_password("Pass1234!", &state.config.session_secret);
        let uid = state
            .db
            .admin_create_user(
                email,
                &hash,
                full_name,
                role,
                org_cat,
                org_name,
                "Uzman",
                is_gov,
                tier,
            )
            .unwrap();
        let conn = state.db.open_conn().unwrap();
        conn.execute(
            "UPDATE users SET email_verified = ?1, subscription_expires_at = '2030-12-31' WHERE id = ?2",
            rusqlite::params![if email_verified { 1 } else { 0 }, uid],
        )
        .unwrap();
        drop(conn);
        state.db.refresh_l1_sections(&["users"]);
        let user = state.db.get_user_by_id(uid).unwrap();
        format!(
            "msi_session={}",
            auth_service::generate_session_token(&user, &state.config.session_secret)
        )
    }

    #[tokio::test]
    async fn download_edergi_enforces_email_verified_and_watermarks_uploaded_pdf() {
        let (state, db_path, edergi_dir) = test_state("dl");
        let app = routes::build_router(state.clone());

        // Copy real 4-page PDF into edergi_dir and bind to an issue
        let sample_pdf_path = format!("{}/sample-4p.pdf", edergi_dir);
        let real_4p = std::fs::read(
            "/var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs/docs/specifications/proje-plani-ve-teslim-takvimi.pdf",
        );
        let pdf_bytes = match real_4p {
            Ok(b) => b,
            Err(_) => {
                // Fallback synthetic 4-page PDF if running without main docs path
                b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R 5 0 R 6 0 R] /Count 4 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n4 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n5 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n6 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] >>\nendobj\n%%EOF\n".to_vec()
            }
        };
        std::fs::write(&sample_pdf_path, &pdf_bytes).unwrap();

        let issue = MagazineIssue {
            issue_number: 777,
            slug: "test-issue-777".into(),
            title_tr: "Sayi 777".into(),
            title_en: "Issue 777".into(),
            publish_month_tr: "Kasim".into(),
            publish_month_en: "November".into(),
            year: 2026,
            cover_image: "images/msi/default-cover.webp".into(),
            is_special_issue: false,
            event_name: None,
            page_count: 4,
            dossier_count: 2,
            summary_tr: "Ozet".into(),
            summary_en: "Summary".into(),
            pdf_path: "/api/v1/edergi/777/download-pdf".into(),
            publication_type: "msi".into(),
            uploaded_pdf_file: Some("sample-4p.pdf".into()),
            spreads: vec![],
            ..Default::default()
        };
        state.db.upsert_issue(&issue).unwrap();

        // 1. Unverified .gov.tr subscriber is blocked (403)
        let unverified_gov = make_session_cookie(
            &state,
            "unverified@msb.gov.tr",
            "Mehmet Unverified",
            "subscriber_public",
            "MSB",
            "Milli Savunma Bakanligi",
            "Kamu E-Dergi Abonesi",
            true,
            false,
        );
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/edergi/777/download-pdf")
                    .header(header::COOKIE, &unverified_gov)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);

        // 2. Verified non-gov corporate subscriber receives watermarked, sliced 2-page PDF with dynamic license label
        let verified_corp = make_session_cookie(
            &state,
            "ali@aselsan.com.tr",
            "Ali Demir",
            "subscriber_digital",
            "Sektör Kuruluşları",
            "ASELSAN",
            "Dijital E-Dergi Abonesi",
            false,
            true,
        );
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/edergi/777/download-pdf?pages=2-3")
                    .header(header::COOKIE, &verified_corp)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let drm_header = resp
            .headers()
            .get("x-msi-drm-watermark")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(drm_header.starts_with("MSI-DRM-"));

        let out_bytes = to_bytes(resp.into_body(), 8 * 1024 * 1024).await.unwrap();
        assert!(out_bytes.starts_with(b"%PDF-"));
        assert_eq!(edergi_pdf_service::extract_pdf_page_count(&out_bytes), Some(2));
        let out_text = String::from_utf8_lossy(&out_bytes);
        assert!(out_text.contains("Ali Demir"));
        assert!(out_text.contains("ASELSAN"));
        assert!(out_text.contains("Sektor Kuruluslari - Dijital E-Dergi Abonesi"));
        assert!(!out_text.contains("KAMU PROTOKOL"));
        assert!(out_text.contains(&format!("%MSI-DRM:{}", drm_header)));

        // 3. site_operator bypasses email_verified requirement
        let site_op = make_session_cookie(
            &state,
            "op@msidergisi.com",
            "Site Operator",
            "site_operator",
            "MSI",
            "UMSA",
            "Tam Yetki",
            false,
            false,
        );
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/edergi/777/download-pdf")
                    .header(header::COOKIE, &site_op)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let _ = std::fs::remove_dir_all(&edergi_dir);
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}

// ============================================================================
// AUTH, LOCKOUT, SESSION REVOCATION & DEDICATED TOTP 2FA CHALLENGE
