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
        let inst_allowed =
            content_service::user_matches_target_institutions(item, user_opt.as_ref(), &state.db);
        let (_, is_locked, _) =
            content_service::apply_paywall_view(item, user_opt.as_ref(), &purchased, "tr");
        if is_locked || !inst_allowed {
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
    let lang_owned = api_response::resolve_lang_from_headers(&params, &headers);
    let lang = lang_owned.as_str();
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

    let inst_allowed =
        content_service::user_matches_target_institutions(&item, user_opt.as_ref(), &state.db);
    let purchased = user_opt
        .as_ref()
        .map(|u| state.db.get_user_purchased_codes(&u.email))
        .unwrap_or_default();

    let (mut rendered_body, mut is_locked, mut lock_reason) =
        content_service::apply_paywall_view(&item, user_opt.as_ref(), &purchased, lang);

    if !inst_allowed {
        is_locked = true;
        let raw = if lang.eq_ignore_ascii_case("en") && !item.body_en.trim().is_empty() {
            &item.body_en
        } else {
            &item.body_tr
        };
        rendered_body = content_service::truncate_to_two_sentences_html(raw);
        let inst_list = item.target_institutions.join(", ");
        lock_reason = if lang.eq_ignore_ascii_case("en") {
            format!(
                "Institution-Exclusive Dossier: This briefing is restricted to verified members of: {}.",
                inst_list
            )
        } else {
            format!(
                "Kuruma Özel İçerik: Bu dosya yalnızca yetkili kurum personeline ({}) açıktır.",
                inst_list
            )
        };
    }

    if is_locked {
        item.body_tr = content_service::truncate_to_two_sentences_html(&item.body_tr);
        item.body_en = content_service::truncate_to_two_sentences_html(&item.body_en);
    }

    let is_sponsored_or_adv = content_service::is_sponsored_or_advertorial(&item);

    let all_contents = state.db.get_all_contents();
    let rules = state.db.get_recommendation_rules();
    let mut related = if is_sponsored_or_adv {
        Vec::new()
    } else {
        content_service::select_related_articles_authorized(
            &item,
            &all_contents,
            &rules,
            user_opt.as_ref(),
            &state.db,
            all_contents.len().max(3),
        )
        .into_iter()
        .filter(|c| {
            c.id != item.id
                && !c.slug.eq_ignore_ascii_case(&item.slug)
                && !c.is_sponsored
                && !c.content_type.eq_ignore_ascii_case("advertorial")
                && !c.content_type.eq_ignore_ascii_case("advertorials")
        })
        .take(3)
        .collect()
    };
    for rel in &mut related {
        let (_, rel_locked, _) =
            content_service::apply_paywall_view(rel, user_opt.as_ref(), &purchased, lang);
        if rel_locked {
            rel.body_tr = content_service::truncate_to_two_sentences_html(&rel.body_tr);
            rel.body_en = content_service::truncate_to_two_sentences_html(&rel.body_en);
        }
    }

    let all_ads = state.db.get_all_ads();
    let mut ad_ctx_item = item.clone();
    if is_sponsored_or_adv {
        ad_ctx_item.is_sponsored = true;
    }
    let context_ads = ad_service::select_ads_for_context(
        &all_ads,
        Some(&item.content_type),
        Some(&ad_ctx_item),
        user_opt.as_ref(),
    );

    Json(json!({
        "content": item,
        "paywall": {
            "is_locked": is_locked,
            "lock_reason": lock_reason,
            "rendered_body": rendered_body,
            "competitor_protection_active": is_sponsored_or_adv
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
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        Json(
            all_ads
                .into_iter()
                .filter(|a| a.is_active && ad_service::is_ad_within_schedule(a, &today))
                .collect(),
        )
    }
}

#[allow(clippy::result_large_err)]
fn authorize_edergi_reader(
    state: &AppState,
    headers: &HeaderMap,
    lang: &str,
    accepts_html: bool,
) -> Result<UserAccount, Response> {
    let Some(user) = auth_service::session_user(state, headers) else {
        if accepts_html {
            return Err(Redirect::to("/hesabim?next=/e-dergi").into_response());
        }
        return Err(ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::UNAUTHORIZED,
            error_codes::UNAUTHORIZED,
            msg_keys::AUTH_REQUIRED_PDF,
        )
        .into_response());
    };

    if !content_service::user_has_subscriber_access(&user) {
        if accepts_html {
            return Err(Redirect::to("/abonelik-ve-satis").into_response());
        }
        return Err(ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::FORBIDDEN,
            error_codes::FORBIDDEN,
            msg_keys::SUBSCRIBER_ACCESS_REQUIRED,
        )
        .into_response());
    }

    if !(user.email_verified || is_staff_role(&state.db, &user.role)) {
        if accepts_html {
            return Err(Redirect::to("/hesabim?verify_email=1").into_response());
        }
        return Err(ApiResponse::raw(
            StatusCode::FORBIDDEN,
            json!({
                "status": "error",
                "error_code": error_codes::FORBIDDEN,
                "message": if lang.eq_ignore_ascii_case("en") {
                    "Please verify your email address before accessing E-Dergi issues."
                } else {
                    "E-Dergi içeriğine erişebilmek için lütfen e-posta adresinizi doğrulayın."
                },
                "data": { "email_verified": false }
            }),
        )
        .into_response());
    }

    Ok(user)
}

#[allow(clippy::result_large_err)]
fn resolve_edergi_issue(
    state: &AppState,
    params: &HashMap<String, String>,
    lang: &str,
    issue_no: i32,
) -> Result<crate::models::MagazineIssue, Response> {
    if let Some(raw_pub) = params
        .get("publication_type")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let Some(pub_type) = super::admin_issues::normalize_publication_type(raw_pub) else {
            return Err(ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::VALIDATION_ERROR,
            )
            .into_response());
        };
        return state.db.get_issue(pub_type, issue_no).ok_or_else(|| {
            ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::NOT_FOUND,
                error_codes::NOT_FOUND,
                msg_keys::ISSUE_NOT_FOUND,
            )
            .into_response()
        });
    }

    let matches = state.db.get_issues_by_number(issue_no);
    match matches.len() {
        0 => Err(ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::NOT_FOUND,
            error_codes::NOT_FOUND,
            msg_keys::ISSUE_NOT_FOUND,
        )
        .into_response()),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let pub_types: Vec<String> = matches.iter().map(|i| i.publication_type.clone()).collect();
            Err(ApiResponse::raw(
                StatusCode::CONFLICT,
                json!({
                    "status": "error",
                    "error_code": error_codes::CONFLICT,
                    "message": if lang.eq_ignore_ascii_case("en") {
                        "Multiple publications share this issue number. Specify publication_type."
                    } else {
                        "Bu sayı numarası birden fazla yayında mevcut. Lütfen publication_type belirtin."
                    },
                    "data": {
                        "issue_number": issue_no,
                        "publication_types": pub_types
                    }
                }),
            )
            .into_response())
        }
    }
}

