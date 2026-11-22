use super::*;
use crate::engine::design::labels_helper::tr_text;

pub async fn render_search_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_search_page_for_lang(&state, &params, &headers, &csp_nonce.0, None).await
}

pub async fn render_en_search_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    render_search_page_for_lang(&state, &params, &headers, &csp_nonce.0, Some("en")).await
}

async fn render_search_page_for_lang(
    state: &AppState,
    params: &HashMap<String, String>,
    headers: &HeaderMap,
    csp_nonce: &str,
    forced_lang: Option<&str>,
) -> Response {
    use crate::engine::content_query_engine::CentralContentQueryEngine;

    let lang = forced_lang
        .map(str::to_string)
        .unwrap_or_else(|| detect_language(params, headers));
    let canonical_path = if forced_lang == Some("en") {
        "/en/search"
    } else {
        "/arama"
    };

    // Unified query via CentralContentQueryEngine — replaces all manual param parsing and faceted search calls.
    let mut criteria = CentralContentQueryEngine::from_params(params);
    criteria.sort_by = criteria.sort_by.or_else(|| Some("latest".to_string()));
    criteria.lang = Some(lang.clone());
    criteria.limit = Some(crate::db::constants::ADMIN_LIST_LIMIT);

    let _query_result = CentralContentQueryEngine::execute(&state.db, &criteria);

    let q = criteria.q.clone().unwrap_or_default();
    let cat = criteria.category.clone().unwrap_or_default();
    let ssb = criteria.ssb_tag.clone().unwrap_or_default();
    let iss = criteria.issue_number;
    let src = criteria.source_group.clone().unwrap_or_default();
    let sort = criteria.sort_by.clone().unwrap_or_else(|| "latest".to_string());

    let has_active_filter = !q.trim().is_empty()
        || !cat.trim().is_empty()
        || !ssb.trim().is_empty()
        || iss.is_some()
        || !src.trim().is_empty();

    // Build a SearchFacetResponse from the unified result for the existing design renderer.
    let snap = CentralQueryEngine::snapshot(&state.db);
    let facet_criteria = SearchFilterQuery {
        q: criteria.q.clone(),
        category: criteria.category.clone(),
        ssb_tag: criteria.ssb_tag.clone(),
        content_type: criteria.content_type.clone(),
        source_group: criteria.source_group.clone(),
        issue_number: criteria.issue_number,
        visibility_level: criteria.visibility_level,
        is_sponsored: criteria.is_sponsored,
        sort_by: criteria.sort_by.clone(),
        lang: criteria.lang.clone(),
        limit: Some(60),
        page: None,
        per_page: None,
    };
    let facet_res = search_engine::execute_faceted_search(
        &facet_criteria,
        &snap.data.contents,
        &snap.data.categories,
        &snap.data.ssb_tags,
        &snap.data.issues,
        &snap.data.site_settings,
    );

    let mut body_html = design_engine::render_search_and_filter_page(
        &lang, &facet_res, &q, &cat, &ssb, iss, &src, &sort, &state.translations,
    );
    body_html.push_str(
        &crate::engine::design::components::sections::render_related_strip(
            &snap.data.contents,
            &crate::engine::design::components::sections::RelatedStripOptions {
                lang: &lang,
                is_current_sponsored: false,
                exclude_ids: &[],
                limit: 4,
                tr: Some(&state.translations),
                source_context: "search",
            },
        ),
    );
    let seo_title = tr_text(
        Some(&state.translations),
        &lang,
        "seo.search_page_title",
        "Arama ve keşif",
        "Search and discovery",
    );

    Html(home::render_shared_shell_with_request(
        state,
        &lang,
        &seo_title,
        &body_html,
        Some(canonical_path),
        None,
        has_active_filter,
        Some(csp_nonce),
        Some(headers),
        None,
    ))
    .into_response()
}

