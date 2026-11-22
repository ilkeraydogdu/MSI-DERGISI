//! Editorial Research Desk (`LeftTab::Research`), Multi-Source Clean Reader View (`#hf-research-reader-sheet`),
//! and Floating Selection-to-Quote Bridge (`#hf-reader-quote-bubble`) for MSI Dergisi Editorial Studio.

use std::cell::RefCell;

use crate::editorial::common::strip_all_html_tags;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
use web_sys::{Element, MouseEvent};
#[cfg(target_arch = "wasm32")]
use web_sys::{Document, HtmlElement};

use crate::editorial::wasm::icons;
use crate::editorial::wasm::render::render_news_image;
use crate::editorial::wasm::render_right::render_preview_blocks_html;
use crate::editorial::wasm::state::{
    escape_html, get_site_name, is_valid_image_url, turkish_lowercase, AppState, ResearchReaderTab,
};

thread_local! {
    static READER_SELECTED_TEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Validates that an external link is strictly `http://` or `https://` to block `javascript:` / `data:` XSS.
pub fn safe_http_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = collapsed.chars().collect();
    if chars.len() <= max_chars {
        collapsed
    } else {
        let head: String = chars[..max_chars.saturating_sub(1)].iter().collect();
        format!("{}…", head.trim_end())
    }
}

