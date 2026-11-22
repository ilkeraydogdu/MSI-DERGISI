-- Migration 001: Initial PostgreSQL 16 schema (translated from src/db/schema.rs baseline)
-- 28 core tables + 19 baseline indexes (including idx_users_email_lower)

CREATE TABLE IF NOT EXISTS categories (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    parent_slug TEXT,
    name_tr TEXT NOT NULL,
    name_en TEXT NOT NULL,
    group_type TEXT NOT NULL,
    description_tr TEXT NOT NULL,
    description_en TEXT NOT NULL,
    icon_class TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_dynamic BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS ssb_tags (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name_tr TEXT NOT NULL,
    name_en TEXT NOT NULL,
    category_group TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS contents (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title_tr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    summary_tr TEXT NOT NULL,
    summary_en TEXT NOT NULL,
    body_tr TEXT NOT NULL,
    body_en TEXT NOT NULL,
    content_type TEXT NOT NULL,
    source_group TEXT NOT NULL,
    cover_image TEXT NOT NULL,
    author_name TEXT NOT NULL,
    author_title TEXT NOT NULL,
    issue_number INTEGER,
    published_at TEXT NOT NULL,
    read_time_min INTEGER NOT NULL DEFAULT 5,
    visibility_level INTEGER NOT NULL DEFAULT 0,
    single_price_try DOUBLE PRECISION NOT NULL DEFAULT 0.0,
    is_sponsored BOOLEAN NOT NULL DEFAULT FALSE,
    sponsor_brand TEXT,
    sponsor_legal_label TEXT,
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    video_url TEXT,
    view_count BIGINT NOT NULL DEFAULT 0,
    true_read_count BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS content_placements (
    content_id BIGINT NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    category_slug TEXT NOT NULL,
    PRIMARY KEY (content_id, category_slug)
);

CREATE INDEX IF NOT EXISTS idx_content_placements_cat ON content_placements(category_slug);
CREATE INDEX IF NOT EXISTS idx_content_placements_composite ON content_placements(content_id, category_slug);

CREATE TABLE IF NOT EXISTS content_ssb_tags (
    content_id BIGINT NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    ssb_code TEXT NOT NULL,
    PRIMARY KEY (content_id, ssb_code)
);

CREATE INDEX IF NOT EXISTS idx_content_ssb_tags_code ON content_ssb_tags(ssb_code);

CREATE TABLE IF NOT EXISTS magazine_issues (
    issue_number INTEGER PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    title_tr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    publish_month_tr TEXT NOT NULL,
    publish_month_en TEXT NOT NULL,
    year INTEGER NOT NULL,
    cover_image TEXT NOT NULL,
    is_special_issue BOOLEAN NOT NULL DEFAULT FALSE,
    event_name TEXT,
    page_count INTEGER NOT NULL,
    dossier_count INTEGER NOT NULL,
    summary_tr TEXT NOT NULL,
    summary_en TEXT NOT NULL,
    pdf_path TEXT NOT NULL,
    spreads_json TEXT NOT NULL CHECK (spreads_json::jsonb IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'reader',
    organization_category TEXT NOT NULL,
    organization_name TEXT NOT NULL,
    job_title TEXT NOT NULL,
    is_gov_verified BOOLEAN NOT NULL DEFAULT FALSE,
    subscription_tier TEXT NOT NULL DEFAULT 'free',
    subscription_expires_at TEXT,
    reminders_halted BOOLEAN NOT NULL DEFAULT FALSE,
    kvkk_consent_at TEXT NOT NULL,
    kvkk_consent_ip TEXT NOT NULL,
    kvkk_consent_hash TEXT NOT NULL,
    email_consent BOOLEAN NOT NULL DEFAULT TRUE,
    ssb_interests_json TEXT NOT NULL DEFAULT '[]' CHECK (ssb_interests_json::jsonb IS NOT NULL),
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS corporate_verification_requests (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    full_name TEXT NOT NULL,
    organization_category TEXT NOT NULL,
    organization_name TEXT NOT NULL,
    request_message TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_by TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS ad_campaigns (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    slot_code TEXT NOT NULL,
    badge_label TEXT NOT NULL,
    dimensions TEXT NOT NULL,
    sponsor_name TEXT NOT NULL,
    sponsor_type TEXT NOT NULL,
    pill_style TEXT NOT NULL,
    title_tr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    desc_tr TEXT NOT NULL,
    desc_en TEXT NOT NULL,
    target_url TEXT NOT NULL,
    image_url TEXT,
    budget_score INTEGER NOT NULL DEFAULT 100,
    is_default_fallback BOOLEAN NOT NULL DEFAULT FALSE,
    is_floating_popup BOOLEAN NOT NULL DEFAULT FALSE,
    is_newsletter_ad BOOLEAN NOT NULL DEFAULT FALSE,
    target_categories_json TEXT NOT NULL DEFAULT '[]' CHECK (target_categories_json::jsonb IS NOT NULL),
    target_ssb_tags_json TEXT NOT NULL DEFAULT '[]' CHECK (target_ssb_tags_json::jsonb IS NOT NULL),
    target_institutions_json TEXT NOT NULL DEFAULT '[]' CHECK (target_institutions_json::jsonb IS NOT NULL),
    impressions_count BIGINT NOT NULL DEFAULT 0,
    clicks_count BIGINT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE INDEX IF NOT EXISTS idx_ad_campaigns_slot ON ad_campaigns(slot_code, is_active);

CREATE TABLE IF NOT EXISTS recommendation_rules (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    rule_name TEXT NOT NULL,
    source_tag_or_cat TEXT NOT NULL,
    target_institution TEXT NOT NULL,
    recommended_content_slug TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS analytics_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id BIGINT REFERENCES users(id) ON DELETE SET NULL,
    organization_category TEXT NOT NULL,
    organization_name TEXT NOT NULL,
    event_type TEXT NOT NULL,
    content_slug TEXT,
    ad_id BIGINT REFERENCES ad_campaigns(id) ON DELETE SET NULL,
    duration_sec INTEGER NOT NULL DEFAULT 0,
    scroll_depth_pct INTEGER NOT NULL DEFAULT 0,
    video_completed BOOLEAN NOT NULL DEFAULT FALSE,
    is_true_read BOOLEAN NOT NULL DEFAULT FALSE,
    ssb_tag TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_contents_content_type ON contents(content_type);
CREATE INDEX IF NOT EXISTS idx_contents_published_at ON contents(published_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_content ON analytics_events(content_slug, event_type);
CREATE INDEX IF NOT EXISTS idx_analytics_ad ON analytics_events(ad_id, event_type);
CREATE INDEX IF NOT EXISTS idx_analytics_org ON analytics_events(organization_category);
CREATE INDEX IF NOT EXISTS idx_analytics_created ON analytics_events(created_at DESC);

CREATE TABLE IF NOT EXISTS subscription_plans (
    code TEXT PRIMARY KEY,
    name_tr TEXT NOT NULL,
    name_en TEXT NOT NULL,
    price_try DOUBLE PRECISION NOT NULL,
    period_label_tr TEXT NOT NULL,
    period_label_en TEXT NOT NULL,
    features_tr_json TEXT NOT NULL CHECK (features_tr_json::jsonb IS NOT NULL),
    features_en_json TEXT NOT NULL CHECK (features_en_json::jsonb IS NOT NULL),
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS commerce_products (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    sku TEXT NOT NULL UNIQUE,
    product_type TEXT NOT NULL,
    title_tr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    author_or_source TEXT NOT NULL,
    description_tr TEXT NOT NULL,
    description_en TEXT NOT NULL,
    price_try DOUBLE PRECISION NOT NULL,
    cover_image TEXT NOT NULL,
    file_resolution_or_pages TEXT NOT NULL,
    is_available BOOLEAN NOT NULL DEFAULT TRUE
);

CREATE TABLE IF NOT EXISTS order_invoices (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    invoice_no TEXT NOT NULL UNIQUE,
    user_email TEXT NOT NULL,
    full_name TEXT NOT NULL,
    organization_name TEXT NOT NULL,
    item_type TEXT NOT NULL,
    item_code TEXT NOT NULL,
    item_title TEXT NOT NULL,
    amount_try DOUBLE PRECISION NOT NULL,
    payment_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    pdf_watermark_code TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS renewal_reminders (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_email TEXT NOT NULL,
    full_name TEXT NOT NULL,
    organization_name TEXT NOT NULL,
    stage_label TEXT NOT NULL,
    days_remaining INTEGER NOT NULL,
    status TEXT NOT NULL,
    sent_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS newsletter_campaigns (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    subject_tr TEXT NOT NULL,
    subject_en TEXT NOT NULL,
    target_group_filter TEXT NOT NULL,
    target_institution TEXT NOT NULL,
    target_ssb_tag TEXT NOT NULL,
    sponsor_ad_id BIGINT REFERENCES ad_campaigns(id) ON DELETE SET NULL,
    sponsor_name TEXT,
    recipient_count BIGINT NOT NULL DEFAULT 0,
    status TEXT NOT NULL,
    sent_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS seo_redirects (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    old_path TEXT NOT NULL UNIQUE,
    new_path TEXT NOT NULL,
    status_code INTEGER NOT NULL DEFAULT 301,
    hit_count BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS user_purchased_items (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_email TEXT NOT NULL,
    item_code TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE(user_email, item_code)
);

CREATE TABLE IF NOT EXISTS site_settings (
    key TEXT PRIMARY KEY,
    value_tr TEXT NOT NULL,
    value_en TEXT NOT NULL,
    setting_group TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS system_notifications (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    target_audience TEXT NOT NULL,
    target_email TEXT,
    notification_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    title_tr TEXT NOT NULL,
    title_en TEXT NOT NULL,
    message_tr TEXT NOT NULL,
    message_en TEXT NOT NULL,
    action_url TEXT,
    is_read BOOLEAN NOT NULL DEFAULT FALSE,
    is_active_banner BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_system_notifications_audience ON system_notifications(target_audience, is_active_banner);

CREATE TABLE IF NOT EXISTS security_audit_logs (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    client_ip TEXT NOT NULL,
    request_path TEXT NOT NULL,
    event_type TEXT NOT NULL,
    threat_level TEXT NOT NULL,
    details TEXT NOT NULL,
    blocked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_security_audit_logs_created ON security_audit_logs(created_at DESC);

CREATE TABLE IF NOT EXISTS translations (
    key TEXT PRIMARY KEY,
    value_tr TEXT NOT NULL DEFAULT '',
    value_en TEXT NOT NULL DEFAULT '',
    value_ar TEXT NOT NULL DEFAULT '',
    value_de TEXT NOT NULL DEFAULT '',
    value_ru TEXT NOT NULL DEFAULT '',
    value_fr TEXT NOT NULL DEFAULT '',
    grp TEXT NOT NULL DEFAULT 'general'
);

CREATE INDEX IF NOT EXISTS idx_translations_grp ON translations(grp);

CREATE TABLE IF NOT EXISTS content_versions (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    content_id BIGINT NOT NULL REFERENCES contents(id) ON DELETE CASCADE,
    version INTEGER NOT NULL DEFAULT 1,
    title_tr TEXT NOT NULL,
    body_tr TEXT NOT NULL,
    summary_tr TEXT NOT NULL,
    changed_by TEXT NOT NULL,
    change_note TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_content_versions_cid ON content_versions(content_id, version DESC);

CREATE TABLE IF NOT EXISTS media_library (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    filename TEXT NOT NULL,
    original_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size BIGINT NOT NULL DEFAULT 0,
    path TEXT NOT NULL,
    alt_text_tr TEXT NOT NULL DEFAULT '',
    alt_text_en TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '',
    uploaded_by TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_media_library_created ON media_library(created_at DESC);

CREATE TABLE IF NOT EXISTS admin_audit_log (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    actor_email TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id BIGINT,
    details_json TEXT NOT NULL DEFAULT '{}' CHECK (details_json::jsonb IS NOT NULL),
    ip_address TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_admin_audit_log_created ON admin_audit_log(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_audit_log_actor ON admin_audit_log(actor_email);

CREATE TABLE IF NOT EXISTS user_roles (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    role_code TEXT NOT NULL UNIQUE,
    name_tr TEXT NOT NULL,
    name_en TEXT NOT NULL,
    permissions_json TEXT NOT NULL DEFAULT '[]' CHECK (permissions_json::jsonb IS NOT NULL),
    is_system BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE IF NOT EXISTS custom_fields (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    field_key TEXT NOT NULL UNIQUE,
    label_tr TEXT NOT NULL,
    label_en TEXT NOT NULL,
    field_type TEXT NOT NULL DEFAULT 'text',
    options_json TEXT NOT NULL DEFAULT '[]' CHECK (options_json::jsonb IS NOT NULL),
    is_required BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    applies_to TEXT NOT NULL DEFAULT 'content'
);

CREATE TABLE IF NOT EXISTS cache_operations (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    cache_key TEXT NOT NULL,
    operation TEXT NOT NULL,
    actor_email TEXT NOT NULL,
    details TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cache_operations_created ON cache_operations(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_users_email_lower ON users(LOWER(email));
