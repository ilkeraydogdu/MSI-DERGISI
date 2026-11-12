use msi_dergisi_platform::db::Database;
use msi_dergisi_platform::models::ContentItem;
use std::path::PathBuf;

fn make_content(slug: &str, title_tr: &str, body_tr: &str) -> ContentItem {
    ContentItem {
        id: 0,
        slug: slug.to_string(),
        title_tr: title_tr.to_string(),
        title_en: String::new(),
        summary_tr: String::new(),
        summary_en: String::new(),
        body_tr: body_tr.to_string(),
        body_en: String::new(),
        content_type: "haberler".to_string(),
        source_group: "gunluk-haber".to_string(),
        cover_image: String::new(),
        author_name: "Test Yazar".to_string(),
        author_title: "Editör".to_string(),
        issue_number: None,
        published_at: "2026-01-01".to_string(),
        read_time_min: 5,
        visibility_level: 0,
        single_price_try: 0.0,
        is_sponsored: false,
        sponsor_brand: None,
        sponsor_legal_label: None,
        is_featured: false,
        video_url: None,
        view_count: 1,
        true_read_count: 1,
        placements: vec!["haberler".to_string()],
        ssb_tags: vec![],
    }
}

fn main() {
    let tmp_dir = std::env::temp_dir();
    let db_path: PathBuf = tmp_dir.join(format!("msi_fts_smoke_{}.db", std::process::id()));
    let db_path_str = db_path.to_string_lossy().to_string();

    // Also track WAL and SHM files for cleanup
    let wal_path = tmp_dir.join(format!("msi_fts_smoke_{}.db-wal", std::process::id()));
    let shm_path = tmp_dir.join(format!("msi_fts_smoke_{}.db-shm", std::process::id()));

    println!("FTS Smoke Test: creating DB at {}", db_path_str);
    let db = Database::new(&db_path_str).expect("Database::new failed");

    // Insert two content items
    let radar_item = make_content(
        "aselsan-yeni-radar-sistemi",
        "ASELSAN Yeni Radar Sistemi",
        "ASELSAN yeni nesil radar sistemi savunma sanayiinde çığır açıyor.",
    );
    let deniz_item = make_content(
        "deniz-platformlari-modernizasyonu",
        "Deniz Platformları Modernizasyonu",
        "Deniz kuvvetleri platform modernizasyon programı devam ediyor.",
    );

    let radar_id = db.upsert_content(&radar_item).expect("upsert radar item failed");
    let deniz_id = db.upsert_content(&deniz_item).expect("upsert deniz item failed");
    println!("Inserted radar item id={}, deniz item id={}", radar_id, deniz_id);

    // Test 1: search_contents("radar") should return radar item but not deniz item
    let results = db.search_contents("radar");
    let has_radar = results.iter().any(|c| c.slug == "aselsan-yeni-radar-sistemi");
    let has_deniz = results.iter().any(|c| c.slug == "deniz-platformlari-modernizasyonu");
    assert!(has_radar, "FAIL: search_contents('radar') should return radar item");
    assert!(!has_deniz, "FAIL: search_contents('radar') should NOT return deniz item");
    println!("PASS: search_contents('radar') returns correct results");

    // Test 2: SQL injection attempt should not panic and contents table should remain intact
    let injection_query = r#"radar"; DROP TABLE contents; --"#;
    let injection_results = db.search_contents(injection_query);
    // Should not panic; results can be anything (likely empty or fallback)
    println!("PASS: search_contents with injection query did not panic (got {} results)", injection_results.len());

    // Verify contents table is still intact
    let all_after_injection = db.get_all_contents();
    assert!(
        all_after_injection.len() >= 2,
        "FAIL: contents table should still have at least 2 rows after injection attempt, got {}",
        all_after_injection.len()
    );
    println!("PASS: contents table intact after injection attempt ({} rows)", all_after_injection.len());

    // Test 3: Update title via upsert_content and verify search reflects change
    let mut updated_radar = radar_item.clone();
    updated_radar.id = radar_id;
    updated_radar.title_tr = "ASELSAN Sonar Sensör Paketi".to_string();
    updated_radar.body_tr = "Sonar sensör paketi deniz platformları için geliştirildi.".to_string();
    db.upsert_content(&updated_radar).expect("upsert update failed");

    let sonar_results = db.search_contents("sonar");
    let has_sonar = sonar_results.iter().any(|c| c.id == radar_id);
    assert!(has_sonar, "FAIL: search_contents('sonar') should return updated item");

    let radar_results_after = db.search_contents("radar");
    let still_has_radar_title = radar_results_after.iter().any(|c| c.id == radar_id && c.title_tr.contains("Radar"));
    assert!(!still_has_radar_title, "FAIL: updated item should no longer have 'Radar' in title");
    println!("PASS: search reflects title update");

    // Test 4: Delete content and verify search no longer returns it
    db.delete_content(radar_id).expect("delete_content failed");
    let after_delete = db.search_contents("sonar");
    let has_deleted = after_delete.iter().any(|c| c.id == radar_id);
    assert!(!has_deleted, "FAIL: deleted item should not appear in search results");
    println!("PASS: deleted item no longer in search results");

    // Cleanup temp files
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(&wal_path);
    let _ = std::fs::remove_file(&shm_path);
    println!("PASS: temp DB files cleaned up");

    println!("\nAll FTS smoke tests PASSED.");
}