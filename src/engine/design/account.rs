use super::components::{
    empty_state, localized, notification_item, page_heading, ssb_tag_display_label,
    CANONICAL_SSB_TAG_CODES,
};
use super::labels_helper::{
    default_institution_choices, lang_href, tr_text, InstitutionChoice,
};
use crate::{
    engine::{security_engine::escape_html as esc, translation_engine::TranslationCache},
    models::*,
};

fn is_staff_role(role: &str) -> bool {
    let r = role.trim();
    !r.is_empty() && !matches!(r, "reader" | "subscriber" | "guest" | "member")
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
        let eyebrow = tr_text(tr, lang, "account.eyebrow", "MSI OKUYUCU MERKEZİ", "MSI READER CENTER");
        let heading = if lang.eq_ignore_ascii_case("en") {
            tr_text(
                tr,
                lang,
                "account.guest_heading",
                "Stratejik savunma arşivi.<br>Kişisel okuma merkeziniz.",
                "Strategic defence archive.<br>Your personal reading command.",
            )
        } else {
            tr_text(
                tr,
                lang,
                "account.guest_heading",
                "Stratejik savunma arşivi.<br>Kişisel okuma merkeziniz.",
                "Strategic defence archive.<br>Your personal reading command.",
            )
        };
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
            r##"<section class="container section-space"><div class="account-welcome"><div class="eyebrow">{eyebrow}</div><h1>{heading}</h1><p>{desc}</p><a href="#modal-login" class="button primary">{login_cta}</a><a href="{sub_href}" class="text-link">{sub_link}</a></div></section>"##,
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
    let is_en = lang.eq_ignore_ascii_case("en");
    let command_card = admin_path_opt
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty() && is_staff_role(&user.role))
        .map(|clean_admin| {
            let safe_admin = esc(clean_admin);
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
            let desc = tr_text(
                tr,
                lang,
                "account.operator_desc",
                "Editoryal içerik akışı, E-Dergi sayıları, savunma reklam kampanyaları, analitik telemetri ve merkezi tasarım stüdyosuna doğrudan geçiş yapın.",
                "Manage editorial dossiers, digital issues, defence ad campaigns, RBAC permissions, and central design architecture.",
            );
            let btn_main = tr_text(
                tr,
                lang,
                "account.operator_btn_main",
                "Yönetim Komuta Merkezi →",
                "Management Command Center →",
            );
            let btn_studio = tr_text(
                tr,
                lang,
                "account.operator_btn_studio",
                "Tasarım Stüdyosu →",
                "Design Studio →",
            );
            format!(
                r#"<aside class="account-command-card"><div class="account-command-copy"><div class="eyebrow">{eyebrow}</div><h2>{title}</h2><p>{desc}</p></div><div class="account-command-actions"><a href="/{safe_admin}" class="button primary">{btn_main}</a><a href="/{safe_admin}/tasarim" class="button secondary">{btn_studio}</a></div></aside>"#,
                eyebrow = esc(&eyebrow),
                title = esc(&title),
                desc = esc(&desc),
                safe_admin = safe_admin,
                btn_main = esc(&btn_main),
                btn_studio = esc(&btn_studio),
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
    let org_options: String = inst_choices
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
            r#"<article class="account-panel account-security-panel" id="account-totp-card"><div class="panel-header-row"><div><div class="eyebrow">{totp_eyebrow}</div><h2>{totp_title}</h2></div><span class="status-badge is-verified">{status_lbl}</span></div><p>{active_desc}</p><form id="account-totp-disable-form" class="ui-form account-totp-form"><div class="form-grid-2"><label>{cur_pw_lbl}<input name="password" type="password" autocomplete="current-password" placeholder="••••••••" required></label><label>{code_lbl}<input name="code" type="text" class="totp-code-input" autocomplete="one-time-code" minlength="6" maxlength="20" placeholder="000000" required></label></div><p class="form-feedback" role="status"></p><button type="submit" class="button secondary danger-outline">{disable_btn}</button></form></article>"#,
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
            format!(
                "<tr><td><code>{}</code></td><td>{}</td><td>₺{:.0}</td><td>{}</td><td>{}</td></tr>",
                esc(&i.invoice_no),
                esc(&i.item_title),
                i.amount_try,
                esc(&i.payment_status),
                esc(&i.created_at)
            )
        })
        .collect();

    let notices: String = notifications
        .iter()
        .map(|n| notification_item(n, lang))
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

    let no_orders_lbl = tr_text(tr, lang, "account.no_orders", "Henüz siparişiniz bulunmuyor.", "You have no orders yet.");
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

    format!(
        r#"<section class="container section-space account-portal">{heading_html}{command_card}<div class="account-grid"><article class="account-panel"><div class="eyebrow">{sub_eyebrow}</div><h2>{tier}</h2><p>{expires_lbl} <strong>{expires}</strong></p><p>{reminder_text}</p><a href="{sub_href}" class="button primary">{sub_opts_lbl}</a></article><article class="account-panel"><div class="eyebrow">{id_eyebrow}</div><h2>{org_name}</h2><p class="account-identity-meta"><span>{email}</span><span>{org_cat}{job_sep}{job_title}</span></p><div class="account-badges-row">{verification_html}{gov_badge}</div><div class="account-logout-row"><button type="button" class="text-link" data-logout>{logout_lbl}</button></div></article></div><div class="account-management-stack">{totp_card_html}<article class="account-panel" id="account-profile-card"><div class="panel-header-row"><div><div class="eyebrow">{prof_eyebrow}</div><h2>{prof_title}</h2></div><a href="/api/v1/account/export" class="button secondary btn-sm" download="msi-hesap-verilerim.json" data-kvkk-export>{kvkk_export_lbl}</a></div><form id="account-profile-form" class="ui-form"><div class="form-grid-2"><label>{full_name_lbl}<input name="full_name" value="{full_name}" autocomplete="name" maxlength="100" required></label><label>{job_lbl}<input name="job_title" value="{job_title}" autocomplete="organization-title" maxlength="100" placeholder="{job_ph}"></label></div><div class="form-grid-2"><label>{org_name_lbl}<input name="organization_name" value="{org_name}" autocomplete="organization" maxlength="120" required></label><label>{org_cat_lbl}<select name="organization_category">{org_options}</select></label></div><fieldset class="ssb-fieldset"><legend>{ssb_legend}</legend><div id="account-ssb-tags" class="ssb-tag-picks">{ssb_checkboxes}</div></fieldset><label class="check-label"><input type="checkbox" name="email_consent"{email_opt_checked}><span>{email_opt_lbl}</span></label><p class="form-feedback" role="status"></p><div class="account-form-actions"><button type="submit" class="button primary">{save_profile_lbl}</button></div></form></article></div><section class="section-space"><div class="section-heading"><h2>{active_interests_lbl}</h2><a class="text-link" href="{search_base}">{explore_topics_lbl}</a></div><div class="topic-chips">{interests_block}</div></section><section class="account-panel"><h2>{invoices_heading}</h2><div class="table-responsive"><table class="table"><thead><tr><th scope="col">{th_invoice}</th><th scope="col">{th_item}</th><th scope="col">{th_amount}</th><th scope="col">{th_status}</th><th scope="col">{th_date}</th></tr></thead><tbody>{invoice_rows}{empty_invoices}</tbody></table></div></section><section class="section-space"><h2>{notices_heading}</h2>{notices_block}</section></section>"#,
        heading_html = heading_html,
        command_card = command_card,
        sub_eyebrow = esc(&sub_eyebrow),
        tier = esc(&user.subscription_tier),
        expires_lbl = esc(&expires_lbl),
        expires = esc(user
            .subscription_expires_at
            .as_deref()
            .unwrap_or(&no_sub_lbl)),
        reminder_text = esc(&reminder_text),
        sub_href = esc(&sub_href),
        sub_opts_lbl = esc(&sub_opts_lbl),
        id_eyebrow = esc(&id_eyebrow),
        org_name = esc(&user.organization_name),
        email = esc(&user.email),
        org_cat = esc(&user.organization_category),
        job_sep = if user.job_title.trim().is_empty() {
            ""
        } else {
            " · "
        },
        job_title = esc(&user.job_title),
        verification_html = verification_html,
        gov_badge = gov_badge,
        logout_lbl = esc(&logout_lbl),
        totp_card_html = totp_card_html,
        prof_eyebrow = esc(&prof_eyebrow),
        prof_title = esc(&prof_title),
        kvkk_export_lbl = esc(&kvkk_export_lbl),
        full_name_lbl = esc(&full_name_lbl),
        full_name = esc(&user.full_name),
        job_lbl = esc(&job_lbl),
        job_ph = esc(&job_ph),
        org_name_lbl = esc(&org_name_lbl),
        org_cat_lbl = esc(&org_cat_lbl),
        org_options = org_options,
        ssb_legend = esc(&ssb_legend),
        ssb_checkboxes = ssb_checkboxes,
        email_opt_checked = email_opt_checked,
        email_opt_lbl = esc(&email_opt_lbl),
        save_profile_lbl = esc(&save_profile_lbl),
        active_interests_lbl = esc(&active_interests_lbl),
        search_base = search_base,
        explore_topics_lbl = esc(&explore_topics_lbl),
        interests_block = if interests.is_empty() {
            format!("<p>{}</p>", esc(&no_interests_lbl))
        } else {
            interests
        },
        invoices_heading = esc(&invoices_heading),
        th_invoice = esc(&th_invoice),
        th_item = esc(&th_item),
        th_amount = esc(&th_amount),
        th_status = esc(&th_status),
        th_date = esc(&th_date),
        invoice_rows = invoice_rows,
        empty_invoices = empty_invoices,
        notices_heading = esc(&notices_heading),
        notices_block = if notices.is_empty() {
            empty_state(&esc(&no_notices_lbl))
        } else {
            notices
        }
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
        }
    }

    #[test]
    fn staff_sees_command_center_while_reader_never_sees_admin_path() {
        let admin_user = sample_user("admin", true, true);
        let staff_html =
            render_with_admin_path("tr", &admin_user, &[], &[], &[], &[], Some("msi-komuta"));
        assert!(staff_html.contains("Yönetim Komuta Merkezi →"));
        assert!(staff_html.contains("href=\"/msi-komuta\""));
        assert!(staff_html.contains("account-totp-disable-form"));
        assert!(staff_html.contains("Doğrulanmış"));

        let reader_user = sample_user("reader", false, false);
        let reader_html = render_with_admin_path("tr", &reader_user, &[], &[], &[], &[], None);
        assert!(!reader_html.contains("msi-komuta"));
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
        let en_html = render_with_admin_path("en", &user, &[], &[], &[], &[], Some("msi-komuta"));
        assert!(en_html.contains("Editorial &amp; Command Center"));
        assert!(en_html.contains("defence ad campaigns"));
        assert!(en_html.contains("Two-Factor Authentication"));
        assert!(en_html.contains("Your Orders &amp; Invoices"));
        assert!(!en_html.contains("Defense"));
        assert!(!en_html.contains("İki Aşamalı Doğrulama"));
        assert!(!en_html.contains("Siparişleriniz ve Faturalar"));
    }
}