fn build_edergi_watermark_meta(
    user: &UserAccount,
    issue_number: i32,
    client_ip: &str,
    secret: &str,
) -> (String, String, String, String) {
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
        issue_number,
        &user.full_name,
        &user.organization_name,
        client_ip,
        &now_ts,
        secret,
    );
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
    let diag_text = format!("{} ({}) | {}", full_name, org_display, watermark_code);
    (license_label, watermark_code, watermark_text, diag_text)
}

pub async fn download_edergi_pdf(
    State(state): State<AppState>,
    opt_ip: Option<Extension<security_engine::ResolvedClientIp>>,
    Path(issue_no): Path<i32>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let accepts_html = headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("text/html"))
        .unwrap_or(false);

    let lang_owned = api_response::resolve_lang_from_headers(&params, &headers);
    let lang = lang_owned.as_str();
    let user = match authorize_edergi_reader(&state, &headers, lang, accepts_html) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    let issue = match resolve_edergi_issue(&state, &params, lang, issue_no) {
        Ok(iss) => iss,
        Err(resp) => return resp,
    };

    let pages = params.get("pages").map(|s| s.as_str());
    let selected_pages = match edergi_pdf_service::parse_page_selection(pages) {
        Ok(sel) => sel,
        Err(msg) => {
            return ApiResponse::raw(
                StatusCode::BAD_REQUEST,
                json!({
                    "status": "error",
                    "error_code": error_codes::BAD_REQUEST,
                    "message": msg
                }),
            )
            .into_response();
        }
    };

    let client_ip = resolve_client_ip_with_forwarded(opt_ip.as_ref(), &headers).0;
    let (license_label, watermark_code, watermark_text, _) = build_edergi_watermark_meta(
        &user,
        issue.issue_number,
        &client_ip,
        &state.config.watermark_secret,
    );

    let uploaded_pdf: Option<(std::path::PathBuf, Vec<u8>)> = issue
        .uploaded_pdf_file
        .as_deref()
        .and_then(edergi_pdf_service::safe_pdf_filename)
        .and_then(|name| {
            let candidate = FsPath::new(&state.config.edergi_dir).join(name);
            let bytes = std::fs::read(&candidate).ok()?;
            Some((candidate, bytes))
        })
        .filter(|(_, bytes)| bytes.starts_with(b"%PDF-"));

    if let (Some((ref pdf_path, ref raw_pdf)), Some(ref sel)) = (&uploaded_pdf, &selected_pages) {
        if let Some(total) = edergi_pdf_service::detect_pdf_page_count(pdf_path, raw_pdf) {
            if total > 0 && sel.iter().any(|&p| p > total as usize) {
                return ApiResponse::raw(
                    StatusCode::BAD_REQUEST,
                    json!({
                        "status": "error",
                        "error_code": error_codes::BAD_REQUEST,
                        "message": if lang.eq_ignore_ascii_case("en") {
                            format!("Selected pages exceed issue page count ({}).", total)
                        } else {
                            format!("Seçilen sayfalar dergi sayfa sayısını ({}) aşıyor.", total)
                        }
                    }),
                )
                .into_response();
            }
        }
    }

    let pdf_bytes = match uploaded_pdf {
        Some((_, raw_pdf)) => edergi_pdf_service::apply_visible_watermark_and_select_pages(
            &raw_pdf,
            &watermark_text,
            &watermark_code,
            selected_pages.as_deref(),
        ),
        None => match edergi_pdf_service::generate_watermarked_pdf_checked(
            &issue,
            &user.full_name,
            &user.organization_name,
            &license_label,
            &client_ip,
            pages,
            &state.config.watermark_secret,
        ) {
            Ok(bytes) => bytes,
            Err(msg) => {
                return ApiResponse::raw(
                    StatusCode::BAD_REQUEST,
                    json!({
                        "status": "error",
                        "error_code": error_codes::BAD_REQUEST,
                        "message": msg
                    }),
                )
                .into_response();
            }
        },
    };

    let page_suffix = pages
        .unwrap_or("all")
        .replace(['/', '\\', ' ', '"', ';', '\r', '\n'], "_");
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

