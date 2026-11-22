#[allow(unused_imports)]
use super::*;

impl Database {
    pub fn get_subscription_plans(&self) -> Vec<SubscriptionPlan> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.subscription_plans.clone();
            }
        }
        self.load_subscription_plans_db()
    }

    pub(super) fn load_subscription_plans_db(&self) -> Vec<SubscriptionPlan> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT code, name_tr, name_en, price_try, period_label_tr, period_label_en,
                    features_tr_json, features_en_json, is_featured
             FROM subscription_plans ORDER BY sort_order ASC",
            [],
            |r| {
                let f_tr: String = r.get(6)?;
                let f_en: String = r.get(7)?;
                Ok(SubscriptionPlan {
                    code: r.get(0)?,
                    name_tr: r.get(1)?,
                    name_en: r.get(2)?,
                    price_try: r.get(3)?,
                    period_label_tr: r.get(4)?,
                    period_label_en: r.get(5)?,
                    features_tr: serde_json::from_str(&f_tr).unwrap_or_default(),
                    features_en: serde_json::from_str(&f_en).unwrap_or_default(),
                    is_featured: r.get::<_, i32>(8)? != 0,
                })
            },
        )
    }

    pub fn get_commerce_products(&self) -> Vec<CommerceProduct> {
        if let Ok(guard) = self.l1_cache.read() {
            if let Some(ref snap) = *guard {
                return snap.commerce_products.clone();
            }
        }
        self.load_commerce_products_db()
    }

    pub(super) fn load_commerce_products_db(&self) -> Vec<CommerceProduct> {
        crate::db::query_helpers::query_all(
            self,
            "SELECT id, sku, product_type, title_tr, title_en, author_or_source,
                    description_tr, description_en, price_try, cover_image,
                    file_resolution_or_pages, is_available
             FROM commerce_products ORDER BY id ASC",
            [],
            |r| Ok(CommerceProduct {
                id: r.get(0)?,
                sku: r.get(1)?,
                product_type: r.get(2)?,
                title_tr: r.get(3)?,
                title_en: r.get(4)?,
                author_or_source: r.get(5)?,
                description_tr: r.get(6)?,
                description_en: r.get(7)?,
                price_try: r.get(8)?,
                cover_image: r.get(9)?,
                file_resolution_or_pages: r.get(10)?,
                is_available: r.get::<_, i32>(11)? != 0,
            }),
        )
    }

    /// Returns revenue summary grouped by date for an inclusive `YYYY-MM-DD` range.
    /// Uses `SUBSTR(created_at, 1, 10)` so both space-delimited and ISO-8601 timestamps match accurately.
    pub fn get_revenue_summary(&self, date_from: &str, date_to: &str) -> serde_json::Value {
        let Ok(conn) = self.open_conn() else {
            return serde_json::json!({
                "daily": [],
                "total_amount": 0.0,
                "weekly_total": 0.0,
                "monthly_total": 0.0,
                "invoice_count": 0,
                "by_item_type": {},
                "date_range": { "from": date_from, "to": date_to }
            });
        };

        let df = date_from.trim();
        let dt = date_to.trim();

        let daily_sql = "SELECT SUBSTR(created_at, 1, 10) AS date, COALESCE(SUM(amount_try), 0) AS total
                         FROM order_invoices
                         WHERE payment_status = 'paid'
                           AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                           AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)
                         GROUP BY date ORDER BY date ASC";
        let daily: Vec<serde_json::Value> = match conn.prepare(daily_sql) {
            Ok(mut stmt) => stmt
                .query_map(rusqlite::params![df, dt], |r| {
                    Ok(serde_json::json!({
                        "date": r.get::<_, String>(0)?,
                        "amount": r.get::<_, f64>(1)?
                    }))
                })
                .map(|iter| iter.filter_map(Result::ok).collect())
                .unwrap_or_default(),
            Err(_) => Vec::new(),
        };

        let total: f64 = daily.iter().filter_map(|d| d["amount"].as_f64()).sum();
        let weekly_total: f64 = daily
            .iter()
            .rev()
            .take(7)
            .filter_map(|d| d["amount"].as_f64())
            .sum();

        let type_sql = "SELECT item_type, COALESCE(SUM(amount_try), 0) AS total
                        FROM order_invoices
                        WHERE payment_status = 'paid'
                          AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                          AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)
                        GROUP BY item_type";
        let by_type: serde_json::Map<String, serde_json::Value> = match conn.prepare(type_sql) {
            Ok(mut stmt) => stmt
                .query_map(rusqlite::params![df, dt], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                })
                .map(|iter| {
                    iter.filter_map(Result::ok)
                        .map(|(k, v)| (k, serde_json::json!(v)))
                        .collect()
                })
                .unwrap_or_default(),
            Err(_) => serde_json::Map::new(),
        };

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM order_invoices
                 WHERE payment_status = 'paid'
                   AND SUBSTR(created_at, 1, 10) >= SUBSTR(?1, 1, 10)
                   AND SUBSTR(created_at, 1, 10) <= SUBSTR(?2, 1, 10)",
                rusqlite::params![df, dt],
                |r| r.get(0),
            )
            .unwrap_or(0);

        serde_json::json!({
            "daily": daily,
            "total_amount": (total * 100.0).round() / 100.0,
            "weekly_total": (weekly_total * 100.0).round() / 100.0,
            "monthly_total": (total * 100.0).round() / 100.0,
            "invoice_count": count,
            "by_item_type": by_type,
            "date_range": { "from": df, "to": dt }
        })
    }

    pub fn get_order_invoices(&self) -> Vec<OrderInvoice> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT id, invoice_no, user_email, full_name, organization_name,
                    item_type, item_code, item_title, amount_try, payment_status,
                    created_at, pdf_watermark_code
             FROM order_invoices ORDER BY id DESC",
        ) else {
            return Vec::new();
        };
        let rows = stmt.query_map([], |r| {
            Ok(OrderInvoice {
                id: r.get(0)?,
                invoice_no: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                item_type: r.get(5)?,
                item_code: r.get(6)?,
                item_title: r.get(7)?,
                amount_try: r.get(8)?,
                payment_status: r.get(9)?,
                created_at: r.get(10)?,
                pdf_watermark_code: r.get(11)?,
            })
        });
        match rows {
            Ok(iter) => iter.filter_map(Result::ok).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn insert_order_invoice(&self, inv: &OrderInvoice) -> rusqlite::Result<(i64, String)> {
        // Retry loop for invoice_no UNIQUE constraint collisions.
        // On collision, regenerate invoice_no with fresh entropy and return the persisted invoice_no.
        let mut current_inv = inv.clone();
        let max_attempts = 5;

        for attempt in 0..max_attempts {
            let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            let result = conn.execute(
                "INSERT INTO order_invoices (
                    invoice_no, user_email, full_name, organization_name,
                    item_type, item_code, item_title, amount_try, payment_status,
                    created_at, pdf_watermark_code
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    current_inv.invoice_no,
                    current_inv.user_email,
                    current_inv.full_name,
                    current_inv.organization_name,
                    current_inv.item_type,
                    current_inv.item_code,
                    current_inv.item_title,
                    current_inv.amount_try,
                    current_inv.payment_status,
                    current_inv.created_at,
                    current_inv.pdf_watermark_code
                ],
            );

            match result {
                Ok(_) => {
                    let rowid = conn.last_insert_rowid();
                    drop(conn);

                    if current_inv.payment_status == "paid" {
                        self.activate_invoice_entitlements(&current_inv);
                    }
                    return Ok((rowid, current_inv.invoice_no));
                }
                Err(e) => {
                    if matches!(e.sqlite_error_code(), Some(rusqlite::ErrorCode::ConstraintViolation))
                        && attempt < max_attempts - 1
                    {
                        let now = chrono::Utc::now();
                        let year_str = now.format("%Y").to_string();
                        let rand_suffix =
                            (uuid::Uuid::new_v4().as_u128() % 900_000 + 100_000) as u32;
                        current_inv.invoice_no = format!("MSI-{}-{:06}", year_str, rand_suffix);
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(rusqlite::Error::InvalidParameterName(
            "Invoice number collision after max retries".to_string(),
        ))
    }

    pub fn get_commerce_product_delivery_asset(&self, sku: &str) -> Option<(CommerceProduct, String)> {
        let norm = sku.trim();
        if norm.is_empty() {
            return None;
        }
        crate::db::query_helpers::query_one(
            self,
            "SELECT id, sku, product_type, title_tr, title_en, author_or_source,
                    description_tr, description_en, price_try, cover_image,
                    file_resolution_or_pages, is_available, COALESCE(delivery_asset_url, '')
             FROM commerce_products WHERE LOWER(sku) = LOWER(?1) LIMIT 1",
            params![norm],
            |r| {
                let prod = CommerceProduct {
                    id: r.get(0)?,
                    sku: r.get(1)?,
                    product_type: r.get(2)?,
                    title_tr: r.get(3)?,
                    title_en: r.get(4)?,
                    author_or_source: r.get(5)?,
                    description_tr: r.get(6)?,
                    description_en: r.get(7)?,
                    price_try: r.get(8)?,
                    cover_image: r.get(9)?,
                    file_resolution_or_pages: r.get(10)?,
                    is_available: r.get::<_, i32>(11)? != 0,
                };
                let asset: String = r.get(12)?;
                Ok((prod, asset))
            },
        )
    }

    pub fn user_has_purchased_item(&self, email: &str, code: &str) -> bool {
        let target = code.trim();
        if email.trim().is_empty() || target.is_empty() {
            return false;
        }
        if self
            .get_user_purchased_codes(email)
            .into_iter()
            .any(|c| {
                c.eq_ignore_ascii_case(target)
                    || c.split(&[',', ';'][..])
                        .map(|tok| tok.trim().strip_prefix("product:").unwrap_or(tok.trim()))
                        .any(|tok| tok.eq_ignore_ascii_case(target))
            })
        {
            return true;
        }
        let Ok(conn) = self.open_conn() else {
            return false;
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT item_code FROM order_invoices
             WHERE LOWER(user_email) = LOWER(?1) AND payment_status = 'paid'",
        ) else {
            return false;
        };
        let Ok(rows) = stmt.query_map(params![email.trim()], |r| r.get::<_, String>(0)) else {
            return false;
        };
        let matched = rows.filter_map(Result::ok).any(|raw| {
            raw.split(&[',', ';'][..])
                .map(|tok| {
                    let t = tok.trim();
                    t.strip_prefix("product:").unwrap_or(t)
                })
                .any(|tok| tok.eq_ignore_ascii_case(target))
        });
        matched
    }

    pub fn activate_invoice_entitlements(&self, inv: &OrderInvoice) {
        let mut sub_codes: Vec<(String, String)> = Vec::new();
        if inv.item_type == "cart" {
            for raw_tok in inv.item_code.split(&[',', ';'][..]) {
                let tok = raw_tok.trim();
                if tok.is_empty() {
                    continue;
                }
                if let Some(sub_code) = tok.strip_prefix("subscription:") {
                    sub_codes.push((sub_code.trim().to_string(), inv.item_title.clone()));
                    let _ = self.record_user_purchase(&inv.user_email, sub_code.trim());
                } else {
                    let sku = tok.strip_prefix("product:").unwrap_or(tok).trim();
                    if !sku.is_empty() {
                        let _ = self.record_user_purchase(&inv.user_email, sku);
                    }
                }
            }
        } else {
            let _ = self.record_user_purchase(&inv.user_email, &inv.item_code);
            if inv.item_type == "subscription" {
                sub_codes.push((inv.item_code.clone(), inv.item_title.clone()));
            }
        }

        if let Some((sub_code, sub_title)) = sub_codes.into_iter().next() {
            let dynamic_exp = (chrono::Utc::now() + chrono::Duration::days(365))
                .format("%Y-%m-%d")
                .to_string();
            let norm_sub = sub_code.trim().to_ascii_lowercase();
            let assigned_role = match norm_sub.as_str() {
                "kurumsal" => "subscriber_corporate",
                "basili-dijital" => "subscriber_print_digital",
                _ => "subscriber_digital",
            };
            let tier_title = self
                .get_subscription_plans()
                .into_iter()
                .find(|p| p.code.eq_ignore_ascii_case(&norm_sub))
                .map(|p| p.name_tr)
                .unwrap_or(sub_title);
            if let Ok(mut conn) = self.open_conn() {
                if let Ok(tx) = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) {
                    let _ = tx.execute(
                        "UPDATE renewal_reminders SET status = 'halted_by_renewal' WHERE LOWER(user_email) = LOWER(?1) AND status = 'scheduled'",
                        params![inv.user_email],
                    );
                    let _ = tx.execute(
                        "UPDATE users SET
                            reminders_halted = 1,
                            role = CASE
                                WHEN role IN ('site_operator', 'admin', 'super_admin')
                                  OR EXISTS (
                                      SELECT 1 FROM user_roles ur
                                      WHERE ur.role_code = users.role
                                        AND ur.role_code NOT IN ('reader', 'registered', 'subscriber_public', 'subscriber_corporate', 'subscriber_print_digital', 'subscriber_digital')
                                  )
                                THEN role
                                ELSE ?1
                            END,
                            subscription_tier = ?2,
                            subscription_expires_at = ?3
                         WHERE LOWER(email) = LOWER(?4)",
                        params![assigned_role, tier_title, dynamic_exp, inv.user_email],
                    );
                    let _ = tx.commit();
                }
            }
            self.refresh_l1_sections(&["users"]);
        }
    }

    pub fn finalize_paid_invoice(&self, merchant_oid: &str) -> Option<OrderInvoice> {
        self.finalize_paid_invoice_strict(merchant_oid)
            .map(|(inv, _)| inv)
    }

    /// Atomically finalizes a paid invoice from PayTR callback (`merchant_oid`).
    /// Returns `(invoice, newly_paid)` so callers activate entitlements idempotently
    /// only when the invoice actually transitioned from non-paid to `paid`.
    pub fn finalize_paid_invoice_strict(&self, merchant_oid: &str) -> Option<(OrderInvoice, bool)> {
        let oid = merchant_oid.trim();
        if oid.is_empty() {
            return None;
        }

        let mut conn = self.open_conn().ok()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .ok()?;

        let mut target: OrderInvoice = {
            let mut stmt = tx
                .prepare(
                    "SELECT id, invoice_no, user_email, full_name, organization_name,
                            item_type, item_code, item_title, amount_try, payment_status,
                            created_at, pdf_watermark_code
                     FROM order_invoices
                     WHERE UPPER(invoice_no) = UPPER(?1)
                        OR UPPER(REPLACE(invoice_no, '-', '')) = UPPER(?1)
                        OR UPPER('MSI' || REPLACE(invoice_no, '-', '')) = UPPER(?1)
                     LIMIT 1",
                )
                .ok()?;
            stmt.query_row(params![oid], |r| {
                Ok(OrderInvoice {
                    id: r.get(0)?,
                    invoice_no: r.get(1)?,
                    user_email: r.get(2)?,
                    full_name: r.get(3)?,
                    organization_name: r.get(4)?,
                    item_type: r.get(5)?,
                    item_code: r.get(6)?,
                    item_title: r.get(7)?,
                    amount_try: r.get(8)?,
                    payment_status: r.get(9)?,
                    created_at: r.get(10)?,
                    pdf_watermark_code: r.get(11)?,
                })
            })
            .ok()?
        };

        let affected = tx
            .execute(
                "UPDATE order_invoices SET payment_status = 'paid'
                 WHERE id = ?1 AND payment_status != 'paid'",
                params![target.id],
            )
            .ok()?;

        if affected == 0 {
            drop(tx);
            return Some((target, false));
        }

        target.payment_status = "paid".to_string();
        let now_rfc = chrono::Utc::now().to_rfc3339();

        if target.item_type == "cart" {
            for raw_tok in target.item_code.split(&[',', ';'][..]) {
                let t = raw_tok.trim();
                if t.is_empty() {
                    continue;
                }
                let clean = t
                    .strip_prefix("product:")
                    .or_else(|| t.strip_prefix("subscription:"))
                    .unwrap_or(t)
                    .trim();
                if !clean.is_empty() {
                    let _ = tx.execute(
                        "INSERT INTO user_purchased_items (user_email, item_code, created_at)
                         VALUES (?1, ?2, ?3) ON CONFLICT(user_email, item_code) DO NOTHING",
                        params![target.user_email.trim().to_lowercase(), clean, now_rfc],
                    );
                }
            }
        } else {
            let _ = tx.execute(
                "INSERT INTO user_purchased_items (user_email, item_code, created_at)
                 VALUES (?1, ?2, ?3) ON CONFLICT(user_email, item_code) DO NOTHING",
                params![target.user_email.trim().to_lowercase(), target.item_code.trim(), now_rfc],
            );
        }

        if tx.commit().is_err() {
            return None;
        }
        drop(conn);

        Some((target, true))
    }

    pub fn mark_invoice_failed(&self, merchant_oid: &str) -> bool {
        let oid = merchant_oid.trim();
        if oid.is_empty() {
            return false;
        }
        let Ok(mut conn) = self.open_conn() else {
            return false;
        };
        let Ok(tx) = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate) else {
            return false;
        };
        let target_opt: Option<(i64, String)> = tx
            .query_row(
                "SELECT id, payment_status FROM order_invoices
                 WHERE UPPER(invoice_no) = UPPER(?1)
                    OR UPPER(REPLACE(invoice_no, '-', '')) = UPPER(?1)
                    OR UPPER('MSI' || REPLACE(invoice_no, '-', '')) = UPPER(?1)
                 LIMIT 1",
                params![oid],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .ok();

        let Some((id, status)) = target_opt else {
            return false;
        };
        if status != "paid" {
            let affected = tx
                .execute(
                    "UPDATE order_invoices SET payment_status = 'failed' WHERE id = ?1 AND payment_status != 'paid'",
                    params![id],
                )
                .unwrap_or(0);
            if tx.commit().is_ok() && affected > 0 {
                return true;
            }
        }
        false
    }

    pub fn get_user_purchased_codes(&self, email: &str) -> Vec<String> {
        let Ok(conn) = self.open_conn() else {
            return Vec::new();
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT item_code FROM user_purchased_items WHERE LOWER(user_email) = LOWER(?1)",
        ) else {
            return Vec::new();
        };
        let Ok(rows) = stmt.query_map(params![email.trim()], |r| r.get::<_, String>(0)) else {
            return Vec::new();
        };
        rows.filter_map(Result::ok).collect()
    }

    pub fn record_user_purchase(&self, email: &str, item_code: &str) -> rusqlite::Result<()> {
        let conn = self.open_conn().map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO user_purchased_items (user_email, item_code, created_at)
             VALUES (?1, ?2, ?3) ON CONFLICT(user_email, item_code) DO NOTHING",
            params![email.trim().to_lowercase(), item_code.trim(), now],
        )?;
        Ok(())
    }

    pub fn upsert_subscription_plan(&self, plan: &SubscriptionPlan, sort_order: i32) -> Result<(), String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let f_tr = serde_json::to_string(&plan.features_tr).unwrap_or_else(|_| "[]".to_string());
        let f_en = serde_json::to_string(&plan.features_en).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO subscription_plans (
                code, name_tr, name_en, price_try, period_label_tr, period_label_en,
                features_tr_json, features_en_json, is_featured, sort_order
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(code) DO UPDATE SET
                name_tr = excluded.name_tr,
                name_en = excluded.name_en,
                price_try = excluded.price_try,
                period_label_tr = excluded.period_label_tr,
                period_label_en = excluded.period_label_en,
                features_tr_json = excluded.features_tr_json,
                features_en_json = excluded.features_en_json,
                is_featured = excluded.is_featured,
                sort_order = excluded.sort_order",
            params![
                plan.code.trim(),
                plan.name_tr.trim(),
                plan.name_en.trim(),
                plan.price_try,
                plan.period_label_tr.trim(),
                plan.period_label_en.trim(),
                f_tr,
                f_en,
                if plan.is_featured { 1 } else { 0 },
                sort_order
            ],
        ).map_err(|e| e.to_string())?;
        drop(conn);
        self.refresh_l1_sections(&["subscription_plans"]);
        Ok(())
    }

    pub fn delete_subscription_plan(&self, code: &str) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let deleted = conn
            .execute("DELETE FROM subscription_plans WHERE code = ?1", params![code.trim()])
            .map_err(|e| e.to_string())?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["subscription_plans"]);
        }
        Ok(deleted > 0)
    }

    pub fn upsert_commerce_product(&self, prod: &CommerceProduct) -> Result<i64, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO commerce_products (
                sku, product_type, title_tr, title_en, author_or_source,
                description_tr, description_en, price_try, cover_image,
                file_resolution_or_pages, is_available
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT(sku) DO UPDATE SET
                product_type = excluded.product_type,
                title_tr = excluded.title_tr,
                title_en = excluded.title_en,
                author_or_source = excluded.author_or_source,
                description_tr = excluded.description_tr,
                description_en = excluded.description_en,
                price_try = excluded.price_try,
                cover_image = excluded.cover_image,
                file_resolution_or_pages = excluded.file_resolution_or_pages,
                is_available = excluded.is_available",
            params![
                prod.sku.trim(),
                prod.product_type.trim(),
                prod.title_tr.trim(),
                prod.title_en.trim(),
                prod.author_or_source.trim(),
                prod.description_tr,
                prod.description_en,
                prod.price_try,
                prod.cover_image.trim(),
                prod.file_resolution_or_pages.trim(),
                if prod.is_available { 1 } else { 0 }
            ],
        ).map_err(|e| e.to_string())?;
        let id = conn
            .query_row("SELECT id FROM commerce_products WHERE sku = ?1", params![prod.sku.trim()], |r| r.get(0))
            .unwrap_or(0);
        drop(conn);
        self.refresh_l1_sections(&["commerce_products"]);
        Ok(id)
    }

    pub fn delete_commerce_product(&self, id: i64) -> Result<bool, String> {
        let conn = self.open_conn().map_err(|e| e.to_string())?;
        let deleted = conn
            .execute("DELETE FROM commerce_products WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        drop(conn);
        if deleted > 0 {
            self.refresh_l1_sections(&["commerce_products"]);
        }
        Ok(deleted > 0)
    }

    pub fn get_invoice_by_oid(&self, merchant_oid: &str) -> Option<OrderInvoice> {
        let oid = merchant_oid.trim();
        let conn = self.open_conn().ok()?;
        let mut stmt = conn.prepare(
            "SELECT id, invoice_no, user_email, full_name, organization_name,
                    item_type, item_code, item_title, amount_try, payment_status,
                    created_at, pdf_watermark_code
             FROM order_invoices
             WHERE UPPER(invoice_no) = UPPER(?1)
                OR UPPER(REPLACE(invoice_no, '-', '')) = UPPER(?1)
                OR UPPER('MSI' || REPLACE(invoice_no, '-', '')) = UPPER(?1)
             LIMIT 1",
        ).ok()?;
        stmt.query_row(params![oid], |r| {
            Ok(OrderInvoice {
                id: r.get(0)?,
                invoice_no: r.get(1)?,
                user_email: r.get(2)?,
                full_name: r.get(3)?,
                organization_name: r.get(4)?,
                item_type: r.get(5)?,
                item_code: r.get(6)?,
                item_title: r.get(7)?,
                amount_try: r.get(8)?,
                payment_status: r.get(9)?,
                created_at: r.get(10)?,
                pdf_watermark_code: r.get(11)?,
            })
        }).ok()
    }

    pub fn mark_invoice_paid_by_id(&self, id: i64) -> Option<OrderInvoice> {
        self.mark_invoice_paid_by_id_strict(id)
            .map(|(inv, _)| inv)
    }

    pub fn mark_invoice_paid_by_id_strict(&self, id: i64) -> Option<(OrderInvoice, bool)> {
        let conn = self.open_conn().ok()?;
        let inv: OrderInvoice = conn
            .query_row(
                "SELECT id, invoice_no, user_email, full_name, organization_name,
                        item_type, item_code, item_title, amount_try, payment_status,
                        created_at, pdf_watermark_code
                 FROM order_invoices WHERE id = ?1",
                params![id],
                |r| {
                    Ok(OrderInvoice {
                        id: r.get(0)?,
                        invoice_no: r.get(1)?,
                        user_email: r.get(2)?,
                        full_name: r.get(3)?,
                        organization_name: r.get(4)?,
                        item_type: r.get(5)?,
                        item_code: r.get(6)?,
                        item_title: r.get(7)?,
                        amount_try: r.get(8)?,
                        payment_status: r.get(9)?,
                        created_at: r.get(10)?,
                        pdf_watermark_code: r.get(11)?,
                    })
                },
            )
            .ok()?;
        drop(conn);
        self.finalize_paid_invoice_strict(&inv.invoice_no)
    }
}
