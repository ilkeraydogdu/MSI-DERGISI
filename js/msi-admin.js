window.MSIAdmin = (function () {
  const state = {
    contents: [],
    ads: [],
    contentAnalytics: [],
    adAnalytics: [],
    readerAnalytics: [],
    corporateRequests: [],
    categories: [],
    ssbTags: [],
    newsletters: [],
    reminders: [],
    redirects: [],
    mediaLibrary: [],
    auditLogs: [],
    userRoles: [],
    customFields: [],
    cacheOperations: [],
    systemHealth: null,
    bulkSelected: new Set(),
  };

  function esc(v) {
    return String(v ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function showToast(msg, isError) {
    const el = document.getElementById("ops-toast");
    const txt = document.getElementById("ops-toast-msg");
    if (!el || !txt) return;
    el.style.borderLeftColor = isError ? "var(--msi-crimson)" : "var(--text-main)";
    txt.textContent = msg;
    el.style.display = "flex";
    clearTimeout(window.__msiToastTimer);
    window.__msiToastTimer = setTimeout(() => {
      el.style.display = "none";
    }, 4200);
  }

  function unwrapList(payload, keyHint) {
    if (!payload) return [];
    if (Array.isArray(payload)) return payload;
    if (keyHint && Array.isArray(payload[keyHint])) return payload[keyHint];
    if (payload.data) {
      if (Array.isArray(payload.data)) return payload.data;
      if (keyHint && Array.isArray(payload.data[keyHint]))
        return payload.data[keyHint];
    }
    if (Array.isArray(payload.items)) return payload.items;
    return [];
  }

  async function safeFetchJson(url, options) {
    try {
      const res = await fetch(url, options);
      if (!res.ok) return null;
      return await res.json();
    } catch (err) {
      return null;
    }
  }

  function switchTab(tabId) {
    document.querySelectorAll(".tab-btn").forEach((btn) => {
      btn.classList.toggle("active", btn.getAttribute("data-tab") === tabId);
    });
    document.querySelectorAll(".tab-pane").forEach((pane) => {
      pane.classList.toggle("active", pane.id === tabId);
    });
  }

  function switchAnalyticsSubview(view) {
    ["content", "ad", "reader"].forEach((v) => {
      const btn = document.getElementById("btn-subview-" + v);
      const pane = document.getElementById("subview-" + v);
      if (btn) btn.classList.toggle("btn-cmd-primary", v === view);
      if (pane) pane.classList.toggle("active", v === view);
    });
  }

  function toggleSponsoredFields() {
    const chk = document.getElementById("c-is-sponsored");
    const row = document.getElementById("c-sponsor-extra-row");
    if (chk && row) {
      row.style.display = chk.checked ? "grid" : "none";
    }
  }

  async function refreshAllData() {
    // Fetch from /api/v1/admin/* with fallback to /api/v1/*
    const [
      overviewRes,
      contentsRes,
      adsRes,
      analyticsRes,
      corpReqRes,
      catsRes,
      ssbRes,
      nlRes,
      remRes,
      seoRes,
    ] = await Promise.all([
      safeFetchJson("/api/v1/admin/overview"),
      safeFetchJson("/api/v1/admin/contents") ||
        safeFetchJson("/api/v1/contents"),
      safeFetchJson("/api/v1/admin/ads") || safeFetchJson("/api/v1/ads"),
      safeFetchJson("/api/v1/admin/analytics"),
      safeFetchJson("/api/v1/admin/corporate-requests"),
      safeFetchJson("/api/v1/admin/categories") ||
        safeFetchJson("/api/v1/categories"),
      safeFetchJson("/api/v1/admin/ssb-tags") ||
        safeFetchJson("/api/v1/ssb-tags"),
      safeFetchJson("/api/v1/admin/newsletters"),
      safeFetchJson("/api/v1/admin/reminders"),
      safeFetchJson("/api/v1/admin/redirects"),
    ]);

    if (overviewRes) {
      const d = overviewRes.data || overviewRes;
      if (d.contents) state.contents = d.contents;
      if (d.ads) state.ads = d.ads;
      if (d.content_analytics) state.contentAnalytics = d.content_analytics;
      if (d.ad_analytics) state.adAnalytics = d.ad_analytics;
      if (d.reader_analytics) state.readerAnalytics = d.reader_analytics;
      if (d.corporate_requests) state.corporateRequests = d.corporate_requests;
      if (d.categories) state.categories = d.categories;
      if (d.ssb_tags) state.ssbTags = d.ssb_tags;
      if (d.newsletters) state.newsletters = d.newsletters;
      if (d.reminders) state.reminders = d.reminders;
      if (d.redirects) state.redirects = d.redirects;
    }

    if (contentsRes) {
      const list = unwrapList(contentsRes, "contents");
      if (list.length) state.contents = list;
    } else if (!state.contents.length) {
      const pubContents = await safeFetchJson("/api/v1/contents");
      if (pubContents) state.contents = unwrapList(pubContents, "contents");
    }

    if (adsRes) {
      const list = unwrapList(adsRes, "ads");
      if (list.length) state.ads = list;
    } else if (!state.ads.length) {
      const pubAds = await safeFetchJson("/api/v1/ads");
      if (pubAds) state.ads = unwrapList(pubAds, "ads");
    }

    if (analyticsRes) {
      const d = analyticsRes.data || analyticsRes;
      if (Array.isArray(d.contents)) state.contentAnalytics = d.contents;
      if (Array.isArray(d.content_reports))
        state.contentAnalytics = d.content_reports;
      if (Array.isArray(d.content_analytics))
        state.contentAnalytics = d.content_analytics;
      if (Array.isArray(d.ads)) state.adAnalytics = d.ads;
      if (Array.isArray(d.ad_reports)) state.adAnalytics = d.ad_reports;
      if (Array.isArray(d.ad_analytics)) state.adAnalytics = d.ad_analytics;
      if (Array.isArray(d.readers)) state.readerAnalytics = d.readers;
      if (Array.isArray(d.reader_reports))
        state.readerAnalytics = d.reader_reports;
      if (Array.isArray(d.reader_analytics))
        state.readerAnalytics = d.reader_analytics;
    }

    if (!state.contentAnalytics.length) {
      const cRep = await safeFetchJson("/api/v1/admin/analytics/contents");
      if (cRep) state.contentAnalytics = unwrapList(cRep, "reports");
    }
    if (!state.adAnalytics.length) {
      const aRep = await safeFetchJson("/api/v1/admin/analytics/ads");
      if (aRep) state.adAnalytics = unwrapList(aRep, "reports");
    }
    if (!state.readerAnalytics.length) {
      const rRep = await safeFetchJson("/api/v1/admin/analytics/readers");
      if (rRep) state.readerAnalytics = unwrapList(rRep, "reports");
    }

    if (corpReqRes) {
      state.corporateRequests = unwrapList(corpReqRes, "requests");
    }
    if (catsRes) {
      state.categories = unwrapList(catsRes, "categories");
    } else if (!state.categories.length) {
      const pubCats = await safeFetchJson("/api/v1/categories");
      if (pubCats) state.categories = unwrapList(pubCats, "categories");
    }
    if (ssbRes) {
      state.ssbTags = unwrapList(ssbRes, "ssb_tags");
    } else if (!state.ssbTags.length) {
      const pubSsb = await safeFetchJson("/api/v1/ssb-tags");
      if (pubSsb) state.ssbTags = unwrapList(pubSsb, "ssb_tags");
    }
    if (nlRes) state.newsletters = unwrapList(nlRes, "campaigns");
    if (remRes) state.reminders = unwrapList(remRes, "reminders");
    if (seoRes) state.redirects = unwrapList(seoRes, "redirects");

    renderKpis();
    renderContentsTable();
    renderAdsTable();
    renderAnalyticsTables();
    renderCorporateRequestsTable();
    renderTaxonomyTables();
    renderNewslettersTable();
    renderRemindersAndRedirects();
  }

  function renderKpis() {
    const totalContents = state.contents.length;
    const totalPlacements = state.contents.reduce(
      (acc, c) => acc + ((c.placements && c.placements.length) || 1),
      0,
    );
    document.getElementById("kpi-total-contents").textContent =
      totalContents + " İçerik";
    document.getElementById("kpi-placements-sub").textContent =
      totalPlacements + " çoklu bölüm gösterimi (Single Source of Truth)";

    const totalViews = state.contents.reduce(
      (acc, c) => acc + (c.view_count || 0),
      0,
    );
    const totalTrue = state.contents.reduce(
      (acc, c) => acc + (c.true_read_count || 0),
      0,
    );
    const ratio =
      totalViews > 0 ? Math.round((totalTrue / totalViews) * 100) : 0;
    document.getElementById("kpi-true-reads").textContent =
      totalTrue.toLocaleString("tr-TR") + " (" + ratio + "%)";
    document.getElementById("kpi-views-sub").textContent =
      "Toplam " + totalViews.toLocaleString("tr-TR") + " sayfa görüntülenme";

    const activeAds = state.ads.filter((a) => a.is_active !== false).length;
    const totalImps = state.ads.reduce(
      (acc, a) => acc + (a.impressions_count || 0),
      0,
    );
    document.getElementById("kpi-active-ads").textContent =
      activeAds + " Kampanya";
    document.getElementById("kpi-ads-sub").textContent =
      totalImps.toLocaleString("tr-TR") + " kurumsal hedefli gösterim";

    const pendingReqs = state.corporateRequests.filter(
      (r) => r.status === "pending",
    ).length;
    document.getElementById("kpi-gov-requests").textContent =
      pendingReqs + " Bekleyen";
    document.getElementById("kpi-gov-sub").textContent =
      "Toplam " + state.corporateRequests.length + " kurumsal doğrulama kaydı";
  }

  function renderContentsTable() {
    const tbody = document.getElementById("tbody-contents");
    if (!tbody) return;
    const q = (document.getElementById("filter-content-search")?.value || "")
      .toLowerCase()
      .trim();
    const filtered = state.contents.filter((c) => {
      if (!q) return true;
      return (
        (c.title_tr || "").toLowerCase().includes(q) ||
        (c.title_en || "").toLowerCase().includes(q) ||
        (c.content_type || "").toLowerCase().includes(q) ||
        (c.placements || []).join(" ").toLowerCase().includes(q) ||
        (c.ssb_tags || []).join(" ").toLowerCase().includes(q)
      );
    });

    if (!filtered.length) {
      tbody.innerHTML =
        '<tr><td colspan="7" style="text-align:center; padding:24px; color:var(--text-muted);">Kriterlere uygun içerik bulunamadı.</td></tr>';
      return;
    }

    tbody.innerHTML = filtered
      .map((c) => {
        const placementsHtml = (c.placements || [])
          .map((p) => `<span class="badge-tag badge-placement">${p}</span>`)
          .join("");
        const ssbHtml = (c.ssb_tags || [])
          .map((t) => `<span class="badge-tag badge-ssb">${t}</span>`)
          .join("");
        const pwLabels = {
          0: "0: Açık Erişim",
          1: "1: Kayıtlı & Abone",
          2: `2: Abone / ${c.single_price_try || 0} ₺`,
        };
        const pwBadge = `<span class="badge-tag badge-paywall-${c.visibility_level || 0}">${pwLabels[c.visibility_level || 0]}</span>`;
        const spBadge = c.is_sponsored
          ? `<div style="margin-top:4px;"><span class="badge-tag badge-sponsored"><i class="fa-solid fa-shield-halved"></i> ${c.sponsor_legal_label || "SPONSORLU"}: ${c.sponsor_brand || ""}</span></div>`
          : "";
        return `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px;">
                            <strong>#${c.id}</strong><br>
                            <span style="color:var(--text-muted);">${c.issue_number ? "Sayı " + c.issue_number : "Online"}</span>
                        </td>
                        <td>
                            <div style="font-weight:700; color:#ffffff; margin-bottom:2px;">
                                ${c.is_featured ? '<i class="fa-solid fa-star" style="color:#fbbf24;" title="Manşet"></i> ' : ""}
                                <a href="/icerik/${c.slug}" style="color:#ffffff; text-decoration:none;">${c.title_tr}</a>
                            </div>
                            <div style="font-size:11.5px; color:var(--text-muted); margin-bottom:4px;">${c.title_en || ""}</div>
                            <span class="badge-tag">${c.content_type}</span>
                            <span class="badge-tag" style="background:rgba(255,255,255,0.06); color:#cbd5e1; border-color:transparent;">${c.source_group}</span>
                        </td>
                        <td>${placementsHtml || '<span style="color:var(--text-muted);">—</span>'}</td>
                        <td>${ssbHtml || '<span style="color:var(--text-muted);">—</span>'}</td>
                        <td>${pwBadge}${spBadge}</td>
                        <td style="font-family:var(--font-mono);">
                            <div style="color:#ffffff; font-weight:700;">${(c.view_count || 0).toLocaleString("tr-TR")} görüntülenme</div>
                            <div style="color:#34d399; font-size:11.5px;">${(c.true_read_count || 0).toLocaleString("tr-TR")} gerçek okuma</div>
                        </td>
                        <td>
                            <div style="display:flex; gap:6px; flex-wrap:wrap;">
                                <button type="button" class="btn-cmd" style="padding:5px 9px; font-size:11px;" onclick="window.MSIAdmin.loadContentIntoForm(${c.id})">
                                    <i class="fa-solid fa-pen"></i> Düzenle
                                </button>
                                <button type="button" class="btn-cmd" style="padding:5px 9px; font-size:11px; color:#fca5a5;" onclick="window.MSIAdmin.deleteContent(${c.id})">
                                    <i class="fa-solid fa-trash"></i>
                                </button>
                            </div>
                        </td>
                    </tr>`;
      })
      .join("");
  }

  function loadContentIntoForm(id) {
    const c = state.contents.find((x) => x.id === id);
    if (!c) return;
    document.getElementById("c-id").value = c.id;
    document.getElementById("c-slug").value = c.slug || "";
    document.getElementById("c-title-tr").value = c.title_tr || "";
    document.getElementById("c-title-en").value = c.title_en || "";
    document.getElementById("c-summary-tr").value = c.summary_tr || "";
    document.getElementById("c-summary-en").value = c.summary_en || "";
    document.getElementById("c-body-tr").value = c.body_tr || "";
    document.getElementById("c-body-en").value = c.body_en || "";
    document.getElementById("c-content-type").value =
      c.content_type || "haberler";
    document.getElementById("c-source-group").value =
      c.source_group || "gunluk-haber";
    document.getElementById("c-issue-number").value = c.issue_number || "";
    document.getElementById("c-visibility-level").value = String(
      c.visibility_level || 0,
    );
    document.getElementById("c-single-price").value = c.single_price_try || 0;
    document.getElementById("c-read-time").value = c.read_time_min || 5;
    document.getElementById("c-author-name").value =
      c.author_name || "MSI Editöryal Masası";
    document.getElementById("c-cover-image").value =
      c.cover_image || "images/trending/trending1.jpg";
    document.getElementById("c-video-url").value = c.video_url || "";
    document.getElementById("c-is-sponsored").checked = !!c.is_sponsored;
    document.getElementById("c-is-featured").checked = !!c.is_featured;
    document.getElementById("c-sponsor-brand").value = c.sponsor_brand || "";
    document.getElementById("c-sponsor-legal-label").value =
      c.sponsor_legal_label || "SPONSORLU İÇERİK";
    toggleSponsoredFields();

    const pSet = new Set(c.placements || []);
    document.querySelectorAll('input[name="c_placement"]').forEach((chk) => {
      chk.checked = pSet.has(chk.value);
    });
    const sSet = new Set(c.ssb_tags || []);
    document.querySelectorAll('input[name="c_ssb"]').forEach((chk) => {
      chk.checked = sSet.has(chk.value);
    });

    document.getElementById("content-form-heading").textContent =
      "İçerik Düzenle (#" + c.id + " — Single Source of Truth)";
    showToast("İçerik #" + c.id + " düzenleme formuna yüklendi.");
  }

  function resetContentForm() {
    document.getElementById("form-content-ssot").reset();
    document.getElementById("c-id").value = "0";
    document.getElementById("content-form-heading").textContent =
      "Hızlı İçerik Girişi (Single Source of Truth)";
    toggleSponsoredFields();
  }

  async function submitContentForm(e) {
    e.preventDefault();
    const placements = Array.from(
      document.querySelectorAll('input[name="c_placement"]:checked'),
    ).map((el) => el.value);
    const ssb_tags = Array.from(
      document.querySelectorAll('input[name="c_ssb"]:checked'),
    ).map((el) => el.value);
    const isSponsored = document.getElementById("c-is-sponsored").checked;
    const issueRaw = document.getElementById("c-issue-number").value;

    const editId = parseInt(document.getElementById("c-id").value || "0", 10);
    const existingItem =
      editId > 0 ? state.contents.find((x) => x.id === editId) : null;

    const payload = {
      id: editId,
      slug: document.getElementById("c-slug").value.trim(),
      title_tr: document.getElementById("c-title-tr").value.trim(),
      title_en:
        document.getElementById("c-title-en").value.trim() ||
        document.getElementById("c-title-tr").value.trim(),
      summary_tr: document.getElementById("c-summary-tr").value.trim(),
      summary_en:
        document.getElementById("c-summary-en").value.trim() ||
        document.getElementById("c-summary-tr").value.trim(),
      body_tr: document.getElementById("c-body-tr").value.trim(),
      body_en:
        document.getElementById("c-body-en").value.trim() ||
        document.getElementById("c-body-tr").value.trim(),
      content_type: document.getElementById("c-content-type").value,
      source_group: document.getElementById("c-source-group").value,
      cover_image:
        document.getElementById("c-cover-image").value.trim() ||
        "images/msi/dergi-286.webp",
      author_name:
        document.getElementById("c-author-name").value.trim() ||
        "MSI Editöryal Masası",
      author_title: existingItem
        ? existingItem.author_title || "Savunma Sanayii Analisti"
        : "Savunma Sanayii Analisti",
      issue_number: issueRaw ? parseInt(issueRaw, 10) : null,
      published_at: existingItem
        ? existingItem.published_at || new Date().toISOString().slice(0, 10)
        : new Date().toISOString().slice(0, 10),
      read_time_min: parseInt(
        document.getElementById("c-read-time").value || "5",
        10,
      ),
      visibility_level: parseInt(
        document.getElementById("c-visibility-level").value || "0",
        10,
      ),
      single_price_try: parseFloat(
        document.getElementById("c-single-price").value || "0",
      ),
      is_sponsored: isSponsored,
      sponsor_brand: isSponsored
        ? document.getElementById("c-sponsor-brand").value.trim() ||
          "Savunma Sanayii Paydaşı"
        : null,
      sponsor_legal_label: isSponsored
        ? document.getElementById("c-sponsor-legal-label").value
        : null,
      is_featured: document.getElementById("c-is-featured").checked,
      video_url: document.getElementById("c-video-url").value.trim() || null,
      view_count: existingItem ? existingItem.view_count || 1 : 1,
      true_read_count: existingItem ? existingItem.true_read_count || 1 : 1,
      placements: placements,
      ssb_tags: ssb_tags,
    };

    const res = await safeFetchJson("/api/v1/admin/contents", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (res) {
      showToast(
        "İçerik başarıyla kaydedildi ve " +
          placements.length +
          " bölümde yayına alındı!",
      );
      resetContentForm();
      await refreshAllData();
    } else {
      showToast("İçerik kaydedilirken sunucu yanıt vermedi.", true);
    }
    return false;
  }

  async function deleteContent(id) {
    const res = await safeFetchJson("/api/v1/admin/contents/" + id, {
      method: "DELETE",
    });
    if (res) {
      showToast("İçerik #" + id + " silindi.");
      await refreshAllData();
    }
  }

  async function checkAdConflictLive() {
    const slot_code = document.getElementById("ad-slot-code").value;
    const target_institutions = Array.from(
      document.querySelectorAll('input[name="ad_inst"]:checked'),
    ).map((el) => el.value);
    const target_ssb_tags = Array.from(
      document.querySelectorAll('input[name="ad_ssb"]:checked'),
    ).map((el) => el.value);
    const sponsor_name =
      document.getElementById("ad-sponsor-name").value.trim() ||
      "Yeni Kampanya";

    const res = await safeFetchJson("/api/v1/admin/ads/check-conflict", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        slot_code,
        sponsor_name,
        target_institutions,
        target_ssb_tags,
      }),
    });

    const banner = document.getElementById("ad-conflict-banner");
    const textEl = document.getElementById("ad-conflict-text");
    if (!banner || !textEl) return;

    const warning = (res && (res.data || res)) || null;
    if (warning && typeof warning.has_conflict === "boolean") {
      if (warning.has_conflict) {
        banner.classList.add("conflict-danger");
        textEl.innerHTML = `<strong>⚠️ REKLAM KOTA / ÇAKIŞMA UYARISI (${warning.current_count}/${warning.quota_limit}):</strong> ${warning.message_tr}<br>
                        <span style="font-size:11.5px;">Çakışan Kampanyalar: ${(warning.conflicting_campaigns || []).join(", ")}</span>`;
      } else {
        banner.classList.remove("conflict-danger");
        textEl.innerHTML = `<strong>✅ ÇAKIŞMA YOK (${warning.current_count}/${warning.quota_limit}):</strong> ${warning.message_tr || "Seçilen slot ve SSB etiketlerinde kota müsait."}`;
      }
    }
  }

  async function submitAdForm(e) {
    e.preventDefault();
    const target_institutions = Array.from(
      document.querySelectorAll('input[name="ad_inst"]:checked'),
    ).map((el) => el.value);
    const target_ssb_tags = Array.from(
      document.querySelectorAll('input[name="ad_ssb"]:checked'),
    ).map((el) => el.value);
    const target_categories = (
      document.getElementById("ad-target-categories").value || ""
    )
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);

    const payload = {
      id: parseInt(document.getElementById("ad-id").value || "0", 10),
      slot_code: document.getElementById("ad-slot-code").value,
      badge_label:
        document.getElementById("ad-badge-label").value.trim() ||
        "SPONSORLU REKLAM ALANI",
      dimensions:
        document.getElementById("ad-dimensions").value.trim() || "970x90",
      sponsor_name: document.getElementById("ad-sponsor-name").value.trim(),
      sponsor_type: document.getElementById("ad-sponsor-type").value,
      pill_style: document.getElementById("ad-pill-style").value,
      title_tr: document.getElementById("ad-title-tr").value.trim(),
      title_en:
        document.getElementById("ad-title-en").value.trim() ||
        document.getElementById("ad-title-tr").value.trim(),
      desc_tr: document.getElementById("ad-desc-tr").value.trim(),
      desc_en:
        document.getElementById("ad-desc-en").value.trim() ||
        document.getElementById("ad-desc-tr").value.trim(),
      target_url: document.getElementById("ad-target-url").value.trim(),
      image_url: null,
      budget_score: parseInt(
        document.getElementById("ad-budget-score").value || "80",
        10,
      ),
      is_default_fallback: document.getElementById("ad-is-fallback").checked,
      is_floating_popup: document.getElementById("ad-is-popup").checked,
      is_newsletter_ad: document.getElementById("ad-is-newsletter").checked,
      target_categories,
      target_ssb_tags,
      target_institutions,
      impressions_count: 1,
      clicks_count: 0,
      is_active: document.getElementById("ad-is-active").checked,
    };

    const res = await safeFetchJson("/api/v1/admin/ads", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (res) {
      showToast("Reklam kampanyası kaydedildi ve hedefleme motoruna eklendi!");
      await checkAdConflictLive();
      await refreshAllData();
    } else {
      showToast("Reklam kampanyası kaydedilirken hata oluştu.", true);
    }
    return false;
  }

  function renderAdsTable() {
    const tbody = document.getElementById("tbody-ads");
    if (!tbody) return;
    if (!state.ads.length) {
      tbody.innerHTML =
        '<tr><td colspan="6" style="text-align:center; padding:20px;">Kayıtlı reklam kampanyası bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = state.ads
      .map((a) => {
        const instBadges = (a.target_institutions || [])
          .map((i) => `<span class="badge-tag">${i}</span>`)
          .join("");
        const ssbBadges = (a.target_ssb_tags || [])
          .map((t) => `<span class="badge-tag badge-ssb">${t}</span>`)
          .join("");
        const imps = a.impressions_count || 0;
        const clicks = a.clicks_count || 0;
        const ctr = imps > 0 ? ((clicks / imps) * 100).toFixed(2) : "0.00";
        return `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px;">
                            <strong>${a.slot_code}</strong><br>
                            <span style="color:var(--text-muted);">${a.dimensions}</span>
                        </td>
                        <td>
                            <div style="font-weight:700; color:#ffffff;">${a.sponsor_name}</div>
                            <span class="badge-tag ${a.sponsor_type === "defense_oem" ? "badge-sponsored" : "badge-paywall-0"}">${a.sponsor_type}</span>
                        </td>
                        <td>
                            <div style="font-weight:600; color:#ffffff;">${a.title_tr}</div>
                            <div style="font-size:11px; color:var(--text-muted);">Bütçe Puanı: <strong>${a.budget_score}</strong> ${a.is_default_fallback ? "• [Fallback]" : ""}</div>
                        </td>
                        <td>${instBadges || '<span style="color:var(--text-muted);">Tüm Kurumlar</span>'}</td>
                        <td>${ssbBadges || '<span style="color:var(--text-muted);">Genel</span>'}</td>
                        <td style="font-family:var(--font-mono);">
                            <div>${imps.toLocaleString("tr-TR")} gösterim</div>
                            <div style="color:#38bdf8;">${clicks.toLocaleString("tr-TR")} tık (${ctr}%)</div>
                        </td>
                    </tr>`;
      })
      .join("");
  }

  function renderInstitutionBars(list, isCyan) {
    if (!Array.isArray(list) || !list.length) {
      return '<span style="color:var(--text-muted); font-size:11.5px;">Veri toplanıyor</span>';
    }
    return list
      .map((item) => {
        const pct =
          typeof item.percentage === "number"
            ? item.percentage.toFixed(1)
            : "0.0";
        return `<div class="inst-bar-row">
                        <div class="inst-bar-meta">
                            <span><strong>${item.organization_category}</strong></span>
                            <span>${item.count} (${pct}%)</span>
                        </div>
                        <div class="inst-bar-track">
                            <div class="inst-bar-fill ${isCyan ? "cyan" : ""}" style="width:${Math.min(100, Math.max(4, parseFloat(pct)))}%;"></div>
                        </div>
                    </div>`;
      })
      .join("");
  }

  function renderAnalyticsTables() {
    // Sub-view A: Content Analytics
    const tbodyContent = document.getElementById("tbody-analytics-content");
    if (tbodyContent) {
      if (state.contentAnalytics.length) {
        tbodyContent.innerHTML = state.contentAnalytics
          .map(
            (r) => `<tr>
                            <td>
                                <div style="font-weight:700; color:#ffffff;">${r.title_tr}</div>
                                <div style="font-family:var(--font-mono); font-size:11px; color:var(--text-muted);">/${r.content_slug}</div>
                            </td>
                            <td style="font-family:var(--font-mono); font-size:14px; font-weight:700;">${(r.total_views || 0).toLocaleString("tr-TR")}</td>
                            <td style="font-family:var(--font-mono); font-size:14px; font-weight:700; color:#34d399;">${(r.true_reads || 0).toLocaleString("tr-TR")}</td>
                            <td style="font-family:var(--font-mono);">
                                <div><i class="fa-regular fa-clock"></i> ${Math.round(r.avg_duration_sec || 0)} sn</div>
                                <div style="color:#38bdf8;"><i class="fa-solid fa-arrows-up-down"></i> %${Math.round(r.avg_scroll_pct || 0)} scroll</div>
                            </td>
                            <td>${renderInstitutionBars(r.institution_breakdown, false)}</td>
                        </tr>`,
          )
          .join("");
      } else {
        tbodyContent.innerHTML =
          '<tr><td colspan="5" style="text-align:center; padding:20px;">İçerik analitik raporu bulunamadı.</td></tr>';
      }
    }

    // Sub-view B: Ad Analytics
    const tbodyAd = document.getElementById("tbody-analytics-ad");
    if (tbodyAd) {
      if (state.adAnalytics.length) {
        tbodyAd.innerHTML = state.adAnalytics
          .map(
            (a) => `<tr>
                            <td>
                                <div style="font-weight:700; color:#ffffff;">${a.sponsor_name}</div>
                                <span class="badge-tag">${a.slot_code}</span>
                            </td>
                            <td style="font-family:var(--font-mono);">
                                <div><strong>${(a.impressions || 0).toLocaleString("tr-TR")}</strong> gösterim</div>
                                <div style="color:#34d399;"><strong>${(a.clicks || 0).toLocaleString("tr-TR")}</strong> tıklama</div>
                                <div style="color:#fbbf24;">CTR: %${(a.ctr_pct || 0).toFixed(2)}</div>
                            </td>
                            <td>${renderInstitutionBars(a.impression_institutions, false)}</td>
                            <td>${renderInstitutionBars(a.click_institutions, true)}</td>
                        </tr>`,
          )
          .join("");
      } else {
        tbodyAd.innerHTML =
          '<tr><td colspan="4" style="text-align:center; padding:20px;">Reklam analitik raporu bulunamadı.</td></tr>';
      }
    }

    // Sub-view C: Reader Analytics
    const tbodyReader = document.getElementById("tbody-analytics-reader");
    if (tbodyReader) {
      if (state.readerAnalytics.length) {
        tbodyReader.innerHTML = state.readerAnalytics
          .map(
            (u) => `<tr>
                            <td>
                                <div style="font-weight:700; color:#ffffff;">${u.full_name}</div>
                                <div style="font-size:11.5px; color:var(--text-muted);">${u.email}</div>
                            </td>
                            <td>
                                <span class="badge-tag">${u.organization_category}</span>
                                <div style="font-size:11.5px; color:var(--text-secondary); margin-top:3px;">${u.organization_name}</div>
                            </td>
                            <td><span class="badge-tag badge-paywall-0">${u.subscription_tier}</span></td>
                            <td>${(u.articles_read || []).map((slug) => `<span class="badge-tag badge-placement">${slug}</span>`).join("")}</td>
                            <td>${(u.ssb_keywords_engaged || []).map((k) => `<span class="badge-tag badge-ssb">${k}</span>`).join("")}</td>
                            <td style="font-family:var(--font-mono); font-size:11.5px;">
                                <div>${u.ad_impressions_count || 0} gösterim / <strong>${u.ad_clicks_count || 0} tık</strong></div>
                                <div style="margin-top:3px;">${(u.clicked_ads || []).map((ad) => `<span class="badge-tag badge-sponsored">${ad}</span>`).join("")}</div>
                            </td>
                        </tr>`,
          )
          .join("");
      } else {
        tbodyReader.innerHTML =
          '<tr><td colspan="6" style="text-align:center; padding:20px;">Okuyucu analitik raporu bulunamadı.</td></tr>';
      }
    }
  }

  async function downloadMonthlyConsolidatedReport() {
    const reportPayload = {
      generated_at: new Date().toISOString(),
      platform:
        "UMSA Prodüksiyon — MSI Dergisi & MSI TDR B2B Kurumsal Analitik Merkezi",
      summary: {
        total_contents: state.contents.length,
        total_ads: state.ads.length,
        total_readers_tracked: state.readerAnalytics.length,
      },
      content_analytics: state.contentAnalytics,
      ad_analytics: state.adAnalytics,
      reader_analytics: state.readerAnalytics,
    };
    const blob = new Blob([JSON.stringify(reportPayload, null, 2)], {
      type: "application/json;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download =
      "MSI_Aylik_Konsolide_Kurumsal_Analitik_Raporu_" +
      new Date().toISOString().slice(0, 10) +
      ".json";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast(
      "Ay Sonu Otomatik Konsolide B2B Analitik Raporu üretildi ve indirildi!",
    );
  }

  function renderCorporateRequestsTable() {
    const tbody = document.getElementById("tbody-corporate-requests");
    if (!tbody) return;
    if (!state.corporateRequests.length) {
      tbody.innerHTML =
        '<tr><td colspan="6" style="text-align:center; padding:20px;">Kurumsal doğrulama talebi bulunmuyor.</td></tr>';
      return;
    }
    tbody.innerHTML = state.corporateRequests
      .map((req) => {
        const statusBadge =
          req.status === "approved"
            ? '<span class="badge-tag badge-paywall-0"><i class="fa-solid fa-check"></i> Onaylandı (Kamu/Kurum Abonesi)</span>'
            : req.status === "rejected"
              ? '<span class="badge-tag badge-sponsored"><i class="fa-solid fa-xmark"></i> Reddedildi</span>'
              : '<span class="badge-tag badge-paywall-2"><i class="fa-regular fa-hourglass-half"></i> Onay Bekliyor</span>';
        return `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px;">
                            <strong>#${req.id}</strong><br>
                            <span style="color:var(--text-muted);">${req.created_at || ""}</span>
                        </td>
                        <td>
                            <div style="font-weight:700; color:#ffffff;">${req.full_name}</div>
                            <div style="font-size:11.5px; color:#38bdf8;">${req.email}</div>
                        </td>
                        <td>
                            <span class="badge-tag">${req.organization_category}</span>
                            <div style="font-weight:600; margin-top:3px;">${req.organization_name}</div>
                        </td>
                        <td style="max-width:340px; color:var(--text-secondary);">${req.request_message}</td>
                        <td>${statusBadge}</td>
                        <td>
                            <div style="display:flex; gap:6px;">
                                <button type="button" class="btn-cmd btn-cmd-emerald" style="padding:6px 10px; font-size:11px;" onclick="window.MSIAdmin.decideCorporateRequest(${req.id}, 'approved')">
                                    <i class="fa-solid fa-check"></i> Onayla (Ücretsiz Abonelik)
                                </button>
                                <button type="button" class="btn-cmd" style="padding:6px 10px; font-size:11px; color:#fca5a5;" onclick="window.MSIAdmin.decideCorporateRequest(${req.id}, 'rejected')">
                                    <i class="fa-solid fa-xmark"></i> Reddet
                                </button>
                            </div>
                        </td>
                    </tr>`;
      })
      .join("");
  }

  async function decideCorporateRequest(id, decision) {
    const res =
      (await safeFetchJson(
        "/api/v1/admin/corporate-requests/" + id + "/decision",
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            id,
            status: decision,
            reviewed_by: "operator@msidergisi.com",
          }),
        },
      )) ||
      (await safeFetchJson("/api/v1/admin/corporate-requests", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id,
          status: decision,
          reviewed_by: "operator@msidergisi.com",
        }),
      }));

    if (res) {
      showToast(
        decision === "approved"
          ? "Talep #" +
              id +
              " onaylandı: Kullanıcıya ücretsiz Kurumsal/Kamu E-Dergi aboneliği tanımlandı!"
          : "Talep #" + id + " reddedildi.",
      );
      await refreshAllData();
    } else {
      const item = state.corporateRequests.find((r) => r.id === id);
      if (item) item.status = decision;
      renderCorporateRequestsTable();
      renderKpis();
      showToast("Talep #" + id + " durumu güncellendi: " + decision);
    }
  }

  function renderTaxonomyTables() {
    const tbodyCats = document.getElementById("tbody-categories");
    if (tbodyCats) {
      tbodyCats.innerHTML = state.categories
        .map(
          (c) => `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px; color:#38bdf8;">/${c.slug}</td>
                        <td>
                            <div style="font-weight:700; color:#ffffff;">${c.name_tr}</div>
                            <div style="font-size:11px; color:var(--text-muted);">${c.name_en}</div>
                        </td>
                        <td><span class="badge-tag">${c.group_type}</span></td>
                        <td>${c.is_dynamic ? '<span class="badge-tag badge-paywall-0">Dinamik</span>' : '<span class="badge-tag">Çekirdek</span>'}</td>
                    </tr>`,
        )
        .join("");
    }

    const tbodySsb = document.getElementById("tbody-ssb-tags");
    if (tbodySsb) {
      tbodySsb.innerHTML = state.ssbTags
        .map(
          (t) => `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px; color:#34d399;">${t.code}</td>
                        <td>
                            <div style="font-weight:700; color:#ffffff;">${t.name_tr}</div>
                            <div style="font-size:11px; color:var(--text-muted);">${t.name_en}</div>
                        </td>
                        <td><span class="badge-tag badge-ssb">${t.category_group}</span></td>
                        <td style="font-family:var(--font-mono);">${t.sort_order}</td>
                    </tr>`,
        )
        .join("");
    }
  }

  async function submitCategoryForm(e) {
    e.preventDefault();
    const payload = {
      id: 0,
      slug: document.getElementById("cat-slug").value.trim(),
      parent_slug: null,
      name_tr: document.getElementById("cat-name-tr").value.trim(),
      name_en: document.getElementById("cat-name-en").value.trim(),
      group_type: document.getElementById("cat-group-type").value,
      description_tr:
        document.getElementById("cat-desc-tr").value.trim() ||
        "MSI Dergisi özel kategori bölümü",
      description_en: document.getElementById("cat-name-en").value.trim(),
      icon_class: "fa-solid fa-layer-group",
      sort_order: parseInt(
        document.getElementById("cat-sort-order").value || "25",
        10,
      ),
      is_dynamic: true,
    };
    const res = await safeFetchJson("/api/v1/admin/categories", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast(
        'Yeni kategori "' + payload.name_tr + '" site ağacına eklendi!',
      );
      document.getElementById("form-new-category").reset();
      await refreshAllData();
    }
    return false;
  }

  async function submitSsbTagForm(e) {
    e.preventDefault();
    const payload = {
      id: 0,
      code: document.getElementById("ssb-code").value.trim(),
      name_tr: document.getElementById("ssb-name-tr").value.trim(),
      name_en: document.getElementById("ssb-name-en").value.trim(),
      category_group:
        document.getElementById("ssb-group").value.trim() || "Savunma Sanayii",
      sort_order: parseInt(
        document.getElementById("ssb-sort").value || "15",
        10,
      ),
    };
    const res = await safeFetchJson("/api/v1/admin/ssb-tags", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast(
        'Yeni SSB İlgi Alanı "' + payload.name_tr + '" taksonomiye eklendi!',
      );
      document.getElementById("form-new-ssb").reset();
      await refreshAllData();
    }
    return false;
  }

  async function submitNewsletterForm(e) {
    e.preventDefault();
    const payload = {
      id: 0,
      subject_tr: document.getElementById("nl-subject-tr").value.trim(),
      subject_en: document.getElementById("nl-subject-en").value.trim(),
      target_group_filter: document.getElementById("nl-group-filter").value,
      target_institution: document.getElementById("nl-institution").value,
      target_ssb_tag: document.getElementById("nl-ssb-tag").value,
      sponsor_ad_id: parseInt(
        document.getElementById("nl-sponsor-ad-id").value || "1",
        10,
      ),
      sponsor_name:
        document.getElementById("nl-sponsor-name").value.trim() || null,
      recipient_count: 0,
      status: "sent",
      sent_at: new Date().toISOString().slice(0, 16).replace("T", " "),
    };
    const res = await safeFetchJson("/api/v1/admin/newsletters", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast(
        "Sponsorlu E-Bülten seçili kurumsal gruba gönderildi ve kaydedildi!",
      );
      document.getElementById("form-newsletter").reset();
      await refreshAllData();
    }
    return false;
  }

  function renderNewslettersTable() {
    const tbody = document.getElementById("tbody-newsletters");
    if (!tbody) return;
    if (!state.newsletters.length) {
      tbody.innerHTML =
        '<tr><td colspan="5" style="text-align:center; padding:20px;">Gönderilmiş bülten kaydı bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = state.newsletters
      .map(
        (n) => `<tr>
                    <td style="font-family:var(--font-mono); font-size:11.5px;">
                        <strong>#${n.id}</strong><br>
                        <span style="color:var(--text-muted);">${n.sent_at || ""}</span>
                    </td>
                    <td>
                        <div style="font-weight:700; color:#ffffff;">${n.subject_tr}</div>
                        <div style="font-size:11px; color:var(--text-muted);">${n.subject_en || ""}</div>
                    </td>
                    <td>
                        <span class="badge-tag">${n.target_institution}</span>
                        <span class="badge-tag badge-ssb">${n.target_ssb_tag}</span>
                        <span class="badge-tag badge-placement">${n.target_group_filter}</span>
                    </td>
                    <td><span class="badge-tag badge-sponsored">${n.sponsor_name || "Sponsorsuz"}</span></td>
                    <td style="font-family:var(--font-mono);">
                        <strong style="color:#34d399;">${(n.recipient_count || 0).toLocaleString("tr-TR")} alıcı</strong><br>
                        <span style="font-size:11px; color:var(--text-muted);">${n.status}</span>
                    </td>
                </tr>`,
      )
      .join("");
  }

  function renderRemindersAndRedirects() {
    const tbodyRem = document.getElementById("tbody-reminders");
    if (tbodyRem) {
      const stageMap = {
        "2_months": "2 Ay Kala (60 Gün)",
        "1_month": "1 Ay Kala (30 Gün)",
        "2_weeks": "2 Hafta Kala (14 Gün)",
        "1_week": "1 Hafta Kala (7 Gün)",
        "1_day": "1 Gün Kala (Son Uyarı)",
      };
      tbodyRem.innerHTML = state.reminders
        .map((r) => {
          const stBadge =
            r.status === "halted_by_renewal"
              ? '<span class="badge-tag badge-paywall-0"><i class="fa-solid fa-circle-stop"></i> Yeniledi — Otomatik Durduruldu</span>'
              : r.status === "sent"
                ? '<span class="badge-tag"><i class="fa-solid fa-check-double"></i> Gönderildi</span>'
                : '<span class="badge-tag badge-paywall-2"><i class="fa-regular fa-clock"></i> Planlandı</span>';
          return `<tr>
                            <td>
                                <div style="font-weight:700; color:#ffffff;">${r.full_name}</div>
                                <div style="font-size:11px; color:var(--text-muted);">${r.user_email} • ${r.organization_name}</div>
                            </td>
                            <td><span class="badge-tag badge-placement">${stageMap[r.stage_label] || r.stage_label}</span></td>
                            <td style="font-family:var(--font-mono); font-weight:700;">${r.days_remaining} gün</td>
                            <td>${stBadge}</td>
                            <td style="font-family:var(--font-mono); font-size:11px; color:var(--text-muted);">${r.sent_at}</td>
                        </tr>`;
        })
        .join("");
    }

    const tbodySeo = document.getElementById("tbody-redirects");
    if (tbodySeo) {
      tbodySeo.innerHTML = state.redirects
        .map(
          (s) => `<tr>
                        <td style="font-family:var(--font-mono); font-size:11.5px; color:#fca5a5;">${s.old_path}</td>
                        <td style="font-family:var(--font-mono); font-size:11.5px; color:#34d399;">${s.new_path}</td>
                        <td><span class="badge-tag badge-paywall-0">${s.status_code || 301}</span></td>
                        <td style="font-family:var(--font-mono); font-weight:700;">${s.hit_count || 0}</td>
                    </tr>`,
        )
        .join("");
    }
  }

  async function simulateRenewalHalt() {
    const res = await safeFetchJson("/api/v1/admin/reminders/halt-renewed", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action: "halt_renewed" }),
    });
    state.reminders.forEach((r) => {
      if (r.status === "scheduled") {
        r.status = "halted_by_renewal";
      }
    });
    renderRemindersAndRedirects();
    showToast(
      "Aboneliğini yenileyen kullanıcılar için kalan hatırlatma e-postaları otomatik olarak durduruldu (halted_by_renewal)!",
    );
    if (res) await refreshAllData();
  }

  async function submitSeoRedirect(e) {
    e.preventDefault();
    const payload = {
      id: 0,
      old_path: document.getElementById("seo-old-path").value.trim(),
      new_path: document.getElementById("seo-new-path").value.trim(),
      status_code: 301,
      hit_count: 1,
    };
    const res = await safeFetchJson("/api/v1/admin/redirects", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast("301 Kalıcı SEO yönlendirme kuralı eklendi!");
      document.getElementById("form-seo-redirect").reset();
      await refreshAllData();
    }
    return false;
  }

  function toggleAdminTheme() {
    const isLight = document.body.classList.toggle("admin-light-mode");
    try {
      localStorage.setItem("msi_admin_theme", isLight ? "light" : "dark");
    } catch (e) {}
  }

  async function refreshCentralEnginesTab() {
    const [settingsRes, secRes, notifRes] = await Promise.all([
      safeFetchJson("/api/v1/admin/settings"),
      safeFetchJson("/api/v1/admin/security-logs"),
      safeFetchJson("/api/v1/admin/notifications"),
    ]);
    const tbodySet = document.getElementById("tbody-site-settings");
    if (tbodySet && settingsRes && Array.isArray(settingsRes.settings)) {
      tbodySet.innerHTML = settingsRes.settings
        .map(
          (s) => `<tr>
                        <td style="font-family:var(--font-mono); color:#38bdf8;">${s.key}</td>
                        <td>${s.value_tr}</td>
                        <td style="color:var(--text-muted);">${s.value_en}</td>
                        <td><span class="badge-tag">${s.setting_group}</span></td>
                    </tr>`,
        )
        .join("");
    }
    const tbodySec = document.getElementById("tbody-security-logs");
    if (tbodySec && secRes && Array.isArray(secRes.logs)) {
      tbodySec.innerHTML = secRes.logs
        .map(
          (l) => `<tr>
                        <td style="font-family:var(--font-mono); font-size:11px;">${l.created_at}</td>
                        <td style="font-family:var(--font-mono); color:#fca5a5;">${l.client_ip}</td>
                        <td><span class="badge-tag badge-sponsored">${l.event_type}</span></td>
                        <td><span class="badge-tag badge-paywall-2">${l.threat_level}</span></td>
                        <td style="font-size:11.5px;"><code>${l.request_path}</code><br><span style="color:var(--text-muted);">${l.details}</span></td>
                    </tr>`,
        )
        .join("");
    }
    const tbodyNotif = document.getElementById("tbody-sys-notifications");
    if (tbodyNotif && notifRes && Array.isArray(notifRes.notifications)) {
      tbodyNotif.innerHTML = notifRes.notifications
        .map(
          (n) => `<tr>
                        <td><span class="badge-tag">${n.target_audience}</span></td>
                        <td><span class="badge-tag badge-paywall-1">${n.severity}</span></td>
                        <td><strong>${n.title_tr}</strong><br><span style="color:var(--text-muted); font-size:11.5px;">${n.message_tr}</span></td>
                        <td>${n.is_banner ? '<span class="badge-tag badge-paywall-0">ÜST BANT</span>' : '<span class="badge-tag">BİLDİRİM</span>'}</td>
                    </tr>`,
        )
        .join("");
    }
  }

  async function submitSiteSetting(e) {
    e.preventDefault();
    const payload = {
      key: document.getElementById("set-key").value,
      value_tr: document.getElementById("set-val-tr").value.trim(),
      value_en:
        document.getElementById("set-val-en").value.trim() ||
        document.getElementById("set-val-tr").value.trim(),
      setting_group:
        document.getElementById("set-group").value.trim() || "general",
    };
    const res = await safeFetchJson("/api/v1/admin/settings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast("Merkezi tasarım ayarı güncellendi ve L1 Önbellek yenilendi!");
      await refreshCentralEnginesTab();
    }
    return false;
  }

  async function submitSysNotification(e) {
    e.preventDefault();
    const payload = {
      target_audience: document.getElementById("notif-audience").value,
      severity: document.getElementById("notif-severity").value,
      is_banner: document.getElementById("notif-banner").value === "true",
      category: "editorial",
      title_tr: document.getElementById("notif-title-tr").value.trim(),
      message_tr: document.getElementById("notif-msg-tr").value.trim(),
    };
    const res = await safeFetchJson("/api/v1/admin/notifications", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (res) {
      showToast("Merkezi bildirim ve üst bant duyurusu yayınlandı!");
      document.getElementById("form-sys-notif").reset();
      await refreshCentralEnginesTab();
    }
    return false;
  }

  async function uploadCoverAsset(fileInput) {
    const file = fileInput?.files?.[0];
    if (!file) return;
    const fd = new FormData();
    fd.append("file", file);
    try {
      const resp = await fetch("/api/v1/admin/upload", {
        method: "POST",
        body: fd,
      });
      const data = await resp.json();
      if (!resp.ok) {
        showToast(data.error || "Dosya yüklenemedi.", true);
        return;
      }
      const target = document.getElementById("c-cover-image");
      if (target && data.path) target.value = data.path;
      showToast("Medya başarıyla yüklendi: " + (data.path || file.name));
    } catch (e) {
      showToast("Medya yükleme sırasında hata oluştu.", true);
    } finally {
      fileInput.value = "";
    }
  }

  document.addEventListener("DOMContentLoaded", () => {
    try {
      if (localStorage.getItem("msi_admin_theme") === "light") {
        document.body.classList.add("admin-light-mode");
      }
    } catch (e) {}
    refreshAllData();
    refreshCentralEnginesTab();
  });

  // =========================================================================
  // ADVANCED ADMIN: Tab 9 - Analytics Charts & System Health
  // =========================================================================

  function renderAnalyticsCharts() {
    if (!state.contentAnalytics.length) return;
    const trendCanvas = document.getElementById("chart-content-trend");
    if (!trendCanvas || typeof Chart === "undefined") return;
    const sorted = [...state.contentAnalytics].sort((a, b) => (a.slug || "").localeCompare(b.slug || ""));
    const labels = sorted.slice(0, 30).map((c) => c.slug || c.title_tr || "?");
    const views = sorted.slice(0, 30).map((c) => c.total_views || c.view_count || 0);
    const reads = sorted.slice(0, 30).map((c) => c.total_true_reads || c.true_read_count || 0);
    if (trendCanvas._chartInstance) trendCanvas._chartInstance.destroy();
    trendCanvas._chartInstance = new Chart(trendCanvas.getContext("2d"), {
      type: "line",
      data: {
        labels,
        datasets: [
          { label: "Görüntülenme", data: views, borderColor: "#38bdf8", backgroundColor: "rgba(56,189,248,0.1)", tension: 0.3, fill: true },
          { label: "Gerçek Okuma", data: reads, borderColor: "#34d399", backgroundColor: "rgba(52,211,153,0.1)", tension: 0.3, fill: true },
        ],
      },
      options: { responsive: true, plugins: { legend: { labels: { color: "#cbd5e1" } } }, scales: { x: { ticks: { color: "#64748b", maxRotation: 45 } }, y: { ticks: { color: "#64748b" }, beginAtZero: true } } },
    });
    const topCanvas = document.getElementById("chart-top-contents");
    if (!topCanvas) return;
    const top10 = [...state.contentAnalytics].sort((a, b) => (b.total_views || b.view_count || 0) - (a.total_views || a.view_count || 0)).slice(0, 10);
    if (topCanvas._chartInstance) topCanvas._chartInstance.destroy();
    topCanvas._chartInstance = new Chart(topCanvas.getContext("2d"), {
      type: "bar",
      data: {
        labels: top10.map((c) => (c.title_tr || c.slug || "?").substring(0, 30)),
        datasets: [{ label: "Toplam Görüntülenme", data: top10.map((c) => c.total_views || c.view_count || 0), backgroundColor: ["#ef4444","#f97316","#fbbf24","#34d399","#38bdf8","#818cf8","#a78bfa","#f472b6","#fb923c","#2dd4bf"] }],
      },
      options: { indexAxis: "y", responsive: true, plugins: { legend: { display: false } }, scales: { x: { ticks: { color: "#64748b" }, beginAtZero: true }, y: { ticks: { color: "#cbd5e1" } } } },
    });
  }

  async function refreshSystemHealth() {
    const panel = document.getElementById("system-health-panel");
    if (!panel) return;
    try {
      const res = await fetch("/api/v1/admin/system-health");
      const data = await res.json();
      state.systemHealth = data;
      const dbMB = ((data.db_size_bytes || 0) / 1048576).toFixed(2);
      const statusColor = data.status === "healthy" ? "#34d399" : "#ef4444";
      panel.innerHTML = `<div style="display:grid; gap:8px;">
        <div><span style="color:var(--text-muted);">Durum:</span> <span style="color:${statusColor}; font-weight:700;">${(data.status || "unknown").toUpperCase()}</span></div>
        <div><span style="color:var(--text-muted);">DB Boyutu:</span> <strong>${dbMB} MB</strong></div>
        <div><span style="color:var(--text-muted);">İçerik Sayısı:</span> <strong>${data.tables?.contents || 0}</strong></div>
        <div><span style="color:var(--text-muted);">Kullanıcı Sayısı:</span> <strong>${data.tables?.users || 0}</strong></div>
        <div><span style="color:var(--text-muted);">Analitik Event:</span> <strong>${data.tables?.analytics_events || 0}</strong></div>
        <div><span style="color:var(--text-muted);">Son Kontrol:</span> ${new Date(data.timestamp).toLocaleString("tr-TR")}</div>
      </div>`;
    } catch (e) { panel.innerHTML = `<div style="color:#ef4444;">Sağlık kontrolü başarısız: ${e.message}</div>`; }
  }

  // =========================================================================
  // ADVANCED ADMIN: Tab 10 - Media Library & SEO Preview
  // =========================================================================

  async function loadMediaLibrary() {
    try {
      const res = await fetch("/api/v1/admin/media-library");
      const data = await res.json();
      state.mediaLibrary = data.items || [];
      renderMediaLibrary(state.mediaLibrary);
    } catch (e) { console.error("Media library load failed:", e); }
  }

  function renderMediaLibrary(items) {
    const grid = document.getElementById("media-library-grid");
    if (!grid) return;
    if (!items.length) { grid.innerHTML = '<div style="color:var(--text-muted); padding:20px; text-align:center;">Medya bulunamadı.</div>'; return; }
    grid.innerHTML = items.map((m) => {
      const isImage = (m.mime_type || "").startsWith("image/");
      const sizeKB = ((m.file_size || 0) / 1024).toFixed(1);
      const thumb = isImage ? `<img src="${m.path}" alt="${esc(m.original_name)}" style="width:100%; height:90px; object-fit:cover; border-radius:6px; margin-bottom:6px;" loading="lazy">` : `<div style="width:100%; height:90px; background:rgba(255,255,255,0.05); border-radius:6px; margin-bottom:6px; display:flex; align-items:center; justify-content:center; font-size:24px; color:var(--text-muted);"><i class="fa-solid fa-file"></i></div>`;
      return `<div style="background:rgba(255,255,255,0.03); border-radius:8px; padding:8px; cursor:pointer;" onclick="navigator.clipboard.writeText('${esc(m.path)}'); showToast('URL kopyalandı!')">
        ${thumb}
        <div style="font-size:11px; color:#fff; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${esc(m.original_name)}">${esc(m.original_name)}</div>
        <div style="font-size:10px; color:var(--text-muted);">${sizeKB} KB · ${esc(m.mime_type)}</div>
      </div>`;
    }).join("");
  }

  function filterMediaLibrary() {
    const q = (document.getElementById("media-search")?.value || "").toLowerCase();
    if (!q) { renderMediaLibrary(state.mediaLibrary); return; }
    const filtered = state.mediaLibrary.filter((m) => (m.original_name || "").toLowerCase().includes(q) || (m.tags || "").toLowerCase().includes(q) || (m.mime_type || "").toLowerCase().includes(q));
    renderMediaLibrary(filtered);
  }

  async function bulkUploadMedia(input) {
    if (!input.files.length) return;
    const formData = new FormData();
    for (const file of input.files) formData.append("file", file);
    try {
      await fetch("/api/v1/admin/upload", { method: "POST", body: formData });
      showToast(`${input.files.length} dosya yüklendi!`);
      loadMediaLibrary();
    } catch (e) { showToast("Yükleme başarısız: " + e.message); }
    input.value = "";
  }

  function populateSeoPreviewSelect() {
    const sel = document.getElementById("seo-preview-select");
    if (!sel) return;
    sel.innerHTML = '<option value="">-- İçerik seçin --</option>' + state.contents.map((c) => `<option value="${c.id}">${esc(c.title_tr || c.slug)} (${c.content_type || "article"})</option>`).join("");
  }

  function updateSeoPreview() {
    const id = parseInt(document.getElementById("seo-preview-select")?.value || "0", 10);
    const content = state.contents.find((c) => c.id === id);
    if (!content) return;
    const title = content.title_tr || content.slug || "Başlıksız";
    const desc = (content.summary_tr || content.body_tr || "").replace(/<[^>]*>/g, "").substring(0, 160);
    const url = `msidergisi.com/icerik/${content.slug || content.id}`;
    const el = (s) => document.getElementById(s);
    if (el("seo-google-title")) el("seo-google-title").textContent = title;
    if (el("seo-google-url")) el("seo-google-url").textContent = url;
    if (el("seo-google-desc")) el("seo-google-desc").textContent = desc || "Açıklama yok.";
    if (el("seo-og-title")) el("seo-og-title").textContent = title;
    if (el("seo-og-desc")) el("seo-og-desc").textContent = desc || "Açıklama yok.";
    const jsonLd = { "@context": "https://schema.org", "@type": "NewsArticle", headline: title, description: desc, url: `https://${url}`, datePublished: content.created_at || new Date().toISOString(), author: { "@type": "Organization", name: "MSI Dergisi" }, publisher: { "@type": "Organization", name: "MSI Dergisi", logo: { "@type": "ImageObject", url: "https://msidergisi.com/images/logo.png" } } };
    if (el("seo-jsonld")) el("seo-jsonld").textContent = JSON.stringify(jsonLd, null, 2);
  }

  // =========================================================================
  // ADVANCED ADMIN: Tab 11 - Roles, Permissions & Audit Log
  // =========================================================================

  async function loadUserRoles() {
    try {
      const res = await fetch("/api/v1/admin/user-roles");
      const data = await res.json();
      state.userRoles = data.roles || [];
      renderUserRoles();
    } catch (e) { console.error("Roles load failed:", e); }
  }

  function renderUserRoles() {
    const tbody = document.getElementById("tbody-user-roles");
    if (!tbody) return;
    if (!state.userRoles.length) { tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; color:var(--text-muted);">Henüz rol tanımlanmamış.</td></tr>'; return; }
    tbody.innerHTML = state.userRoles.map((r) => {
      let permCount = 0;
      try { permCount = JSON.parse(r.permissions_json || "[]").length; } catch (_) {}
      return `<tr>
        <td><code>${esc(r.role_code)}</code></td>
        <td>${esc(r.name_tr)}</td>
        <td>${esc(r.name_en)}</td>
        <td><span style="background:rgba(56,189,248,0.15); color:#38bdf8; padding:2px 8px; border-radius:10px; font-size:11px;">${permCount} izin</span></td>
        <td>${r.is_system ? '<i class="fa-solid fa-lock" style="color:#fbbf24;" title="Sistem rolü"></i>' : '<i class="fa-solid fa-unlock" style="color:var(--text-muted);"></i>'}</td>
      </tr>`;
    }).join("");
  }

  function showRoleForm() { const f = document.getElementById("role-form-container"); if (f) f.style.display = "block"; }

  async function saveRole() {
    const code = document.getElementById("role-code")?.value?.trim();
    const nameTr = document.getElementById("role-name-tr")?.value?.trim();
    const nameEn = document.getElementById("role-name-en")?.value?.trim();
    if (!code || !nameTr) { showToast("Rol kodu ve TR adı zorunludur!"); return; }
    const perms = [...document.querySelectorAll("#role-permissions-grid input:checked")].map((cb) => cb.value);
    try {
      await fetch("/api/v1/admin/user-roles", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ role_code: code, name_tr: nameTr, name_en: nameEn || nameTr, permissions_json: JSON.stringify(perms) }) });
      showToast("Rol kaydedildi!");
      document.getElementById("role-form-container").style.display = "none";
      loadUserRoles();
    } catch (e) { showToast("Rol kaydetme başarısız: " + e.message); }
  }

  async function loadAuditLogs() {
    try {
      const res = await fetch("/api/v1/admin/audit-logs");
      const data = await res.json();
      state.auditLogs = data.logs || [];
      renderAuditLogs(state.auditLogs);
    } catch (e) { console.error("Audit logs load failed:", e); }
  }

  function renderAuditLogs(logs) {
    const tbody = document.getElementById("tbody-audit-logs");
    if (!tbody) return;
    if (!logs.length) { tbody.innerHTML = '<tr><td colspan="6" style="text-align:center; color:var(--text-muted);">Denetim kaydı bulunamadı.</td></tr>'; return; }
    tbody.innerHTML = logs.map((l) => {
      const actionColors = { create: "#34d399", update: "#38bdf8", delete: "#ef4444", login: "#fbbf24", flush: "#a78bfa" };
      const color = actionColors[l.action] || "#94a3b8";
      let detail = "";
      try { const d = JSON.parse(l.details_json || "{}"); detail = Object.entries(d).map(([k,v]) => `${k}: ${v}`).join(", "); } catch (_) { detail = l.details_json || ""; }
      return `<tr>
        <td style="white-space:nowrap; font-size:11px;">${new Date(l.created_at).toLocaleString("tr-TR")}</td>
        <td>${esc(l.actor_email)}</td>
        <td><span style="color:${color}; font-weight:600;">${esc(l.action)}</span></td>
        <td>${esc(l.entity_type)}${l.entity_id ? " #" + l.entity_id : ""}</td>
        <td style="max-width:200px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title="${esc(detail)}">${esc(detail.substring(0, 60))}</td>
        <td style="font-size:11px; color:var(--text-muted);">${esc(l.ip_address || "-")}</td>
      </tr>`;
    }).join("");
  }

  function filterAuditLogs() {
    const actionFilter = document.getElementById("audit-filter-action")?.value || "";
    const actorFilter = (document.getElementById("audit-filter-actor")?.value || "").toLowerCase();
    const filtered = state.auditLogs.filter((l) => {
      if (actionFilter && l.action !== actionFilter) return false;
      if (actorFilter && !(l.actor_email || "").toLowerCase().includes(actorFilter)) return false;
      return true;
    });
    renderAuditLogs(filtered);
  }

  // =========================================================================
  // ADVANCED ADMIN: Tab 12 - Bulk Operations & Version History
  // =========================================================================

  function renderBulkContentsTable() {
    const tbody = document.getElementById("tbody-bulk-contents");
    if (!tbody) return;
    state.bulkSelected.clear();
    updateBulkCount();
    tbody.innerHTML = state.contents.map((c) => {
      const visLabels = { 0: "Açık", 1: "Kayıtlı", 2: "Abone", 3: "Premium" };
      return `<tr>
        <td><input type="checkbox" class="bulk-cb" data-id="${c.id}" onchange="window.MSIAdmin.toggleBulkSelect(${c.id}, this.checked)"></td>
        <td>${c.id}</td>
        <td>${esc(c.title_tr || c.slug)}</td>
        <td><span style="font-size:11px; color:var(--text-muted);">${esc(c.content_type || "article")}</span></td>
        <td><span style="font-size:11px;">${visLabels[c.visibility_level] || c.visibility_level}</span></td>
        <td>${c.view_count || 0}</td>
      </tr>`;
    }).join("");
    populateVersionHistorySelect();
    populateSeoPreviewSelect();
  }

  function toggleBulkSelect(id, checked) {
    if (checked) state.bulkSelected.add(id); else state.bulkSelected.delete(id);
    updateBulkCount();
  }

  function selectAllBulkContents() {
    state.contents.forEach((c) => state.bulkSelected.add(c.id));
    document.querySelectorAll(".bulk-cb").forEach((cb) => { cb.checked = true; });
    updateBulkCount();
  }

  function deselectAllBulkContents() {
    state.bulkSelected.clear();
    document.querySelectorAll(".bulk-cb").forEach((cb) => { cb.checked = false; });
    updateBulkCount();
  }

  function toggleBulkSelectAll(masterCb) {
    if (masterCb.checked) selectAllBulkContents(); else deselectAllBulkContents();
  }

  function updateBulkCount() {
    const el = document.getElementById("bulk-selected-count");
    if (el) el.textContent = state.bulkSelected.size;
  }

  async function executeBulkOperation() {
    if (!state.bulkSelected.size) { showToast("Lütfen en az bir içerik seçin!"); return; }
    const rawAction = document.getElementById("bulk-action-select")?.value;
    if (!rawAction) { showToast("Lütfen bir işlem seçin!"); return; }
    let action = rawAction, value = null;
    if (rawAction.startsWith("set_visibility_")) { action = "set_visibility"; value = rawAction.split("_")[2]; }
    else if (rawAction === "set_featured") { value = "true"; }
    else if (rawAction === "unset_featured") { action = "set_featured"; value = "false"; }
    if (action === "delete" && !confirm(`${state.bulkSelected.size} içerik kalıcı olarak silinecek. Emin misiniz?`)) return;
    try {
      const res = await fetch("/api/v1/admin/contents/bulk", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ids: [...state.bulkSelected], action, value }) });
      const data = await res.json();
      showToast(`İşlem tamamlandı: ${data.affected || 0} içerik etkilen.`);
      state.bulkSelected.clear();
      refreshAllData();
    } catch (e) { showToast("Toplu işlem başarısız: " + e.message); }
  }

  function populateVersionHistorySelect() {
    const sel = document.getElementById("version-history-select");
    if (!sel) return;
    sel.innerHTML = '<option value="">-- İçerik seçin --</option>' + state.contents.map((c) => `<option value="${c.id}">${esc(c.title_tr || c.slug)}</option>`).join("");
  }

  async function loadVersionHistory() {
    const id = parseInt(document.getElementById("version-history-select")?.value || "0", 10);
    const container = document.getElementById("version-history-list");
    if (!container || !id) return;
    try {
      const res = await fetch(`/api/v1/admin/contents/${id}/versions`);
      const data = await res.json();
      const versions = data.versions || [];
      if (!versions.length) { container.innerHTML = '<div style="color:var(--text-muted); padding:20px; text-align:center; font-size:12px;">Bu içerik için versiyon geçmişi bulunamadı.</div>'; return; }
      container.innerHTML = versions.map((v) => `<div style="padding:12px; border-bottom:1px solid var(--border-subtle); font-size:12px;">
        <div style="display:flex; justify-content:space-between; margin-bottom:4px;">
          <strong style="color:#38bdf8;">v${v.version}</strong>
          <span style="color:var(--text-muted); font-size:11px;">${new Date(v.created_at).toLocaleString("tr-TR")}</span>
        </div>
        <div style="margin-bottom:2px;">${esc(v.title_tr)}</div>
        <div style="color:var(--text-muted); font-size:11px;"><i class="fa-solid fa-user-pen"></i> ${esc(v.changed_by)} · ${esc(v.change_note || "Not yok")}</div>
      </div>`).join("");
    } catch (e) { container.innerHTML = `<div style="color:#ef4444; padding:20px; text-align:center;">Versiyon yükleme hatası: ${e.message}</div>`; }
  }

  // =========================================================================
  // ADVANCED ADMIN: Tab 13 - Custom Fields & Cache Management
  // =========================================================================

  async function loadCustomFields() {
    try {
      const res = await fetch("/api/v1/admin/custom-fields?applies_to=content");
      const data = await res.json();
      state.customFields = data.fields || [];
      renderCustomFields();
    } catch (e) { console.error("Custom fields load failed:", e); }
  }

  function renderCustomFields() {
    const tbody = document.getElementById("tbody-custom-fields");
    if (!tbody) return;
    if (!state.customFields.length) { tbody.innerHTML = '<tr><td colspan="8" style="text-align:center; color:var(--text-muted);">Henüz özel alan tanımlanmamış.</td></tr>'; return; }
    tbody.innerHTML = state.customFields.map((f) => `<tr>
      <td><code>${esc(f.field_key)}</code></td>
      <td>${esc(f.label_tr)}</td>
      <td>${esc(f.label_en)}</td>
      <td><span style="font-size:11px; background:rgba(129,140,248,0.15); color:#818cf8; padding:2px 8px; border-radius:10px;">${esc(f.field_type)}</span></td>
      <td style="font-size:11px;">${esc(f.applies_to)}</td>
      <td>${f.is_required ? '<i class="fa-solid fa-check" style="color:#34d399;"></i>' : '<i class="fa-solid fa-minus" style="color:var(--text-muted);"></i>'}</td>
      <td>${f.sort_order}</td>
      <td><button type="button" class="btn-cmd btn-cmd-sm" style="border-color:#ef4444; color:#ef4444; padding:2px 8px;" onclick="window.MSIAdmin.deleteCustomField('${esc(f.field_key)}')"><i class="fa-solid fa-trash"></i></button></td>
    </tr>`).join("");
  }

  function showCustomFieldForm() { const f = document.getElementById("custom-field-form-container"); if (f) f.style.display = "block"; }

  async function saveCustomField() {
    const key = document.getElementById("cf-key")?.value?.trim();
    const labelTr = document.getElementById("cf-label-tr")?.value?.trim();
    const labelEn = document.getElementById("cf-label-en")?.value?.trim();
    if (!key || !labelTr) { showToast("Alan anahtarı ve TR etiketi zorunludur!"); return; }
    try {
      await fetch("/api/v1/admin/custom-fields", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({
        field_key: key, label_tr: labelTr, label_en: labelEn || labelTr,
        field_type: document.getElementById("cf-type")?.value || "text",
        options_json: document.getElementById("cf-options")?.value || "[]",
        is_required: document.getElementById("cf-required")?.checked || false,
        sort_order: parseInt(document.getElementById("cf-sort-order")?.value || "0", 10),
        applies_to: document.getElementById("cf-applies-to")?.value || "content",
      })});
      showToast("Özel alan kaydedildi!");
      document.getElementById("custom-field-form-container").style.display = "none";
      loadCustomFields();
    } catch (e) { showToast("Kaydetme başarısız: " + e.message); }
  }

  async function deleteCustomField(fieldKey) {
    if (!confirm(`"${fieldKey}" alanı silinecek. Emin misiniz?`)) return;
    try {
      await fetch(`/api/v1/admin/custom-fields/${encodeURIComponent(fieldKey)}`, { method: "DELETE" });
      showToast("Alan silindi!");
      loadCustomFields();
    } catch (e) { showToast("Silme başarısız: " + e.message); }
  }

  async function loadCacheOperations() {
    try {
      const res = await fetch("/api/v1/admin/cache");
      const data = await res.json();
      state.cacheOperations = data.operations || [];
      renderCacheOperations();
    } catch (e) { console.error("Cache ops load failed:", e); }
  }

  function renderCacheOperations() {
    const tbody = document.getElementById("tbody-cache-operations");
    if (!tbody) return;
    if (!state.cacheOperations.length) { tbody.innerHTML = '<tr><td colspan="5" style="text-align:center; color:var(--text-muted);">Önbellek işlem kaydı bulunamadı.</td></tr>'; return; }
    tbody.innerHTML = state.cacheOperations.map((op) => `<tr>
      <td style="white-space:nowrap; font-size:11px;">${new Date(op.created_at).toLocaleString("tr-TR")}</td>
      <td><code style="font-size:11px;">${esc(op.cache_key)}</code></td>
      <td><span style="color:#fbbf24; font-weight:600;">${esc(op.operation)}</span></td>
      <td>${esc(op.actor_email)}</td>
      <td style="font-size:11px; color:var(--text-muted);">${esc(op.details || "-")}</td>
    </tr>`).join("");
  }

  async function flushAllCache() {
    if (!confirm("Tüm önbellek katmanları temizlenecek. Devam etmek istiyor musunuz?")) return;
    try {
      await fetch("/api/v1/admin/cache", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ cache_key: "all" }) });
      showToast("Tüm önbellek temizlendi!");
      loadCacheOperations();
    } catch (e) { showToast("Önbellek temizleme başarısız: " + e.message); }
  }

  // =========================================================================
  // Hook into existing refreshAllData to also load advanced tab data
  // =========================================================================

  const _origRefreshAllData = refreshAllData;
  refreshAllData = async function () {
    await _origRefreshAllData();
    renderAnalyticsCharts();
    refreshSystemHealth();
    loadMediaLibrary();
    loadUserRoles();
    loadAuditLogs();
    renderBulkContentsTable();
    loadCustomFields();
    loadCacheOperations();
  };

  return {
    switchTab,
    switchAnalyticsSubview,
    toggleSponsoredFields,
    toggleAdminTheme,
    refreshAllData,
    renderContentsTable,
    loadContentIntoForm,
    resetContentForm,
    submitContentForm,
    deleteContent,
    checkAdConflictLive,
    submitAdForm,
    downloadMonthlyConsolidatedReport,
    decideCorporateRequest,
    submitCategoryForm,
    submitSsbTagForm,
    submitNewsletterForm,
    simulateRenewalHalt,
    submitSeoRedirect,
    submitSiteSetting,
    submitSysNotification,
    uploadCoverAsset,
    // Advanced Admin Tab 9
    refreshSystemHealth,
    renderAnalyticsCharts,
    // Advanced Admin Tab 10
    filterMediaLibrary,
    bulkUploadMedia,
    updateSeoPreview,
    // Advanced Admin Tab 11
    showRoleForm,
    saveRole,
    filterAuditLogs,
    // Advanced Admin Tab 12
    toggleBulkSelect,
    selectAllBulkContents,
    deselectAllBulkContents,
    toggleBulkSelectAll,
    executeBulkOperation,
    loadVersionHistory,
    // Advanced Admin Tab 13
    showCustomFieldForm,
    saveCustomField,
    deleteCustomField,
    flushAllCache,
  };
})();