#[cfg(target_arch = "wasm32")]
pub fn format_research_time(raw: Option<&str>) -> String {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return "Canlı".to_string();
    };
    let normalized = if s.len() >= 19 && s.as_bytes().get(10) == Some(&b' ') {
        s.replacen(' ', "T", 1)
    } else {
        s.to_string()
    };
    let d = js_sys::Date::new(&JsValue::from_str(&normalized));
    let ts = d.get_time();
    if ts.is_nan() {
        return truncate_chars(s, 22);
    }
    let diff_mins = ((js_sys::Date::now() - ts) / 60_000.0).floor();
    if diff_mins < 1.0 {
        "Az önce".to_string()
    } else if diff_mins < 60.0 {
        format!("{:.0} dk önce", diff_mins)
    } else if diff_mins < 1440.0 {
        format!("{:.0} saat önce", (diff_mins / 60.0).floor())
    } else if diff_mins < 10_080.0 {
        format!("{:.0} gün önce", (diff_mins / 1440.0).floor())
    } else {
        format!(
            "{:02}.{:02}.{:04}",
            d.get_date(),
            d.get_month() + 1,
            d.get_full_year()
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn format_research_time(raw: Option<&str>) -> String {
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return "Canlı".to_string();
    };
    truncate_chars(s, 22)
}

// ============================================================================
// 1. Left Column Research Desk (`LeftTab::Research`)
// ============================================================================

pub fn render_research_info_drawer_html(s: &AppState) -> String {
    if !s.show_research_info {
        return String::new();
    }

    const SITE_EXAMPLES: &[&str] = &[
        "site:aa.com.tr",
        "site:trthaber.com",
        "site:ekonomim.com",
        "site:ntv.com.tr",
    ];
    let mut example_chips = String::new();
    for ex in SITE_EXAMPLES {
        example_chips.push_str(&format!(
            r#"<button type="button" data-action="apply-research-example" data-query="{ex}" class="hf-research-example-chip">{ex}</button>"#
        ));
    }

    format!(
        r#"<div class="hf-research-info-card">
            <div class="hf-research-info-head">
                <span class="hf-research-info-badge">✨ Araştırma Masası Kılavuzu</span>
                <button type="button" data-action="toggle-research-info" class="hf-research-info-close" title="Kılavuzu Kapat">×</button>
            </div>
            <div class="hf-research-info-list">
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">🔗</span>
                    <div class="hf-research-info-copy">
                        <strong>Doğrudan Haber Linki Yapıştır &amp; Oku</strong>
                        <p>Herhangi bir haber sitesinden kopyaladığınız linki (<code>https://...</code>) arama kutusuna yapıştırıp <strong>Ara</strong> butonuna basın; reklam ve çerezlerden arındırılmış <strong>Temiz Okuyucu</strong> orta sütunda anında açılır.</p>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">🎯</span>
                    <div class="hf-research-info-copy">
                        <strong>Tek Site / Ajans Arama (<code>site:</code>)</strong>
                        <p>Sadece belirli bir sitenin haberlerini görmek için arama kutusuna <code>site:alanadi.com konu</code> yazabilirsiniz. Tek tıkla deneyin:</p>
                        <div class="hf-research-example-row">
                            {example_chips}
                        </div>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">❝</span>
                    <div class="hf-research-info-copy">
                        <strong>Seçimden Tek Tıkla Alıntı &amp; Bilgi Notu</strong>
                        <p>Orta sütunda okuduğunuz haberdeki herhangi bir cümleyi fareyle seçtiğinizde açılan balondan tek tıkla sağ editöre kaynaklı <strong>Alıntı</strong> veya <strong>Bilgi Notu</strong> aktarabilirsiniz.</p>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">🗂️</span>
                    <div class="hf-research-info-copy">
                        <strong>Çoklu Kaynak Teyidi &amp; Rakam/Medya Köprüsü</strong>
                        <p>6 habere kadar sekme açıp çapraz teyit yapabilir; otomatik ayıklanan <strong>Önemli Rakamları</strong> ve gömülü <strong>YouTube / X (Tweet)</strong> paylaşımlarını tek tıkla haberinize ekleyebilirsiniz.</p>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">⌨️</span>
                    <div class="hf-research-info-copy">
                        <strong>Klavye Kısayolları (Hızlı Gezinme)</strong>
                        <p>Sekmeler ve çalışma alanları arasında fareye dokunmadan ışık hızında geçiş yapın:</p>
                        <div class="hf-research-shortcuts-grid">
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>→</kbd> <span>Sonraki Sekme</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>←</kbd> <span>Önceki Sekme</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>1..6</kbd> <span>Sekmeye Doğrudan Atla</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>W</kbd> <span>Aktif Sekmeyi Kapat</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>E</kbd> <span>Haber Editörüne Dön</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>R</kbd> <span>Araştırma Masasına Geç</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>Y</kbd> <span>Yayınlananlar Paneli</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>T</kbd> <span>Taslaklar Paneli</span></div>
                            <div class="hf-shortcut-pill"><kbd>Alt</kbd> + <kbd>A</kbd> <span>Araştırma Masası Paneli</span></div>
                        </div>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">📡</span>
                    <div class="hf-research-info-copy">
                        <strong>Özel Kaynak &amp; RSS Yönetimi</strong>
                        <p>Sık takip ettiğiniz ajansları, yerel basını veya siteleri sağ üstteki <strong>Sistem Ayarları → Kaynaklar</strong> bölümünden ekleyebilir veya yönetebilirsiniz.</p>
                    </div>
                </div>
                <div class="hf-research-info-item">
                    <span class="hf-research-info-emoji">🛡️</span>
                    <div class="hf-research-info-copy">
                        <strong>MSI Dergisi · Editör Stüdyosu</strong>
                        <p>Saf Rust WebAssembly yayın altyapısı — MSI Dergisi &amp; MSI TDR (© 2026 Tüm Hakları Saklıdır).</p>
                    </div>
                </div>
            </div>
        </div>"#
    )
}

pub fn render_research_filters_html(s: &AppState) -> String {
    let scope = if s.research_scope == "global" {
        "global"
    } else {
        "tr"
    };
    let tr_cls = if scope == "tr" {
        "hf-research-scope-btn hf-research-scope-active"
    } else {
        "hf-research-scope-btn hf-research-scope-idle"
    };
    let gl_cls = if scope == "global" {
        "hf-research-scope-btn hf-research-scope-active"
    } else {
        "hf-research-scope-btn hf-research-scope-idle"
    };

    // Saved Sources horizontal filter bar (`Kaynaklarım`)
    let scoped_sources: Vec<&crate::editorial::common::ResearchSource> = s
        .research_sources
        .iter()
        .filter(|src| {
            if !src.enabled {
                return false;
            }
            let src_scope = if src.scope.trim().eq_ignore_ascii_case("global") {
                "global"
            } else {
                "tr"
            };
            if src_scope != scope {
                return false;
            }
            if !s.research_category.is_empty()
                && !s.research_category.eq_ignore_ascii_case("all")
                && !s.research_category.eq_ignore_ascii_case("gündem")
            {
                if !src.category.eq_ignore_ascii_case(&s.research_category) {
                    return false;
                }
            }
            true
        })
        .collect();

    let sources_bar_html = if !scoped_sources.is_empty() {
        let all_cls = if s.selected_research_source_id.is_none() {
            "hf-research-source-pill hf-research-source-pill-active"
        } else {
            "hf-research-source-pill hf-research-source-pill-idle"
        };
        let mut src_pills = format!(
            r#"<button type="button" data-action="select-research-source" data-id="0" class="{all_cls}">Tüm Kaynaklar</button>"#
        );
        for src in scoped_sources {
            let is_sel = s.selected_research_source_id == Some(src.id);
            let pill_cls = if is_sel {
                "hf-research-source-pill hf-research-source-pill-active"
            } else {
                "hf-research-source-pill hf-research-source-pill-idle"
            };
            let dot_cls = if src.kind == "rss" {
                "hf-source-kind-dot hf-source-kind-rss"
            } else {
                "hf-source-kind-dot hf-source-kind-site"
            };
            let cat_hint = if src.category.trim().is_empty() {
                "Gündem"
            } else {
                src.category.trim()
            };
            src_pills.push_str(&format!(
                r#"<button type="button" data-action="select-research-source" data-id="{id}" title="{ttl}" class="{pill_cls}">
                    <span class="{dot_cls}"></span>
                    <span>{name}</span>
                </button>"#,
                id = src.id,
                ttl = escape_html(&format!("{} ({})", src.name, cat_hint)),
                name = escape_html(&src.name),
            ));
        }
        format!(
            r#"<div class="hf-research-sources-row" role="tablist" aria-label="Kayıtlı Kaynak Filtresi">
                {src_pills}
            </div>"#
        )
    } else {
        String::new()
    };

    const TIME_OPTS: &[(&str, &str)] = &[
        ("all", "Tüm Zamanlar"),
        ("1h", "Son 1 Saat"),
        ("24h", "Son 24 Saat"),
        ("7d", "Son 7 Gün"),
    ];
    let cur_time = match s.research_time.as_str() {
        "1h" | "24h" | "7d" => s.research_time.as_str(),
        _ => "all",
    };
    let mut time_pills = String::new();
    for (val, label) in TIME_OPTS {
        let cls = if *val == cur_time {
            "hf-research-time-pill hf-research-time-active"
        } else {
            "hf-research-time-pill hf-research-time-idle"
        };
        time_pills.push_str(&format!(
            r#"<button type="button" data-action="set-research-time" data-time="{val}" class="{cls}">{label}</button>"#
        ));
    }

    const TOPICS: &[&str] = &[
        "Gündem",
        "Savunma ve Askeri",
        "Ekonomi",
        "Dünya",
        "Teknoloji",
        "Spor",
    ];
    let cat_low = turkish_lowercase(s.research_category.trim());
    let q_low = turkish_lowercase(s.research_query.trim());
    let mut topic_chips = String::new();
    for topic in TOPICS {
        let t_low = turkish_lowercase(topic);
        let is_active = if *topic == "Gündem" {
            (cat_low.is_empty() || cat_low == "gündem") && q_low.is_empty()
        } else {
            cat_low == t_low || q_low == t_low
        };
        let cls = if is_active {
            "hf-research-topic-chip hf-research-topic-active"
        } else {
            "hf-research-topic-chip hf-research-topic-idle"
        };
        topic_chips.push_str(&format!(
            r#"<button type="button" data-action="quick-research-topic" data-topic="{t_esc}" class="{cls}">{t_esc}</button>"#,
            t_esc = escape_html(topic),
        ));
    }

    format!(
        r#"<div class="hf-research-filters-wrap">
            <div class="hf-research-scope-bar" role="tablist" aria-label="Basın Kapsamı">
                <button type="button" data-action="set-research-scope" data-scope="tr" class="{tr_cls}">🇹🇷 Türkçe Basın</button>
                <button type="button" data-action="set-research-scope" data-scope="global" class="{gl_cls}">🌍 Dünya Basını</button>
            </div>
            {sources_bar_html}
            <div class="hf-research-time-row">
                {time_pills}
            </div>
            <div class="hf-research-topics-row">
                {topic_chips}
            </div>
        </div>"#
    )
}

pub fn render_research_results_html(s: &AppState) -> String {
    if s.is_searching_research {
        return format!(
            r#"<div class="hf-research-loading-box">
                <div class="hf-research-loading-head">
                    {spin_ico}
                    <span>Canlı haber kaynakları taranıyor...</span>
                </div>
                <div class="hf-research-skeleton"></div>
                <div class="hf-research-skeleton"></div>
                <div class="hf-research-skeleton"></div>
            </div>"#,
            spin_ico = icons::loader2("w-4 h-4 animate-spin text-brand-500"),
        );
    }

    if s.research_results.is_empty() {
        let msg = if s.research_query.trim().is_empty() {
            "Canlı gündem başlıkları henüz yüklenmedi veya sonuç bulunamadı."
        } else {
            "Aradığınız kriterlere uygun güncel kaynak haber bulunamadı."
        };
        return format!(
            r#"<div class="hf-research-empty-box">
                <div class="hf-research-empty-icon">{comp_ico}</div>
                <div class="hf-research-empty-title">{msg}</div>
                <button type="button" data-action="run-research-search" class="hf-research-retry-btn">Tekrar Ara</button>
            </div>"#,
            comp_ico = icons::compass("w-6 h-6 text-zinc-400"),
            msg = escape_html(msg),
        );
    }

    let selected_src_name = s
        .selected_research_source_id
        .and_then(|sid| s.research_sources.iter().find(|src| src.id == sid))
        .map(|src| src.name.trim())
        .filter(|n| !n.is_empty());

    let heading_label = if let Some(sname) = selected_src_name {
        format!("{sname} ({})", s.research_results.len())
    } else if s.research_query.trim().is_empty() {
        format!("Canlı Gündem ({})", s.research_results.len())
    } else {
        format!("Kaynak Sonuçları ({})", s.research_results.len())
    };

    let open_tabs_badge = if !s.research_tabs.is_empty() {
        format!(
            r#"<button type="button" data-action="enter-research-reader" class="hf-research-open-count-btn" title="Açık Araştırma Sekmelerini Göster">{}/6 Sekme Açık</button>"#,
            s.research_tabs.len()
        )
    } else {
        String::new()
    };

    let mut cards = String::new();
    for item in &s.research_results {
        let Some(clean_url) = safe_http_url(&item.url) else {
            continue;
        };
        let is_active = s.active_research_url.as_deref() == Some(clean_url.as_str());
        let is_open_in_tabs = s.research_tabs.iter().any(|t| t.url == clean_url);

        let card_state_cls = if is_active {
            "hf-research-card hf-research-card-active"
        } else {
            "hf-research-card"
        };

        let open_badge = if is_open_in_tabs {
            r#"<span class="hf-research-open-badge">AÇIK</span>"#
        } else {
            ""
        };

        let source_display = if item.source_name.trim().is_empty() {
            get_site_name(&clean_url)
        } else {
            item.source_name.trim().to_string()
        };

        let time_display = format_research_time(item.published_at.as_deref());
        let clean_title = strip_all_html_tags(&item.title);
        let display_title = if clean_title.trim().is_empty() {
            clean_url.as_str()
        } else {
            clean_title.trim()
        };

        let snippet_plain = item
            .snippet
            .as_deref()
            .map(|sn| strip_all_html_tags(sn))
            .unwrap_or_default();
        let snippet_trimmed = snippet_plain.trim();
        let snippet_html = if !snippet_trimmed.is_empty() {
            format!(
                r#"<p class="hf-research-card-snippet">{}</p>"#,
                escape_html(snippet_trimmed)
            )
        } else {
            String::new()
        };

        let img_opt = item
            .image_url
            .as_deref()
            .filter(|u| is_valid_image_url(Some(u)));
        let thumb_html = if let Some(img_u) = img_opt {
            format!(
                r#"<div class="hf-research-card-thumb">{}</div>"#,
                render_news_image(Some(img_u), display_title, "w-full h-full object-cover")
            )
        } else {
            String::new()
        };

        cards.push_str(&format!(
            r#"<div role="button" tabindex="0" data-action="open-research-item" data-url="{u_attr}" data-title="{t_attr}" data-source="{s_attr}" data-snippet="{sn_attr}" data-image="{img_attr}" class="{card_state_cls}">
                <div class="hf-research-card-meta">
                    <div class="hf-research-card-meta-left">
                        <span class="hf-research-source-badge">{src_esc}</span>
                        <span class="hf-research-time-text">{time_esc}</span>
                    </div>
                    {open_badge}
                </div>
                <div class="hf-research-card-main">
                    <div class="hf-research-card-copy">
                        <h3 class="hf-research-card-title">{title_esc}</h3>
                        {snippet_html}
                    </div>
                    {thumb_html}
                </div>
            </div>"#,
            u_attr = escape_html(&clean_url),
            t_attr = escape_html(display_title),
            s_attr = escape_html(&source_display),
            sn_attr = escape_html(snippet_trimmed),
            img_attr = escape_html(img_opt.unwrap_or("")),
            src_esc = escape_html(&source_display),
            time_esc = escape_html(&time_display),
            title_esc = escape_html(display_title),
        ));
    }

    format!(
        r#"<div class="hf-research-results-section">
            <div class="hf-research-results-header">
                <h2 class="hf-research-results-heading">{heading_esc}</h2>
                {open_tabs_badge}
            </div>
            <div class="hf-research-results-list">
                {cards}
            </div>
        </div>"#,
        heading_esc = escape_html(&heading_label),
    )
}

pub fn render_research_left_tab(s: &AppState) -> String {
    let search_dis = if s.is_searching_research {
        "disabled"
    } else {
        ""
    };
    let info_btn_cls = if s.show_research_info {
        "hf-research-info-btn hf-research-info-btn-active"
    } else {
        "hf-research-info-btn hf-research-info-btn-idle"
    };
    let info_expanded = if s.show_research_info {
        "true"
    } else {
        "false"
    };

    let clear_btn_cls = if s.research_query.trim().is_empty() {
        "hidden"
    } else {
        ""
    };

    if s.research_panel_collapsed {
        let active_cat_desc = if !s.research_category.is_empty() {
            format!(" • {}", escape_html(&s.research_category))
        } else if s.research_scope == "global" {
            " • Dünya Basını".to_string()
        } else {
            " • Gündem".to_string()
        };
        return format!(
            r#"<div class="hf-research-desk">
            <div class="hf-research-hero-box hf-research-hero-box-collapsed">
                <div class="hf-research-kicker">
                    <div class="hf-research-kicker-left">
                        {comp_ico}
                        <span>ARAŞTIRMA MASASI</span>
                        <span class="hf-research-collapsed-tag">{active_cat_desc}</span>
                    </div>
                    <div class="hf-research-kicker-actions">
                        <button type="button" data-action="toggle-research-collapse" class="hf-research-collapse-btn hf-research-collapse-btn-active" title="Filtreleri ve Arama Alanını Aç">
                            <span>▼ Filtreleri Aç</span>
                        </button>
                    </div>
                </div>
            </div>
            <div id="hf-research-results-slot">
                {results_html}
            </div>
        </div>"#,
            comp_ico = icons::compass("w-3.5 h-3.5 text-brand-500"),
            results_html = render_research_results_html(s),
        );
    }

    format!(
        r#"<div class="hf-research-desk">
            <div class="hf-research-hero-box">
                <div class="hf-research-kicker">
                    <div class="hf-research-kicker-left">
                        {comp_ico}
                        <span>ARAŞTIRMA MASASI</span>
                    </div>
                    <div class="hf-research-kicker-actions">
                        <button type="button" data-action="toggle-research-collapse" class="hf-research-collapse-btn" title="Filtreleri ve Arama Alanını Daralt (Haberlere Yer Aç)">
                            <span>▲ Daralt</span>
                        </button>
                        <button id="btn-research-info" type="button" data-action="toggle-research-info" aria-expanded="{info_expanded}" title="Araştırma Masası Özellikleri ve Kısayollar" class="{info_btn_cls}">
                            {info_ico}
                            <span>Özellikler &amp; İpuçları</span>
                        </button>
                        <span class="hf-research-live-pill"><span class="hf-live-dot"></span>CANLI</span>
                    </div>
                </div>
                <form id="hf-research-form" class="hf-research-search-row">
                    <div class="hf-research-input-wrap">
                        <span class="hf-research-input-icon">{search_ico}</span>
                        <input id="hf-research-input" data-input="research-query" type="text" value="{q_val}" placeholder="Konu ara, site:alanadi.com yaz veya haber linki yapıştır..." class="hf-research-input" autocomplete="off" />
                        <button id="btn-research-clear-query" type="button" data-action="clear-research-query" title="Aramayı Temizle" class="hf-research-input-clear {clear_btn_cls}">
                            {clear_ico}
                        </button>
                    </div>
                    <button type="button" data-action="run-research-search" {search_dis} class="hf-research-submit-btn">
                        <span>Ara</span>
                    </button>
                </form>
                <div id="hf-research-info-slot">
                    {info_html}
                </div>
                <div id="hf-research-filters-slot">
                    {filters_html}
                </div>
            </div>
            <div id="hf-research-results-slot">
                {results_html}
            </div>
        </div>"#,
        comp_ico = icons::compass("w-3.5 h-3.5 text-brand-500"),
        info_ico = icons::info("w-3.5 h-3.5"),
        search_ico = icons::search("w-3.5 h-3.5"),
        clear_ico = icons::x_mark("w-3 h-3"),
        q_val = escape_html(&s.research_query),
        info_html = render_research_info_drawer_html(s),
        filters_html = render_research_filters_html(s),
        results_html = render_research_results_html(s),
    )
}

// ============================================================================
// 2. Middle Column Clean Reader View & Multi-Source Fact-Check Tab Bar
// ============================================================================

/// Subtle floating pill shown in `render_middle.rs` when `middle_showing_research == false`
/// while `!s.research_tabs.is_empty()` so the editor never loses open research sources.
pub fn render_open_research_floating_pill(s: &AppState) -> String {
    if s.research_tabs.is_empty() {
        return String::new();
    }
    format!(
        r#"<button type="button" data-action="enter-research-reader" class="hf-research-floating-pill" title="Açık Araştırma Sekmelerine Dön">
            <span>🔍 Açık Araştırma ({n})</span>
        </button>"#,
        n = s.research_tabs.len()
    )
}

pub fn active_research_tab(s: &AppState) -> Option<&ResearchReaderTab> {
    if let Some(active_url) = s.active_research_url.as_deref() {
        if let Some(found) = s.research_tabs.iter().find(|t| t.url == active_url) {
            return Some(found);
        }
    }
    s.research_tabs.last()
}

fn describe_embed_platform(url: &str) -> &'static str {
    let low = url.to_ascii_lowercase();
    if low.contains("youtube.com") || low.contains("youtu.be") {
        "YouTube Videosu"
    } else if low.contains("x.com") || low.contains("twitter.com") {
        "X (Twitter) Paylaşımı"
    } else if low.contains("instagram.com") {
        "Instagram Gönderisi"
    } else if low.contains("tiktok.com") {
        "TikTok Videosu"
    } else {
        "Medya Bağlantısı"
    }
}

pub fn render_research_middle_reader(s: &AppState) -> String {
    let active_tab_opt = active_research_tab(s);
    let active_url_str = active_tab_opt.map(|t| t.url.as_str()).unwrap_or("");
    let total_tabs = s.research_tabs.len();
    let active_idx = s
        .research_tabs
        .iter()
        .position(|t| t.url == active_url_str)
        .unwrap_or(0);

    // 1. Tab Nav Stepper & Counter (< 2 / 5 >)
    let (prev_dis, next_dis) = if total_tabs <= 1 {
        ("disabled", "disabled")
    } else {
        ("", "")
    };
    let nav_stepper_html = if total_tabs > 0 {
        format!(
            r#"<div class="hf-reader-tab-stepper" title="Sekmeler Arasında Gezin (Alt+← / Alt+→)">
                <button type="button" data-action="prev-research-tab" {prev_dis} class="hf-reader-stepper-btn" title="Önceki Sekme (Alt+←)">
                    {prev_ico}
                </button>
                <button type="button" data-action="toggle-research-tabs-menu" class="hf-reader-stepper-badge" title="Tüm Sekmeleri Listele (Açık: {curr_num}/{total_tabs})">
                    <span>{curr_num}/{total_tabs}</span>
                    {chevron_down}
                </button>
                <button type="button" data-action="next-research-tab" {next_dis} class="hf-reader-stepper-btn" title="Sonraki Sekme (Alt+→)">
                    {next_ico}
                </button>
            </div>"#,
            curr_num = if total_tabs == 0 { 0 } else { active_idx + 1 },
            prev_ico = icons::chevron_left("w-3.5 h-3.5"),
            next_ico = icons::chevron_right("w-3.5 h-3.5"),
            chevron_down = icons::chevron_down("w-2.5 h-2.5 opacity-70"),
        )
    } else {
        String::new()
    };

    // 2. Tabs Dropdown Menu (Vertical quick list)
    let tabs_dropdown_html = if s.show_research_tabs_menu && total_tabs > 0 {
        let mut items = String::new();
        for (i, tab) in s.research_tabs.iter().enumerate() {
            let is_cur = tab.url == active_url_str;
            let row_cls = if is_cur {
                "hf-tabs-menu-item hf-tabs-menu-item-active"
            } else {
                "hf-tabs-menu-item"
            };
            let src = if tab.source_hint.trim().is_empty() {
                get_site_name(&tab.url)
            } else {
                tab.source_hint.trim().to_string()
            };
            let ttl = if tab.title_hint.trim().is_empty() {
                tab.url.clone()
            } else {
                strip_all_html_tags(&tab.title_hint)
            };
            items.push_str(&format!(
                r#"<div class="{row_cls}">
                    <button type="button" data-action="switch-research-tab" data-url="{u_esc}" class="hf-tabs-menu-btn">
                        <span class="hf-tabs-menu-num">{num}</span>
                        <div class="hf-tabs-menu-text">
                            <span class="hf-tabs-menu-src">{src_esc}</span>
                            <span class="hf-tabs-menu-ttl">{ttl_esc}</span>
                        </div>
                    </button>
                    <button type="button" data-action="close-research-tab" data-url="{u_esc}" class="hf-tabs-menu-close" title="Kapat">×</button>
                </div>"#,
                num = i + 1,
                u_esc = escape_html(&tab.url),
                src_esc = escape_html(&src),
                ttl_esc = escape_html(&ttl),
            ));
        }
        format!(
            r#"<div id="hf-research-tabs-dropdown" class="hf-research-tabs-dropdown-menu">
                <div class="hf-tabs-menu-header">
                    <span>AÇIK SEKMELER ({total_tabs})</span>
                    <button type="button" data-action="close-all-research-tabs" class="hf-tabs-menu-close-all">Tümünü Kapat</button>
                </div>
                <div class="hf-tabs-menu-list">
                    {items}
                </div>
            </div>"#
        )
    } else {
        String::new()
    };

    // 3. Tab Pills
    let mut tabs_html = String::new();
    for (idx, tab) in s.research_tabs.iter().enumerate() {
        let is_cur = tab.url == active_url_str;
        let item_cls = if is_cur {
            "hf-reader-tab-item hf-reader-tab-active"
        } else {
            "hf-reader-tab-item hf-reader-tab-idle"
        };
        let src_lbl = if tab.source_hint.trim().is_empty() {
            get_site_name(&tab.url)
        } else {
            truncate_chars(tab.source_hint.trim(), 16)
        };
        let ttl_lbl = if tab.title_hint.trim().is_empty() {
            truncate_chars(&tab.url, 28)
        } else {
            truncate_chars(&strip_all_html_tags(&tab.title_hint), 32)
        };
        let status_dot = if tab.is_loading {
            icons::loader2("w-3 h-3 animate-spin text-brand-500 shrink-0")
        } else if is_cur {
            r#"<span class="hf-reader-tab-dot hf-reader-tab-dot-active"></span>"#.to_string()
        } else {
            r#"<span class="hf-reader-tab-dot"></span>"#.to_string()
        };
        let shortcut_hint = if idx < 6 {
            format!("(Alt+{})", idx + 1)
        } else {
            String::new()
        };

        tabs_html.push_str(&format!(
            r#"<div class="{item_cls}" data-action="switch-research-tab" data-url="{u_esc}" title="{full_title} {shortcut_hint}" role="tab" aria-selected="{is_cur}">
                <span class="hf-reader-tab-num">{num}</span>
                {status_dot}
                <span class="hf-reader-tab-source">{src_esc}</span>
                <span class="hf-reader-tab-sep">•</span>
                <span class="hf-reader-tab-title">{ttl_esc}</span>
                <button type="button" data-action="close-research-tab" data-url="{u_esc}" class="hf-reader-tab-close" title="Sekmeyi Kapat" aria-label="Sekmeyi Kapat">×</button>
            </div>"#,
            num = idx + 1,
            u_esc = escape_html(&tab.url),
            full_title = escape_html(&format!("{} — {}", src_lbl, tab.title_hint)),
            src_esc = escape_html(&src_lbl),
            ttl_esc = escape_html(&ttl_lbl),
        ));
    }

    let close_all_btn = if total_tabs > 1 {
        r#"<button type="button" data-action="close-all-research-tabs" class="hf-reader-close-all-btn" title="Tüm Sekmeleri Kapat">Tümünü Kapat</button>"#.to_string()
    } else {
        String::new()
    };

    let back_to_article_btn = if s.selected_article.is_some() {
        format!(
            r#"<button type="button" data-action="exit-research-reader" class="hf-reader-exit-btn" title="Haber Düzenlemeye Dön (Alt+E)">
                {back_ico}
                <span>Haberime Dön</span>
            </button>"#,
            back_ico = icons::arrow_left("w-3.5 h-3.5"),
        )
    } else {
        String::new()
    };

    let mobile_back_btn = format!(
        r#"<button type="button" data-action="mobile-back-left" class="hf-reader-mobile-back lg:hidden" title="Araştırma Listesine Dön">
            {}
        </button>"#,
        icons::arrow_left("w-4 h-4")
    );

    let sheet_inner = match active_tab_opt {
        None => format!(
            r#"<div class="hf-reader-empty-state">
                <div class="hf-research-empty-icon">{comp_ico}</div>
                <div class="hf-reader-empty-heading">Açık Kaynak Sekmesi Yok</div>
                <p class="hf-reader-empty-sub">Sol taraftaki Araştırma Masası'ndan bir habere tıklayın veya doğrudan haber linki yapıştırarak reklamsız okuma modunda açın.</p>
            </div>"#,
            comp_ico = icons::compass("w-7 h-7 text-brand-500"),
        ),
        Some(tab) if tab.is_loading => {
            let ext_link = safe_http_url(&tab.url)
                .map(|u| {
                    format!(
                        r#"<a href="{href}" target="_blank" rel="noopener noreferrer" class="hf-reader-btn-external">
                            <span>Orijinal Sitede Aç</span>
                            {ext_ico}
                        </a>"#,
                        href = escape_html(&u),
                        ext_ico = icons::external_link("w-3.5 h-3.5"),
                    )
                })
                .unwrap_or_default();
            format!(
                r#"<div id="hf-research-reader-sheet" class="hf-research-reader-sheet">
                    <div class="hf-reader-loading-card">
                        <div class="hf-reader-loading-top">
                            {spin_ico}
                            <div>
                                <div class="hf-reader-loading-title">Kaynak haber reklamsız okuma modunda ayıklanıyor...</div>
                                <div class="hf-reader-loading-sub">{src} • {ttl}</div>
                            </div>
                        </div>
                        <div class="hf-research-skeleton"></div>
                        <div class="hf-research-skeleton"></div>
                        <div class="pt-2">{ext_link}</div>
                    </div>
                </div>"#,
                spin_ico = icons::loader2("w-6 h-6 animate-spin text-brand-500 shrink-0"),
                src = escape_html(&tab.source_hint),
                ttl = escape_html(&tab.title_hint),
            )
        }
        Some(tab) if tab.extracted.is_none() => {
            let err_msg = tab
                .error
                .as_deref()
                .unwrap_or("Kaynak haber içeriği alınamadı.");
            let ext_link = safe_http_url(&tab.url)
                .map(|u| {
                    format!(
                        r#"<a href="{href}" target="_blank" rel="noopener noreferrer" class="hf-reader-blocked-cta">
                            <span>Yeni Sekmede Orijinal Haberi Aç ↗</span>
                        </a>"#,
                        href = escape_html(&u),
                    )
                })
                .unwrap_or_default();
            format!(
                r#"<div id="hf-research-reader-sheet" class="hf-research-reader-sheet">
                    <div class="hf-reader-blocked-notice">
                        <div class="hf-reader-blocked-head">
                            {alert_ico}
                            <div>
                                <div class="hf-reader-blocked-title">Kaynak Sayfa Okunamadı</div>
                                <div class="hf-reader-blocked-desc">{err_esc}</div>
                            </div>
                        </div>
                        <div class="hf-reader-blocked-actions">
                            <button type="button" data-action="retry-research-tab" data-url="{u_esc}" class="hf-reader-retry-btn">Tekrar Dene</button>
                            {ext_link}
                        </div>
                    </div>
                </div>"#,
                alert_ico = icons::alert_circle("w-5 h-5 text-amber-600 shrink-0"),
                err_esc = escape_html(err_msg),
                u_esc = escape_html(&tab.url),
            )
        }
        Some(tab) => {
            let art = tab.extracted.as_ref().unwrap();
            let effective_url = safe_http_url(&art.canonical_url)
                .or_else(|| safe_http_url(&art.url))
                .or_else(|| safe_http_url(&tab.url))
                .unwrap_or_default();

            let site_badge = if !art.site_name.trim().is_empty() {
                art.site_name.trim().to_string()
            } else if !art.domain.trim().is_empty() {
                art.domain.trim().to_string()
            } else if !tab.source_hint.trim().is_empty() {
                tab.source_hint.trim().to_string()
            } else {
                get_site_name(&effective_url)
            };

            let domain_html = if !art.domain.trim().is_empty()
                && !art.domain.trim().eq_ignore_ascii_case(&site_badge)
            {
                format!(
                    r#"<span class="hf-reader-domain-label">{}</span>"#,
                    escape_html(art.domain.trim())
                )
            } else {
                String::new()
            };

            let author_html = art
                .author
                .as_deref()
                .map(|a| strip_all_html_tags(a))
                .filter(|a| !a.trim().is_empty())
                .map(|a| {
                    format!(
                        r#"<span class="hf-reader-meta-item">{ico} {author}</span>"#,
                        ico = icons::user("w-3 h-3 text-zinc-400"),
                        author = escape_html(a.trim()),
                    )
                })
                .unwrap_or_default();

            let date_html = art
                .published_at
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(|d| {
                    format!(
                        r#"<span class="hf-reader-meta-item">{}</span>"#,
                        escape_html(&format_research_time(Some(d)))
                    )
                })
                .unwrap_or_default();

            let word_badge = if art.word_count > 0 {
                format!(
                    r#"<span class="hf-reader-word-pill">{} kelime</span>"#,
                    art.word_count
                )
            } else {
                String::new()
            };

            let is_attached = !effective_url.is_empty()
                && s.editing_article.effective_source_url() == Some(effective_url.as_str());
            let attach_btn_html = if !effective_url.is_empty() {
                let (btn_cls, btn_txt) = if is_attached {
                    (
                        "hf-reader-btn-source hf-reader-btn-source-attached",
                        "✓ Kaynak Referansı Eklendi",
                    )
                } else {
                    (
                        "hf-reader-btn-source",
                        "📌 Kaynak Referansı Olarak Ekle",
                    )
                };
                format!(
                    r#"<button type="button" data-action="attach-research-source" data-url="{u_esc}" class="{btn_cls}">
                        <span>{btn_txt}</span>
                    </button>"#,
                    u_esc = escape_html(&effective_url),
                )
            } else {
                String::new()
            };

            let external_open_html = if !effective_url.is_empty() {
                format!(
                    r#"<a href="{u_esc}" target="_blank" rel="noopener noreferrer" class="hf-reader-btn-external">
                        <span>Orijinal Sitede Aç ↗</span>
                    </a>"#,
                    u_esc = escape_html(&effective_url),
                )
            } else {
                String::new()
            };

            let blocked_banner_html = if art.blocked_by_source {
                let cta_html = if !effective_url.is_empty() {
                    format!(
                        r#"<a href="{u_esc}" target="_blank" rel="noopener noreferrer" class="hf-reader-blocked-cta">
                            <span>Yeni Sekmede Orijinal Haberi Aç ↗</span>
                        </a>"#,
                        u_esc = escape_html(&effective_url),
                    )
                } else {
                    String::new()
                };
                format!(
                    r#"<div class="hf-reader-blocked-notice">
                        <div class="hf-reader-blocked-head">
                            {alert_ico}
                            <div>
                                <div class="hf-reader-blocked-title">Yayıncı Tam Metin Erişimini Kısıtlıyor</div>
                                <div class="hf-reader-blocked-desc">{site_esc} bot koruması nedeniyle tam metin erişimini sınırlandırıyor. Mevcut özet aşağıda gösterilmektedir; tüm ayrıntılar için orijinal sayfayı yeni sekmede açabilirsiniz.</div>
                            </div>
                        </div>
                        <div class="hf-reader-blocked-actions">
                            {cta_html}
                        </div>
                    </div>"#,
                    alert_ico = icons::alert_circle("w-5 h-5 text-amber-600 shrink-0"),
                    site_esc = escape_html(&site_badge),
                )
            } else {
                String::new()
            };

            let key_facts_html = if !art.key_facts.is_empty() {
                let mut fact_lis = String::new();
                for fact in &art.key_facts {
                    let clean_f = strip_all_html_tags(fact);
                    let f_trim = clean_f.trim();
                    if !f_trim.is_empty() {
                        fact_lis.push_str(&format!(
                            r#"<li class="hf-research-fact-item"><span class="hf-research-fact-bullet"></span><span>{}</span></li>"#,
                            escape_html(f_trim)
                        ));
                    }
                }
                if fact_lis.is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<section class="hf-research-facts-card">
                            <div class="hf-research-facts-head">
                                <div class="hf-research-facts-title">📊 Önemli Rakamlar &amp; Veriler</div>
                                <button type="button" data-action="insert-research-facts" class="hf-research-facts-insert-btn">
                                    <span>+ Bilgi Kutusu Olarak Sağ Editöre Ekle</span>
                                </button>
                            </div>
                            <ul class="hf-research-facts-list">
                                {fact_lis}
                            </ul>
                        </section>"#
                    )
                }
            } else {
                String::new()
            };

            let embeds_bar_html = if !art.media_embeds.is_empty() {
                let mut embed_btns = String::new();
                for raw_emb in &art.media_embeds {
                    if let Some(safe_emb) = safe_http_url(raw_emb) {
                        let plat = describe_embed_platform(&safe_emb);
                        embed_btns.push_str(&format!(
                            r#"<div class="hf-research-embed-row">
                                <div class="hf-research-embed-info">
                                    {play_ico}
                                    <span class="hf-research-embed-plat">{plat}</span>
                                    <span class="hf-research-embed-url">{u_short}</span>
                                </div>
                                <button type="button" data-action="insert-research-embed" data-embed-url="{u_esc}" class="hf-research-embed-insert-btn">
                                    <span>+ Bu Videoyu / Tweeti Editöre Ekle</span>
                                </button>
                            </div>"#,
                            play_ico = icons::play_circle("w-4 h-4 text-rose-500 shrink-0"),
                            u_short = escape_html(&truncate_chars(&safe_emb, 48)),
                            u_esc = escape_html(&safe_emb),
                        ));
                    }
                }
                if embed_btns.is_empty() {
                    String::new()
                } else {
                    format!(r#"<div class="hf-research-embeds-bar">{embed_btns}</div>"#)
                }
            } else {
                String::new()
            };

            let clean_title = strip_all_html_tags(&art.title);
            let h1_title = if clean_title.trim().is_empty() {
                strip_all_html_tags(&tab.title_hint)
            } else {
                clean_title
            };

            let lead_img_html = art
                .lead_image
                .as_deref()
                .filter(|u| is_valid_image_url(Some(u)))
                .map(|u| {
                    format!(
                        r#"<div class="hf-reader-lead-figure">{}</div>"#,
                        render_news_image(Some(u), h1_title.trim(), "w-full h-full object-cover")
                    )
                })
                .unwrap_or_default();

            let subtitle_html = art
                .subtitle
                .as_deref()
                .map(|sub| strip_all_html_tags(sub))
                .filter(|sub| !sub.trim().is_empty())
                .map(|sub| {
                    format!(
                        r#"<p class="hf-reader-subtitle">{}</p>"#,
                        escape_html(sub.trim())
                    )
                })
                .unwrap_or_default();

            let blocks_html = if !art.blocks.is_empty() {
                let cleaned_blocks: Vec<crate::editorial::common::ContentBlock> = art
                    .blocks
                    .iter()
                    .map(|b| {
                        let mut cb = b.clone();
                        if !matches!(b.block_type.as_str(), "image" | "embed" | "gallery" | "related_article" | "code" | "divider") {
                            cb.content = crate::editorial::wasm::state::sanitize_prose_html(&b.content);
                        }
                        cb
                    })
                    .collect();
                render_preview_blocks_html(&cleaned_blocks)
            } else if !art.blocked_by_source {
                r#"<p class="text-zinc-400 italic">Bu kaynaktan metin paragrafı ayıklanamadı.</p>"#
                    .to_string()
            } else {
                String::new()
            };

            format!(
                r#"<article id="hf-research-reader-sheet" class="hf-research-reader-sheet">
                    <header class="hf-reader-header-bar">
                        <div class="hf-reader-meta-cluster">
                            <span class="hf-reader-source-pill">{site_esc}</span>
                            {domain_html}
                            {author_html}
                            {date_html}
                            {word_badge}
                        </div>
                        <div class="hf-reader-actions-row">
                            {attach_btn_html}
                            {external_open_html}
                        </div>
                    </header>

                    {blocked_banner_html}
                    {key_facts_html}
                    {embeds_bar_html}

                    <div id="hf-research-reader-prose" class="prose hf-rich-prose hf-research-reader-prose max-w-none">
                        {lead_img_html}
                        <h1 class="hf-reader-h1">{h1_esc}</h1>
                        {subtitle_html}
                        <div class="hf-reader-blocks-flow">
                            {blocks_html}
                        </div>
                    </div>

                    <footer class="hf-reader-quote-tip">
                        <span>💡 İpucu: Metin içindeki herhangi bir cümleyi fareyle seçerek tek tıkla <strong>Alıntı</strong> veya <strong>Bilgi Notu</strong> olarak sağ editöre aktarabilirsiniz.</span>
                    </footer>
                </article>"#,
                site_esc = escape_html(&site_badge),
                h1_esc = escape_html(h1_title.trim()),
            )
        }
    };

    format!(
        r#"<div class="hf-research-tabs-bar">
            <div class="hf-research-tabs-left">
                {mobile_back_btn}
                {nav_stepper_html}
                <div class="hf-research-tabs-scroll" role="tablist" aria-label="Açık Araştırma Sekmeleri">
                    {tabs_html}
                </div>
            </div>
            <div class="hf-research-tabs-right">
                {close_all_btn}
                {back_to_article_btn}
            </div>
            {tabs_dropdown_html}
        </div>
        <div id="middle-scroll" class="flex-1 overflow-y-auto custom-scrollbar relative w-full hf-research-reader-stage">
            {sheet_inner}
        </div>"#
    )
}

