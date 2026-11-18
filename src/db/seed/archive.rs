use rusqlite::{params, Connection};
use serde_json::Value;
use std::path::Path;

pub fn seed_archive_from_json(conn: &Connection) -> rusqlite::Result<()> {
    let archive_path = std::env::var("MSI_ARCHIVE_JSON_PATH")
        .unwrap_or_else(|_| "storage/seed/live_msi_archive.json".to_string());
    if !Path::new(&archive_path).exists() {
        return Ok(());
    }
    let Ok(raw) = std::fs::read_to_string(&archive_path) else {
        return Ok(());
    };
    let Ok(doc) = serde_json::from_str::<Value>(&raw) else {
        return Ok(());
    };

    conn.execute_batch("BEGIN IMMEDIATE;")?;

    if let Some(cats) = doc.get("categories").and_then(|v| v.as_array()) {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"
        )?;
        for c in cats {
            let _ = stmt.execute(params![
                c["slug"].as_str().unwrap_or(""),
                c["parent_slug"].as_str(),
                c["name_tr"].as_str().unwrap_or(""),
                c["name_en"].as_str().unwrap_or(""),
                c["group_type"].as_str().unwrap_or("icerikler"),
                c["description_tr"].as_str().unwrap_or(""),
                c["description_en"].as_str().unwrap_or(""),
                c["icon_class"].as_str().unwrap_or("ph-newspaper"),
                c["sort_order"].as_i64().unwrap_or(0),
                c["is_dynamic"].as_i64().unwrap_or(1),
            ]);
        }
    }

    if let Some(issues) = doc.get("issues").and_then(|v| v.as_array()) {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO magazine_issues (
                issue_number, slug, title_tr, title_en, publish_month_tr, publish_month_en,
                year, cover_image, is_special_issue, event_name, page_count, dossier_count,
                summary_tr, summary_en, pdf_path, spreads_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"
        )?;
        for iss in issues {
            let _ = stmt.execute(params![
                iss["issue_number"].as_i64().unwrap_or(0),
                iss["slug"].as_str().unwrap_or(""),
                iss["title_tr"].as_str().unwrap_or(""),
                iss["title_en"].as_str().unwrap_or(""),
                iss["publish_month_tr"].as_str().unwrap_or(""),
                iss["publish_month_en"].as_str().unwrap_or(""),
                iss["year"].as_i64().unwrap_or(2026),
                iss["cover_image"].as_str().unwrap_or(""),
                iss["is_special_issue"].as_i64().unwrap_or(0),
                iss["event_name"].as_str(),
                iss["page_count"].as_i64().unwrap_or(96),
                iss["dossier_count"].as_i64().unwrap_or(8),
                iss["summary_tr"].as_str().unwrap_or(""),
                iss["summary_en"].as_str().unwrap_or(""),
                iss["pdf_path"].as_str().unwrap_or(""),
                iss["spreads_json"].as_str().unwrap_or("[]"),
            ]);
        }
    }

    if let Some(contents) = doc.get("contents").and_then(|v| v.as_array()) {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO contents (
                slug, title_tr, title_en, summary_tr, summary_en, body_tr, body_en,
                content_type, source_group, cover_image, author_name, author_title,
                issue_number, published_at, read_time_min, visibility_level, single_price_try,
                is_sponsored, sponsor_brand, sponsor_legal_label, is_featured, video_url,
                view_count, true_read_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)"
        )?;
        let mut pl_stmt = conn.prepare("INSERT OR IGNORE INTO content_placements (content_id, category_slug) VALUES (?1, ?2)")?;
        let mut ssb_stmt = conn.prepare("INSERT OR IGNORE INTO content_ssb_tags (content_id, ssb_code) VALUES (?1, ?2)")?;
        for c in contents {
            let slug = c["slug"].as_str().unwrap_or("");
            if slug.is_empty() { continue; }
            let _ = stmt.execute(params![
                slug,
                c["title_tr"].as_str().unwrap_or(""),
                c["title_en"].as_str().unwrap_or(""),
                c["summary_tr"].as_str().unwrap_or(""),
                c["summary_en"].as_str().unwrap_or(""),
                c["body_tr"].as_str().unwrap_or(""),
                c["body_en"].as_str().unwrap_or(""),
                c["content_type"].as_str().unwrap_or("haberler"),
                c["source_group"].as_str().unwrap_or("gunluk-haber"),
                c["cover_image"].as_str().unwrap_or(""),
                c["author_name"].as_str().unwrap_or("MSI Dergisi"),
                c["author_title"].as_str().unwrap_or("Yayın Kurulu"),
                c["issue_number"].as_i64(),
                c["published_at"].as_str().unwrap_or(""),
                c["read_time_min"].as_i64().unwrap_or(5),
                c["visibility_level"].as_i64().unwrap_or(0),
                c["single_price_try"].as_f64().unwrap_or(0.0),
                c["is_sponsored"].as_i64().unwrap_or(0),
                c["sponsor_brand"].as_str(),
                c["sponsor_legal_label"].as_str(),
                c["is_featured"].as_i64().unwrap_or(0),
                c["video_url"].as_str(),
                c["view_count"].as_i64().unwrap_or(0),
                c["true_read_count"].as_i64().unwrap_or(0),
            ]);
            if let Ok(cid) = conn.query_row("SELECT id FROM contents WHERE slug = ?1", params![slug], |r| r.get::<_, i64>(0)) {
                if let Some(pls) = c["placements"].as_array() {
                    for p in pls {
                        if let Some(ps) = p.as_str() {
                            let _ = pl_stmt.execute(params![cid, ps]);
                        }
                    }
                }
                if let Some(tags) = c["ssb_tags"].as_array() {
                    for t in tags {
                        if let Some(ts) = t.as_str() {
                            let _ = ssb_stmt.execute(params![cid, ts]);
                        }
                    }
                }
            }
        }
    }

    if let Some(rds) = doc.get("seo_redirects").and_then(|v| v.as_array()) {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO seo_redirects (old_path, new_path, status_code, hit_count) VALUES (?1, ?2, ?3, 0)"
        )?;
        for rd in rds {
            let _ = stmt.execute(params![
                rd["old_path"].as_str().unwrap_or(""),
                rd["new_path"].as_str().unwrap_or(""),
                rd["status_code"].as_i64().unwrap_or(301),
            ]);
        }
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}
