//! Integration tests for SEO generation: sitemap, robots.txt, JSON-LD, RSS.

mod common;

use common::*;
use msi_dergisi_platform::engine::seo_router_engine::SeoBrandConfig;
use std::collections::HashMap;

fn test_brand() -> SeoBrandConfig {
    SeoBrandConfig::from_settings(&HashMap::new())
}

#[test]
fn seo_robots_txt_contains_required_directives() {
    let robots = msi_dergisi_platform::engine::seo_router_engine::generate_robots_txt(
        "https://test.msidergisi.com",
    );
    assert_valid_robots_txt(&robots);
    assert!(robots.contains("Host: test.msidergisi.com"));
}

#[test]
fn seo_rss_feed_is_valid_xml() {
    let contents = (0..3).map(fixture_content).collect::<Vec<_>>();
    let brand = test_brand();
    let rss = msi_dergisi_platform::engine::seo_router_engine::generate_rss_xml(
        "https://test.msidergisi.com",
        &contents,
        "tr",
        &brand,
    );
    assert!(rss.starts_with("<?xml"));
    assert!(rss.contains("<rss"));
    assert!(rss.contains("</rss>"));
    assert!(rss.contains("<item>"));
}

#[test]
fn seo_llms_txt_includes_issn_and_content_links() {
    let contents = vec![fixture_content(1)];
    let brand = test_brand();
    let llms =
        msi_dergisi_platform::engine::seo_router_engine::CentralSeoEngine::generate_llms_txt(
            "https://test.msidergisi.com",
            &[],
            &[],
            &[],
            &contents,
            false,
            &brand,
        );
    assert!(llms.contains("ISSN: 1306-0309"));
    assert!(llms.contains("/icerik/test-article-1"));
}

#[test]
fn seo_sitemap_xml_is_well_formed() {
    let contents = (0..5).map(fixture_content).collect::<Vec<_>>();
    let brand = test_brand();
    let xml = msi_dergisi_platform::engine::seo_router_engine::generate_sitemap_xml(
        "https://test.msidergisi.com",
        &[],
        &contents,
        &[],
        &brand,
    );
    assert_valid_sitemap(&xml);
    assert!(xml.contains("https://test.msidergisi.com/icerik/test-article-"));
}

#[test]
fn seo_news_sitemap_includes_publication_dates() {
    let contents = vec![fixture_content(1), fixture_content(2)];
    let brand = test_brand();
    let xml =
        msi_dergisi_platform::engine::seo_router_engine::CentralSeoEngine::generate_news_sitemap_xml(
            "https://test.msidergisi.com",
            &contents,
            &brand,
        );
    assert!(xml.contains("<news:publication_date>"));
    assert!(xml.contains("<news:name>"));
}