pub async fn render_account_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    let lang = detect_language(&params, &headers);
    let tr = &state.translations;
    let user_opt = authenticated_reader(&state, &headers);
    let Some(user) = user_opt else {
        let body = crate::engine::design::account::account_with_context(
            None,
            &[],
            &lang,
            None,
            Some(tr),
        );
        let guest_title = tr_text(Some(tr), &lang, "account.page_title_guest", "Hesabım", "My Account");
        return Html(home::render_shared_shell_with_request(
            &state,
            &lang,
            &guest_title,
            &body,
            Some("/hesabim"),
            None,
            true,
            Some(&csp_nonce.0),
            Some(&headers),
            None,
        ))
        .into_response();
    };
    let reminders: Vec<_> = state
        .db
        .get_renewal_reminders()
        .into_iter()
        .filter(|r| r.user_id == user.id)
        .collect();
    let invoices: Vec<_> = state
        .db
        .get_order_invoices()
        .into_iter()
        .filter(|i| i.user_email.eq_ignore_ascii_case(&user.email))
        .collect();
    let notifications = crate::engine::notification_engine::filter_user_notifications(
        &state.db.get_system_notifications(Some(&user.email)),
        Some(&user.email),
        Some(&user.organization_category),
    );
    let ssb_tags = state.db.get_all_ssb_tags();

    let is_staff = crate::extractors::operator_user::is_staff_role(&state.db, &user.role);
    let normalized_admin = is_staff
        .then(|| crate::config::normalize_admin_path_secret(&state.config.admin_path_secret));
    let admin_path_opt = normalized_admin.as_deref();
    let can_access_design_studio = is_staff
        && (crate::extractors::operator_user::role_has_permission(
            &state.db,
            &user.role,
            "settings:read",
        ) || crate::extractors::operator_user::role_has_permission(
            &state.db,
            &user.role,
            "settings:write",
        ));

    let institutions = crate::engine::design::labels_helper::institution_choices_from_db(&state.db);
    let kvkk_logs = state.db.list_kvkk_consent_logs_for_user(user.id, &user.email);
    let snap = CentralQueryEngine::snapshot(&state.db).data;
    let allowed_contents: Vec<ContentItem> = snap
        .contents
        .iter()
        .filter(|c| content_service::user_matches_target_institutions(c, Some(&user), &state.db))
        .cloned()
        .collect();
    let mut issues = snap.issues.clone();
    issues.sort_by(|a, b| {
        b.issue_number
            .cmp(&a.issue_number)
            .then_with(|| b.year.cmp(&a.year))
    });
    let corporate_requests: Vec<crate::models::CorporateVerificationRequest> = state
        .db
        .get_corporate_requests()
        .into_iter()
        .filter(|r| r.user_id == user.id || r.email.eq_ignore_ascii_case(&user.email))
        .collect();
    let purchased_codes = state.db.get_user_purchased_codes(&user.email);
    let has_subscriber_access = content_service::user_has_subscriber_access(&user);
    let can_download_edergi = has_subscriber_access && (user.email_verified || is_staff);

    let payment_success = params
        .get("payment")
        .map(|v| v.eq_ignore_ascii_case("success"))
        .unwrap_or(false);
    let payment_oid = params.get("oid").map(String::as_str);
    let verify_email_prompt = params
        .get("verify_email")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let next_url = params.get("next").map(String::as_str);
    let active_tab = params.get("tab").map(String::as_str);

    let ctx = crate::engine::design::account::AccountPortalContext {
        lang: &lang,
        user: &user,
        reminders: &reminders,
        invoices: &invoices,
        notifications: &notifications,
        tags: &ssb_tags,
        admin_path_opt,
        tr: Some(tr),
        institutions: &institutions,
        kvkk_logs: &kvkk_logs,
        contents: &allowed_contents,
        issues: &issues,
        corporate_requests: &corporate_requests,
        purchased_codes: &purchased_codes,
        can_download_edergi,
        has_subscriber_access,
        can_access_design_studio,
        hints: crate::engine::design::account::PortalQueryHints {
            payment_success,
            payment_oid,
            verify_email_prompt,
            next_url,
            active_tab,
        },
        csp_nonce: Some(&csp_nonce.0),
    };

    Html(crate::engine::design::account::render_portal_document(&ctx)).into_response()
}