pub async fn stream_edergi_page(
    State(state): State<AppState>,
    opt_ip: Option<Extension<security_engine::ResolvedClientIp>>,
    Path((issue_no, page_num)): Path<(i32, i32)>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Response {
    let lang_owned = api_response::resolve_lang_from_headers(&params, &headers);
    let lang = lang_owned.as_str();
    let user = match authorize_edergi_reader(&state, &headers, lang, false) {
        Ok(u) => u,
        Err(resp) => return resp,
    };
    if page_num <= 0 {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::VALIDATION_ERROR,
        )
        .into_response();
    }
    let issue = match resolve_edergi_issue(&state, &params, lang, issue_no) {
        Ok(iss) => iss,
        Err(resp) => return resp,
    };

    let client_ip = resolve_client_ip_with_forwarded(opt_ip.as_ref(), &headers).0;
    let (_license_label, watermark_code, watermark_text, diag_text) = build_edergi_watermark_meta(
        &user,
        issue.issue_number,
        &client_ip,
        &state.config.watermark_secret,
    );

    let edergi_dir = state.config.edergi_dir.clone();
    let issue_clone = issue.clone();
    let diag_clone = diag_text.clone();
    let wm_clone = watermark_text.clone();
    let page_usize = page_num as usize;

    let rendered = tokio::task::spawn_blocking(move || {
        let pdf_safe_opt = issue_clone
            .uploaded_pdf_file
            .as_deref()
            .and_then(edergi_pdf_service::safe_pdf_filename)
            .filter(|name| {
                let p = FsPath::new(&edergi_dir).join(name);
                let mut hdr = [0u8; 5];
                std::fs::File::open(&p)
                    .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut hdr))
                    .map(|_| &hdr == b"%PDF-")
                    .unwrap_or(false)
            });

        let total_pages: i32 = if let Some(ref safe_name) = pdf_safe_opt {
            let pdf_path = FsPath::new(&edergi_dir).join(safe_name);
            let cached_cnt = edergi_pdf_service::count_cached_pages(
                &edergi_dir,
                &issue_clone.publication_type,
                issue_clone.issue_number,
            );
            let physical_pages = edergi_pdf_service::detect_pdf_page_count(&pdf_path, &[])
                .or_else(|| {
                    std::fs::read(&pdf_path)
                        .ok()
                        .and_then(|bytes| edergi_pdf_service::extract_pdf_page_count(&bytes))
                });
            physical_pages
                .filter(|&c| c > 0)
                .unwrap_or_else(|| issue_clone.page_count.max(cached_cnt as i32).max(1))
        } else {
            (1 + issue_clone.spreads.len().max(1)) as i32
        };

        if page_num > total_pages {
            return Err(total_pages);
        }

        if let Some(ref safe_name) = pdf_safe_opt {
            if let Some(cached_jpg) = edergi_pdf_service::ensure_single_page_jpeg(
                &edergi_dir,
                &issue_clone.publication_type,
                issue_clone.issue_number,
                safe_name,
                page_usize,
            ) {
                if let Some(webp_bytes) =
                    edergi_pdf_service::render_watermarked_webp(&cached_jpg, &diag_clone, &wm_clone)
                {
                    return Ok((total_pages, "image/webp".to_string(), webp_bytes));
                }
            }
        }

        let svg_bytes = edergi_pdf_service::render_fallback_page_svg(
            &issue_clone,
            page_usize,
            total_pages as usize,
            &diag_clone,
            &wm_clone,
        );
        Ok((total_pages, "image/svg+xml; charset=utf-8".to_string(), svg_bytes))
    })
    .await;

    match rendered {
        Ok(Ok((total_pages, content_type, bytes))) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (
                    header::CACHE_CONTROL,
                    "private, no-store, max-age=0".to_string(),
                ),
                (
                    header::HeaderName::from_static("x-content-type-options"),
                    "nosniff".to_string(),
                ),
                (
                    header::HeaderName::from_static("x-msi-drm-watermark"),
                    watermark_code,
                ),
                (
                    header::HeaderName::from_static("x-msi-page-count"),
                    total_pages.to_string(),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(Err(total_pages)) => (
            StatusCode::NOT_FOUND,
            [
                (
                    header::HeaderName::from_static("x-msi-page-count"),
                    total_pages.to_string(),
                ),
                (
                    header::CACHE_CONTROL,
                    "private, no-store, max-age=0".to_string(),
                ),
            ],
            Json(json!({
                "status": "error",
                "error_code": error_codes::NOT_FOUND,
                "message": if lang.eq_ignore_ascii_case("en") {
                    "Requested page does not exist in this issue."
                } else {
                    "İstenen sayfa bu sayıda bulunamadı."
                },
                "data": { "page_num": page_num, "total_pages": total_pages }
            })),
        )
            .into_response(),
        Err(e) => ApiResponse::server_error(&state.translations, lang, "stream_edergi_page", &e)
            .into_response(),
    }
}

