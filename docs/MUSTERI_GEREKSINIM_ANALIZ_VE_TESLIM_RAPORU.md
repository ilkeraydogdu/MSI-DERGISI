# MSI Dergisi & MSI TDR / savunmahaber.com — Müşteri Gereksinimleri Analiz ve Satışa Hazır Teslim Raporu

**Platform:** UMSA Prodüksiyon — MSI Dergisi (`msidergisi.com`), MSI TDR (EN) ve `savunmahaber.com`  
**Canlı Yayın URL:** `https://trusting-brahmagupta.194-110-169-165.plesk.page`  
**Yönetim Komuta Merkezi:** `/podmin` · **Görsel Tasarım Stüdyosu:** `/podmin/tasarim` · **Editör Stüdyosu (WASM):** `/podmin/editor` · **Rol / Okuyucu Portalı:** `/hesabim`  
**Teknoloji Yığını:** Pure Rust (Axum 0.7, Tokio, SQLite WAL + `rusqlite_migration` `001`..`017`, `HtmlBuf` SSR, Vanilla CSS/JS + GSAP + ScrollTrigger + Lenis + Chart.js + Pure-Rust WASM Editör Stüdyosu [`src/editorial/`])

---

## 1. Yönetici Özeti ve Yapılan Ana Dönüşüm

Bu belge, `docs/specifications/` klasöründeki dört resmi müşteri dokümanının canlı kod tabanı ve üretim veritabanıyla satır satır karşılaştırılması sonucunda tespit edilen tüm gereksinimlerin (`REQ-001`..`REQ-043`) kapanma durumunu, **17 RBAC rolü** için yenilenen arayüz mimarisini (`/podmin`, `/podmin/tasarim`, `/podmin/editor`, `/hesabim`) ve canlı ortam doğrulama sonuçlarını özetler.

### İncelenen Müşteri Dokümanları (`docs/specifications/`)
1. `altyapi-gereksinimleri-v6.pdf` (6 sayfa + gömülü makale içi `"RELATED"` örnek görseli)
2. `hedef-kitle-gelir-kaynagi-analizi-v6.docx` (Hedef kitle segmentasyonu, içerik tipleri, çoklu alan adı stratejisi ve gelir modeli)
3. `proje-plani-ve-teslim-takvimi.pdf` (`POF-MSI-2026-V5` — 6 haftalık anahtar teslim kapsamı, ETL/301, DRP/RPO<1h, açık standart dışa aktarım)
4. `site-haritasi-v4.docx` (3 kaynak yayın modeli, SSOT çoklu yerleşim kuralları, temel site haritası ve gelecek fazlar)

