use super::components::{
    empty_state, localized, page_heading, ssb_tag_display_label, CANONICAL_SSB_TAG_CODES,
};
use super::config::safe_url;
use super::html_dsl::HtmlBuf;
use super::labels_helper::{
    default_institution_choices, lang_href, tr_text, InstitutionChoice,
};
use crate::{
    engine::{security_engine::escape_html as esc, translation_engine::TranslationCache},
    extractors::operator_user::is_non_staff_role_name,
    models::*,
};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Timelike};
use serde_json::{Map, Value};

const THEME_PREHYDRATE_JS: &str =
    crate::engine::design::theme_boot::UNIFIED_THEME_PREHYDRATE_JS;
const VALID_PORTAL_TABS: &[&str] = &["overview", "edergi", "profile", "security", "notifications"];

fn normalize_portal_tab(raw: Option<&str>, payment_success: bool) -> &'static str {
    if let Some(tab) = raw.map(str::trim).filter(|t| !t.is_empty()) {
        for &allowed in VALID_PORTAL_TABS {
            if tab.eq_ignore_ascii_case(allowed) {
                return allowed;
            }
        }
    }
    if payment_success {
        "edergi"
    } else {
        "overview"
    }
}

fn can_access_editorial_studio(role: &str) -> bool {
    matches!(
        role.trim().to_ascii_lowercase().as_str(),
        "super_admin"
            | "admin"
            | "site_operator"
            | "editor_in_chief"
            | "managing_editor"
            | "content_publisher"
            | "content_editor"
            | "editor"
    )
}

fn localize_subscription_tier(raw: &str, is_en: bool) -> String {
    let trimmed = raw.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "free" | "ucretsiz" | "ücretsiz" | "ücretsiz okuma" | "standart" | "kayitli"
        | "kayıtlı" | "kayıtlı okuyucu" | "registered" | "registered reader" => {
            if is_en { "Free Reader" } else { "Ücretsiz Okuyucu" }.to_string()
        }
        "dijital" | "digital" | "dijital abone" | "dijital e-dergi abonesi"
        | "digital subscriber" | "digital e-magazine subscriber" | "subscriber_digital" => {
            if is_en {
                "Digital E-Magazine Subscriber"
            } else {
                "Dijital E-Dergi Abonesi"
            }
            .to_string()
        }
        "basili-dijital" | "basılı + dijital" | "basılı + dijital abone" | "basili + dijital"
        | "print_digital" | "print-digital" | "print & digital"
        | "print & digital subscriber" | "subscriber_print_digital" => {
            if is_en {
                "Print & Digital Subscriber"
            } else {
                "Basılı + Dijital Abone"
            }
            .to_string()
        }
        "kurumsal" | "kurumsal abone" | "corporate" | "corporate subscriber"
        | "subscriber_corporate" => {
            if is_en {
                "Corporate Subscriber"
            } else {
                "Kurumsal Abone"
            }
            .to_string()
        }
        "kamu" | "kamu e-dergi abonesi" | "public" | "public sector subscriber"
        | "subscriber_public" => {
            if is_en {
                "Public Sector Subscriber"
            } else {
                "Kamu E-Dergi Abonesi"
            }
            .to_string()
        }
        _ => trimmed.to_string(),
    }
}

fn localize_payment_status(raw: &str, is_en: bool) -> (&'static str, &'static str) {
    let norm = raw.trim().to_ascii_lowercase();
    if norm.is_empty() {
        return ("is-muted", if is_en { "Unknown" } else { "Bilinmiyor" });
    }
    if norm.contains("refund") {
        return ("is-muted", if is_en { "Refunded" } else { "İade Edildi" });
    }
    if norm.contains("failed") {
        return ("is-danger", if is_en { "Failed" } else { "Başarısız" });
    }
    if norm.contains("cancel") {
        return ("is-muted", if is_en { "Cancelled" } else { "İptal Edildi" });
    }
    if norm == "paid" || norm.starts_with("paid_") {
        return ("is-verified", if is_en { "Paid" } else { "Ödendi" });
    }
    match norm.as_str() {
        "pending" | "pending_payment" => {
            ("is-pending", if is_en { "Pending" } else { "Beklemede" })
        }
        "pending_bank_transfer" => (
            "is-pending",
            if is_en {
                "Awaiting Bank Transfer"
            } else {
                "Havale Bekleniyor"
            },
        ),
        _ => ("is-muted", if is_en { "Processing" } else { "İşlemde" }),
    }
}

fn localize_consent_type(raw: &str, is_en: bool) -> String {
    let trimmed = raw.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "kvkk" | "" => if is_en {
            "KVKK Privacy Notice"
        } else {
            "KVKK Aydınlatma Metni"
        }
        .to_string(),
        "tos" => if is_en {
            "Terms of Service"
        } else {
            "Kullanım Koşulları"
        }
        .to_string(),
        "etk" => if is_en {
            "Commercial Message Consent (ETK)"
        } else {
            "Ticari İleti İzni (ETK)"
        }
        .to_string(),
        _ => trimmed.to_string(),
    }
}

fn format_portal_date(raw: &str, is_en: bool) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "—".to_string();
    }
    let format_dt = |ndt: NaiveDateTime, has_time: bool| -> String {
        let include_time = has_time && (ndt.hour() != 0 || ndt.minute() != 0);
        if is_en {
            if include_time {
                ndt.format("%d %b %Y %H:%M").to_string()
            } else {
                ndt.format("%d %b %Y").to_string()
            }
        } else if include_time {
            ndt.format("%d.%m.%Y %H:%M").to_string()
        } else {
            ndt.format("%d.%m.%Y").to_string()
        }
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return format_dt(dt.naive_local(), true);
    }
    for fmt in ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return format_dt(ndt, true);
        }
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return if is_en {
            d.format("%d %b %Y").to_string()
        } else {
            d.format("%d.%m.%Y").to_string()
        };
    }
    if let Some(prefix) = trimmed.get(..10) {
        if let Ok(d) = NaiveDate::parse_from_str(prefix, "%Y-%m-%d") {
            return if is_en {
                d.format("%d %b %Y").to_string()
            } else {
                d.format("%d.%m.%Y").to_string()
            };
        }
    }
    trimmed.to_string()
}

fn resolve_org_category_label(raw: &str, choices: &[InstitutionChoice], is_en: bool) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(found) = choices
        .iter()
        .find(|inst| inst.code.eq_ignore_ascii_case(trimmed))
    {
        if is_en && !found.label_en.trim().is_empty() {
            return found.label_en.clone();
        }
        if !found.label_tr.trim().is_empty() {
            return found.label_tr.clone();
        }
    }
    trimmed.to_string()
}

#[derive(Clone, Debug, Default)]
pub struct PortalQueryHints<'a> {
    pub payment_success: bool,
    pub payment_oid: Option<&'a str>,
    pub verify_email_prompt: bool,
    pub next_url: Option<&'a str>,
    pub active_tab: Option<&'a str>,
}

pub struct AccountPortalContext<'a> {
    pub lang: &'a str,
    pub user: &'a UserAccount,
    pub reminders: &'a [RenewalReminderLog],
    pub invoices: &'a [OrderInvoice],
    pub notifications: &'a [SystemNotification],
    pub tags: &'a [SsbInterestTag],
    pub admin_path_opt: Option<&'a str>,
    pub tr: Option<&'a TranslationCache>,
    pub institutions: &'a [InstitutionChoice],
    pub kvkk_logs: &'a [KvkkConsentLog],
    pub contents: &'a [ContentItem],
    pub issues: &'a [MagazineIssue],
    pub corporate_requests: &'a [CorporateVerificationRequest],
    pub purchased_codes: &'a [String],
    pub can_download_edergi: bool,
    pub has_subscriber_access: bool,
    pub can_access_design_studio: bool,
    pub hints: PortalQueryHints<'a>,
    pub csp_nonce: Option<&'a str>,
}

pub fn account(user: Option<&UserAccount>, invoices: &[OrderInvoice], lang: &str) -> String {
    account_with_admin_path(user, invoices, lang, None)
}

pub fn account_with_admin_path(
    user: Option<&UserAccount>,
    invoices: &[OrderInvoice],
    lang: &str,
    admin_path_opt: Option<&str>,
) -> String {
    account_with_context(user, invoices, lang, admin_path_opt, None)
}

pub fn account_with_context(
    user: Option<&UserAccount>,
    invoices: &[OrderInvoice],
    lang: &str,
    admin_path_opt: Option<&str>,
    tr: Option<&TranslationCache>,
) -> String {
    let Some(u) = user else {
        let eyebrow = tr_text(
            tr,
            lang,
            "account.eyebrow",
            "MSI OKUYUCU MERKEZİ",
            "MSI READER CENTER",
        );
        let heading = tr_text(
            tr,
            lang,
            "account.guest_heading",
            "Stratejik savunma arşivi.<br>Kişisel okuma merkeziniz.",
            "Strategic defence archive.<br>Your personal reading command.",
        );
        let desc = tr_text(
            tr,
            lang,
            "account.description",
            "Dijital E-Dergi sayılarına erişmek, SSB teknoloji ilgi alanlarınızı yönetmek ve kurumsal fatura kayıtlarınızı görüntülemek için giriş yapın.",
            "Sign in with your corporate or personal account to access digital E-Magazine issues, manage SSB technology alerts, and view invoices.",
        );
        let login_cta = tr_text(
            tr,
            lang,
            "account.login_cta",
            "Giriş yap / Hesap oluştur →",
            "Sign in / Create account →",
        );
        let sub_link = tr_text(
            tr,
            lang,
            "account.subscription_link",
            "Abonelik seçeneklerini incele →",
            "Explore subscriptions →",
        );
        let sub_href = lang_href("/abonelik-ve-satis", lang);
        return format!(
            r##"<section class="container section-space"><div class="account-welcome"><div class="eyebrow">{eyebrow}</div><h1>{heading}</h1><p>{desc}</p><div class="account-welcome-actions"><a href="#modal-login" class="button primary">{login_cta}</a><a href="{sub_href}" class="text-link">{sub_link}</a></div></div></section>"##,
            eyebrow = esc(&eyebrow),
            heading = heading,
            desc = esc(&desc),
            login_cta = esc(&login_cta),
            sub_href = esc(&sub_href),
            sub_link = esc(&sub_link),
        );
    };
    render_with_context(lang, u, &[], invoices, &[], &[], admin_path_opt, tr, &[])
}

pub fn render(
    lang: &str,
    user: &UserAccount,
    reminders: &[RenewalReminderLog],
    invoices: &[OrderInvoice],
    notifications: &[SystemNotification],
    tags: &[SsbInterestTag],
) -> String {
    render_with_admin_path(lang, user, reminders, invoices, notifications, tags, None)
}

pub fn render_with_admin_path(
    lang: &str,
    user: &UserAccount,
    reminders: &[RenewalReminderLog],
    invoices: &[OrderInvoice],
    notifications: &[SystemNotification],
    tags: &[SsbInterestTag],
    admin_path_opt: Option<&str>,
) -> String {
    render_with_context(
        lang,
        user,
        reminders,
        invoices,
        notifications,
        tags,
        admin_path_opt,
        None,
        &[],
    )
}

#[allow(clippy::too_many_arguments)]
pub fn render_with_context(
    lang: &str,
    user: &UserAccount,
    reminders: &[RenewalReminderLog],
    invoices: &[OrderInvoice],
    notifications: &[SystemNotification],
    tags: &[SsbInterestTag],
    admin_path_opt: Option<&str>,
    tr: Option<&TranslationCache>,
    institutions: &[InstitutionChoice],
) -> String {
    render_with_full_context(
        lang,
        user,
        reminders,
        invoices,
        notifications,
        tags,
        admin_path_opt,
        tr,
        institutions,
        &[],
        &[],
    )
}

fn compute_portal_edergi_permissions(user: &UserAccount) -> (bool, bool) {
    let has_sub = crate::services::content_service::user_has_subscriber_access(user);
    let can_dl = has_sub && (user.email_verified || !is_non_staff_role_name(&user.role));
    (has_sub, can_dl)
}

#[allow(clippy::too_many_arguments)]
pub fn render_with_full_context(
    lang: &str,
    user: &UserAccount,
    reminders: &[RenewalReminderLog],
    invoices: &[OrderInvoice],
    notifications: &[SystemNotification],
    tags: &[SsbInterestTag],
    admin_path_opt: Option<&str>,
    tr: Option<&TranslationCache>,
    institutions: &[InstitutionChoice],
    kvkk_logs: &[KvkkConsentLog],
    contents: &[ContentItem],
) -> String {
    let (has_sub, can_dl) = compute_portal_edergi_permissions(user);
    let can_studio = admin_path_opt.is_some()
        && matches!(
            user.role.trim().to_ascii_lowercase().as_str(),
            "super_admin" | "admin" | "site_operator" | "editor_in_chief" | "managing_editor"
        );
    let ctx = AccountPortalContext {
        lang,
        user,
        reminders,
        invoices,
        notifications,
        tags,
        admin_path_opt,
        tr,
        institutions,
        kvkk_logs,
        contents,
        issues: &[],
        corporate_requests: &[],
        purchased_codes: &[],
        can_download_edergi: can_dl,
        has_subscriber_access: has_sub,
        can_access_design_studio: can_studio,
        hints: PortalQueryHints::default(),
        csp_nonce: None,
    };
    render_portal_inner(&ctx)
}

struct RoleProfile<'a> {
    label: &'a str,
    summary: &'a str,
    tone: &'a str,
}

