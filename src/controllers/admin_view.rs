//! OMS/CMS Operator Dashboard View (`src/controllers/admin_view.rs`)
//! Renders the complete, self-contained HTML5 Operator & B2B Defense Analytics Center (`/admin`)
//! for UMSA Prodüksiyon — MSI Dergisi & MSI TDR.

pub fn render_admin_dashboard() -> String {
    r#####"<!DOCTYPE html>
<html lang="tr">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>UMSA Prodüksiyon — MSI Dergisi &amp; MSI TDR | OMS/CMS Operasyon ve Analitik Merkezi</title>
    <link rel="shortcut icon" type="image/x-icon" href="/images/favicon.png">
    <link rel="stylesheet" href="/css/icons.css" type="text/css">
    <link rel="stylesheet" href="/fonts/line-icons.css" type="text/css">
    <link rel="stylesheet" href="/css/msi-admin.css?v=1">
</head>
<body>
    <div class="ambient-orb orb-1"></div>
    <div class="ambient-orb orb-2"></div>
    <div class="ambient-orb orb-3"></div>
    <div class="noise-overlay"></div>

    <div class="app-shell">
        <!-- COMMAND HEADER -->
        <header class="command-header">
            <div class="brand-cluster">
                <div class="brand-emblem">MSI</div>
                <div class="brand-titles">
                    <h1>
                        UMSA PRODÜKSİYON — OMS/CMS OPERASYON &amp; ANALİTİK MERKEZİ
                        <span class="live-pulse-badge"><span class="live-pulse-dot"></span> SINGLE SOURCE OF TRUTH</span>
                    </h1>
                    <p>MSI Dergisi (TR) &amp; MSI Turkish Defence Review (EN) • Teknik Bilgi Gerektirmeyen 2 Dakikalık Operatör İş Akışı</p>
                </div>
            </div>
            <div class="header-actions">
                <a href="/" class="btn-cmd"><i class="fa-solid fa-globe"></i> MSI Dergisi (TR)</a>
                <a href="/?lang=en" class="btn-cmd"><i class="fa-solid fa-earth-americas"></i> MSI TDR (EN)</a>
                <a href="/e-dergi" class="btn-cmd"><i class="fa-solid fa-book-open"></i> E-Dergi Kiosk (#286)</a>
                <a href="/api/v1/edergi/286/download-pdf?pages=1-12" target="_blank" class="btn-cmd"><i class="fa-solid fa-file-pdf"></i> Filigranlı PDF Test (#286)</a>
                <a href="/abonelik-ve-satis" class="btn-cmd"><i class="fa-solid fa-crown"></i> Abonelik &amp; Mağaza</a>
                <button type="button" class="btn-cmd" onclick="window.MSIAdmin.toggleAdminTheme()">
                    <i class="fa-solid fa-circle-half-stroke"></i> Açık / Soft Koyu Tema
                </button>
                <button type="button" class="btn-cmd btn-cmd-emerald" onclick="window.MSIAdmin.refreshAllData()">
                    <i class="fa-solid fa-rotate"></i> Canlı Verileri Senkronize Et
                </button>
            </div>
        </header>

        <!-- EXECUTIVE KPI RIBBON -->
        <section class="kpi-grid">
            <div class="kpi-card">
                <div class="kpi-label"><span>TEKİL İÇERİK &amp; ÇOKLU GÖSTERİM</span> <i class="fa-solid fa-layer-group"></i></div>
                <div class="kpi-value" id="kpi-total-contents">—</div>
                <div class="kpi-sub" id="kpi-placements-sub">Single Source of Truth kayıt havuzu</div>
            </div>
            <div class="kpi-card cyan">
                <div class="kpi-label"><span>GERÇEK OKUMA (TRUE READ) &amp; GÖRÜNTÜLENME</span> <i class="fa-solid fa-eye"></i></div>
                <div class="kpi-value" id="kpi-true-reads">—</div>
                <div class="kpi-sub" id="kpi-views-sub">Süre + Scroll derinliği doğrulanmış okuma</div>
            </div>
            <div class="kpi-card gold">
                <div class="kpi-label"><span>AKTİF REKLAM &amp; RAKİP KORUMA</span> <i class="fa-solid fa-shield-halved"></i></div>
                <div class="kpi-value" id="kpi-active-ads">—</div>
                <div class="kpi-sub" id="kpi-ads-sub">Sponsorlu içerik rakip OEM blokajı aktif</div>
            </div>
            <div class="kpi-card emerald">
                <div class="kpi-label"><span>KAMU (.GOV.TR / .TSK.TR) &amp; KURUMSAL ONAY</span> <i class="fa-solid fa-building-shield"></i></div>
                <div class="kpi-value" id="kpi-gov-requests">—</div>
                <div class="kpi-sub" id="kpi-gov-sub">Onay bekleyen kurumsal abonelik talepleri</div>
            </div>
        </section>

        <!-- 7 OPERATIONAL TABS -->
        <nav class="tabs-nav" role="tablist" aria-label="Yönetim bölümleri">
            <a class="tab-btn" href="/admin/tasarim">Tasarım stüdyosu ↗</a>
            <button type="button" class="tab-btn active" data-tab="tab-1" onclick="window.MSIAdmin.switchTab('tab-1')">
                <span class="tab-num">01</span> <i class="fa-solid fa-bolt"></i> Hızlı İçerik &amp; Çoklu Gösterim
            </button>
            <button type="button" class="tab-btn" data-tab="tab-2" onclick="window.MSIAdmin.switchTab('tab-2')">
                <span class="tab-num">02</span> <i class="fa-solid fa-bullhorn"></i> Akıllı Reklam &amp; Çakışma Kontrolü
            </button>
            <button type="button" class="tab-btn" data-tab="tab-3" onclick="window.MSIAdmin.switchTab('tab-3')">
                <span class="tab-num">03</span> <i class="fa-solid fa-chart-pie"></i> 3 Boyutlu Kurumsal Analitik
            </button>
            <button type="button" class="tab-btn" data-tab="tab-4" onclick="window.MSIAdmin.switchTab('tab-4')">
                <span class="tab-num">04</span> <i class="fa-solid fa-user-check"></i> Kamu &amp; Kurumsal Onay Havuzu
            </button>
            <button type="button" class="tab-btn" data-tab="tab-5" onclick="window.MSIAdmin.switchTab('tab-5')">
                <span class="tab-num">05</span> <i class="fa-solid fa-sitemap"></i> Esnek Site Ağacı &amp; SSB Taksonomi
            </button>
            <button type="button" class="tab-btn" data-tab="tab-6" onclick="window.MSIAdmin.switchTab('tab-6')">
                <span class="tab-num">06</span> <i class="fa-solid fa-envelope-open-text"></i> Dinamik Bülten &amp; E-Posta Grupları
            </button>
            <button type="button" class="tab-btn" data-tab="tab-7" onclick="window.MSIAdmin.switchTab('tab-7')">
                <span class="tab-num">07</span> <i class="fa-solid fa-database"></i> 5 Kademeli Otomasyon, 301 &amp; Açık Veri
            </button>
            <button type="button" class="tab-btn" data-tab="tab-8" onclick="window.MSIAdmin.switchTab('tab-8')">
                <span class="tab-num">08</span> <i class="fa-solid fa-microchip"></i> Merkezi Motorlar (Tasarım, Bildirim &amp; WAF)
            </button>
            <button type="button" class="tab-btn" data-tab="tab-9" onclick="window.MSIAdmin.switchTab('tab-9')">
                <span class="tab-num">09</span> <i class="fa-solid fa-chart-line"></i> Analitik Grafikler &amp; Sistem Sağlığı
            </button>
            <button type="button" class="tab-btn" data-tab="tab-10" onclick="window.MSIAdmin.switchTab('tab-10')">
                <span class="tab-num">10</span> <i class="fa-solid fa-images"></i> Medya Kütüphanesi &amp; SEO Önizleme
            </button>
            <button type="button" class="tab-btn" data-tab="tab-11" onclick="window.MSIAdmin.switchTab('tab-11')">
                <span class="tab-num">11</span> <i class="fa-solid fa-users-gear"></i> Roller, İzinler &amp; Denetim Logu
            </button>
            <button type="button" class="tab-btn" data-tab="tab-12" onclick="window.MSIAdmin.switchTab('tab-12')">
                <span class="tab-num">12</span> <i class="fa-solid fa-layer-group"></i> Toplu İşlemler &amp; Versiyon Geçmişi
            </button>
            <button type="button" class="tab-btn" data-tab="tab-13" onclick="window.MSIAdmin.switchTab('tab-13')">
                <span class="tab-num">13</span> <i class="fa-solid fa-puzzle-piece"></i> Dinamik Form Alanları &amp; Önbellek
            </button>
        </nav>

        <!-- ===================================================================
             TAB 1: HIZLI İÇERİK & ÇOKLU GÖSTERİM (SINGLE SOURCE OF TRUTH) MERKEZİ
             =================================================================== -->
        <section id="tab-1" class="tab-pane active">
            <div class="workspace-grid">
                <!-- Left: 2-Minute Content Creation / Edit Form -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-pen-to-square"></i> <span id="content-form-heading">Hızlı İçerik Girişi (Single Source of Truth)</span></div>
                            <div class="panel-subtitle">Tek seferde girin; seçilen tüm bölümlerde ve SSB ilgi alanlarında otomatik yayınlansın.</div>
                        </div>
                        <button type="button" class="btn-cmd" onclick="window.MSIAdmin.resetContentForm()">
                            <i class="fa-solid fa-eraser"></i> Yeni / Temizle
                        </button>
                    </div>

                    <form id="form-content-ssot" onsubmit="return window.MSIAdmin.submitContentForm(event)">
                        <input type="hidden" id="c-id" value="0">

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="c-title-tr">Başlık (TR — MSI Dergisi) *</label>
                                <input type="text" id="c-title-tr" class="form-control" required placeholder="Örn: KAAN Milli Muharip Uçak Yeni Faz Testleri">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-title-en">Title (EN — MSI TDR)</label>
                                <input type="text" id="c-title-en" class="form-control" placeholder="e.g. KAAN National Combat Aircraft New Phase Trials">
                            </div>
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="c-content-type">Ana İçerik Tipi (`content_type`) *</label>
                                <select id="c-content-type" class="form-select">
                                    <option value="haberler">Haberler (haberler)</option>
                                    <option value="ozel-haberler">Özel Haberler (ozel-haberler)</option>
                                    <option value="soylesiler">Söyleşiler (soylesiler)</option>
                                    <option value="mesajlar">Mesajlar (mesajlar)</option>
                                    <option value="tanitici-makaleler">Tanıtıcı Makaleler (tanitici-makaleler)</option>
                                    <option value="makaleler-analizler">Makaleler &amp; Analizler (makaleler-analizler)</option>
                                    <option value="advertorial">Advertorial (advertorial)</option>
                                    <option value="basyazi">Başyazı (basyazi)</option>
                                    <option value="video">Video &amp; Multimedya (video)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-source-group">Kaynak Grubu (`source_group`) *</label>
                                <select id="c-source-group" class="form-select">
                                    <option value="gunluk-haber">Günlük Haber (gunluk-haber)</option>
                                    <option value="dergi-icerigi">Dergi İçeriği (dergi-icerigi)</option>
                                    <option value="sadece-online">Sadece Online (sadece-online)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-issue-number">Dergi Sayı No (`issue_number`)</label>
                                <input type="number" id="c-issue-number" class="form-control" value="280" placeholder="280">
                            </div>
                        </div>

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="c-summary-tr">Özet / Spot Metni (TR) *</label>
                                <textarea id="c-summary-tr" class="form-textarea" required placeholder="Makale veya haberin yönetici özeti..."></textarea>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-summary-en">Executive Summary (EN)</label>
                                <textarea id="c-summary-en" class="form-textarea" placeholder="English summary for MSI TDR readers..."></textarea>
                            </div>
                        </div>

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="c-body-tr">Zengin İçerik Gövdesi (TR — HTML/Metin) *</label>
                                <textarea id="c-body-tr" class="form-textarea" required placeholder="<p>Detaylı teknik analiz ve haber metni...</p>"></textarea>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-body-en">Rich Article Body (EN)</label>
                                <textarea id="c-body-en" class="form-textarea" placeholder="<p>Detailed defence analysis in English...</p>"></textarea>
                            </div>
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="c-visibility-level">Görünürlük / Paywall (`visibility_level`)</label>
                                <select id="c-visibility-level" class="form-select">
                                    <option value="0">0: Herkes (Açık Erişim)</option>
                                    <option value="1">1: Kayıtlı &amp; Aboneler</option>
                                    <option value="2">2: Sadece Aboneler / Tekil Satış</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-single-price">Tekil Makale Satış Fiyatı (₺)</label>
                                <input type="number" step="0.5" id="c-single-price" class="form-control" value="0" placeholder="125.0">
                            </div>
                            <div class="form-group">
                                <label class="form-label">Okuma Süresi (Dk) &amp; Yazar</label>
                                <div style="display:flex; gap:6px;">
                                    <input type="number" id="c-read-time" class="form-control" style="width:70px;" value="5" title="Okuma Süresi (Dk)">
                                    <input type="text" id="c-author-name" class="form-control" value="MSI Editöryal Masası" placeholder="Yazar Adı">
                                </div>
                            </div>
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="c-cover-image">Kapak Görseli (`cover_image`)</label>
                                <div style="display:flex; gap:6px;">
                                    <input type="text" id="c-cover-image" class="form-control" value="images/trending/trending1.jpg">
                                    <input type="file" id="c-cover-file" accept=".webp,.png,.jpg,.jpeg,.gif,.pdf,.mp4,.webm" style="display:none;" onchange="window.MSIAdmin.uploadCoverAsset(this)">
                                    <button type="button" class="btn-outline-sm" onclick="document.getElementById('c-cover-file').click()" title="Dosya Yükle"><i class="fa-solid fa-upload"></i></button>
                                </div>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-video-url">Video URL (Opsiyonel)</label>
                                <input type="text" id="c-video-url" class="form-control" placeholder="https://player.vimeo.com/...">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="c-slug">Özel URL Slug (Opsiyonel)</label>
                                <input type="text" id="c-slug" class="form-control" placeholder="Otomatik oluşturulur">
                            </div>
                        </div>

                        <!-- Sponsored Content & Legal Label Box -->
                        <div class="form-group" style="background:rgba(203,56,42,0.09); border:1px solid rgba(203,56,42,0.3); border-radius:8px; padding:12px;">
                            <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:10px; margin-bottom:8px;">
                                <label class="check-pill" style="background:transparent; padding:0; font-weight:700; color:#fca5a5;">
                                    <input type="checkbox" id="c-is-sponsored" onchange="window.MSIAdmin.toggleSponsoredFields()">
                                    <span><i class="fa-solid fa-certificate"></i> Sponsorlu İçerik / Advertorial (Rakip Reklam Korumasını Aktifleştirir)</span>
                                </label>
                                <label class="check-pill" style="background:transparent; padding:0; font-weight:700; color:#fbbf24;">
                                    <input type="checkbox" id="c-is-featured" checked>
                                    <span><i class="fa-solid fa-star"></i> Manşet / Öne Çıkan</span>
                                </label>
                            </div>
                            <div class="form-row-2" id="c-sponsor-extra-row" style="display:none;">
                                <div>
                                    <label class="form-label" for="c-sponsor-brand">Sponsor Marka (`sponsor_brand`)</label>
                                    <input type="text" id="c-sponsor-brand" class="form-control" placeholder="Örn: ASELSAN, TUSAŞ, ROKETSAN, HAVELSAN">
                                </div>
                                <div>
                                    <label class="form-label" for="c-sponsor-legal-label">Zorunlu Yasal Etiket (`sponsor_legal_label`)</label>
                                    <select id="c-sponsor-legal-label" class="form-select">
                                        <option value="SPONSORLU İÇERİK">SPONSORLU İÇERİK</option>
                                        <option value="İLANDIR">İLANDIR</option>
                                        <option value="ADVERTORIAL">ADVERTORIAL</option>
                                    </select>
                                </div>
                            </div>
                        </div>

                        <!-- Multi-Placement Checkboxes (Single Source of Truth) -->
                        <div class="form-group">
                            <label class="form-label"><i class="fa-solid fa-diagram-project" style="color:var(--msi-purple);"></i> Çoklu Gösterim Alanları (`placements` — Tek Kayıtla Birden Fazla Bölümde Gösterim)</label>
                            <div class="checkbox-matrix" id="c-placements-matrix">
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="mesajlar"> <span>Mesajlar</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="ozel-dosyalar"> <span>Özel Dosyalar</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="kapak-dosyalari" checked> <span>Kapak Dosyaları</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="tskdan-haberler"> <span>TSK'dan Haberler</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="dergi-icerikleri" checked> <span>Dergi İçerikleri</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="etkinlik-ozel-sayilari"> <span>Etkinlik Özel Sayıları</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_placement" value="etkinlik-sonrasi-ozel-dosyalar"> <span>Etkinlik Sonrası Özel Dosyalar</span></label>
                            </div>
                        </div>

                        <!-- 14 SSB Catalog Interest Checkboxes -->
                        <div class="form-group">
                            <label class="form-label"><i class="fa-solid fa-crosshairs" style="color:var(--msi-emerald);"></i> 14 SSB Katalog İlgi Alanı Etiketleri (`ssb_tags` — Kişiselleştirme &amp; Hedefleme)</label>
                            <div class="checkbox-matrix" id="c-ssb-matrix">
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="hava-araclari" checked> <span>Hava Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="deniz-araclari"> <span>Deniz Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="kara-araclari"> <span>Kara Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="fuzeler-roketler-ve-muhimmatlar"> <span>Füzeler, Roketler ve Mühimmatlar</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="silahlar-ve-kuleler"> <span>Silahlar ve Kuleler</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="radarlar-ve-sensorler" checked> <span>Radarlar ve Sensörler</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="elektronik-harp"> <span>Elektronik Harp</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="muhabere-ve-bilisim"> <span>Muhabere ve Bilişim</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="komuta-kontrol"> <span>Komuta Kontrol</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="simulasyon-ve-egitim"> <span>Simülasyon ve Eğitim</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="alt-sistemler"> <span>Alt Sistemler</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="lojistik-destek-ve-bakim"> <span>Lojistik Destek ve Bakım</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="siber-guvenlik-ve-yapay-zeka"> <span>Siber Güvenlik ve Yapay Zeka</span></label>
                                <label class="check-pill"><input type="checkbox" name="c_ssb" value="uzay-ve-uydu-sistemleri"> <span>Uzay ve Uydu Sistemleri</span></label>
                            </div>
                        </div>

                        <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center; padding:12px; font-size:13.5px;">
                            <i class="fa-solid fa-cloud-arrow-up"></i> İçeriği Kaydet ve Seçili Tüm Bölümlerde Yayınla
                        </button>
                    </form>
                </div>

                <!-- Right: Live Table of Single-Source-of-Truth Contents -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-table-list"></i> Canlı İçerik &amp; Çoklu Gösterim Envanteri</div>
                            <div class="panel-subtitle">Tüm içeriklerin tekil kayıtları, çoklu yerleşimleri, paywall seviyeleri ve gerçek okuma sayaçları.</div>
                        </div>
                        <div style="display:flex; gap:8px;">
                            <input type="text" id="filter-content-search" class="form-control" style="width:220px;" placeholder="Başlık, etiket veya sayı ara..." oninput="window.MSIAdmin.renderContentsTable()">
                        </div>
                    </div>

                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>ID / Sayı</th>
                                    <th>Başlık (TR / EN) &amp; Tip</th>
                                    <th>Çoklu Gösterim (`placements`)</th>
                                    <th>SSB İlgi Alanları (`ssb_tags`)</th>
                                    <th>Paywall &amp; Sponsor</th>
                                    <th>Okuma (View / True)</th>
                                    <th>İşlem</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-contents">
                                <tr><td colspan="7" style="text-align:center; padding:24px; color:var(--text-muted);">Canlı içerik veritabanı yükleniyor...</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 2: AKILLI REKLAM YÖNETİMİ, ÇAKIŞMA KONTROLÜ & RAKİP KORUMA
             =================================================================== -->
        <section id="tab-2" class="tab-pane">
            <!-- Competitor Protection Status Banner -->
            <div class="glass-panel" style="margin-bottom:20px; border-left:4px solid var(--msi-gold);">
                <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:14px;">
                    <div>
                        <div style="font-size:14px; font-weight:800; color:#fbbf24; display:flex; align-items:center; gap:8px;">
                            <i class="fa-solid fa-shield-cat"></i> SPONSORLU İÇERİK RAKİP KORUMA KURALI (COMPETITOR SHIELD) AKTİF
                        </div>
                        <div style="font-size:12.5px; color:var(--text-secondary); margin-top:4px;">
                            Bir içerik <strong>Sponsorlu (`is_sponsored = true`)</strong> olarak işaretlendiğinde, o sayfada başka bir <code>defense_oem</code> (Ana Savunma Sanayii Üreticisi) reklamı otomatik olarak engellenir. Yalnızca aynı markanın reklamları, <code>auxiliary_service</code> (Yan Hizmet / Lojistik / Finans) reklamları veya <code>in_house</code> (MSI Öz Tanıtım) banner'ları gösterilir.
                        </div>
                    </div>
                    <span class="live-pulse-badge"><span class="live-pulse-dot"></span> KORUMA MOTORU DEVREDE</span>
                </div>
            </div>

            <div class="workspace-grid">
                <!-- Left: Smart Ad Campaign Form + Real-Time Conflict Check -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-rectangle-ad"></i> Akıllı Reklam Kampanyası &amp; Kurumsal Hedefleme</div>
                            <div class="panel-subtitle">Slot, Kurum (MSB/SSB/Kuvvetler) ve SSB İlgi Alanı bazlı dinamik reklam tanımlama.</div>
                        </div>
                        <button type="button" class="btn-cmd btn-cmd-gold" onclick="window.MSIAdmin.checkAdConflictLive()">
                            <i class="fa-solid fa-triangle-exclamation"></i> Çakışma Kontrolü Yap
                        </button>
                    </div>

                    <!-- Real-time Conflict Detection Alert Banner -->
                    <div id="ad-conflict-banner" class="conflict-alert-box">
                        <i class="fa-solid fa-circle-check" style="font-size:18px; margin-top:2px;"></i>
                        <div id="ad-conflict-text">
                            <strong>Çakışma &amp; Kota Kontrolü Hazır:</strong> Seçilen slot, kurum ve SSB anahtar kelimeleri için gerçek zamanlı kota analizi (`/api/v1/admin/ads/check-conflict`) otomatik çalışır.
                        </div>
                    </div>

                    <form id="form-ad-campaign" onsubmit="return window.MSIAdmin.submitAdForm(event)">
                        <input type="hidden" id="ad-id" value="0">

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="ad-slot-code">Reklam Alanı Kodu (`slot_code`) *</label>
                                <select id="ad-slot-code" class="form-select" onchange="window.MSIAdmin.checkAdConflictLive()">
                                    <option value="ad-mag-top">ad-mag-top (Dergi Üst Leaderboard 970x90)</option>
                                    <option value="ad-mag-mid">ad-mag-mid (Dergi Orta Kuşak 728x90)</option>
                                    <option value="ad-mag-pricing">ad-mag-pricing (Abonelik Yanı 300x250)</option>
                                    <option value="ad-news-top">ad-news-top (Haber Üst Leaderboard 970x90)</option>
                                    <option value="ad-news-infeed">ad-news-infeed (Haber Akış İçi 728x90)</option>
                                    <option value="ad-news-mid">ad-news-mid (Haber Orta Kutu 300x250)</option>
                                    <option value="ad-news-billboard">ad-news-billboard (Prestij Billboard 970x250)</option>
                                    <option value="ad-floating-popup">ad-floating-popup (Giriş Pop-up / Floating 600x400)</option>
                                    <option value="ad-newsletter-top">ad-newsletter-top (E-Bülten Sponsor Kuşağı 600x120)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-sponsor-type">Sponsor Tipi (`sponsor_type` — Rakip Koruması) *</label>
                                <select id="ad-sponsor-type" class="form-select">
                                    <option value="defense_oem">defense_oem (Savunma OEM — Rakip Korumasına Tabi)</option>
                                    <option value="auxiliary_service">auxiliary_service (Yan Hizmet — Rakip Korumasından Muaf)</option>
                                    <option value="in_house">in_house (MSI Dergisi Öz Tanıtım / Fallback)</option>
                                </select>
                            </div>
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="ad-sponsor-name">Sponsor Marka (`sponsor_name`) *</label>
                                <input type="text" id="ad-sponsor-name" class="form-control" required placeholder="Örn: ASELSAN, ROKETSAN, STM">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-dimensions">Boyut (`dimensions`)</label>
                                <input type="text" id="ad-dimensions" class="form-control" value="970x90">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-budget-score">Bütçe / Ağırlık Puanı (`budget_score`)</label>
                                <input type="number" id="ad-budget-score" class="form-control" value="85" min="1" max="100">
                            </div>
                        </div>

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="ad-title-tr">Kampanya Başlığı (TR) *</label>
                                <input type="text" id="ad-title-tr" class="form-control" required placeholder="Örn: AESA Radar Teknolojilerinde Küresel Liderlik">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-title-en">Campaign Title (EN)</label>
                                <input type="text" id="ad-title-en" class="form-control" placeholder="Global Leadership in AESA Radar Systems">
                            </div>
                        </div>

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="ad-desc-tr">Açıklama Metni (TR) *</label>
                                <input type="text" id="ad-desc-tr" class="form-control" required placeholder="Yeni nesil hava ve deniz platformları için entegre çözümler.">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-desc-en">Description (EN)</label>
                                <input type="text" id="ad-desc-en" class="form-control" placeholder="Integrated solutions for next-gen air and naval platforms.">
                            </div>
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="ad-target-url">Hedef URL (`target_url`) *</label>
                                <input type="text" id="ad-target-url" class="form-control" required value="https://www.msidergisi.com">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-badge-label">Yasal Etiket (`badge_label`)</label>
                                <input type="text" id="ad-badge-label" class="form-control" value="SPONSORLU REKLAM ALANI">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="ad-pill-style">Tema Stili (`pill_style`)</label>
                                <select id="ad-pill-style" class="form-select">
                                    <option value="blue">Mavi (Taktik Kurumsal)</option>
                                    <option value="gold">Altın (VIP / Lansman)</option>
                                    <option value="emerald">Zümrüt (Teknoloji)</option>
                                </select>
                            </div>
                        </div>

                        <!-- Target Institutions -->
                        <div class="form-group">
                            <label class="form-label"><i class="fa-solid fa-building-columns" style="color:var(--msi-cyan);"></i> Hedef Kurumlar (`target_institutions` — Çalışılan Yer Hedeflemesi)</label>
                            <div class="checkbox-matrix" id="ad-institutions-matrix" onchange="window.MSIAdmin.checkAdConflictLive()">
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="MSB" checked> <span>MSB</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="SSB" checked> <span>SSB</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="K.K.K.lığı" checked> <span>K.K.K.lığı</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="Dz.K.K.lığı"> <span>Dz.K.K.lığı</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="Hv.K.K.lığı" checked> <span>Hv.K.K.lığı</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="J.G.K.lığı"> <span>J.G.K.lığı</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="EGM"> <span>EGM</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_inst" value="Sektör Kuruluşları" checked> <span>Sektör Kuruluşları</span></label>
                            </div>
                        </div>

                        <!-- Target SSB Interest Keywords -->
                        <div class="form-group">
                            <label class="form-label"><i class="fa-solid fa-tags" style="color:var(--msi-emerald);"></i> Hedef SSB İlgi Alanı Anahtar Kelimeleri (`target_ssb_tags`)</label>
                            <div class="checkbox-matrix" id="ad-ssb-matrix" onchange="window.MSIAdmin.checkAdConflictLive()">
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="hava-araclari" checked> <span>Hava Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="deniz-araclari"> <span>Deniz Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="kara-araclari"> <span>Kara Araçları</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="fuzeler-roketler-ve-muhimmatlar"> <span>Füzeler &amp; Mühimmatlar</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="silahlar-ve-kuleler"> <span>Silahlar ve Kuleler</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="radarlar-ve-sensorler" checked> <span>Radarlar ve Sensörler</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="elektronik-harp"> <span>Elektronik Harp</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="muhabere-ve-bilisim"> <span>Muhabere ve Bilişim</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="komuta-kontrol"> <span>Komuta Kontrol</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="simulasyon-ve-egitim"> <span>Simülasyon ve Eğitim</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="alt-sistemler"> <span>Alt Sistemler</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="lojistik-destek-ve-bakim"> <span>Lojistik Destek</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="siber-guvenlik-ve-yapay-zeka"> <span>Siber Güvenlik &amp; YZ</span></label>
                                <label class="check-pill"><input type="checkbox" name="ad_ssb" value="uzay-ve-uydu-sistemleri"> <span>Uzay ve Uydu</span></label>
                            </div>
                        </div>

                        <!-- Target Categories -->
                        <div class="form-group">
                            <label class="form-label" for="ad-target-categories">Hedef Site Kategorileri (`target_categories` — Virgülle ayrılmış)</label>
                            <input type="text" id="ad-target-categories" class="form-control" value="ozel-haberler, kapak-dosyalari, soylesiler">
                        </div>

                        <div style="display:flex; flex-wrap:wrap; gap:12px; margin-bottom:14px;">
                            <label class="check-pill"><input type="checkbox" id="ad-is-fallback"> <span>Varsayılan Fallback Reklamı</span></label>
                            <label class="check-pill"><input type="checkbox" id="ad-is-popup"> <span>Floating / Pop-up Katmanı</span></label>
                            <label class="check-pill"><input type="checkbox" id="ad-is-newsletter"> <span>E-Bülten Reklamı</span></label>
                            <label class="check-pill"><input type="checkbox" id="ad-is-active" checked> <span>Kampanya Aktif</span></label>
                        </div>

                        <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center; padding:12px; font-size:13.5px;">
                            <i class="fa-solid fa-satellite-dish"></i> Reklam Kampanyasını Kaydet &amp; Yayına Al
                        </button>
                    </form>
                </div>

                <!-- Right: Live Ad Inventory Table -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-bullhorn"></i> Aktif Reklam Envanteri &amp; Kurumsal Hedef Matrisi</div>
                            <div class="panel-subtitle">Tüm reklam slotları, sponsor tipleri, hedef kurumlar ve canlı gösterim/tıklama performansları.</div>
                        </div>
                    </div>
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Slot &amp; Boyut</th>
                                    <th>Sponsor &amp; Tip</th>
                                    <th>Başlık &amp; Bütçe Puanı</th>
                                    <th>Hedef Kurumlar (`target_institutions`)</th>
                                    <th>SSB Etiketleri</th>
                                    <th>Gösterim / Tık / CTR</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-ads">
                                <tr><td colspan="6" style="text-align:center; padding:24px; color:var(--text-muted);">Reklam kampanyaları yükleniyor...</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 3: 3 BOYUTLU B2B KURUMSAL ANALİTİK & OKUMA DERİNLİĞİ RAPORLARI
             =================================================================== -->
        <section id="tab-3" class="tab-pane">
            <div class="glass-panel" style="margin-bottom:20px;">
                <div class="panel-header" style="margin-bottom:12px; padding-bottom:12px;">
                    <div>
                        <div class="panel-title"><i class="fa-solid fa-chart-column"></i> 3 Boyutlu B2B Savunma Analitik &amp; Okuma Derinliği Merkezi</div>
                        <div class="panel-subtitle">İçerik Özelinde, Reklam Özelinde ve Okuyucu Bazlı Kurumsal Dağılım (MSB, SSB, Kuvvet Komutanlıkları, Sektör Kuruluşları).</div>
                    </div>
                    <div style="display:flex; gap:10px; flex-wrap:wrap;">
                        <button type="button" class="btn-cmd btn-cmd-gold" onclick="window.MSIAdmin.downloadMonthlyConsolidatedReport()">
                            <i class="fa-solid fa-file-export"></i> Ay Sonu Otomatik Konsolide Rapor Üret &amp; İndir
                        </button>
                    </div>
                </div>

                <div class="subview-bar">
                    <button type="button" class="btn-cmd btn-cmd-primary" id="btn-subview-content" onclick="window.MSIAdmin.switchAnalyticsSubview('content')">
                        <i class="fa-solid fa-newspaper"></i> A. İçerik Özelinde Rapor (Okuma Derinliği &amp; Kurum Dağılımı)
                    </button>
                    <button type="button" class="btn-cmd" id="btn-subview-ad" onclick="window.MSIAdmin.switchAnalyticsSubview('ad')">
                        <i class="fa-solid fa-rectangle-ad"></i> B. Reklam Özelinde Rapor (Gören &amp; Tıklayan Kurum Dağılımı)
                    </button>
                    <button type="button" class="btn-cmd" id="btn-subview-reader" onclick="window.MSIAdmin.switchAnalyticsSubview('reader')">
                        <i class="fa-solid fa-user-shield"></i> C. Okuyucu Bazlı Rapor (Okunan Makaleler, SSB &amp; Reklam Etkileşimi)
                    </button>
                </div>

                <!-- Sub-view A: Content Analytics Report -->
                <div id="subview-content" class="subview-pane active">
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>İçerik Başlığı (`title_tr`)</th>
                                    <th>Toplam Görüntülenme</th>
                                    <th>Gerçek Okuma (`true_reads`)</th>
                                    <th>Ort. Süre (Sn) &amp; Scroll %</th>
                                    <th style="min-width:340px;">Çalışılan Yer / Kurum Dağılımı (`institution_breakdown`)</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-analytics-content">
                                <tr><td colspan="5" style="text-align:center; padding:24px;">İçerik analitiği yükleniyor...</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>

                <!-- Sub-view B: Ad Analytics Report -->
                <div id="subview-ad" class="subview-pane">
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Reklam Kampanyası &amp; Slot</th>
                                    <th>Gösterim / Tık / CTR %</th>
                                    <th style="min-width:290px;">Görenlerin Kurum Dağılımı (`impression_institutions`)</th>
                                    <th style="min-width:290px;">Tıklayanların Kurum Dağılımı (`click_institutions`)</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-analytics-ad">
                                <tr><td colspan="4" style="text-align:center; padding:24px;">Reklam analitiği yükleniyor...</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>

                <!-- Sub-view C: Reader Analytics Report -->
                <div id="subview-reader" class="subview-pane">
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Okuyucu Ad-Soyad &amp; E-Posta</th>
                                    <th>Çalışılan Yer / Kurum (`organization_category`)</th>
                                    <th>Abonelik Kademesi</th>
                                    <th>Okuduğu Makaleler (`articles_read`)</th>
                                    <th>Etkileşimli SSB Alanları</th>
                                    <th>Reklam Gösterim / Tık (`clicked_ads`)</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-analytics-reader">
                                <tr><td colspan="6" style="text-align:center; padding:24px;">Okuyucu bazlı analitik yükleniyor...</td></tr>
                            </tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 4: KAMU (.GOV.TR / .TSK.TR) & KURUMSAL ÜCRETSİZ ABONELİK ONAY HAVUZU
             =================================================================== -->
        <section id="tab-4" class="tab-pane">
            <div class="glass-panel">
                <div class="panel-header">
                    <div>
                        <div class="panel-title"><i class="fa-solid fa-building-shield"></i> Kamu (`.gov.tr` / `.tsk.tr`) &amp; Kurumsal Ücretsiz Abonelik Onay Havuzu</div>
                        <div class="panel-subtitle">
                            <code>.gov.tr</code> ve <code>.tsk.tr</code> uzantılı e-postalar otomatik doğrulanır; harici e-posta ile başvuran kamu/kurum personelinin taleplerini buradan tek tıkla onaylayabilir veya reddedebilirsiniz.
                        </div>
                    </div>
                </div>

                <div class="table-responsive">
                    <table class="ops-table">
                        <thead>
                            <tr>
                                <th>Talep ID &amp; Tarih</th>
                                <th>Ad Soyad &amp; E-Posta</th>
                                <th>Çalışılan Yer (`organization_category`) &amp; Kurum Adı</th>
                                <th>Kullanıcı Beyanı / Talep Mesajı (`request_message`)</th>
                                <th>Durum (`status`)</th>
                                <th>Operatör Kararı</th>
                            </tr>
                        </thead>
                        <tbody id="tbody-corporate-requests">
                            <tr><td colspan="6" style="text-align:center; padding:24px;">Kurumsal doğrulama talepleri yükleniyor...</td></tr>
                        </tbody>
                    </table>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 5: ESNEK SİTE AĞACI (KATEGORİ & SSB İLGİ ALANI TAKSONOMİ YÖNETİMİ)
             =================================================================== -->
        <section id="tab-5" class="tab-pane">
            <div class="workspace-grid">
                <!-- Left: Dynamic Category & SSB Tag Creation -->
                <div style="display:flex; flex-direction:column; gap:20px;">
                    <div class="glass-panel">
                        <div class="panel-header">
                            <div>
                                <div class="panel-title"><i class="fa-solid fa-folder-plus"></i> Site Ağacına Yeni Bölüm / Kategori Ekle</div>
                                <div class="panel-subtitle">Kod değişikliği gerektirmeden menüye ve içerik dağıtım ağacına yeni kategori ekleyin.</div>
                            </div>
                        </div>
                        <form id="form-new-category" onsubmit="return window.MSIAdmin.submitCategoryForm(event)">
                            <div class="form-row-2">
                                <div class="form-group">
                                    <label class="form-label" for="cat-name-tr">Kategori Adı (TR) *</label>
                                    <input type="text" id="cat-name-tr" class="form-control" required placeholder="Örn: İnsansız Deniz Sistemleri">
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="cat-name-en">Category Name (EN) *</label>
                                    <input type="text" id="cat-name-en" class="form-control" required placeholder="e.g. Unmanned Naval Systems">
                                </div>
                            </div>
                            <div class="form-row-3">
                                <div class="form-group">
                                    <label class="form-label" for="cat-slug">Slug (URL Kodu)</label>
                                    <input type="text" id="cat-slug" class="form-control" placeholder="insansiz-deniz-sistemleri">
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="cat-group-type">Grup (`group_type`)</label>
                                    <select id="cat-group-type" class="form-select">
                                        <option value="icerikler">İçerikler (icerikler)</option>
                                        <option value="gruplandirilmis">Gruplandırılmış (gruplandirilmis)</option>
                                        <option value="abonelik">Abonelik (abonelik)</option>
                                        <option value="kurumsal">Kurumsal (kurumsal)</option>
                                    </select>
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="cat-sort-order">Sıra (`sort_order`)</label>
                                    <input type="number" id="cat-sort-order" class="form-control" value="25">
                                </div>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="cat-desc-tr">Açıklama (TR)</label>
                                <input type="text" id="cat-desc-tr" class="form-control" placeholder="Bölüm açıklaması...">
                            </div>
                            <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center;">
                                <i class="fa-solid fa-plus"></i> Yeni Kategoriyi Site Ağacına Ekle
                            </button>
                        </form>
                    </div>

                    <div class="glass-panel">
                        <div class="panel-header">
                            <div>
                                <div class="panel-title"><i class="fa-solid fa-crosshairs"></i> Yeni SSB Katalog İlgi Alanı Etiketi Ekle</div>
                                <div class="panel-subtitle">Okuyucu profili, içerik etiketleme ve reklam hedefleme matrisine anında yansır.</div>
                            </div>
                        </div>
                        <form id="form-new-ssb" onsubmit="return window.MSIAdmin.submitSsbTagForm(event)">
                            <div class="form-row-2">
                                <div class="form-group">
                                    <label class="form-label" for="ssb-name-tr">SSB İlgi Alanı Adı (TR) *</label>
                                    <input type="text" id="ssb-name-tr" class="form-control" required placeholder="Örn: Kuantum ve Fotonik Teknolojiler">
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="ssb-name-en">SSB Interest Name (EN) *</label>
                                    <input type="text" id="ssb-name-en" class="form-control" required placeholder="e.g. Quantum &amp; Photonic Technologies">
                                </div>
                            </div>
                            <div class="form-row-3">
                                <div class="form-group">
                                    <label class="form-label" for="ssb-code">Kod (`code`)</label>
                                    <input type="text" id="ssb-code" class="form-control" placeholder="kuantum-ve-fotonik">
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="ssb-group">Ana Grup (`category_group`)</label>
                                    <input type="text" id="ssb-group" class="form-control" value="İleri Teknolojiler">
                                </div>
                                <div class="form-group">
                                    <label class="form-label" for="ssb-sort">Sıra (`sort_order`)</label>
                                    <input type="number" id="ssb-sort" class="form-control" value="15">
                                </div>
                            </div>
                            <button type="submit" class="btn-cmd btn-cmd-emerald" style="width:100%; justify-content:center;">
                                <i class="fa-solid fa-tags"></i> Yeni SSB İlgi Alanını Sisteme Tanımla
                            </button>
                        </form>
                    </div>
                </div>

                <!-- Right: Live Site Tree & SSB Taxonomy Tables -->
                <div style="display:flex; flex-direction:column; gap:20px;">
                    <div class="glass-panel">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-sitemap"></i> Mevcut Site Ağacı (`categories`)</div>
                        </div>
                        <div class="table-responsive" style="max-height:320px; overflow-y:auto;">
                            <table class="ops-table">
                                <thead>
                                    <tr>
                                        <th>Slug</th>
                                        <th>Kategori Adı (TR / EN)</th>
                                        <th>Grup Tipi</th>
                                        <th>Dinamik</th>
                                    </tr>
                                </thead>
                                <tbody id="tbody-categories"></tbody>
                            </table>
                        </div>
                    </div>

                    <div class="glass-panel">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-list-check"></i> SSB Katalog İlgi Alanları (`ssb_interest_tags`)</div>
                        </div>
                        <div class="table-responsive" style="max-height:340px; overflow-y:auto;">
                            <table class="ops-table">
                                <thead>
                                    <tr>
                                        <th>Kod (`code`)</th>
                                        <th>İlgi Alanı (TR / EN)</th>
                                        <th>Grup</th>
                                        <th>Sıra</th>
                                    </tr>
                                </thead>
                                <tbody id="tbody-ssb-tags"></tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 6: DİNAMİK E-POSTA GRUPLARI & SPONSORLU BÜLTEN (NEWSLETTER) GÖNDERİMİ
             =================================================================== -->
        <section id="tab-6" class="tab-pane">
            <div class="workspace-grid">
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-paper-plane"></i> Dinamik E-Posta Grubu Oluştur &amp; Sponsorlu Bülten Gönder</div>
                            <div class="panel-subtitle">Kurum (`MSB`, `SSB`, `Sektör Kuruluşları`), SSB İlgi Alanı ve Rol filtreleriyle hedefli bülten.</div>
                        </div>
                    </div>

                    <form id="form-newsletter" onsubmit="return window.MSIAdmin.submitNewsletterForm(event)">
                        <div class="form-group">
                            <label class="form-label" for="nl-subject-tr">Bülten Konu Başlığı (TR — `subject_tr`) *</label>
                            <input type="text" id="nl-subject-tr" class="form-control" required placeholder="Örn: [MSI Özel Bülten] Hava Savunma ve Radar Sistemlerinde Kritik Gelişmeler">
                        </div>
                        <div class="form-group">
                            <label class="form-label" for="nl-subject-en">Newsletter Subject (EN — `subject_en`) *</label>
                            <input type="text" id="nl-subject-en" class="form-control" required placeholder="e.g. [MSI TDR Briefing] Critical Developments in Air Defence &amp; Radar">
                        </div>

                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="nl-institution">Alıcı Kurum Filtresi (`target_institution`)</label>
                                <select id="nl-institution" class="form-select">
                                    <option value="ALL">Tüm Kurumlar (Genel)</option>
                                    <option value="MSB">MSB (Milli Savunma Bakanlığı)</option>
                                    <option value="SSB">SSB (Savunma Sanayii Başkanlığı)</option>
                                    <option value="K.K.K.lığı">K.K.K.lığı</option>
                                    <option value="Dz.K.K.lığı">Dz.K.K.lığı</option>
                                    <option value="Hv.K.K.lığı">Hv.K.K.lığı</option>
                                    <option value="Sektör Kuruluşları">Sektör Kuruluşları</option>
                                    <option value="Üniversite / Öğrenci">Üniversite / Öğrenci</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="nl-ssb-tag">SSB İlgi Alanı (`target_ssb_tag`)</label>
                                <select id="nl-ssb-tag" class="form-select">
                                    <option value="ALL">Tüm SSB İlgi Alanları</option>
                                    <option value="hava-araclari">Hava Araçları</option>
                                    <option value="deniz-araclari">Deniz Araçları</option>
                                    <option value="kara-araclari">Kara Araçları</option>
                                    <option value="radarlar-ve-sensorler">Radarlar ve Sensörler</option>
                                    <option value="elektronik-harp">Elektronik Harp</option>
                                    <option value="fuzeler-roketler-ve-muhimmatlar">Füzeler ve Mühimmatlar</option>
                                    <option value="siber-guvenlik-ve-yapay-zeka">Siber Güvenlik ve YZ</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="nl-group-filter">Kullanıcı Grubu (`target_group_filter`)</label>
                                <select id="nl-group-filter" class="form-select">
                                    <option value="all_consented">KVKK &amp; İleti Onaylı Tüm Okuyucular</option>
                                    <option value="gov_verified">Doğrulanmış Kamu (.gov.tr / .tsk.tr)</option>
                                    <option value="active_subscribers">Aktif Kurumsal &amp; Bireysel Aboneler</option>
                                </select>
                            </div>
                        </div>

                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="nl-sponsor-name">Bülten Sponsoru (`sponsor_name`)</label>
                                <input type="text" id="nl-sponsor-name" class="form-control" value="ASELSAN — Bülten Ana Sponsoru">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="nl-sponsor-ad-id">Sponsor Reklam Kampanya ID (`sponsor_ad_id`)</label>
                                <input type="number" id="nl-sponsor-ad-id" class="form-control" value="1">
                            </div>
                        </div>

                        <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center; padding:12px;">
                            <i class="fa-solid fa-envelope-circle-check"></i> Dinamik Gruba Sponsorlu Bülteni Gönder ve Kayıt Altına Al
                        </button>
                    </form>
                </div>

                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-clock-rotate-left"></i> Gönderilen E-Bülten Kampanyaları (`newsletter_campaigns`)</div>
                            <div class="panel-subtitle">Hedeflenen kurum, SSB etiketi, sponsor banner ve ulaşılan alıcı sayısı.</div>
                        </div>
                    </div>
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>ID / Tarih</th>
                                    <th>Konu (TR / EN)</th>
                                    <th>Hedef Kurum &amp; SSB Filtresi</th>
                                    <th>Sponsor Banner</th>
                                    <th>Alıcı Sayısı &amp; Durum</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-newsletters"></tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 7: ABONELİK OTOMASYONU (5 KADEMELİ HATIRLATMA), 301 SEO & AÇIK VERİ
             =================================================================== -->
        <section id="tab-7" class="tab-pane">
            <!-- Open Data Zero Vendor-Lock Banner -->
            <div class="glass-panel" style="margin-bottom:20px; border-left:4px solid var(--msi-emerald);">
                <div style="display:flex; align-items:center; justify-content:space-between; flex-wrap:wrap; gap:14px;">
                    <div>
                        <div style="font-size:15px; font-weight:800; color:#34d399; display:flex; align-items:center; gap:8px;">
                            <i class="fa-solid fa-unlock-keyhole"></i> AÇIK VERİ MİMARİSİ (ZERO VENDOR-LOCK) &amp; TAM VERİTABANI YEDEKLEME
                        </div>
                        <div style="font-size:12.5px; color:var(--text-secondary); margin-top:4px;">
                            Tüm içerikler, çoklu yerleşimler, SSB taksonomisi, kurumsal analitikler, kullanıcılar ve abonelik kayıtları şifresiz, standart JSON/SQL formatında tek tıkla dışa aktarılabilir.
                        </div>
                    </div>
                    <a href="/api/v1/admin/export/open-data" target="_blank" class="btn-cmd btn-cmd-emerald" style="padding:11px 18px; font-size:13px;">
                        <i class="fa-solid fa-download"></i> Açık Veri JSON / SQL Tam Yedek İndir (`/api/v1/admin/export/open-data`)
                    </a>
                </div>
            </div>

            <div class="workspace-grid">
                <!-- Left: 5-Stage Subscription Renewal Reminder Automation -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-bell"></i> 5 Kademeli Abonelik Yenileme Otomasyonu</div>
                            <div class="panel-subtitle">Bitişe <strong>2 Ay (60g)</strong>, <strong>1 Ay (30g)</strong>, <strong>2 Hafta (14g)</strong>, <strong>1 Hafta (7g)</strong> ve <strong>1 Gün (1g)</strong> kala otomatik uyarı; yenileyenlerde anında durdurma (`halted_by_renewal`).</div>
                        </div>
                        <button type="button" class="btn-cmd btn-cmd-gold" onclick="window.MSIAdmin.simulateRenewalHalt()">
                            <i class="fa-solid fa-ban"></i> Yenileyenlerde Otomatik Durdurma Testi
                        </button>
                    </div>
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Abone &amp; Kurum</th>
                                    <th>Kademe (`stage_label`)</th>
                                    <th>Kalan Gün</th>
                                    <th>Durum (`status`)</th>
                                    <th>Zaman Damgası</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-reminders"></tbody>
                        </table>
                    </div>
                </div>

                <!-- Right: 301 SEO Redirect Manager -->
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-route"></i> 301 Kalıcı SEO Yönlendirme Yöneticisi (`seo_redirect_rules`)</div>
                            <div class="panel-subtitle">Eski site URL'lerini yeni Single-Source-of-Truth adreslerine 0 SEO kaybıyla yönlendirin.</div>
                        </div>
                    </div>
                    <form id="form-seo-redirect" onsubmit="return window.MSIAdmin.submitSeoRedirect(event)" style="margin-bottom:16px;">
                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="seo-old-path">Eski URL (`old_path`) *</label>
                                <input type="text" id="seo-old-path" class="form-control" required placeholder="/eski-haber-adresi.html">
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="seo-new-path">Yeni URL (`new_path`) *</label>
                                <input type="text" id="seo-new-path" class="form-control" required placeholder="/icerik/kaan-milli-muharip-ucak">
                            </div>
                            <div class="form-group" style="display:flex; align-items:flex-end;">
                                <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center; height:38px;">
                                    <i class="fa-solid fa-plus"></i> 301 Kuralı Ekle
                                </button>
                            </div>
                        </div>
                    </form>

                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Eski Adres (`old_path`)</th>
                                    <th>Yeni Adres (`new_path`)</th>
                                    <th>HTTP Kodu</th>
                                    <th>Hit Sayısı (`hit_count`)</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-redirects"></tbody>
                        </table>
                    </div>
                </div>
            </div>
        </section>

        <!-- ===================================================================
             TAB 8: MERKEZİ MOTORLAR (TASARIM AYARLARI, BİLDİRİM & WAF GÜVENLİK)
             =================================================================== -->
        <section id="tab-8" class="tab-pane">
            <div class="workspace-grid">
                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title">Site ayarları · <a href="/admin/tasarim">Görsel tasarım stüdyosunu aç ↗</a></div>
                            <div class="panel-subtitle">Tüm SSR sayfalarındaki tema modu, vurgu rengi, güncel dergi sayısı ve üst duyuru metnini anında güncelleyin (L1 Cache otomatik yenilenir).</div>
                        </div>
                    </div>
                    <form id="form-site-setting" onsubmit="return window.MSIAdmin.submitSiteSetting(event)" style="margin-bottom:18px;">
                        <div class="form-row-2">
                            <div class="form-group">
                                <label class="form-label" for="set-key">Ayar Anahtarı (`key`) *</label>
                                <select id="set-key" class="form-select">
                                    <option value="announcement_banner_text">announcement_banner_text (Üst Duyuru Kuşağı)</option>
                                    <option value="current_featured_issue">current_featured_issue (Vitrin Dergi Sayısı: 286)</option>
                                    <option value="contact_phone">contact_phone (Merkez Telefon)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="set-group">Grup (`setting_group`)</label>
                                <input type="text" id="set-group" class="form-control" value="theme">
                            </div>
                        </div>
                        <div class="form-group">
                            <label class="form-label" for="set-val-tr">Türkçe Değer (`value_tr`) *</label>
                            <input type="text" id="set-val-tr" class="form-control" required placeholder="Yeni ayar değerini giriniz...">
                        </div>
                        <div class="form-group">
                            <label class="form-label" for="set-val-en">İngilizce Değer (`value_en`)</label>
                            <input type="text" id="set-val-en" class="form-control" placeholder="English value...">
                        </div>
                        <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center;">
                            <i class="fa-solid fa-bolt"></i> Ayarı Kaydet &amp; L1 Önbelleği Yenile
                        </button>
                    </form>

                    <div class="panel-header" style="margin-top:24px;">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-bell"></i> Merkezi Bildirim &amp; Duyuru Yayın Motoru (`system_notifications`)</div>
                            <div class="panel-subtitle">Tüm kullanıcılara veya belirli bir kuruma (MSB, SSB, TSK) anlık bildirim ve üst bant duyurusu gönderin.</div>
                        </div>
                    </div>
                    <form id="form-sys-notif" onsubmit="return window.MSIAdmin.submitSysNotification(event)">
                        <div class="form-row-3">
                            <div class="form-group">
                                <label class="form-label" for="notif-audience">Hedef Kitle</label>
                                <select id="notif-audience" class="form-select">
                                    <option value="ALL">TÜM ZİYARETÇİLER (ALL)</option>
                                    <option value="MSB">MSB</option>
                                    <option value="SSB">SSB</option>
                                    <option value="Sektör Kuruluşları">Sektör Kuruluşları</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="notif-severity">Önem Derecesi</label>
                                <select id="notif-severity" class="form-select">
                                    <option value="info">Bilgi (info)</option>
                                    <option value="success">Başarı (success)</option>
                                    <option value="warning">Uyarı (warning)</option>
                                    <option value="critical">Kritik (critical)</option>
                                </select>
                            </div>
                            <div class="form-group">
                                <label class="form-label" for="notif-banner">Üst Bant (Banner)?</label>
                                <select id="notif-banner" class="form-select">
                                    <option value="true">Evet (Üst Duyuru Bandı)</option>
                                    <option value="false">Hayır (Sadece Bildirim Çanı)</option>
                                </select>
                            </div>
                        </div>
                        <div class="form-group">
                            <label class="form-label" for="notif-title-tr">Başlık (TR) *</label>
                            <input type="text" id="notif-title-tr" class="form-control" required placeholder="Örn: 286. Sayı Dijital Kiosk'ta Yayında!">
                        </div>
                        <div class="form-group">
                            <label class="form-label" for="notif-msg-tr">Mesaj (TR) *</label>
                            <input type="text" id="notif-msg-tr" class="form-control" required placeholder="Bildirim detay metni...">
                        </div>
                        <button type="submit" class="btn-cmd btn-cmd-primary" style="width:100%; justify-content:center;">
                            <i class="fa-solid fa-paper-plane"></i> Merkezi Bildirimi Yayınla
                        </button>
                    </form>
                </div>

                <div class="glass-panel">
                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-shield-halved"></i> Merkezi WAF Güvenlik Filtresi &amp; Saldırı Denetim Logları (`security_audit_logs`)</div>
                            <div class="panel-subtitle">SQL Injection, XSS, Path Traversal ve Rate-Limit ihlallerinin gerçek zamanlı blokaj kayıtları.</div>
                        </div>
                    </div>
                    <div class="table-responsive" style="margin-bottom:22px;">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Zaman</th>
                                    <th>İstemci IP</th>
                                    <th>Tür (`event_type`)</th>
                                    <th>Seviye</th>
                                    <th>İstek Yolu &amp; Detay</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-security-logs"></tbody>
                        </table>
                    </div>

                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-gears"></i> Aktif Merkezi Site Ayarları (`site_settings`)</div>
                        </div>
                    </div>
                    <div class="table-responsive" style="margin-bottom:22px;">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Anahtar (`key`)</th>
                                    <th>Değer (TR)</th>
                                    <th>Değer (EN)</th>
                                    <th>Grup</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-site-settings"></tbody>
                        </table>
                    </div>

                    <div class="panel-header">
                        <div>
                            <div class="panel-title"><i class="fa-solid fa-bullhorn"></i> Aktif Merkezi Bildirimler &amp; Üst Bant Duyuruları (`system_notifications`)</div>
                        </div>
                    </div>
                    <div class="table-responsive">
                        <table class="ops-table">
                            <thead>
                                <tr>
                                    <th>Hedef</th>
                                    <th>Seviye</th>
                                    <th>Başlık &amp; Mesaj (TR)</th>
                                    <th>Üst Bant</th>
                                </tr>
                            </thead>
                            <tbody id="tbody-sys-notifications"></tbody>
                        </table>
                    </div>
                </div>
            </div>

            <!-- TAB 9: Analytics Charts & System Health -->
            <div class="tab-pane" id="tab-9">
                <div class="panel-grid">
                    <div class="panel-card" style="grid-column: span 2;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-chart-area"></i> İçerik Okuma Trendi (Son 30 Gün)</div>
                        </div>
                        <canvas id="chart-content-trend" height="220" style="width:100%; max-height:280px;"></canvas>
                    </div>
                    <div class="panel-card">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-heart-pulse"></i> Sistem Sağlığı</div>
                            <button type="button" class="btn-cmd btn-cmd-sm" onclick="window.MSIAdmin.refreshSystemHealth()"><i class="fa-solid fa-rotate"></i> Yenile</button>
                        </div>
                        <div id="system-health-panel" style="font-family:var(--font-mono); font-size:12px; line-height:1.8;">
                            <div style="color:var(--text-muted);">Yükleniyor...</div>
                        </div>
                    </div>
                    <div class="panel-card" style="grid-column: span 3;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-chart-bar"></i> En Çok Okunan İçerikler (Top 10)</div>
                        </div>
                        <canvas id="chart-top-contents" height="180" style="width:100%; max-height:240px;"></canvas>
                    </div>
                </div>
            </div>

            <!-- TAB 10: Media Library & SEO Preview -->
            <div class="tab-pane" id="tab-10">
                <div class="panel-grid">
                    <div class="panel-card" style="grid-column: span 2;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-images"></i> Medya Kütüphanesi</div>
                            <div style="display:flex; gap:8px;">
                                <input type="text" id="media-search" placeholder="Dosya adı veya etiket ara..." style="padding:6px 12px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px; width:200px;" oninput="window.MSIAdmin.filterMediaLibrary()">
                                <label class="btn-cmd btn-cmd-sm" style="cursor:pointer;"><i class="fa-solid fa-upload"></i> Yükle <input type="file" multiple style="display:none;" onchange="window.MSIAdmin.bulkUploadMedia(this)"></label>
                            </div>
                        </div>
                        <div id="media-library-grid" style="display:grid; grid-template-columns:repeat(auto-fill, minmax(140px, 1fr)); gap:12px; max-height:500px; overflow-y:auto; padding:8px 0;">
                            <div style="color:var(--text-muted); padding:20px; text-align:center;">Medya yükleniyor...</div>
                        </div>
                    </div>
                    <div class="panel-card">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-magnifying-glass"></i> Canlı SEO Önizleme</div>
                        </div>
                        <div style="margin-bottom:12px;">
                            <label style="font-size:11px; color:var(--text-muted); display:block; margin-bottom:4px;">İçerik Seçin:</label>
                            <select id="seo-preview-select" onchange="window.MSIAdmin.updateSeoPreview()" style="width:100%; padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <option value="">-- İçerik seçin --</option>
                            </select>
                        </div>
                        <div id="seo-preview-panel" style="font-size:12px;">
                            <div style="background:#1a1a2e; border-radius:8px; padding:16px; margin-bottom:12px;">
                                <div style="font-size:10px; color:#38bdf8; margin-bottom:4px;">GOOGLE SEARCH PREVIEW</div>
                                <div id="seo-google-title" style="color:#8ab4f8; font-size:16px; margin-bottom:2px;">Başlık burada görünecek</div>
                                <div id="seo-google-url" style="color:#bdc1c6; font-size:12px; margin-bottom:4px;">msidergisi.com/icerik/slug</div>
                                <div id="seo-google-desc" style="color:#bdc1c6; font-size:13px; line-height:1.4;">Açıklama metni burada görünecek...</div>
                            </div>
                            <div style="background:#1a1a2e; border-radius:8px; padding:16px; margin-bottom:12px;">
                                <div style="font-size:10px; color:#34d399; margin-bottom:8px;">OPEN GRAPH / SOCIAL PREVIEW</div>
                                <div id="seo-og-preview" style="border:1px solid #333; border-radius:6px; overflow:hidden;">
                                    <div style="background:#2a2a3e; height:80px; display:flex; align-items:center; justify-content:center; color:var(--text-muted); font-size:11px;">Görsel önizleme</div>
                                    <div style="padding:10px;">
                                        <div id="seo-og-title" style="font-weight:700; color:#fff; font-size:13px; margin-bottom:4px;">Başlık</div>
                                        <div id="seo-og-desc" style="color:var(--text-muted); font-size:11px;">Açıklama</div>
                                    </div>
                                </div>
                            </div>
                            <div style="background:#1a1a2e; border-radius:8px; padding:16px;">
                                <div style="font-size:10px; color:#fbbf24; margin-bottom:8px;">JSON-LD STRUCTURED DATA</div>
                                <pre id="seo-jsonld" style="font-family:var(--font-mono); font-size:10px; color:#a5b4fc; white-space:pre-wrap; max-height:200px; overflow-y:auto;">{}</pre>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- TAB 11: Roles, Permissions & Audit Log -->
            <div class="tab-pane" id="tab-11">
                <div class="panel-grid">
                    <div class="panel-card">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-user-shield"></i> Kullanıcı Rolleri &amp; İzinler</div>
                            <button type="button" class="btn-cmd btn-cmd-sm" onclick="window.MSIAdmin.showRoleForm()"><i class="fa-solid fa-plus"></i> Yeni Rol</button>
                        </div>
                        <div id="role-form-container" style="display:none; margin-bottom:16px; padding:16px; background:rgba(255,255,255,0.03); border-radius:8px;">
                            <div style="display:grid; grid-template-columns:1fr 1fr; gap:10px; margin-bottom:10px;">
                                <input type="text" id="role-code" placeholder="Rol kodu (örn: editor)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <input type="text" id="role-name-tr" placeholder="Rol adı (TR)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <input type="text" id="role-name-en" placeholder="Role name (EN)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                            </div>
                            <div style="margin-bottom:10px;">
                                <label style="font-size:11px; color:var(--text-muted); display:block; margin-bottom:6px;">İzinler:</label>
                                <div id="role-permissions-grid" style="display:grid; grid-template-columns:repeat(2, 1fr); gap:6px; font-size:11px;">
                                    <label><input type="checkbox" value="content:read"> İçerik Okuma</label>
                                    <label><input type="checkbox" value="content:write"> İçerik Yazma</label>
                                    <label><input type="checkbox" value="content:delete"> İçerik Silme</label>
                                    <label><input type="checkbox" value="ads:manage"> Reklam Yönetimi</label>
                                    <label><input type="checkbox" value="users:manage"> Kullanıcı Yönetimi</label>
                                    <label><input type="checkbox" value="analytics:view"> Analitik Görüntüleme</label>
                                    <label><input type="checkbox" value="settings:manage"> Site Ayarları</label>
                                    <label><input type="checkbox" value="media:upload"> Medya Yükleme</label>
                                    <label><input type="checkbox" value="audit:view"> Denetim Logu</label>
                                    <label><input type="checkbox" value="cache:manage"> Önbellek Yönetimi</label>
                                </div>
                            </div>
                            <button type="button" class="btn-cmd btn-cmd-primary" onclick="window.MSIAdmin.saveRole()"><i class="fa-solid fa-save"></i> Kaydet</button>
                            <button type="button" class="btn-cmd btn-cmd-sm" onclick="document.getElementById('role-form-container').style.display='none'">İptal</button>
                        </div>
                        <div class="table-responsive">
                            <table class="ops-table">
                                <thead><tr><th>Rol Kodu</th><th>Ad (TR)</th><th>Ad (EN)</th><th>İzin Sayısı</th><th>Sistem</th></tr></thead>
                                <tbody id="tbody-user-roles"></tbody>
                            </table>
                        </div>
                    </div>
                    <div class="panel-card" style="grid-column: span 2;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-scroll"></i> Denetim Logu (Audit Trail)</div>
                            <div style="display:flex; gap:8px;">
                                <select id="audit-filter-action" onchange="window.MSIAdmin.filterAuditLogs()" style="padding:6px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:11px;">
                                    <option value="">Tüm İşlemler</option>
                                    <option value="create">Oluşturma</option>
                                    <option value="update">Güncelleme</option>
                                    <option value="delete">Silme</option>
                                    <option value="login">Giriş</option>
                                    <option value="flush">Önbellek Temizleme</option>
                                </select>
                                <input type="text" id="audit-filter-actor" placeholder="Kullanıcı filtrele..." oninput="window.MSIAdmin.filterAuditLogs()" style="padding:6px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:11px; width:150px;">
                            </div>
                        </div>
                        <div class="table-responsive" style="max-height:500px; overflow-y:auto;">
                            <table class="ops-table">
                                <thead><tr><th>Zaman</th><th>Kullanıcı</th><th>İşlem</th><th>Varlık</th><th>Detay</th><th>IP</th></tr></thead>
                                <tbody id="tbody-audit-logs"></tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>

            <!-- TAB 12: Bulk Operations & Version History -->
            <div class="tab-pane" id="tab-12">
                <div class="panel-grid">
                    <div class="panel-card" style="grid-column: span 2;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-list-check"></i> Toplu İçerik İşlemleri</div>
                            <div style="display:flex; gap:8px;">
                                <button type="button" class="btn-cmd btn-cmd-sm" onclick="window.MSIAdmin.selectAllBulkContents()"><i class="fa-solid fa-check-double"></i> Tümünü Seç</button>
                                <button type="button" class="btn-cmd btn-cmd-sm" onclick="window.MSIAdmin.deselectAllBulkContents()"><i class="fa-solid fa-xmark"></i> Seçimi Kaldır</button>
                            </div>
                        </div>
                        <div style="display:flex; gap:8px; margin-bottom:12px; padding:10px; background:rgba(255,255,255,0.03); border-radius:8px; align-items:center;">
                            <span style="font-size:12px; color:var(--text-muted);">Seçili: <strong id="bulk-selected-count" style="color:#34d399;">0</strong></span>
                            <select id="bulk-action-select" style="padding:6px 10px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <option value="">-- İşlem seçin --</option>
                                <option value="set_featured">Manşet Olarak İşaretle</option>
                                <option value="unset_featured">Manşetten Kaldır</option>
                                <option value="set_visibility_0">Açık Erişim Yap</option>
                                <option value="set_visibility_1">Kayıtlı Kullanıcıya Aç</option>
                                <option value="set_visibility_2">Aboneye Aç</option>
                                <option value="delete">Sil (Geri Alınamaz!)</option>
                            </select>
                            <button type="button" class="btn-cmd btn-cmd-primary" onclick="window.MSIAdmin.executeBulkOperation()"><i class="fa-solid fa-play"></i> Uygula</button>
                        </div>
                        <div class="table-responsive" style="max-height:400px; overflow-y:auto;">
                            <table class="ops-table">
                                <thead><tr><th style="width:40px;"><input type="checkbox" id="bulk-select-all-cb" onchange="window.MSIAdmin.toggleBulkSelectAll(this)"></th><th>ID</th><th>Başlık</th><th>Tür</th><th>Görünürlük</th><th>Okuma</th></tr></thead>
                                <tbody id="tbody-bulk-contents"></tbody>
                            </table>
                        </div>
                    </div>
                    <div class="panel-card">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-code-branch"></i> Versiyon Geçmişi</div>
                        </div>
                        <div style="margin-bottom:12px;">
                            <label style="font-size:11px; color:var(--text-muted); display:block; margin-bottom:4px;">İçerik Seçin:</label>
                            <select id="version-history-select" onchange="window.MSIAdmin.loadVersionHistory()" style="width:100%; padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <option value="">-- İçerik seçin --</option>
                            </select>
                        </div>
                        <div id="version-history-list" style="max-height:400px; overflow-y:auto;">
                            <div style="color:var(--text-muted); padding:20px; text-align:center; font-size:12px;">Versiyon geçmişini görmek için bir içerik seçin.</div>
                        </div>
                    </div>
                </div>
            </div>

            <!-- TAB 13: Dynamic Form Fields & Cache Management -->
            <div class="tab-pane" id="tab-13">
                <div class="panel-grid">
                    <div class="panel-card" style="grid-column: span 2;">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-puzzle-piece"></i> Dinamik Form Alanları (Custom Fields)</div>
                            <button type="button" class="btn-cmd btn-cmd-sm" onclick="window.MSIAdmin.showCustomFieldForm()"><i class="fa-solid fa-plus"></i> Yeni Alan</button>
                        </div>
                        <div id="custom-field-form-container" style="display:none; margin-bottom:16px; padding:16px; background:rgba(255,255,255,0.03); border-radius:8px;">
                            <div style="display:grid; grid-template-columns:1fr 1fr 1fr; gap:10px; margin-bottom:10px;">
                                <input type="text" id="cf-key" placeholder="Alan anahtarı (örn: defense_sector)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <input type="text" id="cf-label-tr" placeholder="Etiket (TR)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                <input type="text" id="cf-label-en" placeholder="Label (EN)" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                            </div>
                            <div style="display:grid; grid-template-columns:1fr 1fr 1fr; gap:10px; margin-bottom:10px;">
                                <select id="cf-type" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                    <option value="text">Metin</option>
                                    <option value="textarea">Uzun Metin</option>
                                    <option value="select">Açılır Liste</option>
                                    <option value="checkbox">Onay Kutusu</option>
                                    <option value="number">Sayı</option>
                                    <option value="date">Tarih</option>
                                    <option value="url">URL</option>
                                </select>
                                <select id="cf-applies-to" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                                    <option value="content">İçerik</option>
                                    <option value="ad">Reklam</option>
                                    <option value="user">Kullanıcı</option>
                                </select>
                                <input type="number" id="cf-sort-order" placeholder="Sıra" value="0" style="padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                            </div>
                            <div style="margin-bottom:10px;">
                                <input type="text" id="cf-options" placeholder='Seçenekler (JSON array, sadece select tipi için): ["Hava","Kara","Deniz"]' style="width:100%; padding:8px; border-radius:6px; border:1px solid var(--border-subtle); background:var(--bg-card); color:#fff; font-size:12px;">
                            </div>
                            <label style="font-size:11px; color:var(--text-muted); margin-bottom:10px; display:block;"><input type="checkbox" id="cf-required"> Zorunlu alan</label>
                            <button type="button" class="btn-cmd btn-cmd-primary" onclick="window.MSIAdmin.saveCustomField()"><i class="fa-solid fa-save"></i> Kaydet</button>
                            <button type="button" class="btn-cmd btn-cmd-sm" onclick="document.getElementById('custom-field-form-container').style.display='none'">İptal</button>
                        </div>
                        <div class="table-responsive">
                            <table class="ops-table">
                                <thead><tr><th>Anahtar</th><th>Etiket (TR)</th><th>Etiket (EN)</th><th>Tür</th><th>Hedef</th><th>Zorunlu</th><th>Sıra</th><th>İşlem</th></tr></thead>
                                <tbody id="tbody-custom-fields"></tbody>
                            </table>
                        </div>
                    </div>
                    <div class="panel-card">
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-broom"></i> Önbellek Yönetimi</div>
                            <button type="button" class="btn-cmd btn-cmd-sm" style="border-color:#ef4444; color:#ef4444;" onclick="window.MSIAdmin.flushAllCache()"><i class="fa-solid fa-trash-can"></i> Tümünü Temizle</button>
                        </div>
                        <div style="margin-bottom:16px; padding:16px; background:rgba(255,255,255,0.03); border-radius:8px;">
                            <div style="font-size:12px; color:var(--text-muted); margin-bottom:8px;">Mevcut önbellek katmanları:</div>
                            <div style="display:grid; gap:6px; font-size:12px;">
                                <div style="display:flex; justify-content:space-between; padding:6px 10px; background:rgba(52,211,153,0.1); border-radius:4px;"><span>L1: In-Memory Translation Cache</span><span style="color:#34d399;">Aktif</span></div>
                                <div style="display:flex; justify-content:space-between; padding:6px 10px; background:rgba(56,189,248,0.1); border-radius:4px;"><span>L2: Content Query Cache</span><span style="color:#38bdf8;">Aktif</span></div>
                                <div style="display:flex; justify-content:space-between; padding:6px 10px; background:rgba(251,191,36,0.1); border-radius:4px;"><span>L3: SEO Redirect Cache</span><span style="color:#fbbf24;">Aktif</span></div>
                            </div>
                        </div>
                        <div class="panel-header">
                            <div class="panel-title"><i class="fa-solid fa-clock-rotate-left"></i> Son Önbellek İşlemleri</div>
                        </div>
                        <div class="table-responsive" style="max-height:300px; overflow-y:auto;">
                            <table class="ops-table">
                                <thead><tr><th>Zaman</th><th>Anahtar</th><th>İşlem</th><th>Kullanıcı</th><th>Detay</th></tr></thead>
                                <tbody id="tbody-cache-operations"></tbody>
                            </table>
                        </div>
                    </div>
                </div>
            </div>
        </section>
    </div>

    <div id="ops-toast" role="status" aria-live="polite"><i class="fa-solid fa-circle-check" style="color:var(--msi-emerald);"></i> <span id="ops-toast-msg">İşlem tamamlandı.</span></div>

    <script src="https://cdn.jsdelivr.net/npm/chart.js@4.4.7/dist/chart.umd.min.js"></script>
    <script src="/js/msi-admin.js?v=2" defer></script>
</body>
</html>"#####.to_string()
}
