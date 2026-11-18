use super::{CentralSeoEngine, SeoBrandConfig};
use crate::models::{CategoryNode, ContentItem, MagazineIssue, SsbInterestTag};

impl CentralSeoEngine {
    /// Generates `/llms.txt` and `/llms-full.txt` (AI / LLM / GEO Discovery Standard).
    /// Header text and publisher identity sourced from `SeoBrandConfig`.
    pub fn generate_llms_txt(
        base_url: &str,
        categories: &[CategoryNode],
        ssb_tags: &[SsbInterestTag],
        issues: &[MagazineIssue],
        contents: &[ContentItem],
        full: bool,
        brand: &SeoBrandConfig,
    ) -> String {
        let clean_base = Self::normalize_base_url(base_url);
        let mut out = String::with_capacity(if full { 131_072 } else { 24_576 });

        out.push_str(&format!("# {} & MSI Turkish Defence Review (MSI TDR)\n\n", brand.opensearch_short_name));
        out.push_str(&format!("> {}\n\n", brand.llms_header_text));
        out.push_str("## Kurumsal Kimlik & Otorite (Entity Facts)\n");
        out.push_str(&format!("- **Resmi Yayın Adları:** {} (Türkçe, {}–günümüz, 286+ sayı), MSI Turkish Defence Review / MSI TDR (İngilizce, 2014–günümüz, 153+ sayı), AMAC Magazine (Ankara Askerî Ataşeler Birliği resmi yayını, 2015–günümüz)\n", brand.opensearch_short_name, brand.founding_date));
        out.push_str(&format!("- **ISSN:** {}\n", brand.issn));
        out.push_str(&format!("- **Yayıncı Tüzel Kişilik:** {}\n", brand.publisher_legal_name));
        out.push_str(&format!("- **Merkez Adres:** {} {} / {}, Türkiye • Tel: {}\n", brand.address_street, brand.address_locality, brand.address_region, brand.contact_phone));
        out.push_str(&format!("- **Ana Web Adresi:** {}/\n", clean_base));
        out.push_str(&format!("- **Resmi Künye (Masthead):** {}/kurumsal/kunye\n", clean_base));
        out.push_str(&format!("- **Hakkımızda & Tarihçe:** {}/kurumsal/hakkimizda | {}/kurumsal/yolculugumuz\n", clean_base, clean_base));
        out.push_str(&format!("- **Yayın Etiği & Telif Politikası:** {}/kurumsal/telif-ve-etik\n", clean_base));
        out.push_str(&format!("- **KVKK & Hukuki Metinler:** {}/kurumsal/kvkk | {}/kurumsal/gizlilik-politikasi | {}/kurumsal/cerez-politikasi | {}/kurumsal/mesafeli-satis-sozlesmesi\n\n", clean_base, clean_base, clean_base, clean_base));

        out.push_str("## Makine Okunabilir Veri Akışları (Machine-Readable Feeds)\n");
        out.push_str(&format!("- XML Sitemap: {}/sitemap.xml\n", clean_base));
        out.push_str(&format!("- Google/Yandex News Sitemap: {}/sitemap-news.xml\n", clean_base));
        out.push_str(&format!("- Image Sitemap: {}/sitemap-images.xml\n", clean_base));
        out.push_str(&format!("- Türkçe RSS 2.0: {}/rss.xml?lang=tr\n", clean_base));
        out.push_str(&format!("- English RSS 2.0 (MSI TDR): {}/rss.xml?lang=en\n", clean_base));
        out.push_str(&format!("- JSON Feed 1.1: {}/feed.json\n", clean_base));
        out.push_str(&format!("- Full AI Content Index: {}/llms-full.txt\n\n", clean_base));

        out.push_str("## Savunma Sanayii Teknoloji Taksonomisi (SSB Dikeyleri)\n");
        for t in ssb_tags {
            out.push_str(&format!(
                "- [{} / {}]({}/arama?ssb_tag={})\n",
                t.name_tr, t.name_en, clean_base, t.code
            ));
        }
        out.push('\n');

        out.push_str("## Ana Kategori ve Özel Dosya Merkezleri\n");
        for c in categories.iter().filter(|c| c.group_type != "kurumsal") {
            out.push_str(&format!(
                "- [{} ({})]({}/kategori/{}): {}\n",
                c.name_tr, c.name_en, clean_base, c.slug, c.description_tr
            ));
        }
        out.push('\n');

        let mut sorted_issues: Vec<&MagazineIssue> = issues.iter().collect();
        sorted_issues.sort_by(|a, b| b.issue_number.cmp(&a.issue_number));
        out.push_str("## Güncel Dergi Sayıları (MSI Dergisi & MSI TDR Dijital Kütüphane)\n");
        for iss in sorted_issues.iter().take(if full { crate::db::constants::LLMS_FULL_ISSUE_LIMIT } else { crate::db::constants::LLMS_PREVIEW_ISSUE_LIMIT }) {
            out.push_str(&format!(
                "- [{}]({}/e-dergi?issue={}): {}\n",
                iss.title_tr, clean_base, iss.issue_number, iss.summary_tr
            ));
        }
        out.push('\n');

        let limit = if full { crate::db::constants::SITEMAP_FULL_LIMIT } else { crate::db::constants::SITEMAP_PREVIEW_LIMIT };
        out.push_str(&format!(
            "## Son Savunma Sanayii Haberleri, Özel Dosyalar ve Analizler (Toplam Arşiv: {} Makale)\n",
            contents.len()
        ));
        for item in contents.iter().take(limit) {
            let date = item.published_at.get(..10).unwrap_or(&item.published_at);
            out.push_str(&format!(
                "- [{}]({}/icerik/{}) ({}, {}): {}\n",
                item.title_tr, clean_base, item.slug, date, item.author_name, item.summary_tr
            ));
        }

        out
    }
}