fn resolve_role_profile(user: &UserAccount, is_en: bool) -> RoleProfile<'static> {
    let norm_role = user.role.trim().to_ascii_lowercase();
    match norm_role.as_str() {
        "subscriber_public" => RoleProfile {
            label: if is_en { "Public Sector Subscriber" } else { "Kamu E-Dergi Abonesi" },
            summary: if is_en {
                "Verified public/defence/academic full E-Magazine & premium dossier access."
            } else {
                "Doğrulanmış kamu, TSK ve akademi tam E-Dergi ve özel dosya erişimi aktif."
            },
            tone: "emerald",
        },
        "subscriber_digital" => RoleProfile {
            label: if is_en { "Digital E-Magazine Subscriber" } else { "Dijital E-Dergi Abonesi" },
            summary: if is_en {
                "Full digital E-Magazine archive, watermarked PDF downloads & premium intelligence."
            } else {
                "Tüm dijital E-Dergi sayıları, filigranlı PDF arşivi ve premium analizler aktif."
            },
            tone: "accent",
        },
        "subscriber_print_digital" => RoleProfile {
            label: if is_en { "Print & Digital Subscriber" } else { "Basılı + Dijital Abone" },
            summary: if is_en {
                "Print magazine delivery plus unlimited digital E-Magazine & special report access."
            } else {
                "Basılı dergi teslimatı ve sınırsız dijital E-Dergi arşivi erişiminiz aktif."
            },
            tone: "gold",
        },
        "subscriber_corporate" => RoleProfile {
            label: if is_en { "Corporate Subscriber" } else { "Kurumsal Abone" },
            summary: if is_en {
                "Corporate access pool, institutional briefings & full digital E-Magazine archive."
            } else {
                "Kurumsal erişim havuzu, sektör brifingleri ve tam dijital dergi arşivi aktif."
            },
            tone: "blue",
        },
        "super_admin" => RoleProfile {
            label: if is_en { "Super Admin" } else { "Süper Yönetici" },
            summary: if is_en {
                "Full platform authority across editorial command, RBAC, commerce & design studio."
            } else {
                "Yayın komuta merkezi, RBAC, ticari operasyonlar ve tasarım stüdyosunda tam yetki."
            },
            tone: "gold",
        },
        "admin" => RoleProfile {
            label: if is_en { "System Admin" } else { "Sistem Yöneticisi" },
            summary: if is_en {
                "Administrative command access across editorial operations, users and analytics."
            } else {
                "Editoryal operasyonlar, kullanıcı yönetimi ve analitik panellerinde tam yetki."
            },
            tone: "gold",
        },
        "site_operator" => RoleProfile {
            label: if is_en { "Site Operator" } else { "Site Operatörü" },
            summary: if is_en {
                "Operational platform access across publishing, design studio and infrastructure."
            } else {
                "Yayın akışı, tasarım stüdyosu ve platform operasyonlarında tam yetkili erişim."
            },
            tone: "gold",
        },
        "editor_in_chief" => RoleProfile {
            label: if is_en { "Editor-in-Chief" } else { "Genel Yayın Yönetmeni" },
            summary: if is_en {
                "Editorial approval authority, dossier curation and E-Magazine publishing access."
            } else {
                "Yayın onay yetkisi, özel dosya kürasyonu ve E-Dergi yayın kontrolü aktif."
            },
            tone: "gold",
        },
        "managing_editor" => RoleProfile {
            label: if is_en { "Managing Editor" } else { "Yazı İşleri Müdürü" },
            summary: if is_en {
                "Newsroom workflow, scheduling and editorial coordination access."
            } else {
                "Haber merkezi iş akışı, zamanlama ve editoryal koordinasyon yetkisi aktif."
            },
            tone: "gold",
        },
        "content_publisher" => RoleProfile {
            label: if is_en { "Content Publisher" } else { "Yayın Sorumlusu" },
            summary: if is_en {
                "Publishing queue, placement management and E-Magazine release access."
            } else {
                "Yayın kuyruğu, yerleştirme yönetimi ve sayı yayınlama erişimi aktif."
            },
            tone: "gold",
        },
        "content_editor" => RoleProfile {
            label: if is_en { "Content Editor" } else { "İçerik Editörü" },
            summary: if is_en {
                "Article authoring, taxonomy tagging and editorial workspace access."
            } else {
                "Haber/analiz hazırlama, taksonomi etiketleme ve editoryal panel erişimi aktif."
            },
            tone: "blue",
        },
        "editor" => RoleProfile {
            label: if is_en { "Editor" } else { "Editör" },
            summary: if is_en {
                "Editorial curation, content authoring and newsroom studio access."
            } else {
                "Editoryal kürasyon, içerik hazırlama ve yayın stüdyosu erişimi aktif."
            },
            tone: "blue",
        },
        "business_dev" => RoleProfile {
            label: if is_en { "Business Development" } else { "İş Geliştirme" },
            summary: if is_en {
                "Commercial partnerships, institutional outreach and revenue analytics access."
            } else {
                "Ticari iş birlikleri, kurumsal analizler ve gelir raporlama erişimi aktif."
            },
            tone: "blue",
        },
        "ads_manager" => RoleProfile {
            label: if is_en { "Ads Manager" } else { "Reklam Yöneticisi" },
            summary: if is_en {
                "Defence ad campaigns, slot quotas and sponsor analytics access."
            } else {
                "Savunma reklam kampanyaları, kota yönetimi ve sponsor analitiği erişimi aktif."
            },
            tone: "blue",
        },
        "finance_viewer" => RoleProfile {
            label: if is_en { "Finance Viewer" } else { "Finans İzleyici" },
            summary: if is_en {
                "Invoices, subscription revenue and commercial ledger audit access."
            } else {
                "Fatura kayıtları, abonelik geliri ve ticari denetim görünümü aktif."
            },
            tone: "blue",
        },
        "support_agent" => RoleProfile {
            label: if is_en { "Support Specialist" } else { "Destek Uzmanı" },
            summary: if is_en {
                "Reader support, corporate verification queue and subscription assistance access."
            } else {
                "Okuyucu destek, kurumsal doğrulama kuyruğu ve abonelik yardım erişimi aktif."
            },
            tone: "blue",
        },
        _ if !is_non_staff_role_name(&user.role) => RoleProfile {
            label: if is_en { "Staff Member" } else { "Yetkili Personel" },
            summary: if is_en {
                "Authorised staff workspace with editorial & operational portal links."
            } else {
                "Yetkili personel çalışma alanı ve operasyonel komuta bağlantıları aktif."
            },
            tone: "gold",
        },
        _ => RoleProfile {
            label: if is_en { "Registered Reader" } else { "Kayıtlı Okuyucu" },
            summary: if is_en {
                "Standard membership · Open archive, personalised SSB feed & upgrade options."
            } else {
                "Standart üyelik · Açık arşiv, kişiselleştirilmiş SSB akışı ve abonelik seçenekleri."
            },
            tone: "neutral",
        },
    }
}

fn portal_svg(name: &str) -> &'static str {
    match name {
        "overview" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><rect x=\"3\" y=\"3\" width=\"7\" height=\"7\" rx=\"1.5\"/><rect x=\"14\" y=\"3\" width=\"7\" height=\"7\" rx=\"1.5\"/><rect x=\"3\" y=\"14\" width=\"7\" height=\"7\" rx=\"1.5\"/><rect x=\"14\" y=\"14\" width=\"7\" height=\"7\" rx=\"1.5\"/></svg>",
        "edergi" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><path d=\"M4 5.5A2.5 2.5 0 0 1 6.5 3H12v18H6.5A2.5 2.5 0 0 0 4 23V5.5Z\"/><path d=\"M20 5.5A2.5 2.5 0 0 0 17.5 3H12v18h5.5A2.5 2.5 0 0 1 20 23V5.5Z\"/></svg>",
        "profile" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M4 21v-2a7 7 0 0 1 14 0v2\"/></svg>",
        "security" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><path d=\"M12 2 4 5.5v6c0 5 3.4 9.5 8 10.5 4.6-1 8-5.5 8-10.5v-6L12 2Z\"/><path d=\"m9.5 12 1.8 1.8 3.6-3.6\"/></svg>",
        "notifications" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><path d=\"M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9\"/><path d=\"M10.3 21a1.94 1.94 0 0 0 3.4 0\"/></svg>",
        "menu" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><path d=\"M4 6h16M4 12h16M4 18h16\"/></svg>",
        "close" => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><path d=\"m6 6 12 12M6 18 18 6\"/></svg>",
        "sun" => "<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"4\"/><path d=\"M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41\"/></svg>",
        _ => "<svg width=\"18\" height=\"18\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" aria-hidden=\"true\"><circle cx=\"12\" cy=\"12\" r=\"9\"/></svg>",
    }
}

fn user_initials(name: &str) -> String {
    let parts: Vec<&str> = name.split_whitespace().filter(|p| !p.is_empty()).collect();
    match (parts.first(), parts.last()) {
        (Some(a), Some(b)) if parts.len() > 1 => {
            let c1 = a.chars().next().unwrap_or('M');
            let c2 = b.chars().next().unwrap_or('S');
            format!("{}{}", c1.to_uppercase(), c2.to_uppercase())
        }
        (Some(a), _) => a.chars().take(2).flat_map(|c| c.to_uppercase()).collect(),
        _ => "MS".to_string(),
    }
}

fn portal_i18n_json(is_en: bool) -> Value {
    let mut map = Map::new();
    let pairs: &[(&str, &str, &str)] = &[
        ("portal.theme_light", "Açık", "Light"),
        ("portal.theme_dark", "Koyu", "Dark"),
        ("portal.theme_to_light", "Açık temaya geç", "Switch to light theme"),
        ("portal.theme_to_dark", "Koyu temaya geç", "Switch to dark theme"),
        ("portal.close", "Kapat", "Close"),
        ("portal.sending", "Gönderiliyor…", "Sending…"),
        ("portal.saving", "Kaydediliyor…", "Saving…"),
        ("portal.verifying", "Doğrulanıyor…", "Verifying…"),
        ("portal.processing", "İşleniyor…", "Processing…"),
        ("portal.deleting", "Hesabınız kalıcı olarak siliniyor…", "Permanently deleting your account…"),
        ("portal.conn_err", "Bağlantı hatası.", "Connection error."),
        ("portal.verify_err", "Doğrulama e-postası gönderilemedi.", "Verification email could not be sent."),
        ("portal.verify_ok", "Doğrulama e-postası adresinize tekrar gönderildi.", "Verification email has been resent."),
        ("portal.profile_err", "Profil güncellenemedi.", "Profile could not be updated."),
        ("portal.profile_ok", "Profiliniz ve SSB ilgi alanlarınız güncellendi.", "Your profile and SSB interests have been updated."),
        ("portal.totp_gen", "Güvenli 2FA anahtarı üretiliyor…", "Generating secure 2FA secret…"),
        ("portal.totp_setup_err", "2FA kurulumu başlatılamadı.", "2FA setup could not be started."),
        ("portal.totp_code_err", "Doğrulama kodu hatalı. Lütfen tekrar deneyin.", "Invalid verification code. Please try again."),
        ("portal.totp_ok", "İki aşamalı doğrulama (2FA) etkinleştirildi.", "Two-factor authentication (2FA) is now enabled."),
        ("portal.codes_copied", "Kurtarma kodları kopyalandı.", "Recovery codes copied."),
        ("portal.copy_manual", "Kodları manuel olarak kopyalayın.", "Please copy the codes manually."),
        ("portal.totp_dis_err", "2FA devre dışı bırakılamadı.", "2FA could not be disabled."),
        ("portal.totp_dis_ok", "İki aşamalı doğrulama devre dışı bırakıldı.", "Two-factor authentication has been disabled."),
        ("portal.delete_err", "Hesap silme işlemi tamamlanamadı. Şifrenizi kontrol edin.", "Account deletion failed. Please check your password."),
        ("portal.delete_ok", "Hesabınız KVKK kapsamında kalıcı olarak silindi.", "Your account has been permanently deleted."),
        ("portal.confirm_erasure_err", "Kalıcı hesap silme işlemini onaylamanız gerekmektedir.", "Please confirm permanent account erasure."),
        ("portal.pw_required", "Lütfen mevcut şifrenizi girin.", "Please enter your current password."),
        ("portal.notif_read", "Bildirim okundu olarak işaretlendi.", "Notification marked as read."),
        ("portal.notif_all_read", "Tüm bildirimler okundu işaretlendi.", "All notifications marked as read."),
        ("portal.notif_partial", "Bazı bildirimler güncellenemedi. Lütfen tekrar deneyin.", "Some notifications could not be updated. Please try again."),
        ("portal.notif_err", "Bildirimler güncellenemedi.", "Could not update notifications."),
        ("portal.notif_status_read", "Okundu", "Read"),
        ("portal.logout_err", "Çıkış yapılamadı. Tekrar deneyin.", "Sign-out failed. Please try again."),
    ];
    for (k, tr_v, en_v) in pairs {
        map.insert((*k).to_string(), Value::String(if is_en { *en_v } else { *tr_v }.to_string()));
    }
    Value::Object(map)
}

