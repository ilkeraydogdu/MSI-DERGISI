-- Migration 012: Central i18n, Locales, Defence Terminology Glossary (220+ rules),
-- Dynamic Institution Categories, Gov/Personal Email Domains, Email Groups,
-- Composite E-Dergi PK + 22 PDF Links, EN Approval Status Gate, B2B Extensions & RBAC Hierarchy.

-- ============================================================================
-- 1. CENTRAL LOCALES REGISTRY
-- ============================================================================
CREATE TABLE IF NOT EXISTS locales (
    code TEXT PRIMARY KEY,
    name_native TEXT NOT NULL,
    name_en TEXT NOT NULL,
    hreflang TEXT NOT NULL,
    og_locale TEXT NOT NULL,
    dir TEXT NOT NULL DEFAULT 'ltr' CHECK (dir IN ('ltr', 'rtl')),
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    is_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    fallback_code TEXT NOT NULL DEFAULT 'tr',
    sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_locales_single_default
    ON locales(is_default) WHERE is_default = TRUE;

INSERT INTO locales (
    code, name_native, name_en, hreflang, og_locale, dir, is_default, is_enabled, fallback_code, sort_order
) VALUES
    ('tr', 'Türkçe', 'Turkish', 'tr', 'tr_TR', 'ltr', TRUE, TRUE, 'tr', 1),
    ('en', 'English', 'English', 'en', 'en_US', 'ltr', FALSE, TRUE, 'tr', 2),
    ('ar', 'العربية', 'Arabic', 'ar', 'ar_AR', 'rtl', FALSE, FALSE, 'en', 3),
    ('de', 'Deutsch', 'German', 'de', 'de_DE', 'ltr', FALSE, FALSE, 'en', 4),
    ('ru', 'Русский', 'Russian', 'ru', 'ru_RU', 'ltr', FALSE, FALSE, 'en', 5),
    ('fr', 'Français', 'French', 'fr', 'fr_FR', 'ltr', FALSE, FALSE, 'en', 6)
ON CONFLICT (code) DO NOTHING;

-- ============================================================================
-- 2. DEFENCE, NATO & EDITORIAL TRANSLATION GLOSSARY (220+ TERMS)
-- ============================================================================
CREATE TABLE IF NOT EXISTS translation_glossary (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    term_tr TEXT NOT NULL,
    term_en TEXT NOT NULL,
    abbreviation_tr TEXT NOT NULL DEFAULT '',
    abbreviation_en TEXT NOT NULL DEFAULT '',
    domain TEXT NOT NULL,
    rule_type TEXT NOT NULL DEFAULT 'fixed' CHECK (rule_type IN ('fixed', 'preserve', 'forbidden')),
    preferred_en TEXT NOT NULL,
    forbidden_variants_json TEXT NOT NULL DEFAULT '[]' CHECK (forbidden_variants_json::jsonb IS NOT NULL),
    case_sensitive BOOLEAN NOT NULL DEFAULT FALSE,
    priority INTEGER NOT NULL DEFAULT 50,
    source_ref TEXT NOT NULL DEFAULT '',
    notes TEXT NOT NULL DEFAULT '',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"'),
    UNIQUE(term_tr, domain)
);

CREATE INDEX IF NOT EXISTS idx_glossary_domain_active
    ON translation_glossary(domain, is_active, priority DESC);

INSERT INTO translation_glossary (
    term_tr, term_en, abbreviation_tr, abbreviation_en, domain, rule_type,
    preferred_en, forbidden_variants_json, case_sensitive, priority, source_ref, notes
) VALUES
-- ── A. Style & Sovereign Nomenclature Rules (Priority 100) ─────────────────
('Türkiye', 'Türkiye', 'TR', 'TR', 'style', 'fixed', 'Türkiye', '["Turkey","Republic of Turkey"]', TRUE, 100, 'Official MFA/UN Style', 'Use official UN-registered country name Türkiye.'),
('Turkey (Yasak Biçim)', 'Türkiye', '', '', 'style', 'forbidden', 'Türkiye', '["Turkey"]', TRUE, 100, 'MSI Editorial Style', 'Replace legacy Turkey with Türkiye in English copy.'),
('Savunma (İngiliz İngilizcesi)', 'Defence', '', '', 'style', 'fixed', 'Defence', '["Defense","defense"]', FALSE, 100, 'MSI TDR British English', 'MSI TDR strictly follows British English spelling: Defence.'),
('Defense (ABD Yazımı Yasak)', 'Defence', '', '', 'style', 'forbidden', 'Defence', '["Defense","defense"]', FALSE, 100, 'MSI TDR British English', 'Never use US spelling Defense in editorial or UI text.'),
('Zırh (İngiliz İngilizcesi)', 'Armour', '', '', 'style', 'fixed', 'Armour', '["Armor","Armored"]', FALSE, 95, 'MSI TDR British English', 'Prefer British English Armour / Armoured in editorial prose.'),
('Program (İngiliz İngilizcesi)', 'Programme', '', '', 'style', 'fixed', 'Programme', '["Program"]', FALSE, 80, 'MSI TDR British English', 'Prefer British English Programme for procurement initiatives.'),

-- ── B. Official Institutions, Ministries, Armed Forces & NATO (Institutional) ─
('Millî Savunma Bakanlığı', 'Ministry of National Defence (MoND)', 'MSB', 'MoND', 'institutional', 'fixed', 'Ministry of National Defence (MoND)', '["Ministry of Defense","MoD","Ministry of National Defense"]', FALSE, 100, 'MSB Official', 'Always Ministry of National Defence (MoND); never MoD or Defense.'),
('Savunma Sanayii Başkanlığı', 'Presidency of Defence Industries (SSB)', 'SSB', 'SSB', 'institutional', 'fixed', 'Presidency of Defence Industries (SSB)', '["Undersecretariat for Defence Industries","Undersecretariat of Defence Industries","Secretariat of Defence Industries","SSM","Defense Industries Presidency"]', FALSE, 100, 'SSB Official', 'SSB became Presidency under 2018 Presidential Decree; forbid SSM/Undersecretariat.'),
('Savunma Sanayii Müsteşarlığı (Tarihsel)', 'Presidency of Defence Industries (SSB)', 'SSM', 'SSB', 'institutional', 'forbidden', 'Presidency of Defence Industries (SSB)', '["SSM","Undersecretariat for Defence Industries"]', FALSE, 98, 'SSB Official', 'Historic SSM transitions to Presidency of Defence Industries (SSB) in modern refs.'),
('Türk Silahlı Kuvvetleri', 'Turkish Armed Forces (TAF)', 'TSK', 'TAF', 'institutional', 'fixed', 'Turkish Armed Forces (TAF)', '["Turkish Army","TSK"]', FALSE, 100, 'TSK/NATO Official', 'Use Turkish Armed Forces (TAF).'),
('Genelkurmay Başkanlığı', 'Turkish General Staff (TGS)', 'Gnkur.Bşk.lığı', 'TGS', 'institutional', 'fixed', 'Turkish General Staff', '["Chief of Staff Office"]', FALSE, 95, 'TSK Official', 'Official English designation.'),
('Kara Kuvvetleri Komutanlığı', 'Turkish Land Forces (TLF)', 'K.K.K.lığı', 'TLF', 'institutional', 'fixed', 'Turkish Land Forces (TLF)', '["Turkish Army Command","Land Forces Command"]', FALSE, 98, 'TSK Official', 'Force branch nomenclature.'),
('Deniz Kuvvetleri Komutanlığı', 'Turkish Naval Forces (TNF)', 'Dz.K.K.lığı', 'TNF', 'institutional', 'fixed', 'Turkish Naval Forces (TNF)', '["Turkish Navy Command"]', FALSE, 98, 'TSK Official', 'Force branch nomenclature.'),
('Hava Kuvvetleri Komutanlığı', 'Turkish Air Force (TurAF)', 'Hv.K.K.lığı', 'TurAF', 'institutional', 'fixed', 'Turkish Air Force (TurAF)', '["TAF Air Force"]', FALSE, 98, 'TSK/NATO Official', 'Use TurAF to distinguish from TAF (Turkish Armed Forces).'),
('Jandarma Genel Komutanlığı', 'Gendarmerie General Command', 'J.G.K.lığı', 'GGC', 'institutional', 'fixed', 'Gendarmerie General Command', '["Gendarmerie Command"]', FALSE, 95, 'Ministry of Interior', 'Official English title.'),
('Sahil Güvenlik Komutanlığı', 'Turkish Coast Guard Command', 'S.G.K.lığı', 'TCGC', 'institutional', 'fixed', 'Turkish Coast Guard Command', '["Coast Guard"]', FALSE, 95, 'Ministry of Interior', 'Official English title.'),
('Emniyet Genel Müdürlüğü', 'Turkish National Police (TNP)', 'EGM', 'TNP', 'institutional', 'fixed', 'Turkish National Police (TNP)', '["Police Directorate"]', FALSE, 95, 'Ministry of Interior', 'Official English title.'),
('Millî İstihbarat Teşkilatı', 'National Intelligence Organization (MIT)', 'MİT', 'MIT', 'institutional', 'fixed', 'National Intelligence Organization (MIT)', '["National Intelligence Agency"]', FALSE, 95, 'Official', 'Official English title.'),
('Özel Kuvvetler Komutanlığı', 'Special Forces Command', 'ÖKK', 'SFC', 'institutional', 'fixed', 'Special Forces Command', '[]', FALSE, 90, 'TSK Official', 'Special forces designation.'),
('Donanma Komutanlığı', 'Turkish Fleet Command', '', '', 'institutional', 'fixed', 'Turkish Fleet Command', '[]', FALSE, 88, 'TNF Official', 'Fleet command.'),
('Denizaltı Filosu Komutanlığı', 'Submarine Fleet Command', '', '', 'institutional', 'fixed', 'Submarine Fleet Command', '[]', FALSE, 85, 'TNF Official', 'Submarine fleet.'),
('Savunma Sanayii İcra Komitesi', 'Defence Industry Executive Committee (SSİK)', 'SSİK', 'DIEC', 'institutional', 'fixed', 'Defence Industry Executive Committee (SSİK)', '["Defense Industry Executive Committee"]', FALSE, 95, 'SSB Official', 'Top procurement decision body.'),
('Türk Silahlı Kuvvetlerini Güçlendirme Vakfı', 'Turkish Armed Forces Foundation (TAFF)', 'TSKGV', 'TAFF', 'institutional', 'fixed', 'Turkish Armed Forces Foundation (TAFF)', '[]', FALSE, 95, 'TSKGV Official', 'Parent foundation of ASELSAN, TUSAŞ, ROKETSAN, HAVELSAN, ASPİLSAN, İŞBİR.'),
('Savunma ve Havacılık Sanayii İmalatçılar Derneği', 'Defence and Aerospace Industry Manufacturers Association (SaSaD)', 'SaSaD', 'SaSaD', 'institutional', 'fixed', 'Defence and Aerospace Industry Manufacturers Association (SaSaD)', '["SASAD Defense"]', FALSE, 92, 'SaSaD Official', 'Industry association.'),
('SAHA İstanbul', 'SAHA Istanbul', 'SAHA', 'SAHA Istanbul', 'institutional', 'fixed', 'SAHA Istanbul', '[]', FALSE, 92, 'SAHA Official', 'Europe largest aerospace and defence cluster.'),
('Ankara Askerî Ataşeler Birliği', 'Ankara Military Attaché Corps (AMAC)', 'AMAC', 'AMAC', 'institutional', 'fixed', 'Ankara Military Attaché Corps (AMAC)', '[]', FALSE, 92, 'AMAC Official', 'UMSA publishing partner.'),
('NATO', 'NATO', 'NATO', 'NATO', 'institutional', 'preserve', 'NATO', '[]', TRUE, 100, 'NATO Terminology', 'Preserve uppercase acronym NATO.'),
('NATO Müttefik Kara Komutanlığı', 'NATO Allied Land Command (LANDCOM)', 'LANDCOM', 'LANDCOM', 'institutional', 'fixed', 'NATO Allied Land Command (LANDCOM)', '[]', FALSE, 90, 'NATO Official', 'Izmir-based NATO command.'),
('NATO Destek ve Tedarik Ajansı', 'NATO Support and Procurement Agency (NSPA)', 'NSPA', 'NSPA', 'institutional', 'fixed', 'NATO Support and Procurement Agency (NSPA)', '[]', FALSE, 90, 'NATO Official', 'NATO logistics and procurement.'),
('NATO İletişim ve Bilgi Ajansı', 'NATO Communications and Information Agency (NCIA)', 'NCIA', 'NCIA', 'institutional', 'fixed', 'NATO Communications and Information Agency (NCIA)', '[]', FALSE, 90, 'NATO Official', 'NATO C4ISR agency.'),
('NATO Standardizasyon Anlaşması', 'NATO Standardization Agreement (STANAG)', 'STANAG', 'STANAG', 'institutional', 'fixed', 'NATO Standardization Agreement (STANAG)', '[]', FALSE, 90, 'NATO Official', 'STANAG technical standard.'),
('Askerî Fabrika ve Tersane İşletme A.Ş.', 'ASFAT (Military Factory and Shipyard Management)', 'ASFAT', 'ASFAT', 'institutional', 'fixed', 'ASFAT', '[]', FALSE, 92, 'MSB/ASFAT', 'State-owned defence contractor.'),
('Makine ve Kimya Endüstrisi A.Ş.', 'MKE (Mechanical and Chemical Industry Corporation)', 'MKE', 'MKE', 'institutional', 'fixed', 'MKE', '["MKEK"]', FALSE, 92, 'MKE Official', 'Transformed from MKEK to MKE A.Ş.'),
('TÜBİTAK SAGE', 'TÜBİTAK SAGE', 'SAGE', 'TÜBİTAK SAGE', 'institutional', 'preserve', 'TÜBİTAK SAGE', '["TUBITAK SAGE"]', TRUE, 95, 'TÜBİTAK Official', 'Preserve Turkish diacritics in proper noun TÜBİTAK SAGE.'),
('TÜBİTAK BİLGEM', 'TÜBİTAK BİLGEM', 'BİLGEM', 'TÜBİTAK BİLGEM', 'institutional', 'preserve', 'TÜBİTAK BİLGEM', '["TUBITAK BILGEM"]', TRUE, 95, 'TÜBİTAK Official', 'Preserve diacritics.'),
('TÜBİTAK UZAY', 'TÜBİTAK UZAY', 'UZAY', 'TÜBİTAK UZAY', 'institutional', 'preserve', 'TÜBİTAK UZAY', '["TUBITAK UZAY"]', TRUE, 95, 'TÜBİTAK Official', 'Preserve diacritics.'),
('Millî Güvenlik Kurulu', 'National Security Council (NSC)', 'MGK', 'NSC', 'institutional', 'fixed', 'National Security Council (NSC)', '[]', FALSE, 88, 'Official', 'State security body.'),
('Cumhurbaşkanlığı', 'Presidency of the Republic of Türkiye', '', '', 'institutional', 'fixed', 'Presidency of the Republic of Türkiye', '["Presidency of Turkey"]', FALSE, 95, 'Official', 'Use Türkiye.'),
('Dışişleri Bakanlığı', 'Ministry of Foreign Affairs (MFA)', '', 'MFA', 'institutional', 'fixed', 'Ministry of Foreign Affairs', '[]', FALSE, 85, 'Official', 'MFA.'),
('İçişleri Bakanlığı', 'Ministry of Interior', '', '', 'institutional', 'fixed', 'Ministry of Interior', '[]', FALSE, 85, 'Official', 'Interior ministry.'),

-- ── C. Aerospace Platforms, Aircraft, UAVs & Space (Aerospace) ───────────────
('KAAN', 'KAAN', 'KAAN', 'KAAN', 'aerospace', 'preserve', 'KAAN', '["TF-X","TFX","Kaan"]', TRUE, 100, 'TUSAŞ Official', 'Always uppercase KAAN (5th-generation National Combat Aircraft).'),
('Millî Muharip Uçak', 'National Combat Aircraft (KAAN)', 'MMU', 'KAAN', 'aerospace', 'fixed', 'KAAN National Combat Aircraft', '["TF-X Fighter"]', FALSE, 98, 'TUSAŞ Official', 'MMU is officially named KAAN.'),
('HÜRJET', 'HÜRJET', 'HÜRJET', 'HÜRJET', 'aerospace', 'preserve', 'HÜRJET', '["HURJET","Hurjet"]', TRUE, 98, 'TUSAŞ Official', 'Preserve uppercase + Ü diacritic.'),
('HÜRKUŞ', 'HÜRKUŞ', 'HÜRKUŞ', 'HÜRKUŞ', 'aerospace', 'preserve', 'HÜRKUŞ', '["HURKUS","Hurkus"]', TRUE, 98, 'TUSAŞ Official', 'Preserve uppercase + diacritics.'),
('GÖKBEY', 'GÖKBEY', 'T625', 'GÖKBEY', 'aerospace', 'preserve', 'GÖKBEY', '["GOKBEY","Gokbey"]', TRUE, 98, 'TUSAŞ Official', 'T625 GÖKBEY utility helicopter.'),
('T129 ATAK', 'T129 ATAK', 'ATAK', 'T129 ATAK', 'aerospace', 'preserve', 'T129 ATAK', '[]', TRUE, 95, 'TUSAŞ Official', 'Attack helicopter.'),
('ATAK 2', 'ATAK 2 (T929)', 'ATAK 2', 'ATAK 2', 'aerospace', 'preserve', 'ATAK 2', '[]', TRUE, 95, 'TUSAŞ Official', 'Heavy-class attack helicopter.'),
('T925', 'T925', 'T925', 'T925', 'aerospace', 'preserve', 'T925', '[]', TRUE, 90, 'TUSAŞ Official', '10-tonne utility helicopter.'),
('ANKA', 'ANKA', 'ANKA', 'ANKA', 'aerospace', 'preserve', 'ANKA', '["Anka"]', TRUE, 95, 'TUSAŞ Official', 'MALE UAV platform.'),
('ANKA III', 'ANKA III', 'ANKA III', 'ANKA III', 'aerospace', 'preserve', 'ANKA III', '["ANKA-3","Anka 3"]', TRUE, 98, 'TUSAŞ Official', 'Flying-wing stealth UCAV.'),
('AKSUNGUR', 'AKSUNGUR', 'AKSUNGUR', 'AKSUNGUR', 'aerospace', 'preserve', 'AKSUNGUR', '["Aksungur"]', TRUE, 95, 'TUSAŞ Official', 'Twin-engine MALE UAV.'),
('AKINCI', 'AKINCI', 'AKINCI', 'AKINCI', 'aerospace', 'preserve', 'AKINCI', '["Akinci","AKINCı"]', TRUE, 98, 'Baykar Official', 'Bayraktar AKINCI UCAV.'),
('KIZILELMA', 'KIZILELMA', 'MİUS', 'KIZILELMA', 'aerospace', 'preserve', 'KIZILELMA', '["Kizilelma","Kızıl Elma"]', TRUE, 100, 'Baykar Official', 'Unmanned fighter aircraft.'),
('Bayraktar TB2', 'Bayraktar TB2', 'TB2', 'TB2', 'aerospace', 'preserve', 'Bayraktar TB2', '[]', TRUE, 95, 'Baykar Official', 'Tactical UCAV.'),
('Bayraktar TB3', 'Bayraktar TB3', 'TB3', 'TB3', 'aerospace', 'preserve', 'Bayraktar TB3', '[]', TRUE, 95, 'Baykar Official', 'Shipborne folding-wing UCAV.'),
('ŞİMŞEK', 'ŞİMŞEK', 'ŞİMŞEK', 'ŞİMŞEK', 'aerospace', 'preserve', 'ŞİMŞEK', '["SIMSEK","Simsek"]', TRUE, 92, 'TUSAŞ Official', 'High-speed target/kamikaze drone.'),
('SÜPER ŞİMŞEK', 'SÜPER ŞİMŞEK', 'SÜPER ŞİMŞEK', 'SÜPER ŞİMŞEK', 'aerospace', 'preserve', 'SÜPER ŞİMŞEK', '["SUPER SIMSEK"]', TRUE, 92, 'TUSAŞ Official', 'Supersonic/subsonic deep strike drone.'),
('KARGU', 'KARGU', 'KARGU', 'KARGU', 'aerospace', 'preserve', 'KARGU', '["Kargu"]', TRUE, 90, 'STM Official', 'Rotary-wing loitering munition.'),
('ALPAGU', 'ALPAGU', 'ALPAGU', 'ALPAGU', 'aerospace', 'preserve', 'ALPAGU', '[]', TRUE, 90, 'STM Official', 'Fixed-wing loitering munition.'),
('TOGAN', 'TOGAN', 'TOGAN', 'TOGAN', 'aerospace', 'preserve', 'TOGAN', '[]', TRUE, 88, 'STM Official', 'Autonomous rotary-wing reconnaissance UAV.'),
('BOYGA', 'BOYGA', 'BOYGA', 'BOYGA', 'aerospace', 'preserve', 'BOYGA', '[]', TRUE, 88, 'STM Official', 'Mortar-drop UAV.'),
('GÖKTÜRK', 'GÖKTÜRK', 'GÖKTÜRK', 'GÖKTÜRK', 'aerospace', 'preserve', 'GÖKTÜRK', '["GOKTURK"]', TRUE, 92, 'Space Official', 'Earth observation satellite series.'),
('İMECE', 'İMECE', 'İMECE', 'İMECE', 'aerospace', 'preserve', 'İMECE', '["IMECE"]', TRUE, 92, 'TÜBİTAK UZAY', 'Sub-metre resolution EO satellite.'),
('TÜRKSAT 6A', 'TÜRKSAT 6A', '6A', 'TÜRKSAT 6A', 'aerospace', 'preserve', 'TÜRKSAT 6A', '["TURKSAT 6A"]', TRUE, 92, 'TUSAŞ/TÜRKSAT', 'First indigenous geostationary communications satellite.'),
('İnsansız Hava Aracı', 'Unmanned Aerial Vehicle (UAV)', 'İHA', 'UAV', 'aerospace', 'fixed', 'Unmanned Aerial Vehicle (UAV)', '["Unmanned Air Vehicle"]', FALSE, 95, 'NATO AAP-6', 'Standard aerospace acronym.'),
('Silahlı İnsansız Hava Aracı', 'Armed Unmanned Aerial Vehicle (UCAV)', 'SİHA', 'UCAV', 'aerospace', 'fixed', 'Armed Unmanned Aerial Vehicle (UCAV)', '["Armed Drone"]', FALSE, 95, 'MSI/NATO Style', 'Use UCAV for armed tactical/HALE platforms.'),
('Muharip İnsansız Uçak Sistemi', 'Unmanned Combat Aircraft System (UCAS)', 'MİUS', 'UCAS', 'aerospace', 'fixed', 'Unmanned Combat Aircraft System (UCAS)', '[]', FALSE, 92, 'Industry Standard', 'Used for KIZILELMA class.'),
('Dolanan Mühimmat', 'Loitering Munition', '', '', 'aerospace', 'fixed', 'Loitering Munition', '["Kamikaze Drone"]', FALSE, 92, 'NATO Terminology', 'Prefer Loitering Munition over Kamikaze Drone in formal copy.'),
('Harekât Yarıçapı', 'Combat Radius', '', '', 'aerospace', 'fixed', 'Combat Radius', '["Operational Radius"]', FALSE, 88, 'Aerospace Tech', 'Standard specification term.'),
('Azami Kalkış Ağırlığı', 'Maximum Take-Off Weight (MTOW)', 'AKA', 'MTOW', 'aerospace', 'fixed', 'Maximum Take-Off Weight (MTOW)', '[]', FALSE, 88, 'Aerospace Tech', 'Standard aviation term.'),
('Servis Tavanı', 'Service Ceiling', '', '', 'aerospace', 'fixed', 'Service Ceiling', '[]', FALSE, 88, 'Aerospace Tech', 'Altitude specification.'),
('Görev Bilgisayarı', 'Mission Computer', '', '', 'aerospace', 'fixed', 'Mission Computer', '[]', FALSE, 88, 'Avionics', 'Core avionics hardware.'),
('Aviyonik Süit', 'Avionics Suite', '', '', 'aerospace', 'fixed', 'Avionics Suite', '[]', FALSE, 88, 'Avionics', 'Integrated flight electronics.'),
('Dahili Silah İstasyonu', 'Internal Weapons Bay', '', '', 'aerospace', 'fixed', 'Internal Weapons Bay', '[]', FALSE, 90, '5th-Gen Stealth', 'KAAN / ANKA III stealth bay.'),
('Düşük Görünürlük', 'Low Observability (Stealth)', '', 'LO', 'aerospace', 'fixed', 'Low Observability', '[]', FALSE, 88, '5th-Gen Stealth', 'RCS reduction technology.'),
('Görüş Ötesi', 'Beyond Visual Range (BVR)', '', 'BVR', 'aerospace', 'fixed', 'Beyond Visual Range (BVR)', '[]', FALSE, 90, 'Air Combat', 'Missile engagement envelope.'),
('Havadan İhbar ve Kontrol', 'Airborne Early Warning and Control (AEW&C)', 'HİK', 'AEW&C', 'aerospace', 'fixed', 'Airborne Early Warning and Control (AEW&C)', '["AWACS"]', FALSE, 92, 'TurAF E-7T', 'TurAF Peace Eagle designation.'),
('Havada Yakıt İkmali', 'Aerial Refuelling', '', 'AAR', 'aerospace', 'fixed', 'Aerial Refuelling', '["Aerial Refueling"]', FALSE, 88, 'British English', 'Double-l in British English refuelling.'),
('Fırlatma Koltuğu', 'Ejection Seat', '', '', 'aerospace', 'fixed', 'Ejection Seat', '[]', FALSE, 85, 'Aviation', 'Cockpit escape system.'),
('Yüksek İrtifa Uzun Havada Kalış', 'High-Altitude Long-Endurance (HALE)', '', 'HALE', 'aerospace', 'fixed', 'High-Altitude Long-Endurance (HALE)', '[]', FALSE, 90, 'UAV Classification', 'NATO Class III HALE.'),
('Orta İrtifa Uzun Havada Kalış', 'Medium-Altitude Long-Endurance (MALE)', '', 'MALE', 'aerospace', 'fixed', 'Medium-Altitude Long-Endurance (MALE)', '[]', FALSE, 90, 'UAV Classification', 'NATO Class III MALE.'),
('Faydalı Yük', 'Payload', '', '', 'aerospace', 'fixed', 'Payload', '["Useful Load"]', FALSE, 90, 'Aerospace Tech', 'Standard payload term.'),
('Kanat Açıklığı', 'Wingspan', '', '', 'aerospace', 'fixed', 'Wingspan', '[]', FALSE, 85, 'Aerospace Tech', 'Airframe geometry.'),

-- ── D. Naval Platforms, Vessels & Maritime Warfare (Naval) ───────────────────
('MİLGEM', 'MİLGEM', 'MİLGEM', 'MİLGEM', 'naval', 'preserve', 'MİLGEM', '["MILGEM","Milgem"]', TRUE, 100, 'TNF/STM Official', 'National Ship programme (Ada/İstif/TF-2000).'),
('İSTİF Sınıfı Fırkateyn', 'İSTİF-Class Frigate', 'İ Sınıfı', 'İSTİF-Class', 'naval', 'fixed', 'İSTİF-Class Frigate', '["ISTIF Class","Stack Class"]', TRUE, 98, 'TNF Official', 'Preserve İSTİF; never translate literally as Stack.'),
('ADA Sınıfı Korvet', 'ADA-Class Corvette', 'ADA', 'ADA-Class', 'naval', 'fixed', 'ADA-Class Corvette', '["Island Class"]', TRUE, 95, 'TNF Official', 'Never translate Ada as Island.'),
('MİLDEN', 'MİLDEN', 'MİLDEN', 'MİLDEN', 'naval', 'preserve', 'MİLDEN', '["MILDEN","Milden"]', TRUE, 98, 'TNF Official', 'National Submarine programme.'),
('TF-2000 Hava Savunma Harbi Muhribi', 'TF-2000 Air Defence Warfare Destroyer', 'TF-2000', 'TF-2000', 'naval', 'fixed', 'TF-2000 Air Defence Warfare Destroyer', '["TF-2000 Defense Destroyer"]', FALSE, 95, 'TNF Official', 'British Defence spelling.'),
('STM500', 'STM500', 'STM500', 'STM500', 'naval', 'preserve', 'STM500', '["STM-500"]', TRUE, 95, 'STM Official', 'Shallow-water compact submarine.'),
('REİS Sınıfı Denizaltı', 'REİS-Class Submarine', 'REİS', 'REİS-Class', 'naval', 'fixed', 'REİS-Class Submarine', '["Type 214TN"]', TRUE, 95, 'TNF Official', 'AIP-equipped submarine class.'),
('TCG ANADOLU', 'TCG ANADOLU', 'L-400', 'TCG ANADOLU', 'naval', 'preserve', 'TCG ANADOLU', '[]', TRUE, 98, 'TNF Official', 'Amphibious assault ship / UCAV carrier.'),
('TCG İSTANBUL', 'TCG İSTANBUL', 'F-515', 'TCG İSTANBUL', 'naval', 'preserve', 'TCG İSTANBUL', '["TCG ISTANBUL"]', TRUE, 96, 'TNF Official', 'Lead ship of İSTİF class.'),
('TCG PİRİREİS', 'TCG PİRİREİS', 'S-330', 'TCG PİRİREİS', 'naval', 'preserve', 'TCG PİRİREİS', '["TCG PIRIREIS"]', TRUE, 95, 'TNF Official', 'Lead ship of REİS class.'),
('HİSAR Sınıfı Açık Deniz Karakol Gemisi', 'HİSAR-Class Offshore Patrol Vessel (OPV)', 'ADKG', 'OPV', 'naval', 'fixed', 'HİSAR-Class Offshore Patrol Vessel (OPV)', '["HISAR OPV"]', FALSE, 94, 'ASFAT/TNF', 'Preserve HİSAR.'),
('ULAQ İDA', 'ULAQ Armed USV', 'ULAQ', 'ULAQ', 'naval', 'preserve', 'ULAQ', '[]', TRUE, 92, 'ARES/Meteksan', 'Unmanned surface vessel.'),
('MARLİN İDA', 'MARLİN USV', 'MARLİN', 'MARLİN', 'naval', 'preserve', 'MARLİN', '["MARLIN"]', TRUE, 92, 'ASELSAN/SEFİNE', 'EW-capable unmanned surface vessel.'),
('SANCAR İDA', 'SANCAR USV', 'SANCAR', 'SANCAR', 'naval', 'preserve', 'SANCAR', '[]', TRUE, 90, 'HAVELSAN/Yonca', 'Autonomous USV.'),
('ALBATROS-S', 'ALBATROS-S', 'ALBATROS-S', 'ALBATROS-S', 'naval', 'preserve', 'ALBATROS-S', '[]', TRUE, 90, 'ASELSAN', 'Swarm USV system.'),
('MİDLAS', 'MİDLAS', 'MİDLAS', 'MİDLAS', 'naval', 'preserve', 'MİDLAS', '["MIDLAS"]', TRUE, 96, 'ROKETSAN Official', 'National Vertical Launching System.'),
('GÖKDENİZ', 'GÖKDENİZ', 'GÖKDENİZ', 'GÖKDENİZ', 'naval', 'preserve', 'GÖKDENİZ', '["GOKDENIZ"]', TRUE, 95, 'ASELSAN Official', 'Close-in weapon system (CIWS).'),
('ADVENT', 'ADVENT', 'ADVENT', 'ADVENT', 'naval', 'preserve', 'ADVENT', '[]', TRUE, 95, 'HAVELSAN Official', 'Network-enabled combat management system.'),
('GENESİS', 'GENESIS', 'GENESİS', 'GENESIS', 'naval', 'fixed', 'GENESIS', '[]', TRUE, 90, 'HAVELSAN Official', 'Combat management system.'),
('Mavi Vatan', 'Blue Homeland', '', '', 'naval', 'fixed', 'Blue Homeland', '[]', FALSE, 92, 'Turkish Maritime Doctrine', 'Official maritime strategy term.'),
('İnsansız Deniz Aracı', 'Unmanned Surface Vessel (USV)', 'İDA', 'USV', 'naval', 'fixed', 'Unmanned Surface Vessel (USV)', '["Unmanned Sea Vehicle"]', FALSE, 95, 'NATO Maritime', 'Standard USV term.'),
('İnsansız Sualtı Aracı', 'Unmanned Underwater Vehicle (UUV)', 'İSA', 'UUV', 'naval', 'fixed', 'Unmanned Underwater Vehicle (UUV)', '[]', FALSE, 92, 'NATO Maritime', 'Standard UUV term.'),
('Savaş Yönetim Sistemi', 'Combat Management System (CMS)', 'SYS', 'CMS', 'naval', 'fixed', 'Combat Management System (CMS)', '["War Management System"]', FALSE, 95, 'Naval Engineering', 'Never translate as War Management System.'),
('Denizaltı Savunma Harbi', 'Anti-Submarine Warfare (ASW)', 'DSH', 'ASW', 'naval', 'fixed', 'Anti-Submarine Warfare (ASW)', '[]', FALSE, 92, 'NATO AAP-6', 'Standard NATO warfare area.'),
('Su Üstü Harbi', 'Anti-Surface Warfare (ASuW)', 'SUH', 'ASuW', 'naval', 'fixed', 'Anti-Surface Warfare (ASuW)', '[]', FALSE, 92, 'NATO AAP-6', 'Standard NATO warfare area.'),
('Hava Savunma Harbi', 'Anti-Air Warfare (AAW)', 'HSH', 'AAW', 'naval', 'fixed', 'Anti-Air Warfare (AAW)', '[]', FALSE, 92, 'NATO AAP-6', 'Standard NATO warfare area.'),
('Mayın Karşı Tedbirleri', 'Mine Countermeasures (MCM)', 'MKT', 'MCM', 'naval', 'fixed', 'Mine Countermeasures (MCM)', '[]', FALSE, 90, 'NATO AAP-6', 'Standard NATO warfare area.'),
('Havadan Bağımsız Tahrik', 'Air-Independent Propulsion (AIP)', '', 'AIP', 'naval', 'fixed', 'Air-Independent Propulsion (AIP)', '[]', FALSE, 92, 'Submarine Tech', 'Submarine propulsion.'),
('Dikey Atım Sistemi', 'Vertical Launching System (VLS)', 'MİDLAS', 'VLS', 'naval', 'fixed', 'Vertical Launching System (VLS)', '[]', FALSE, 92, 'Naval Weapons', 'VLS launcher.'),
('Yakın Hava Savunma Sistemi', 'Close-In Weapon System (CIWS)', 'YHSS', 'CIWS', 'naval', 'fixed', 'Close-In Weapon System (CIWS)', '[]', FALSE, 92, 'Naval Weapons', 'Point defence gun/missile.'),
('Deniz Karakol Uçağı', 'Maritime Patrol Aircraft (MPA)', 'DKU', 'MPA', 'naval', 'fixed', 'Maritime Patrol Aircraft (MPA)', '[]', FALSE, 90, 'MELTEM III', 'Naval aviation.'),
('Deplasman', 'Displacement', '', '', 'naval', 'fixed', 'Displacement', '[]', FALSE, 85, 'Naval Architecture', 'Vessel tonnage.'),

-- ── E. Land Systems & Armoured Vehicles (Land) ───────────────────────────────
('ALTAY', 'ALTAY', 'AMT', 'ALTAY', 'land', 'preserve', 'ALTAY', '["Altay"]', TRUE, 98, 'BMC Official', 'Main Battle Tank.'),
('Ana Muharebe Tankı', 'Main Battle Tank (MBT)', 'AMT', 'MBT', 'land', 'fixed', 'Main Battle Tank (MBT)', '[]', FALSE, 95, 'NATO Armour', 'Standard MBT designation.'),
('TULPAR', 'TULPAR', 'TULPAR', 'TULPAR', 'land', 'preserve', 'TULPAR', '["Tulpar"]', TRUE, 92, 'Otokar Official', 'Tracked IFV / light tank.'),
('KAPLAN', 'KAPLAN', 'KAPLAN', 'KAPLAN', 'land', 'preserve', 'KAPLAN', '["Kaplan"]', TRUE, 92, 'FNSS Official', 'Tracked armoured vehicle family.'),
('PARS', 'PARS', 'PARS', 'PARS', 'land', 'preserve', 'PARS', '["Pars"]', TRUE, 92, 'FNSS Official', '6x6 / 8x8 wheeled armoured family.'),
('PARS ALPHA', 'PARS ALPHA', 'PARS ALPHA', 'PARS ALPHA', 'land', 'preserve', 'PARS ALPHA', '[]', TRUE, 92, 'FNSS Official', 'New-gen 8x8/6x6 IFV.'),
('ARMA', 'ARMA', 'ARMA', 'ARMA', 'land', 'preserve', 'ARMA', '[]', TRUE, 90, 'Otokar Official', '6x6 / 8x8 armoured carrier.'),
('ALTUĞ', 'ALTUĞ', 'ALTUĞ', 'ALTUĞ', 'land', 'preserve', 'ALTUĞ', '["ALTUG"]', TRUE, 90, 'BMC Official', '8x8 armoured vehicle.'),
('KİRPİ II', 'KİRPİ II', 'KİRPİ II', 'KİRPİ II', 'land', 'preserve', 'KİRPİ II', '["KIRPI II"]', TRUE, 90, 'BMC Official', 'MRAP vehicle.'),
('VURAN', 'VURAN', 'VURAN', 'VURAN', 'land', 'preserve', 'VURAN', '[]', TRUE, 88, 'BMC Official', '4x4 tactical wheeled armoured vehicle.'),
('KOBRA II', 'KOBRA II', 'KOBRA II', 'KOBRA II', 'land', 'preserve', 'KOBRA II', '["COBRA II"]', TRUE, 90, 'Otokar Official', 'Use Otokar KOBRA II / COBRA II consistently.'),
('EJDER YALÇIN', 'EJDER YALÇIN', 'EJDER YALÇIN', 'EJDER YALÇIN', 'land', 'preserve', 'EJDER YALÇIN', '["EJDER YALCIN"]', TRUE, 92, 'Nurol Makina', '4x4 armoured combat vehicle.'),
('YÖRÜK 4x4', 'YÖRÜK 4x4', 'NMS', 'YÖRÜK 4x4', 'land', 'preserve', 'YÖRÜK 4x4', '["YORUK 4x4"]', TRUE, 90, 'Nurol Makina', 'Light tactical armoured vehicle.'),
('FIRTINA', 'FIRTINA', 'T-155', 'FIRTINA', 'land', 'preserve', 'FIRTINA', '["Firtina","Storm Howitzer"]', TRUE, 92, 'BMC/TLF', '155 mm self-propelled howitzer; never translate as Storm.'),
('BORAN', 'BORAN', 'BORAN', 'BORAN', 'land', 'preserve', 'BORAN', '[]', TRUE, 90, 'MKE Official', '105 mm air-transportable light towed howitzer.'),
('KORKUT', 'KORKUT', 'KORKUT', 'KORKUT', 'land', 'preserve', 'KORKUT', '[]', TRUE, 92, 'ASELSAN/FNSS', 'Self-propelled 35 mm air defence gun system.'),
('AKKOR', 'AKKOR', 'AKKOR', 'AKKOR', 'land', 'preserve', 'AKKOR', '[]', TRUE, 92, 'ASELSAN Official', 'Active protection system.'),
('Zırhlı Muharebe Aracı', 'Armoured Fighting Vehicle (AFV)', 'ZMA', 'AFV', 'land', 'fixed', 'Armoured Fighting Vehicle (AFV)', '["Armored Combat Vehicle"]', FALSE, 92, 'British English', 'Use British Armoured.'),
('Zırhlı Personel Taşıyıcı', 'Armoured Personnel Carrier (APC)', 'ZPT', 'APC', 'land', 'fixed', 'Armoured Personnel Carrier (APC)', '["Armored Personnel Carrier"]', FALSE, 92, 'British English', 'Use British Armoured.'),
('Mayına Karşı Korumalı Araç', 'Mine-Resistant Ambush Protected (MRAP) Vehicle', 'MKKA', 'MRAP', 'land', 'fixed', 'MRAP Vehicle', '[]', FALSE, 90, 'NATO Terminology', 'Standard MRAP class.'),
('Aktif Koruma Sistemi', 'Active Protection System (APS)', 'AKS', 'APS', 'land', 'fixed', 'Active Protection System (APS)', '[]', FALSE, 90, 'Land Survivability', 'Hard-kill/soft-kill APS.'),
('Uzaktan Komutalı Silah Sistemi', 'Remote Controlled Weapon Station (RCWS)', 'UKSS', 'RCWS', 'land', 'fixed', 'Remote Controlled Weapon Station (RCWS)', '[]', FALSE, 92, 'Land Weapons', 'Standard RCWS.'),
('Reaktif Zırh', 'Explosive Reactive Armour (ERA)', 'ERA', 'ERA', 'land', 'fixed', 'Explosive Reactive Armour (ERA)', '["Reactive Armor"]', FALSE, 88, 'British English', 'British Armour spelling.'),
('Kompozit Zırh', 'Composite Armour', '', '', 'land', 'fixed', 'Composite Armour', '["Composite Armor"]', FALSE, 88, 'British English', 'British Armour spelling.'),
('Güç Grubu', 'Powerpack', '', '', 'land', 'fixed', 'Powerpack', '["Power Group"]', FALSE, 90, 'Propulsion', 'Engine + transmission assembly.'),
('Kundağı Motorlu Obüs', 'Self-Propelled Howitzer (SPH)', 'KMO', 'SPH', 'land', 'fixed', 'Self-Propelled Howitzer (SPH)', '[]', FALSE, 90, 'Artillery', 'FIRTINA class.'),

-- ── F. Munitions, Missiles & Air Defence (Munition) ──────────────────────────
('ÇELİK KUBBE', 'Steel Dome', 'ÇELİK KUBBE', 'Steel Dome', 'munition', 'fixed', 'Steel Dome', '["Iron Dome"]', FALSE, 98, 'SSB/ASELSAN/ROKETSAN', 'Multi-layered national integrated air defence architecture.'),
('SİPER', 'SİPER', 'SİPER', 'SİPER', 'munition', 'preserve', 'SİPER', '["SIPER","Siper"]', TRUE, 98, 'ASELSAN/ROKETSAN/SAGE', 'Long-range regional air and missile defence system.'),
('HİSAR', 'HİSAR', 'HİSAR', 'HİSAR', 'munition', 'preserve', 'HİSAR', '["HISAR","Hisar"]', TRUE, 98, 'ASELSAN/ROKETSAN', 'HİSAR-A+ / HİSAR-O+ air defence missile family.'),
('HİSAR-A+', 'HİSAR-A+', 'HİSAR-A+', 'HİSAR-A+', 'munition', 'preserve', 'HİSAR-A+', '["HISAR-A+"]', TRUE, 95, 'ASELSAN/ROKETSAN', 'Low-altitude air defence.'),
('HİSAR-O+', 'HİSAR-O+', 'HİSAR-O+', 'HİSAR-O+', 'munition', 'preserve', 'HİSAR-O+', '["HISAR-O+"]', TRUE, 95, 'ASELSAN/ROKETSAN', 'Medium-altitude air defence.'),
('SUNGUR', 'SUNGUR', 'SUNGUR', 'SUNGUR', 'munition', 'preserve', 'SUNGUR', '["Sungur"]', TRUE, 92, 'ROKETSAN Official', 'MANPADS / short-range air defence missile.'),
('GÖKDOĞAN', 'GÖKDOĞAN', 'GÖKDOĞAN', 'GÖKDOĞAN', 'munition', 'preserve', 'GÖKDOĞAN', '["GOKDOGAN"]', TRUE, 95, 'TÜBİTAK SAGE', 'BVR active-RF air-to-air missile.'),
('BOZDOĞAN', 'BOZDOĞAN', 'BOZDOĞAN', 'BOZDOĞAN', 'munition', 'preserve', 'BOZDOĞAN', '["BOZDOGAN"]', TRUE, 95, 'TÜBİTAK SAGE', 'Within-visual-range IIR air-to-air missile.'),
('GÖKHAN', 'GÖKHAN', 'GÖKHAN', 'GÖKHAN', 'munition', 'preserve', 'GÖKHAN', '["GOKHAN"]', TRUE, 92, 'TÜBİTAK SAGE', 'Ramjet BVR air-to-air missile.'),
('ATMACA', 'ATMACA', 'ATMACA', 'ATMACA', 'munition', 'preserve', 'ATMACA', '["Atmaca"]', TRUE, 98, 'ROKETSAN Official', 'Long-range anti-ship missile.'),
('ÇAKIR', 'ÇAKIR', 'ÇAKIR', 'ÇAKIR', 'munition', 'preserve', 'ÇAKIR', '["CAKIR"]', TRUE, 95, 'ROKETSAN Official', 'Compact cruise missile.'),
('SOM', 'SOM', 'SOM', 'SOM', 'munition', 'preserve', 'SOM', '[]', TRUE, 98, 'ROKETSAN/SAGE', 'Stand-Off Missile (SOM / SOM-J).'),
('GEZGİN', 'GEZGİN', 'GEZGİN', 'GEZGİN', 'munition', 'preserve', 'GEZGİN', '["GEZGIN"]', TRUE, 94, 'ROKETSAN', 'Long-range strategic cruise missile.'),
('TAYFUN', 'TAYFUN', 'TAYFUN', 'TAYFUN', 'munition', 'preserve', 'TAYFUN', '[]', TRUE, 95, 'ROKETSAN Official', 'Short-range ballistic missile.'),
('BORA', 'BORA', 'BORA', 'BORA', 'munition', 'preserve', 'BORA', '["KHAN"]', TRUE, 92, 'ROKETSAN Official', 'Tactical ballistic missile.'),
('İHA-230', 'İHA-230', 'İHA-230', 'İHA-230', 'munition', 'preserve', 'İHA-230', '["UAV-230"]', TRUE, 92, 'ROKETSAN Official', 'Supersonic air-to-surface ballistic missile.'),
('İHA-300', 'İHA-300', 'İHA-300', 'İHA-300', 'munition', 'preserve', 'İHA-300', '["UAV-300"]', TRUE, 92, 'ROKETSAN Official', 'Long-range supersonic UAV-launched missile.'),
('MAM-L', 'MAM-L', 'MAM-L', 'MAM-L', 'munition', 'preserve', 'MAM-L', '[]', TRUE, 92, 'ROKETSAN Official', 'Smart micro munition.'),
('MAM-C', 'MAM-C', 'MAM-C', 'MAM-C', 'munition', 'preserve', 'MAM-C', '[]', TRUE, 90, 'ROKETSAN Official', 'Smart micro munition.'),
('MAM-T', 'MAM-T', 'MAM-T', 'MAM-T', 'munition', 'preserve', 'MAM-T', '[]', TRUE, 92, 'ROKETSAN Official', 'Smart micro munition.'),
('TOLUN', 'TOLUN', 'TOLUN', 'TOLUN', 'munition', 'preserve', 'TOLUN', '[]', TRUE, 92, 'ASELSAN Official', 'Guided miniature bomb.'),
('TEBER', 'TEBER', 'TEBER', 'TEBER', 'munition', 'preserve', 'TEBER', '[]', TRUE, 90, 'ROKETSAN Official', 'Laser guidance kit.'),
('HGK', 'HGK', 'HGK', 'HGK', 'munition', 'preserve', 'HGK', '[]', TRUE, 90, 'TÜBİTAK SAGE', 'Precision guidance kit.'),
('KGK', 'KGK', 'KGK', 'KGK', 'munition', 'preserve', 'KGK', '[]', TRUE, 90, 'TÜBİTAK SAGE', 'Winged guidance kit.'),
('LGK', 'LGK', 'LGK', 'LGK', 'munition', 'preserve', 'LGK', '[]', TRUE, 90, 'ASELSAN Official', 'Laser guidance kit.'),
('UMTAS', 'UMTAS', 'UMTAS', 'UMTAS', 'munition', 'preserve', 'UMTAS', '[]', TRUE, 90, 'ROKETSAN Official', 'Long-range anti-tank missile.'),
('OMTAS', 'OMTAS', 'OMTAS', 'OMTAS', 'munition', 'preserve', 'OMTAS', '[]', TRUE, 90, 'ROKETSAN Official', 'Medium-range anti-tank missile.'),
('KARAOK', 'KARAOK', 'KARAOK', 'KARAOK', 'munition', 'preserve', 'KARAOK', '[]', TRUE, 90, 'ROKETSAN Official', 'Short-range fire-and-forget ATGM.'),
('AKYA', 'AKYA', 'AKYA', 'AKYA', 'munition', 'preserve', 'AKYA', '[]', TRUE, 92, 'ROKETSAN Official', 'New-generation heavyweight torpedo.'),
('ORKA', 'ORKA', 'ORKA', 'ORKA', 'munition', 'preserve', 'ORKA', '[]', TRUE, 90, 'ROKETSAN Official', 'Lightweight torpedo.'),
('KUZGUN', 'KUZGUN', 'KUZGUN', 'KUZGUN', 'munition', 'preserve', 'KUZGUN', '[]', TRUE, 90, 'TÜBİTAK SAGE', 'Modular joint munition.'),
('KEMANKEŞ', 'KEMANKEŞ', 'KEMANKEŞ', 'KEMANKEŞ', 'munition', 'preserve', 'KEMANKEŞ', '["KEMANKES"]', TRUE, 90, 'Baykar Official', 'Mini smart cruise missile.'),
('Hava Savunma Sistemi', 'Air Defence System', 'HSS', 'ADS', 'munition', 'fixed', 'Air Defence System', '["Air Defense System"]', FALSE, 98, 'British English', 'Use Air Defence System.'),
('Katmanlı Hava Savunma', 'Multi-Layered Air Defence', '', '', 'munition', 'fixed', 'Multi-Layered Air Defence', '["Layered Air Defense"]', FALSE, 95, 'British English', 'ÇELİK KUBBE architecture.'),
('Seyir Füzesi', 'Cruise Missile', '', '', 'munition', 'fixed', 'Cruise Missile', '[]', FALSE, 90, 'NATO Terminology', 'Standard munition term.'),
('Balistik Füze', 'Ballistic Missile', '', '', 'munition', 'fixed', 'Ballistic Missile', '[]', FALSE, 90, 'NATO Terminology', 'Standard munition term.'),
('Gemisavar Füze', 'Anti-Ship Missile (AShM)', '', 'AShM', 'munition', 'fixed', 'Anti-Ship Missile (AShM)', '[]', FALSE, 90, 'Naval Weapons', 'ATMACA class.'),
('Havadan Havaya Füze', 'Air-to-Air Missile (AAM)', '', 'AAM', 'munition', 'fixed', 'Air-to-Air Missile (AAM)', '[]', FALSE, 90, 'Air Weapons', 'GÖKDOĞAN/BOZDOĞAN class.'),
('Havadan Karaya Füze', 'Air-to-Surface Missile (ASM)', '', 'ASM', 'munition', 'fixed', 'Air-to-Surface Missile (ASM)', '[]', FALSE, 90, 'Air Weapons', 'SOM/İHA-230 class.'),
('Tanksavar Füzesi', 'Anti-Tank Guided Missile (ATGM)', '', 'ATGM', 'munition', 'fixed', 'Anti-Tank Guided Missile (ATGM)', '[]', FALSE, 90, 'Land Weapons', 'OMTAS/UMTAS/KARAOK class.'),
('Arayıcı Başlık', 'Seeker Head', '', '', 'munition', 'fixed', 'Seeker Head', '[]', FALSE, 88, 'Missile Guidance', 'IIR/RF seeker.'),
('Güdüm Kiti', 'Guidance Kit', '', '', 'munition', 'fixed', 'Guidance Kit', '[]', FALSE, 88, 'Munitions', 'HGK/KGK/LGK/TEBER.'),
('Harp Başlığı', 'Warhead', '', '', 'munition', 'fixed', 'Warhead', '[]', FALSE, 88, 'Munitions', 'Fragmentation/tandem/penetrator warhead.'),
('Ateşle ve Unut', 'Fire-and-Forget', '', '', 'munition', 'fixed', 'Fire-and-Forget', '[]', FALSE, 85, 'Missile Guidance', 'Autonomous terminal guidance.'),

-- ── G. Radar, EW, EO/IR, C4ISR & Propulsion (Radar & Subsystems) ─────────────
('AESA Radar', 'AESA Radar', 'AESA', 'AESA', 'radar_ew', 'preserve', 'AESA Radar', '[]', TRUE, 95, 'ASELSAN Official', 'Active Electronically Scanned Array.'),
('MURAD AESA', 'MURAD AESA', 'MURAD', 'MURAD AESA', 'radar_ew', 'preserve', 'MURAD AESA', '[]', TRUE, 96, 'ASELSAN Official', 'Airborne fire-control AESA radar.'),
('ÇAFRAD', 'ÇAFRAD', 'ÇAFRAD', 'ÇAFRAD', 'radar_ew', 'preserve', 'ÇAFRAD', '["CAFRAD"]', TRUE, 95, 'ASELSAN Official', 'Multifunction phased array naval radar.'),
('ALP-300G', 'ALP-300G', 'ALP-300G', 'ALP-300G', 'radar_ew', 'preserve', 'ALP-300G', '["EİRS"]', TRUE, 92, 'ASELSAN Official', 'Long-range early warning radar.'),
('ALP-100G', 'ALP-100G', 'ALP-100G', 'ALP-100G', 'radar_ew', 'preserve', 'ALP-100G', '[]', TRUE, 90, 'ASELSAN Official', 'Low-altitude radar.'),
('CENK-S', 'CENK-S', 'CENK-S', 'CENK-S', 'radar_ew', 'preserve', 'CENK-S', '[]', TRUE, 90, 'ASELSAN Official', 'S-band 3D naval search radar.'),
('KORAL', 'KORAL', 'KORAL', 'KORAL', 'radar_ew', 'preserve', 'KORAL', '[]', TRUE, 95, 'ASELSAN Official', 'Mobile land-based electronic warfare system.'),
('REDET', 'REDET', 'REDET', 'REDET', 'radar_ew', 'preserve', 'REDET', '[]', TRUE, 90, 'ASELSAN Official', 'Radar electronic support / attack system.'),
('VURAL', 'VURAL', 'VURAL', 'VURAL', 'radar_ew', 'preserve', 'VURAL', '[]', TRUE, 90, 'ASELSAN Official', 'Radar EW system.'),
('ASELFLIR-500', 'ASELFLIR-500', 'ASELFLIR-500', 'ASELFLIR-500', 'radar_ew', 'preserve', 'ASELFLIR-500', '[]', TRUE, 94, 'ASELSAN Official', 'High-performance EO/IR targeting pod.'),
('CATS', 'CATS', 'CATS', 'CATS', 'radar_ew', 'preserve', 'CATS', '[]', TRUE, 92, 'ASELSAN Official', 'Common Aperture Targeting System.'),
('TOYGUN', 'TOYGUN', 'TOYGUN', 'TOYGUN', 'radar_ew', 'preserve', 'TOYGUN', '[]', TRUE, 92, 'ASELSAN Official', 'KAAN EOTS electro-optical targeting system.'),
('KARAT', 'KARAT', 'KARAT', 'KARAT', 'radar_ew', 'preserve', 'KARAT', '[]', TRUE, 90, 'ASELSAN Official', 'Infrared Search and Track (IRST).'),
('YILDIRIM', 'YILDIRIM', 'YILDIRIM', 'YILDIRIM', 'radar_ew', 'preserve', 'YILDIRIM', '[]', TRUE, 90, 'ASELSAN Official', 'DIRCM (Directed Infrared Countermeasures).'),
('İHTAR', 'İHTAR', 'İHTAR', 'İHTAR', 'radar_ew', 'preserve', 'İHTAR', '["IHTAR"]', TRUE, 90, 'ASELSAN Official', 'Anti-drone defence system.'),
('KANGAL', 'KANGAL', 'KANGAL', 'KANGAL', 'radar_ew', 'preserve', 'KANGAL', '[]', TRUE, 88, 'ASELSAN Official', 'RF jammer system.'),
('Elektronik Harp', 'Electronic Warfare (EW)', 'EH', 'EW', 'radar_ew', 'fixed', 'Electronic Warfare (EW)', '[]', FALSE, 95, 'NATO AAP-6', 'Standard EW term.'),
('Elektronik Destek', 'Electronic Support (ES)', 'ED', 'ES', 'radar_ew', 'fixed', 'Electronic Support (ES)', '[]', FALSE, 90, 'NATO EW', 'ESM/ES.'),
('Elektronik Taarruz', 'Electronic Attack (EA)', 'ET', 'EA', 'radar_ew', 'fixed', 'Electronic Attack (EA)', '[]', FALSE, 90, 'NATO EW', 'ECM/EA.'),
('Elektronik Korunma', 'Electronic Protection (EP)', 'EK', 'EP', 'radar_ew', 'fixed', 'Electronic Protection (EP)', '[]', FALSE, 90, 'NATO EW', 'ECCM/EP.'),
('Sinyal İstihbaratı', 'Signals Intelligence (SIGINT)', 'SIGINT', 'SIGINT', 'radar_ew', 'fixed', 'Signals Intelligence (SIGINT)', '[]', FALSE, 90, 'NATO Intelligence', 'SIGINT.'),
('Elektro-Optik / Kızılötesi', 'Electro-Optical / Infrared (EO/IR)', 'EO/IR', 'EO/IR', 'radar_ew', 'fixed', 'Electro-Optical / Infrared (EO/IR)', '[]', FALSE, 92, 'Sensor Tech', 'Standard EO/IR.'),
('Dost-Düşman Tanıma', 'Identification Friend or Foe (IFF)', 'IFF', 'IFF', 'radar_ew', 'fixed', 'Identification Friend or Foe (IFF)', '[]', FALSE, 90, 'NATO Avionics', 'IFF Mode 5.'),
('Veri Bağı', 'Data Link', '', '', 'radar_ew', 'fixed', 'Data Link', '[]', FALSE, 88, 'C4ISR', 'Tactical data link.'),
('Uydu Haberleşmesi', 'Satellite Communications (SATCOM)', 'SATCOM', 'SATCOM', 'radar_ew', 'fixed', 'Satellite Communications (SATCOM)', '[]', FALSE, 92, 'CTech/ASELSAN', 'SATCOM.'),
('Komuta Kontrol Muhabere Bilgisayar İstihbarat Gözetleme Keşif', 'Command, Control, Communications, Computers, Intelligence, Surveillance and Reconnaissance (C4ISR)', 'C4ISR', 'C4ISR', 'radar_ew', 'fixed', 'C4ISR', '[]', FALSE, 94, 'NATO C4ISR', 'Joint C4ISR.'),
('TEI-TS1400', 'TEI-TS1400', 'TS1400', 'TEI-TS1400', 'radar_ew', 'preserve', 'TEI-TS1400', '[]', TRUE, 95, 'TEI Official', 'GÖKBEY turboshaft engine.'),
('TEI-PD170', 'TEI-PD170', 'PD170', 'TEI-PD170', 'radar_ew', 'preserve', 'TEI-PD170', '[]', TRUE, 94, 'TEI Official', 'UAV turbodiesel engine.'),
('TEI-TF6000', 'TEI-TF6000', 'TF6000', 'TEI-TF6000', 'radar_ew', 'preserve', 'TEI-TF6000', '[]', TRUE, 94, 'TEI Official', 'Turbofan engine for ANKA III/KIZILELMA.'),
('KTJ-3200', 'KTJ-3200', 'KTJ-3200', 'KTJ-3200', 'radar_ew', 'preserve', 'KTJ-3200', '[]', TRUE, 92, 'Kale Arge Official', 'Turbojet missile engine for SOM/ATMACA.'),
('BATU', 'BATU', 'BATU', 'BATU', 'radar_ew', 'preserve', 'BATU', '[]', TRUE, 92, 'BMC Power Official', '1500 hp MBT powerpack.'),
('UTKU', 'UTKU', 'UTKU', 'UTKU', 'radar_ew', 'preserve', 'UTKU', '[]', TRUE, 90, 'BMC Power Official', '1000 hp armoured vehicle powerpack.'),

-- ── H. Defence & Aerospace Prime Contractors / OEMs (OEM) ────────────────────
('ASELSAN', 'ASELSAN', 'ASELSAN', 'ASELSAN', 'oem', 'preserve', 'ASELSAN', '["Aselsan"]', TRUE, 100, 'Company Official', 'Always uppercase ASELSAN.'),
('TUSAŞ', 'Turkish Aerospace (TUSAŞ)', 'TUSAŞ', 'TUSAŞ', 'oem', 'fixed', 'Turkish Aerospace (TUSAŞ)', '["TAI","TUSAS"]', TRUE, 100, 'Company Official', 'Official global corporate brand is Turkish Aerospace (TUSAŞ).'),
('ROKETSAN', 'ROKETSAN', 'ROKETSAN', 'ROKETSAN', 'oem', 'preserve', 'ROKETSAN', '["Roketsan"]', TRUE, 100, 'Company Official', 'Always uppercase ROKETSAN.'),
('HAVELSAN', 'HAVELSAN', 'HAVELSAN', 'HAVELSAN', 'oem', 'preserve', 'HAVELSAN', '["Havelsan"]', TRUE, 100, 'Company Official', 'Always uppercase HAVELSAN.'),
('STM', 'STM', 'STM', 'STM', 'oem', 'preserve', 'STM', '[]', TRUE, 100, 'Company Official', 'Always uppercase STM.'),
('TEI', 'TEI (TUSAŞ Engine Industries)', 'TEI', 'TEI', 'oem', 'fixed', 'TEI (TUSAŞ Engine Industries)', '[]', TRUE, 100, 'Company Official', 'Official corporate designation.'),
('FNSS', 'FNSS', 'FNSS', 'FNSS', 'oem', 'preserve', 'FNSS', '[]', TRUE, 98, 'Company Official', 'Always uppercase FNSS.'),
('BMC', 'BMC', 'BMC', 'BMC', 'oem', 'preserve', 'BMC', '[]', TRUE, 98, 'Company Official', 'BMC Defence / BMC Otomotiv.'),
('BAYKAR', 'BAYKAR', 'BAYKAR', 'BAYKAR', 'oem', 'preserve', 'BAYKAR', '[]', TRUE, 98, 'Company Official', 'BAYKAR / Baykar.'),
('ARES Tersanesi', 'ARES Shipyard', 'ARES', 'ARES Shipyard', 'oem', 'fixed', 'ARES Shipyard', '["ARES Tersanesi"]', FALSE, 96, 'Company Official', 'Official English trade name.'),
('CTech', 'CTech', 'CTech', 'CTech', 'oem', 'preserve', 'CTech', '["C-Tech","CTECH"]', TRUE, 96, 'Company Official', 'Preserve camelCase CTech.'),
('Meteksan Savunma', 'Meteksan Defence', 'Meteksan', 'Meteksan Defence', 'oem', 'fixed', 'Meteksan Defence', '["Meteksan Defense"]', FALSE, 95, 'Company Official', 'Use British Defence.'),
('OTOKAR', 'Otokar', 'OTOKAR', 'Otokar', 'oem', 'preserve', 'Otokar', '[]', FALSE, 94, 'Company Official', 'Land systems OEM.'),
('Nurol Makina', 'Nurol Makina', '', '', 'oem', 'preserve', 'Nurol Makina', '[]', TRUE, 92, 'Company Official', '4x4 armoured vehicle OEM.'),
('SARSILMAZ', 'SARSILMAZ', 'SARSILMAZ', 'SARSILMAZ', 'oem', 'preserve', 'SARSILMAZ', '["Sarsilmaz"]', TRUE, 95, 'Company Official', 'Preserve uppercase + I.'),
('CANiK', 'CANiK (Samsun Yurt Savunma)', 'SYS', 'CANiK', 'oem', 'preserve', 'CANiK', '["Canik"]', TRUE, 94, 'Company Official', 'Brand styling CANiK.'),
('YALTES', 'YALTES', 'YALTES', 'YALTES', 'oem', 'preserve', 'YALTES', '[]', TRUE, 94, 'Company Official', 'Naval electronics / IPMS.'),
('SEFİNE Tersanesi', 'SEFİNE Shipyard', 'SEFİNE', 'SEFİNE Shipyard', 'oem', 'fixed', 'SEFİNE Shipyard', '["Sefine Tersanesi"]', FALSE, 92, 'Company Official', 'Naval shipyard.'),
('TAİS', 'TAIS Shipyards', 'TAİS', 'TAIS', 'oem', 'fixed', 'TAIS Shipyards', '[]', TRUE, 92, 'Company Official', 'Shipbuilding consortium.'),
('DEARSAN', 'DEARSAN Shipyard', 'DEARSAN', 'DEARSAN', 'oem', 'fixed', 'DEARSAN Shipyard', '[]', TRUE, 90, 'Company Official', 'Naval shipyard.'),
('DESAN Tersanesi', 'DESAN Shipyard', 'DESAN', 'DESAN', 'oem', 'fixed', 'DESAN Shipyard', '[]', FALSE, 90, 'Company Official', 'Naval shipyard.'),
('Anadolu Tersanesi', 'ADIK (Anadolu Shipyard)', 'ADİK', 'ADIK', 'oem', 'fixed', 'ADIK / Anadolu Shipyard', '[]', FALSE, 90, 'Company Official', 'Naval shipyard.'),
('ÜNİMETAL', 'ÜNİMETAL', 'ÜNİMETAL', 'ÜNİMETAL', 'oem', 'preserve', 'ÜNİMETAL', '["UNIMETAL","Unimetal"]', TRUE, 92, 'Company Official', 'Preserve Turkish diacritics Ü and İ.'),
('SAĞLAMTEK', 'SAĞLAMTEK', 'SAĞLAMTEK', 'SAĞLAMTEK', 'oem', 'preserve', 'SAĞLAMTEK', '["SAGLAMTEK","Saglamtek"]', TRUE, 92, 'Company Official', 'Preserve Ğ in SAĞLAMTEK.'),
('MilSOFT', 'MilSOFT', 'MilSOFT', 'MilSOFT', 'oem', 'preserve', 'MilSOFT', '[]', TRUE, 90, 'Company Official', 'Preserve camelCase MilSOFT.'),
('SDT Uzay ve Savunma', 'SDT Space & Defence', 'SDT', 'SDT Space & Defence', 'oem', 'fixed', 'SDT Space & Defence', '["SDT Defense"]', FALSE, 92, 'Company Official', 'British Defence spelling.'),
('AYESAŞ', 'AYESAŞ', 'AYESAŞ', 'AYESAŞ', 'oem', 'preserve', 'AYESAŞ', '["AYESAS"]', TRUE, 90, 'Company Official', 'Avionics & C4I.'),
('BİTES', 'BİTES', 'BİTES', 'BİTES', 'oem', 'preserve', 'BİTES', '["BITES"]', TRUE, 90, 'Company Official', 'Simulation & training software.'),
('KoçSavunma', 'KoçSavunma', '', '', 'oem', 'preserve', 'KoçSavunma', '["KocSavunma"]', TRUE, 90, 'Company Official', 'Underwater acoustics & naval systems.'),
('ULAK Haberleşme', 'ULAK Communications', 'ULAK', 'ULAK Communications', 'oem', 'fixed', 'ULAK Communications', '[]', FALSE, 90, 'Company Official', '5G / tactical communications.'),
('Kale Arge', 'Kale Arge', '', '', 'oem', 'preserve', 'Kale Arge', '[]', TRUE, 92, 'Company Official', 'Turbojet missile engines.'),
('Lentatek', 'Lentatek', '', '', 'oem', 'preserve', 'Lentatek', '[]', TRUE, 88, 'Company Official', 'UAV & hydrogen fuel cells.'),
('Titra Teknoloji', 'Titra Technology', 'Titra', 'Titra Technology', 'oem', 'fixed', 'Titra Technology', '[]', FALSE, 88, 'Company Official', 'Unmanned helicopter ALPIN.'),
('Repkon', 'Repkon', '', '', 'oem', 'preserve', 'Repkon', '[]', TRUE, 88, 'Company Official', 'Flowforming & ammunition lines.'),
('Asisguard', 'Asisguard', '', '', 'oem', 'preserve', 'Asisguard', '[]', TRUE, 88, 'Company Official', 'SONGAR armed drone & vehicle optronics.'),
('MILMAST', 'MILMAST', 'MILMAST', 'MILMAST', 'oem', 'preserve', 'MILMAST', '[]', TRUE, 88, 'Company Official', 'Telescopic tactical masts.'),
('Nero Endüstri', 'Nero Industry', '', '', 'oem', 'fixed', 'Nero Industry', '[]', FALSE, 88, 'Company Official', 'CBRN & APU systems.'),

-- ── I. MSI Editorial Sections, Publishing & Exhibitions (Editorial) ──────────
('Kapak Dosyası', 'Cover Story', '', '', 'editorial', 'fixed', 'Cover Story', '["Cover File"]', FALSE, 98, 'MSI Editorial', 'Official MSI TDR magazine section name.'),
('Özel Dosya', 'Special Dossier', '', '', 'editorial', 'fixed', 'Special Dossier', '["Special File"]', FALSE, 98, 'MSI Editorial', 'Official MSI TDR section name.'),
('Başyazı', 'Editorial', '', '', 'editorial', 'fixed', 'Editorial', '["Lead Article"]', FALSE, 98, 'MSI Editorial', 'Monthly editorial commentary.'),
('Söyleşi', 'Interview', '', '', 'editorial', 'fixed', 'Interview', '["Conversation"]', FALSE, 98, 'MSI Editorial', 'Executive interview.'),
('Etkinlik Dosyası', 'Event Dossier', '', '', 'editorial', 'fixed', 'Event Dossier', '["Activity File"]', FALSE, 96, 'MSI Editorial', 'Defence exhibition dossier.'),
('Etkinlik Özel Sayısı', 'Event Special Issue', '', '', 'editorial', 'fixed', 'Event Special Issue', '[]', FALSE, 95, 'MSI Editorial', 'Special exhibition print edition.'),
('Etkinlik Sonrası Özel Dosya', 'Post-Event Special Dossier', '', '', 'editorial', 'fixed', 'Post-Event Special Dossier', '[]', FALSE, 95, 'MSI Editorial', 'Post-exhibition coverage.'),
('Tanıtıcı Makale', 'Technical Showcase', '', '', 'editorial', 'fixed', 'Technical Showcase', '["Promotional Article"]', FALSE, 94, 'MSI Editorial', 'Technical system showcase.'),
('Makale / Analiz', 'Article & Analysis', '', '', 'editorial', 'fixed', 'Article & Analysis', '[]', FALSE, 94, 'MSI Editorial', 'Strategic analysis section.'),
('Özel Haber', 'Exclusive Report', '', '', 'editorial', 'fixed', 'Exclusive Report', '["Special News"]', FALSE, 94, 'MSI Editorial', 'Exclusive news.'),
('Mesajlar', 'Official Messages', '', '', 'editorial', 'fixed', 'Official Messages', '["Messages"]', FALSE, 92, 'MSI Editorial', 'Official statements & forewords.'),
('Dergi İçerikleri', 'Magazine Features', '', '', 'editorial', 'fixed', 'Magazine Features', '["Magazine Contents"]', FALSE, 92, 'MSI Editorial', 'Print magazine archive.'),
('TSK''dan Haberler', 'Turkish Armed Forces News', '', '', 'editorial', 'fixed', 'Turkish Armed Forces News', '[]', FALSE, 92, 'MSI Editorial', 'TAF news section.'),
('Künye', 'Masthead', '', '', 'editorial', 'fixed', 'Masthead', '["Imprint"]', FALSE, 90, 'MSI Editorial', 'Editorial masthead.'),
('Yayın Kurulu', 'Editorial Board', '', '', 'editorial', 'fixed', 'Editorial Board', '[]', FALSE, 90, 'MSI Editorial', 'Editorial board.'),
('Genel Yayın Yönetmeni', 'Editor-in-Chief', '', '', 'editorial', 'fixed', 'Editor-in-Chief', '["General Publication Director"]', FALSE, 92, 'MSI Editorial', 'Masthead title.'),
('Yazı İşleri Müdürü', 'Managing Editor', '', '', 'editorial', 'fixed', 'Managing Editor', '[]', FALSE, 92, 'MSI Editorial', 'Masthead title.'),
('Sayı', 'Issue', '', '', 'editorial', 'fixed', 'Issue', '["Number"]', FALSE, 88, 'MSI Editorial', 'Magazine issue unit.'),
('IDEF', 'IDEF (International Defence Industry Fair)', 'IDEF', 'IDEF', 'editorial', 'preserve', 'IDEF', '["IDEF Defense Fair"]', TRUE, 96, 'Exhibition Official', 'Istanbul biennial defence exhibition.'),
('SAHA EXPO', 'SAHA EXPO', 'SAHA EXPO', 'SAHA EXPO', 'editorial', 'preserve', 'SAHA EXPO', '[]', TRUE, 96, 'Exhibition Official', 'Istanbul aerospace & defence exhibition.'),
('SEDEC', 'SEDEC', 'SEDEC', 'SEDEC', 'editorial', 'preserve', 'SEDEC', '[]', TRUE, 92, 'Exhibition Official', 'Ankara homeland & border security exhibition.'),
('İGEF', 'IGEF', 'İGEF', 'IGEF', 'editorial', 'fixed', 'IGEF', '[]', TRUE, 90, 'Exhibition Official', 'Internal security exhibition.'),
('MSPO', 'MSPO', 'MSPO', 'MSPO', 'editorial', 'preserve', 'MSPO', '[]', TRUE, 92, 'Exhibition Official', 'Kielce Poland defence exhibition.'),
('Eurosatory', 'Eurosatory', 'Eurosatory', 'Eurosatory', 'editorial', 'preserve', 'Eurosatory', '[]', TRUE, 92, 'Exhibition Official', 'Paris land & airland defence exhibition.'),
('DSEI', 'DSEI', 'DSEI', 'DSEI', 'editorial', 'preserve', 'DSEI', '[]', TRUE, 92, 'Exhibition Official', 'London defence exhibition.'),
('IDEX', 'IDEX', 'IDEX', 'IDEX', 'editorial', 'preserve', 'IDEX', '[]', TRUE, 92, 'Exhibition Official', 'Abu Dhabi defence exhibition.'),
('DIMDEX', 'DIMDEX', 'DIMDEX', 'DIMDEX', 'editorial', 'preserve', 'DIMDEX', '[]', TRUE, 90, 'Exhibition Official', 'Doha naval defence exhibition.'),

-- ── J. Procurement, Testing, Operations & Engineering (Operations) ───────────
('Envantere Giriş', 'Induction into Inventory', '', '', 'operations', 'fixed', 'Induction into Inventory', '[]', FALSE, 90, 'Procurement', 'Delivery to armed forces.'),
('Seri Üretim Sözleşmesi', 'Serial Production Contract', '', '', 'operations', 'fixed', 'Serial Production Contract', '["Mass Production"]', FALSE, 92, 'Procurement', 'Formal procurement milestone.'),
('İlk Uçuş', 'Maiden Flight', '', '', 'operations', 'fixed', 'Maiden Flight', '["First Flight"]', FALSE, 90, 'Flight Test', 'Formal aerospace milestone.'),
('Atış Testi', 'Firing Trial', '', '', 'operations', 'fixed', 'Firing Trial', '["Shooting Test"]', FALSE, 88, 'Weapons Test', 'Live firing evaluation.'),
('Fabrika Kabul Testi', 'Factory Acceptance Test (FAT)', 'FAT', 'FAT', 'operations', 'fixed', 'Factory Acceptance Test (FAT)', '[]', FALSE, 88, 'Quality Assurance', 'FAT milestone.'),
('Deniz Kabul Testleri', 'Sea Acceptance Trials (SAT)', 'SAT', 'SAT', 'operations', 'fixed', 'Sea Acceptance Trials (SAT)', '[]', FALSE, 88, 'Naval QA', 'SAT milestone.'),
('Teknoloji Hazırlık Seviyesi', 'Technology Readiness Level (TRL)', 'THS', 'TRL', 'operations', 'fixed', 'Technology Readiness Level (TRL)', '[]', FALSE, 88, 'R&D Standard', 'TRL scale 1-9.'),
('İhracat Sözleşmesi', 'Export Contract', '', '', 'operations', 'fixed', 'Export Contract', '[]', FALSE, 88, 'Defence Trade', 'International export agreement.'),
('Ortak Üretim', 'Co-Production', '', '', 'operations', 'fixed', 'Co-Production', '[]', FALSE, 88, 'Industrial Cooperation', 'Joint manufacturing.'),
('Teknoloji Transferi', 'Technology Transfer (ToT)', 'ToT', 'ToT', 'operations', 'fixed', 'Technology Transfer (ToT)', '[]', FALSE, 88, 'Industrial Cooperation', 'Transfer of technology.'),
('Yerlileştirme', 'Indigenisation', '', '', 'operations', 'fixed', 'Indigenisation', '["Indigenization"]', FALSE, 88, 'British English', 'British -isation spelling.'),
('Millîleştirme', 'Nationalisation / Indigenisation', '', '', 'operations', 'fixed', 'Indigenisation', '[]', FALSE, 88, 'British English', 'Domestic subsystem replacement.'),
('Tedarik Zinciri', 'Supply Chain', '', '', 'operations', 'fixed', 'Supply Chain', '[]', FALSE, 88, 'Industry', 'Tier-1/Tier-2/SME supply chain.'),
('Ana Yüklenici', 'Prime Contractor', '', '', 'operations', 'fixed', 'Prime Contractor', '[]', FALSE, 90, 'Procurement', 'Lead OEM.'),
('Alt Yüklenici', 'Subcontractor', '', '', 'operations', 'fixed', 'Subcontractor', '[]', FALSE, 88, 'Procurement', 'Tier-2/3 supplier.'),
('Mutabakat Zaptı', 'Memorandum of Understanding (MoU)', 'MoU', 'MoU', 'operations', 'fixed', 'Memorandum of Understanding (MoU)', '[]', FALSE, 90, 'Diplomacy/Contracts', 'Standard MoU.'),
('Niyet Mektubu', 'Letter of Intent (LoI)', 'LoI', 'LoI', 'operations', 'fixed', 'Letter of Intent (LoI)', '[]', FALSE, 88, 'Procurement', 'Standard LoI.'),
('Bilgi İstek Dokümanı', 'Request for Information (RFI)', 'BİD', 'RFI', 'operations', 'fixed', 'Request for Information (RFI)', '[]', FALSE, 88, 'SSB Procurement', 'RFI.'),
('Teklife Çağrı Dosyası', 'Request for Proposals (RFP)', 'TÇD', 'RFP', 'operations', 'fixed', 'Request for Proposals (RFP)', '[]', FALSE, 90, 'SSB Procurement', 'RFP.'),
('Siber Güvenlik', 'Cyber Security', '', '', 'operations', 'fixed', 'Cyber Security', '[]', FALSE, 88, 'NATO C4ISR', 'NATO style Cyber Security.'),
('Yapay Zekâ', 'Artificial Intelligence (AI)', 'YZ', 'AI', 'operations', 'fixed', 'Artificial Intelligence (AI)', '[]', FALSE, 88, 'Tech', 'AI.'),
('Sürü Zekâsı', 'Swarm Intelligence', '', '', 'operations', 'fixed', 'Swarm Intelligence', '[]', FALSE, 88, 'Autonomy', 'Swarm UAV/USV autonomy.'),
('Otonom Seyrüsefer', 'Autonomous Navigation', '', '', 'operations', 'fixed', 'Autonomous Navigation', '[]', FALSE, 88, 'Autonomy', 'GNSS-denied navigation.'),
('Karıştırmaya Dayanıklı Küresel Konumlama', 'Anti-Jam GNSS', '', '', 'operations', 'fixed', 'Anti-Jam GNSS', '[]', FALSE, 88, 'EW/Nav', 'CRPA antenna & anti-jam receiver.'),
('Mühimmat Bırakma', 'Weapon Release', '', '', 'operations', 'fixed', 'Weapon Release', '[]', FALSE, 85, 'Flight Test', 'Store separation.'),
('Lazer Hedef İşaretleme', 'Laser Target Designation', '', '', 'operations', 'fixed', 'Laser Target Designation', '[]', FALSE, 88, 'EO/IR', 'Precision targeting.'),
('Askerî Standart', 'Military Standard (MIL-STD)', 'MIL-STD', 'MIL-STD', 'operations', 'fixed', 'Military Standard (MIL-STD)', '[]', FALSE, 88, 'Engineering', 'Environmental/EMI/EMC standards.'),
('Harekât İhtiyacı', 'Operational Requirement', '', '', 'operations', 'fixed', 'Operational Requirement', '[]', FALSE, 88, 'Doctrine', 'Staff requirement.'),
('Caydırıcılık', 'Deterrence', '', '', 'operations', 'fixed', 'Deterrence', '[]', FALSE, 90, 'Strategic Doctrine', 'Defence posture.'),
('Müşterek Harekât', 'Joint Operations', '', '', 'operations', 'fixed', 'Joint Operations', '[]', FALSE, 90, 'NATO Doctrine', 'Multi-service operations.'),
('Çok Alanlı Harekât', 'Multi-Domain Operations (MDO)', '', 'MDO', 'operations', 'fixed', 'Multi-Domain Operations (MDO)', '[]', FALSE, 90, 'NATO Doctrine', 'Land, air, maritime, cyber, space.'),
('Durumsal Farkındalık', 'Situational Awareness', '', '', 'operations', 'fixed', 'Situational Awareness', '[]', FALSE, 90, 'C4ISR', 'Common operational picture.'),
('Ortak Harekât Resmi', 'Common Operational Picture (COP)', 'OHR', 'COP', 'operations', 'fixed', 'Common Operational Picture (COP)', '[]', FALSE, 90, 'C4ISR', 'NATO COP.'),
('Lojistik Destek', 'Logistics Support', '', '', 'operations', 'fixed', 'Logistics Support', '[]', FALSE, 85, 'Sustainment', 'ILS / sustainment.'),
('Ömür Devri Yönetimi', 'Life Cycle Management', '', '', 'operations', 'fixed', 'Life Cycle Management', '[]', FALSE, 85, 'Sustainment', 'NATO LCM.'),
('Modernizasyon', 'Modernisation', '', '', 'operations', 'fixed', 'Modernisation', '["Modernization"]', FALSE, 88, 'British English', 'British -isation spelling.'),
('Entegre Lojistik Destek', 'Integrated Logistics Support (ILS)', 'ELD', 'ILS', 'operations', 'fixed', 'Integrated Logistics Support (ILS)', '[]', FALSE, 88, 'Sustainment', 'ILS.') ON CONFLICT (term_tr, domain) DO NOTHING;

-- ============================================================================
-- 3. DYNAMIC INSTITUTION CATEGORIES (14 CATEGORIES INCL. savunma_medya)
-- ============================================================================
CREATE TABLE IF NOT EXISTS institution_categories (
    code TEXT PRIMARY KEY,
    legacy_value TEXT NOT NULL UNIQUE,
    label_tr TEXT NOT NULL,
    label_en TEXT NOT NULL,
    kind TEXT NOT NULL DEFAULT 'industry',
    is_public_sector BOOLEAN NOT NULL DEFAULT FALSE,
    requires_verification BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE
);

INSERT INTO institution_categories (
    code, legacy_value, label_tr, label_en, kind, is_public_sector, requires_verification, sort_order, is_active
) VALUES
    ('sektor_kuruluslari', 'Sektör Kuruluşları', 'Sektör Kuruluşları (Savunma Sanayii)', 'Defence Industry Organisations', 'industry', FALSE, TRUE, 1, TRUE),
    ('msb', 'MSB', 'MSB (Millî Savunma Bakanlığı)', 'MoND (Ministry of National Defence)', 'public_sector', TRUE, TRUE, 2, TRUE),
    ('ssb', 'SSB', 'SSB (Savunma Sanayii Başkanlığı)', 'SSB (Presidency of Defence Industries)', 'public_sector', TRUE, TRUE, 3, TRUE),
    ('kkk', 'K.K.K.lığı', 'K.K.K.lığı (Kara Kuvvetleri Komutanlığı)', 'TLF (Turkish Land Forces Command)', 'public_sector', TRUE, TRUE, 4, TRUE),
    ('dzkk', 'Dz.K.K.lığı', 'Dz.K.K.lığı (Deniz Kuvvetleri Komutanlığı)', 'TNF (Turkish Naval Forces Command)', 'public_sector', TRUE, TRUE, 5, TRUE),
    ('hvkk', 'Hv.K.K.lığı', 'Hv.K.K.lığı (Hava Kuvvetleri Komutanlığı)', 'TurAF (Turkish Air Force Command)', 'public_sector', TRUE, TRUE, 6, TRUE),
    ('jgk', 'J.G.K.lığı', 'J.G.K.lığı (Jandarma Genel Komutanlığı)', 'Gendarmerie General Command', 'public_sector', TRUE, TRUE, 7, TRUE),
    ('egm', 'EGM', 'EGM (Emniyet Genel Müdürlüğü)', 'TNP (Turkish National Police)', 'public_sector', TRUE, TRUE, 8, TRUE),
    ('sgk', 'S.G.K.lığı', 'S.G.K.lığı (Sahil Güvenlik Komutanlığı)', 'Turkish Coast Guard Command', 'public_sector', TRUE, TRUE, 9, TRUE),
    ('mit', 'MİT', 'MİT (Millî İstihbarat Teşkilatı)', 'MIT (National Intelligence Organization)', 'public_sector', TRUE, TRUE, 10, TRUE),
    ('diger_kamu', 'Diğer Kamu Kurumları', 'Diğer Kamu Kurumları', 'Other Public Institutions', 'public_sector', TRUE, TRUE, 11, TRUE),
    ('universite_ogrenci', 'Üniversite / Öğrenci', 'Üniversite / Akademik / Öğrenci', 'University / Academic / Student', 'academic', FALSE, FALSE, 12, TRUE),
    ('yurt_disi_paydas', 'Yurt Dışı Paydaş', 'Yurt Dışı Paydaş / Uluslararası', 'International Stakeholder / Allied', 'international', FALSE, FALSE, 13, TRUE),
    ('savunma_medya', 'savunma_medya', 'Savunma Medya / Basın', 'Defence Media / Press', 'media', FALSE, FALSE, 14, TRUE) ON CONFLICT (code) DO NOTHING;

-- ============================================================================
-- 4. PUBLIC SECTOR, ACADEMIC & PERSONAL EMAIL DOMAINS
-- ============================================================================
CREATE TABLE IF NOT EXISTS gov_email_domains (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    domain TEXT NOT NULL UNIQUE,
    match_mode TEXT NOT NULL DEFAULT 'suffix' CHECK (match_mode IN ('suffix', 'exact')),
    domain_kind TEXT NOT NULL DEFAULT 'public_sector' CHECK (domain_kind IN ('public_sector', 'academic', 'personal')),
    institution_code TEXT REFERENCES institution_categories(code) ON DELETE SET NULL,
    grants_role TEXT NOT NULL DEFAULT 'registered',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    note TEXT NOT NULL DEFAULT ''
);

INSERT INTO gov_email_domains (
    domain, match_mode, domain_kind, institution_code, grants_role, is_active, note
) VALUES
    ('.msb.gov.tr', 'suffix', 'public_sector', 'msb', 'subscriber_public', TRUE, 'Millî Savunma Bakanlığı'),
    ('.ssb.gov.tr', 'suffix', 'public_sector', 'ssb', 'subscriber_public', TRUE, 'Savunma Sanayii Başkanlığı'),
    ('.jandarma.gov.tr', 'suffix', 'public_sector', 'jgk', 'subscriber_public', TRUE, 'Jandarma Genel Komutanlığı'),
    ('.sahilguvenlik.gov.tr', 'suffix', 'public_sector', 'sgk', 'subscriber_public', TRUE, 'Sahil Güvenlik Komutanlığı'),
    ('.mit.gov.tr', 'suffix', 'public_sector', 'mit', 'subscriber_public', TRUE, 'Millî İstihbarat Teşkilatı'),
    ('.egm.gov.tr', 'suffix', 'public_sector', 'egm', 'subscriber_public', TRUE, 'Emniyet Genel Müdürlüğü'),
    ('.tsk.tr', 'suffix', 'public_sector', 'msb', 'subscriber_public', TRUE, 'Türk Silahlı Kuvvetleri'),
    ('.mil.tr', 'suffix', 'public_sector', 'msb', 'subscriber_public', TRUE, 'Askerî kurumlar'),
    ('.pol.tr', 'suffix', 'public_sector', 'egm', 'subscriber_public', TRUE, 'Emniyet teşkilatı'),
    ('.gov.tr', 'suffix', 'public_sector', 'diger_kamu', 'subscriber_public', TRUE, 'Türkiye Cumhuriyeti kamu kurumları'),
    ('.edu.tr', 'suffix', 'academic', 'universite_ogrenci', 'registered', TRUE, 'Akademik ve öğrenci e-postaları'),
    ('gmail.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('hotmail.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('outlook.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('yahoo.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('icloud.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('yandex.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('yandex.com.tr', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('windowslive.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('live.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('protonmail.com', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı'),
    ('proton.me', 'exact', 'personal', NULL, 'registered', TRUE, 'Kişisel e-posta sağlayıcı') ON CONFLICT (domain) DO NOTHING;

-- ============================================================================
-- 5. SEGMENTED EMAIL GROUPS FOR NEWSLETTER & TARGETED DISPATCH
-- ============================================================================
CREATE TABLE IF NOT EXISTS email_groups (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    name_tr TEXT NOT NULL,
    name_en TEXT NOT NULL,
    description_tr TEXT NOT NULL DEFAULT '',
    description_en TEXT NOT NULL DEFAULT '',
    filter_json TEXT NOT NULL DEFAULT '{}' CHECK (filter_json::jsonb IS NOT NULL),
    static_emails_json TEXT NOT NULL DEFAULT '[]' CHECK (static_emails_json::jsonb IS NOT NULL),
    requires_marketing_consent BOOLEAN NOT NULL DEFAULT TRUE,
    is_system BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_by TEXT NOT NULL DEFAULT 'system',
    updated_at TEXT NOT NULL DEFAULT to_char(NOW() AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
);

INSERT INTO email_groups (
    code, name_tr, name_en, description_tr, description_en, filter_json, static_emails_json,
    requires_marketing_consent, is_system, is_active, sort_order, created_by
) VALUES
    ('all', 'Tüm E-Bülten Aboneleri', 'All Newsletter Subscribers', 'KVKK ve ticari ileti onayı bulunan tüm aktif üyeler.', 'All active members with KVKK and marketing email opt-in.', '{"group":"all"}', '[]', TRUE, TRUE, TRUE, 1, 'system'),
    ('gov_verified', 'Doğrulanmış Kamu & TSK Personeli', 'Verified Public Sector & TAF Personnel', '.gov.tr, .tsk.tr ve onaylı kamu personeli listesi.', 'Verified .gov.tr, .tsk.tr and public-sector personnel.', '{"group":"gov_verified","is_gov_verified":true}', '[]', TRUE, TRUE, TRUE, 2, 'system'),
    ('active_subscribers', 'Aktif Ücretli & Kurumsal Aboneler', 'Active Paid & Corporate Subscribers', 'Süresi geçmemiş basılı, dijital veya kurumsal aboneler.', 'Active non-expired print, digital, corporate or public subscribers.', '{"group":"active_subscribers"}', '[]', TRUE, TRUE, TRUE, 3, 'system'),
    ('email_verified', 'E-Postası Doğrulanmış Üyeler', 'Email-Verified Members', 'E-posta doğrulamasını tamamlamış tüm kullanıcılar.', 'All users with verified email addresses.', '{"group":"email_verified","email_verified":true}', '[]', TRUE, TRUE, TRUE, 4, 'system'),
    ('defence_industry_leaders', 'Savunma Sanayii Karar Vericileri', 'Defence Industry Decision Makers', 'MSB, SSB, Kuvvet Komutanlıkları ve Savunma Sanayii Kuruluşları.', 'MoND, SSB, Force Commands and Defence Industry Organisations.', '{"group":"defence_industry_leaders","organization_categories":["MSB","SSB","K.K.K.lığı","Dz.K.K.lığı","Hv.K.K.lığı","Sektör Kuruluşları"]}', '[]', TRUE, FALSE, TRUE, 5, 'system') ON CONFLICT (code) DO NOTHING;

-- ============================================================================

-- ============================================================================
-- 6. UPGRADE magazine_issues WITH COMPOSITE PK + EN APPROVAL + 22 PDF LINKS
-- ============================================================================
ALTER TABLE magazine_issues DROP CONSTRAINT IF EXISTS magazine_issues_pkey;
ALTER TABLE magazine_issues
    ADD COLUMN IF NOT EXISTS en_status TEXT NOT NULL DEFAULT 'none',
    ADD COLUMN IF NOT EXISTS en_translated_at TEXT,
    ADD COLUMN IF NOT EXISTS en_approved_by TEXT,
    ADD COLUMN IF NOT EXISTS en_approved_at TEXT,
    ADD COLUMN IF NOT EXISTS tr_source_hash TEXT;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_magazine_issues_en_status'
    ) THEN
        ALTER TABLE magazine_issues ADD CONSTRAINT chk_magazine_issues_en_status CHECK (en_status IN ('none', 'auto_draft', 'approved'));
    END IF;
END $$;

UPDATE magazine_issues
SET publication_type = CASE
    WHEN COALESCE(NULLIF(TRIM(publication_type), ''), '') IN ('msi', 'msi_tdr') THEN TRIM(publication_type)
    WHEN issue_number < 200 OR slug LIKE 'msi-tdr%' THEN 'msi_tdr'
    ELSE 'msi'
END;

UPDATE magazine_issues
SET en_status = 'approved'
WHERE publication_type = 'msi_tdr' AND en_status = 'none';

ALTER TABLE magazine_issues ADD PRIMARY KEY (publication_type, issue_number);

CREATE INDEX IF NOT EXISTS idx_magazine_issues_number ON magazine_issues(issue_number DESC);
CREATE INDEX IF NOT EXISTS idx_magazine_issues_en_status ON magazine_issues(en_status);

UPDATE magazine_issues
SET uploaded_pdf_file = 'msi-dergisi-sayi-' || issue_number::text || '.pdf'
WHERE publication_type = 'msi'
  AND issue_number IN (263, 266, 270, 271, 273, 274, 275, 276, 277, 278, 279, 280, 281, 284, 285, 286)
  AND (uploaded_pdf_file IS NULL OR TRIM(uploaded_pdf_file) = '');

UPDATE magazine_issues
SET uploaded_pdf_file = 'msi-tdr-issue-' || issue_number::text || '.pdf'
WHERE publication_type = 'msi_tdr'
  AND issue_number IN (143, 147, 149, 150, 151, 152, 153)
  AND (uploaded_pdf_file IS NULL OR TRIM(uploaded_pdf_file) = '');

-- ============================================================================
-- 7. ADD EN APPROVAL LIFECYCLE COLUMNS TO contents & BACKFILL CLEAN ITEMS
-- ============================================================================
ALTER TABLE contents ADD COLUMN IF NOT EXISTS en_status TEXT NOT NULL DEFAULT 'none';
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_contents_en_status'
    ) THEN
        ALTER TABLE contents ADD CONSTRAINT chk_contents_en_status CHECK (en_status IN ('none', 'auto_draft', 'approved'));
    END IF;
END $$;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS en_translated_at TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS en_approved_by TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS en_approved_at TEXT;
ALTER TABLE contents ADD COLUMN IF NOT EXISTS tr_source_hash TEXT;

CREATE INDEX IF NOT EXISTS idx_contents_en_status ON contents(en_status);

UPDATE contents
SET en_status = 'approved'
WHERE source_group = 'msi-tdr'
   OR (TRIM(title_en) <> '' AND TRIM(body_en) <> '' AND title_en <> title_tr AND body_en <> body_tr);

-- ============================================================================
-- 8. EXTEND content_versions, approval_requests, ad_campaigns, commerce & corporate
-- ============================================================================
ALTER TABLE content_versions ADD COLUMN IF NOT EXISTS en_status TEXT NOT NULL DEFAULT 'none';
ALTER TABLE content_versions ADD COLUMN IF NOT EXISTS previous_en_status TEXT NOT NULL DEFAULT '';
ALTER TABLE content_versions ADD COLUMN IF NOT EXISTS source_action TEXT NOT NULL DEFAULT 'edit';
ALTER TABLE content_versions ADD COLUMN IF NOT EXISTS review_note TEXT NOT NULL DEFAULT '';

ALTER TABLE approval_requests ADD COLUMN IF NOT EXISTS previous_en_status TEXT NOT NULL DEFAULT 'none';
ALTER TABLE approval_requests ADD COLUMN IF NOT EXISTS source_action TEXT NOT NULL DEFAULT 'submit_review';
ALTER TABLE approval_requests ADD COLUMN IF NOT EXISTS target_en_status TEXT NOT NULL DEFAULT '';

ALTER TABLE ad_campaigns ADD COLUMN IF NOT EXISTS start_date TEXT;
ALTER TABLE ad_campaigns ADD COLUMN IF NOT EXISTS end_date TEXT;
ALTER TABLE ad_campaigns ADD COLUMN IF NOT EXISTS rotation_mode TEXT NOT NULL DEFAULT 'weighted';

ALTER TABLE commerce_products ADD COLUMN IF NOT EXISTS delivery_asset_url TEXT NOT NULL DEFAULT '';

ALTER TABLE corporate_verification_requests ADD COLUMN IF NOT EXISTS decision_note TEXT NOT NULL DEFAULT '';

-- ============================================================================
-- 9. FIX US ENGLISH ("Defense" -> "Defence") & "Turkey" -> "Türkiye" IN DB DEFAULTS
-- ============================================================================
UPDATE translations
SET value_en = 'MSI Magazine & MSI TDR — Defence, Aerospace and Security Technologies Reference Platform (ISSN 1306-0309)'
WHERE key = 'seo.default_description' AND value_en LIKE '%Defense%';

UPDATE translations
SET value_en = 'Latest news, analyses and magazine content from MSI Dergisi, Türkiye''s defence and aerospace industry magazine.'
WHERE key = 'plugin.description_human' AND (value_en LIKE '%Turkey''s%' OR value_en LIKE '%defense%');

UPDATE translations
SET value_tr = 'Reklam Alanı Varsayılan Başlığı (EN)',
    value_en = 'Copyrighted Aviation & Defence Photography Across Air-to-Air, Land and Naval Platforms'
WHERE key = 'ads.fallback_title_en';

UPDATE translations
SET value_en = 'Copyrighted Aviation & Defence Photography Across Air-to-Air, Land and Naval Platforms'
WHERE key = 'ads.fallback_title_tr' AND value_en LIKE '%Defense%';

UPDATE translations
SET value_tr = 'Reklam Alanı Varsayılan Açıklaması (EN)',
    value_en = 'Remove Before Photos high-resolution defence visual archive and MSI Dergisi corporate subscription solutions.'
WHERE key = 'ads.fallback_desc_en';

UPDATE translations
SET value_en = 'Remove Before Photos high-resolution defence visual archive and MSI Dergisi corporate subscription solutions.'
WHERE key = 'ads.fallback_desc_tr' AND value_en LIKE '%defense%';

UPDATE translations
SET value_tr = 'Abonelere Özel İçerik: Bu analizin tamamı sadece MSI Dergisi abonelerine açıktır. Okumaya devam etmek için abone olabilir veya bu makaleyi tekil olarak ({price} TL) satın alabilirsiniz. (.gov.tr ve .tsk.tr kurumsal e-postalarına ücretsizdir.)',
    value_en = 'Subscriber-Only Dossier: Full access requires an active MSI Magazine subscription or single-article purchase ({price} TRY). Public sector personnel (.gov.tr / .tsk.tr) receive complimentary access.'
WHERE key IN ('paywall.subscriber_only_tr', 'paywall.subscriber_only_en') AND value_tr = value_en;

UPDATE site_settings
SET value_en = 'MSI TDR — Turkish Defence Review | Defence & Aerospace Platform'
WHERE key = 'brand_title' AND value_en LIKE '%Defense%';

UPDATE site_settings
SET value_en = 'Authoritative Defence, Aerospace & Security Intelligence Since 2004.'
WHERE key = 'brand_tagline' AND value_en LIKE '%Defense%';

UPDATE site_settings
SET value_en = REPLACE(REPLACE(REPLACE(value_en, 'Defense', 'Defence'), 'defense', 'defence'), 'Turkey', 'Türkiye')
WHERE value_en LIKE '%Defense%' OR value_en LIKE '%defense%' OR value_en LIKE '%Turkey%';

UPDATE categories
SET name_en = REPLACE(REPLACE(name_en, 'Defense', 'Defence'), 'defense', 'defence'),
    description_en = REPLACE(REPLACE(REPLACE(REPLACE(description_en, 'Defense', 'Defence'), 'defense', 'defence'), 'MoD', 'MoND'), 'Turkey', 'Türkiye')
WHERE name_en LIKE '%Defense%' OR name_en LIKE '%defense%'
   OR description_en LIKE '%Defense%' OR description_en LIKE '%defense%' OR description_en LIKE '%MoD%' OR description_en LIKE '%Turkey%';

UPDATE ssb_tags
SET name_en = REPLACE(REPLACE(name_en, 'Defense', 'Defence'), 'defense', 'defence')
WHERE name_en LIKE '%Defense%' OR name_en LIKE '%defense%';

-- ============================================================================
-- 10. RBAC ROLE HIERARCHY & SUPPORT AGENT ENTITLEMENT PERMISSIONS
-- ============================================================================
UPDATE user_roles
SET permissions_json = '["*"]', is_system = TRUE
WHERE role_code = 'super_admin';

UPDATE user_roles
SET permissions_json = '["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","ads:manage","commerce:manage","subscriptions:manage","invoices:manage","users:read","users:write","users:verify","users:manage","corporate_requests:manage","newsletters:manage","email_groups:manage","notifications:manage","redirects:manage","translations:manage","glossary:manage","locales:manage","analytics:read","revenue:read","audit:read","audit:write","settings:manage","monitoring:read","roles:manage","backup:manage"]',
    is_system = TRUE
WHERE role_code = 'admin';

UPDATE user_roles
SET permissions_json = '["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","ads:manage","commerce:manage","subscriptions:manage","invoices:read","invoices:write","users:read","users:verify","corporate_requests:manage","newsletters:manage","email_groups:manage","notifications:manage","redirects:manage","translations:manage","glossary:manage","analytics:read","revenue:read","audit:read","settings:read","settings:write","monitoring:read"]',
    is_system = TRUE
WHERE role_code = 'site_operator';

UPDATE user_roles
SET permissions_json = '["users:read","users:verify","users:write","corporate_requests:read","corporate_requests:decide","subscriptions:read","invoices:read","notifications:read","audit:read"]',
    is_system = FALSE
WHERE role_code = 'support_agent';

UPDATE user_roles
SET permissions_json = '["contents:manage","categories:manage","media:manage","issues:manage","approvals:manage","newsletters:write","email_groups:manage","notifications:write","redirects:write","translations:write","glossary:manage","analytics:read","settings:read"]',
    is_system = FALSE
WHERE role_code = 'editor_in_chief';

UPDATE user_roles
SET permissions_json = '["contents:manage","categories:write","media:manage","issues:manage","approvals:manage","newsletters:write","email_groups:write","redirects:write","translations:write","glossary:write","settings:read"]',
    is_system = FALSE
WHERE role_code = 'managing_editor';
