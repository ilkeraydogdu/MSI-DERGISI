use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubscriptionPlan {
 pub code: String,
 pub name_tr: String,
 pub name_en: String,
 pub price_try: f64,
 pub period_label_tr: String,
 pub period_label_en: String,
 pub features_tr: Vec<String>,
 pub features_en: Vec<String>,
 pub is_featured: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommerceProduct {
 pub id: i64,
 pub sku: String,
 /// `book`, `report`, `photo_license`, `single_article`
 pub product_type: String,
 pub title_tr: String,
 pub title_en: String,
 pub author_or_source: String,
 pub description_tr: String,
 pub description_en: String,
 pub price_try: f64,
 pub cover_image: String,
 pub file_resolution_or_pages: String,
 pub is_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderInvoice {
 pub id: i64,
 pub invoice_no: String,
 pub user_email: String,
 pub full_name: String,
 pub organization_name: String,
 pub item_type: String,
 pub item_code: String,
 pub item_title: String,
 pub amount_try: f64,
 pub payment_status: String,
 pub created_at: String,
 pub pdf_watermark_code: String,
}