pub fn render_portal_document(ctx: &AccountPortalContext<'_>) -> String {
    let is_en = ctx.lang.eq_ignore_ascii_case("en");
    let norm_lang = if is_en { "en" } else { "tr" };
    let clean_nonce = ctx.csp_nonce.map(str::trim).filter(|n| !n.is_empty());
    let role_prof = resolve_role_profile(ctx.user, is_en);
    let unread_count = ctx.notifications.iter().filter(|n| !n.is_read).count();

    let page_title = tr_text(
        ctx.tr,
        norm_lang,
        "account.page_title_logged_in",
        "Okuyucu & Abone Kontrol Merkezi — MSI Dergisi",
        "Reader & Subscriber Control Center — MSI Review",
    );
    let skip_lbl = tr_text(ctx.tr, norm_lang, "account.skip_to_workspace", "Çalışma alanına geç", "Skip to workspace");
    let brand_title = tr_text(ctx.tr, norm_lang, "account.brand_title", "MSI DERGİSİ", "MSI REVIEW");
    let brand_sub = tr_text(ctx.tr, norm_lang, "account.brand_sub", "Okuyucu Portalı", "Reader Portal");
    let home_href = lang_href("/", norm_lang);
    let back_site_lbl = tr_text(ctx.tr, norm_lang, "account.back_to_site", "Siteye Dön", "Back to Site");
    let other_lang = if is_en { "tr" } else { "en" };
    let switch_label = if is_en { "TR" } else { "EN" };
    let switch_aria = tr_text(ctx.tr, norm_lang, "account.switch_lang_aria", "Switch to English", "Türkçe arayüze geç");
    let logout_short = tr_text(ctx.tr, norm_lang, "account.logout_short", "Çıkış", "Sign out");
    let nav_aria = tr_text(ctx.tr, norm_lang, "account.nav_aria", "Hesap portalı menüsü", "Account portal navigation");
    let nav_group_lbl = tr_text(ctx.tr, norm_lang, "account.nav_group", "Çalışma Alanı", "Workspace");
    let close_menu_lbl = tr_text(ctx.tr, norm_lang, "account.close_menu", "Menüyü kapat", "Close navigation");
    let open_menu_lbl = tr_text(ctx.tr, norm_lang, "account.open_menu", "Menüyü aç", "Open navigation");
    let theme_dark_lbl = tr_text(ctx.tr, norm_lang, "portal.theme_dark", "Koyu", "Dark");

    let initial_tab = normalize_portal_tab(ctx.hints.active_tab, ctx.hints.payment_success);

    let nav_items: &[(&str, &str, &str, &str, &str)] = &[
        ("overview", "overview", "account.nav_overview", "Genel Bakış & Akış", "Overview & Feed"),
        ("edergi", "edergi", "account.nav_edergi", "E-Dergi & Abonelik", "E-Magazine & Subscription"),
        ("profile", "profile", "account.nav_profile", "Profil & SSB İlgi", "Profile & SSB Interests"),
        ("security", "security", "account.nav_security", "Güvenlik & KVKK", "Security & KVKK"),
        ("notifications", "notifications", "account.nav_notifications", "Bildirimler", "Notifications"),
    ];

    let clean_admin = ctx
        .admin_path_opt
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty() && !is_non_staff_role_name(&ctx.user.role))
        .map(crate::config::normalize_admin_path_secret);

    let initials = user_initials(&ctx.user.full_name);
    let inner_html = render_portal_inner(ctx);
    let edergi_href = lang_href("/e-dergi", norm_lang);
    let sub_href = lang_href("/abonelik-ve-satis", norm_lang);
    let footer_edergi_lbl = tr_text(ctx.tr, norm_lang, "account.footer_edergi", "E-Dergi Arşivi", "E-Magazine Kiosk");
    let footer_sub_lbl = tr_text(ctx.tr, norm_lang, "account.footer_sub", "Abonelik & Mağaza", "Subscriptions & Store");
    let footer_cmd_lbl = tr_text(ctx.tr, norm_lang, "account.footer_cmd", "Komuta Merkezi", "Command Center");
    let footer_studio_lbl = tr_text(ctx.tr, norm_lang, "account.footer_studio", "Tasarım Stüdyosu", "Design Studio");
    let footer_editor_lbl = tr_text(ctx.tr, norm_lang, "account.footer_editor", "Editör Stüdyosu", "Editorial Studio");

    let mut buf = HtmlBuf::with_capacity(65536);
    buf.raw("<!DOCTYPE html>");
    buf.el("html", &[("lang", norm_lang)], |html| {
        html.el("head", &[], |head| {
            head.void_el("meta", &[("charset", "UTF-8")]);
            head.void_el("meta", &[("name", "viewport"), ("content", "width=device-width, initial-scale=1.0")]);
            head.void_el("meta", &[("name", "robots"), ("content", "noindex,nofollow,noarchive")]);
            head.void_el("meta", &[("name", "referrer"), ("content", "same-origin")]);
            head.el("title", &[], |t| {
                t.text(&page_title);
            });
            head.void_el("link", &[("rel", "icon"), ("href", "/favicon.ico"), ("sizes", "any")]);
            head.void_el("link", &[("rel", "icon"), ("type", "image/svg+xml"), ("href", "/favicon.svg")]);
            head.void_el(
                "link",
                &[
                    ("rel", "preload"),
                    ("as", "font"),
                    ("type", "font/woff2"),
                    ("href", "/fonts/editorial/ibm-plex-sans-latin-ext-400-normal.woff2"),
                    ("crossorigin", "anonymous"),
                ],
            );
            head.void_el(
                "link",
                &[
                    ("rel", "preload"),
                    ("as", "font"),
                    ("type", "font/woff2"),
                    ("href", "/fonts/editorial/ibm-plex-sans-latin-400-normal.woff2"),
                    ("crossorigin", "anonymous"),
                ],
            );
            head.script_inline(THEME_PREHYDRATE_JS, clean_nonce, None);
            head.link_css("/css/design-tokens.css?v=35");
            head.link_css("/css/account-portal.css?v=35");
        });

        html.el(
            "body",
            &[("class", "ap-body"), ("data-initial-tab", initial_tab)],
            |body| {
                body.el("a", &[("class", "ap-skip-link"), ("href", "#main-content")], |a| {
                    a.text(&skip_lbl);
                });
                body.el("div", &[("class", "ap-ambient-stage"), ("aria-hidden", "true")], |_| {});

                // Sticky Top Bar
                body.el("header", &[("class", "ap-topbar")], |hdr| {
                    hdr.el("div", &[("class", "ap-topbar-left")], |left| {
                        left.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "ap-menu-btn"),
                                ("id", "ap-sidebar-toggle"),
                                ("aria-controls", "ap-sidebar"),
                                ("aria-expanded", "false"),
                                ("aria-label", &open_menu_lbl),
                            ],
                            |b| {
                                b.raw(portal_svg("menu"));
                            },
                        );
                        left.el("a", &[("class", "ap-brand"), ("href", &home_href)], |a| {
                            a.void_el(
                                "img",
                                &[
                                    ("class", "ap-brand-logo"),
                                    ("src", "/images/logo.svg"),
                                    ("alt", "MSI"),
                                    ("width", "64"),
                                    ("height", "24"),
                                ],
                            );
                            a.el("span", &[("class", "ap-brand-text")], |bt| {
                                bt.el("span", &[("class", "ap-brand-title")], |t| {
                                    t.text(&brand_title);
                                });
                                bt.el("span", &[("class", "ap-brand-sub")], |t| {
                                    t.text(&brand_sub);
                                });
                            });
                        });
                        left.el("a", &[("class", "ap-back-link"), ("href", &home_href)], |a| {
                            a.text("← ");
                            a.el("span", &[("class", "ap-back-text")], |s| {
                                s.text(&back_site_lbl);
                            });
                        });
                    });

                    hdr.el("div", &[("class", "ap-topbar-right")], |right| {
                        let role_cls = format!("ap-role-pill is-{}", role_prof.tone);
                        right.el("span", &[("class", &role_cls)], |p| {
                            p.text(role_prof.label);
                        });
                        let switch_href = format!("/switch/{}", other_lang);
                        right.el(
                            "a",
                            &[
                                ("href", &switch_href),
                                ("class", "ap-util-btn"),
                                ("data-ap-lang-switch", ""),
                                ("aria-label", &switch_aria),
                            ],
                            |a| {
                                a.text(switch_label);
                            },
                        );
                        right.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("id", "ap-theme-toggle"),
                                ("class", "ap-util-btn"),
                                ("aria-pressed", "false"),
                            ],
                            |b| {
                                b.raw(portal_svg("sun"));
                                b.el("span", &[("data-theme-label", "")], |s| {
                                    s.text(&theme_dark_lbl);
                                });
                            },
                        );
                        right.el(
                            "button",
                            &[
                                ("type", "button"),
                                ("class", "ap-logout-btn"),
                                ("data-logout", ""),
                            ],
                            |b| {
                                b.text(&logout_short);
                            },
                        );
                    });
                });

                // Workspace Shell Grid
                body.el("div", &[("class", "ap-shell")], |shell| {
                    shell.el_with(
                        "div",
                        |b| {
                            b.attr("class", "ap-sidebar-backdrop")
                                .attr("id", "ap-sidebar-backdrop")
                                .bool_attr("hidden", true);
                        },
                        |_| {},
                    );

                    shell.el(
                        "aside",
                        &[
                            ("class", "ap-sidebar"),
                            ("id", "ap-sidebar"),
                            ("aria-label", &nav_aria),
                        ],
                        |sb| {
                            sb.el("div", &[("class", "ap-sidebar-close-row")], |cr| {
                                cr.el("span", &[("class", "ap-nav-group-label")], |gl| {
                                    gl.text(&brand_sub);
                                });
                                cr.el(
                                    "button",
                                    &[
                                        ("type", "button"),
                                        ("class", "ap-menu-btn"),
                                        ("id", "ap-sidebar-close"),
                                        ("aria-label", &close_menu_lbl),
                                    ],
                                    |b| {
                                        b.raw(portal_svg("close"));
                                    },
                                );
                            });

                            sb.el("div", &[("class", "ap-user-card")], |uc| {
                                uc.el("div", &[("class", "ap-avatar"), ("aria-hidden", "true")], |av| {
                                    av.text(&initials);
                                });
                                uc.el("div", &[("class", "ap-user-meta")], |um| {
                                    um.el("div", &[("class", "ap-user-name")], |n| {
                                        n.text(&ctx.user.full_name);
                                    });
                                    um.el("div", &[("class", "ap-user-email")], |em| {
                                        em.text(&ctx.user.email);
                                    });
                                    if !ctx.user.organization_name.trim().is_empty() {
                                        um.el("div", &[("class", "ap-user-org")], |org| {
                                            org.text(&ctx.user.organization_name);
                                        });
                                    }
                                });
                            });

                            sb.el("div", &[("class", "ap-sidebar-tabs-group")], |grp| {
                                grp.el("div", &[("class", "ap-nav-group-label")], |gl| {
                                    gl.text(&nav_group_lbl);
                                });
                                grp.el(
                                    "div",
                                    &[
                                        ("class", "ap-nav-list"),
                                        ("role", "tablist"),
                                        ("aria-orientation", "vertical"),
                                    ],
                                    |list| {
                                        for (key, ico, tr_key, lbl_tr, lbl_en) in nav_items {
                                            let is_active = *key == initial_tab;
                                            let btn_id = format!("ap-tab-{}", key);
                                            let panel_id = format!("ap-panel-{}", key);
                                            let lbl_txt = tr_text(ctx.tr, norm_lang, tr_key, lbl_tr, lbl_en);
                                            let cls = if is_active {
                                                "ap-nav-btn is-active"
                                            } else {
                                                "ap-nav-btn"
                                            };
                                            list.el(
                                                "button",
                                                &[
                                                    ("type", "button"),
                                                    ("role", "tab"),
                                                    ("id", &btn_id),
                                                    ("class", cls),
                                                    ("data-ap-tab", key),
                                                    ("aria-controls", &panel_id),
                                                    ("aria-selected", if is_active { "true" } else { "false" }),
                                                ],
                                                |btn| {
                                                    btn.raw(portal_svg(ico));
                                                    btn.el("span", &[("class", "ap-nav-label")], |sp| {
                                                        sp.text(&lbl_txt);
                                                    });
                                                    if *key == "notifications" {
                                                        let cnt_str = unread_count.to_string();
                                                        btn.el_with(
                                                            "span",
                                                            |ba| {
                                                                ba.attr("class", "ap-nav-badge")
                                                                    .attr("id", "ap-notif-badge")
                                                                    .attr("data-notif-badge", "")
                                                                    .attr("data-unread-count", &cnt_str)
                                                                    .bool_attr("hidden", unread_count == 0);
                                                            },
                                                            |b| {
                                                                b.text(&cnt_str);
                                                            },
                                                        );
                                                    }
                                                },
                                            );
                                        }
                                    },
                                );
                            });

                            sb.el("div", &[("class", "ap-sidebar-footer")], |ft| {
                                ft.el("a", &[("class", "ap-sidebar-link"), ("href", &edergi_href)], |a| {
                                    a.text(&footer_edergi_lbl);
                                    a.text(" ↗");
                                });
                                ft.el("a", &[("class", "ap-sidebar-link"), ("href", &sub_href)], |a| {
                                    a.text(&footer_sub_lbl);
                                    a.text(" ↗");
                                });
                                if let Some(adm) = clean_admin {
                                    let adm_href = format!("/{}", adm);
                                    let studio_href = format!("/{}/tasarim", adm);
                                    let editor_href = format!("/{}/editor", adm);
                                    ft.el(
                                        "a",
                                        &[("class", "ap-sidebar-link is-staff"), ("href", &adm_href)],
                                        |a| {
                                            a.text(&footer_cmd_lbl);
                                            a.text(" →");
                                        },
                                    );
                                    if ctx.can_access_design_studio {
                                        ft.el(
                                            "a",
                                            &[("class", "ap-sidebar-link is-staff"), ("href", &studio_href)],
                                            |a| {
                                                a.text(&footer_studio_lbl);
                                                a.text(" →");
                                            },
                                        );
                                    }
                                    if can_access_editorial_studio(&ctx.user.role) {
                                        ft.el(
                                            "a",
                                            &[("class", "ap-sidebar-link is-staff"), ("href", &editor_href)],
                                            |a| {
                                                a.text(&footer_editor_lbl);
                                                a.text(" →");
                                            },
                                        );
                                    }
                                }
                            });
                        },
                    );

                    shell.el(
                        "main",
                        &[
                            ("id", "main-content"),
                            ("class", "ap-main"),
                            ("tabindex", "-1"),
                        ],
                        |main| {
                            main.raw(&inner_html);
                        },
                    );
                });

                body.el(
                    "div",
                    &[
                        ("class", "ap-toast-region"),
                        ("aria-live", "polite"),
                        ("aria-atomic", "false"),
                    ],
                    |_| {},
                );
                body.json_script("msi-i18n", &portal_i18n_json(is_en), clean_nonce);
                body.script_src("/js/account-portal.js?v=35", clean_nonce, true);
            },
        );
    });
    buf.into_string()
}