fn serialize_feed_card(item: &ContentItem, lang: &str) -> Value {
    let is_en = lang.eq_ignore_ascii_case("en");
    let title = if is_en && !item.title_en.trim().is_empty() {
        &item.title_en
    } else {
        &item.title_tr
    };
    let summary = if is_en && !item.summary_en.trim().is_empty() {
        &item.summary_en
    } else {
        &item.summary_tr
    };
    json!({
        "id": item.id,
        "slug": item.slug,
        "title": title,
        "title_tr": item.title_tr,
        "title_en": item.title_en,
        "summary": summary,
        "summary_tr": item.summary_tr,
        "summary_en": item.summary_en,
        "cover_image": item.cover_image,
        "content_type": item.content_type,
        "category_label": item.content_type.replace('-', " "),
        "published_at": item.published_at,
        "read_time_min": item.read_time_min,
        "ssb_tags": item.ssb_tags,
        "is_sponsored": item.is_sponsored
    })
}

fn is_organic_public_item(c: &ContentItem, lang: &str) -> bool {
    let is_en = lang.eq_ignore_ascii_case("en");
    content_service::is_publicly_visible(c)
        && !c.is_sponsored
        && !c.content_type.eq_ignore_ascii_case("advertorial")
        && !c.content_type.eq_ignore_ascii_case("advertorials")
        && !c.content_type.eq_ignore_ascii_case("yeni-sayi-kapak")
        && (!is_en || crate::engine::design::labels_helper::is_en_content_ready(c))
        && (is_en || !c.source_group.eq_ignore_ascii_case("msi-tdr"))
}

