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

    let Some(issue) = state.db.get_issue_by_number(issue_no) else {
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
    let client_ip = extract_client_ip(&headers);
    let license_label = if user.is_gov_verified {
        "KAMU PROTOKOL & KURUMSAL E-DERGI LISANSI"
    } else {
        user.subscription_tier.as_str()
    };

    let now_ts = chrono::Utc::now().to_rfc3339();
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

    let pdf_bytes = uploaded_pdf_bytes.unwrap_or_else(|| {
        edergi_pdf_service::generate_watermarked_pdf(
            &issue,
            &user.full_name,
            &user.organization_name,
            license_label,
            &client_ip,
            pages,
            &state.config.watermark_secret,
        )
    });

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

// ============================================================================
// AUTH, LOCKOUT, SESSION REVOCATION & DEDICATED TOTP 2FA CHALLENGE