fn render_portal_inner(ctx: &AccountPortalContext<'_>) -> String {
    let lang = ctx.lang;
    let user = ctx.user;
    let reminders = ctx.reminders;
    let invoices = ctx.invoices;
    let notifications = ctx.notifications;
    let tags = ctx.tags;
    let tr = ctx.tr;
    let institutions = ctx.institutions;
    let kvkk_logs = ctx.kvkk_logs;
    let contents = ctx.contents;
    let is_en = lang.eq_ignore_ascii_case("en");
    let role_prof = resolve_role_profile(user, is_en);
    let unread_count = notifications.iter().filter(|n| !n.is_read).count();

    let initial_tab = normalize_portal_tab(ctx.hints.active_tab, ctx.hints.payment_success);

    let command_card = ctx
        .admin_path_opt
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty() && !is_non_staff_role_name(&user.role))
        .map(crate::config::normalize_admin_path_secret)
        .map(|clean_admin| {
            let safe_admin = esc(&clean_admin);
            let eyebrow = tr_text(
                tr,
                lang,
                "account.operator_eyebrow",
                "YETKİLİ OPERATÖR ERİŞİMİ",
                "AUTHORIZED OPERATOR ACCESS",
            );
            let title = tr_text(
                tr,
                lang,
                "account.operator_title",
                "Yönetim Komuta Merkezi",
                "Editorial & Command Center",
            );
            let role_slug = user.role.trim().to_ascii_lowercase();
            let (default_desc_tr, default_desc_en) = match role_slug.as_str() {
                "super_admin" | "admin" | "site_operator" => (
                    "Editoryal içerik akışı, E-Dergi sayıları, savunma reklam kampanyaları, analitik telemetri ve merkezi tasarım stüdyosuna doğrudan geçiş yapın.",
                    "Manage editorial dossiers, digital issues, defence ad campaigns, RBAC permissions, and central design architecture.",
                ),
                "editor_in_chief" | "managing_editor" => (
                    "Yayın onay kuyruğu, E-Dergi sayı planlaması, editoryal stüdyo ve merkezi tasarım mimarisine doğrudan geçiş yapın.",
                    "Access editorial approval queues, E-Magazine issue planning, the Editorial Studio, and design architecture.",
                ),
                "content_publisher" | "content_editor" | "editor" => (
                    "Haber ve analiz taslakları, medya kütüphanesi, taksonomi etiketleme ve Editör Stüdyosu çalışma alanına doğrudan geçin.",
                    "Jump directly to article drafts, media library curation, taxonomy tagging, and the Editorial Studio.",
                ),
                "business_dev" | "ads_manager" => (
                    "Savunma reklam kampanyaları, yerleştirme kotaları, sponsorluk analitiği ve kurumsal ticari panellere geçin.",
                    "Manage defence ad campaigns, slot quotas, sponsorship analytics, and commercial outreach panels.",
                ),
                "finance_viewer" => (
                    "Abonelik gelirleri, fatura kayıtları, sipariş mutabakatı ve finansal denetim görünümüne doğrudan geçin.",
                    "Review subscription revenue, invoice ledgers, order settlements, and financial audit reports.",
                ),
                "support_agent" => (
                    "Okuyucu hesap doğrulama kuyruğu, kurumsal erişim talepleri ve abonelik destek araçlarına doğrudan geçin.",
                    "Access reader verification queues, corporate approval requests, and subscriber support tools.",
                ),
                _ => (
                    "Yetkili operasyonel panellere ve rolünüze tanımlı yönetim araçlarına doğrudan geçiş yapın.",
                    "Access authorised operational dashboards and management tools scoped to your role.",
                ),
            };
            let desc = tr_text(
                tr,
                lang,
                "account.operator_desc",
                default_desc_tr,
                default_desc_en,
            );
            let btn_main = tr_text(
                tr,
                lang,
                "account.operator_btn_main",
                "Yönetim Komuta Merkezi →",
                "Management Command Center →",
            );
            let studio_action = if ctx.can_access_design_studio {
                let btn_studio = tr_text(
                    tr,
                    lang,
                    "account.operator_btn_studio",
                    "Tasarım Stüdyosu →",
                    "Design Studio →",
                );
                format!(
                    r#"<a href="/{safe_admin}/tasarim" class="button secondary">{btn_studio}</a>"#,
                    safe_admin = safe_admin,
                    btn_studio = esc(&btn_studio),
                )
            } else {
                String::new()
            };
            let editor_action = if can_access_editorial_studio(&user.role) {
                let btn_editor = tr_text(
                    tr,
                    lang,
                    "account.operator_btn_editor",
                    "Editör Stüdyosu →",
                    "Editorial Studio →",
                );
                format!(
                    r#"<a href="/{safe_admin}/editor" class="button secondary">{btn_editor}</a>"#,
                    safe_admin = safe_admin,
                    btn_editor = esc(&btn_editor),
                )
            } else {
                String::new()
            };
            format!(
                r#"<aside class="account-command-card"><div class="account-command-copy"><div class="eyebrow">{eyebrow}</div><h2>{title}</h2><p>{desc}</p></div><div class="account-command-actions"><a href="/{safe_admin}" class="button primary">{btn_main}</a>{studio_action}{editor_action}</div></aside>"#,
                eyebrow = esc(&eyebrow),
                title = esc(&title),
                desc = esc(&desc),
                safe_admin = safe_admin,
                btn_main = esc(&btn_main),
                studio_action = studio_action,
                editor_action = editor_action,
            )
        })
        .unwrap_or_default();

    let search_base = if is_en { "/en/search" } else { "/arama" };
    let interests: String = user
        .ssb_interests
        .iter()
        .map(|code| {
            let name = tags
                .iter()
                .find(|t| t.code == *code)
                .map(|t| localized(lang, &t.name_tr, &t.name_en).to_string())
                .unwrap_or_else(|| ssb_tag_display_label(code, lang));
            format!(
                "<a href=\"{}?ssb_tag={}\" class=\"topic-chip\">#{}</a>",
                search_base,
                esc(&urlencoding::encode(code)),
                esc(&name)
            )
        })
        .collect();

    let ssb_checkboxes: String = if !tags.is_empty() {
        tags.iter()
            .map(|t| {
                let checked = if user.ssb_interests.iter().any(|c| c == &t.code) {
                    " checked"
                } else {
                    ""
                };
                format!(
                    r#"<label class="ssb-chip"><input type="checkbox" name="ssb_interests" value="{code}"{checked}><span>{label}</span></label>"#,
                    code = esc(&t.code),
                    checked = checked,
                    label = esc(localized(lang, &t.name_tr, &t.name_en))
                )
            })
            .collect()
    } else {
        CANONICAL_SSB_TAG_CODES
            .iter()
            .map(|code| {
                let checked = if user.ssb_interests.iter().any(|c| c == code) {
                    " checked"
                } else {
                    ""
                };
                format!(
                    r#"<label class="ssb-chip"><input type="checkbox" name="ssb_interests" value="{code}"{checked}><span>{label}</span></label>"#,
                    code = esc(code),
                    checked = checked,
                    label = esc(&ssb_tag_display_label(code, lang))
                )
            })
            .collect()
    };

    let fallback_insts;
    let inst_choices: &[InstitutionChoice] = if institutions.is_empty() {
        fallback_insts = default_institution_choices();
        &fallback_insts
    } else {
        institutions
    };
    let mut org_options: String = inst_choices
        .iter()
        .map(|inst| {
            let sel = if user.organization_category.eq_ignore_ascii_case(&inst.code) {
                " selected"
            } else {
                ""
            };
            let label = if is_en && !inst.label_en.trim().is_empty() {
                &inst.label_en
            } else {
                &inst.label_tr
            };
            format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = esc(&inst.code),
                sel = sel,
                label = esc(label)
            )
        })
        .collect();
    if !user.organization_category.trim().is_empty()
        && !inst_choices
            .iter()
            .any(|inst| user.organization_category.trim().eq_ignore_ascii_case(&inst.code))
    {
        let current_cat = esc(user.organization_category.trim());
        org_options = format!(
            r#"<option value="{val}" selected>{val}</option>{rest}"#,
            val = current_cat,
            rest = org_options
        );
    }
    let is_institution_locked = is_non_staff_role_name(&user.role)
        && (user.is_gov_verified
            || user.role.eq_ignore_ascii_case("subscriber_public")
            || user.role.eq_ignore_ascii_case("subscriber_corporate"));

    let verification_html = if user.email_verified {
        let lbl = tr_text(tr, lang, "account.verified", "✓ Doğrulanmış", "✓ Verified");
        format!(r#"<span class="status-badge is-verified">{}</span>"#, esc(&lbl))
    } else {
        let badge_lbl = tr_text(
            tr,
            lang,
            "account.pending_verification",
            "● Doğrulama Bekliyor",
            "● Verification Pending",
        );
        let resend_lbl = tr_text(
            tr,
            lang,
            "account.resend_verification",
            "Doğrulama E-postasını Tekrar Gönder",
            "Resend Verification Email",
        );
        format!(
            r#"<span class="status-badge is-pending">{badge_lbl}</span><button type="button" class="button secondary btn-sm" id="account-resend-verify" data-user-email="{email}">{resend_lbl}</button><span id="account-verify-feedback" class="inline-feedback" role="status"></span>"#,
            badge_lbl = esc(&badge_lbl),
            email = esc(&user.email),
            resend_lbl = esc(&resend_lbl)
        )
    };

    let gov_badge = if user.is_gov_verified {
        let lbl = tr_text(
            tr,
            lang,
            "account.gov_verified_badge",
            "✓ Kamu / Akademik Doğrulamalı",
            "✓ Public / Academic Verified",
        );
        format!(r#"<span class="status-badge is-gov">{}</span>"#, esc(&lbl))
    } else {
        String::new()
    };

    let totp_eyebrow = tr_text(
        tr,
        lang,
        "account.totp_eyebrow",
        "HESAP GÜVENLİĞİ · 2FA",
        "ACCOUNT SECURITY · 2FA",
    );
    let totp_title = tr_text(
        tr,
        lang,
        "account.totp_title",
        "İki Aşamalı Doğrulama",
        "Two-Factor Authentication",
    );
    let totp_card_html = if user.totp_enabled {
        let status_lbl = tr_text(
            tr,
            lang,
            "account.totp_active",
            "✓ Etkin (Authenticator)",
            "✓ Active (TOTP)",
        );
        let active_desc = tr_text(
            tr,
            lang,
            "account.totp_active_desc",
            "Hesabınız TOTP tabanlı Authenticator uygulaması ve tek kullanımlık kurtarma kodları ile korunmaktadır.",
            "Your account is protected with a TOTP authenticator app and single-use recovery codes.",
        );
        let cur_pw_lbl = tr_text(tr, lang, "account.current_password", "Mevcut şifreniz", "Current password");
        let code_lbl = tr_text(
            tr,
            lang,
            "account.totp_code_label",
            "Authenticator / Yedek kod",
            "Authenticator / Backup code",
        );
        let disable_btn = tr_text(
            tr,
            lang,
            "account.totp_disable_btn",
            "2FA Korumasını Devre Dışı Bırak",
            "Disable 2FA Protection",
        );
        format!(
            r#"<article class="account-panel account-security-panel" id="account-totp-card"><div class="panel-header-row"><div><div class="eyebrow">{totp_eyebrow}</div><h2>{totp_title}</h2></div><span class="status-badge is-verified">{status_lbl}</span></div><p>{active_desc}</p><form id="account-totp-disable-form" class="ui-form account-totp-form"><div class="form-grid-2"><label>{cur_pw_lbl}<input name="password" type="password" autocomplete="current-password" placeholder="••••••••" required></label><label>{code_lbl}<input name="code" type="text" class="totp-code-input" autocomplete="one-time-code" minlength="6" maxlength="20" placeholder="000000" required></label></div><p class="form-feedback" role="status"></p><button type="submit" class="button danger-outline">{disable_btn}</button></form></article>"#,
            totp_eyebrow = esc(&totp_eyebrow),
            totp_title = esc(&totp_title),
            status_lbl = esc(&status_lbl),
            active_desc = esc(&active_desc),
            cur_pw_lbl = esc(&cur_pw_lbl),
            code_lbl = esc(&code_lbl),
            disable_btn = esc(&disable_btn),
        )
    } else {
        let status_lbl = tr_text(tr, lang, "account.totp_disabled", "Devre Dışı", "Disabled");
        let setup_desc = tr_text(
            tr,
            lang,
            "account.totp_setup_desc",
            "Google Authenticator, Microsoft Authenticator veya 1Password ile kurumsal hesabınıza ikinci güvenlik katmanı ekleyin.",
            "Add a second security layer to your institutional account with Google Authenticator, Microsoft Authenticator or 1Password.",
        );
        let setup_btn = tr_text(
            tr,
            lang,
            "account.totp_setup_btn",
            "2FA Kurulumunu Başlat (QR Kod) →",
            "Start 2FA Setup (QR Code) →",
        );
        let manual_key = tr_text(
            tr,
            lang,
            "account.totp_manual_key",
            "MANUEL KURULUM ANAHTARI (BASE32)",
            "MANUAL SETUP KEY (BASE32)",
        );
        let qr_help = tr_text(
            tr,
            lang,
            "account.totp_qr_help",
            "QR kodu kamerayla okutun veya anahtarı Authenticator uygulamanıza manuel ekleyin, ardından üretilen doğrulama kodunu aşağıya girin.",
            "Scan the QR code or enter the key manually in your authenticator app, then enter the generated verification code below.",
        );
        let verify_code_lbl = tr_text(tr, lang, "account.totp_verify_label", "Doğrulama kodu", "Verification code");
        let activate_btn = tr_text(
            tr,
            lang,
            "account.totp_activate_btn",
            "Doğrula ve 2FA'yı Etkinleştir →",
            "Verify and Enable 2FA →",
        );
        let backup_eyebrow = tr_text(
            tr,
            lang,
            "account.totp_backup_eyebrow",
            "TEK KULLANIMLIK KURTARMA KODLARI",
            "SINGLE-USE RECOVERY CODES",
        );
        let backup_desc = tr_text(
            tr,
            lang,
            "account.totp_backup_desc",
            "Cihazınıza erişemediğinizde giriş yapabilmek için bu kodları güvenli bir yerde saklayın:",
            "Store these codes safely to sign in if you lose access to your device:",
        );
        let copy_btn = tr_text(tr, lang, "account.totp_copy_codes", "Kodları Kopyala", "Copy Codes");
        let done_btn = tr_text(tr, lang, "account.totp_done_btn", "Kaydettim, Tamamla →", "Saved, Complete →");
        let qr_alt = tr_text(tr, lang, "account.totp_qr_alt", "Authenticator 2FA QR Kodu", "Authenticator 2FA QR Code");
        format!(
            r#"<article class="account-panel account-security-panel" id="account-totp-card"><div class="panel-header-row"><div><div class="eyebrow">{totp_eyebrow}</div><h2>{totp_title}</h2></div><span class="status-badge is-muted">{status_lbl}</span></div><p>{setup_desc}</p><div class="account-security-actions"><button type="button" class="button primary" id="account-totp-setup-btn">{setup_btn}</button></div><p class="form-feedback" id="account-totp-setup-feedback" role="status"></p><div id="account-totp-setup-panel" class="totp-setup-box" hidden><div class="totp-qr-wrap"><div class="totp-qr-frame"><img id="account-totp-qr" alt="{qr_alt}"></div><div class="totp-manual-meta"><span class="eyebrow">{manual_key}</span><code id="account-totp-secret" class="totp-secret-code"></code><p class="field-help">{qr_help}</p></div></div><form id="account-totp-verify-form" class="ui-form"><input type="hidden" name="secret" id="account-totp-secret-input" value=""><label>{verify_code_lbl}<input name="code" type="text" class="totp-code-input" autocomplete="one-time-code" minlength="6" maxlength="20" placeholder="000000" required></label><p class="form-feedback" role="status"></p><button type="submit" class="button primary">{activate_btn}</button></form><div id="totp-backup-codes-box" class="totp-backup-codes" hidden><div class="eyebrow">{backup_eyebrow}</div><p>{backup_desc}</p><div id="totp-backup-codes-list" class="backup-codes-grid"></div><div class="hero-actions"><button type="button" class="button secondary" data-copy-backup-codes>{copy_btn}</button><button type="button" class="button primary" data-reload-page>{done_btn}</button></div></div></div></article>"#,
            totp_eyebrow = esc(&totp_eyebrow),
            totp_title = esc(&totp_title),
            status_lbl = esc(&status_lbl),
            setup_desc = esc(&setup_desc),
            setup_btn = esc(&setup_btn),
            qr_alt = esc(&qr_alt),
            manual_key = esc(&manual_key),
            qr_help = esc(&qr_help),
            verify_code_lbl = esc(&verify_code_lbl),
            activate_btn = esc(&activate_btn),
            backup_eyebrow = esc(&backup_eyebrow),
            backup_desc = esc(&backup_desc),
            copy_btn = esc(&copy_btn),
            done_btn = esc(&done_btn),
        )
    };

    let invoice_rows: String = invoices
        .iter()
        .map(|i| {
            let (st_cls, st_lbl) = localize_payment_status(&i.payment_status, is_en);
            let inv_date = format_portal_date(&i.created_at, is_en);
            format!(
                r#"<tr><td><code>{}</code></td><td>{}</td><td>₺{:.0}</td><td><span class="status-badge {}">{}</span></td><td>{}</td></tr>"#,
                esc(&i.invoice_no),
                esc(&i.item_title),
                i.amount_try,
                st_cls,
                esc(st_lbl),
                esc(&inv_date)
            )
        })
        .collect();

    let notices_html: String = notifications
        .iter()
        .map(|n| render_portal_notification_card(n, lang, tr))
        .collect();

    let reminder_text = reminders
        .first()
        .map(|r| {
            if is_en {
                format!("Your subscription expires in {} days.", r.days_remaining)
            } else {
                format!("Aboneliğinizin bitişine {} gün kaldı.", r.days_remaining)
            }
        })
        .unwrap_or_else(|| {
            tr_text(
                tr,
                lang,
                "account.no_reminders",
                "Şu anda bekleyen abonelik hatırlatmanız yok.",
                "You have no pending subscription renewal reminders.",
            )
        });

    let no_orders_lbl = tr_text(
        tr,
        lang,
        "account.no_orders",
        "Henüz siparişiniz bulunmuyor.",
        "You have no orders yet.",
    );
    let empty_invoices = if invoices.is_empty() {
        format!("<tr><td colspan=\"5\">{}</td></tr>", esc(&no_orders_lbl))
    } else {
        String::new()
    };

    let email_opt_checked = if user.email_consent && user.marketing_emails_opted_in {
        " checked"
    } else {
        ""
    };

    let eyebrow_top = tr_text(tr, lang, "account.eyebrow", "OKUYUCU MERKEZİ", "READER CENTER");
    let greet = if is_en {
        format!("Hello, {}.", user.full_name)
    } else {
        format!("Merhaba, {}.", user.full_name)
    };
    let subtitle = tr_text(
        tr,
        lang,
        "account.subtitle",
        "Aboneliğiniz, güvenlik ayarlarınız ve kişiselleştirilmiş savunma akışınız.",
        "Your subscription, security settings and personalised defence feed.",
    );
    let heading_html = page_heading(&eyebrow_top, &greet, &subtitle);

    let sub_eyebrow = tr_text(tr, lang, "account.sub_eyebrow", "ABONELİĞİNİZ", "YOUR SUBSCRIPTION");
    let expires_lbl = tr_text(tr, lang, "account.expires_lbl", "Bitiş tarihi:", "Expires:");
    let no_sub_lbl = tr_text(
        tr,
        lang,
        "account.no_active_sub",
        "Aktif abonelik bulunmuyor",
        "No active subscription",
    );
    let sub_opts_lbl = tr_text(
        tr,
        lang,
        "account.sub_options",
        "Abonelik seçenekleri →",
        "Subscription options →",
    );
    let sub_href = lang_href("/abonelik-ve-satis", lang);
    let id_eyebrow = tr_text(tr, lang, "account.id_eyebrow", "KİMLİK & DOĞRULAMA", "IDENTITY & VERIFICATION");
    let logout_lbl = tr_text(tr, lang, "account.logout", "Oturumu kapat →", "Sign out →");
    let prof_eyebrow = tr_text(tr, lang, "account.prof_eyebrow", "PROFİL & İLGİ ALANLARI", "PROFILE & INTERESTS");
    let prof_title = tr_text(
        tr,
        lang,
        "account.prof_title",
        "Okuyucu Profili ve SSB Taksonomisi",
        "Reader Profile & SSB Taxonomy",
    );
    let kvkk_export_lbl = tr_text(
        tr,
        lang,
        "account.kvkk_export",
        "KVKK Verilerimi İndir (JSON) ↓",
        "Download My KVKK Data (JSON) ↓",
    );
    let full_name_lbl = tr_text(tr, lang, "dialogs.auth.full_name", "Ad soyad", "Full name");
    let job_lbl = tr_text(tr, lang, "dialogs.auth.job_title", "Görev / Ünvan", "Role / Title");
    let job_ph = tr_text(
        tr,
        lang,
        "dialogs.auth.job_placeholder",
        "Örn: Sistem Mühendisi, Proje Subayı",
        "e.g. Systems Engineer, Programme Officer",
    );
    let org_name_lbl = tr_text(tr, lang, "dialogs.auth.org_name", "Kurum adı", "Institution / Company name");
    let org_cat_lbl = tr_text(tr, lang, "account.org_cat", "Kurum kategorisi", "Institution category");
    let ssb_legend = tr_text(
        tr,
        lang,
        "dialogs.auth.ssb_legend",
        "SSB Teknoloji İlgi Alanları (Kişiselleştirilmiş Akış)",
        "SSB Technology Interest Areas (Personalised Feed)",
    );
    let email_opt_lbl = tr_text(
        tr,
        lang,
        "account.email_opt_in",
        "Yeni sayılar, özel dosyalar ve savunma sanayii bültenleri hakkında e-posta ile bilgilendirilmek istiyorum.",
        "I would like to receive email updates about new issues, special dossiers and defence industry briefings.",
    );
    let save_profile_lbl = tr_text(
        tr,
        lang,
        "account.save_profile",
        "Profil ve İlgi Alanlarını Güncelle →",
        "Update Profile & Interests →",
    );
    let active_interests_lbl = tr_text(
        tr,
        lang,
        "account.active_interests",
        "Aktif ilgi alanlarınız",
        "Your active interest areas",
    );
    let explore_topics_lbl = tr_text(
        tr,
        lang,
        "account.explore_topics",
        "Tüm konuları keşfet →",
        "Explore all topics →",
    );
    let no_interests_lbl = tr_text(
        tr,
        lang,
        "account.no_interests",
        "Henüz bir ilgi alanı seçilmedi. Yukarıdaki SSB teknoloji etiketlerinden seçim yapabilirsiniz.",
        "No interest areas selected yet. Choose from the SSB technology tags above.",
    );
    let invoices_heading = tr_text(
        tr,
        lang,
        "account.invoices_heading",
        "Siparişleriniz ve Faturalar",
        "Your Orders & Invoices",
    );
    let th_invoice = tr_text(tr, lang, "account.th_invoice", "Fatura", "Invoice");
    let th_item = tr_text(tr, lang, "account.th_item", "Yayın / plan", "Publication / Plan");
    let th_amount = tr_text(tr, lang, "account.th_amount", "Tutar", "Amount");
    let th_status = tr_text(tr, lang, "account.th_status", "Durum", "Status");
    let th_date = tr_text(tr, lang, "account.th_date", "Tarih", "Date");
    let notices_heading = tr_text(
        tr,
        lang,
        "account.notices_heading",
        "Size özel bildirimler",
        "Your notifications",
    );
    let no_notices_lbl = tr_text(
        tr,
        lang,
        "account.no_notices",
        "Yeni bildiriminiz yok.",
        "You have no new notifications.",
    );

    let kvkk_eyebrow = tr_text(
        tr,
        lang,
        "account.kvkk_eyebrow",
        "KVKK VE SÖZLEŞME UYUMLULUĞU",
        "KVKK & TERMS COMPLIANCE",
    );
    let kvkk_title = tr_text(
        tr,
        lang,
        "account.kvkk_title",
        "Açık Rıza ve Aydınlatma Geçmişi",
        "Consent & Terms Audit History",
    );
    let kvkk_tos_lbl = tr_text(
        tr,
        lang,
        "account.kvkk_tos_version",
        "Onaylanan Kullanım Koşulları Sürümü",
        "Accepted Terms Version",
    );
    let kvkk_accepted_lbl = tr_text(tr, lang, "account.kvkk_accepted", "Onaylandı", "Accepted");
    let kvkk_withdrawn_lbl = tr_text(tr, lang, "account.kvkk_withdrawn", "Geri Çekildi", "Withdrawn");
    let kvkk_rows: String = if kvkk_logs.is_empty() {
        format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            esc(&localize_consent_type("kvkk", is_en)),
            esc(if user.tos_version.is_empty() { "2026-01" } else { &user.tos_version }),
            esc(&kvkk_accepted_lbl),
            esc(&format_portal_date(&user.kvkk_consent_at, is_en))
        )
    } else {
        kvkk_logs
            .iter()
            .take(15)
            .map(|l| {
                let status_txt = if l.accepted {
                    kvkk_accepted_lbl.as_str()
                } else {
                    kvkk_withdrawn_lbl.as_str()
                };
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    esc(&localize_consent_type(&l.consent_type, is_en)),
                    esc(&l.consent_version),
                    esc(status_txt),
                    esc(&format_portal_date(&l.created_at, is_en))
                )
            })
            .collect()
    };

    let del_eyebrow = tr_text(
        tr,
        lang,
        "account.delete_eyebrow",
        "KVKK MADDE 7 · UNUTULMA HAKKI",
        "KVKK / GDPR RIGHT TO ERASURE",
    );
    let del_title = tr_text(
        tr,
        lang,
        "account.delete_title",
        "Hesabımı Kalıcı Olarak Sil",
        "Permanently Delete My Account",
    );
    let del_desc = tr_text(
        tr,
        lang,
        "account.delete_desc",
        "Hesabınızı sildiğinizde kişisel verileriniz anonimleştirilir, oturumunuz kapatılır ve aktif bildirim tercihleriniz temizlenir.",
        "Deleting your account anonymizes your personal data, terminates your active session, and purges non-statutory records.",
    );
    let del_pw_lbl = tr_text(
        tr,
        lang,
        "account.delete_password",
        "Mevcut Şifreniz (doğrulama için)",
        "Current Password (for verification)",
    );
    let del_confirm_lbl = tr_text(
        tr,
        lang,
        "account.delete_confirm",
        "Hesabımın ve kişisel verilerimin KVKK kapsamında kalıcı olarak silinmesini/anonimleştirilmesini onaylıyorum.",
        "I confirm that I want my account and personal data permanently erased/anonymized.",
    );
    let del_btn_lbl = tr_text(
        tr,
        lang,
        "account.delete_submit",
        "Hesabımı Kalıcı Olarak Sil",
        "Delete My Account Permanently",
    );
    let th_kvkk_type = tr_text(tr, lang, "account.kvkk_col_type", "Tür", "Type");
    let th_kvkk_version = tr_text(tr, lang, "account.kvkk_col_version", "Sürüm", "Version");
    let th_kvkk_status = tr_text(tr, lang, "account.kvkk_col_status", "Durum", "Status");
    let th_kvkk_date = tr_text(tr, lang, "account.kvkk_col_date", "Tarih", "Date");

    let personalized_html = if !contents.is_empty() {
        crate::engine::design::components::sections::render_personalized_feed(
            contents,
            &user.ssb_interests,
            tags,
            lang,
            tr,
            6,
        )
    } else {
        String::new()
    };

    let related_html = if !contents.is_empty() {
        crate::engine::design::components::sections::render_related_strip(
            contents,
            &crate::engine::design::components::sections::RelatedStripOptions {
                lang,
                is_current_sponsored: false,
                exclude_ids: &[],
                limit: 4,
                tr,
                source_context: "account",
            },
        )
    } else {
        String::new()
    };

    // Hint alerts (payment=success, verify_email=1, next=...)
    let mut hint_banners = String::new();
    let dismiss_lbl = tr_text(tr, lang, "account.dismiss", "Kapat", "Dismiss");
    if ctx.hints.payment_success {
        let oid_part = ctx
            .hints
            .payment_oid
            .filter(|s| !s.trim().is_empty())
            .map(|o| format!(" (<code>{}</code>)", esc(o)))
            .unwrap_or_default();
        let msg = if is_en {
            format!("Payment received{oid_part}. Your subscription and invoice records have been updated.")
        } else {
            format!("Ödemeniz başarıyla alındı{oid_part}. Abonelik ve fatura kayıtlarınız güncellendi.")
        };
        hint_banners.push_str(&format!(
            r#"<div class="ap-alert-banner is-success" role="status"><div>{msg}</div><button type="button" class="button secondary btn-sm" data-dismiss-alert>{dismiss_lbl}</button></div>"#,
            msg = msg,
            dismiss_lbl = esc(&dismiss_lbl),
        ));
    }
    if ctx.hints.verify_email_prompt && !user.email_verified {
        let msg = tr_text(
            tr,
            lang,
            "account.verify_email_hint",
            "E-Dergi filigranlı PDF indirmelerini açmak için lütfen e-posta adresinizi doğrulayın.",
            "Please verify your email address to unlock full watermarked E-Magazine PDF downloads.",
        );
        hint_banners.push_str(&format!(
            r#"<div class="ap-alert-banner is-warning" role="status"><div>{msg}</div><button type="button" class="button secondary btn-sm" data-dismiss-alert>{dismiss_lbl}</button></div>"#,
            msg = esc(&msg),
            dismiss_lbl = esc(&dismiss_lbl),
        ));
    }
    if let Some(next_raw) = ctx.hints.next_url.filter(|u| safe_url(u) && u.starts_with('/')) {
        let lbl = tr_text(
            tr,
            lang,
            "account.continue_next",
            "İstenen sayfaya devam et →",
            "Continue to requested page →",
        );
        hint_banners.push_str(&format!(
            r#"<div class="ap-alert-banner is-success" role="status"><a href="{href}" class="text-link">{lbl}</a></div>"#,
            href = esc(next_raw),
            lbl = esc(&lbl)
        ));
    }

    // Mobile quick tab rail (unified ARIA tablist + preserved [data-ap-jump] E2E selector)
    let mobile_tabs_html = {
        let items: &[(&str, &str, &str, &str)] = &[
            ("overview", "account.mtab_overview", "Genel Bakış", "Overview"),
            ("edergi", "account.mtab_edergi", "E-Dergi & Abonelik", "E-Magazine"),
            ("profile", "account.mtab_profile", "Profil & SSB", "Profile & SSB"),
            ("security", "account.mtab_security", "Güvenlik & KVKK", "Security"),
            ("notifications", "account.mtab_notifications", "Bildirimler", "Notifications"),
        ];
        let btns: String = items
            .iter()
            .map(|(key, tr_key, l_tr, l_en)| {
                let active = *key == initial_tab;
                let cls = if active { "ap-mobile-tab is-active" } else { "ap-mobile-tab" };
                let lbl = tr_text(tr, lang, tr_key, l_tr, l_en);
                let badge_html = if *key == "notifications" {
                    let hidden_attr = if unread_count == 0 { " hidden" } else { "" };
                    format!(
                        r#"<span class="ap-nav-badge" data-notif-badge data-unread-count="{cnt}"{hidden_attr}>{cnt}</span>"#,
                        cnt = unread_count,
                        hidden_attr = hidden_attr
                    )
                } else {
                    String::new()
                };
                format!(
                    r#"<button type="button" role="tab" id="ap-mtab-{key}" class="{cls}" data-ap-tab="{key}" data-ap-jump="{key}" aria-controls="ap-panel-{key}" aria-selected="{selected}"><span>{lbl}</span>{badge_html}</button>"#,
                    cls = cls,
                    key = key,
                    selected = if active { "true" } else { "false" },
                    lbl = esc(&lbl),
                    badge_html = badge_html
                )
            })
            .collect();
        let tabs_aria = tr_text(tr, lang, "account.section_tabs_aria", "Bölüm sekmeleri", "Section tabs");
        format!(
            r#"<div class="ap-mobile-tabs" role="tablist" aria-label="{aria}">{btns}</div>"#,
            aria = esc(&tabs_aria),
            btns = btns
        )
    };

    let org_cat_display = resolve_org_category_label(&user.organization_category, inst_choices, is_en);
    let has_org_cat = !org_cat_display.trim().is_empty();
    let has_job_title = !user.job_title.trim().is_empty();
    let job_sep = if has_org_cat && has_job_title { " · " } else { "" };

    // KPI tiles
    let kpi_tier_lbl = tr_text(tr, lang, "account.kpi_tier_lbl", "Erişim & Plan", "Access & Plan");
    let kpi_tier_val = if !user.subscription_tier.trim().is_empty() {
        localize_subscription_tier(&user.subscription_tier, is_en)
    } else {
        role_prof.label.to_string()
    };
    let formatted_expires = user
        .subscription_expires_at
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format_portal_date(s, is_en))
        .unwrap_or_else(|| no_sub_lbl.clone());
    let kpi_sec_lbl = tr_text(tr, lang, "account.kpi_sec_lbl", "Güvenlik Durumu", "Security Posture");
    let kpi_sec_val = if user.totp_enabled && user.email_verified {
        tr_text(tr, lang, "account.kpi_sec_both", "2FA + E-posta Doğrulu", "2FA + Email Verified")
    } else if user.totp_enabled {
        tr_text(tr, lang, "account.kpi_sec_totp", "2FA Etkin", "2FA Active")
    } else if user.email_verified {
        tr_text(tr, lang, "account.kpi_sec_email", "E-posta Doğrulu · 2FA Kapalı", "Email Verified · 2FA Off")
    } else {
        tr_text(tr, lang, "account.kpi_sec_needed", "Doğrulama Bekliyor", "Action Needed")
    };
    let kpi_sec_sub = if user.totp_enabled {
        tr_text(tr, lang, "account.kpi_sec_sub_on", "Authenticator koruması aktif", "Authenticator protection enabled")
    } else {
        tr_text(tr, lang, "account.kpi_sec_sub_off", "Güvenlik sekmesinden 2FA açın", "Enable 2FA in Security & KVKK")
    };
    let kpi_feed_lbl = tr_text(tr, lang, "account.kpi_feed_lbl", "SSB Akışı & Bildirim", "SSB Feed & Alerts");

    // Quick Action Cards (Overview)
    let quick_actions_html = format!(
        r#"<div class="ap-quick-grid">
  <button type="button" class="ap-quick-card" data-ap-jump="edergi">
    <span class="ap-quick-icon">{ico_edergi}</span>
    <span class="ap-quick-title">{qa_ed_t}</span>
    <span class="ap-quick-desc">{qa_ed_d}</span>
  </button>
  <button type="button" class="ap-quick-card" data-ap-jump="profile">
    <span class="ap-quick-icon">{ico_prof}</span>
    <span class="ap-quick-title">{qa_pr_t}</span>
    <span class="ap-quick-desc">{qa_pr_d}</span>
  </button>
  <button type="button" class="ap-quick-card" data-ap-jump="security">
    <span class="ap-quick-icon">{ico_sec}</span>
    <span class="ap-quick-title">{qa_sc_t}</span>
    <span class="ap-quick-desc">{qa_sc_d}</span>
  </button>
  <button type="button" class="ap-quick-card" data-ap-jump="notifications">
    <span class="ap-quick-icon">{ico_not}</span>
    <span class="ap-quick-title">{qa_nt_t}</span>
    <span class="ap-quick-desc">{qa_nt_d}</span>
  </button>
</div>"#,
        ico_edergi = portal_svg("edergi"),
        ico_prof = portal_svg("profile"),
        ico_sec = portal_svg("security"),
        ico_not = portal_svg("notifications"),
        qa_ed_t = esc(&tr_text(tr, lang, "account.qa_ed_t", "E-Dergi Rafı", "E-Magazine Shelf")),
        qa_ed_d = esc(&tr_text(tr, lang, "account.qa_ed_d", "Sayılar, filigranlı PDF ve faturalar", "Browse issues, PDFs & invoices")),
        qa_pr_t = esc(&tr_text(tr, lang, "account.qa_pr_t", "SSB İlgi Alanları", "SSB Interest Tags")),
        qa_pr_d = esc(&tr_text(tr, lang, "account.qa_pr_d", "Savunma akışınızı özelleştirin", "Personalise your defence feed")),
        qa_sc_t = esc(&tr_text(tr, lang, "account.qa_sc_t", "2FA & KVKK Gizlilik", "2FA & KVKK Privacy")),
        qa_sc_d = esc(&tr_text(tr, lang, "account.qa_sc_d", "İki aşamalı doğrulama ve veri dışa aktarım", "Authenticator, consent & export")),
        qa_nt_t = esc(&tr_text(tr, lang, "account.qa_nt_t", "Bildirimler", "Notifications")),
        qa_nt_d = esc(&tr_text(tr, lang, "account.qa_nt_d", "Hesap ve yayın bildirimlerini yönetin", "Review account & editorial alerts")),
    );

    // E-Dergi Issue Shelf (Section 2)
    let edergi_shelf_html = render_edergi_shelf(ctx, is_en, lang);
    let purchased_html = render_purchased_library(ctx, is_en);

    // Corporate Verification Card (Section 3)
    let corp_status_html = render_corporate_status_card(ctx, inst_choices, is_en);

    // Notifications Mark-All button & list (Section 5)
    let mark_all_lbl = tr_text(tr, lang, "account.mark_all_read", "Tümünü Okundu İşaretle", "Mark All Read");
    let mark_all_hidden = if unread_count == 0 { " hidden" } else { "" };

    let hidden_overview = if initial_tab == "overview" { "" } else { " hidden" };
    let hidden_edergi = if initial_tab == "edergi" { "" } else { " hidden" };
    let hidden_profile = if initial_tab == "profile" { "" } else { " hidden" };
    let hidden_security = if initial_tab == "security" { "" } else { " hidden" };
    let hidden_notifications = if initial_tab == "notifications" { "" } else { " hidden" };

    let ssb_toolbar_select = tr_text(tr, lang, "account.ssb_select_all", "Tümünü Seç", "Select All");
    let ssb_toolbar_clear = tr_text(tr, lang, "account.ssb_clear", "Temizle", "Clear");
    let total_ssb = if !tags.is_empty() {
        tags.len()
    } else {
        CANONICAL_SSB_TAG_CODES.len()
    };
    let ssb_selected_lbl = if is_en {
        format!("{} / {} selected", user.ssb_interests.len(), total_ssb)
    } else {
        format!("{} / {} seçili", user.ssb_interests.len(), total_ssb)
    };

    let del_modal_title = tr_text(tr, lang, "account.del_modal_title", "Kalıcı Hesap Silmeyi Onaylayın", "Confirm Permanent Account Deletion");
    let del_modal_body = tr_text(
        tr,
        lang,
        "account.del_modal_body",
        "Bu işlem profilinizi kalıcı olarak anonimleştirir, oturumunuzu sonlandırır ve geri alınamaz. Devam etmek istediğinizden emin misiniz?",
        "This action permanently anonymizes your profile, terminates your session, and cannot be undone. Are you sure you wish to proceed?",
    );
    let del_modal_cancel = tr_text(tr, lang, "account.del_modal_cancel", "Vazgeç", "Cancel");
    let del_modal_confirm = tr_text(tr, lang, "account.del_modal_confirm", "Evet, Kalıcı Olarak Sil", "Yes, Delete Permanently");
    let del_modal_close = tr_text(tr, lang, "portal.close", "Kapat", "Close");
    let export_filename = if is_en {
        "msi-account-data.json"
    } else {
        "msi-hesap-verilerim.json"
    };
    let notices_eyebrow = tr_text(tr, lang, "account.notices_eyebrow", "BİLDİRİMLER", "NOTIFICATIONS");

    let sec_edergi_title = tr_text(tr, lang, "account.sec_edergi_title", "E-Dergi & Abonelik Merkezi", "E-Magazine & Subscription Hub");
    let sec_edergi_desc = tr_text(
        tr,
        lang,
        "account.sec_edergi_desc",
        "Dijital E-Dergi sayılarını okuyun, filigranlı PDF indirin ve fatura geçmişinizi inceleyin.",
        "Read digital issues, download watermarked subscriber PDFs, and review invoices.",
    );
    let sec_prof_title = tr_text(tr, lang, "account.sec_prof_title", "Profil & SSB Teknoloji İlgi Alanları", "Profile & SSB Technology Interests");
    let sec_prof_desc = tr_text(
        tr,
        lang,
        "account.sec_prof_desc",
        "Kurumsal bilgilerinizi ve kişiselleştirilmiş savunma teknolojisi etiketlerinizi güncelleyin.",
        "Keep your institutional details and personalised defence technology topics up to date.",
    );
    let sec_sec_title = tr_text(tr, lang, "account.sec_sec_title", "Güvenlik, 2FA & KVKK Gizlilik", "Security, 2FA & KVKK Privacy");
    let sec_sec_desc = tr_text(
        tr,
        lang,
        "account.sec_sec_desc",
        "Hesabınızı İki Aşamalı Doğrulama ile koruyun, KVKK rıza kayıtlarını inceleyin veya verilerinizi indirin.",
        "Protect your account with Two-Factor Authentication, inspect KVKK consent logs, or export data.",
    );
    let sec_notif_desc = tr_text(
        tr,
        lang,
        "account.sec_notif_desc",
        "Hesap durumu güncellemeleri, abonelik hatırlatmaları ve özel yayın bildirimleri.",
        "Account status updates, renewal reminders, and curated intelligence alerts.",
    );
    let security_posture_title = tr_text(tr, lang, "account.sec_posture_title", "Hesap Doğrulama ve Şifre Güvenliği", "Account Verification & Password Access");
    let security_posture_desc = tr_text(
        tr,
        lang,
        "account.sec_posture_desc",
        "E-posta doğrulaması filigranlı E-Dergi indirmelerini ve güvenlik bildirimlerini korur. Şifrenizi dilediğiniz zaman şifre sıfırlama akışı üzerinden güncelleyebilirsiniz.",
        "Email verification protects watermarked E-Magazine downloads and security alerts. You can reset your password anytime via the sign-in recovery flow.",
    );
    let role_pill_cls = format!("ap-role-pill is-{}", role_prof.tone);
    let identity_fallback = tr_text(tr, lang, "account.reader_identity", "Okuyucu Kimliği", "Reader Identity");
    let identity_heading = if !user.organization_name.trim().is_empty() {
        user.organization_name.as_str()
    } else if !user.full_name.trim().is_empty() {
        user.full_name.as_str()
    } else {
        identity_fallback.as_str()
    };
    let org_lock_attr = if is_institution_locked { " disabled" } else { "" };
    let org_lock_note = if is_institution_locked {
        let note_txt = tr_text(
            tr,
            lang,
            "account.org_lock_note",
            "Doğrulanmış kurum bilgileriniz kilitlidir. Kurum değişikliği için destek ekibimizle iletişime geçin.",
            "Verified institutional details are locked. Contact support if your organisation changes.",
        );
        format!(r#"<p class="field-help">{}</p>"#, esc(&note_txt))
    } else {
        String::new()
    };

    format!(
        r#"<div class="ap-workspace-inner">
  {mobile_tabs_html}
  <section class="ap-hero">
    <div class="ap-hero-top">
      <div class="ap-hero-copy">
        {heading_html}
      </div>
      <div class="ap-hero-badges">
        <span class="{role_pill_cls}">{role_label}</span>
        {gov_badge}
      </div>
    </div>
    <p>{role_summary}</p>
    <div class="ap-kpi-grid">
      <div class="ap-kpi-tile">
        <span class="ap-kpi-label">{kpi_tier_lbl}</span>
        <strong class="ap-kpi-value">{kpi_tier_val}</strong>
        <span class="ap-kpi-sub">{expires_lbl} {kpi_expires}</span>
      </div>
      <div class="ap-kpi-tile">
        <span class="ap-kpi-label">{kpi_sec_lbl}</span>
        <strong class="ap-kpi-value">{kpi_sec_val}</strong>
        <span class="ap-kpi-sub">{kpi_sec_sub}</span>
      </div>
      <div class="ap-kpi-tile">
        <span class="ap-kpi-label">{kpi_feed_lbl}</span>
        <strong class="ap-kpi-value">{ssb_count} SSB · <span id="ap-kpi-unread">{unread_count}</span> {unread_word}</strong>
        <span class="ap-kpi-sub">{email}</span>
      </div>
    </div>
  </section>

  {hint_banners}

  <section id="ap-panel-overview" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-overview" data-ap-panel="overview"{hidden_overview}>
    {command_card}
    {quick_actions_html}
    <div class="ap-grid-2">
      <article class="account-panel">
        <div class="eyebrow">{id_eyebrow}</div>
        <h2>{identity_heading}</h2>
        <p class="account-identity-meta"><span>{email}</span><span>{org_cat}{job_sep}{job_title}</span></p>
        <div class="account-badges-row">{verification_html}{gov_badge}</div>
        <div class="account-logout-row"><button type="button" class="text-link" data-logout>{logout_lbl}</button></div>
      </article>
      <article class="account-panel">
        <div class="panel-header-row">
          <div>
            <div class="eyebrow">SSB</div>
            <h2>{active_interests_lbl}</h2>
          </div>
          <a class="text-link" href="{search_base}">{explore_topics_lbl}</a>
        </div>
        <div class="topic-chips">{interests_block}</div>
      </article>
    </div>
    {personalized_html}
    {related_html}
  </section>

  <section id="ap-panel-edergi" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-edergi" data-ap-panel="edergi"{hidden_edergi}>
    <div class="ap-panel-head">
      <div>
        <div class="eyebrow">{sub_eyebrow}</div>
        <h2>{sec_edergi_title}</h2>
        <p>{sec_edergi_desc}</p>
      </div>
      <a href="{sub_href}" class="button primary">{sub_opts_lbl}</a>
    </div>
    <article class="account-panel">
      <div class="panel-header-row">
        <div>
          <div class="eyebrow">{sub_eyebrow}</div>
          <h2>{tier}</h2>
        </div>
        <span class="{role_pill_cls}">{role_label}</span>
      </div>
      <p>{expires_lbl} <strong>{expires}</strong></p>
      <p>{reminder_text}</p>
    </article>
    {edergi_shelf_html}
    {purchased_html}
    <article class="account-panel">
      <h2>{invoices_heading}</h2>
      <div class="table-responsive"><table class="table" id="account-invoices"><thead><tr><th scope="col">{th_invoice}</th><th scope="col">{th_item}</th><th scope="col">{th_amount}</th><th scope="col">{th_status}</th><th scope="col">{th_date}</th></tr></thead><tbody>{invoice_rows}{empty_invoices}</tbody></table></div>
    </article>
  </section>

  <section id="ap-panel-profile" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-profile" data-ap-panel="profile"{hidden_profile}>
    <div class="ap-panel-head">
      <div>
        <div class="eyebrow">{prof_eyebrow}</div>
        <h2>{sec_prof_title}</h2>
        <p>{sec_prof_desc}</p>
      </div>
    </div>
    {corp_status_html}
    <article class="account-panel" id="account-profile-card">
      <div class="panel-header-row">
        <div><div class="eyebrow">{prof_eyebrow}</div><h2>{prof_title}</h2></div>
      </div>
      <form id="account-profile-form" class="ui-form">
        <div class="form-grid-2">
          <label>{full_name_lbl}<input name="full_name" value="{full_name}" autocomplete="name" maxlength="100" required></label>
          <label>{job_lbl}<input name="job_title" value="{job_title}" autocomplete="organization-title" maxlength="100" placeholder="{job_ph}"></label>
        </div>
        <div class="form-grid-2">
          <label>{org_name_lbl}<input name="organization_name" value="{org_name}" autocomplete="organization" maxlength="120" required{org_lock_attr}></label>
          <label>{org_cat_lbl}<select name="organization_category"{org_lock_attr}>{org_options}</select></label>
        </div>
        {org_lock_note}
        <fieldset class="ssb-fieldset">
          <legend>{ssb_legend}</legend>
          <div class="ap-ssb-toolbar">
            <span class="ap-ssb-count" id="ap-ssb-count">{ssb_selected_lbl}</span>
            <div class="ap-ssb-actions">
              <button type="button" class="button secondary btn-sm" data-ssb-select-all>{ssb_toolbar_select}</button>
              <button type="button" class="button secondary btn-sm" data-ssb-clear-all>{ssb_toolbar_clear}</button>
            </div>
          </div>
          <div id="account-ssb-tags" class="ssb-tag-picks">{ssb_checkboxes}</div>
        </fieldset>
        <label class="check-label"><input type="checkbox" name="email_consent"{email_opt_checked}><span>{email_opt_lbl}</span></label>
        <p class="form-feedback" role="status"></p>
        <div class="account-form-actions"><button type="submit" class="button primary">{save_profile_lbl}</button></div>
      </form>
    </article>
  </section>

  <section id="ap-panel-security" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-security" data-ap-panel="security"{hidden_security}>
    <div class="ap-panel-head">
      <div>
        <div class="eyebrow">{totp_eyebrow}</div>
        <h2>{sec_sec_title}</h2>
        <p>{sec_sec_desc}</p>
      </div>
    </div>
    <article class="account-panel">
      <div class="eyebrow">{id_eyebrow}</div>
      <h2>{security_posture_title}</h2>
      <p>{security_posture_desc}</p>
      <div class="account-badges-row">{verification_html}{gov_badge}</div>
    </article>
    {totp_card_html}
    <article class="account-panel" id="account-kvkk-card">
      <div class="panel-header-row">
        <div><div class="eyebrow">{kvkk_eyebrow}</div><h2>{kvkk_title}</h2></div>
        <a href="/api/v1/account/export" class="button secondary btn-sm" download="{export_filename}" data-kvkk-export>{kvkk_export_lbl}</a>
      </div>
      <p>{kvkk_tos_lbl}: <strong>{tos_version}</strong></p>
      <div class="table-responsive"><table class="table"><thead><tr><th scope="col">{th_kvkk_type}</th><th scope="col">{th_kvkk_version}</th><th scope="col">{th_kvkk_status}</th><th scope="col">{th_kvkk_date}</th></tr></thead><tbody>{kvkk_rows}</tbody></table></div>
    </article>
    <article class="account-panel account-danger-zone" id="account-delete-card">
      <div class="eyebrow">{del_eyebrow}</div>
      <h2>{del_title}</h2>
      <p>{del_desc}</p>
      <form id="account-delete-form" class="ui-form">
        <label>{del_pw_lbl}<input type="password" name="password" autocomplete="current-password" placeholder="••••••••" required></label>
        <label class="check-label"><input type="checkbox" name="confirm_erasure" required><span>{del_confirm_lbl}</span></label>
        <p class="form-feedback" role="status"></p>
        <div class="account-form-actions"><button type="submit" class="button danger-outline">{del_btn_lbl}</button></div>
      </form>
    </article>
  </section>

  <section id="ap-panel-notifications" class="portal-section" role="tabpanel" aria-labelledby="ap-tab-notifications" data-ap-panel="notifications"{hidden_notifications}>
    <div class="ap-panel-head">
      <div>
        <div class="eyebrow">{notices_eyebrow}</div>
        <h2>{notices_heading}</h2>
        <p>{sec_notif_desc}</p>
      </div>
      <button type="button" class="button secondary btn-sm" data-mark-all-read{mark_all_hidden}>{mark_all_lbl}</button>
    </div>
    <div class="ap-notif-list">
      {notices_block}
    </div>
  </section>

  <dialog id="account-delete-confirm-dialog" class="ap-dialog" aria-labelledby="ap-del-modal-title" aria-describedby="ap-del-modal-body" aria-modal="true">
    <div class="ap-dialog-head">
      <h2 id="ap-del-modal-title">{del_modal_title}</h2>
      <button type="button" class="button ghost ap-dialog-close" id="account-delete-close-btn" aria-label="{del_modal_close}">×</button>
    </div>
    <p id="ap-del-modal-body" class="ap-dialog-body">{del_modal_body}</p>
    <div class="ap-dialog-actions">
      <button type="button" class="button secondary" id="account-delete-cancel-btn">{del_modal_cancel}</button>
      <button type="button" class="button danger-solid" id="account-delete-confirm-btn">{del_modal_confirm}</button>
    </div>
  </dialog>
</div>"#,
        heading_html = heading_html,
        role_pill_cls = role_pill_cls,
        role_label = esc(role_prof.label),
        gov_badge = gov_badge,
        role_summary = esc(role_prof.summary),
        kpi_tier_lbl = esc(&kpi_tier_lbl),
        kpi_tier_val = esc(&kpi_tier_val),
        expires_lbl = esc(&expires_lbl),
        kpi_expires = esc(&formatted_expires),
        kpi_sec_lbl = esc(&kpi_sec_lbl),
        kpi_sec_val = esc(&kpi_sec_val),
        kpi_sec_sub = esc(&kpi_sec_sub),
        kpi_feed_lbl = esc(&kpi_feed_lbl),
        ssb_count = user.ssb_interests.len(),
        unread_count = unread_count,
        unread_word = esc(&tr_text(tr, lang, "account.unread_word", "Okunmamış", "Unread")),
        email = esc(&user.email),
        hint_banners = hint_banners,
        mobile_tabs_html = mobile_tabs_html,
        hidden_overview = hidden_overview,
        hidden_edergi = hidden_edergi,
        hidden_profile = hidden_profile,
        hidden_security = hidden_security,
        hidden_notifications = hidden_notifications,
        command_card = command_card,
        quick_actions_html = quick_actions_html,
        id_eyebrow = esc(&id_eyebrow),
        identity_heading = esc(identity_heading),
        org_name = esc(&user.organization_name),
        org_cat = esc(&org_cat_display),
        job_sep = job_sep,
        job_title = esc(user.job_title.trim()),
        verification_html = verification_html,
        logout_lbl = esc(&logout_lbl),
        active_interests_lbl = esc(&active_interests_lbl),
        search_base = search_base,
        explore_topics_lbl = esc(&explore_topics_lbl),
        interests_block = if interests.is_empty() {
            format!("<p>{}</p>", esc(&no_interests_lbl))
        } else {
            interests
        },
        personalized_html = personalized_html,
        related_html = related_html,
        sub_eyebrow = esc(&sub_eyebrow),
        sec_edergi_title = esc(&sec_edergi_title),
        sec_edergi_desc = esc(&sec_edergi_desc),
        sub_href = esc(&sub_href),
        sub_opts_lbl = esc(&sub_opts_lbl),
        tier = esc(&kpi_tier_val),
        expires = esc(&formatted_expires),
        reminder_text = esc(&reminder_text),
        edergi_shelf_html = edergi_shelf_html,
        purchased_html = purchased_html,
        invoices_heading = esc(&invoices_heading),
        th_invoice = esc(&th_invoice),
        th_item = esc(&th_item),
        th_amount = esc(&th_amount),
        th_status = esc(&th_status),
        th_date = esc(&th_date),
        invoice_rows = invoice_rows,
        empty_invoices = empty_invoices,
        prof_eyebrow = esc(&prof_eyebrow),
        sec_prof_title = esc(&sec_prof_title),
        sec_prof_desc = esc(&sec_prof_desc),
        corp_status_html = corp_status_html,
        prof_title = esc(&prof_title),
        full_name_lbl = esc(&full_name_lbl),
        full_name = esc(&user.full_name),
        job_lbl = esc(&job_lbl),
        job_ph = esc(&job_ph),
        org_name_lbl = esc(&org_name_lbl),
        org_cat_lbl = esc(&org_cat_lbl),
        org_options = org_options,
        org_lock_attr = org_lock_attr,
        org_lock_note = org_lock_note,
        ssb_legend = esc(&ssb_legend),
        ssb_selected_lbl = esc(&ssb_selected_lbl),
        ssb_toolbar_select = esc(&ssb_toolbar_select),
        ssb_toolbar_clear = esc(&ssb_toolbar_clear),
        ssb_checkboxes = ssb_checkboxes,
        email_opt_checked = email_opt_checked,
        email_opt_lbl = esc(&email_opt_lbl),
        save_profile_lbl = esc(&save_profile_lbl),
        totp_eyebrow = esc(&totp_eyebrow),
        sec_sec_title = esc(&sec_sec_title),
        sec_sec_desc = esc(&sec_sec_desc),
        security_posture_title = esc(&security_posture_title),
        security_posture_desc = esc(&security_posture_desc),
        totp_card_html = totp_card_html,
        kvkk_eyebrow = esc(&kvkk_eyebrow),
        kvkk_title = esc(&kvkk_title),
        export_filename = export_filename,
        kvkk_export_lbl = esc(&kvkk_export_lbl),
        kvkk_tos_lbl = esc(&kvkk_tos_lbl),
        tos_version = esc(if user.tos_version.is_empty() { "2026-01" } else { &user.tos_version }),
        th_kvkk_type = esc(&th_kvkk_type),
        th_kvkk_version = esc(&th_kvkk_version),
        th_kvkk_status = esc(&th_kvkk_status),
        th_kvkk_date = esc(&th_kvkk_date),
        kvkk_rows = kvkk_rows,
        del_eyebrow = esc(&del_eyebrow),
        del_title = esc(&del_title),
        del_desc = esc(&del_desc),
        del_pw_lbl = esc(&del_pw_lbl),
        del_confirm_lbl = esc(&del_confirm_lbl),
        del_btn_lbl = esc(&del_btn_lbl),
        del_modal_title = esc(&del_modal_title),
        del_modal_close = esc(&del_modal_close),
        del_modal_body = esc(&del_modal_body),
        del_modal_cancel = esc(&del_modal_cancel),
        del_modal_confirm = esc(&del_modal_confirm),
        notices_eyebrow = esc(&notices_eyebrow),
        notices_heading = esc(&notices_heading),
        sec_notif_desc = esc(&sec_notif_desc),
        mark_all_hidden = mark_all_hidden,
        mark_all_lbl = esc(&mark_all_lbl),
        notices_block = if notices_html.is_empty() {
            empty_state(&esc(&no_notices_lbl))
        } else {
            notices_html
        },
    )
}