pub async fn public_personalized_feed(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let lang_owned = api_response::resolve_lang_from_headers(&params, &headers);
    let lang = lang_owned.as_str();
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(6)
        .clamp(1, 24);

    let user_opt = auth_service::session_user(&state, &headers);
    let interests: Vec<String> = if let Some(ref u) = user_opt {
        u.ssb_interests.clone()
    } else {
        params
            .get("interests")
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };

    let all_contents = state.db.get_all_contents();
    let rules = state.db.get_recommendation_rules();
    let user_org = user_opt
        .as_ref()
        .map(|u| u.organization_category.as_str())
        .unwrap_or("");

    let mut scored: Vec<(i64, String, i64, &ContentItem)> = all_contents
        .iter()
        .filter(|c| {
            is_organic_public_item(c, lang)
                && content_service::user_matches_target_institutions(
                    c,
                    user_opt.as_ref(),
                    &state.db,
                )
        })
        .map(|c| {
            let mut score: i64 = 0;
            for interest in &interests {
                if c.ssb_tags.iter().any(|t| t.eq_ignore_ascii_case(interest)) {
                    score += 100;
                }
            }
            for rule in rules.iter().filter(|r| r.is_active) {
                if rule.recommended_content_slug.eq_ignore_ascii_case(&c.slug)
                    && (rule.target_institution.is_empty()
                        || rule.target_institution == "*"
                        || rule.target_institution.eq_ignore_ascii_case("all")
                        || (!user_org.is_empty()
                            && rule.target_institution.eq_ignore_ascii_case(user_org)))
                {
                    score += 50 + (rule.priority as i64 * 5);
                }
            }
            if c.is_featured {
                score += 10;
            }
            (score, c.published_at.clone(), c.view_count, c)
        })
        .collect();

    scored.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| b.3.id.cmp(&a.3.id))
    });

    let items: Vec<Value> = scored
        .into_iter()
        .take(limit)
        .map(|(_, _, _, c)| serialize_feed_card(c, lang))
        .collect();

    ApiResponse::ok(json!({
        "personalized": user_opt.is_some(),
        "interests": interests,
        "items": items
    }))
    .into_response()
}

