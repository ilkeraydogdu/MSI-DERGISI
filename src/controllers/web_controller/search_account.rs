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

    let body_html = design_engine::render_search_and_filter_page(
        &lang, &facet_res, &q, &cat, &ssb, iss, &src, &sort, &state.translations,
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

    let admin_path_opt = if crate::extractors::operator_user::is_staff_role(&state.db, &user.role) {
        Some(state.config.admin_path_secret.as_str())
    } else {
        None
    };

    let institutions = crate::engine::design::labels_helper::institution_choices_from_db(&state.db);
    let body_html = crate::engine::design::account::render_with_context(
        &lang,
        &user,
        &reminders,
        &invoices,
        &notifications,
        &ssb_tags,
        admin_path_opt,
        Some(tr),
        &institutions,
    );
    let logged_title = tr_text(
        Some(tr),
        &lang,
        "account.page_title_logged_in",
        "Okuyucu & Abone Kontrol Merkezi",
        "Reader & Subscriber Control Center",
    );

    Html(home::render_shared_shell_with_request(
        &state,
        &lang,
        &logged_title,
        &body_html,
        Some("/hesabim"),
        None,
        true,
        Some(&csp_nonce.0),
        Some(&headers),
        None,
    ))
    .into_response()
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

    let body_html = format!(
        r#"<section class="container section-space">
  <div class="transactional-card reset-password-card">
    <div class="eyebrow">{eyebrow}</div>
    <h1>{heading}</h1>
    <p>{desc}</p>
    <form id="reset-password-form" class="ui-form">
      <input type="hidden" name="token" value="{escaped_token}">
      <label>{new_pw_lbl}<input type="password" name="new_password" required minlength="8" autocomplete="new-password" placeholder="••••••••"></label>
      <label>{confirm_pw_lbl}<input type="password" name="confirm_password" required minlength="8" autocomplete="new-password" placeholder="••••••••"></label>
      <p class="form-feedback" data-feedback role="status"></p>
      <button type="submit" class="button primary">{submit_lbl}</button>
    </form>
  </div>
</section>"#,
        eyebrow = esc(&eyebrow),
        heading = esc(&heading),
        desc = esc(&desc),
        escaped_token = escaped_token,
        new_pw_lbl = esc(&new_pw_lbl),
        confirm_pw_lbl = esc(&confirm_pw_lbl),
        submit_lbl = esc(&submit_lbl),
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