fn render_edergi_shelf(ctx: &AccountPortalContext<'_>, is_en: bool, lang: &str) -> String {
    let tr = ctx.tr;
    let title = tr_text(tr, lang, "account.edergi_shelf_title", "Dijital E-Dergi Rafı", "Digital E-Magazine Shelf");
    let eyebrow = tr_text(tr, lang, "account.edergi_shelf_eyebrow", "SAYI KÜTÜPHANESİ", "ISSUE LIBRARY");
    let kiosk_href = lang_href("/e-dergi", lang);
    let all_issues_lbl = tr_text(tr, lang, "account.edergi_all_issues", "Tüm sayılar →", "All issues →");

    if ctx.issues.is_empty() {
        let empty_lbl = tr_text(
            tr,
            lang,
            "account.edergi_empty",
            "Henüz yayınlanmış E-Dergi sayısı bulunmuyor.",
            "No published E-Magazine issues available yet.",
        );
        return format!(
            r#"<article class="account-panel"><div class="panel-header-row"><div><div class="eyebrow">{eyebrow}</div><h2>{title}</h2></div><a href="{kiosk_href}" class="text-link">{all_issues_lbl}</a></div>{empty}</article>"#,
            eyebrow = esc(&eyebrow),
            title = esc(&title),
            kiosk_href = esc(&kiosk_href),
            all_issues_lbl = esc(&all_issues_lbl),
            empty = empty_state(&esc(&empty_lbl)),
        );
    }

    let sub_href = lang_href("/abonelik-ve-satis", lang);
    let open_flip_lbl = tr_text(tr, lang, "account.edergi_open_flip", "Flipbook'ta Aç →", "Open Flipbook →");
    let preview_lbl = tr_text(tr, lang, "account.edergi_preview", "İçindekileri İncele →", "Preview Contents →");
    let pdf_lbl = tr_text(tr, lang, "account.edergi_pdf", "Filigranlı PDF ↓", "Watermarked PDF ↓");
    let verify_lbl = tr_text(tr, lang, "account.edergi_verify", "PDF için E-posta Doğrula", "Verify Email to Download");
    let upg_lbl = tr_text(tr, lang, "account.edergi_unlock", "PDF Erişimini Aç", "Unlock PDF Access");
    let cards: String = ctx
        .issues
        .iter()
        .take(8)
        .map(|issue| {
            let issue_title = localized(lang, &issue.title_tr, &issue.title_en);
            let issue_month = localized(lang, &issue.publish_month_tr, &issue.publish_month_en);
            let pub_badge = if issue.publication_type.eq_ignore_ascii_case("msi_tdr") {
                "MSI TDR"
            } else {
                "MSI DERGİSİ"
            };
            let cover = if !issue.cover_image.trim().is_empty() && safe_url(&issue.cover_image) {
                if issue.cover_image.starts_with("https://") {
                    issue.cover_image.clone()
                } else {
                    format!("/{}", issue.cover_image.trim_start_matches('/'))
                }
            } else {
                "/images/logo.svg".to_string()
            };
            let pub_type = if issue.publication_type.trim().eq_ignore_ascii_case("msi_tdr") {
                "msi_tdr"
            } else {
                "msi"
            };
            let mut raw_open = format!(
                "/e-dergi?issue={}&publication_type={}",
                issue.issue_number,
                urlencoding::encode(pub_type)
            );
            if ctx.can_download_edergi {
                raw_open.push_str("&view=flipbook");
            }
            let open_href = lang_href(&raw_open, lang);
            let open_lbl = if ctx.can_download_edergi {
                open_flip_lbl.as_str()
            } else {
                preview_lbl.as_str()
            };
            let pdf_btn = if ctx.can_download_edergi {
                let pdf_href = format!(
                    "/api/v1/edergi/{}/download-pdf?publication_type={}&pages=all",
                    issue.issue_number,
                    urlencoding::encode(&issue.publication_type)
                );
                format!(
                    r#"<a href="{href}" class="button primary btn-sm">{lbl}</a>"#,
                    href = esc(&pdf_href),
                    lbl = esc(&pdf_lbl)
                )
            } else if ctx.has_subscriber_access && !ctx.user.email_verified {
                format!(
                    r#"<button type="button" class="button secondary btn-sm" data-ap-jump="security">{lbl}</button>"#,
                    lbl = esc(&verify_lbl)
                )
            } else {
                format!(
                    r#"<a href="{href}" class="button secondary btn-sm">{lbl}</a>"#,
                    href = esc(&sub_href),
                    lbl = esc(&upg_lbl)
                )
            };
            let pages_lbl = if is_en {
                format!("{} pages", issue.page_count.max(0))
            } else {
                format!("{} sayfa", issue.page_count.max(0))
            };
            format!(
                r#"<article class="ap-issue-card">
  <div class="ap-issue-cover">
    <img src="{cover}" alt="{title}" loading="lazy">
    <span class="ap-issue-pub-badge">{pub_badge}</span>
  </div>
  <div class="ap-issue-body">
    <div class="ap-issue-meta">#{num} · {month} {year} · {pages}</div>
    <h3 class="ap-issue-title">{title}</h3>
    <div class="ap-issue-actions">
      <a href="{open_href}" class="button secondary btn-sm">{open_lbl}</a>
      {pdf_btn}
    </div>
  </div>
</article>"#,
                cover = esc(&cover),
                title = esc(issue_title),
                pub_badge = esc(pub_badge),
                num = issue.issue_number,
                month = esc(issue_month),
                year = issue.year,
                pages = esc(&pages_lbl),
                open_href = esc(&open_href),
                open_lbl = esc(open_lbl),
                pdf_btn = pdf_btn,
            )
        })
        .collect();

    format!(
        r#"<article class="account-panel"><div class="panel-header-row"><div><div class="eyebrow">{eyebrow}</div><h2>{title}</h2></div><a href="{kiosk_href}" class="text-link">{all_issues_lbl}</a></div><div class="ap-edergi-shelf">{cards}</div></article>"#,
        eyebrow = esc(&eyebrow),
        title = esc(&title),
        kiosk_href = esc(&kiosk_href),
        all_issues_lbl = esc(&all_issues_lbl),
        cards = cards
    )
}

