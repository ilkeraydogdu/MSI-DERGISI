use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryNode {
    pub id: i64,
    pub slug: String,
    pub parent_slug: Option<String>,
    pub name_tr: String,
    pub name_en: String,
    /// Group: `icerikler`, `gruplandirilmis`, `abonelik`, `kurumsal`, `gelecek-faz`
    pub group_type: String,
    pub description_tr: String,
    pub description_en: String,
    pub icon_class: String,
    pub sort_order: i32,
    pub is_dynamic: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SsbInterestTag {
    pub id: i64,
    pub code: String,
    pub name_tr: String,
    pub name_en: String,
    pub category_group: String,
    pub sort_order: i32,
}