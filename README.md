# MSI Dergisi & MSI TDR Platform

Full-stack Turkish defense, aerospace, and security publishing, digital magazine kiosk, editorial studio, and e-commerce platform serving **`msidergisi.com`** (TR), **MSI TDR** (EN), and **`savunmahaber.com`**.

---

## Tech Stack

- **Backend:** Pure Rust (`msi-dergisi-platform` v2.6.0) + Axum 0.7 + Tokio async runtime + `HtmlBuf` SSR DSL
- **Database:** SQLite in WAL mode (`52` tables, idempotent migrations `001`..`017`) + PostgreSQL 16 reference schema parity (`migrations_pg/001`..`017`)
- **Frontend:** Pure Vanilla CSS & Vanilla JS + local vendor GSAP / ScrollTrigger / Lenis / Chart.js + Pure-Rust WebAssembly Editorial Studio (`src/editorial/` -> `storage/editorial-pkg/{editorial_studio.js,editorial_studio_bg.wasm}`, served at `/{admin}/editor/studio.{js,wasm}`)
- **Languages:** Bilingual TR / EN portal (`?lang=` > `msi_lang` cookie > weighted `Accept-Language`) + extensible DB-backed translations, approval workflow, defense glossary, and locale management
- **Payment & Mail:** PayTR POS gateway (`POST /api/v1/payments/paytr-callback` & compatible alias `POST /api/v1/payments/paytr/callback`) + Postfix / `plesk-sendmail` MTA
- **SEO & Feeds:** Multi-brand `Host` routing, IndexNow, XML/News/Image sitemaps, RSS, JSON Feed, OpenSearch, `llms.txt`, `1,334` 301 redirects, Schema.org JSON-LD, and strict `noindex, nofollow` 404 hygiene
- **Security:** WAF & rate limiting, per-request CSP nonces, Argon2id password hashing, TOTP 2FA, Ammonia HTML sanitization, CSRF protection, and persistent `revoked_tokens`

---

## Architecture

- **`226` Rust source files** under `src/`:
  - `src/routes/` — `256` active Axum `.route()` paths (`84` public/SSR/gateway routes, `39` public/auth/commerce API endpoints, `133` RBAC admin API endpoints incl. `31` Editorial Studio endpoints — `172` REST endpoints total) + `5` static `ServeDir` mounts
  - `src/middleware/` & `src/extractors/` — WAF telemetry, auth gate, CSRF validation, `OperatorUser`, `RequirePermission` (`17` canonical RBAC roles)
  - `src/controllers/` — `web_controller/`, `api_controller/`, `admin_view.rs`
  - `src/services/` & `src/engine/` — auth, email, PayTR, E-Dergi PDF conversion & DRM watermarking, editorial locks/media/research/social, encrypted incremental DRP backup (`MSIBKP01`/`MSIDELTA1` + optional S3 SigV4), SEO router, search, design engine
  - `src/workers/` — **7** background workers (`analytics_aggregation`, `b2b_report_worker`, `backup_worker`, `editorial_scheduler`, `monitoring`, `newsletter_scheduler`, `renewal_reminder`)
  - `src/editorial/` — consolidated shared editorial core (`common.rs`) and Pure-Rust WASM Editorial Studio (`wasm/*.rs`)
- **Portals & Gateways:**
  - `/podmin` — Split-screen command login & 18-tab RBAC Command Center (retired `/msi-komuta` returns `404` + `noindex, nofollow`)
  - `/podmin/tasarim` — Live Visual Design Studio (`editor.rs` / `editor.html`)
  - `/podmin/editor` — Pure-Rust WASM Editorial Studio (`editorial_studio.rs`)
  - `/hesabim` — Authenticated Role & Reader Portal tailored across all `17` RBAC roles

---

## Getting Started

### 1. Environment Configuration
Copy `.env.example` to `.env` and configure domain, database, session/watermark/backup secrets, PayTR credentials, `ADMIN_PATH_SECRET=podmin`, and `SENDMAIL_BIN`.

### 2. Build Embedded Editorial WASM Studio & Release Binary
```bash
cargo run --bin msi-dergisi-platform -- --build-editorial-wasm
cargo build --release --workspace
./target/release/msi-dergisi-platform
```

---

## Testing

```bash
# Full Rust unit & integration test suite
cargo test --all

# Full Playwright / Node E2E suite (623 tests across Tiers 0-4)
node tests/e2e/run.mjs --tier=all
```

---

*Built by UMSA Prodüksiyon*