fn render_purchased_library(ctx: &AccountPortalContext<'_>, _is_en: bool) -> String {
    if ctx.purchased_codes.is_empty() {
        return String::new();
    }
    let tr = ctx.tr;
    let lang = ctx.lang;
    let eyebrow = tr_text(tr, lang, "account.purchased_eyebrow", "DİJİTAL KÜTÜPHANE", "DIGITAL LIBRARY");
    let title = tr_text(tr, lang, "account.purchased_title", "Satın Alınan Rapor ve Yayınlar", "Purchased Reports & Assets");
    let dl_lbl = tr_text(tr, lang, "account.purchased_dl", "İndir ↓", "Download ↓");
    let rows: String = ctx
        .purchased_codes
        .iter()
        .map(|sku| {
            let href = format!("/api/v1/commerce/download/{}", urlencoding::encode(sku));
            format!(
                r#"<div class="notification-item"><div class="notification-item-top"><strong><code>{sku}</code></strong><a href="{href}" class="button secondary btn-sm">{dl_lbl}</a></div></div>"#,
                sku = esc(sku),
                href = esc(&href),
                dl_lbl = esc(&dl_lbl)
            )
        })
        .collect();

    format!(
        r#"<article class="account-panel"><div class="eyebrow">{eyebrow}</div><h2>{title}</h2><div class="ap-notif-list">{rows}</div></article>"#,
        eyebrow = esc(&eyebrow),
        title = esc(&title),
        rows = rows
    )
}

