//! Integration tests for Content CRUD operations and paywall logic.

mod common;

use common::*;

#[test]
fn content_paywall_level_0_is_open_to_anonymous() {
    use msi_dergisi_platform::services::content_service::apply_paywall_view;

    let article = fixture_content(0); // visibility_level 0
    let (body, locked, reason) = apply_paywall_view(&article, None, &[], "tr");
    assert!(!locked);
    assert!(reason.is_empty());
    assert!(body.contains("Test makalesi"));
}

#[test]
fn content_paywall_level_1_requires_login() {
    use msi_dergisi_platform::services::content_service::apply_paywall_view;

    let mut article = fixture_content(1);
    article.visibility_level = 1;

    // Anonymous user gets truncated preview
    let (_, locked, reason) = apply_paywall_view(&article, None, &[], "tr");
    assert!(locked);
    assert!(reason.contains("kayıtlı kullanıcılar") || reason.contains("registered"));

    // Logged-in user gets full access
    let user = fixture_free_user(10);
    let (_, locked, _) = apply_paywall_view(&article, Some(&user), &[], "tr");
    assert!(!locked);
}

#[test]
fn content_paywall_level_2_requires_subscription_or_purchase() {
    use msi_dergisi_platform::services::content_service::apply_paywall_view;

    let mut article = fixture_content(2);
    article.visibility_level = 2;

    // Anonymous: locked
    let (_, locked, _) = apply_paywall_view(&article, None, &[], "tr");
    assert!(locked);

    // Free user: locked
    let free = fixture_free_user(20);
    let (_, locked, _) = apply_paywall_view(&article, Some(&free), &[], "tr");
    assert!(locked);

    // Subscriber: unlocked
    let sub = fixture_subscriber_user(30);
    let (_, locked, _) = apply_paywall_view(&article, Some(&sub), &[], "tr");
    assert!(!locked);

    // Single article purchase: unlocked
    let purchase_code = format!("ARTICLE-{}", article.id);
    let (_, locked, _) = apply_paywall_view(&article, None, &[purchase_code], "tr");
    assert!(!locked);
}

#[test]
fn content_sponsored_articles_bypass_paywall() {
    use msi_dergisi_platform::services::content_service::apply_paywall_view;

    let mut article = fixture_content(5); // seed 5 => is_sponsored = true
    article.visibility_level = 2;
    article.is_sponsored = true;

    let (_, locked, _) = apply_paywall_view(&article, None, &[], "tr");
    assert!(!locked, "Sponsored content should bypass paywall regardless of visibility level");
}

#[test]
fn content_related_articles_exclude_sponsored_current() {
    use msi_dergisi_platform::services::content_service::select_related_articles;

    let mut sponsored = fixture_content(5);
    sponsored.is_sponsored = true;
    let all = vec![fixture_content(1), fixture_content(2), fixture_content(3)];

    let related = select_related_articles(&sponsored, &all, &[], None, 5);
    assert!(related.is_empty(), "No related articles should be shown for sponsored content");
}

#[test]
fn content_related_articles_return_results_for_editorial() {
    use msi_dergisi_platform::services::content_service::select_related_articles;

    let current = fixture_content(1);
    let all = vec![fixture_content(2), fixture_content(3), fixture_content(4), fixture_content(7)];

    let related = select_related_articles(&current, &all, &[], None, 5);
    assert!(!related.is_empty(), "Editorial content should have related article suggestions");
}

#[test]
fn content_fixture_generator_produces_deterministic_data() {
    let a = fixture_content(42);
    let b = fixture_content(42);
    assert_eq!(a.slug, b.slug);
    assert_eq!(a.title_tr, b.title_tr);
    assert_eq!(a.visibility_level, b.visibility_level);
    assert_eq!(a.ssb_tags, b.ssb_tags);
}

#[test]
fn content_db_round_trip_via_test_state() {
    let (state, db_path) = build_test_state("content_crud");
    let contents = state.db.get_all_contents();
    assert!(!contents.is_empty(), "Seeded database should contain content items");

    let categories = state.db.get_all_categories();
    assert!(!categories.is_empty(), "Seeded database should contain categories");

    cleanup_test_db(&db_path);
}