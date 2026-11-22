-- Migration 003: Data fixes and unique reminder index

UPDATE users
SET email = 'hakan.erdem@aselsan.com.tr'
WHERE email = 'hakan. erdem@aselsan.com.tr';

INSERT INTO ad_campaigns (
    slot_code, badge_label, dimensions, sponsor_name, sponsor_type, pill_style,
    title_tr, title_en, desc_tr, desc_en, target_url, image_url,
    budget_score, is_default_fallback, is_floating_popup, is_newsletter_ad,
    target_categories_json, target_ssb_tags_json, target_institutions_json,
    impressions_count, clicks_count, is_active
)
SELECT
    'ad-inhouse-rbf-fallback', 'KURUMSAL ÇÖZÜM ORTAĞI', '970 × 90 STANDART VARSAYILAN VİTRİN',
    'REMOVE BEFORE PHOTOS (RBF) & MSI DERGİSİ', 'in_house', 'navy',
    'Hava-Hava, Kara ve Deniz Platformlarında Telifli Havacılık & Savunma Fotoğrafçılığı',
    'Copyrighted Aviation & Defense Photography Across Air-to-Air, Land and Naval Platforms',
    'Onur Kurç & Tayfun Yaşar yönetiminde Remove Before Photos yüksek çözünürlüklü telifli savunma görsel arşivi ve MSI Dergisi kurumsal abonelik çözümleri.',
    'Remove Before Photos high-resolution defense visual archive and MSI Dergisi corporate subscription solutions.',
    '/abonelik-ve-satis', 'images/msi/dergi-286.webp',
    80, TRUE, FALSE, TRUE,
    '["*"]', '["hava-araclari","deniz-araclari","kara-araclari","hizmetler-kurumsal-iletisim"]', '["*"]',
    19500, 840, TRUE
WHERE NOT EXISTS (
    SELECT 1 FROM ad_campaigns WHERE slot_code = 'ad-inhouse-rbf-fallback'
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_renewal_reminders_user_stage
    ON renewal_reminders(user_id, stage_label);