fn render_corporate_status_card(
    ctx: &AccountPortalContext<'_>,
    inst_choices: &[InstitutionChoice],
    is_en: bool,
) -> String {
    let tr = ctx.tr;
    let lang = ctx.lang;
    let Some(latest) = ctx.corporate_requests.first() else {
        if !ctx.user.is_gov_verified {
            return String::new();
        }
        let eyebrow = tr_text(tr, lang, "account.corp_verified_eyebrow", "KURUMSAL DOĞRULAMA", "INSTITUTIONAL VERIFICATION");
        let title = tr_text(tr, lang, "account.corp_verified_title", "Kamu / Akademik Doğrulama Aktif", "Public / Academic Status Active");
        let desc = tr_text(
            tr,
            lang,
            "account.corp_verified_desc",
            "Kurumsal e-posta alan adınız kamu sektörü E-Dergi erişimi için doğrulanmıştır.",
            "Your institutional domain has been verified for public-sector E-Magazine privileges.",
        );
        let badge = tr_text(tr, lang, "account.corp_verified_badge", "Doğrulandı", "Verified");
        return format!(
            r#"<article class="account-panel"><div class="panel-header-row"><div><div class="eyebrow">{eyebrow}</div><h2>{title}</h2></div><span class="status-badge is-gov">✓ {badge}</span></div><p>{desc}</p></article>"#,
            eyebrow = esc(&eyebrow),
            title = esc(&title),
            badge = esc(&badge),
            desc = esc(&desc),
        );
    };

    let status_norm = latest.status.trim().to_ascii_lowercase();
    let (badge_cls, status_lbl) = match status_norm.as_str() {
        "approved" => ("is-verified", tr_text(tr, lang, "account.corp_approved", "Onaylandı", "Approved")),
        "rejected" => ("is-danger", tr_text(tr, lang, "account.corp_rejected", "Reddedildi", "Rejected")),
        _ => ("is-pending", tr_text(tr, lang, "account.corp_pending", "İnceleme Bekliyor", "Pending Review")),
    };
    let eyebrow = tr_text(tr, lang, "account.corp_req_eyebrow", "KURUMSAL ERİŞİM TALEBİ", "CORPORATE ACCESS REQUEST");
    let title = tr_text(tr, lang, "account.corp_req_title", "Kurumsal Doğrulama Durumu", "Institutional Verification Status");
    let submitted_lbl = tr_text(tr, lang, "account.corp_submitted", "Başvuru tarihi:", "Submitted:");
    let cat_display = resolve_org_category_label(&latest.organization_category, inst_choices, is_en);
    let formatted_date = format_portal_date(&latest.created_at, is_en);
    let org_html = if cat_display.trim().is_empty() {
        format!("<strong>{}</strong>", esc(&latest.organization_name))
    } else {
        format!(
            "<strong>{}</strong> ({})",
            esc(&latest.organization_name),
            esc(&cat_display)
        )
    };

    format!(
        r#"<article class="account-panel"><div class="panel-header-row"><div><div class="eyebrow">{eyebrow}</div><h2>{title}</h2></div><span class="status-badge {badge_cls}">{status_lbl}</span></div><p>{org_html} · {submitted_lbl} <code>{date}</code></p></article>"#,
        eyebrow = esc(&eyebrow),
        title = esc(&title),
        badge_cls = badge_cls,
        status_lbl = esc(&status_lbl),
        org_html = org_html,
        submitted_lbl = esc(&submitted_lbl),
        date = esc(&formatted_date),
    )
}

