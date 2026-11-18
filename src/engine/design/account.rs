use super::components::{
    empty_state, localized, notification_item, page_heading, ssb_tag_display_label,
    CANONICAL_SSB_TAG_CODES,
};
use crate::{engine::security_engine::escape_html as esc, models::*};

const ORG_CATEGORIES: &[(&str, &str)] = &[
    ("Sektör Kuruluşları", "Sektör Kuruluşları"),
    ("MSB", "MSB (Millî Savunma Bakanlığı)"),
    ("SSB", "SSB (Savunma Sanayii Başkanlığı)"),
    ("K.K.K.lığı", "K.K.K.lığı (Kara Kuvvetleri)"),
    ("Dz.K.K.lığı", "Dz.K.K.lığı (Deniz Kuvvetleri)"),
    ("Hv.K.K.lığı", "Hv.K.K.lığı (Hava Kuvvetleri)"),
    ("J.G.K.lığı", "J.G.K.lığı (Jandarma Genel K.)"),
    ("EGM", "EGM (Emniyet Genel Müdürlüğü)"),
    ("S.G.K.lığı", "S.G.K.lığı (Sahil Güvenlik K.)"),
    ("MİT", "MİT (Millî İstihbarat Teşkilatı)"),
    ("Diğer Kamu Kurumları", "Diğer Kamu Kurumları"),
    ("Üniversite / Öğrenci", "Üniversite / Öğrenci"),
    ("Yurt Dışı Paydaş", "Yurt Dışı Paydaş"),
];

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
    let Some(u) = user else {
        let eyebrow = if lang == "en" {
            "MSI READER CENTER"
        } else {
            "MSI OKUYUCU MERKEZİ"
        };
        let heading = if lang == "en" {
            "Strategic defense archive.<br>Your personal reading command."
        } else {
            "Stratejik savunma arşivi.<br>Kişisel okuma merkeziniz."
        };
        let desc = if lang == "en" {
            "Sign in with your corporate or personal account to access digital E-Magazine issues, manage SSB technology alerts, and view invoices."
        } else {
            "Dijital E-Dergi sayılarına erişmek, SSB teknoloji ilgi alanlarınızı yönetmek ve kurumsal fatura kayıtlarınızı görüntülemek için giriş yapın."
        };
        let login_cta = if lang == "en" {
            "Sign in / Create account →"
        } else {
            "Giriş yap / Hesap oluştur →"
        };
        let sub_link = if lang == "en" {
            "Explore subscriptions →"
        } else {
            "Abonelik seçeneklerini incele →"
        };
        return format!(
            r##"<section class="container section-space"><div class="account-welcome"><div class="eyebrow">{eyebrow}</div><h1>{heading}</h1><p>{desc}</p><a href="#modal-login" class="button primary">{login_cta}</a><a href="/abonelik-ve-satis" class="text-link">{sub_link}</a></div></section>"##
        );
    };
    render_with_admin_path(lang, u, &[], invoices, &[], &[], admin_path_opt)
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
    let command_card = admin_path_opt
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty() && is_staff_role(&user.role))
        .map(|clean_admin| {
            let safe_admin = esc(clean_admin);
            let eyebrow = if lang == "en" {
                "AUTHORIZED OPERATOR ACCESS"
            } else {
                "YETKİLİ OPERATÖR ERİŞİMİ"
            };
            let title = if lang == "en" {
                "Editorial & Command Center"
            } else {
                "Yönetim Komuta Merkezi"
            };
            let desc = if lang == "en" {
                "Manage editorial dossiers, digital issues, defense ad campaigns, RBAC permissions, and central design architecture."
            } else {
                "Editoryal içerik akışı, E-Dergi sayıları, savunma reklam kampanyaları, analitik telemetri ve merkezi tasarım stüdyosuna doğrudan geçiş yapın."
            };
            let btn_main = if lang == "en" {
                "Management Command Center →"
            } else {
                "Yönetim Komuta Merkezi →"
            };
            let btn_studio = if lang == "en" {
                "Design Studio →"
            } else {
                "Tasarım Stüdyosu →"
            };
            format!(
                r#"<aside class="account-command-card"><div class="account-command-copy"><div class="eyebrow">{eyebrow}</div><h2>{title}</h2><p>{desc}</p></div><div class="account-command-actions"><a href="/{safe_admin}" class="button primary">{btn_main}</a><a href="/{safe_admin}/tasarim" class="button secondary">{btn_studio}</a></div></aside>"#
            )
        })
        .unwrap_or_default();

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
                "<a href=\"/arama?ssb_tag={}\" class=\"topic-chip\">#{}</a>",
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

    let org_options: String = ORG_CATEGORIES
        .iter()
        .map(|(val, label)| {
            let sel = if user.organization_category == *val {
                " selected"
            } else {
                ""
            };
            format!(
                r#"<option value="{val}"{sel}>{label}</option>"#,
                val = esc(val),
                sel = sel,
                label = esc(label)
            )
        })
        .collect();

    let verification_html = if user.email_verified {
        let lbl = if lang == "en" {
            "✓ Verified"
        } else {
            "✓ Doğrulanmış"
        };
        format!(r#"<span class="status-badge is-verified">{lbl}</span>"#)
    } else {
        let badge_lbl = if lang == "en" {
            "● Verification Pending"
        } else {
            "● Doğrulama Bekliyor"
        };
        let resend_lbl = if lang == "en" {
            "Resend Verification Email"
        } else {
            "Doğrulama E-postasını Tekrar Gönder"
        };
        format!(
            r#"<span class="status-badge is-pending">{badge_lbl}</span><button type="button" class="button secondary btn-sm" id="account-resend-verify" data-user-email="{email}">{resend_lbl}</button><span id="account-verify-feedback" class="inline-feedback" role="status"></span>"#,
            badge_lbl = badge_lbl,
            email = esc(&user.email),
            resend_lbl = resend_lbl
        )
    };

    let gov_badge = if user.is_gov_verified {
        if lang == "en" {
            r#"<span class="status-badge is-gov">✓ Public / Academic Verified</span>"#
        } else {
            r#"<span class="status-badge is-gov">✓ Kamu / Akademik Doğrulamalı</span>"#
        }
    } else {
        ""
    };

    let totp_card_html = if user.totp_enabled {
        let status_lbl = if lang == "en" {
            "✓ Active (TOTP)"
        } else {
            "✓ Etkin (Authenticator)"
        };
        format!(
            r#"<article class="account-panel account-security-panel" id="account-totp-card"><div class="panel-header-row"><div><div class="eyebrow">HESAP GÜVENLİĞİ · 2FA</div><h2>İki Aşamalı Doğrulama</h2></div><span class="status-badge is-verified">{status_lbl}</span></div><p>Hesabınız TOTP tabanlı Authenticator uygulaması ve tek kullanımlık kurtarma kodları ile korunmaktadır.</p><form id="account-totp-disable-form" class="ui-form account-totp-form"><div class="form-grid-2"><label>Mevcut şifreniz<input name="password" type="password" autocomplete="current-password" placeholder="••••••••" required></label><label>Authenticator / Yedek kod<input name="code" type="text" class="totp-code-input" autocomplete="one-time-code" minlength="6" maxlength="20" placeholder="000000" required></label></div><p class="form-feedback" role="status"></p><button type="submit" class="button secondary danger-outline">2FA Korumasını Devre Dışı Bırak</button></form></article>"#
        )
    } else {
        let status_lbl = if lang == "en" {
            "Disabled"
        } else {
            "Devre Dışı"
        };
        format!(
            r#"<article class="account-panel account-security-panel" id="account-totp-card"><div class="panel-header-row"><div><div class="eyebrow">HESAP GÜVENLİĞİ · 2FA</div><h2>İki Aşamalı Doğrulama</h2></div><span class="status-badge is-muted">{status_lbl}</span></div><p>Google Authenticator, Microsoft Authenticator veya 1Password ile kurumsal hesabınıza ikinci güvenlik katmanı ekleyin.</p><div class="account-security-actions"><button type="button" class="button primary" id="account-totp-setup-btn">2FA Kurulumunu Başlat (QR Kod) →</button></div><p class="form-feedback" id="account-totp-setup-feedback" role="status"></p><div id="account-totp-setup-panel" class="totp-setup-box" hidden><div class="totp-qr-wrap"><div class="totp-qr-frame"><img id="account-totp-qr" alt="Authenticator 2FA QR Kodu"></div><div class="totp-manual-meta"><span class="eyebrow">MANUEL KURULUM ANAHTARI (BASE32)</span><code id="account-totp-secret" class="totp-secret-code"></code><p class="field-help">QR kodu kamerayla okutun veya anahtarı Authenticator uygulamanıza manuel ekleyin, ardından üretilen doğrulama kodunu aşağıya girin.</p></div></div><form id="account-totp-verify-form" class="ui-form"><input type="hidden" name="secret" id="account-totp-secret-input" value=""><label>Doğrulama kodu<input name="code" type="text" class="totp-code-input" autocomplete="one-time-code" minlength="6" maxlength="20" placeholder="000000" required></label><p class="form-feedback" role="status"></p><button type="submit" class="button primary">Doğrula ve 2FA'yı Etkinleştir →</button></form><div id="totp-backup-codes-box" class="totp-backup-codes" hidden><div class="eyebrow">TEK KULLANIMLIK KURTARMA KODLARI</div><p>Cihazınıza erişemediğinizde giriş yapabilmek için bu kodları güvenli bir yerde saklayın:</p><div id="totp-backup-codes-list" class="backup-codes-grid"></div><div class="hero-actions"><button type="button" class="button secondary" data-copy-backup-codes>Kodları Kopyala</button><button type="button" class="button primary" data-reload-page>Kaydettim, Tamamla →</button></div></div></div></article>"#
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
        .map(|r| format!("Aboneliğinizin bitişine {} gün kaldı.", r.days_remaining))
        .unwrap_or_else(|| "Şu anda bekleyen abonelik hatırlatmanız yok.".into());

    let empty_invoices = if invoices.is_empty() {
        "<tr><td colspan=\"5\">Henüz siparişiniz bulunmuyor.</td></tr>"
    } else {
        ""
    };

    let email_opt_checked = if user.email_consent && user.marketing_emails_opted_in {
        " checked"
    } else {
        ""
    };

    let heading_html = page_heading(
        "OKUYUCU MERKEZİ",
        &format!("Merhaba, {}.", &user.full_name),
        "Aboneliğiniz, güvenlik ayarlarınız ve kişiselleştirilmiş savunma akışınız.",
    );

    format!(
        r#"<section class="container section-space account-portal">{heading_html}{command_card}<div class="account-grid"><article class="account-panel"><div class="eyebrow">ABONELİĞİNİZ</div><h2>{tier}</h2><p>Bitiş tarihi: <strong>{expires}</strong></p><p>{reminder_text}</p><a href="/abonelik-ve-satis" class="button primary">Abonelik seçenekleri →</a></article><article class="account-panel"><div class="eyebrow">KİMLİK &amp; DOĞRULAMA</div><h2>{org_name}</h2><p class="account-identity-meta"><span>{email}</span><span>{org_cat}{job_sep}{job_title}</span></p><div class="account-badges-row">{verification_html}{gov_badge}</div><div class="account-logout-row"><button type="button" class="text-link" data-logout>Oturumu kapat →</button></div></article></div><div class="account-management-stack">{totp_card_html}<article class="account-panel" id="account-profile-card"><div class="panel-header-row"><div><div class="eyebrow">PROFİL &amp; İLGİ ALANLARI</div><h2>Okuyucu Profili ve SSB Taksonomisi</h2></div><a href="/api/v1/account/export" class="button secondary btn-sm" download="msi-hesap-verilerim.json" data-kvkk-export>KVKK Verilerimi İndir (JSON) ↓</a></div><form id="account-profile-form" class="ui-form"><div class="form-grid-2"><label>Ad soyad<input name="full_name" value="{full_name}" autocomplete="name" maxlength="100" required></label><label>Görev / Ünvan<input name="job_title" value="{job_title}" autocomplete="organization-title" maxlength="100" placeholder="Örn: Sistem Mühendisi, Proje Subayı"></label></div><div class="form-grid-2"><label>Kurum adı<input name="organization_name" value="{org_name}" autocomplete="organization" maxlength="120" required></label><label>Kurum kategorisi<select name="organization_category">{org_options}</select></label></div><fieldset class="ssb-fieldset"><legend>SSB Teknoloji İlgi Alanları (Kişiselleştirilmiş Akış)</legend><div id="account-ssb-tags" class="ssb-tag-picks">{ssb_checkboxes}</div></fieldset><label class="check-label"><input type="checkbox" name="email_consent"{email_opt_checked}><span>Yeni sayılar, özel dosyalar ve savunma sanayii bültenleri hakkında e-posta ile bilgilendirilmek istiyorum.</span></label><p class="form-feedback" role="status"></p><div class="account-form-actions"><button type="submit" class="button primary">Profil ve İlgi Alanlarını Güncelle →</button></div></form></article></div><section class="section-space"><div class="section-heading"><h2>Aktif ilgi alanlarınız</h2><a class="text-link" href="/arama">Tüm konuları keşfet →</a></div><div class="topic-chips">{interests_block}</div></section><section class="account-panel"><h2>Siparişleriniz ve Faturalar</h2><div class="table-responsive"><table class="table"><thead><tr><th scope="col">Fatura</th><th scope="col">Yayın / plan</th><th scope="col">Tutar</th><th scope="col">Durum</th><th scope="col">Tarih</th></tr></thead><tbody>{invoice_rows}{empty_invoices}</tbody></table></div></section><section class="section-space"><h2>Size özel bildirimler</h2>{notices_block}</section></section>"#,
        heading_html = heading_html,
        command_card = command_card,
        tier = esc(&user.subscription_tier),
        expires = esc(user
            .subscription_expires_at
            .as_deref()
            .unwrap_or("Aktif abonelik bulunmuyor")),
        reminder_text = reminder_text,
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
        totp_card_html = totp_card_html,
        full_name = esc(&user.full_name),
        org_options = org_options,
        ssb_checkboxes = ssb_checkboxes,
        email_opt_checked = email_opt_checked,
        interests_block = if interests.is_empty() {
            "<p>Henüz bir ilgi alanı seçilmedi. Yukarıdaki SSB teknoloji etiketlerinden seçim yapabilirsiniz.</p>".to_string()
        } else {
            interests
        },
        invoice_rows = invoice_rows,
        empty_invoices = empty_invoices,
        notices_block = if notices.is_empty() {
            empty_state("Yeni bildiriminiz yok.")
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
}