pub async fn render_reset_password_page(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
    Extension(csp_nonce): Extension<CspNonce>,
) -> Response {
    use crate::engine::security_engine::escape_html as esc;

    let lang = detect_language(&params, &headers);
    let tr = Some(&state.translations);
    let raw_token = params.get("token").map(String::as_str).unwrap_or("").trim();
    let escaped_token = esc(raw_token);

    let eyebrow = tr_text(
        tr,
        &lang,
        "auth.reset_eyebrow",
        "HESAP GÜVENLİĞİ · ŞİFRE YENİLEME",
        "ACCOUNT SECURITY · PASSWORD RESET",
    );
    let heading = tr_text(
        tr,
        &lang,
        "auth.reset_heading",
        "Yeni Şifrenizi Belirleyin",
        "Set Your New Password",
    );
    let desc = tr_text(
        tr,
        &lang,
        "auth.reset_desc",
        "MSI Dergisi hesabınız için en az 8 karakterden oluşan yeni şifrenizi girin.",
        "Enter a strong new password of at least 8 characters to secure your MSI Dergisi account.",
    );
    let new_pw_lbl = tr_text(tr, &lang, "auth.reset_new_pw", "Yeni Şifre", "New Password");
    let confirm_pw_lbl = tr_text(
        tr,
        &lang,
        "auth.reset_confirm_pw",
        "Yeni Şifre (Tekrar)",
        "Confirm New Password",
    );
    let submit_lbl = tr_text(
        tr,
        &lang,
        "auth.reset_submit",
        "Şifremi Güncelle →",
        "Update Password →",
    );
    let page_title = tr_text(
        tr,
        &lang,
        "auth.reset_page_title",
        "Şifre Sıfırlama",
        "Reset Password",
    );
    let pw_hint = tr_text(
        tr,
        &lang,
        "auth.reset_pw_hint",
        "En az 8 karakter kullanın; büyük/küçük harf ve rakam önerilir.",
        "Use at least 8 characters; mixing upper/lowercase letters and numbers is recommended.",
    );
    let missing_token_title = tr_text(
        tr,
        &lang,
        "auth.reset_missing_token_title",
        "Sıfırlama bağlantısı eksik veya geçersiz",
        "Reset token missing or incomplete",
    );
    let missing_token_desc = tr_text(
        tr,
        &lang,
        "auth.reset_missing_token_desc",
        "E-postanızdaki şifre sıfırlama bağlantısına tıklayarak devam edin veya Hesabım sayfasından yeni bir sıfırlama bağlantısı isteyin.",
        "Open this page using the reset link in your email, or request a new reset link from My Account.",
    );
    let back_account_lbl = tr_text(
        tr,
        &lang,
        "auth.reset_back_account",
        "← Hesabım / Giriş sayfasına dön",
        "← Back to My Account / Sign in",
    );
    let account_href = crate::engine::design::labels_helper::lang_href("/hesabim", &lang);
    let token_banner = if raw_token.is_empty() {
        format!(
            r#"<div class="auth-step-banner is-warning" role="note"><strong>{title}</strong><p>{desc}</p></div>"#,
            title = esc(&missing_token_title),
            desc = esc(&missing_token_desc),
        )
    } else {
        String::new()
    };

    let body_html = format!(
        r#"<section class="container section-space">
  <div class="transactional-card reset-password-card">
    <div class="eyebrow">{eyebrow}</div>
    <h1>{heading}</h1>
    <p>{desc}</p>
    {token_banner}
    <form id="reset-password-form" class="ui-form">
      <input type="hidden" name="token" value="{escaped_token}">
      <label>{new_pw_lbl}<input type="password" name="new_password" required minlength="8" autocomplete="new-password" placeholder="••••••••"><small class="auth-field-hint">{pw_hint}</small></label>
      <label>{confirm_pw_lbl}<input type="password" name="confirm_password" required minlength="8" autocomplete="new-password" placeholder="••••••••"></label>
      <p class="form-feedback" data-feedback role="status"></p>
      <button type="submit" class="button primary">{submit_lbl}</button>
    </form>
    <div class="reset-password-footer">
      <a href="{account_href}" class="text-link">{back_account_lbl}</a>
    </div>
  </div>
</section>"#,
        eyebrow = esc(&eyebrow),
        heading = esc(&heading),
        desc = esc(&desc),
        token_banner = token_banner,
        escaped_token = escaped_token,
        new_pw_lbl = esc(&new_pw_lbl),
        pw_hint = esc(&pw_hint),
        confirm_pw_lbl = esc(&confirm_pw_lbl),
        submit_lbl = esc(&submit_lbl),
        account_href = esc(&account_href),
        back_account_lbl = esc(&back_account_lbl),
    );

    Html(home::render_shared_shell_with_request(
        &state,
        &lang,
        &page_title,
        &body_html,
        Some("/sifre-sifirla"),
        None,
        true,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
    ))
    .into_response()
}