fn render_portal_notification_card(
    n: &SystemNotification,
    lang: &str,
    tr: Option<&TranslationCache>,
) -> String {
    let is_en = lang.eq_ignore_ascii_case("en");
    let view_lbl = tr_text(tr, lang, "account.notif_view", "İncele →", "View →");
    let action = n
        .action_url
        .as_ref()
        .filter(|u| safe_url(u))
        .map(|u| format!(r#"<a class="text-link" href="{}">{}</a>"#, esc(u), esc(&view_lbl)))
        .unwrap_or_default();

    let state_cls = if n.is_read { "is-read" } else { "is-unread" };
    let badge_cls = if n.is_read { "status-badge is-muted" } else { "status-badge is-pending" };
    let badge_lbl = if n.is_read {
        tr_text(tr, lang, "portal.notif_status_read", "Okundu", "Read")
    } else {
        tr_text(tr, lang, "account.notif_status_unread", "Okunmadı", "Unread")
    };
    let mark_btn = if !n.is_read {
        let lbl = tr_text(tr, lang, "account.notif_mark_read", "Okundu işaretle", "Mark as read");
        format!(
            r#"<button type="button" class="button secondary btn-sm" data-mark-read="{}">{}</button>"#,
            n.id,
            esc(&lbl)
        )
    } else {
        String::new()
    };
    let formatted_date = format_portal_date(&n.created_at, is_en);

    format!(
        r#"<article class="notification-item {state_cls}" data-notification-id="{id}" data-is-read="{is_read}">
  <div class="notification-item-top">
    <time>{date}</time>
    <span class="{badge_cls}" data-notif-state>{badge_lbl}</span>
  </div>
  <h3>{title}</h3>
  <p>{msg}</p>
  <div class="notification-item-actions">{action}{mark_btn}</div>
</article>"#,
        state_cls = state_cls,
        id = n.id,
        is_read = if n.is_read { "true" } else { "false" },
        date = esc(&formatted_date),
        badge_cls = badge_cls,
        badge_lbl = esc(&badge_lbl),
        title = esc(localized(lang, &n.title_tr, &n.title_en)),
        msg = esc(localized(lang, &n.message_tr, &n.message_en)),
        action = action,
        mark_btn = mark_btn,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(role: &str, verified: bool, totp: bool) -> UserAccount {
        UserAccount {
            id: 42,
            email: "okur@savunma.gov.tr".into(),
            full_name: "Test Kullanıcısı".into(),
            role: role.into(),
            organization_category: "SSB".into(),
            organization_name: "Savunma Sanayii Başkanlığı".into(),
            job_title: "Kıdemli Uzman".into(),
            is_gov_verified: true,
            subscription_tier: "Kurumsal Abone".into(),
            subscription_expires_at: Some("2027-12-31".into()),
            reminders_halted: false,
            kvkk_consent_at: "2026-01-01".into(),
            kvkk_consent_ip: "127.0.0.1".into(),
            kvkk_consent_hash: "abc".into(),
            email_consent: true,
            ssb_interests: vec!["hava-araclari".into()],
            created_at: "2026-01-01".into(),
            email_verified: verified,
            marketing_emails_opted_in: true,
            totp_enabled: totp,
            tos_accepted_at: "2026-01-01T00:00:00Z".into(),
            tos_version: "2026-01".into(),
        }
    }

    #[test]
    fn staff_sees_command_center_while_reader_never_sees_admin_path() {
        let admin_user = sample_user("admin", true, true);
        let staff_html =
            render_with_admin_path("tr", &admin_user, &[], &[], &[], &[], Some("podmin"));
        assert!(staff_html.contains("Yönetim Komuta Merkezi →"));
        assert!(staff_html.contains("href=\"/podmin\""));
        assert!(staff_html.contains("account-totp-disable-form"));
        assert!(staff_html.contains("Doğrulanmış"));

        // Retired legacy slug normalizes to /podmin for staff as well
        let normalized_staff_html =
            render_with_admin_path("tr", &admin_user, &[], &[], &[], &[], Some("msi-komuta"));
        assert!(normalized_staff_html.contains("href=\"/podmin\""));
        assert!(!normalized_staff_html.contains("msi-komuta"));

        let reader_user = sample_user("reader", false, false);
        let reader_html = render_with_admin_path("tr", &reader_user, &[], &[], &[], &[], None);
        assert!(!reader_html.contains("podmin"));
        assert!(!reader_html.contains("Yönetim Komuta Merkezi"));
        assert!(reader_html.contains("Doğrulama Bekliyor"));
        assert!(reader_html.contains("id=\"account-resend-verify\""));
        assert!(reader_html.contains("id=\"account-totp-setup-btn\""));
        assert!(reader_html.contains("id=\"account-profile-form\""));
        assert!(reader_html.contains("/api/v1/account/export"));
    }

    #[test]
    fn en_account_portal_uses_british_defence_and_no_tr_ui_leaks() {
        let guest_en = account(None, &[], "en");
        assert!(guest_en.contains("Strategic defence archive"));
        assert!(!guest_en.contains("Defense"));
        assert!(!guest_en.contains("Giriş yap"));

        let user = sample_user("admin", false, false);
        let en_html = render_with_admin_path("en", &user, &[], &[], &[], &[], Some("podmin"));
        assert!(en_html.contains("Editorial &amp; Command Center"));
        assert!(en_html.contains("defence ad campaigns"));
        assert!(en_html.contains("Two-Factor Authentication"));
        assert!(en_html.contains("Your Orders &amp; Invoices"));
        assert!(!en_html.contains("Defense"));
        assert!(!en_html.contains("İki Aşamalı Doğrulama"));
        assert!(!en_html.contains("Siparişleriniz ve Faturalar"));
    }

    #[test]
    fn registered_and_subscriber_roles_never_see_operator_card_even_with_admin_path() {
        for role in [
            "registered",
            "subscriber_public",
            "subscriber_corporate",
            "subscriber_print_digital",
            "subscriber_digital",
            "subscriber",
            "reader",
            "guest",
            "member",
            "",
        ] {
            let user = sample_user(role, true, false);
            let html =
                render_with_admin_path("tr", &user, &[], &[], &[], &[], Some("podmin"));
            assert!(
                !html.contains("account-command-card"),
                "Role '{}' must never see account-command-card",
                role
            );
            assert!(
                !html.contains("/podmin"),
                "Role '{}' must never see admin path link",
                role
            );
        }

        for staff_role in ["admin", "super_admin", "site_operator", "editor_in_chief", "support_agent"] {
            let user = sample_user(staff_role, true, false);
            let html =
                render_with_admin_path("tr", &user, &[], &[], &[], &[], Some("podmin"));
            assert!(
                html.contains("account-command-card"),
                "Staff role '{}' should see account-command-card when admin path is passed",
                staff_role
            );
        }
    }

    #[test]
    fn portal_document_uses_v29_light_default_and_isolates_all_17_roles() {
        let non_staff_roles = [
            "subscriber_public",
            "subscriber_corporate",
            "subscriber_print_digital",
            "subscriber_digital",
            "registered",
        ];
        let staff_roles = [
            "super_admin",
            "admin",
            "site_operator",
            "editor_in_chief",
            "managing_editor",
            "content_publisher",
            "content_editor",
            "editor",
            "business_dev",
            "ads_manager",
            "finance_viewer",
            "support_agent",
        ];

        for role in non_staff_roles {
            let user = sample_user(role, true, false);
            let ctx = AccountPortalContext {
                lang: "tr",
                user: &user,
                reminders: &[],
                invoices: &[],
                notifications: &[],
                tags: &[],
                admin_path_opt: Some("podmin"),
                tr: None,
                institutions: &[],
                kvkk_logs: &[],
                contents: &[],
                issues: &[],
                corporate_requests: &[],
                purchased_codes: &[],
                can_download_edergi: true,
                has_subscriber_access: true,
                can_access_design_studio: false,
                hints: PortalQueryHints::default(),
                csp_nonce: Some("test-nonce"),
            };
            let html = render_portal_document(&ctx);
            assert!(!html.contains(r#"data-theme="light""#), "SSR HTML must not hardcode data-theme=\"light\"");
            let prehydrate_pos = html.find("UNIFIED_THEME_PREHYDRATE_JS").or_else(|| html.find("msi_admin_theme")).expect("theme prehydrate script");
            let tokens_pos = html.find("/css/design-tokens.css?v=35").expect("design-tokens.css link");
            let portal_css_pos = html.find("/css/account-portal.css?v=35").expect("account-portal.css link");
            assert!(prehydrate_pos < tokens_pos, "Theme prehydrate must run before design-tokens.css");
            assert!(tokens_pos < portal_css_pos, "design-tokens.css must load before account-portal.css");
            assert!(html.contains("/js/account-portal.js?v=35"));
            assert!(!html.contains("gsap.min.js"));
            assert!(!html.contains("account-command-card"), "Non-staff '{}' leaked command card", role);
            assert!(!html.contains("/podmin"), "Non-staff '{}' leaked /podmin link", role);
            assert!(!html.contains("<h2></h2>"), "Non-staff '{}' rendered empty <h2>", role);
        }

        for role in staff_roles {
            let user = sample_user(role, true, false);
            let expect_editor = matches!(
                role,
                "super_admin"
                    | "admin"
                    | "site_operator"
                    | "editor_in_chief"
                    | "managing_editor"
                    | "content_publisher"
                    | "content_editor"
                    | "editor"
            );
            let ctx = AccountPortalContext {
                lang: "en",
                user: &user,
                reminders: &[],
                invoices: &[],
                notifications: &[],
                tags: &[],
                admin_path_opt: Some("podmin"),
                tr: None,
                institutions: &[],
                kvkk_logs: &[],
                contents: &[],
                issues: &[],
                corporate_requests: &[],
                purchased_codes: &[],
                can_download_edergi: true,
                has_subscriber_access: true,
                can_access_design_studio: matches!(
                    role,
                    "super_admin" | "admin" | "site_operator" | "editor_in_chief" | "managing_editor"
                ),
                hints: PortalQueryHints::default(),
                csp_nonce: Some("test-nonce"),
            };
            let html = render_portal_document(&ctx);
            assert!(!html.contains(r#"data-theme="light""#), "SSR HTML must not hardcode data-theme=\"light\"");
            let prehydrate_pos = html.find("msi_admin_theme").expect("theme prehydrate script");
            let tokens_pos = html.find("/css/design-tokens.css?v=35").expect("design-tokens.css link");
            let portal_css_pos = html.find("/css/account-portal.css?v=35").expect("account-portal.css link");
            assert!(prehydrate_pos < tokens_pos, "Theme prehydrate must run before design-tokens.css");
            assert!(tokens_pos < portal_css_pos, "design-tokens.css must load before account-portal.css");
            assert!(html.contains("/js/account-portal.js?v=35"));
            assert!(html.contains("account-command-card"), "Staff '{}' missing command card", role);
            assert!(html.contains(r#"href="/podmin""#), "Staff '{}' missing /podmin link", role);
            assert_eq!(
                html.contains(r#"href="/podmin/editor""#),
                expect_editor,
                "Role '{}' /podmin/editor quick-link expectation mismatch",
                role
            );
            assert!(!html.contains("<h2></h2>"), "Staff '{}' rendered empty <h2>", role);
        }
    }

    #[test]
    fn editor_role_badge_tier_org_separator_and_edergi_links_are_accurate() {
        let mut user = sample_user("editor", true, false);
        user.organization_category = String::new();
        user.job_title = "Havacılık Editörü".into();
        user.subscription_tier = "basili-dijital".into();
        let issue = MagazineIssue {
            issue_number: 286,
            publication_type: "msi_tdr".into(),
            slug: "msi-tdr-286".into(),
            title_tr: "MSI TDR 286".into(),
            title_en: "MSI TDR 286".into(),
            publish_month_tr: "Kasım".into(),
            publish_month_en: "November".into(),
            year: 2026,
            cover_image: "images/msi/cover.webp".into(),
            page_count: 64,
            dossier_count: 10,
            ..Default::default()
        };
        let ctx_en = AccountPortalContext {
            lang: "en",
            user: &user,
            reminders: &[],
            invoices: &[],
            notifications: &[],
            tags: &[],
            admin_path_opt: Some("podmin"),
            tr: None,
            institutions: &[],
            kvkk_logs: &[],
            contents: &[],
            issues: std::slice::from_ref(&issue),
            corporate_requests: &[],
            purchased_codes: &[],
            can_download_edergi: true,
            has_subscriber_access: true,
            can_access_design_studio: false,
            hints: PortalQueryHints::default(),
            csp_nonce: Some("n"),
        };
        let html_en = render_portal_document(&ctx_en);
        assert!(html_en.contains(">Editor<"));
        assert!(!html_en.contains(">Content Editor<"));
        assert!(html_en.contains("Print &amp; Digital Subscriber"));
        assert!(!html_en.contains("<span> · Havacılık Editörü</span>"));
        assert!(html_en.contains("<span>Havacılık Editörü</span>"));
        assert!(html_en.contains("/e-dergi?issue=286&amp;publication_type=msi_tdr&amp;view=flipbook&amp;lang=en"));
    }

    #[test]
    fn payment_status_consent_type_and_portal_dates_are_localized() {
        assert_eq!(localize_payment_status("paid", false), ("is-verified", "Ödendi"));
        assert_eq!(localize_payment_status("PAID_PAYTR", true), ("is-verified", "Paid"));
        assert_eq!(localize_payment_status("pending", false), ("is-pending", "Beklemede"));
        assert_eq!(localize_payment_status("pending_payment", true), ("is-pending", "Pending"));
        assert_eq!(localize_payment_status("pending_bank_transfer", false), ("is-pending", "Havale Bekleniyor"));
        assert_eq!(localize_payment_status("failed", true), ("is-danger", "Failed"));
        assert_eq!(localize_payment_status("failed_payment", true), ("is-danger", "Failed"));
        assert_eq!(localize_payment_status("failed_payment", false), ("is-danger", "Başarısız"));
        assert_eq!(localize_payment_status("canceled", false), ("is-muted", "İptal Edildi"));
        assert_eq!(localize_payment_status("refunded", true), ("is-muted", "Refunded"));
        assert_eq!(localize_payment_status("paid_refunded", true), ("is-muted", "Refunded"));
        assert_eq!(localize_payment_status("paid_refunded", false), ("is-muted", "İade Edildi"));
        assert_eq!(localize_payment_status("custom_unknown", false), ("is-muted", "İşlemde"));

        let upper_admin = sample_user("ADMIN", true, false);
        let upper_prof = resolve_role_profile(&upper_admin, true);
        assert_eq!(upper_prof.label, "System Admin");
        assert_eq!(upper_prof.tone, "gold");
        let mixed_sub = sample_user("Subscriber_Public", true, false);
        assert_eq!(resolve_role_profile(&mixed_sub, false).tone, "emerald");

        assert_eq!(localize_consent_type("kvkk", false), "KVKK Aydınlatma Metni");
        assert_eq!(localize_consent_type("tos", true), "Terms of Service");
        assert_eq!(localize_consent_type("etk", false), "Ticari İleti İzni (ETK)");

        assert_eq!(format_portal_date("2026-11-21T14:35:00Z", false), "21.11.2026 14:35");
        assert_eq!(format_portal_date("2026-11-21T14:35:00Z", true), "21 Nov 2026 14:35");
        assert_eq!(format_portal_date("2027-12-31", false), "31.12.2027");
        assert_eq!(format_portal_date("2027-12-31", true), "31 Dec 2027");
        assert_eq!(format_portal_date("2026-01-01 00:00:00", false), "01.01.2026");
        assert_eq!(format_portal_date("", false), "—");
    }

    #[test]
    fn corporate_status_card_resolves_organization_category_and_formats_date() {
        let user = sample_user("registered", true, false);
        let choices = default_institution_choices();
        let req = CorporateVerificationRequest {
            id: 1,
            user_id: 42,
            email: "okur@savunma.gov.tr".into(),
            full_name: "Test Kullanıcısı".into(),
            organization_category: "SSB".into(),
            organization_name: "Savunma Sanayii Başkanlığı".into(),
            request_message: "Erişim talebi".into(),
            status: "pending".into(),
            reviewed_by: None,
            created_at: "2026-11-21T12:00:00Z".into(),
        };
        let ctx_tr = AccountPortalContext {
            lang: "tr",
            user: &user,
            reminders: &[],
            invoices: &[],
            notifications: &[],
            tags: &[],
            admin_path_opt: None,
            tr: None,
            institutions: &choices,
            kvkk_logs: &[],
            contents: &[],
            issues: &[],
            corporate_requests: std::slice::from_ref(&req),
            purchased_codes: &[],
            can_download_edergi: false,
            has_subscriber_access: false,
            can_access_design_studio: false,
            hints: PortalQueryHints::default(),
            csp_nonce: None,
        };
        let html_tr = render_corporate_status_card(&ctx_tr, &choices, false);
        assert!(html_tr.contains("SSB (Savunma Sanayii Başkanlığı)"));
        assert!(html_tr.contains("21.11.2026 12:00"));

        let ctx_en = AccountPortalContext { lang: "en", ..ctx_tr };
        let html_en = render_corporate_status_card(&ctx_en, &choices, true);
        assert!(html_en.contains("SSB (Presidency of Defence Industries)"));
        assert!(html_en.contains("21 Nov 2026 12:00"));

        let mut req_no_cat = req.clone();
        req_no_cat.organization_category = String::new();
        req_no_cat.status = "APPROVED".into();
        let ctx_empty_cat = AccountPortalContext {
            corporate_requests: std::slice::from_ref(&req_no_cat),
            ..ctx_en
        };
        let html_no_cat = render_corporate_status_card(&ctx_empty_cat, &choices, true);
        assert!(html_no_cat.contains("status-badge is-verified"));
        assert!(html_no_cat.contains("<strong>Savunma Sanayii Başkanlığı</strong> · "));
        assert!(!html_no_cat.contains("()"));
    }

    #[test]
    fn can_dl_parity_matches_across_all_17_canonical_roles() {
        let issue = MagazineIssue {
            issue_number: 285,
            publication_type: "msi".into(),
            slug: "msi-285".into(),
            title_tr: "MSI 285".into(),
            title_en: "MSI 285".into(),
            publish_month_tr: "Ekim".into(),
            publish_month_en: "October".into(),
            year: 2026,
            cover_image: "images/msi/cover.webp".into(),
            page_count: 80,
            dossier_count: 8,
            ..Default::default()
        };
        let staff_roles = [
            "super_admin", "admin", "site_operator", "editor_in_chief",
            "managing_editor", "content_publisher", "content_editor", "editor",
            "business_dev", "ads_manager", "finance_viewer", "support_agent",
        ];
        // Unverified staff WITH gov/subscriber access CAN download PDFs (parity with authorize_edergi_reader)
        for role in staff_roles {
            let mut u = sample_user(role, false, false);
            u.is_gov_verified = true;
            let (has_sub, can_dl) = compute_portal_edergi_permissions(&u);
            assert!(has_sub, "Staff '{}' with gov_verified must have subscriber access", role);
            assert!(can_dl, "Staff '{}' with subscriber access must bypass email_verified gate", role);

            // Unverified non-editorial staff WITHOUT subscriber access cannot download
            let mut u_nosub = sample_user(role, false, false);
            u_nosub.is_gov_verified = false;
            u_nosub.subscription_tier = String::new();
            let (has_sub2, can_dl2) = compute_portal_edergi_permissions(&u_nosub);
            let expect_sub_by_role = matches!(
                role,
                "super_admin" | "admin" | "site_operator" | "editor_in_chief"
                | "managing_editor" | "content_publisher" | "content_editor" | "editor"
            );
            assert_eq!(has_sub2, expect_sub_by_role, "Unexpected subscriber access for unsubscribed staff '{}'", role);
            assert_eq!(can_dl2, expect_sub_by_role, "Unexpected can_dl for unsubscribed staff '{}'", role);
        }
        // All 5 non-staff canonical roles: verified subscribers CAN download, unverified CANNOT, registered without sub CANNOT
        for role in [
            "subscriber_public",
            "subscriber_corporate",
            "subscriber_print_digital",
            "subscriber_digital",
        ] {
            let mut v = sample_user(role, true, false);
            v.is_gov_verified = false;
            v.subscription_tier = "Dijital Abone".into();
            assert_eq!(compute_portal_edergi_permissions(&v), (true, true), "Verified '{}' must download", role);

            let mut uv = sample_user(role, false, false);
            uv.is_gov_verified = false;
            uv.subscription_tier = "Dijital Abone".into();
            assert_eq!(compute_portal_edergi_permissions(&uv), (true, false), "Unverified '{}' must NOT download", role);
        }
        let mut reg = sample_user("registered", true, false);
        reg.is_gov_verified = false;
        reg.subscription_tier = String::new();
        assert_eq!(compute_portal_edergi_permissions(&reg), (false, false));
        // Unverified non-staff subscriber CANNOT download PDFs
        let mut unverified_sub = sample_user("subscriber_digital", false, false);
        unverified_sub.is_gov_verified = false;
        unverified_sub.subscription_tier = "Dijital Abone".into();
        let ctx_uv = AccountPortalContext {
            lang: "tr",
            user: &unverified_sub,
            reminders: &[],
            invoices: &[],
            notifications: &[],
            tags: &[],
            admin_path_opt: None,
            tr: None,
            institutions: &[],
            kvkk_logs: &[],
            contents: &[],
            issues: std::slice::from_ref(&issue),
            corporate_requests: &[],
            purchased_codes: &[],
            can_download_edergi: false,
            has_subscriber_access: true,
            can_access_design_studio: false,
            hints: PortalQueryHints::default(),
            csp_nonce: None,
        };
        let html_uv = render_portal_document(&ctx_uv);
        assert!(html_uv.contains("PDF için E-posta Doğrula"));
        assert!(!html_uv.contains("/download-pdf?"));
    }
}