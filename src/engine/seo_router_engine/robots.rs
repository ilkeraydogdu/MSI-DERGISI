use super::CentralSeoEngine;

impl CentralSeoEngine {
    /// Generates `robots.txt` engineered for Google, Yandex (`Host:` + `Clean-param:`), Bing,
    /// and AI/LLM Answer Engines (`OAI-SearchBot`, `ChatGPT-User`, `PerplexityBot`, `ClaudeBot`, `Applebot`, `Google-Extended`).
    pub fn generate_robots_txt(base_url: &str) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let host_only = clean_base
            .trim_start_matches("https://")
            .trim_start_matches("http://");

        format!(
            "# MSI Dergisi & MSI TDR (UMSA Produksiyon) — Dynamic robots.txt
# Engineered for Google, Yandex, Bing, and AI Answer Engines (GEO)

User-agent: *
Allow: /
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /abonelik-ve-satis
Allow: /images/msi/
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /api/v1/admin/
Disallow: /api/v1/payments/
Disallow: /api/v1/auth/
Disallow: /arama?

# Google Search, News, Discover & Images
User-agent: Googlebot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/v1/admin/

User-agent: Googlebot-News
Allow: /icerik/
Allow: /kategori/

User-agent: Googlebot-Image
Allow: /images/

# Yandex Search, Yandex News & Yandex Images
User-agent: Yandex
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/
Clean-param: utm_source&utm_medium&utm_campaign&utm_term&utm_content&fbclid&gclid /

User-agent: YandexBot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/

User-agent: YandexImages
Allow: /images/

# Bing & Copilot
User-agent: bingbot
Allow: /
Disallow: /admin
Disallow: /hesabim
Disallow: /api/v1/admin/

# AI Search & Generative Engine Optimization (GEO) Crawlers
User-agent: OAI-SearchBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: ChatGPT-User
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: GPTBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: PerplexityBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: ClaudeBot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: Google-Extended
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

User-agent: Applebot
Allow: /icerik/
Allow: /kategori/
Allow: /kurumsal/
Allow: /e-dergi
Allow: /llms.txt
Allow: /llms-full.txt
Disallow: /admin
Disallow: /yonetim
Disallow: /hesabim
Disallow: /sifre-
Disallow: /hesap-dogrula
Disallow: /abonelik-iptal
Disallow: /api/

# Aggressive AI scrapers — block entirely
User-agent: Bytespider
Disallow: /

User-agent: CCBot
Disallow: /

Host: {host}
Sitemap: {base}/sitemap.xml
Sitemap: {base}/sitemap-news.xml
Sitemap: {base}/sitemap-images.xml
Sitemap: {base}/rss.xml?lang=tr
Sitemap: {base}/rss.xml?lang=en
",
            host = host_only,
            base = clean_base
        )
    }
}
