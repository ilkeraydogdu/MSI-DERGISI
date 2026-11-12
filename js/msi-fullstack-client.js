/**
 * UMSA Prodüksiyon — MSI Dergisi & MSI TDR Fullstack Interactive Client Layer
 * File: js/msi-fullstack-client.js
 * Harmonized with Central Corporate Red (#C4122F) & White (#FFFFFF) + Soft Dark Mode
 */
(function () {
    'use strict';

    const SSB_TAGS_14 = [
        { code: 'hava-araclari', tr: 'Hava Araçları', en: 'Air Platforms' },
        { code: 'deniz-araclari', tr: 'Deniz Araçları', en: 'Naval Platforms' },
        { code: 'kara-araclari', tr: 'Kara Araçları', en: 'Land Platforms' },
        { code: 'fuzeler-roketler-ve-muhimmatlar', tr: 'Füzeler, Roketler ve Mühimmatlar', en: 'Missiles, Rockets & Ammunition' },
        { code: 'silahlar-ve-kuleler', tr: 'Silahlar ve Kuleler', en: 'Weapons & Turrets' },
        { code: 'radarlar-ve-sensorler', tr: 'Radarlar ve Sensörler', en: 'Radars & Sensors' },
        { code: 'elektronik-harp', tr: 'Elektronik Harp', en: 'Electronic Warfare' },
        { code: 'muhabere-ve-bilisim', tr: 'Muhabere ve Bilişim', en: 'Communications & IT' },
        { code: 'komuta-kontrol', tr: 'Komuta Kontrol', en: 'Command & Control' },
        { code: 'simulasyon-ve-egitim', tr: 'Simülasyon ve Eğitim', en: 'Simulation & Training' },
        { code: 'alt-sistemler', tr: 'Alt Sistemler', en: 'Subsystems' },
        { code: 'lojistik-destek-ve-bakim', tr: 'Lojistik Destek ve Bakım', en: 'Logistics & Maintenance' },
        { code: 'siber-guvenlik-ve-yapay-zeka', tr: 'Siber Güvenlik ve Yapay Zeka', en: 'Cyber Security & AI' },
        { code: 'uzay-ve-uydu-sistemleri', tr: 'Uzay ve Uydu Sistemleri', en: 'Space & Satellite Systems' }
    ];

    const INSTITUTIONS_LIST = [
        'MSB',
        'SSB',
        'K.K.K.lığı',
        'Dz.K.K.lığı',
        'Hv.K.K.lığı',
        'J.G.K.lığı',
        'EGM',
        'S.G.K.lığı',
        'MİT',
        'Diğer Kamu Kurumları',
        'Sektör Kuruluşları',
        'Üniversite / Öğrenci'
    ];

    const PUBLIC_GOV_SUFFIXES = [
        '.gov.tr',
        '.tsk.tr',
        '.mil.tr',
        '.pol.tr',
        '.jandarma.gov.tr',
        '.msb.gov.tr',
        '.ssb.gov.tr',
        '.sahilguvenlik.gov.tr',
        '.mit.gov.tr',
        '.egm.gov.tr'
    ];

    const state = {
        lang: localStorage.getItem('msi_lang') || (window.location.search.includes('lang=en') ? 'en' : 'tr'),
        selectedSsbTag: localStorage.getItem('msi_ssb_filter') || 'ALL',
        sessionId: getOrCreateSessionId(),
        user: loadStoredUser(),
        telemetry: {
            startTime: Date.now(),
            durationSec: 0,
            maxScrollPct: 12,
            videoCompleted: false,
            trueReadSent: false,
            contentSlug: detectContentSlug(),
            observedAds: new Set()
        },
        flipbook: {
            issueNumber: 286,
            currentSpreadIdx: 0,
            spreads: [
                {
                    page_no: 'Kapak & S. 01-04',
                    title_tr: 'Sayı 286 Kapak Dosyası: Entegre Hava ve Füze Savunma Mimarisi',
                    title_en: 'Issue 286 Cover Dossier: Integrated Air & Missile Defence Architecture',
                    category_badge: 'KAPAK DOSYASI',
                    image_url: '/images/msi/dergi-286.webp',
                    excerpt_tr: 'Çelik Kubbe doktrini kapsamında alçak, orta ve yüksek irtifa hava savunma katmanlarının ağ merkezli komuta kontrol entegrasyonu.',
                    excerpt_en: 'Network-centric command and control integration of low, medium, and high altitude air defence layers.'
                },
                {
                    page_no: 'Sayfa 12-18',
                    title_tr: 'KAAN Milli Muharip Uçak ve AESA Radar Entegrasyon Takvimi',
                    title_en: 'KAAN National Combat Aircraft & AESA Radar Integration Schedule',
                    category_badge: 'ÖZEL HABER',
                    image_url: '/images/msi/kaan.webp',
                    excerpt_tr: 'MURAD AESA burun radarı, dahili mühimmat yuvaları ve düşük görünürlük kaplama teknolojilerinde ulaşılan son aşama.',
                    excerpt_en: 'Latest milestone in MURAD AESA nose radar, internal weapon bays, and low-observable coating technologies.'
                },
                {
                    page_no: 'Sayfa 24-32',
                    title_tr: 'AKINCI TİHA ve İHA-300 Balistik Süpersonik Füze Atış Testi',
                    title_en: 'AKINCI UCAV & IHA-300 Supersonic Ballistic Missile Test',
                    category_badge: 'HAVA SİSTEMLERİ',
                    image_url: '/images/msi/akinci-iha300.webp',
                    excerpt_tr: 'Yüksek irtifadan bırakılan süpersonik füze mühimmatlarıyla stratejik derinlikteki hedeflerin tam isabetle imhası.',
                    excerpt_en: 'Precision strike capabilities at strategic depth with air-launched supersonic munitions.'
                },
                {
                    page_no: 'Sayfa 40-48',
                    title_tr: 'MİLGEM İ-Sınıfı Firkateynler ve Milli Dikey Atım Sistemi (MİDLAS)',
                    title_en: 'MILGEM I-Class Frigates & National Vertical Launching System (MIDLAS)',
                    category_badge: 'DENİZ SİSTEMLERİ',
                    image_url: '/images/msi/saha-deniz.webp',
                    excerpt_tr: 'HİSAR-D RF ve SİPER Blok-1 füzelerinin MİDLAS üzerinden deniz platformlarına entegrasyonu ve açık deniz görev kabiliyeti.',
                    excerpt_en: 'Integration of HISAR-D RF and SIPER Block-1 missiles into naval platforms via MIDLAS.'
                }
            ]
        }
    };

    function getOrCreateSessionId() {
        let sid = sessionStorage.getItem('msi_session_id');
        if (!sid) {
            sid = 'msi-sess-' + Math.random().toString(36).substring(2, 11) + '-' + Date.now();
            sessionStorage.setItem('msi_session_id', sid);
        }
        return sid;
    }

    function loadStoredUser() {
        try {
            const raw = localStorage.getItem('msi_active_user');
            if (raw) return JSON.parse(raw);
        } catch (_) {}
        return null;
    }

    function detectContentSlug() {
        const path = window.location.pathname;
        if (path.startsWith('/icerik/')) {
            return path.replace('/icerik/', '').replace(/\/$/, '');
        }
        const metaSlug = document.querySelector('[data-msi-content-slug], [data-content-slug]');
        if (metaSlug) return metaSlug.getAttribute('data-msi-content-slug') || metaSlug.getAttribute('data-content-slug');
        return 'anasayfa-vitrin';
    }

    function isGovEmail(email) {
        if (!email || !email.includes('@')) return false;
        const domain = email.trim().toLowerCase().split('@')[1] || '';
        return PUBLIC_GOV_SUFFIXES.some(suffix => {
            const bare = suffix.replace(/^\./, '');
            return domain === bare || domain.endsWith(suffix);
        });
    }

    function setLanguage(lang) {
        state.lang = lang === 'en' ? 'en' : 'tr';
        localStorage.setItem('msi_lang', state.lang);

        document.querySelectorAll('[data-tr][data-en]').forEach(el => {
            const val = state.lang === 'en' ? el.getAttribute('data-en') : el.getAttribute('data-tr');
            if (val) el.textContent = val;
        });
    }

    function applySsbPersonalizationFilter(ssbCode) {
        state.selectedSsbTag = ssbCode;
        localStorage.setItem('msi_ssb_filter', ssbCode);

        const cards = document.querySelectorAll('[data-ssb-tags]');
        if (cards.length > 0) {
            cards.forEach(card => {
                if (ssbCode === 'ALL') {
                    card.style.display = '';
                    card.style.outline = '';
                } else {
                    const tags = (card.getAttribute('data-ssb-tags') || '').split(',').map(s => s.trim());
                    if (tags.includes(ssbCode)) {
                        card.style.display = '';
                        card.style.outline = '2px solid #C4122F';
                    } else {
                        card.style.display = 'none';
                        card.style.outline = '';
                    }
                }
            });
        }

        if (ssbCode !== 'ALL') {
            sendTelemetryBeacon('ssb_filter_engage', { ssb_tag: ssbCode });
        }
    }

    // -------------------------------------------------------------------------
    // 1. AUTH MODAL — REMOVED (duplicate of msi-ui.js canonical implementation)
    // The canonical login/registration dialog lives in js/msi-ui.js (lines ~689-806)
    // with proper KVKK consent gating and sendBeacon analytics. This duplicate
    // caused conflicting auth UIs and double registration submissions.
    // -------------------------------------------------------------------------
    // Stubs kept so that any legacy inline onclick="window.MSIClient.openLoginModal()"
    // calls do not throw; they delegate to msi-ui.js when available.
    function openLoginModal() {
        if (typeof window.MSIUI_openAuth === 'function') { window.MSIUI_openAuth(); return; }
        // Fallback: trigger the msi-ui.js click handler programmatically
        const trigger = document.querySelector('[href="#modal-login"], [data-open-dialog="auth-dialog"]');
        if (trigger) trigger.click();
    }
    function closeLoginModal() {
        const modal = document.getElementById('auth-dialog') || document.getElementById('modal-login');
        if (modal && typeof modal.close === 'function') modal.close();
        else if (modal) { modal.classList.remove('open'); modal.style.display = 'none'; }
    }
    function submitRegistration() { /* no-op — handled by msi-ui.js */ return false; }

    // -------------------------------------------------------------------------
    // 2. CORPORATE E-DERGİ FLIPBOOK & WATERMARK PDF MODAL (#modal-magazine)
    // -------------------------------------------------------------------------
    function ensureUpgradedMagazineModal() {
        let modal = document.getElementById('modal-magazine');
        if (!modal) {
            modal = document.createElement('div');
            modal.id = 'modal-magazine';
            document.body.appendChild(modal);
        }

        if (modal.getAttribute('data-msi-upgraded') === '1') return;
        modal.setAttribute('data-msi-upgraded', '1');
        modal.className = 'msi-fs-modal-backdrop';
        modal.setAttribute('aria-hidden', 'true');

        const userNameDefault = (state.user && state.user.full_name) || 'Alb. Murat Yilmaz';
        const orgNameDefault = (state.user && state.user.organization_name) || 'MSB / SSB Kurumsal Paydas';

        modal.innerHTML = `
            <div class="msi-fs-modal-dialog modal-xl" role="dialog" aria-modal="true" aria-labelledby="msi-mag-modal-title">
                <button type="button" class="msi-fs-modal-close" data-close-modal="modal-magazine" aria-label="Kapat">
                    <i class="fas fa-times"></i>
                </button>

                <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:12px; border-bottom:1px solid var(--msi-border, #E2E8F0); padding-bottom:12px;">
                    <div>
                        <span class="msi-badge msi-badge-red mb-1">
                            <i class="fas fa-book-open me-1"></i> İNTERAKTİF E-DERGİ OKUYUCU &amp; DİNAMİK DRM FİLİGRAN MOTORU
                        </span>
                        <h3 id="msi-mag-modal-title" style="color:var(--msi-text-heading, #0F172A); font-size:20px; font-weight:800; margin:3px 0 0;">
                            MSI Dergisi Sayı #286 — İnteraktif Sayfa Çevirme (Flipbook) &amp; Kişiye Özel PDF İndirme
                        </h3>
                    </div>
                    <div style="display:flex; gap:8px; align-items:center;">
                        <button type="button" id="btn-flip-prev" class="btn-msi-outline px-3 py-1" style="font-size:12px;">
                            <i class="fas fa-chevron-left me-1"></i> Önceki Sayfa
                        </button>
                        <span id="flipbook-page-indicator" style="display:inline-flex; align-items:center; padding:0 10px; font-family:monospace; font-size:12.5px; color:var(--msi-red, #C4122F); font-weight:700;">
                            1 / 4
                        </span>
                        <button type="button" id="btn-flip-next" class="btn-msi-outline px-3 py-1" style="font-size:12px;">
                            Sonraki Sayfa <i class="fas fa-chevron-right ms-1"></i>
                        </button>
                    </div>
                </div>

                <div class="msi-flipbook-stage">
                    <div id="msi-flipbook-spread-container" class="msi-flipbook-spread-card"></div>
                    <div id="msi-flipbook-watermark-preview" class="msi-watermark-stamp-overlay"></div>
                </div>

                <div class="msi-pdf-download-bar">
                    <div style="flex:1; min-width:240px;">
                        <div style="font-size:13px; font-weight:800; color:var(--msi-red, #C4122F); display:flex; align-items:center; gap:6px;">
                            <i class="fas fa-file-pdf"></i> Dinamik Filigranlı (Watermark) PDF İndir
                        </div>
                        <div style="font-size:11.5px; color:var(--msi-text-muted, #64748B); margin-top:2px;">
                            İndirilen PDF'in her sayfasına kullanıcı adı, kurum, tarih/saat, IP ve kriptografik DRM kodu basılır.
                        </div>
                    </div>

                    <div style="display:flex; align-items:center; flex-wrap:wrap; gap:8px;">
                        <input type="text" id="pdf-wm-user" class="msi-field-input" style="width:165px; padding:7px 10px; font-size:12px;" value="${userNameDefault}" placeholder="Ad Soyad">
                        <input type="text" id="pdf-wm-org" class="msi-field-input" style="width:165px; padding:7px 10px; font-size:12px;" value="${orgNameDefault}" placeholder="Kurum Adı">
                        <select id="pdf-wm-pages" class="msi-field-select" style="width:210px; padding:7px 10px; font-size:12px;">
                            <option value="all">Tüm Sayı (Tam Arşiv Paketi)</option>
                            <option value="1-12">Sayfa 1-12 (Kapak Dosyası)</option>
                            <option value="12-18">Sayfa 12-18 (KAAN &amp; AESA Radar)</option>
                            <option value="24-32">Sayfa 24-32 (AKINCI &amp; İHA-300)</option>
                            <option value="40-48">Sayfa 40-48 (Deniz Sistemleri &amp; MİDLAS)</option>
                        </select>
                        <button type="button" id="btn-trigger-watermarked-pdf" class="btn-msi-primary px-3 py-2" style="font-size:12.5px;">
                            <i class="fas fa-file-pdf me-1"></i> Dinamik Filigranlı PDF İndir
                        </button>
                    </div>
                </div>
            </div>
        `;

        document.getElementById('btn-flip-prev')?.addEventListener('click', () => stepFlipbook(-1));
        document.getElementById('btn-flip-next')?.addEventListener('click', () => stepFlipbook(1));
        document.getElementById('btn-trigger-watermarked-pdf')?.addEventListener('click', () => downloadWatermarkedPdf());
        document.getElementById('pdf-wm-user')?.addEventListener('input', renderFlipbookSpread);
        document.getElementById('pdf-wm-org')?.addEventListener('input', renderFlipbookSpread);

        modal.querySelectorAll('[data-close-modal="modal-magazine"]').forEach(btn => {
            btn.addEventListener('click', () => closeMagazineModal());
        });
        modal.addEventListener('click', (e) => {
            if (e.target === modal) closeMagazineModal();
        });

        renderFlipbookSpread();
    }

    function renderFlipbookSpread() {
        const container = document.getElementById('msi-flipbook-spread-container');
        const indicator = document.getElementById('flipbook-page-indicator');
        const wmPreview = document.getElementById('msi-flipbook-watermark-preview');
        if (!container) return;

        const spreads = state.flipbook.spreads;
        const idx = Math.max(0, Math.min(spreads.length - 1, state.flipbook.currentSpreadIdx));
        const sp = spreads[idx];

        if (indicator) {
            indicator.textContent = `${idx + 1} / ${spreads.length} (${sp.page_no})`;
        }

        const title = state.lang === 'en' ? (sp.title_en || sp.title_tr) : sp.title_tr;
        const excerpt = state.lang === 'en' ? (sp.excerpt_en || sp.excerpt_tr) : sp.excerpt_tr;

        container.innerHTML = `
            <div style="text-align:center;">
                <img src="${sp.image_url}" alt="${title}" style="width:100%; max-height:240px; object-fit:cover; border-radius:8px; border:1px solid var(--msi-border, #E2E8F0);">
            </div>
            <div>
                <div style="display:flex; align-items:center; gap:8px; margin-bottom:8px;">
                    <span class="msi-badge msi-badge-red">${sp.category_badge}</span>
                    <span class="msi-badge msi-badge-slate">${sp.page_no}</span>
                </div>
                <h4 style="color:var(--msi-text-heading, #0F172A); font-size:18px; font-weight:800; line-height:1.35; margin-bottom:10px;">${title}</h4>
                <p style="color:var(--msi-text-body, #334155); font-size:13.5px; line-height:1.65; margin-bottom:12px;">${excerpt}</p>
                <div style="font-size:11.5px; color:var(--msi-text-muted, #64748B);">
                    <i class="fas fa-shield-alt text-danger me-1"></i>
                    Kurumsal E-Dergi Kiosk • Tam Çözünürlüklü Vektörel PDF ve Sayfa Bazlı Arşiv İndirme Hazır
                </div>
            </div>
        `;

        if (wmPreview) {
            const u = document.getElementById('pdf-wm-user')?.value || 'Kayitli Okur';
            const o = document.getElementById('pdf-wm-org')?.value || 'MSB / SSB Kurumsal';
            const ts = new Date().toISOString().slice(0, 16).replace('T', ' ') + ' UTC';
            wmPreview.innerHTML = `<i class="fas fa-fingerprint me-1"></i> <strong>CANLI FİLİGRAN ÖNİZLEMESİ:</strong> MSB/SSB/KURUMSAL LISANS: ${u} (${o}) - TARIH: ${ts} - DRM KODU: MSI-DRM-2026-VERIFIED`;
        }
    }

    function stepFlipbook(delta) {
        const len = state.flipbook.spreads.length;
        state.flipbook.currentSpreadIdx = (state.flipbook.currentSpreadIdx + delta + len) % len;
        renderFlipbookSpread();
    }

    function downloadWatermarkedPdf() {
        const pages = document.getElementById('pdf-wm-pages')?.value || 'all';
        const user = document.getElementById('pdf-wm-user')?.value || 'Kayitli Okur';
        const org = document.getElementById('pdf-wm-org')?.value || 'Kurumsal Paydas';
        const issueNo = state.flipbook.issueNumber || 286;

        const url = `/api/v1/edergi/${issueNo}/download-pdf?pages=${encodeURIComponent(pages)}&user=${encodeURIComponent(user)}&org=${encodeURIComponent(org)}`;
        sendTelemetryBeacon('edergi_pdf_download', { content_slug: `edergi-${issueNo}-${pages}` });
        window.open(url, '_blank');
    }

    function openMagazineModal() {
        ensureUpgradedMagazineModal();
        const modal = document.getElementById('modal-magazine');
        if (modal) {
            modal.classList.add('open');
            modal.style.display = 'flex';
            renderFlipbookSpread();
        }
    }

    function closeMagazineModal() {
        const modal = document.getElementById('modal-magazine');
        if (modal) {
            modal.classList.remove('open');
            modal.style.display = 'none';
        }
    }

    // -------------------------------------------------------------------------
    // 3. TELEMETRY — REMOVED (duplicate of msi-ui.js canonical implementation)
    //    The canonical reading-depth tracker, ad impression observer, and
    //    sendBeacon-based analytics live in js/msi-ui.js (lines ~944-1005+)
    //    with proper KVKK consent gating. This duplicate caused double beacons.
    // -------------------------------------------------------------------------
    // sendTelemetryBeacon kept as a thin proxy so that unique functionality
    // below (e.g. watermark PDF download) can still emit events without
    // importing msi-ui internals directly.
    function sendTelemetryBeacon(eventType, extra) {
        // Delegate to msi-ui.js track() if available; otherwise no-op.
        if (typeof window.MSIUI_track === 'function') {
            window.MSIUI_track(eventType, extra);
            return;
        }
        // Fallback: silent no-op when msi-ui.js is not loaded.
    }

    function initCinematicEffectsAndTriggers() {
        const cards = document.querySelectorAll('.msi-modern-card, .msi-mag-dossier-card, .msi-mag-subcard, .msi-archive-card, .msi-ad-box');
        cards.forEach(card => {
            card.classList.add('msi-cursor-glow-card');
        });

        document.addEventListener('click', (e) => {
            const loginTrigger = e.target.closest('[data-bs-target="#modal-login"], [href="#modal-login"], .btn-open-login-modal');
            if (loginTrigger) {
                e.preventDefault();
                openLoginModal();
                return;
            }

            const magTrigger = e.target.closest('[data-bs-target="#modal-magazine"], [href="#modal-magazine"], .btn-open-edergi-modal, .btn-nav-edergi');
            if (magTrigger) {
                e.preventDefault();
                openMagazineModal();
            }
        });
    }

    function init() {
        // initReadingDepthTelemetry() removed — canonical implementation in msi-ui.js
        initCinematicEffectsAndTriggers();
        setLanguage(state.lang);
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }

    window.MSIClient = {
        setLanguage,
        applySsbPersonalizationFilter,
        // Auth stubs delegate to msi-ui.js canonical implementation
        openLoginModal,
        closeLoginModal,
        submitRegistration,
        // UNIQUE: E-Dergi flipbook & watermarked PDF download (not in msi-ui.js)
        openMagazineModal,
        closeMagazineModal,
        downloadWatermarkedPdf,
        // Telemetry proxy delegates to msi-ui.js when available
        sendTelemetryBeacon
    };
})();