pub async fn public_related_articles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let lang_owned = api_response::resolve_lang_from_headers(&params, &headers);
    let lang = lang_owned.as_str();
    let limit = params
        .get("limit")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(3, 12);
    let slug_opt = params
        .get("slug")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let category_opt = params
        .get("category")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());

    let user_opt = auth_service::session_user(&state, &headers);
    let all_contents = state.db.get_all_contents();
    let rules = state.db.get_recommendation_rules();

    if let Some(slug) = slug_opt {
        if let Some(current) = all_contents.iter().find(|c| c.slug.eq_ignore_ascii_case(slug)) {
            if current.is_sponsored
                || current.content_type.eq_ignore_ascii_case("advertorial")
                || current.content_type.eq_ignore_ascii_case("advertorials")
            {
                return ApiResponse::ok(json!({ "items": [] })).into_response();
            }
            let selected = content_service::select_related_articles_authorized(
                current,
                &all_contents,
                &rules,
                user_opt.as_ref(),
                &state.db,
                limit,
            );
            let items: Vec<Value> = selected
                .into_iter()
                .filter(|c| {
                    is_organic_public_item(c, lang)
                        && content_service::user_matches_target_institutions(
                            c,
                            user_opt.as_ref(),
                            &state.db,
                        )
                })
                .take(limit)
                .map(|c| serialize_feed_card(&c, lang))
                .collect();
            return ApiResponse::ok(json!({ "items": items })).into_response();
        }
    }

    let mut organic: Vec<&ContentItem> = all_contents
        .iter()
        .filter(|c| {
            is_organic_public_item(c, lang)
                && content_service::user_matches_target_institutions(
                    c,
                    user_opt.as_ref(),
                    &state.db,
                )
        })
        .collect();

    organic.sort_by(|a, b| {
        let a_cat = category_opt
            .map(|cat| {
                a.content_type.eq_ignore_ascii_case(cat)
                    || a.placements.iter().any(|p| p.eq_ignore_ascii_case(cat))
            })
            .unwrap_or(false);
        let b_cat = category_opt
            .map(|cat| {
                b.content_type.eq_ignore_ascii_case(cat)
                    || b.placements.iter().any(|p| p.eq_ignore_ascii_case(cat))
            })
            .unwrap_or(false);
        b_cat
            .cmp(&a_cat)
            .then_with(|| b.published_at.cmp(&a.published_at))
            .then_with(|| b.id.cmp(&a.id))
    });

    let items: Vec<Value> = organic
        .into_iter()
        .take(limit)
        .map(|c| serialize_feed_card(c, lang))
        .collect();

    ApiResponse::ok(json!({ "items": items })).into_response()
}

#[derive(Deserialize)]
pub struct PublicNewsletterSubscribePayload {
    pub email: String,
    pub full_name: Option<String>,
    pub email_group_code: Option<String>,
    pub kvkk_consent: Option<bool>,
    pub email_consent: Option<bool>,
    pub lang: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default, deserialize_with = "crate::engine::security_engine::deserialize_opt_i64")]
    pub _form_ts: Option<i64>,
}

