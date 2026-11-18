-- Phase 11: Canonical RBAC Normalization, Issues/Versions Schema Extensions & Mock Data Cleanup

-- 1. Normalize all 17 RBAC roles to canonical `resource:action` vocabulary.
-- Only super roles (super_admin, admin, site_operator) are system-locked (`is_system = 1`).
UPDATE user_roles SET permissions_json = '["*"]', is_system = 1 WHERE role_code IN ('super_admin', 'admin', 'site_operator');
UPDATE user_roles SET permissions_json = '["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","newsletters:write","notifications:write","redirects:write","translations:write","analytics:read","settings:read"]', is_system = 0 WHERE role_code = 'editor_in_chief';
UPDATE user_roles SET permissions_json = '["contents:manage","categories:write","media:manage","issues:manage","approvals:manage","newsletters:write","redirects:write","translations:write","settings:read"]', is_system = 0 WHERE role_code = 'managing_editor';
UPDATE user_roles SET permissions_json = '["contents:read","contents:write","contents:publish","contents:delete","categories:write","media:manage","issues:read","approvals:approve","redirects:write"]', is_system = 0 WHERE role_code = 'content_publisher';
UPDATE user_roles SET permissions_json = '["contents:read","contents:write","categories:read","media:read","media:upload","issues:read"]', is_system = 0 WHERE role_code IN ('editor', 'content_editor');
UPDATE user_roles SET permissions_json = '["ads:manage","commerce:manage","subscriptions:manage","invoices:write","analytics:read","revenue:read","media:upload"]', is_system = 0 WHERE role_code = 'business_dev';
UPDATE user_roles SET permissions_json = '["ads:manage","analytics:read","media:read","media:upload"]', is_system = 0 WHERE role_code = 'ads_manager';
UPDATE user_roles SET permissions_json = '["analytics:read","revenue:read","invoices:read","subscriptions:read","commerce:read"]', is_system = 0 WHERE role_code = 'finance_viewer';
UPDATE user_roles SET permissions_json = '["users:read","users:verify","corporate_requests:read","corporate_requests:decide","subscriptions:read","notifications:read","audit:read"]', is_system = 0 WHERE role_code = 'support_agent';
UPDATE user_roles SET permissions_json = '["edergi:read","edergi:download","articles:premium"]', is_system = 0 WHERE role_code IN ('subscriber_public', 'subscriber_corporate', 'subscriber_print_digital', 'subscriber_digital');
UPDATE user_roles SET permissions_json = '["articles:read"]', is_system = 0 WHERE role_code = 'registered';

-- 2. Extend magazine_issues for publication type (MSI vs MSI TDR) & real uploaded PDF support.
ALTER TABLE magazine_issues ADD COLUMN publication_type TEXT NOT NULL DEFAULT 'msi';
ALTER TABLE magazine_issues ADD COLUMN uploaded_pdf_file TEXT;
UPDATE magazine_issues SET publication_type = 'msi_tdr' WHERE issue_number < 200 OR slug LIKE 'msi-tdr%';

-- 3. Extend content_versions to preserve English fields during restore.
ALTER TABLE content_versions ADD COLUMN title_en TEXT NOT NULL DEFAULT '';
ALTER TABLE content_versions ADD COLUMN summary_en TEXT NOT NULL DEFAULT '';
ALTER TABLE content_versions ADD COLUMN body_en TEXT NOT NULL DEFAULT '';

-- 4. Purge synthetic/mock seed data (Zero Mockdata Rule).
DELETE FROM approval_requests WHERE content_id IN (
    SELECT id FROM contents WHERE slug IN (
        'kaan-programinda-mevcut-durum-ve-yol-haritasi',
        'akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu',
        'ctech-suru-ida-hibrit-haberlesme-konsepti',
        'aselsan-aesa-radar-ve-elektronik-harp-mimarisi-advertorial',
        'stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri',
        'msi-dergisi-280-sayi-basyazi-caydiricilik-ve-teknolojik-bagimsizlik',
        'genelkurmay-baskani-idef-mesaji-teknolojik-ustunluk',
        'polonya-mspo-fuarinda-turk-savunma-sanayii-cikarmasi',
        'sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi',
        'nato-hava-savunma-sistemlerinin-gelecegi-analiz',
        'turk-savunma-sanayii-ihracat-rekoru-2026',
        'savunma-ve-havacilik-kobilerine-yonelik-yenilikci-destek-mekanizmalari',
        'yeni-nesil-firkateyn-ve-savas-yonetim-sistemleri-gundemi',
        'aselsan-yuksek-irtifa-hava-savunma-radari-teslimati',
        'baykar-kizilelma-ve-tb3-deniz-testleri-basariyla-suruyor',
        'dr-m-cuneyd-firat-ctech-kapak-soylesisi'
    )
);