// ============================================================================
// 3. Floating Selection-to-Quote Bridge (`#hf-reader-quote-bubble`)
// ============================================================================

/// Persistent floating quote-bridge toolbar mounted once inside `#root`.
pub fn render_reader_quote_bubble_shell() -> String {
    r#"<div id="hf-reader-quote-bubble" role="toolbar" aria-label="Araştırma Alıntı Köprüsü" class="hf-quote-bridge-bubble opacity-0 pointer-events-none">
        <button type="button" data-action="send-reader-selection-quote" class="hf-quote-bridge-btn hf-quote-bridge-btn-primary">
            <span>❝ Alıntı Olarak Editöre Ekle</span>
        </button>
        <button type="button" data-action="send-reader-selection-callout" class="hf-quote-bridge-btn hf-quote-bridge-btn-secondary">
            <span>+ Bilgi Notu Ekle</span>
        </button>
        <div class="hf-quote-bridge-arrow"></div>
    </div>"#
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn hide_reader_quote_bubble() {
    READER_SELECTED_TEXT.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn update_reader_quote_bubble_from_dom() {}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_reader_quote_bubble_mousedown(_el: &Element, _ev: &MouseEvent) -> bool {
    false
}

#[cfg(not(target_arch = "wasm32"))]
pub fn take_reader_selected_text_and_clear() -> Option<String> {
    READER_SELECTED_TEXT.with(|cell| cell.borrow_mut().take())
}

#[cfg(target_arch = "wasm32")]
fn doc() -> Option<Document> {
    web_sys::window()?.document()
}

#[cfg(target_arch = "wasm32")]
pub fn hide_reader_quote_bubble() {
    READER_SELECTED_TEXT.with(|cell| {
        *cell.borrow_mut() = None;
    });
    let Some(document) = doc() else { return };
    if let Some(bubble) = document.get_element_by_id("hf-reader-quote-bubble") {
        let _ = bubble.class_list().remove_1("hf-bubble-visible");
        let _ = bubble.class_list().remove_1("hf-bubble-below");
        let _ = bubble.class_list().add_2("opacity-0", "pointer-events-none");
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_reader_quote_bubble_mousedown(el: &Element, ev: &MouseEvent) -> bool {
    if el
        .closest("#hf-reader-quote-bubble")
        .ok()
        .flatten()
        .is_some()
    {
        // Prevent mouse press from collapsing text selection inside `#hf-research-reader-prose`
        ev.prevent_default();
        return true;
    }
    false
}

#[cfg(target_arch = "wasm32")]
pub fn update_reader_quote_bubble_from_dom() {
    let Some(win) = web_sys::window() else { return };
    let Some(document) = win.document() else {
        return;
    };
    let Some(prose_el) = document.get_element_by_id("hf-research-reader-prose") else {
        hide_reader_quote_bubble();
        return;
    };
    let Ok(Some(sel)) = win.get_selection() else {
        hide_reader_quote_bubble();
        return;
    };
    if sel.is_collapsed() || sel.range_count() == 0 {
        hide_reader_quote_bubble();
        return;
    }
    let Ok(range) = sel.get_range_at(0) else {
        hide_reader_quote_bubble();
        return;
    };
    let Ok(common_ancestor) = range.common_ancestor_container() else {
        hide_reader_quote_bubble();
        return;
    };
    if !prose_el.contains(Some(&common_ancestor)) {
        hide_reader_quote_bubble();
        return;
    }

    let raw_sel = sel.to_string().as_string().unwrap_or_default();
    let cleaned = raw_sel
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    if cleaned.chars().count() < 8 {
        hide_reader_quote_bubble();
        return;
    }

    let rect = range.get_bounding_client_rect();
    if rect.width() <= 0.0 && rect.height() <= 0.0 {
        hide_reader_quote_bubble();
        return;
    }

    READER_SELECTED_TEXT.with(|cell| {
        *cell.borrow_mut() = Some(cleaned);
    });

    let Some(bubble) = document
        .get_element_by_id("hf-reader-quote-bubble")
        .and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };

    let measured_w = bubble.get_bounding_client_rect().width();
    let bubble_w = if measured_w > 60.0 { measured_w } else { 340.0 };
    let win_w = win
        .inner_width()
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(1280.0);

    let center_x = rect.left() + rect.width() / 2.0;
    let max_left = (win_w - bubble_w - 12.0).max(12.0);
    let left = (center_x - bubble_w / 2.0).clamp(12.0, max_left);

    let place_below = rect.top() < 68.0;
    let top = if place_below {
        rect.bottom() + 10.0
    } else {
        (rect.top() - 48.0).max(8.0)
    };

    let _ = bubble.style().set_property("left", &format!("{left:.1}px"));
    let _ = bubble.style().set_property("top", &format!("{top:.1}px"));

    if place_below {
        let _ = bubble.class_list().add_1("hf-bubble-below");
    } else {
        let _ = bubble.class_list().remove_1("hf-bubble-below");
    }
    let _ = bubble
        .class_list()
        .remove_2("opacity-0", "pointer-events-none");
    let _ = bubble.class_list().add_1("hf-bubble-visible");
}

#[cfg(target_arch = "wasm32")]
pub fn take_reader_selected_text_and_clear() -> Option<String> {
    let cached = READER_SELECTED_TEXT.with(|cell| cell.borrow_mut().take());
    let fallback = web_sys::window()
        .and_then(|w| w.get_selection().ok().flatten())
        .and_then(|sel| {
            let txt = sel.to_string().as_string().unwrap_or_default();
            let _ = sel.remove_all_ranges();
            let clean = txt
                .lines()
                .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            (clean.chars().count() >= 8).then_some(clean)
        });
    hide_reader_quote_bubble();
    cached.or(fallback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editorial::wasm::state::LeftTab;
    use crate::editorial::common::{ContentBlock, ExtractedReaderArticle, ResearchSearchItem};

    #[test]
    fn renders_left_research_desk_filters_and_result_cards_safely() {
        let mut s = AppState::new();
        s.left_tab = LeftTab::Research;
        s.research_query = "enflasyon".to_string();
        s.research_scope = "tr".to_string();
        s.research_time = "24h".to_string();
        s.research_results = vec![ResearchSearchItem {
            title: "Merkez Bankası'ndan Kritik Enflasyon Açıklaması <script>alert(1)</script>"
                .to_string(),
            url: "https://www.ekonomim.com/gundem/enflasyon-haberi-123".to_string(),
            source_name: "Ekonomim".to_string(),
            snippet: Some(
                "Yıl sonu beklentileri güncellendi ve <b>yeni</b> tahmin paylaşıldı.".to_string(),
            ),
            image_url: Some("https://cdn.ekonomim.com/kapak.jpg".to_string()),
            published_at: Some("15 dk önce".to_string()),
        }];
        s.active_research_url =
            Some("https://www.ekonomim.com/gundem/enflasyon-haberi-123".to_string());
        s.research_tabs.push(ResearchReaderTab {
            url: "https://www.ekonomim.com/gundem/enflasyon-haberi-123".to_string(),
            title_hint: "Merkez Bankası".to_string(),
            source_hint: "Ekonomim".to_string(),
            is_loading: false,
            error: None,
            extracted: None,
        });

        let html = render_research_left_tab(&s);
        assert!(html.contains("id=\"hf-research-input\""));
        assert!(html.contains("id=\"btn-research-info\""));
        assert!(html.contains("id=\"btn-research-clear-query\""));
        assert!(html.contains("data-action=\"clear-research-query\""));
        assert!(html.contains("data-action=\"toggle-research-info\""));
        assert!(html.contains("data-action=\"run-research-search\""));
        assert!(html.contains("🇹🇷 Türkçe Basın"));
        assert!(html.contains("🌍 Dünya Basını"));
        assert!(html.contains("Tüm Zamanlar"));
        assert!(html.contains("Son 24 Saat"));
        assert!(html.contains("data-action=\"quick-research-topic\""));
        assert!(html.contains("Savunma ve Askeri"));
        assert!(html.contains("data-action=\"open-research-item\""));
        assert!(html.contains("Ekonomim"));
        assert!(html.contains("AÇIK"));
        assert!(html.contains("hf-research-card-active"));
        assert!(!html.contains("<script>"));
        // Ensure no redundant duplicate category row or top-level desk close X
        assert!(!html.contains("data-action=\"close-research-desk\""));
        assert!(!html.contains("hf-research-cats-row"));

        // Info drawer & custom sources filter pills test
        s.user = Some(crate::editorial::common::User {
            id: 1,
            username: "admin".to_string(),
            role: "admin".to_string(),
            ..Default::default()
        });
        s.show_research_info = true;
        s.research_sources = vec![
            crate::editorial::common::ResearchSource {
                id: 10,
                name: "Anadolu Ajansı".to_string(),
                url: "https://www.aa.com.tr/tr/rss/default?cat=guncel".to_string(),
                kind: "rss".to_string(),
                category: "Ajans".to_string(),
                scope: "tr".to_string(),
                enabled: true,
                created_at: None,
            },
            crate::editorial::common::ResearchSource {
                id: 11,
                name: "Reuters World".to_string(),
                url: "https://www.reuters.com".to_string(),
                kind: "site".to_string(),
                category: "Dünya".to_string(),
                scope: "global".to_string(),
                enabled: true,
                created_at: None,
            },
        ];
        s.selected_research_source_id = Some(10);
        let html_with_info = render_research_left_tab(&s);
        assert!(html_with_info.contains("Araştırma Masası Kılavuzu"));
        assert!(html_with_info.contains("site:aa.com.tr"));
        assert!(html_with_info.contains("site:ekonomim.com"));
        assert!(html_with_info.contains("data-action=\"apply-research-example\""));
        assert!(!html_with_info.contains("+ Kaynak Ekle"));
        assert!(html_with_info.contains("Tüm Kaynaklar"));
        assert!(html_with_info.contains("Anadolu Ajansı"));
        assert!(!html_with_info.contains(">Reuters World<")); // global source hidden in tr scope!
    }

    #[test]
    fn renders_middle_clean_reader_multisource_tabs_key_facts_embeds_and_blocks_without_xss() {
        let mut s = AppState::new();
        s.middle_showing_research = true;
        s.selected_article = Some(crate::editorial::common::Article {
            id: "pub-1".to_string(),
            title: "Mevcut Haber".to_string(),
            ..Default::default()
        });

        let extracted = ExtractedReaderArticle {
            url: "https://www.bbc.com/news/world-101".to_string(),
            canonical_url: "https://www.bbc.com/news/world-101".to_string(),
            domain: "bbc.com".to_string(),
            site_name: "BBC News".to_string(),
            title: "Global Energy Summit Reaches Landmark Accord".to_string(),
            subtitle: Some("Delegates from 40 countries agreed on grid modernization.".to_string()),
            author: Some("Jane Doe".to_string()),
            published_at: Some("2 saat önce".to_string()),
            lead_image: Some("https://ichef.bbci.co.uk/images/summit.jpg".to_string()),
            blocks: vec![
                ContentBlock {
                    id: "rb-1".to_string(),
                    block_type: "paragraph".to_string(),
                    content: "Investment pledges reached <strong>$120 billion</strong> globally.<script>evil()</script>".to_string(),
                    meta: None,
                },
                ContentBlock {
                    id: "rb-2".to_string(),
                    block_type: "heading".to_string(),
                    content: "Key Regional Targets".to_string(),
                    meta: None,
                },
            ],
            key_facts: vec![
                "Toplam yatırım taahhüdü 120 milyar dolar".to_string(),
                "40 ülke ortak bildiriye imza attı".to_string(),
            ],
            media_embeds: vec![
                "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
                "javascript:alert(99)".to_string(),
            ],
            word_count: 342,
            blocked_by_source: false,
        };

        s.research_tabs = vec![
            ResearchReaderTab {
                url: "https://www.ekonomim.com/tarim-1".to_string(),
                title_hint: "TÜİK tarım verileri açıklandı".to_string(),
                source_hint: "Ekonomim".to_string(),
                is_loading: false,
                error: None,
                extracted: None,
            },
            ResearchReaderTab {
                url: "https://www.bbc.com/news/world-101".to_string(),
                title_hint: "Global Energy Summit".to_string(),
                source_hint: "BBC News".to_string(),
                is_loading: false,
                error: None,
                extracted: Some(extracted),
            },
        ];
        s.active_research_url = Some("https://www.bbc.com/news/world-101".to_string());

        let html = render_research_middle_reader(&s);
        // Multi-source tabs & return button
        assert!(html.contains("hf-research-tabs-bar"));
        assert!(html.contains("data-action=\"switch-research-tab\""));
        assert!(html.contains("data-action=\"close-research-tab\""));
        assert!(html.contains("Ekonomim"));
        assert!(html.contains("BBC News"));
        assert!(html.contains("Haberime Dön"));
        assert!(html.contains("data-action=\"exit-research-reader\""));

        // Reader header & editorial actions
        assert!(html.contains("id=\"hf-research-reader-sheet\""));
        assert!(html.contains("342 kelime"));
        assert!(html.contains("Jane Doe"));
        assert!(html.contains("data-action=\"attach-research-source\""));
        assert!(html.contains("Kaynak Referansı Olarak Ekle"));
        assert!(html.contains("Orijinal Sitede Aç ↗"));

        // Key facts & embed actions
        assert!(html.contains("📊 Önemli Rakamlar &amp; Veriler"));
        assert!(html.contains("Toplam yatırım taahhüdü 120 milyar dolar"));
        assert!(html.contains("data-action=\"insert-research-facts\""));
        assert!(html.contains("data-action=\"insert-research-embed\""));
        assert!(html.contains("https://www.youtube.com/watch?v=dQw4w9WgXcQ"));
        assert!(!html.contains("javascript:alert"));

        // Clean body prose & XSS stripped
        assert!(html.contains("id=\"hf-research-reader-prose\""));
        assert!(html.contains("Global Energy Summit Reaches Landmark Accord"));
        assert!(html.contains("Delegates from 40 countries agreed on grid modernization."));
        assert!(html.contains("<strong>$120 billion</strong>"));
        assert!(html.contains("Key Regional Targets"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("evil()"));

        // Blocked-by-source fallback notice test
        s.research_tabs[1]
            .extracted
            .as_mut()
            .unwrap()
            .blocked_by_source = true;
        let blocked_html = render_research_middle_reader(&s);
        assert!(blocked_html.contains("Yayıncı Tam Metin Erişimini Kısıtlıyor"));
        assert!(blocked_html.contains("Yeni Sekmede Orijinal Haberi Aç ↗"));
    }

    #[test]
    fn replaces_initial_empty_block_then_appends_subsequent_research_quotes() {
        let mut s = AppState::new();
        assert_eq!(s.editing_article.blocks.as_ref().unwrap().len(), 1);
        assert_eq!(s.editing_article.blocks.as_ref().unwrap()[0].id, "b-init-1");

        s.append_or_replace_initial_block(
            "quote",
            "Enflasyon yıl sonunda tek haneye yaklaşacak.".to_string(),
            Some("Ekonomim".to_string()),
        );
        let blocks = s.editing_article.blocks.as_ref().unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "quote");
        assert_eq!(blocks[0].meta.as_deref(), Some("Ekonomim"));

        s.append_or_replace_initial_block(
            "callout",
            "Toplam ihracat %8,4 arttı.".to_string(),
            Some("info|Ekonomim".to_string()),
        );
        let blocks2 = s.editing_article.blocks.as_ref().unwrap();
        assert_eq!(blocks2.len(), 2);
        assert_eq!(blocks2[1].block_type, "callout");
        assert_eq!(blocks2[1].meta.as_deref(), Some("info|Ekonomim"));
    }
}
