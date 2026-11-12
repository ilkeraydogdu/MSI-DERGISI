use super::components::{empty_state, localized, notification_item, page_heading};
use crate::{engine::security_engine::escape_html as esc, models::*};
pub fn render(
    lang: &str,
    user: &UserAccount,
    reminders: &[RenewalReminderLog],
    invoices: &[OrderInvoice],
    notifications: &[SystemNotification],
    tags: &[SsbInterestTag],
) -> String {
    let interests: String = user
        .ssb_interests
        .iter()
        .map(|code| {
            let name = tags
                .iter()
                .find(|t| t.code == *code)
                .map(|t| localized(lang, &t.name_tr, &t.name_en))
                .unwrap_or(code);
            format!(
                "<a href=\"/arama?ssb_tag={}\" class=\"topic-chip\">{}</a>",
                esc(&urlencoding::encode(code)),
                esc(name)
            )
        })
        .collect();
    let invoice_rows: String = invoices
        .iter()
        .map(|i| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>₺{:.0}</td><td>{}</td><td>{}</td></tr>",
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
    let heading_html = page_heading(
        "OKUYUCU MERKEZİ",
        &format!("Merhaba, {}.", esc(&user.full_name)),
        "Aboneliğiniz ve okuma dünyanız.",
    );
    format!(
        r#"<section class="container section-space">{heading_html}<div class="account-grid"><article class="account-panel"><div class="eyebrow">ABONELİĞİNİZ</div><h2>{}</h2><p>Bitiş tarihi: {}</p><p>{reminder_text}</p><a href="/abonelik-ve-satis" class="button primary">Abonelik seçenekleri →</a></article><article class="account-panel"><div class="eyebrow">PROFİL</div><h2>{}</h2><p>{}<br>{}</p><button class="text-link" data-logout>Çıkış yap</button></article></div><section class="section-space"><div class="section-heading"><h2>İlgi alanlarınız</h2><a class="text-link" href="/arama">Tüm konuları keşfet →</a></div><div class="topic-chips">{}</div></section><section class="account-panel"><h2>Siparişleriniz</h2><div class="table-responsive"><table class="table"><thead><tr><th scope="col">Fatura</th><th scope="col">Yayın / plan</th><th scope="col">Tutar</th><th scope="col">Durum</th><th scope="col">Tarih</th></tr></thead><tbody>{invoice_rows}{empty_invoices}</tbody></table></div></section><section class="section-space"><h2>Size özel bildirimler</h2>{}</section></section>"#,
        esc(&user.subscription_tier),
        esc(user
            .subscription_expires_at
            .as_deref()
            .unwrap_or("Aktif abonelik bulunmuyor")),
        esc(&user.organization_name),
        esc(&user.email),
        esc(&user.job_title),
        if interests.is_empty() {
            "<p>Henüz bir ilgi alanı seçilmedi. Konuları arama sayfasından keşfedebilirsiniz.</p>"
                .into()
        } else {
            interests
        },
        if notices.is_empty() {
            empty_state("Yeni bildiriminiz yok.")
        } else {
            notices
        }
    )
}
