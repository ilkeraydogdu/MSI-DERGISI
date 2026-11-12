# MSI Dergisi & MSI TDR Platform — Architecture Documentation

> **Last Updated:** 2026-11-11
> **Stack:** Rust + Axum + SQLite (WAL) + Tower HTTP
> **Purpose:** Full-stack defense/aerospace publishing platform with SSR, REST API, admin CMS, e-commerce, and SEO engine.

---

## Table of Contents

1. [System Overview Diagram](#1-system-overview-diagram)
2. [Module Dependency Graph](#2-module-dependency-graph)
3. [Request Data Flow](#3-request-data-flow)
4. [Central Engines](#4-central-engines)
5. [Database Schema Summary](#5-database-schema-summary)
6. [Environment Variables Reference](#6-environment-variables-reference)
7. [API Endpoint Reference](#7-api-endpoint-reference)
8. [Admin Panel Feature List](#8-admin-panel-feature-list)
9. [Deployment Instructions](#9-deployment-instructions)

---

## 1. System Overview Diagram

```
                            ┌─────────────────────────────────────────────────┐
                            │              CLIENT (Browser / API)             │
                            └──────────────────────┬──────────────────────────┘
                                                   │ HTTPS
                                                   ▼
                            ┌─────────────────────────────────────────────────┐
                            │          TOWER HTTP MIDDLEWARE STACK            │
                            │  ┌─────────────┐ ┌──────────┐ ┌─────────────┐  │
                            │  │ Compression │ │   CORS   │ │ WAF + Auth  │  │
                            │  │    Layer     │ │  Layer   │ │ Middleware  │  │
                            │  └─────────────┘ └──────────┘ └──────┬──────┘  │
                            └──────────────────────────────────────┼──────────┘
                                                                   │
                                                   ┌───────────────┴───────────────┐
                                                   │        AXUM ROUTER            │
                                                   │  ┌─────────┐  ┌────────────┐  │
                                                   │  │Web SSR  │  │ REST API   │  │
                                                   │  │Routes   │  │ /api/v1/*  │  │
                                                   │  └────┬────┘  └─────┬──────┘  │
                                                   └───────┼─────────────┼──────────┘
                                                           │             │
                                          ┌────────────────┴─────────────┴────────────────┐
                                          │              CONTROLLERS                       │
                                          │  web_controller  │  api_controller             │
                                          │  admin_view      │                             │
                                          └────────────────┬──────────────────────────────┘
                                                           │
                                          ┌────────────────┴──────────────────────────────┐
                                          │               SERVICES                         │
                                          │  auth_service       │ content_service           │
                                          │  ad_service         │ analytics_service         │
                                          │  email_service      │ payment_service           │
                                          │  subscription_svc   │ media_service             │
                                          │  edergi_pdf_service │                           │
                                          └────────────────┬──────────────────────────────┘
                                                           │
                                          ┌────────────────┴──────────────────────────────┐
                                          │            CENTRAL ENGINES                     │
                                          │  SecurityEngine  │ SeoRouterEngine             │
                                          │  TranslationEng  │ ContentQueryEngine          │
                                          │  SearchEngine    │ DesignEngine                │
                                          │  NotificationEng │ ValidationEngine            │
                                          │  QueryEngine     │ ApiResponse                 │
                                          └────────────────┬──────────────────────────────┘
                                                           │
                                          ┌────────────────┴──────────────────────────────┐
                                          │          DATA ACCESS LAYER (db/)               │
                                          │  Database (L1 Cache RwLock)                   │
                                          │  repo.rs (all CRUD + queries)                 │
                                          │  schema.rs (DDL init)                          │
                                          │  seed.rs (initial data)                        │
                                          └────────────────┬──────────────────────────────┘
                                                           │
                                          ┌────────────────┴──────────────────────────────┐
                                          │         SQLite WAL (data/msi_platform.db)      │
                                          │  22 tables │ PRAGMA journal_mode=WAL           │
                                          │  PRAGMA foreign_keys=ON │ busy_timeout=5000ms  │
                                          └────────────────────────────────────────────────┘
```

---

## 2. Module Dependency Graph

```
main.rs
 ├── config.rs                    (AppConfig — env-driven configuration)
 ├── controllers/
 │   ├── mod.rs                   (AppState struct definition)
 │   ├── web_controller.rs        (SSR page rendering handlers)
 │   ├── api_controller.rs        (REST API endpoint handlers)
 │   └── admin_view.rs            (Admin panel HTML rendering)
 ├── db/
 │   ├── mod.rs                   (Database struct, L1CacheSnapshot, open_conn)
 │   ├── schema.rs                (init_schema — all CREATE TABLE statements)
 │   ├── repo.rs                  (All DB operations: CRUD, cache refresh, queries)
 │   └── seed.rs                  (Initial data seeding)
 ├── engine/
 │   ├── mod.rs                   (Engine module index)
 │   ├── security_engine.rs       (WAF, rate limiting, XSS/SQLi detection, headers)
 │   ├── seo_router_engine.rs     (SEO meta, sitemaps, RSS, JSON-LD, llms.txt)
 │   ├── translation_engine.rs    (Multi-language translation cache)
 │   ├── content_query_engine.rs  (Content filtering and querying logic)
 │   ├── search_engine.rs         (Full-text search implementation)
 │   ├── design_engine.rs         (Page design/template engine)
 │   ├── notification_engine.rs   (Notification generation logic)
 │   ├── validation_engine.rs     (Input validation rules)
 │   ├── query_engine.rs          (Generic query building utilities)
 │   ├── api_response.rs          (Standardized API response formatting)
 │   └── design/                  (Design editor sub-module)
 │       └── editor.rs            (Visual design editor GET/POST/save/preview)
 ├── services/
 │   ├── mod.rs                   (Service module index)
 │   ├── auth_service.rs          (Session management, Argon2id password hashing)
 │   ├── content_service.rs       (Content business logic)
 │   ├── ad_service.rs            (Ad campaign selection and conflict checking)
 │   ├── analytics_service.rs     (Analytics event tracking and reporting)
 │   ├── email_service.rs         (SMTP/sendmail dispatch for reminders)
 │   ├── payment_service.rs       (PayTR payment gateway integration)
 │   ├── subscription_service.rs  (5-stage renewal reminder evaluation)
 │   ├── media_service.rs         (File upload and media processing)
 │   └── edergi_pdf_service.rs    (E-magazine PDF download with DRM watermark)
 └── models/                      (Shared data structures / domain types)
```

### Shared State (`AppState`)

Defined in `src/controllers/mod.rs`, cloned into every handler via Axum's `State` extractor:

```rust
pub struct AppState {
    pub config: Arc<AppConfig>,        // Immutable app configuration
    pub db: Database,                  // DB handle with L1 cache (Arc<RwLock>)
    pub translations: TranslationCache, // In-memory translation lookup
}
```

---

## 3. Request Data Flow

```
Client Request
      │
      ▼
┌─────────────────────────────────────────────────────────────────┐
│ 1. COMPRESSION LAYER (tower_http::CompressionLayer)             │
│    Gzip/Brotli response compression                             │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. CORS LAYER (tower_http::CorsLayer)                           │
│    Origin whitelist from AppConfig (prod vs dev aware)          │
│    Credentials allowed; methods: GET/POST/PUT/DELETE/OPTIONS    │
│    Headers: Content-Type, Authorization, Cookie                 │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. CENTRAL WAF & TELEMETRY MIDDLEWARE                           │
│    a. Extract client IP (X-Forwarded-For / X-Real-IP)           │
│    b. CentralSecurityFilter::inspect_request_security()         │
│       - Rate Limiter (token bucket per IP, configurable RPM)    │
│       - Path Traversal detection (../, %2e%2e%2f, etc.)         │
│       - SQL Injection pattern matching                          │
│       - XSS vector detection                                    │
│    c. If blocked → return 403/429 JSON + audit log to DB        │
│    d. Operator route check (/admin/*, /api/v1/admin/*, etc.)    │
│       - Requires session with role "site_operator" or "admin"   │
│       - Non-API GET → redirect to /hesabim?next=...             │
│       - API/non-GET → 401 JSON                                  │
│    e. Forward to handler via next.run(req)                      │
│    f. Inject security headers on response:                      │
│       X-Content-Type-Options, X-Frame-Options, HSTS,            │
│       Referrer-Policy, Permissions-Policy, CSP,                 │
│       X-Engine-Latency-Us                                       │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. AXUM ROUTER                                                  │
│    Match path → Controller handler function                     │
│    Static assets served via ServeDir (/css, /js, /images, etc.) │
│    Fallback → handle_fallback_or_redirect (301 SEO redirects)   │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. CONTROLLER                                                   │
│    Parse extractors (Path, Query, Json, State, Headers)         │
│    Delegate business logic to Services                          │
│    For SSR: call Engine(s) to build HTML                        │
│    For API: serialize JSON response                             │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. SERVICE LAYER                                                │
│    Business rules, validation, external integrations            │
│    (PayTR, sendmail, Argon2id hashing, DRM watermarking)        │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. DATABASE LAYER                                               │
│    L1 Cache check (RwLock<Option<L1CacheSnapshot>>)             │
│    Cache hit → return cloned snapshot data                      │
│    Cache miss → execute SQL via rusqlite                        │
│    Write operations → refresh_l1_cache() after commit           │
└──────────────────────────┬──────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│ 8. RESPONSE                                                     │
│    Security headers injected by middleware                      │
│    Compression applied by tower layer                           │
│    Returned to client                                           │
└─────────────────────────────────────────────────────────────────┘
```

### Background Workers

- **Renewal Reminder Worker** (`spawn_renewal_reminder_worker`): Runs every 6 hours. Evaluates 5-stage subscription renewal reminders for all users, syncs to DB, and dispatches emails if `MAIL_DELIVERY_ENABLED=true`.

---

## 4. Central Engines

| Engine | Module | Purpose |
|--------|--------|---------|
| **SecurityEngine** | `engine/security_engine.rs` | WAF (rate limiting, SQLi, XSS, path traversal detection), HTML escaping, slug sanitization, rich HTML sanitization (ammonia allowlist), security response headers |
| **SeoRouterEngine** | `engine/seo_router_engine.rs` | OpenGraph/Twitter/Yandex meta tags, Schema.org JSON-LD (NewsArticle, WebSite, NewsMediaOrganization, BreadcrumbList), XML sitemaps (standard, news, image), RSS 2.0, JSON Feed, robots.txt, llms.txt/llms-full.txt, OpenSearch XML, PWA manifest, ISO 8601/RFC 2822 date conversion |
| **TranslationEngine** | `engine/translation_engine.rs` | Multi-language translation cache (TR, EN, AR, DE, RU, FR), seed defaults, in-memory lookup |
| **ContentQueryEngine** | `engine/content_query_engine.rs` | Content filtering, faceted queries, placement/tag-based retrieval |
| **SearchEngine** | `engine/search_engine.rs` | Full-text search across contents (title, summary, body, author, tags) |
| **DesignEngine** | `engine/design_engine.rs` | Page template/design system for SSR rendering |
| **Design Editor** | `engine/design/editor.rs` | Visual design editor (GET view, POST preview, GET/SAVE API) |
| **NotificationEngine** | `engine/notification_engine.rs` | System notification generation and targeting |
| **ValidationEngine** | `engine/validation_engine.rs` | Input validation rules for forms and API payloads |
| **QueryEngine** | `engine/query_engine.rs` | Generic query building utilities |
| **ApiResponse** | `engine/api_response.rs` | Standardized JSON API response formatting |

---

## 5. Database Schema Summary

**Database:** SQLite with WAL mode, foreign keys enabled, 5-second busy timeout.
**File:** Configurable via `SQLITE_DB_PATH` (default: `data/msi_platform.db`).
**L1 Cache:** In-memory `RwLock<Option<L1CacheSnapshot>>` holding full copies of categories, ssb_tags, contents, issues, ads, site_settings, notifications, seo_redirects, subscription_plans, commerce_products. Refreshed on every write operation.

### Tables (22 total)

| Table | Key Columns | Purpose |
|-------|-------------|---------|
| `categories` | id, slug (UNIQUE), parent_slug, name_tr, name_en, group_type, sort_order, is_dynamic | Hierarchical content categories (TR/EN) |
| `ssb_tags` | id, code (UNIQUE), name_tr, name_en, category_group, sort_order | Defense industry taxonomy tags |
| `contents` | id, slug (UNIQUE), title_tr/en, summary_tr/en, body_tr/en, content_type, source_group, cover_image, author_name, issue_number, published_at, visibility_level, single_price_try, is_sponsored, is_featured, view_count, true_read_count | Core article/content storage |
| `content_placements` | content_id (FK), category_slug | Many-to-many content-category mapping |
| `content_ssb_tags` | content_id (FK), ssb_code | Many-to-many content-tag mapping |
| `magazine_issues` | issue_number (PK), slug, title_tr/en, year, cover_image, pdf_path, spreads_json, page_count, dossier_count | E-magazine issue catalog |
| `users` | id, email (UNIQUE), password_hash, full_name, role, organization_category/name, subscription_tier, subscription_expires_at, kvkk_consent_*, ssb_interests_json | User accounts with KVKK compliance |
| `corporate_verification_requests` | id, user_id, email, organization_name, status, reviewed_by | Corporate account verification workflow |
| `ad_campaigns` | id, slot_code, sponsor_name, title_tr/en, target_url, budget_score, target_categories/tags/institutions_json, impressions_count, clicks_count, is_active | Advertising campaign management |
| `recommendation_rules` | id, rule_name, source_tag_or_cat, target_institution, recommended_content_slug, priority | Content recommendation engine rules |
| `analytics_events` | id, session_id, user_id, organization_category, event_type, content_slug, ad_id, duration_sec, scroll_depth_pct, is_true_read | User behavior analytics |
| `subscription_plans` | code (PK), name_tr/en, price_try, period_label_tr/en, features_tr/en_json, is_featured | Subscription tier definitions |
| `commerce_products` | id, sku (UNIQUE), product_type, title_tr/en, price_try, cover_image, is_available | Digital product catalog |
| `order_invoices` | id, invoice_no (UNIQUE), user_email, item_type/code, amount_try, payment_status, pdf_watermark_code | Order/payment records |
| `renewal_reminders` | id, user_id, stage_label, days_remaining, status, sent_at | 5-stage subscription renewal tracking |
| `newsletter_campaigns` | id, subject_tr/en, target_group_filter, sponsor_ad_id, recipient_count, status | Newsletter campaign management |
| `seo_redirects` | id, old_path (UNIQUE), new_path, status_code, hit_count | 301/302 SEO redirect rules |
| `user_purchased_items` | id, user_email, item_code (UNIQUE pair) | Digital purchase entitlements |
| `site_settings` | key (PK), value_tr, value_en, setting_group, updated_at | Key-value site configuration store |
| `system_notifications` | id, target_audience, notification_type, severity, title/message_tr/en, is_active_banner | In-app notification system |
| `security_audit_logs` | id, client_ip, request_path, event_type, threat_level, details, blocked, created_at | WAF security event log |
| `translations` | key (PK), value_tr/en/ar/de/ru/fr, grp | Multi-language UI string translations |
| `content_versions` | id, content_id (FK), version, title_tr, body_tr, summary_tr, changed_by, change_note | Content version history |
| `media_library` | id, filename, original_name, mime_type, file_size, path, alt_text_tr/en, tags, uploaded_by | Media asset registry |
| `admin_audit_log` | id, actor_email, action, entity_type, entity_id, details_json, ip_address | Admin action audit trail |
| `user_roles` | id, role_code (UNIQUE), name_tr/en, permissions_json, is_system | RBAC role definitions |
| `custom_fields` | id, field_key (UNIQUE), label_tr/en, field_type, options_json, is_required, applies_to | Dynamic form field builder |
| `cache_operations` | id, cache_key, operation, actor_email, details | Cache management tracking |

---

## 6. Environment Variables Reference

All variables are loaded via `dotenvy::dotenv()` then `std::env::var()`. Defaults shown where applicable.

| Variable | Default | Description |
|----------|---------|-------------|
| `APP_NAME` | "MSI Dergisi & MSI TDR - Savunma Sanayii..." | Application display name |
| `APP_ENV` | "production" | Environment ("production", "prod", "development") |
| `HOST` | "127.0.0.1" | TCP bind address |
| `PORT` | 8090 | TCP bind port |
| `PRIMARY_DOMAIN` | "msidergisi.com" | Primary domain for SEO/CORS |
| `NEWS_DOMAIN` | "savunmahaber.com" | News domain for SEO/CORS |
| `PUBLIC_URL` | "https://trusting-brahmagupta.194-110-169-165.plesk.page" | Canonical public URL |
| `STATIC_DIR` | Current working directory | Static asset root directory |
| `SQLITE_DB_PATH` | "data/msi_platform.db" | SQLite database file path |
| `MARIADB_DATABASE` | "msi_dergisi_db" | MariaDB database name (reserved, not actively used) |
| `SESSION_SECRET` | "msi-umsa-2026-tactical-secret-key" | Session signing secret |
| `WATERMARK_SECRET` | "UMSA-MSI-DRM-WATERMARK-2026" | PDF DRM watermark secret |
| `ADMIN_DEFAULT_USER` | "operator@msidergisi.com" | Default operator email |
| `ADMIN_DEFAULT_PASS` | "MsiUmsa2026!" | Default operator password (Argon2id synced at startup) |
| `AD_MAX_QUOTA_PER_KEYWORD` | 3 | Max ads per keyword slot |
| `PAYTR_MERCHANT_ID` | "" | PayTR merchant ID |
| `PAYTR_MERCHANT_KEY` | "" | PayTR merchant key |
| `PAYTR_MERCHANT_SALT` | "" | PayTR merchant salt |
| `PAYTR_TEST_MODE` | false | PayTR test mode flag |
| `PAYTR_API_URL` | "https://www.paytr.com/odeme/api/get-token" | PayTR API endpoint |
| `SMTP_FROM` | "MSI Dergisi <noreply@msidergisi.com>" | Email sender address |
| `SENDMAIL_BIN` | "/usr/sbin/sendmail" | Sendmail binary path |
| `MAIL_DELIVERY_ENABLED` | false | Enable/disable email dispatch |
| `RATE_LIMIT_MAX_RPM` | 600 | Max requests per minute per IP |
| `RATE_LIMIT_WINDOW_SECS` | 60 | Rate limit window in seconds |
| `SOCIAL_MEDIA_URLS` | Comma-separated list of social URLs | Social media links for Schema.org sameAs |

---

## 7. API Endpoint Reference

### Public SSR Pages (Server-Side Rendered)

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/` | render_home | No |
| GET | `/index.html` | render_home | No |
| GET | `/en` | redirect_en_home | No |
| GET | `/arama` | render_search_page | No |
| GET | `/hesabim` | render_account_page | No |
| GET | `/sayfa/:slug` | render_designed_page | No |
| GET | `/icerik/:slug` | render_article_detail | No |
| GET | `/kategori/:slug` | render_category_page | No |
| GET | `/coming-soon`, `/yakinda` | render_coming_soon | No |
| GET | `/e-dergi` | render_edergi_kiosk | No |
| GET | `/abonelik-ve-satis` | render_subscriptions_and_store | No |
| GET | `/kurumsal/:slug` | render_corporate_page | No |

### SEO & Feed Endpoints

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/sitemap.xml` | render_sitemap_xml | No |
| GET | `/sitemap-news.xml` | render_news_sitemap_xml | No |
| GET | `/sitemap-images.xml` | render_image_sitemap_xml | No |
| GET | `/robots.txt` | render_robots_txt | No |
| GET | `/rss.xml` | render_rss_xml | No |
| GET | `/feed.json` | render_json_feed | No |
| GET | `/llms.txt` | render_llms_txt | No |
| GET | `/llms-full.txt` | render_llms_full_txt | No |
| GET | `/.well-known/ai-plugin.json` | render_ai_plugin_json | No |
| GET | `/opensearch.xml` | render_opensearch_xml | No |
| GET | `/manifest.webmanifest` | render_web_manifest | No |
| GET | `/favicon.ico` | render_favicon_ico | No |

### Legacy Redirect Routes

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/haberler/:slug` | redirect_legacy_haber_tr | No |
| GET | `/en/haberler/:slug` | redirect_legacy_haber_en | No |
| GET | `/kategoriler/:slug` | redirect_legacy_kategori_tr | No |
| GET | `/en/kategoriler/:slug` | redirect_legacy_kategori_en | No |

### Public REST API

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/api/v1/health` | health_check | No |
| GET | `/api/v1/search` | faceted_search_api | No |
| GET | `/api/v1/notifications` | list_notifications_api | No |
| POST | `/api/v1/notifications/:id/read` | mark_notification_read_api | No |
| GET | `/api/v1/settings` | list_site_settings_api | No |
| GET | `/api/v1/contents` | list_contents | No |
| GET | `/api/v1/contents/:slug` | get_content_detail | No |
| GET | `/api/v1/categories` | list_categories | No |
| GET | `/api/v1/ssb-tags` | list_ssb_tags | No |
| GET | `/api/v1/issues` | list_issues | No |
| GET | `/api/v1/ads` | list_ads | No |
| GET | `/api/v1/edergi/:issue_no/download-pdf` | download_edergi_pdf | No |
| POST | `/api/v1/auth/register` | register_user | No |
| POST | `/api/v1/auth/login` | login_user | No |
| POST | `/api/v1/auth/logout` | logout_user | No |
| POST | `/api/v1/analytics/track` | track_analytics_event | No |
| GET | `/api/v1/commerce/catalog` | get_commerce_catalog | No |
| POST | `/api/v1/subscriptions/checkout` | checkout_order | No |
| POST | `/api/v1/payments/paytr-callback` | paytr_callback | No |

### Operator-Protected Routes (require site_operator or admin role)

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/admin` | render_admin | Operator |
| GET | `/yonetim` | render_admin | Operator |
| GET | `/admin/tasarim` | design::editor::view | Operator |
| POST | `/admin/tasarim/onizleme` | design::editor::preview | Operator |
| GET/POST | `/api/v1/admin/design` | design::editor::get/save | Operator |
| GET | `/api/v1/analytics/sponsor-report` | admin_analytics_bundle | Operator |
| POST | `/api/v1/subscriptions/trigger-reminders` | admin_halt_renewed_reminders | Operator |
| POST | `/api/v1/ads/check-conflict` | admin_check_ad_conflict | Operator |

### Admin CMS/OMS REST API (all require operator/admin role)

| Method | Path | Handler |
|--------|------|---------|
| GET | `/api/v1/admin/overview` | admin_overview |
| POST | `/api/v1/admin/upload` | admin_upload_media (max 25 MB) |
| GET/POST | `/api/v1/admin/notifications` | list/create notifications |
| GET/POST | `/api/v1/admin/settings` | list/upsert site settings |
| GET | `/api/v1/admin/security-logs` | admin_list_security_logs_api |
| GET/POST | `/api/v1/admin/contents` | list/upsert content |
| DELETE | `/api/v1/admin/contents/:id` | admin_delete_content |
| POST | `/api/v1/admin/contents/bulk` | admin_bulk_content_operation |
| GET/POST | `/api/v1/admin/contents/:id/versions` | list/create content versions |
| GET/POST | `/api/v1/admin/ads` | list/upsert ads |
| POST | `/api/v1/admin/ads/check-conflict` | admin_check_ad_conflict |
| GET | `/api/v1/admin/analytics` | admin_analytics_bundle |
| GET | `/api/v1/admin/analytics/bundle` | admin_analytics_bundle |
| GET | `/api/v1/admin/analytics/sponsor-report` | admin_analytics_bundle |
| GET | `/api/v1/admin/analytics/contents` | admin_analytics_contents |
| GET | `/api/v1/admin/analytics/ads` | admin_analytics_ads |
| GET | `/api/v1/admin/analytics/readers` | admin_analytics_readers |
| GET/POST | `/api/v1/admin/corporate-requests` | list/decide corporate requests |
| POST | `/api/v1/admin/corporate-requests/decide` | admin_decide_corporate_request_body |
| POST | `/api/v1/admin/corporate-requests/:id/decide` | admin_decide_corporate_request_by_path |
| POST | `/api/v1/admin/corporate-requests/:id/decision` | admin_decide_corporate_request_by_path |
| GET/POST | `/api/v1/admin/categories` | list/insert categories |
| GET/POST | `/api/v1/admin/ssb-tags` | list/insert SSB tags |
| GET/POST | `/api/v1/admin/newsletters` | list/insert newsletters |
| GET | `/api/v1/admin/reminders` | admin_list_reminders |
| POST | `/api/v1/admin/reminders/halt-renewed` | admin_halt_renewed_reminders |
| GET/POST | `/api/v1/admin/redirects` | list/insert SEO redirects |
| GET | `/api/v1/admin/export/open-data` | admin_export_open_data |
| GET/POST | `/api/v1/admin/translations` | list/upsert translations |
| DELETE | `/api/v1/admin/translations/:key` | admin_delete_translation |
| GET | `/api/v1/admin/media-library` | admin_list_media_library |
| GET/POST | `/api/v1/admin/audit-logs` | list/insert audit logs |
| GET/POST | `/api/v1/admin/user-roles` | list/upsert user roles |
| GET/POST | `/api/v1/admin/custom-fields` | list/upsert custom fields |
| DELETE | `/api/v1/admin/custom-fields/:field_key` | admin_delete_custom_field |
| GET/POST | `/api/v1/admin/cache` | list/flush cache operations |
| GET | `/api/v1/admin/system-health` | admin_system_health |

### Static Asset Directories

| Path | Source |
|------|--------|
| `/css/*` | `{STATIC_DIR}/css/` |
| `/js/*` | `{STATIC_DIR}/js/` |
| `/images/*` | `{STATIC_DIR}/images/` |
| `/fonts/*` | `{STATIC_DIR}/fonts/` |
| `/scss/*` | `{STATIC_DIR}/scss/` |

### Fallback

All unmatched routes are handled by `handle_fallback_or_redirect`, which checks `seo_redirects` table for 301 redirects before returning 404.

---

## 8. Admin Panel Feature List

The admin panel is accessible at `/admin` (or `/yonetim`) and requires `site_operator` or `admin` role.

### Core CMS
- **Content Management**: Create, edit, delete articles with bilingual (TR/EN) support
- **Content Version History**: Track all changes with version numbering, change notes, and actor attribution
- **Bulk Content Operations**: Batch update/publish/unpublish content items
- **Category Management**: Hierarchical category CRUD with dynamic/static flags
- **SSB Tag Management**: Defense industry taxonomy tag CRUD
- **Media Upload**: File upload up to 25 MB with media library registry
- **Media Library**: Browse all uploaded assets with metadata (alt text, tags, uploader)

### Advertising & Commerce
- **Ad Campaign Management**: Create/edit ads with targeting (categories, SSB tags, institutions)
- **Ad Conflict Checker**: Validate ad placement conflicts before publishing
- **Subscription Plans**: Manage subscription tiers and pricing
- **Commerce Catalog**: Digital product management
- **Order/Invoice Tracking**: View payment records and statuses

### Analytics & Reporting
- **Dashboard Overview**: High-level platform statistics
- **Content Analytics**: Per-article view counts, read metrics
- **Ad Analytics**: Impression/click tracking per campaign
- **Reader Analytics**: Audience segmentation by organization category
- **Sponsor Reports**: Bundled analytics for sponsor reporting

### User & Access Management
- **Corporate Verification Requests**: Review and approve/reject corporate account requests
- **User Roles & Permissions**: RBAC role definition with JSON permission sets
- **Custom Fields**: Dynamic form builder for content/user metadata
- **Newsletter Campaigns**: Create and manage targeted newsletter campaigns

### System Administration
- **Site Settings**: Key-value configuration store (bilingual)
- **Translation Management**: Edit UI strings across 6 languages (TR, EN, AR, DE, RU, FR)
- **SEO Redirects**: Manage 301/302 redirect rules with hit counting
- **Security Audit Logs**: View WAF-blocked events and threat details
- **Admin Audit Logs**: Track all admin actions with actor, entity, and IP
- **Cache Management**: View cache operations and flush L1 cache
- **System Health Monitor**: DB size, table counts, connectivity status
- **Open Data Export**: Export platform data in open format
- **Renewal Reminders**: View and halt subscription renewal reminder pipeline
- **Design Editor**: Visual page design editor with live preview

---

## 9. Deployment Instructions

### Prerequisites

- Rust toolchain (stable, 1.75+ recommended)
- SQLite 3.x (bundled via rusqlite)
- Optional: sendmail binary at `/usr/sbin/sendmail` for email delivery

### Build

```bash
cd /var/www/vhosts/trusting-brahmagupta.194-110-169-165.plesk.page/httpdocs
cargo build --release
```

### Configuration

1. Copy `.env.example` to `.env` (or create `.env` manually).
2. Set required environment variables (see Section 6 above).
3. Ensure `data/` directory exists and is writable:
   ```bash
   mkdir -p data
   ```

### Run

```bash
# Production
APP_ENV=production ./target/release/msi-platform

# Development (includes http://host:port in CORS origins)
APP_ENV=development RUST_LOG=debug ./target/release/msi-platform
```

### Database Initialization

The database is auto-initialized on first run:
- Schema creation via `schema::init_schema()` (all 22+ tables)
- Seed data via `seed::seed_initial_data()` if `contents` table is empty
- Site settings and notifications seeded separately if missing
- Default operator password synced to Argon2id hash at startup

### Static Assets

Place static files in the configured `STATIC_DIR` (defaults to project root):
- `css/` — Stylesheets
- `js/` — JavaScript bundles
- `images/` — Images, icons, covers
- `fonts/` — Web fonts
- `scss/` — SCSS source files

### Reverse Proxy (Recommended)

Configure Nginx/Apache as a reverse proxy in front of the Axum server:

```nginx
server {
    listen 443 ssl http2;
    server_name msidergisi.com savunmahaber.com;

    location / {
        proxy_pass http://127.0.0.1:8090;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

### Security Notes

- Change `SESSION_SECRET` and `WATERMARK_SECRET` in production.
- Change `ADMIN_DEFAULT_PASS` immediately after first login.
- Set `MAIL_DELIVERY_ENABLED=true` only when sendmail/SMTP is properly configured.
- In production (`APP_ENV=production`), the dev origin `http://host:port` is excluded from CORS to prevent CSRF.
- The WAF rate limiter defaults to 600 RPM per IP; adjust via `RATE_LIMIT_MAX_RPM`.

### Monitoring

- Structured logging via `tracing-subscriber` (configure level with `RUST_LOG`).
- Security events logged to `security_audit_logs` table.
- Admin actions logged to `admin_audit_log` table.
- Engine execution latency exposed via `X-Engine-Latency-Us` response header.
- System health endpoint: `GET /api/v1/admin/system-health` (requires operator role).