### Başlıca Kritik Bulgular ve Çözümler
- **Canlı İkili (Binary), `/podmin` Geçidi & 404 SEO Hijyeni:** `ADMIN_PATH_SECRET=podmin` olarak yapılandırıldı; `/podmin`, `/podmin/tasarim`, `/podmin/tasarim/onizleme` ve `/podmin/editor` birincil yol haline getirildi. Emekliye ayrılan `/msi-komuta` (büyük/küçük harf ve URL-encoded varyantları dahil) her zaman `404 Not Found` + HTTP `x-robots-tag: noindex, nofollow` + HTML `<meta name="robots" content="noindex, nofollow" />` döndürür ve 404/`noindex` yanıtlarında hatalı `<link rel="canonical">`, `hreflang`, `x-default`, `og:url` veya anasayfa JSON-LD yayımı tamamen bastırılır (`src/controllers/web_controller/home.rs`, `src/engine/seo_router_engine/meta.rs`).
- **Yönetici Giriş Ekranı (`/podmin`), 18 Sekmeli Komuta Merkezi, Tasarım Stüdyosu & WASM Editör Stüdyosu:** Split-screen `/podmin` giriş kapısı (`public/images/admin/podmin-fighter-vertical.webp`), Obsidian Koyu / Slate Açık tema, TOTP 2FA doğrulama ve 18 sekmeli modüler komuta merkezi modern arayüze taşındı; `/podmin/tasarim` ve Pure-Rust WASM tabanlı `/podmin/editor` (`017_editorial_studio.sql`, `src/editorial/`, `storage/editorial-pkg/`) ile tüm rollerde `?v=35` senkron önbellek kırma ve WCAG AA kontrast sağlandı.
- **Operasyonel Varsayılan Değerler (Sıfır Mock Veri):** `recommendation_rules` (`7` savunma sektörü kuralı), `custom_fields` (`7` alan tanımı: `3 content`, `2 ad`, `2 user`), `editorial_research_sources` (`6` RSS/araştırma kaynağı) ve `storage/edergi/` (`22` geçerli çok sayfalı başlangıç arşiv PDF'si: `15 msi` 6 sayfa + `7 msi_tdr` 5 sayfa; yetim `281` PDF temizlendi) eksiksiz çalışır duruma getirildi.
- **PayTR Çift Rota Uyumu & Canlı E-Posta Teslimi:** `POST /api/v1/payments/paytr-callback` ve uyumluluk takma adı `POST /api/v1/payments/paytr/callback` HMAC-SHA256 doğrulamasıyla aktif edildi; `msi-rust-web.service` systemd sanal alanı `plesk-sendmail` ve doğrulanmış `-f <envelope_from>` zarf göndericisiyle canlı e-posta teslimi için düzeltildi.

---

## 2. Tam Müşteri Gereksinimleri İzlenebilirlik Matrisi (`REQ-001` – `REQ-043`)

### Özet Durum Dağılımı
| Durum | Kapsanan Maddeler | Adet |
|---|---|---|
| **Doğrulandı & Sertleştirildi (Mevcut + M1–M4 İyileştirmeleri)** | `REQ-001` – `REQ-043` (Tüm kapsam) | **43 / 43** |
| **Harici Müşteri Hesabı / İçeriği Gerektiren Dış Adımlar** | Alan adı DNS/SSL, Gerçek Baskı PDF'leri, S3 Anahtarları, PayTR Panel Teyidi (Bkz. Bölüm 6) | **4 Dış Madde** |

### Tüm Gereksinimlerin Satır Satır Doğrulama Matrisi (`REQ-001` – `REQ-043`)

| Kod | Gereksinim | Uygulama & Dosya / Tablo Kanıtı | Doğrulama Durumu |
|---|---|---|---|
| **REQ-001** | Çoklu alan adı tek veritabanı (`msidergisi.com` / `savunmahaber.com`) | `src/engine/seo_router_engine/brand.rs`, `src/controllers/web_controller/helpers.rs` (`Host` başlığı yönlendirmesi) | **Doğrulandı** |
| **REQ-002** | TR/EN dil yönetimi & kalite ağırlıklı `Accept-Language` | `?lang=` > `msi_lang` çerezi > ağırlıklı `Accept-Language`; `/en`, `/switch/:locale` (`helpers.rs`) | **Doğrulandı** |
| **REQ-003** | DB tabanlı çeviriler, onay akışı, savunma sözlüğü & yereller | `migrations/012..014`, `src/db/repo/i18n_glossary/`, `/api/v1/admin/translations*`, `/api/v1/admin/glossary*` | **Doğrulandı** |
| **REQ-004** | Duyarlı mobil/masaüstü portal & PWA manifest | `src/engine/design/shell.rs`, `public/css/msi-design-system.css`, `/manifest.webmanifest` (`1440x900` & `390x844` sıfır taşma) | **Doğrulandı** |
| **REQ-005** | Anasayfa bölümleri & canlı Görsel Tasarım Stüdyosu | `src/engine/design/editor.rs`, `editor.html`, `/podmin/tasarim`, `/podmin/tasarim/onizleme` | **Doğrulandı** |
| **REQ-006** | Kişiselleştirilmiş akış & 14+ kanonik SSB etiketi | `/api/v1/public/personalized`, `/api/v1/ssb-tags` (`ssb_tags = 14` satır) | **Doğrulandı** |
| **REQ-007** | SSOT çoklu yerleşim yayıncılığı (`content_placements`) | Tek içerik kaydının `content_placements` üzerinden çoklu bölümde yayını (`src/db/repo/contents.rs`) | **Doğrulandı** |
| **REQ-008** | Kategoriler & Özel Alan (`custom_fields`) varsayılanları | `49` kategori + `7` `custom_fields` varsayılanı (`3 content`, `2 ad`, `2 user`), `src/db/repo/custom_fields.rs`, `/api/v1/admin/custom-field*` | **Doğrulandı** |
| **REQ-009** | Editöryal onay kuyruğu & çift dilli sürüm geri alma | `migrations/006, 011`, `src/db/repo/content_versions.rs`, `src/engine/design/admin/workflow.rs` | **Doğrulandı** |
| **REQ-010** | Yasal Sponsorlu / İlandır / Advertorial etiketlemesi | `src/engine/design/reader.rs` (`Sponsorlu İçerik`, `İlandır`, `Advertorial` rozetleri) | **Doğrulandı** |
| **REQ-011** | Medya teknik kuralları (`<=25MB`/`<=12MB`), WebP boyutları & kütüphane | `src/services/editorial_media_service.rs`, `src/db/repo/media.rs`, `migrations/017` | **Doğrulandı** |
| **REQ-012** | Makale içi + altı ilgili içerikler & Öneri Kuralları | `reader.rs` (`<aside data-related-inline>`), sponsor hariç tutma, `recommendation_rules = 7` kural, `/api/v1/public/related` | **Doğrulandı** |
| **REQ-013** | 3+ görünürlük seviyesi (`0`/`1`/`2`) & 17 RBAC rolü izolasyonu | `user_roles = 17`, `src/extractors/operator_user.rs`, `src/extractors/require_permission.rs` | **Doğrulandı** |
| **REQ-014** | Anonim okuyucu teaser kesme & `lock_reason` koruması | `src/controllers/web_controller/articles.rs`, `src/services/content_service.rs` (SSR + arama/API uyumu) | **Doğrulandı** |
| **REQ-015** | Kayıt alanları (Ad Soyad, kurum sınıfı, kurumsal e-posta, SSB, `verification_note`, KVKK) | `POST /api/v1/auth/register` (`src/controllers/api_controller/auth.rs`) | **Doğrulandı** |
| **REQ-016** | `.gov.tr`/`.tsk.tr`/`.mil.tr`/`.pol.tr` otomatik doğrulama & operatör havuzu | `gov_email_domains` otomatik `subscriber_public` terfisi + `tab-4` kurumsal onay havuzu (`commercial.rs`) | **Doğrulandı** |
| **REQ-017** | `/hesabim` (5 sekme), e-posta doğrulama, parola sıfırlama & TOTP 2FA | `src/engine/design/account.rs`, `src/controllers/api_controller/totp.rs`, `/hesap-dogrula`, `/sifre-sifirla` | **Doğrulandı** |
| **REQ-018** | Tarayıcı tabanlı E-Dergi Flipbook okuyucu | `#issue-dialog` (`dialogs.rs`, `msi-ui.js`), `GET /api/v1/edergi/:issue_no/pages/:page_num` | **Doğrulandı** |
| **REQ-019** | Tek tıkla Baskı PDF → Web E-Dergi dönüştürücü | `src/services/edergi_pdf_service/converter.rs`, `POST /api/v1/admin/issues/:publication_type/:issue_number/convert-pdf` | **Doğrulandı** |
| **REQ-020** | Kısmi/Tam DRM filigranlı PDF indirme & 22 geçerli başlangıç PDF'si | `src/services/edergi_pdf_service/watermark.rs`, `?pages=...`, `storage/edergi/` (`15 msi` 6p + `7 msi_tdr` 5p) | **Doğrulandı** |
| **REQ-021** | Reklam formatları (standart + floating/pop-up) & çok boyutlu hedefleme | `src/services/ad_service.rs`, `src/engine/design/shell.rs`, 404/auth rotalarında floating popup engeli (`home.rs:279`) | **Doğrulandı** |
| **REQ-022** | Reklam kota çakışması (`409 Conflict`) & takvim kalıcılığı | `migrations/016` (`start_date`, `end_date`, `rotation_mode`), `force_override`, `/api/v1/admin/ads/check-conflict` | **Doğrulandı** |
| **REQ-023** | Sponsorlu içeriklerde rakip reklam koruması | `reader.rs`, `ad_service.rs` (sponsorlu sayfalarda dış rakip reklam bastırma) | **Doğrulandı** |
| **REQ-024** | Okuma derinliği, video, reklam & gerçek `data-ssb-tag` telemetrisi | `reader.rs` (`data-ssb-tag`), `public/js/msi-ui.js`, `POST /api/v1/analytics/track` | **Doğrulandı** |
| **REQ-025** | Okuyucu/kohort analitiği & `k >= 3` anonimleştirme | `/api/v1/admin/analytics/readers` (`src/controllers/api_controller/admin_analytics.rs`) | **Doğrulandı** |
| **REQ-026** | Kurumsal içerik/reklam analitiği & otomatik ay sonu B2B e-posta raporu | `src/workers/b2b_report_worker.rs`, `/api/v1/admin/analytics/sponsor-report` | **Doğrulandı** |
| **REQ-027** | E-Bültenler, 8 e-posta grubu, zamanlama, sponsor bandı & RFC 8058 abonelik iptali | `email_groups = 8`, `newsletter_scheduler.rs`, `src/services/email_service.rs`, canlı `plesk-sendmail` | **Doğrulandı** |
| **REQ-028** | Otomatik abonelikler, PayTR POS & fatura üretimi | `POST /api/v1/payments/paytr-callback` + `/api/v1/payments/paytr/callback` (`api_routes.rs:66-67`), HMAC + oto-fatura | **Doğrulandı** |
| **REQ-029** | 5 aşamalı abonelik yenileme hatırlatıcısı (`2a/1a/2h/1h/1g`) | `src/workers/renewal_reminder.rs` (yenilemede otomatik durdurma dahil) | **Doğrulandı** |
| **REQ-030** | İsteğe bağlı abone yaşam döngüsü raporları (kohort, kayıp, LTV, huni) | `/api/v1/admin/reports/subscribers`, `public/js/admin/analytics.js` | **Doğrulandı** |
| **REQ-031** | E-Ticaret (Kitap, Rapor, RBF Fotoğraf) & korumalı dijital SKU indirme | `/abonelik-ve-satis`, `commerce_products = 4`, `subscription_plans = 3`, `src/controllers/api_controller/commerce.rs` | **Doğrulandı** |
| **REQ-032** | KVKK izin denetim kayıtları, JSON veri dışa aktarım & hesap silme | `kvkk_consent_logs`, `/api/v1/account/export`, `/api/v1/account/delete` | **Doğrulandı** |
| **REQ-033** | Siber güvenlik sertleştirmesi & `/podmin` geçidi (`/msi-komuta*` → 404) | WAF, CSP nonce, Argon2id, Ammonia, CSRF, `revoked_tokens`, `/msi-komuta*` `404` + `noindex, nofollow` (sıfır canonical) | **Doğrulandı** |
| **REQ-034** | Şifreli artımlı DRP (`MSIBKP01` + `MSIDELTA1`, RPO<1h) & açık veri ihracı | `src/workers/backup_worker.rs` (isteğe bağlı AWS SigV4 S3 kancası + `storage/db/backups/`) | **Doğrulandı** |
| **REQ-035** | ETL, `1.334` 301 yönlendirme, XML/News/Image sitemap, RSS, JSON Feed, `llms.txt`, IndexNow | `seo_redirects = 1334`, `src/engine/seo_router_engine/`, `src/controllers/web_controller/feeds.rs` | **Doğrulandı** |
| **REQ-036** | Tarihçe, yazarlar, öğrenci kulüpleri, etkinlikler & kurumsal sayfalar | `/savunma-sanayii-tarihi`, `/yazarlar`, `/yazar/:slug`, `/ogrenci-kulupleri`, `/etkinlikler`, `/kurumsal/:slug` | **Doğrulandı** |
| **REQ-037** | `/podmin` split-screen giriş, 18 sekmeli Komuta Merkezi & `/podmin/tasarim` | `src/engine/design/admin/*`, `public/css/admin/*`, `podmin-fighter-vertical.webp`, `editor.rs` (`?v=35`) | **Doğrulandı** |
| **REQ-038** | Tüm 17 RBAC rolü için `/hesabim` portalı & personel `/podmin/editor` kısayolu | `src/engine/design/account.rs`, `public/css/account-portal.css`, `public/js/account-portal.js` (`?v=35`) | **Doğrulandı** |
| **REQ-039** | Option-B `target_institutions` UI & kalıcılığı | `migrations/015`, `public/js/admin/editorial.js`, `src/db/repo/contents.rs` | **Doğrulandı** |
| **REQ-040** | Bileşik anahtar `(publication_type, issue_number)` (`msi` vs `msi_tdr`) | `migrations/011`, `magazine_issues = 22` (`15 msi`, `7 msi_tdr`), `issues.js`, `admin_editorial.rs` (`409` ayrıştırma) | **Doğrulandı** |
| **REQ-041** | Admin Kullanıcılar (`tab-11`) sayfalama, 17-rol matrisi CRUD & denetim günlüğü | `src/engine/design/admin/system.rs`, `src/controllers/api_controller/admin_users_roles.rs` | **Doğrulandı** |
| **REQ-042** | Pure-Rust WASM Editör Stüdyosu (`017_editorial_studio.sql`, `/podmin/editor`) | `src/editorial/`, `storage/editorial-pkg/`, `31` `/api/v1/admin/editorial/*` ucu, kilitler, revizyonlar, `6` RSS kaynağı, `editorial_scheduler` | **Doğrulandı** |
| **REQ-043** | Depo hijyeni, `--release` dağıtımı, `cargo test --all`, E2E & canlı e-posta/HTTPS doğrulaması | `.gitignore`, `/usr/local/bin/msi-dergisi-platform`, `msi-rust-web.service`, Bölüm 5 canlı kanıtları | **Doğrulandı** |

---

## 3. Admin Paneli, Tasarım Stüdyosu, Editör Stüdyosu ve Rol Portalları

### 3.1. Giriş Geçidi (`/podmin`) ve Yönetim Komuta Merkezi (`tab-1` .. `tab-18`)
- **Görsel Sistem (`public/css/admin/{tokens,layout,components,login}.css`):** Yerel `IBM Plex Sans` & `IBM Plex Mono` tipografisi, Açık (Slate-Paper) + Koyu (Obsidian Charcoal) tema desteği, CSP nonce uyumlu tema ön-yükleme (`THEME_PREHYDRATE_JS`), imleç takipli cam efektli kartlar (`[data-glow-card]`), satır içi SVG ikon primitifleri (`src/engine/design/admin/ui.rs`), tüm statik varlıklarda senkronize `?v=35` önbellek kırma parametresi.
- **Giriş Ekranı (`/podmin`):**
  - Masaüstünde iki sütunlu kurumsal komuta sahnesi (`public/images/admin/podmin-fighter-vertical.webp` görseli, cam efektli form kartı), sağda/kart içinde çift temalı MSI logosu, e-posta/parola girişi, boşluk temizlemeli TOTP 2FA doğrulama adımı, süresi dolmuş geçici token kurtarma, Escape/klavye/ARIA desteği ve `prefers-reduced-motion` uyumlu GSAP sahnesi.
  - Mobilde (`<= 980px`) giriş kartı otomatik olarak üst sıraya (`order: -1`) alınarak kaydırma gerektirmeden erişilebilir hale getirilmiştir.
- **18 Sekmeli Modüler Komuta Merkezi (`src/engine/design/admin/`):** 5 alan grubunda toplanmış hızlı modül filtrelemeli kenar çubuğu, gerçek zamanlı onay kuyruğu rozeti, rol bazlı sunucu + istemci yetki izolasyonu (`data-required-perm`), `tab-11` kullanıcı sayfalama ve 17-rol RBAC matrisi.

### 3.2. Görsel Tasarım Stüdyosu (`/podmin/tasarim`)
- `src/engine/design/editor.html`, `src/engine/design/editor.rs`, `public/css/msi-design-editor.css`, `public/js/msi-design-editor.js` (`?v=35`):
  - Masaüstünde yan yana kontrol paneli ve canlı önizleme `iframe`, mobilde (`390x844`) 36px+ dokunmatik hedefli `Kontroller / Canlı Önizleme` geçiş çubuğu (`.mobile-studio-switcher`), `#editor-account-link` (`/hesabim`), RBAC kontrollü `#editor-studio-link` (`/podmin/editor`) ve `data-admin-path="/podmin"` dönüş bağlantısı.

### 3.3. Pure-Rust WASM Editör Stüdyosu (`/podmin/editor`)
- `src/editorial/` (`common.rs` + `wasm/*.rs`), `src/engine/design/editorial_studio.rs`, `public/css/admin/editorial-studio.css` (`?v=35`), `storage/editorial-pkg/{editorial_studio.js,editorial_studio_bg.wasm}` (`/{admin}/editor/studio.wasm` gömülü `editorial_studio_bg.wasm` dosyasını, `/{admin}/editor/studio.js` ise `editorial_studio.js` dosyasını sunar):
  - Tek Cargo paketi altında derlenen Pure-Rust WebAssembly editör arayüzü (`--build-editorial-wasm`) ve `/api/v1/admin/editorial/*` altında `31` REST ucu.
  - İçerik kilitleri (`editorial_lock_service.rs`), sürüm revizyonları ve geri alma, `6` tohumlanmış RSS/web araştırma kaynağı (`editorial_research_sources`), görsel medya kütüphanesi & filigran, X/Telegram sosyal paylaşım, `(publication_type, issue_number)` `409` ayrıştırması, `editorial_scheduler` zamanlı yayın, sRGB WCAG AA kontrastı, dinamik `admin_path()` gezinme bağlantıları, mobil `#menu-msi-theme-toggle` ve `Tasarım Stüdyosu` kısayolu.

### 3.4. Giriş Yapılmış Rol / Okuyucu Portalı (`/hesabim`)
- `src/engine/design/account.rs`, `public/css/account-portal.css`, `public/js/account-portal.js` (`?v=35`):
  - Kamu ziyaretçi sayfalarından tamamen izole edilmiş, Açık/Koyu temalı Executive Workspace; WCAG AA yüksek kontrast tokenları (`#73521d`, `#ff8f8f`, `#b2bac8`).
  - **17 RBAC Rolünün Tamamına Uyarlanmış Deneyim:**
    - **Personel Rolleri (`12` rol):** Yetkiye göre `/podmin` Komuta Merkezi, izinli roller için `/podmin/tasarim` ve `contents:write` yetkili 8 rol (`can_access_editorial_studio`) için `/podmin/editor` hızlı geçiş kartları, TOTP 2FA güvenliği ve hesap yönetimi.
    - **Okuyucu ve Abone Rolleri (`5` rol):** Genel Bakış, `publication_type` & `view=flipbook` bağlantılı E-Dergi Rafı (filigranlı PDF okuma/kısmi indirme), Profil & dinamik `14` SSB İlgi Alanı, Güvenlik (TOTP 2FA, parola yenileme, KVKK izin geçmişi, JSON veri dışa aktarım, hesap silme) ve Bildirimler sekmeleri.

---

## 4. Veritabanı, Arka Plan İşçileri ve Altyapı Hazırlığı

- **Şema & Migrasyonlar:** SQLite (`storage/db/msi_platform.db`, WAL modu) üzerinde **`52` tablo**, `migrations/001`..`017_editorial_studio.sql` (`PRAGMA user_version = 17`) idempotent olarak uygulanmıştır. Ayrıca **`migrations_pg/001`..`017`** ile tam PostgreSQL 16 referans şema paritesi sağlanmıştır.
- **Canlı Veritabanı Envanteri:** `contents=1.010`, `seo_redirects=1.334`, `categories=49`, `ssb_tags=14`, `user_roles=17`, `magazine_issues=22` (`15 msi` + `7 msi_tdr`, `storage/edergi/` altında `22` geçerli çok sayfalı başlangıç PDF'si), `recommendation_rules=7`, `custom_fields=7` (`3 content`, `2 ad`, `2 user`), `email_groups=8`, `commerce_products=4`, `subscription_plans=3`, `editorial_research_sources=6`, `users=16`.
- **Arka Plan İşçileri (`src/workers/` — 7 İşçi):**
  1. `analytics_aggregation`: Günlük okuma derinliği ve trafik toplulaştırma.
  2. `b2b_report_worker`: Ay sonu içerik ve reklam bazlı kurumsal dağılım e-postası.
  3. `backup_worker`: AES-256-GCM şifreli (`MSIBKP01`), 4 KB sayfa farkı (`MSIDELTA1`) destekli artımlı DRP yedekleme ve isteğe bağlı AWS SigV4 S3 replikasyonu (`storage/db/backups`).
  4. `editorial_scheduler`: Editör Stüdyosu üzerinden zamanlanmış içeriklerin otomatik yayını.
  5. `monitoring`: Sistem, alan adı, trafik, saldırı ve servis sağlığı izleme.
  6. `newsletter_scheduler`: Zamanlanmış e-bülten ve hedef e-posta grubu dağıtımı.
  7. `renewal_reminder`: 5 aşamalı abonelik yenileme hatırlatıcısı.
- **Systemd MTA / Postfix `plesk-sendmail` Entegrasyonu:** `/etc/systemd/system/msi-rust-web.service` biriminde `NoNewPrivileges=false`, `CapabilityBoundingSet=CAP_SETUID CAP_SETGID CAP_DAC_OVERRIDE CAP_DAC_READ_SEARCH CAP_CHOWN CAP_FOWNER` ve `/var/spool/postfix/public`, `/var/spool/postfix/maildrop`, `/usr/local/psa/handlers/spool`, `/var/tmp` `ReadWritePaths` izinleri tanımlanmış; `src/services/email_service.rs` içinde doğrulanmış `-f <envelope_from>` ile `SENDMAIL_BIN -t -i -f ...` çağrısı güvence altına alınmıştır.

---

## 5. Müşteriye Satış & Canlı Yayın Doğrulama Özeti

- **Derleme, WASM & Statik Analiz (`2026-11-22`):**
  - `cargo run --bin msi-dergisi-platform -- --build-editorial-wasm`: Gömülü Editör Stüdyosu (`storage/editorial-pkg/{editorial_studio.js,editorial_studio_bg.wasm}`; `/{admin}/editor/studio.wasm` gömülü `editorial_studio_bg.wasm` ikilisini sunar) hatasız derlendi.
  - `cargo clippy --workspace --all-targets`: 0 hata ile tamamlandı.
  - `cargo build --release`: 0 hata ile tamamlandı (optimize release ikilisi; `/usr/local/bin/msi-dergisi-platform` ve canlı systemd süreci ile özdeş SHA-256: `431c61fbaa7f6bf4820c33f5c0c54bc9524c7400102b294a24ad1466d7f099c9`).
- **Otomatik Rust Test Paketi (`cargo test --all`):**
  - Kütüphane birim testleri (`src/lib.rs`: `199 passed, 1 ignored`), `16` aktif entegrasyon test paketi (`130 passed` — `account_portal_test`, `admin_access_control_test`, `admin_rbac_matrix_test`, `auth_flow_test`, `content_crud_test`, `editorial_studio_integration_test`, `fts_smoke`, `live_parity_routes_test`, `m1_spec_parity_test`, `m2_edergi_test`, `m3_reader_parity_test`, `m4_admin_ui_test`, `option_b_api_test`, `payment_webhook_test`, `seo_generation_test`, `tier5_adversarial_test` [`28/28 passed`]) ve doc-test (`1 ignored`) dahil **toplam `329 passed, 0 failed, 2 ignored`** (`0` hata).
  - **Faz 1 & Faz 2 Tier 5 Güvenlik / Bütünlük Sertleştirmeleri:** `admin_upsert_content`, `admin_bulk_content_operation`, `admin_create_content_version` (negatif `content_id` için `400`, mevcut olmayan `id` için `404`), `admin_restore_content_version` ve `submit_content_for_review` uçlarında `contents:publish` yaşam döngüsü koruması (`""` dahil `admin_editorial.rs` hizalaması) + `Path(id)`/`content_id` doğrulaması ve `content_versions` ekleme/geri yükleme sınırında `sanitize_html`/`sanitize_plain_text` temizliği (`published`/`scheduled`/`archived` kayıtların yetkisiz değiştirilmesini/geri çekilmesini engellerken `draft` düzenlemelerini korur); `validate_svg_safe` için UTF-8 karakter sınırı panik koruması (`rest.as_bytes().get(..5)` ile `<?\xc4\x9f\xc4\x9f>` / `<?\xc3\xa9\xc3\xa9>` gibi çok baytlı UTF-8 işleme talimatlarının paniksiz `400` reddi; `svg_rejects_multibyte_utf8_after_pi_without_panic`), XML ad alanı (`prefix:` yerel ad ayrıştırma) ve yapısal etiket/öznitelik tarayıcısı (`<s:script>`, `<h:script>`, `<s:foreignObject>`, `<s:set>`, `<style>`, `<!DOCTYPE`/`<!ENTITY`, `<?xml-stylesheet`, `data:image/svg+xml`, `\x0b`/`\x0c` olay ayırıcıları dahil tam engel) ile `/uploads/editorial/` ve `/images/uploads/` altındaki yol segmenti normalizasyonu (`//`, `/./`, `..`, `/images//uploads/...`, `/images/./uploads/...`, `/images/%75ploads/...`, `/uploads//editorial/...`, `/uploads/./editorial/...`) dahil düz ve yüzde-kodlanmış (`.%73vg`, `%2esvg`, `%2e%73%76%67`, `Content-Type: image/svg+xml`) SVG yanıtlarında `X-Content-Type-Options: nosniff` + `Content-Security-Policy: sandbox; default-src 'none'`; PayTR `KURUMSAL` büyük/küçük harf duyarsız abonelik rol yükseltmesi, fatura no çakışma dönüşü ve `BEGIN IMMEDIATE` eşzamanlı callback kilitlemesi; `resend-verification` hız sınırlaması (`429`) + eski doğrulama jetonu iptali ve `totp_verify_setup` kaba kuvvet kilidi; kişiselleştirilmiş akışta Türkçe `'İ'` vaka katlama (`fold_tr`/`fold_eq_tr`) ve `institution_categories` takma ad eşlemesi (Faz 1 `T3-55` / `T3-01` iyileştirmeleriyle birlikte); `sanitize_inline_rich_html` kapanmamış `<` kaçışı; sıralı yerinde `refresh_l1_sections` güncellemesi; `noindex` rotalarda HTTP `Link: rel="canonical"` ve HTML `<link rel="canonical">` bastırması (`t5_not_found_responses_omit_canonical_and_emit_noindex`, `t5_noindex_routes_omit_link_canonical_header`); `ArticleRevision.id` (`i64`) ve `editorial_publish_due_scheduled` TR saat dilimi + atomik `WHERE status = 'scheduled'` koruması.
- **Uçtan Uca Playwright & UX Kalite Denetimleri:**
  - `tests/e2e/` (`E2E_BINARY=/usr/local/bin/msi-dergisi-platform`): `smoke` (`6/6`), Tier 1 (`259/259`), Tier 2 (`252/252`), Tier 3 (`60/60`) ve Tier 4 (`46/46`) genelinde **toplam `623 / 623` E2E test** senaryosu 0 hata ile geçmiştir (`TEST_READY.md`).
  - `ux_rubric_audit.mjs --strict` (CSS Color 4 `color(srgb ...)` gradyan ve `alpha === 0` saydam durak filtreleme desteği dahil): `/podmin`, 18 yönetici sekmesi, `/podmin/tasarim`, `/podmin/editor`, `/hesabim` tüm rol portalları ve kamu sayfaları genelinde **`104/104`** (`core=104, strict=104, reducedMotion=true`) geçer not almış; masaüstü (`1440x900`) ve mobil (`390x844`) görünümlerde sıfır yatay taşma ve sıfır konsol/ağ hatası doğrulanmıştır.
- **Canlı Sistem, Veritabanı & E-Posta Teslim Doğrulaması (`https://trusting-brahmagupta.194-110-169-165.plesk.page`):**
  - **Systemd Servisi:** `msi-rust-web.service` `active (running)` (`PRAGMA user_version = 17`, `52` tablo, `recommendation_rules=7`, `custom_fields=7`, `magazine_issues=22`, `storage/edergi/` `22` PDF, `journalctl` **0 `WARN`/`ERROR`**).
  - **Canlı HTTPS Uçları:** `/api/v1/health` (`200 OK`), `/podmin` (`200 OK`, `?v=35` + `podmin-fighter-vertical.webp`), `/msi-komuta` (`404 Not Found`, `x-robots-tag: noindex, nofollow`, `<meta name="robots" content="noindex, nofollow" />`, sıfır `canonical`/`hreflang`/`og:url`), `/podmin/tasarim` (`303 -> /podmin`), `/podmin/editor` (`303 -> /podmin`), `/hesabim` (`200 OK`, `?v=35`, `x-robots-tag: noindex, nofollow, noarchive`, sıfır `Link: rel="canonical"`), `/api/v1/public/personalized?interests=%C4%B0HA` (`200 OK`, Türkçe `'İ'` eşlemesi), `/api/v1/public/related` (normal yazıda ilişkili kayıtlar, sponsorlu yazıda `[]`), `POST /api/v1/payments/paytr-callback` ve `/paytr/callback` (hatalı imzada `403`).
  - **Canlı `plesk-sendmail` Doğrulaması:** `POST /api/v1/auth/register` ile `mail@ilkeraydogdu.com.tr` adresine gerçek doğrulama e-postası gönderilmiş; `/var/log/maillog` üzerinde `plesk-sendmail` (`dk_sign: PASS`, `dmarc: PASS`, `arc-sign: PASS`) ve `postfix/pipe` (`status=sent (delivered via plesk_virtual service)`) + Dovecot `INBOX` teslimi kanıtlanmış, ardından geçici test kaydı veritabanından ve L1 önbellekten temizlenmiştir (`test kullanıcısı = 0`).

---

## 6. Müşteri Teslimi Öncesi Dış Hesap / İçerik Kontrol Listesi (Yalnızca 4 Dış Madde)

Yazılım, güvenlik, operasyonel varsayılanlar, e-posta gönderimi ve canlı sunucu altyapısı teknik olarak %100 çalışır durumda olup, müşteriye son anahtar teslimi öncesinde yalnızca müşteri dış hesapları/varlıkları gerektiren şu **4 dış adım** kalmıştır:

1. **Gerçek Alan Adları & DNS / SSL Geçişi:** `msidergisi.com` ve `savunmahaber.com` DNS kayıtlarının sunucuya yönlendirilmesi, SSL sertifikalarının atanması ve `.env` içinde `PUBLIC_URL="https://msidergisi.com"` (ve istenirse `SMTP_FROM="[REDACTED_SESSION_FALLBACK]"`) güncellemesi.
2. **Gerçek Çok Sayfalı Baskı PDF'lerinin Yüklenmesi:** `storage/edergi/` altındaki 22 geçerli başlangıç arşiv PDF'sinin (`6` / `5` sayfa) yerine müşterinin gerçek çok sayfalı baskı PDF'lerini `/podmin` → **E-Dergi & Sayılar (`tab-18`)** üzerinden yükleyip tek tıkla web formatına dönüştürmesi.
3. **Harici S3 Off-Site Yedekleme Anahtarları (İsteğe Bağlı):** Yerel AES-256-GCM şifreli artımlı yedekler (`storage/db/backups/`) otomatik alınmaktadır; bulut S3 replikasyonu için `.env` içindeki `S3_BACKUP_ENDPOINT`, `S3_BACKUP_BUCKET`, `S3_BACKUP_ACCESS_KEY` ve `S3_BACKUP_SECRET_KEY` alanlarının müşterinin S3 kovası ile doldurulması.
4. **PayTR Mağaza Paneli Geri Bildirim URL'si & İsteğe Bağlı Çeviri API Anahtarı:** PayTR mağaza panelinde bildirim URL'sinin `https://msidergisi.com/api/v1/payments/paytr-callback` olarak teyit edilmesi (uyumluluk için `/api/v1/payments/paytr/callback` takma adı da aktiftir) ve istenirse **Motorlar & Ayarlar (`tab-8`)** üzerinden LibreTranslate/DeepL API anahtarının girilmesi.
