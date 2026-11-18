#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_all_categories(&self) -> Vec<CategoryNode> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.categories.clone();
            }
        }
        self.load_categories_db()
    }

    pub(super) fn load_categories_db(&self) -> Vec<CategoryNode> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic
             FROM categories ORDER BY sort_order ASC, id ASC",
            [],
            |r| Ok(CategoryNode {
                id: r.get(0)?,
                slug: r.get(1)?,
                parent_slug: r.get(2)?,
                name_tr: r.get(3)?,
                name_en: r.get(4)?,
                group_type: r.get(5)?,
                description_tr: r.get(6)?,
                description_en: r.get(7)?,
                icon_class: r.get(8)?,
                sort_order: r.get(9)?,
                is_dynamic: r.get::<_, i32>(10)? != 0,
            }),
        )
    }

    pub fn get_category_by_slug(&self, slug: &str) -> Option<CategoryNode> {
        self.get_all_categories()
            .into_iter()
            .find(|c| c.slug.eq_ignore_ascii_case(slug))
    }

    pub fn insert_category(&self, cat: &CategoryNode) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO categories (slug, parent_slug, name_tr, name_en, group_type, description_tr, description_en, icon_class, sort_order, is_dynamic)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                cat.slug,
                cat.parent_slug,
                cat.name_tr,
                cat.name_en,
                cat.group_type,
                cat.description_tr,
                cat.description_en,
                cat.icon_class,
                cat.sort_order,
                if cat.is_dynamic { 1 } else { 0 }
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["categories"]);
        Ok(rowid)
    }

    pub fn get_all_ssb_tags(&self) -> Vec<SsbInterestTag> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.ssb_tags.clone();
            }
        }
        self.load_ssb_tags_db()
    }

    pub(super) fn load_ssb_tags_db(&self) -> Vec<SsbInterestTag> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, code, name_tr, name_en, category_group, sort_order
             FROM ssb_tags ORDER BY sort_order ASC, id ASC",
            [],
            |r| Ok(SsbInterestTag {
                id: r.get(0)?,
                code: r.get(1)?,
                name_tr: r.get(2)?,
                name_en: r.get(3)?,
                category_group: r.get(4)?,
                sort_order: r.get(5)?,
            }),
        )
    }

    pub fn insert_ssb_tag(&self, tag: &SsbInterestTag) -> rusqlite::Result<i64> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO ssb_tags (code, name_tr, name_en, category_group, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                tag.code,
                tag.name_tr,
                tag.name_en,
                tag.category_group,
                tag.sort_order
            ],
        )?;
        let rowid = conn.last_insert_rowid();
        drop(conn);
        self.refresh_l1_sections(&["ssb_tags"]);
        Ok(rowid)
    }

    pub fn update_category(&self, id: i64, cat: &CategoryNode) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let old_slug: String = conn
            .query_row("SELECT slug FROM categories WHERE id = ?1", params![id], |r| r.get(0))
            .map_err(|_| "Kategori bulunamadı.".to_string())?;
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE categories SET
                slug = ?1, parent_slug = ?2, name_tr = ?3, name_en = ?4, group_type = ?5,
                description_tr = ?6, description_en = ?7, icon_class = ?8, sort_order = ?9, is_dynamic = ?10
             WHERE id = ?11",
            params![
                cat.slug.trim(),
                cat.parent_slug,
                cat.name_tr.trim(),
                cat.name_en.trim(),
                cat.group_type.trim(),
                cat.description_tr,
                cat.description_en,
                cat.icon_class.trim(),
                cat.sort_order,
                if cat.is_dynamic { 1 } else { 0 },
                id
            ],
        ).map_err(|e| e.to_string())?;
        if old_slug != cat.slug.trim() {
            let _ = tx.execute(
                "UPDATE content_placements SET category_slug = ?1 WHERE category_slug = ?2",
                params![cat.slug.trim(), old_slug],
            );
            let _ = tx.execute(
                "UPDATE categories SET parent_slug = ?1 WHERE parent_slug = ?2",
                params![cat.slug.trim(), old_slug],
            );
        }
        tx.commit().map_err(|e| e.to_string())?;
        drop(conn);
        self.refresh_l1_sections(&["categories", "contents"]);
        Ok(true)
    }

    pub fn delete_category(&self, id: i64) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let slug: String = conn
            .query_row("SELECT slug FROM categories WHERE id = ?1", params![id], |r| r.get(0))
            .map_err(|_| "Kategori bulunamadı.".to_string())?;
        let refs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_placements WHERE category_slug = ?1",
                params![slug],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if refs > 0 {
            return Err(format!("Bu kategori {} adet içerikte kullanıldığı için silinemez.", refs));
        }
        let deleted = conn
            .execute("DELETE FROM categories WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["categories"]);
        }
        Ok(deleted > 0)
    }

    pub fn update_ssb_tag(&self, old_code: &str, tag: &SsbInterestTag) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let exists: i64 = conn
            .query_row("SELECT COUNT(*) FROM ssb_tags WHERE code = ?1", params![old_code.trim()], |r| r.get(0))
            .unwrap_or(0);
        if exists == 0 {
            return Err("SSB etiketi bulunamadı.".to_string());
        }
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE ssb_tags SET code = ?1, name_tr = ?2, name_en = ?3, category_group = ?4, sort_order = ?5 WHERE code = ?6",
            params![tag.code.trim(), tag.name_tr.trim(), tag.name_en.trim(), tag.category_group.trim(), tag.sort_order, old_code.trim()],
        ).map_err(|e| e.to_string())?;
        if old_code.trim() != tag.code.trim() {
            let _ = tx.execute(
                "UPDATE content_ssb_tags SET ssb_code = ?1 WHERE ssb_code = ?2",
                params![tag.code.trim(), old_code.trim()],
            );
        }
        tx.commit().map_err(|e| e.to_string())?;
        drop(conn);
        self.refresh_l1_sections(&["ssb_tags", "contents"]);
        Ok(true)
    }

    pub fn delete_ssb_tag(&self, code: &str) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let refs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM content_ssb_tags WHERE ssb_code = ?1",
                params![code.trim()],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if refs > 0 {
            return Err(format!("Bu SSB etiketi {} içerikte kullanıldığı için silinemez.", refs));
        }
        let deleted = conn
            .execute("DELETE FROM ssb_tags WHERE code = ?1", params![code.trim()])
            .map_err(|e| e.to_string())?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["ssb_tags"]);
        }
        Ok(deleted > 0)
    }
}