pub async fn public_newsletter_subscribe(
    State(state): State<AppState>,
    opt_ip: Option<Extension<security_engine::ResolvedClientIp>>,
    headers: HeaderMap,
    Json(payload): Json<PublicNewsletterSubscribePayload>,
) -> Response {
    let lang = payload
        .lang
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("tr");
    let (client_ip, was_forwarded) = resolve_client_ip_with_forwarded(opt_ip.as_ref(), &headers);

    match security_engine::validate_form_submission_with_forwarded(
        "newsletter",
        &client_ip,
        was_forwarded,
        payload.website_url.as_deref(),
        payload._form_ts,
    ) {
        security_engine::FormSpamResult::Passed => {}
        security_engine::FormSpamResult::HoneypotTriggered => {
            return ApiResponse::ok(json!({ "subscribed": true })).into_response();
        }
        security_engine::FormSpamResult::TimingViolation(_)
        | security_engine::FormSpamResult::RateLimited { .. } => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::TOO_MANY_REQUESTS,
                error_codes::RATE_LIMITED,
                msg_keys::RATE_LIMIT_EXCEEDED,
            )
            .into_response();
        }
    }

    let email = match validation_engine::validate_email(&payload.email) {
        validation_engine::EmailValidation::Valid(e) => e,
        _ => {
            return ApiResponse::error(
                &state.translations,
                lang,
                StatusCode::BAD_REQUEST,
                error_codes::BAD_REQUEST,
                msg_keys::INVALID_EMAIL,
            )
            .into_response();
        }
    };

    if payload.kvkk_consent != Some(true) {
        return ApiResponse::error(
            &state.translations,
            lang,
            StatusCode::BAD_REQUEST,
            error_codes::BAD_REQUEST,
            msg_keys::KVKK_CONSENT_REQUIRED,
        )
        .into_response();
    }

    let group_code = payload
        .email_group_code
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("general");

    let etk_accepted = payload.email_consent != Some(false);
    let user_id = state.db.get_user_by_email(&email).map(|(u, _)| u.id);
    let now_iso = chrono::Utc::now().to_rfc3339();
    let consent_hash = auth_service::generate_kvkk_consent_hash(
        &email,
        &client_ip,
        &now_iso,
        &state.config.session_secret,
    );

    for (ctype, accepted) in [("kvkk", true), ("etk", etk_accepted)] {
        let _ = state.db.insert_kvkk_consent_log(&crate::models::KvkkConsentLog {
            id: 0,
            user_id,
            email: email.clone(),
            consent_type: ctype.to_string(),
            consent_version: "2026-01".to_string(),
            accepted,
            ip_address: client_ip.clone(),
            consent_hash: consent_hash.clone(),
            created_at: now_iso.clone(),
        });
    }

    if etk_accepted {
        if let Err(e) = state.db.add_email_to_email_group(group_code, &email) {
            return ApiResponse::server_error(&state.translations, lang, "newsletter_subscribe", &e)
                .into_response();
        }
    }

    let msg = if !etk_accepted {
        if lang.eq_ignore_ascii_case("en") {
            "Your KVKK preference was recorded without marketing email opt-in."
        } else {
            "KVKK kaydınız alındı; ticari elektronik ileti izniniz kapalı tutuldu."
        }
    } else if lang.eq_ignore_ascii_case("en") {
        "Your newsletter subscription and consent preferences have been recorded."
    } else {
        "Bülten aboneliğiniz ve iletişim onayınız kaydedildi."
    };

    ApiResponse::raw(
        StatusCode::OK,
        json!({
            "status": "ok",
            "message": msg,
            "data": {
                "subscribed": etk_accepted,
                "etk_accepted": etk_accepted,
                "email": email,
                "full_name": payload.full_name.unwrap_or_default().trim(),
                "email_group_code": group_code
            }
        }),
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
            cover_image: crate::db::constants::DEFAULT_ISSUE_COVER_PATH.into(),
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

    #[tokio::test]
    async fn list_ads_unfiltered_excludes_expired_and_future_and_protects_advertorials_slug() {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let db_path = std::env::temp_dir()
            .join(format!("msi_pub_ads_{}_{}.db", std::process::id(), ts))
            .to_string_lossy()
            .to_string();
        let mut cfg = AppConfig::from_env();
        cfg.app_env = "test".into();
        cfg.sqlite_db_path = db_path.clone();
        cfg.session_secret = "test-session-secret".into();
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
        let app = routes::build_router(state.clone());

        let mk_ad = |sponsor: &str, sponsor_type: &str, start: Option<&str>, end: Option<&str>| {
            crate::models::AdCampaign {
                id: 0,
                slot_code: "ad-news-top".to_string(),
                badge_label: "SPONSOR".to_string(),
                dimensions: "728x90".to_string(),
                sponsor_name: sponsor.to_string(),
                sponsor_type: sponsor_type.to_string(),
                pill_style: "solid".to_string(),
                title_tr: sponsor.to_string(),
                title_en: sponsor.to_string(),
                desc_tr: "Desc".to_string(),
                desc_en: "Desc".to_string(),
                target_url: "https://example.com".to_string(),
                image_url: None,
                budget_score: 50,
                is_default_fallback: false,
                is_floating_popup: false,
                is_newsletter_ad: false,
                target_categories: vec!["haberler".to_string()],
                target_ssb_tags: vec![],
                target_institutions: vec![],
                impressions_count: 0,
                clicks_count: 0,
                is_active: true,
                start_date: start.map(ToString::to_string),
                end_date: end.map(ToString::to_string),
                rotation_mode: "weighted".to_string(),
            }
        };

        state
            .db
            .upsert_ad(&mk_ad(
                "ExpiredSponsor",
                "defense_oem",
                Some("2020-01-01"),
                Some("2020-12-31"),
            ))
            .unwrap();
        state
            .db
            .upsert_ad(&mk_ad(
                "FutureSponsor",
                "defense_oem",
                Some("2035-01-01"),
                Some("2035-12-31"),
            ))
            .unwrap();
        state
            .db
            .upsert_ad(&mk_ad("ActiveCurrentOEM", "defense_oem", Some("2020-01-01"), Some("2035-12-31")))
            .unwrap();
        state
            .db
            .upsert_ad(&mk_ad("ActiveAuxService", "auxiliary_service", None, None))
            .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/v1/ads")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 2 * 1024 * 1024).await.unwrap();
        let ads: Vec<crate::models::AdCampaign> = serde_json::from_slice(&body).unwrap();
        let sponsors: Vec<&str> = ads.iter().map(|a| a.sponsor_name.as_str()).collect();
        assert!(!sponsors.contains(&"ExpiredSponsor"));
        assert!(!sponsors.contains(&"FutureSponsor"));
        assert!(sponsors.contains(&"ActiveCurrentOEM"));
        assert!(sponsors.contains(&"ActiveAuxService"));

        // Advertorials (plural) with is_sponsored = false must shield defense_oem on /api/v1/ads?content_slug=...
        let adv_item = crate::models::ContentItem {
            slug: "test-adv-plural-shield".to_string(),
            title_tr: "Sponsorlu Dosya".to_string(),
            body_tr: "<p>Test içerik.</p>".to_string(),
            content_type: "advertorials".to_string(),
            source_group: "gunluk-haber".to_string(),
            is_sponsored: false,
            status: "published".to_string(),
            ..crate::models::ContentItem::default()
        };
        state.db.upsert_content(&adv_item).unwrap();

        let resp_adv = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/ads?content_slug=test-adv-plural-shield")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp_adv.status(), StatusCode::OK);
        let body_adv = to_bytes(resp_adv.into_body(), 2 * 1024 * 1024).await.unwrap();
        let ads_adv: Vec<crate::models::AdCampaign> = serde_json::from_slice(&body_adv).unwrap();
        assert!(!ads_adv.iter().any(|a| a.sponsor_name == "ActiveCurrentOEM"));
        assert!(ads_adv.iter().any(|a| a.sponsor_name == "ActiveAuxService"));

        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-wal", db_path));
        let _ = std::fs::remove_file(format!("{}-shm", db_path));
    }
}

// ============================================================================
// AUTH, LOCKOUT, SESSION REVOCATION & DEDICATED TOTP 2FA CHALLENGE