DELETE FROM contents WHERE slug IN (
    'kaan-programinda-mevcut-durum-ve-yol-haritasi',
    'akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu',
    'ctech-suru-ida-hibrit-haberlesme-konsepti',
    'aselsan-aesa-radar-ve-elektronik-harp-mimarisi-advertorial',
    'stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri',
    'msi-dergisi-280-sayi-basyazi-caydiricilik-ve-teknolojik-bagimsizlik',
    'genelkurmay-baskani-idef-mesaji-teknolojik-ustunluk',
    'polonya-mspo-fuarinda-turk-savunma-sanayii-cikarmasi',
    'sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi',
    'nato-hava-savunma-sistemlerinin-gelecegi-analiz',
    'turk-savunma-sanayii-ihracat-rekoru-2026',
    'savunma-ve-havacilik-kobilerine-yonelik-yenilikci-destek-mekanizmalari',
    'yeni-nesil-firkateyn-ve-savas-yonetim-sistemleri-gundemi',
    'aselsan-yuksek-irtifa-hava-savunma-radari-teslimati',
    'baykar-kizilelma-ve-tb3-deniz-testleri-basariyla-suruyor',
    'dr-m-cuneyd-firat-ctech-kapak-soylesisi'
);

DELETE FROM seo_redirects WHERE
    old_path IN ('/tr/dergi/280', '/haber/kaan-programinda-mevcut-durum', '/soylesi/dr-m-cuneyd-firat', '/detail-1.html')
    OR new_path LIKE '%kaan-programinda-mevcut-durum-ve-yol-haritasi%'
    OR new_path LIKE '%akinci-iha-300-fuzesi-ile-250-km-hedefi-vurdu%'
    OR new_path LIKE '%dr-m-cuneyd-firat-ctech-kapak-soylesisi%';

DELETE FROM user_purchased_items WHERE item_code = 'kaan-programinda-mevcut-durum-ve-yol-haritasi' OR user_email IN (
    'murat.yilmaz@msb.gov.tr', 'selin.kaya@ssb.gov.tr', 'levent.aksoy@dzkk.tsk.tr',
    'cem.demir@hvkk.tsk.tr', 'serkan.oz@jandarma.gov.tr', 'hakan.erdem@aselsan.com.tr',
    'zeynep.ucar@tusas.com', 'baris.koc@stm.com.tr', 'emre.aydiner@itu.edu.tr'
);

DELETE FROM corporate_verification_requests WHERE email IN (
    'kurumsal.tedarik@gmail.com', 'ata.savunma.kobi@outlook.com', 'murat.yilmaz@msb.gov.tr'
);

DELETE FROM renewal_reminders WHERE user_email IN (
    'hakan.erdem@aselsan.com.tr', 'zeynep.ucar@tusas.com', 'baris.koc@stm.com.tr'
);

DELETE FROM users WHERE email IN (
    'murat.yilmaz@msb.gov.tr', 'selin.kaya@ssb.gov.tr', 'levent.aksoy@dzkk.tsk.tr',
    'cem.demir@hvkk.tsk.tr', 'serkan.oz@jandarma.gov.tr', 'hakan.erdem@aselsan.com.tr',
    'zeynep.ucar@tusas.com', 'baris.koc@stm.com.tr', 'emre.aydiner@itu.edu.tr'
);

DELETE FROM order_invoices WHERE invoice_no = 'MSI-2026-104821' OR payment_status = 'PAID_VIRTUAL_POS';
DELETE FROM security_audit_logs WHERE client_ip = '127.0.0.1' AND request_path IN ('/system/waf-init', '/api/v1/auth/register', '/api/v1/*');
DELETE FROM system_notifications WHERE target_email IN ('murat.yilmaz@msb.gov.tr', 'zeynep.ucar@tusas.com') OR action_url = '/icerik/kaan-programinda-mevcut-durum-ve-yol-haritasi';
DELETE FROM newsletter_campaigns WHERE recipient_count IN (4820, 3150);
DELETE FROM recommendation_rules WHERE recommended_content_slug IN (
    'kaan-programinda-mevcut-durum-ve-yol-haritasi',
    'ctech-suru-ida-hibrit-haberlesme-konsepti',
    'stm-milgem-istif-sinifi-ve-stm500-denizalti-cozumleri',
    'nato-hava-savunma-sistemlerinin-gelecegi-analiz',
    'savunma-ve-havacilik-kobilerine-yonelik-yenilikci-destek-mekanizmalari',
    'sedec-ve-saha-expo-sonrasi-tedarik-zinciri-ozel-dosyasi'
);

-- Remove duplicate fallback ad row sharing slot_code with an older row.
DELETE FROM ad_campaigns WHERE id NOT IN (SELECT MIN(id) FROM ad_campaigns GROUP BY slot_code);

-- Clean bogus issue numbers mistakenly extracted from weapon calibers (e.g., 155, 157, 250, 295).
UPDATE contents SET issue_number = NULL WHERE issue_number IS NOT NULL AND issue_number NOT IN (SELECT issue_number FROM magazine_issues);
