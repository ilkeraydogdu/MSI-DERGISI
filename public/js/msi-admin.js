window.MSIAdmin = (function () {
  "use strict";

  const OFFICIAL_SSB_TAGS = [
    { code: "kara-araclari", name_tr: "Kara Araçları" },
    { code: "deniz-araclari", name_tr: "Deniz Araçları" },
    { code: "hava-araclari", name_tr: "Hava Araçları" },
    { code: "muhimmatlar-ve-fuze-sistemleri", name_tr: "Mühimmatlar ve Füze Sistemleri" },
    { code: "namlulu-silahlar", name_tr: "Namlulu Silahlar" },
    { code: "haberlesme-ve-muharebe-destek-sistemleri", name_tr: "Haberleşme ve Muharebe Destek Sistemleri" },
    { code: "radarlar-ve-sensorler", name_tr: "Radarlar ve Sensörler" },
    { code: "simulatorler-ve-egitim-sistemleri", name_tr: "Simülatörler ve Eğitim Sistemleri" },
    { code: "elektronik-harp-sistemleri", name_tr: "Elektronik Harp Sistemleri" },
    { code: "itki-tahrik-ve-guc-sistemleri", name_tr: "İtki / Tahrik ve Güç Sistemleri" },
    { code: "hizmetler-danismanlik", name_tr: "Hizmetler - Danışmanlık" },
    { code: "hizmetler-egitim", name_tr: "Hizmetler - Eğitim" },
    { code: "hizmetler-kurumsal-iletisim", name_tr: "Hizmetler - Kurumsal İletişim" },
    { code: "hizmetler-kurumsal-catering", name_tr: "Hizmetler - Kurumsal Catering" },
  ];

  const state = {
    overviewStats: null,
    totalContentsCount: 0,
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
    adminUsers: [],
    customFields: [],
    cacheOperations: [],
    systemHealth: null,
    activeAnalyticsSubview: "content",
    bulkSelected: new Set(),
  };

  const paginationState = {
    contents: 1,
    userRoles: 1,
    customFields: 1,
    cacheOperations: 1,
    newsletters: 1,
    reminders: 1,
    redirects: 1,
    auditLogs: 1,
    versionHistory: 1,
  };

  const paginationHandlers = {};

  function esc(v) {
    return String(v ?? "")
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function getCookieValue(name) {
    const prefix = name + "=";
    const match = document.cookie
      .split(";")
      .map((c) => c.trim())
      .find((c) => c.startsWith(prefix));
    return match ? decodeURIComponent(match.substring(prefix.length)) : "";
  }

  function getOperatorIdentity() {
    const ds = (document.body && document.body.dataset) || {};
    return {
      email: (ds.operatorEmail || "").trim(),
      role: (ds.operatorRole || "").trim(),
      adminPath: (ds.adminPath || "/admin").trim(),
    };
  }

  function showToast(msg, isError) {
    const el = document.getElementById("ops-toast");
    const txt = document.getElementById("ops-toast-msg");
    if (!el || !txt) return;
    el.style.borderLeftColor = isError ? "var(--color-danger)" : "var(--color-success)";
    txt.textContent = String(msg ?? "");
    el.style.display = "flex";
    clearTimeout(window.__msiToastTimer);
    window.__msiToastTimer = setTimeout(() => {
      el.style.display = "none";
    }, 4500);
  }

  /**
   * Render table body with loading / empty / error / data states.
   * @param {HTMLElement} tbody - target <tbody> element
   * @param {string} state - "loading" | "empty" | "error" | "data"
   * @param {Object} opts - { colCount, errorMsg, retryFn, rowsHtml }
   */
  function renderTableState(tbody, state, opts = {}) {
    if (!tbody) return;
    const cols = opts.colCount || 5;
    if (state === "loading") {
      let skel = "";
      for (let i = 0; i < 3; i++) {
        skel += '<tr class="skeleton-row">';
        for (let c = 0; c < cols; c++) skel += "<td></td>";
        skel += "</tr>";
      }
      tbody.innerHTML = skel;
    } else if (state === "empty") {
      tbody.innerHTML = `<tr><td colspan="${cols}" class="empty-state"><i class="fa fa-inbox"></i><p>Veri bulunamadı</p></td></tr>`;
    } else if (state === "error") {
      const msg = esc(opts.errorMsg || "Veri yüklenemedi");
      const retryAttr = typeof opts.retryFn === "function"
        ? `data-admin-action="${opts.retryAction || ""}"` : "";
      tbody.innerHTML = `<tr><td colspan="${cols}" class="error-state"><i class="fa fa-exclamation-triangle"></i><p>${msg}</p><button type="button" class="btn-cmd btn-cmd-sm" ${retryAttr}>Tekrar Dene</button></td></tr>`;
    }
    // "data" state: caller sets innerHTML directly with row markup
  }

  function unwrapData(payload) {
    if (!payload || typeof payload !== "object") return payload;
    if ("status" in payload && "data" in payload && payload.data !== undefined && payload.data !== null) {
      return payload.data;
    }
    return payload;
  }

  function unwrapList(payload, keyHint) {
    if (!payload) return [];
    if (Array.isArray(payload)) return payload;
    if (keyHint && Array.isArray(payload[keyHint])) return payload[keyHint];
    if (Array.isArray(payload.items)) return payload.items;
    if (payload.data !== undefined && payload.data !== null) {
      if (Array.isArray(payload.data)) return payload.data;
      if (keyHint && Array.isArray(payload.data[keyHint])) return payload.data[keyHint];
      if (Array.isArray(payload.data.items)) return payload.data.items;
      if (Array.isArray(payload.data.data)) return payload.data.data;
    }
    return [];
  }

  function extractMeta(payload) {
    if (!payload || typeof payload !== "object") return null;
    const m = payload.meta || (payload.data && payload.data.meta) || (typeof payload.total_pages === "number" ? payload : null);
    if (!m) return null;
    const page = Number(m.page ?? m.current_page ?? 1);
    const totalPages = Number(m.total_pages ?? 1);
    const totalCount = Number(m.total_count ?? m.total ?? 0);
    return {
      page: Number.isFinite(page) && page > 0 ? page : 1,
      total_pages: Number.isFinite(totalPages) && totalPages > 0 ? totalPages : 1,
      total_count: Number.isFinite(totalCount) && totalCount >= 0 ? totalCount : 0,
    };
  }

  async function apiRequest(url, options = {}) {
    const opts = { ...options, credentials: "same-origin" };
    const method = (opts.method || "GET").toUpperCase();
    const headers = { ...(opts.headers || {}) };
    if (["POST", "PUT", "DELETE", "PATCH"].includes(method)) {
      const csrf = getCookieValue("csrf_token");
      if (csrf) headers["X-CSRF-Token"] = csrf;
    }
    opts.headers = headers;

    try {
      const res = await fetch(url, opts);
      let payload = null;
      const text = await res.text();
      if (text) {
        try {
          payload = JSON.parse(text);
        } catch (_) {
          payload = null;
        }
      }
      if (!res.ok) {
        let errorMessage =
          (payload && (payload.message || payload.error)) ||
          (res.status === 401
            ? "Oturum süresi doldu veya kimlik doğrulama gerekli (401)."
            : res.status === 403
              ? "Bu işlem için yetkiniz bulunmuyor (RBAC 403)."
              : `İstek başarısız oldu (HTTP ${res.status}).`);
        return {
          ok: false,
          status: res.status,
          payload,
          data: null,
          meta: null,
          errorMessage,
        };
      }
      return {
        ok: true,
        status: res.status,
        payload,
        data: unwrapData(payload),
        meta: extractMeta(payload),
        errorMessage: "",
      };
    } catch (err) {
      return {
        ok: false,
        status: 0,
        payload: null,
        data: null,
        meta: null,
        errorMessage: "Ağ bağlantı hatası: " + (err && err.message ? err.message : "Sunucuya ulaşılamadı"),
      };
    }
  }

  async function safeFetchJson(url, options = {}) {
    const resp = await apiRequest(url, options);
    return resp.ok ? resp.payload : null;
  }

  function renderPaginationControls(containerId, rawMeta, fetchPageFn) {
    const container = document.getElementById(containerId);
    const meta = rawMeta && typeof rawMeta.total_pages === "number" ? rawMeta : extractMeta({ meta: rawMeta });
    if (!container || !meta || meta.total_pages <= 1) {
      if (container) container.innerHTML = "";
      return;
    }
    paginationHandlers[containerId] = fetchPageFn;
    const prevDisabled = meta.page <= 1 ? "disabled" : "";
    const nextDisabled = meta.page >= meta.total_pages ? "disabled" : "";
    const totalCount = Number(meta.total_count ?? meta.total ?? 0);
    container.innerHTML = `
      <div class="pagination-controls" style="display:flex;align-items:center;gap:12px;padding:12px 0;justify-content:center;">
        <button type="button" class="btn-cmd btn-cmd-sm" ${prevDisabled} data-admin-action="paginate" data-container="${esc(containerId)}" data-page="${meta.page - 1}">&larr; Önceki</button>
        <span style="font-size:12px; color:var(--text-secondary);">Sayfa ${esc(meta.page)} / ${esc(meta.total_pages)} (${esc(totalCount.toLocaleString("tr-TR"))} kayıt)</span>
        <button type="button" class="btn-cmd btn-cmd-sm" ${nextDisabled} data-admin-action="paginate" data-container="${esc(containerId)}" data-page="${meta.page + 1}">Sonraki &rarr;</button>
      </div>
    `;
  }

  function switchTab(tabId) {
    // New sidebar links with ARIA tab pattern
    document.querySelectorAll(".sidebar-link[data-tab]").forEach((item) => {
      const isActive = item.getAttribute("data-tab") === tabId;
      item.classList.toggle("active", isActive);
      if (item.hasAttribute("role")) {
        item.setAttribute("aria-selected", String(isActive));
      }
    });
    // Legacy tab-btn support (backward compat)
    document.querySelectorAll(".tab-btn[data-tab]").forEach((btn) => {
      btn.classList.toggle("active", btn.getAttribute("data-tab") === tabId);
    });
    // Tab panes — support both old (.tabs-content > .tab-pane) and new (.tab-pane) structures
    document.querySelectorAll(".tab-pane").forEach((pane) => {
      pane.classList.toggle("active", pane.id === tabId);
    });

    if (tabId === "tab-17") {
      startMonitoringAutoRefresh();
    } else {
      stopMonitoringAutoRefresh();
    }

    if (tabId === "tab-9") {
      renderAnalyticsCharts();
      refreshSystemHealth();
    } else if (tabId === "tab-14") {
      loadRevenueSummary();
    } else if (tabId === "tab-15") {
      loadUserAnalytics();
    } else if (tabId === "tab-16") {
      loadPendingApprovals();
    }
    window.dispatchEvent(new CustomEvent("msi:admin:tab", { detail: { tabId } }));
  }

  function switchAnalyticsSubview(view) {
    state.activeAnalyticsSubview = view;
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

  function syncDynamicTaxonomyControls() {
    const tagMap = new Map();
    OFFICIAL_SSB_TAGS.forEach((t) => tagMap.set(t.code, t.name_tr));
    (state.ssbTags || []).forEach((t) => {
      if (t && t.code) tagMap.set(t.code, t.name_tr || t.code);
    });
    const mergedTags = Array.from(tagMap.entries()).map(([code, name_tr]) => ({ code, name_tr }));

    const syncCheckboxMatrix = (matrixId, inputName) => {
      const matrix = document.getElementById(matrixId);
      if (!matrix) return;
      const checkedSet = new Set(
        Array.from(matrix.querySelectorAll(`input[name="${inputName}"]:checked`)).map((el) => el.value),
      );
      matrix.innerHTML = mergedTags
        .map(
          (t) =>
            `<label class="check-pill"><input type="checkbox" name="${esc(inputName)}" value="${esc(t.code)}" ${checkedSet.has(t.code) ? "checked" : ""}> <span>${esc(t.name_tr)}</span></label>`,
        )
        .join("");
    };

    syncCheckboxMatrix("c-ssb-matrix", "c_ssb");
    syncCheckboxMatrix("ad-ssb-matrix", "ad_ssb");

    const nlSsbSelect = document.getElementById("nl-ssb-tag");
    if (nlSsbSelect) {
      const currentVal = nlSsbSelect.value || "ALL";
      nlSsbSelect.innerHTML =
        '<option value="ALL">Tüm SSB İlgi Alanları</option>' +
        mergedTags.map((t) => `<option value="${esc(t.code)}">${esc(t.name_tr)}</option>`).join("");
      if ([...nlSsbSelect.options].some((o) => o.value === currentVal)) {
        nlSsbSelect.value = currentVal;
      }
    }

    if (Array.isArray(state.categories) && state.categories.length) {
      const placementMatrix = document.getElementById("c-placements-matrix");
      if (placementMatrix) {
        const existingValues = new Set(
          Array.from(placementMatrix.querySelectorAll('input[name="c_placement"]')).map((el) => el.value),
        );
        const dynamicPlacements = state.categories.filter(
          (c) =>
            c &&
            c.slug &&
            !existingValues.has(c.slug) &&
            (c.is_dynamic || c.group_type === "gruplandirilmis" || c.group_type === "icerikler"),
        );
        if (dynamicPlacements.length) {
          const extraHtml = dynamicPlacements
            .map(
              (c) =>
                `<label class="check-pill"><input type="checkbox" name="c_placement" value="${esc(c.slug)}"> <span>${esc(c.name_tr || c.slug)}</span></label>`,
            )
            .join("");
          placementMatrix.insertAdjacentHTML("beforeend", extraHtml);
        }
      }

      const contentTypeSel = document.getElementById("c-content-type");
      if (contentTypeSel) {
        const existingOpts = new Set(Array.from(contentTypeSel.options).map((o) => o.value));
        state.categories
          .filter((c) => c && c.slug && c.is_dynamic && c.group_type === "icerikler" && !existingOpts.has(c.slug))
          .forEach((c) => {
            const opt = document.createElement("option");
            opt.value = c.slug;
            opt.textContent = `${c.name_tr || c.slug} (${c.slug})`;
            contentTypeSel.appendChild(opt);
          });
      }
    }
  }

  async function loadContentsPage(page) {
    paginationState.contents = page || paginationState.contents || 1;
    const contentsTbody = document.getElementById("tbody-contents");
    renderTableState(contentsTbody, "loading", { colCount: 7 });
    let res = await apiRequest(
      `/api/v1/admin/contents?page=${paginationState.contents}&per_page=100`,
    );
    if (!res.ok) {
      if (res.status === 401 || res.status === 403) {
        showToast("Bu işlem için yetkiniz yetersiz. Yöneticinizle iletişime geçin.", true);
        renderTableState(contentsTbody, "error", { colCount: 7, errorMsg: "Yetki hatası" });
        return;
      }
      res = await apiRequest(`/api/v1/contents?page=${paginationState.contents}&per_page=100`);
      if (!res.ok) {
        renderTableState(contentsTbody, "error", { colCount: 7, errorMsg: res.errorMessage || "İçerikler yüklenemedi", retryAction: "retry-contents" });
        return;
      }
    }
    if (res.ok && res.payload) {
      const list = unwrapList(res.payload, "contents");
      state.contents = list;
      if (res.meta && res.meta.total_count > 0) {
        state.totalContentsCount = res.meta.total_count;
      }
      renderContentsTable();
      renderBulkContentsTable();
      renderKpis();
      renderPaginationControls("pagination-contents", res.meta, loadContentsPage);
    }
  }

  async function refreshAllData() {
    const [
      overviewResp,
      contentsResp,
      adsResp,
      analyticsResp,
      corpReqResp,
      catsResp,
      ssbResp,
      nlResp,
      remResp,
      seoResp,
    ] = await Promise.all([
      apiRequest("/api/v1/admin/overview"),
      apiRequest(`/api/v1/admin/contents?page=${paginationState.contents}&per_page=100`),
      apiRequest("/api/v1/admin/ads"),
      apiRequest("/api/v1/admin/analytics"),
      apiRequest("/api/v1/admin/corporate-requests"),
      apiRequest("/api/v1/admin/categories"),
      apiRequest("/api/v1/admin/ssb-tags"),
      apiRequest(`/api/v1/admin/newsletters?page=${paginationState.newsletters}&per_page=25`),
      apiRequest(`/api/v1/admin/reminders?page=${paginationState.reminders}&per_page=25`),
      apiRequest(`/api/v1/admin/redirects?page=${paginationState.redirects}&per_page=25`),
    ]);

    if (overviewResp.ok && overviewResp.data) {
      const d = overviewResp.data;
      state.overviewStats = d;
      if (typeof d.total_contents === "number") {
        state.totalContentsCount = d.total_contents;
      }
      if (Array.isArray(d.contents) && d.contents.length) state.contents = d.contents;
      if (Array.isArray(d.ads) && d.ads.length) state.ads = d.ads;
      if (Array.isArray(d.content_analytics)) state.contentAnalytics = d.content_analytics;
      if (Array.isArray(d.ad_analytics)) state.adAnalytics = d.ad_analytics;
      if (Array.isArray(d.reader_analytics)) state.readerAnalytics = d.reader_analytics;
      if (Array.isArray(d.corporate_requests)) state.corporateRequests = d.corporate_requests;
      if (Array.isArray(d.categories)) state.categories = d.categories;
      if (Array.isArray(d.ssb_tags)) state.ssbTags = d.ssb_tags;
    }

    if (contentsResp.ok && contentsResp.payload) {
      state.contents = unwrapList(contentsResp.payload, "contents");
      if (contentsResp.meta && contentsResp.meta.total_count > 0) {
        state.totalContentsCount = Math.max(state.totalContentsCount, contentsResp.meta.total_count);
      }
      renderPaginationControls("pagination-contents", contentsResp.meta, loadContentsPage);
    } else if (!state.contents.length) {
      if (contentsResp.status === 401 || contentsResp.status === 403) {
        showToast("Bu işlem için yetkiniz yetersiz. Yöneticinizle iletişime geçin.", true);
      } else {
        const pubContents = await apiRequest("/api/v1/contents?per_page=100");
        if (pubContents.ok && pubContents.payload) {
          state.contents = unwrapList(pubContents.payload, "contents");
          if (pubContents.meta && pubContents.meta.total_count > 0) {
            state.totalContentsCount = Math.max(state.totalContentsCount, pubContents.meta.total_count);
          }
        }
      }
    }

    if (adsResp.ok && adsResp.payload) {
      state.ads = unwrapList(adsResp.payload, "ads");
    } else if (!state.ads.length) {
      if (adsResp.status === 401 || adsResp.status === 403) {
        showToast("Bu işlem için yetkiniz yetersiz. Yöneticinizle iletişime geçin.", true);
      } else {
        const pubAds = await apiRequest("/api/v1/ads");
        if (pubAds.ok && pubAds.payload) state.ads = unwrapList(pubAds.payload, "ads");
      }
    }

    if (analyticsResp.ok && analyticsResp.data) {
      const d = analyticsResp.data;
      if (Array.isArray(d.contents)) state.contentAnalytics = d.contents;
      if (Array.isArray(d.content_reports)) state.contentAnalytics = d.content_reports;
      if (Array.isArray(d.content_analytics)) state.contentAnalytics = d.content_analytics;
      if (Array.isArray(d.ads)) state.adAnalytics = d.ads;
      if (Array.isArray(d.ad_reports)) state.adAnalytics = d.ad_reports;
      if (Array.isArray(d.ad_analytics)) state.adAnalytics = d.ad_analytics;
      if (Array.isArray(d.readers)) state.readerAnalytics = d.readers;
      if (Array.isArray(d.reader_reports)) state.readerAnalytics = d.reader_reports;
      if (Array.isArray(d.reader_analytics)) state.readerAnalytics = d.reader_analytics;
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

    if (corpReqResp.ok && corpReqResp.payload) {
      state.corporateRequests = unwrapList(corpReqResp.payload, "requests");
    }
    if (catsResp.ok && catsResp.payload) {
      state.categories = unwrapList(catsResp.payload, "categories");
    } else if (!state.categories.length) {
      if (catsResp.status === 401 || catsResp.status === 403) {
        showToast("Bu işlem için yetkiniz yetersiz. Yöneticinizle iletişime geçin.", true);
      } else {
        const pubCats = await safeFetchJson("/api/v1/categories");
        if (pubCats) state.categories = unwrapList(pubCats, "categories");
      }
    }
    if (ssbResp.ok && ssbResp.payload) {
      state.ssbTags = unwrapList(ssbResp.payload, "ssb_tags");
    } else if (!state.ssbTags.length) {
      if (ssbResp.status === 401 || ssbResp.status === 403) {
        showToast("Bu işlem için yetkiniz yetersiz. Yöneticinizle iletişime geçin.", true);
      } else {
        const pubSsb = await safeFetchJson("/api/v1/ssb-tags");
        if (pubSsb) state.ssbTags = unwrapList(pubSsb, "ssb_tags");
      }
    }

    if (nlResp.ok && nlResp.payload) {
      state.newsletters = unwrapList(nlResp.payload, "campaigns");
      renderPaginationControls("pagination-newsletters", nlResp.meta, loadNewsletters);
    }
    if (remResp.ok && remResp.payload) {
      state.reminders = unwrapList(remResp.payload, "reminders");
      renderPaginationControls("pagination-reminders", remResp.meta, loadReminders);
    }
    if (seoResp.ok && seoResp.payload) {
      state.redirects = unwrapList(seoResp.payload, "redirects");
      renderPaginationControls("pagination-redirects", seoResp.meta, loadRedirects);
    }

    syncDynamicTaxonomyControls();
    renderKpis();
    renderContentsTable();
    renderAdsTable();
    renderAnalyticsTables();
    renderCorporateRequestsTable();
    renderTaxonomyTables();
    renderNewslettersTable();
    renderRemindersAndRedirects();
    renderBulkContentsTable();

    await Promise.all([
      refreshCentralEnginesTab(),
      refreshSystemHealth(),
      loadMediaLibrary(),
      loadUserRoles(),
      loadAuditLogs(),
      loadCustomFields(),
      loadCacheOperations(),
      loadRevenueSummary(),
      loadUserAnalytics(),
      loadPendingApprovals(),
    ]);
    renderAnalyticsCharts();
    await loadAdminUsers();
    window.dispatchEvent(new CustomEvent("msi:admin:refreshed"));
  }

  function renderKpis() {
    const ov = state.overviewStats || {};
    const totalContents =
      (typeof ov.total_contents === "number" && ov.total_contents > 0
        ? ov.total_contents
        : state.totalContentsCount) || state.contents.length;

    const samplePlacements = state.contents.reduce(
      (acc, c) => acc + ((c.placements && c.placements.length) || 1),
      0,
    );
    const estPlacements =
      state.contents.length > 0 && totalContents > state.contents.length
        ? Math.round((samplePlacements / state.contents.length) * totalContents)
        : samplePlacements;

    const kpiContents = document.getElementById("kpi-total-contents");
    const kpiPlacementsSub = document.getElementById("kpi-placements-sub");
    if (kpiContents) kpiContents.textContent = totalContents.toLocaleString("tr-TR") + " İçerik";
    if (kpiPlacementsSub) {
      kpiPlacementsSub.textContent =
        estPlacements.toLocaleString("tr-TR") + " çoklu bölüm gösterimi (Single Source of Truth)";
    }

    const totalViews =
      typeof ov.total_views === "number"
        ? ov.total_views
        : state.contents.reduce((acc, c) => acc + (c.view_count || 0), 0);
    const totalTrue =
      typeof ov.total_true_reads === "number"
        ? ov.total_true_reads
        : state.contents.reduce((acc, c) => acc + (c.true_read_count || 0), 0);
    const ratio = totalViews > 0 ? Math.round((totalTrue / totalViews) * 100) : 0;

    const kpiTrue = document.getElementById("kpi-true-reads");
    const kpiViewsSub = document.getElementById("kpi-views-sub");
    if (kpiTrue) kpiTrue.textContent = totalTrue.toLocaleString("tr-TR") + " (" + ratio + "%)";
    if (kpiViewsSub) {
      kpiViewsSub.textContent = "Toplam " + totalViews.toLocaleString("tr-TR") + " sayfa görüntülenme";
    }

    const activeAds =
      typeof ov.active_ads === "number"
        ? ov.active_ads
        : state.ads.filter((a) => a.is_active !== false).length;
    const totalImps = state.ads.reduce((acc, a) => acc + (a.impressions_count || 0), 0);
    const kpiAds = document.getElementById("kpi-active-ads");
    const kpiAdsSub = document.getElementById("kpi-ads-sub");
    if (kpiAds) kpiAds.textContent = activeAds.toLocaleString("tr-TR") + " Kampanya";
    if (kpiAdsSub) kpiAdsSub.textContent = totalImps.toLocaleString("tr-TR") + " kurumsal hedefli gösterim";

    const pendingReqs =
      typeof ov.pending_corporate_requests === "number"
        ? ov.pending_corporate_requests
        : state.corporateRequests.filter((r) => r.status === "pending").length;
    const kpiGov = document.getElementById("kpi-gov-requests");
    const kpiGovSub = document.getElementById("kpi-gov-sub");
    if (kpiGov) kpiGov.textContent = pendingReqs.toLocaleString("tr-TR") + " Bekleyen";
    if (kpiGovSub) {
      kpiGovSub.textContent = "Toplam " + state.corporateRequests.length + " kurumsal doğrulama kaydı";
    }
  }

  // =========================================================================
  // TAB 1: CONTENT SSOT & EDITORIAL WORKFLOW
  // =========================================================================

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
        (c.status || "").toLowerCase().includes(q) ||
        String(c.issue_number || "").includes(q) ||
        (c.placements || []).join(" ").toLowerCase().includes(q) ||
        (c.ssb_tags || []).join(" ").toLowerCase().includes(q)
      );
    });

    if (!filtered.length) {
      renderTableState(tbody, "empty", { colCount: 7 });
      return;
    }

    const statusLabels = {
      published: "Yayında",
      draft: "Taslak",
      pending_review: "Onay Bekliyor",
      rejected: "Reddedildi",
      archived: "Arşiv",
    };

    tbody.innerHTML = filtered
      .map((c) => {
        const placementsHtml = (c.placements || [])
          .map((p) => `<span class="badge-tag badge-placement">${esc(p)}</span>`)
          .join("");
        const ssbHtml = (c.ssb_tags || [])
          .map((t) => `<span class="badge-tag badge-ssb">${esc(t)}</span>`)
          .join("");
        const vis = Number(c.visibility_level || 0);
        const pwLabels = {
          0: "0: Açık Erişim",
          1: "1: Kayıtlı & Abone",
          2: `2: Abone / ${esc(c.single_price_try || 0)} ₺`,
        };
        const pwBadge = `<span class="badge-tag badge-paywall-${vis}">${pwLabels[vis] || esc(vis)}</span>`;
        const spBadge = c.is_sponsored
          ? `<div style="margin-top:4px;"><span class="badge-tag badge-sponsored"><i class="fa fa-shield"></i> ${esc(c.sponsor_legal_label || "SPONSORLU")}: ${esc(c.sponsor_brand || "")}</span></div>`
          : "";

        const stCode = c.status || "published";
        const stBadge = `<span class="badge-tag badge-status-${esc(stCode)}">${esc(statusLabels[stCode] || stCode)}</span>`;
        const canSubmitReview = stCode !== "pending_review";

        return `<tr>
          <td style="font-family:var(--font-mono); font-size:11.5px;">
            <strong>#${esc(c.id)}</strong><br>
            <span style="color:var(--text-muted);">${c.issue_number ? "Sayı " + esc(c.issue_number) : "Online"}</span>
          </td>
          <td>
            <div style="font-weight:700; color:var(--text-main); margin-bottom:2px;">
              ${c.is_featured ? '<i class="fa fa-star" style="color:#f59e0b;" title="Manşet"></i> ' : ""}
              <a href="/icerik/${esc(c.slug)}" target="_blank" style="color:var(--text-main); text-decoration:none;">${esc(c.title_tr)}</a>
            </div>
            <div style="font-size:11.5px; color:var(--text-muted); margin-bottom:4px;">${esc(c.title_en || "")}</div>
            ${stBadge}
            <span class="badge-tag">${esc(c.content_type)}</span>
            <span class="badge-tag" style="background:rgba(148,163,184,0.12); color:var(--text-secondary); border-color:transparent;">${esc(c.source_group)}</span>
          </td>
          <td>${placementsHtml || '<span style="color:var(--text-muted);">—</span>'}</td>
          <td>${ssbHtml || '<span style="color:var(--text-muted);">—</span>'}</td>
          <td>${pwBadge}${spBadge}</td>
          <td style="font-family:var(--font-mono);">
            <div style="font-weight:700;">${Number(c.view_count || 0).toLocaleString("tr-TR")} görüntülenme</div>
            <div style="color:var(--color-success); font-size:11.5px;">${Number(c.true_read_count || 0).toLocaleString("tr-TR")} gerçek okuma</div>
          </td>
          <td>
            <div style="display:flex; gap:5px; flex-wrap:wrap;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-content" data-id="${esc(c.id)}">
                <i class="fa fa-pencil"></i> Düzenle
              </button>
              ${
                canSubmitReview
                  ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-gold" data-admin-action="submit-content-review" data-id="${esc(c.id)}" title="Editöryal Onaya Gönder">
                      <i class="fa fa-paper-plane"></i> Onaya Gönder
                    </button>`
                  : ""
              }
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-content" data-id="${esc(c.id)}" title="Sil">
                <i class="fa fa-trash"></i>
              </button>
            </div>
          </td>
        </tr>`;
      })
      .join("");
  }

  function loadContentIntoForm(id) {
    const numId = Number(id);
    const c = state.contents.find((x) => Number(x.id) === numId);
    if (!c) return;
    document.getElementById("c-id").value = c.id;
    document.getElementById("c-slug").value = c.slug || "";
    document.getElementById("c-title-tr").value = c.title_tr || "";
    document.getElementById("c-title-en").value = c.title_en || "";
    document.getElementById("c-summary-tr").value = c.summary_tr || "";
    document.getElementById("c-summary-en").value = c.summary_en || "";
    document.getElementById("c-body-tr").value = c.body_tr || "";
    document.getElementById("c-body-en").value = c.body_en || "";
    document.getElementById("c-content-type").value = c.content_type || "haberler";
    document.getElementById("c-source-group").value = c.source_group || "gunluk-haber";
    const stEl = document.getElementById("c-status");
    if (stEl) stEl.value = c.status || "published";
    document.getElementById("c-issue-number").value = c.issue_number || "";
    document.getElementById("c-visibility-level").value = String(c.visibility_level || 0);
    document.getElementById("c-single-price").value = c.single_price_try || 0;
    document.getElementById("c-read-time").value = c.read_time_min || 5;
    document.getElementById("c-author-name").value = c.author_name || "MSI Editöryal Masası";
    const atEl = document.getElementById("c-author-title");
    if (atEl) atEl.value = c.author_title || "";
    const pubEl = document.getElementById("c-published-at");
    if (pubEl) pubEl.value = c.published_at || "";
    document.getElementById("c-cover-image").value = c.cover_image || "images/trending/trending1.jpg";
    document.getElementById("c-video-url").value = c.video_url || "";
    document.getElementById("c-is-sponsored").checked = !!c.is_sponsored;
    document.getElementById("c-is-featured").checked = !!c.is_featured;
    document.getElementById("c-sponsor-brand").value = c.sponsor_brand || "";
    document.getElementById("c-sponsor-legal-label").value = c.sponsor_legal_label || "SPONSORLU İÇERİK";
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
    const form = document.getElementById("form-content-ssot");
    if (form) form.reset();
    document.getElementById("c-id").value = "0";
    const stEl = document.getElementById("c-status");
    if (stEl) stEl.value = "published";
    const pubEl = document.getElementById("c-published-at");
    if (pubEl) pubEl.value = "";
    const atEl = document.getElementById("c-author-title");
    if (atEl) atEl.value = "";
    document.getElementById("content-form-heading").textContent =
      "Hızlı İçerik Girişi (Single Source of Truth)";
    toggleSponsoredFields();
  }

  function buildContentPayload(statusOverride) {
    const placements = Array.from(
      document.querySelectorAll('input[name="c_placement"]:checked'),
    ).map((el) => el.value);
    const ssb_tags = Array.from(
      document.querySelectorAll('input[name="c_ssb"]:checked'),
    ).map((el) => el.value);
    const isSponsored = document.getElementById("c-is-sponsored").checked;
    const issueRaw = document.getElementById("c-issue-number").value;
    const editId = parseInt(document.getElementById("c-id").value || "0", 10);
    const existingItem = editId > 0 ? state.contents.find((x) => Number(x.id) === editId) : null;
    const statusVal =
      statusOverride ||
      (document.getElementById("c-status") ? document.getElementById("c-status").value : "published");
    const pubInput = (document.getElementById("c-published-at")?.value || "").trim();
    const authorTitleInput = (document.getElementById("c-author-title")?.value || "").trim();

    return {
      id: editId,
      slug: document.getElementById("c-slug").value.trim(),
      title_tr: document.getElementById("c-title-tr").value.trim(),
      title_en: document.getElementById("c-title-en").value.trim(),
      summary_tr: document.getElementById("c-summary-tr").value.trim(),
      summary_en: document.getElementById("c-summary-en").value.trim(),
      body_tr: document.getElementById("c-body-tr").value.trim(),
      body_en: document.getElementById("c-body-en").value.trim(),
      content_type: document.getElementById("c-content-type").value,
      source_group: document.getElementById("c-source-group").value,
      cover_image:
        document.getElementById("c-cover-image").value.trim() ||
        "images/msi/dergi-286.webp",
      author_name:
        document.getElementById("c-author-name").value.trim() ||
        "MSI Editöryal Masası",
      author_title:
        authorTitleInput ||
        (existingItem
          ? existingItem.author_title || "Savunma Sanayii Analisti"
          : "Savunma Sanayii Analisti"),
      issue_number: issueRaw ? parseInt(issueRaw, 10) : null,
      published_at:
        pubInput ||
        (existingItem
          ? existingItem.published_at || new Date().toISOString().slice(0, 16).replace("T", " ")
          : new Date().toISOString().slice(0, 16).replace("T", " ")),
      read_time_min: parseInt(document.getElementById("c-read-time").value || "5", 10),
      visibility_level: parseInt(document.getElementById("c-visibility-level").value || "0", 10),
      single_price_try: parseFloat(document.getElementById("c-single-price").value || "0"),
      is_sponsored: isSponsored,
      sponsor_brand: isSponsored
        ? document.getElementById("c-sponsor-brand").value.trim() || "Savunma Sanayii Paydaşı"
        : null,
      sponsor_legal_label: isSponsored
        ? document.getElementById("c-sponsor-legal-label").value
        : null,
      is_featured: document.getElementById("c-is-featured").checked,
      video_url: document.getElementById("c-video-url").value.trim() || null,
      view_count: existingItem ? existingItem.view_count || 1 : 1,
      true_read_count: existingItem ? existingItem.true_read_count || 1 : 1,
      placements,
      ssb_tags,
      status: statusVal,
    };
  }

  async function submitContentForm(e) {
    if (e) e.preventDefault();
    const payload = buildContentPayload();
    if (!payload.title_tr || !payload.summary_tr || !payload.body_tr) {
      showToast("Lütfen zorunlu başlık, özet ve gövde alanlarını doldurun.", true);
      return false;
    }

    const resp = await apiRequest("/api/v1/admin/contents", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (resp.ok) {
      const savedId = (resp.data && resp.data.id) || payload.id;
      if (payload.status === "pending_review" && savedId) {
        await apiRequest(`/api/v1/admin/contents/${encodeURIComponent(savedId)}/submit-review`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({}),
        });
      }
      showToast(
        payload.status === "draft"
          ? "İçerik taslak olarak kaydedildi."
          : payload.status === "pending_review"
            ? "İçerik kaydedildi ve editöryal onaya gönderildi!"
            : "İçerik başarıyla kaydedildi ve " + payload.placements.length + " bölümde yayına alındı!",
      );
      resetContentForm();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function submitFormAndSendForReview() {
    const stEl = document.getElementById("c-status");
    if (stEl) stEl.value = "draft";
    const payload = buildContentPayload("draft");
    if (!payload.title_tr || !payload.summary_tr || !payload.body_tr) {
      showToast("Onaya göndermeden önce başlık, özet ve içerik gövdesini doldurun.", true);
      return;
    }
    const saveResp = await apiRequest("/api/v1/admin/contents", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!saveResp.ok) {
      showToast(saveResp.errorMessage, true);
      return;
    }
    const savedId = (saveResp.data && saveResp.data.id) || payload.id;
    if (!savedId) {
      showToast("İçerik kaydedildi ancak ID alınamadı.", true);
      return;
    }
    await submitContentForReview(savedId, true);
    resetContentForm();
  }

  async function submitContentForReview(contentId, silentConfirm) {
    if (!silentConfirm && !confirm(`#${contentId} numaralı içerik editöryal onaya gönderilsin mi?`)) {
      return;
    }
    const resp = await apiRequest(`/api/v1/admin/contents/${encodeURIComponent(contentId)}/submit-review`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    if (resp.ok) {
      showToast(`İçerik #${contentId} editöryal onaya gönderildi (Onay Bekleyenler sekmesi).`);
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function deleteContent(id) {
    if (!confirm(`#${id} numaralı içerik silinecek. Emin misiniz?`)) return;
    const resp = await apiRequest("/api/v1/admin/contents/" + encodeURIComponent(id), {
      method: "DELETE",
    });
    if (resp.ok) {
      showToast("İçerik #" + id + " silindi.");
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  // =========================================================================
  // TAB 2: SMART AD CAMPAIGNS, IMAGE UPLOAD, EDIT & DELETE
  // =========================================================================

  async function checkAdConflictLive() {
    const slotEl = document.getElementById("ad-slot-code");
    if (!slotEl) return;
    const slot_code = slotEl.value;
    const target_institutions = Array.from(
      document.querySelectorAll('input[name="ad_inst"]:checked'),
    ).map((el) => el.value);
    const target_ssb_tags = Array.from(
      document.querySelectorAll('input[name="ad_ssb"]:checked'),
    ).map((el) => el.value);
    const sponsor_name =
      document.getElementById("ad-sponsor-name")?.value.trim() || "Yeni Kampanya";

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

    const warning = unwrapData(res);
    if (warning && typeof warning.has_conflict === "boolean") {
      banner.style.display = "block";
      const conflicts = (warning.conflicting_campaigns || []).map((c) => esc(c)).join(", ");
      if (warning.has_conflict) {
        banner.classList.add("conflict-danger");
        textEl.innerHTML = `<strong>REKLAM KOTA / ÇAKIŞMA UYARISI (${esc(warning.current_count)}/${esc(warning.quota_limit)}):</strong> ${esc(warning.message_tr)}<br>
          <span style="font-size:11.5px;">Çakışan Kampanyalar: ${conflicts || "—"}</span>`;
      } else {
        banner.classList.remove("conflict-danger");
        textEl.innerHTML = `<strong>ÇAKIŞMA YOK (${esc(warning.current_count)}/${esc(warning.quota_limit)}):</strong> ${esc(warning.message_tr || "Seçilen slot ve SSB etiketlerinde kota müsait.")}`;
      }
    }
  }

  function loadAdIntoForm(id) {
    const numId = Number(id);
    const a = state.ads.find((x) => Number(x.id) === numId);
    if (!a) return;
    document.getElementById("ad-id").value = a.id;
    document.getElementById("ad-slot-code").value = a.slot_code || "ad-mag-top";
    document.getElementById("ad-sponsor-type").value = a.sponsor_type || "defense_oem";
    document.getElementById("ad-sponsor-name").value = a.sponsor_name || "";
    document.getElementById("ad-dimensions").value = a.dimensions || "970x90";
    document.getElementById("ad-budget-score").value = a.budget_score || 85;
    document.getElementById("ad-title-tr").value = a.title_tr || "";
    document.getElementById("ad-title-en").value = a.title_en || "";
    document.getElementById("ad-desc-tr").value = a.desc_tr || "";
    document.getElementById("ad-desc-en").value = a.desc_en || "";
    document.getElementById("ad-target-url").value = a.target_url || "https://www.msidergisi.com";
    const imgEl = document.getElementById("ad-image-url");
    if (imgEl) imgEl.value = a.image_url || "";
    document.getElementById("ad-badge-label").value = a.badge_label || "SPONSORLU REKLAM ALANI";
    document.getElementById("ad-pill-style").value = a.pill_style || "blue";
    document.getElementById("ad-target-categories").value = (a.target_categories || []).join(", ");
    document.getElementById("ad-is-fallback").checked = !!a.is_default_fallback;
    document.getElementById("ad-is-popup").checked = !!a.is_floating_popup;
    document.getElementById("ad-is-newsletter").checked = !!a.is_newsletter_ad;
    document.getElementById("ad-is-active").checked = a.is_active !== false;

    const instSet = new Set(a.target_institutions || []);
    document.querySelectorAll('input[name="ad_inst"]').forEach((chk) => {
      chk.checked = instSet.has(chk.value);
    });
    const ssbSet = new Set(a.target_ssb_tags || []);
    document.querySelectorAll('input[name="ad_ssb"]').forEach((chk) => {
      chk.checked = ssbSet.has(chk.value);
    });

    const heading = document.getElementById("ad-form-heading");
    if (heading) heading.textContent = `Reklam Kampanyası Düzenle (#${a.id} — ${a.sponsor_name})`;
    showToast(`Reklam kampanyası #${a.id} düzenleme formuna yüklendi.`);
  }

  function resetAdForm() {
    const form = document.getElementById("form-ad-campaign");
    if (form) form.reset();
    const idEl = document.getElementById("ad-id");
    if (idEl) idEl.value = "0";
    const heading = document.getElementById("ad-form-heading");
    if (heading) heading.textContent = "Akıllı Reklam Kampanyası & Kurumsal Hedefleme";
  }

  async function submitAdForm(e) {
    if (e) e.preventDefault();
    const editId = parseInt(document.getElementById("ad-id").value || "0", 10);
    const existingAd = editId > 0 ? state.ads.find((x) => Number(x.id) === editId) : null;

    const target_institutions = Array.from(
      document.querySelectorAll('input[name="ad_inst"]:checked'),
    ).map((el) => el.value);
    const target_ssb_tags = Array.from(
      document.querySelectorAll('input[name="ad_ssb"]:checked'),
    ).map((el) => el.value);
    const target_categories = (document.getElementById("ad-target-categories").value || "")
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean);
    const imageUrlRaw = (document.getElementById("ad-image-url")?.value || "").trim();

    const payload = {
      id: editId,
      slot_code: document.getElementById("ad-slot-code").value,
      badge_label:
        document.getElementById("ad-badge-label").value.trim() || "SPONSORLU REKLAM ALANI",
      dimensions: document.getElementById("ad-dimensions").value.trim() || "970x90",
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
      image_url: imageUrlRaw || null,
      budget_score: parseInt(document.getElementById("ad-budget-score").value || "80", 10),
      is_default_fallback: document.getElementById("ad-is-fallback").checked,
      is_floating_popup: document.getElementById("ad-is-popup").checked,
      is_newsletter_ad: document.getElementById("ad-is-newsletter").checked,
      target_categories,
      target_ssb_tags,
      target_institutions,
      impressions_count: existingAd ? existingAd.impressions_count || 0 : 1,
      clicks_count: existingAd ? existingAd.clicks_count || 0 : 0,
      is_active: document.getElementById("ad-is-active").checked,
    };

    const resp = await apiRequest("/api/v1/admin/ads", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (resp.ok) {
      showToast(
        editId > 0
          ? `Reklam kampanyası #${editId} güncellendi!`
          : "Reklam kampanyası kaydedildi ve hedefleme motoruna eklendi!",
      );
      resetAdForm();
      await checkAdConflictLive();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function deleteAd(id) {
    if (!confirm(`#${id} numaralı reklam kampanyası silinecek. Emin misiniz?`)) return;
    const resp = await apiRequest(`/api/v1/admin/ads/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
    if (resp.ok) {
      showToast(`Reklam kampanyası #${id} silindi.`);
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  function renderAdsTable() {
    const tbody = document.getElementById("tbody-ads");
    if (!tbody) return;
    if (!state.ads.length) {
      renderTableState(tbody, "empty", { colCount: 7 });
      return;
    }
    tbody.innerHTML = state.ads
      .map((a) => {
        const instBadges = (a.target_institutions || [])
          .map((i) => `<span class="badge-tag">${esc(i)}</span>`)
          .join("");
        const ssbBadges = (a.target_ssb_tags || [])
          .map((t) => `<span class="badge-tag badge-ssb">${esc(t)}</span>`)
          .join("");
        const imps = Number(a.impressions_count || 0);
        const clicks = Number(a.clicks_count || 0);
        const ctr = imps > 0 ? ((clicks / imps) * 100).toFixed(2) : "0.00";
        const activeBadge =
          a.is_active === false
            ? '<span class="badge-tag badge-status-draft">Pasif</span>'
            : '<span class="badge-tag badge-status-published">Aktif</span>';
        const imgTag = a.image_url
          ? `<div style="margin-top:4px; font-size:10.5px; color:var(--color-info); font-family:var(--font-mono);"><i class="fa fa-image"></i> ${esc(a.image_url)}</div>`
          : "";
        return `<tr>
          <td style="font-family:var(--font-mono); font-size:11.5px;">
            <strong>${esc(a.slot_code)}</strong><br>
            <span style="color:var(--text-muted);">${esc(a.dimensions)}</span><br>
            ${activeBadge}
          </td>
          <td>
            <div style="font-weight:700;">${esc(a.sponsor_name)}</div>
            <span class="badge-tag ${a.sponsor_type === "defense_oem" ? "badge-sponsored" : "badge-paywall-0"}">${esc(a.sponsor_type)}</span>
          </td>
          <td>
            <div style="font-weight:600;">${esc(a.title_tr)}</div>
            <div style="font-size:11px; color:var(--text-muted);">Bütçe Puanı: <strong>${esc(a.budget_score)}</strong> ${a.is_default_fallback ? "• [Fallback]" : ""}</div>
            ${imgTag}
          </td>
          <td>${instBadges || '<span style="color:var(--text-muted);">Tüm Kurumlar</span>'}</td>
          <td>${ssbBadges || '<span style="color:var(--text-muted);">Genel</span>'}</td>
          <td style="font-family:var(--font-mono);">
            <div>${imps.toLocaleString("tr-TR")} gösterim</div>
            <div style="color:#0ea5e9;">${clicks.toLocaleString("tr-TR")} tık (${ctr}%)</div>
          </td>
          <td>
            <div style="display:flex; gap:5px;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-ad" data-id="${esc(a.id)}" title="Düzenle">
                <i class="fa fa-pencil"></i>
              </button>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-ad" data-id="${esc(a.id)}" title="Sil">
                <i class="fa fa-trash"></i>
              </button>
            </div>
          </td>
        </tr>`;
      })
      .join("");
  }

  // =========================================================================
  // TAB 3: 3D B2B ANALYTICS + JSON & CSV EXPORT
  // =========================================================================

  function renderInstitutionBars(list, isCyan) {
    if (!Array.isArray(list) || !list.length) {
      return '<span style="color:var(--text-muted); font-size:11.5px;">Veri toplanıyor</span>';
    }
    return list
      .map((item) => {
        const pctNum = typeof item.percentage === "number" ? item.percentage : 0;
        const pct = pctNum.toFixed(1);
        const widthPct = Math.min(100, Math.max(4, pctNum));
        return `<div class="inst-bar-row">
          <div class="inst-bar-meta">
            <span><strong>${esc(item.organization_category)}</strong></span>
            <span>${esc(item.count)} (${esc(pct)}%)</span>
          </div>
          <div class="inst-bar-track">
            <div class="inst-bar-fill ${isCyan ? "cyan" : ""}" style="width:${widthPct}%;"></div>
          </div>
        </div>`;
      })
      .join("");
  }

  function renderAnalyticsTables() {
    const tbodyContent = document.getElementById("tbody-analytics-content");
    if (tbodyContent) {
      if (state.contentAnalytics.length) {
        tbodyContent.innerHTML = state.contentAnalytics
          .map(
            (r) => `<tr>
              <td>
                <div style="font-weight:700;">${esc(r.title_tr)}</div>
                <div style="font-family:var(--font-mono); font-size:11px; color:var(--text-muted);">/${esc(r.content_slug || r.slug || "")}</div>
              </td>
              <td style="font-family:var(--font-mono); font-size:14px; font-weight:700;">${Number(r.total_views || 0).toLocaleString("tr-TR")}</td>
              <td style="font-family:var(--font-mono); font-size:14px; font-weight:700; color:var(--color-success);">${Number(r.true_reads ?? r.true_read_count ?? 0).toLocaleString("tr-TR")}</td>
              <td style="font-family:var(--font-mono);">
                <div><i class="fa fa-clock-o"></i> ${Math.round(r.avg_duration_sec || 0)} sn</div>
                <div style="color:#0ea5e9;"><i class="fa fa-sort"></i> %${Math.round(r.avg_scroll_pct || 0)} scroll</div>
              </td>
              <td>${renderInstitutionBars(r.institution_breakdown, false)}</td>
            </tr>`,
          )
          .join("");
      } else {
        renderTableState(tbodyContent, "empty", { colCount: 5 });
      }
    }

    const tbodyAd = document.getElementById("tbody-analytics-ad");
    if (tbodyAd) {
      if (state.adAnalytics.length) {
        tbodyAd.innerHTML = state.adAnalytics
          .map(
            (a) => `<tr>
              <td>
                <div style="font-weight:700;">${esc(a.sponsor_name)}</div>
                <span class="badge-tag">${esc(a.slot_code)}</span>
              </td>
              <td style="font-family:var(--font-mono);">
                <div><strong>${Number(a.impressions || 0).toLocaleString("tr-TR")}</strong> gösterim</div>
                <div style="color:var(--color-success);"><strong>${Number(a.clicks || 0).toLocaleString("tr-TR")}</strong> tıklama</div>
                <div style="color:#f59e0b;">CTR: %${Number(a.ctr_pct || 0).toFixed(2)}</div>
              </td>
              <td>${renderInstitutionBars(a.impression_institutions, false)}</td>
              <td>${renderInstitutionBars(a.click_institutions, true)}</td>
            </tr>`,
          )
          .join("");
      } else {
        renderTableState(tbodyAd, "empty", { colCount: 4 });
      }
    }

    const tbodyReader = document.getElementById("tbody-analytics-reader");
    if (tbodyReader) {
      if (state.readerAnalytics.length) {
        tbodyReader.innerHTML = state.readerAnalytics
          .map(
            (u) => `<tr>
              <td>
                <div style="font-weight:700;">${esc(u.full_name)}</div>
                <div style="font-size:11.5px; color:var(--text-muted);">${esc(u.email)}</div>
              </td>
              <td>
                <span class="badge-tag">${esc(u.organization_category)}</span>
                <div style="font-size:11.5px; color:var(--text-secondary); margin-top:3px;">${esc(u.organization_name)}</div>
              </td>
              <td><span class="badge-tag badge-paywall-0">${esc(u.subscription_tier)}</span></td>
              <td>${(u.articles_read || []).map((slug) => `<span class="badge-tag badge-placement">${esc(slug)}</span>`).join("")}</td>
              <td>${(u.ssb_keywords_engaged || []).map((k) => `<span class="badge-tag badge-ssb">${esc(k)}</span>`).join("")}</td>
              <td style="font-family:var(--font-mono); font-size:11.5px;">
                <div>${Number(u.ad_impressions_count || 0)} gösterim / <strong>${Number(u.ad_clicks_count || 0)} tık</strong></div>
                <div style="margin-top:3px;">${(u.clicked_ads || []).map((ad) => `<span class="badge-tag badge-sponsored">${esc(ad)}</span>`).join("")}</div>
              </td>
            </tr>`,
          )
          .join("");
      } else {
        renderTableState(tbodyReader, "empty", { colCount: 6 });
      }
    }
  }

  function downloadMonthlyConsolidatedReport() {
    const totalContents =
      (state.overviewStats && state.overviewStats.total_contents) ||
      state.totalContentsCount ||
      state.contents.length;
    const reportPayload = {
      generated_at: new Date().toISOString(),
      generated_by: getOperatorIdentity().email || "authenticated_operator",
      platform: "UMSA Prodüksiyon — MSI Dergisi & MSI TDR B2B Kurumsal Analitik Merkezi",
      summary: {
        total_contents: totalContents,
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
      "MSI_Konsolide_Kurumsal_Analitik_Raporu_" +
      new Date().toISOString().slice(0, 10) +
      ".json";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast("Konsolide B2B Analitik JSON raporu indirildi!");
  }

  function exportAnalyticsCsv() {
    const csvCell = (val) => `"${String(val ?? "").replace(/"/g, '""')}"`;
    let rows = [];
    if (state.activeAnalyticsSubview === "ad") {
      rows.push(["Sponsor", "Slot", "Gosterim", "Tiklama", "CTR_Pct"]);
      state.adAnalytics.forEach((a) => {
        rows.push([a.sponsor_name, a.slot_code, a.impressions || 0, a.clicks || 0, (a.ctr_pct || 0).toFixed(2)]);
      });
    } else if (state.activeAnalyticsSubview === "reader") {
      rows.push(["Ad_Soyad", "Eposta", "Kurum_Kategorisi", "Kurum_Adi", "Abonelik", "Reklam_Tik"]);
      state.readerAnalytics.forEach((u) => {
        rows.push([
          u.full_name,
          u.email,
          u.organization_category,
          u.organization_name,
          u.subscription_tier,
          u.ad_clicks_count || 0,
        ]);
      });
    } else {
      rows.push(["Slug", "Baslik_TR", "Toplam_Goruntulenme", "Gercek_Okuma", "Ort_Sure_Sn", "Ort_Scroll_Pct"]);
      state.contentAnalytics.forEach((r) => {
        rows.push([
          r.content_slug || r.slug || "",
          r.title_tr || "",
          r.total_views || 0,
          r.true_reads ?? r.true_read_count ?? 0,
          Math.round(r.avg_duration_sec || 0),
          Math.round(r.avg_scroll_pct || 0),
        ]);
      });
    }
    const csvText = "\uFEFF" + rows.map((r) => r.map(csvCell).join(",")).join("\r\n");
    const blob = new Blob([csvText], { type: "text/csv;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `MSI_Analitik_${state.activeAnalyticsSubview}_${new Date().toISOString().slice(0, 10)}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    showToast("CSV analitik raporu indirildi!");
  }

  // =========================================================================
  // TAB 4: CORPORATE VERIFICATION REQUESTS (ZERO FAKE SIMULATION)
  // =========================================================================

  function renderCorporateRequestsTable() {
    const tbody = document.getElementById("tbody-corporate-requests");
    if (!tbody) return;
    if (!state.corporateRequests.length) {
      renderTableState(tbody, "empty", { colCount: 6 });
      return;
    }
    tbody.innerHTML = state.corporateRequests
      .map((req) => {
        const statusBadge =
          req.status === "approved"
            ? '<span class="badge-tag badge-paywall-0"><i class="fa fa-check"></i> Onaylandı (Kamu/Kurum Abonesi)</span>'
            : req.status === "rejected"
              ? '<span class="badge-tag badge-sponsored"><i class="fa fa-times"></i> Reddedildi</span>'
              : '<span class="badge-tag badge-paywall-2"><i class="fa fa-hourglass-half"></i> Onay Bekliyor</span>';
        return `<tr>
          <td style="font-family:var(--font-mono); font-size:11.5px;">
            <strong>#${esc(req.id)}</strong><br>
            <span style="color:var(--text-muted);">${esc(req.created_at || "")}</span>
          </td>
          <td>
            <div style="font-weight:700;">${esc(req.full_name)}</div>
            <div style="font-size:11.5px; color:#0ea5e9;">${esc(req.email)}</div>
          </td>
          <td>
            <span class="badge-tag">${esc(req.organization_category)}</span>
            <div style="font-weight:600; margin-top:3px;">${esc(req.organization_name)}</div>
          </td>
          <td style="max-width:340px; color:var(--text-secondary);">${esc(req.request_message)}</td>
          <td>${statusBadge}</td>
          <td>
            <div style="display:flex; gap:6px; flex-wrap:wrap;">
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-emerald" data-admin-action="decide-corporate" data-id="${esc(req.id)}" data-decision="approved">
                <i class="fa fa-check"></i> Onayla
              </button>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="decide-corporate" data-id="${esc(req.id)}" data-decision="rejected">
                <i class="fa fa-times"></i> Reddet
              </button>
            </div>
          </td>
        </tr>`;
      })
      .join("");
  }

  async function decideCorporateRequest(id, decision) {
    const numId = Number(id);
    const operatorEmail = getOperatorIdentity().email;
    const bodyPayload = {
      id: numId,
      status: decision,
      reviewed_by: operatorEmail || undefined,
    };

    let resp = await apiRequest(
      `/api/v1/admin/corporate-requests/${encodeURIComponent(numId)}/decide`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(bodyPayload),
      },
    );

    if (resp.status === 404) {
      resp = await apiRequest(
        `/api/v1/admin/corporate-requests/${encodeURIComponent(numId)}/decision`,
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(bodyPayload),
        },
      );
    }

    if (resp.ok) {
      showToast(
        decision === "approved"
          ? `Talep #${numId} onaylandı: Kullanıcıya ücretsiz Kurumsal/Kamu E-Dergi aboneliği tanımlandı!`
          : `Talep #${numId} reddedildi.`,
      );
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  // =========================================================================
  // TAB 5: TAXONOMY (CATEGORIES & SSB TAGS)
  // =========================================================================

  function renderTaxonomyTables() {
    const tbodyCats = document.getElementById("tbody-categories");
    if (tbodyCats) {
      tbodyCats.innerHTML = state.categories
        .map(
          (c) => `<tr>
            <td style="font-family:var(--font-mono); font-size:11.5px; color:#0ea5e9;">/${esc(c.slug)}</td>
            <td>
              <div style="font-weight:700;">${esc(c.name_tr)}</div>
              <div style="font-size:11px; color:var(--text-muted);">${esc(c.name_en)}</div>
            </td>
            <td><span class="badge-tag">${esc(c.group_type)}</span></td>
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
            <td style="font-family:var(--font-mono); font-size:11.5px; color:var(--color-success);">${esc(t.code)}</td>
            <td>
              <div style="font-weight:700;">${esc(t.name_tr)}</div>
              <div style="font-size:11px; color:var(--text-muted);">${esc(t.name_en)}</div>
            </td>
            <td><span class="badge-tag badge-ssb">${esc(t.category_group)}</span></td>
            <td style="font-family:var(--font-mono);">${esc(t.sort_order)}</td>
          </tr>`,
        )
        .join("");
    }
  }

  async function submitCategoryForm(e) {
    if (e) e.preventDefault();
    const payload = {
      id: 0,
      slug: document.getElementById("cat-slug").value.trim(),
      parent_slug: null,
      name_tr: document.getElementById("cat-name-tr").value.trim(),
      name_en: document.getElementById("cat-name-en").value.trim(),
      group_type: document.getElementById("cat-group-type").value,
      description_tr:
        document.getElementById("cat-desc-tr").value.trim() || "MSI Dergisi özel kategori bölümü",
      description_en: document.getElementById("cat-name-en").value.trim(),
      icon_class: "fa fa-cubes",
      sort_order: parseInt(document.getElementById("cat-sort-order").value || "25", 10),
      is_dynamic: true,
    };
    const resp = await apiRequest("/api/v1/admin/categories", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast('Yeni kategori "' + payload.name_tr + '" site ağacına eklendi!');
      document.getElementById("form-new-category").reset();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function submitSsbTagForm(e) {
    if (e) e.preventDefault();
    const payload = {
      id: 0,
      code: document.getElementById("ssb-code").value.trim(),
      name_tr: document.getElementById("ssb-name-tr").value.trim(),
      name_en: document.getElementById("ssb-name-en").value.trim(),
      category_group: document.getElementById("ssb-group").value.trim() || "Savunma Sanayii",
      sort_order: parseInt(document.getElementById("ssb-sort").value || "15", 10),
    };
    const resp = await apiRequest("/api/v1/admin/ssb-tags", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast('Yeni SSB İlgi Alanı "' + payload.name_tr + '" taksonomiye eklendi!');
      document.getElementById("form-new-ssb").reset();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  // =========================================================================
  // TAB 6: DYNAMIC NEWSLETTERS (WITH RICH HTML BODY SUPPORT)
  // =========================================================================

  async function submitNewsletterForm(e) {
    if (e) e.preventDefault();
    const bodyHtml = (document.getElementById("nl-body-html")?.value || "").trim();
    const adIdRaw = document.getElementById("nl-sponsor-ad-id")?.value;
    const payload = {
      id: 0,
      subject_tr: document.getElementById("nl-subject-tr").value.trim(),
      subject_en: document.getElementById("nl-subject-en").value.trim(),
      target_group_filter: document.getElementById("nl-group-filter").value,
      target_institution: document.getElementById("nl-institution").value,
      target_ssb_tag: document.getElementById("nl-ssb-tag").value,
      sponsor_ad_id: adIdRaw ? parseInt(adIdRaw, 10) : null,
      sponsor_name: document.getElementById("nl-sponsor-name").value.trim() || null,
      body_html: bodyHtml || null,
      recipient_count: 0,
      status: "sent",
      sent_at: new Date().toISOString().slice(0, 16).replace("T", " "),
    };
    const resp = await apiRequest("/api/v1/admin/newsletters", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast("Sponsorlu E-Bülten seçili kurumsal gruba gönderildi ve kaydedildi!");
      document.getElementById("form-newsletter").reset();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function loadNewsletters(page) {
    paginationState.newsletters = page || paginationState.newsletters;
    const resp = await apiRequest(
      `/api/v1/admin/newsletters?page=${paginationState.newsletters}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.newsletters = unwrapList(resp.payload, "campaigns");
    renderNewslettersTable();
    renderPaginationControls("pagination-newsletters", resp.meta, loadNewsletters);
  }

  function renderNewslettersTable() {
    const tbody = document.getElementById("tbody-newsletters");
    if (!tbody) return;
    if (!state.newsletters.length) {
      tbody.innerHTML =
        renderTableState(tbody, "empty", { colCount: 6 });
      return;
    }
    tbody.innerHTML = state.newsletters
      .map(
        (n) => `<tr>
          <td style="font-family:var(--font-mono); font-size:11.5px;">
            <strong>#${esc(n.id)}</strong><br>
            <span style="color:var(--text-muted);">${esc(n.sent_at || "")}</span>
          </td>
          <td>
            <div style="font-weight:700;">${esc(n.subject_tr)}</div>
            <div style="font-size:11px; color:var(--text-muted);">${esc(n.subject_en || "")}</div>
          </td>
          <td>
            <span class="badge-tag">${esc(n.target_institution)}</span>
            <span class="badge-tag badge-ssb">${esc(n.target_ssb_tag)}</span>
            <span class="badge-tag badge-placement">${esc(n.target_group_filter)}</span>
          </td>
          <td><span class="badge-tag badge-sponsored">${esc(n.sponsor_name || "Sponsorsuz")}</span></td>
          <td style="font-family:var(--font-mono);">
            <strong style="color:var(--color-success);">${Number(n.recipient_count || 0).toLocaleString("tr-TR")} alıcı</strong><br>
            <span style="font-size:11px; color:var(--text-muted);">${esc(n.status)}</span>
          </td>
        </tr>`,
      )
      .join("");
  }

  // =========================================================================
  // TAB 7: RENEWAL AUTOMATION (ZERO FAKE SIMULATION) & 301 REDIRECTS
  // =========================================================================

  async function loadReminders(page) {
    paginationState.reminders = page || paginationState.reminders;
    const resp = await apiRequest(
      `/api/v1/admin/reminders?page=${paginationState.reminders}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.reminders = unwrapList(resp.payload, "reminders");
    renderRemindersTable();
    renderPaginationControls("pagination-reminders", resp.meta, loadReminders);
  }

  async function loadRedirects(page) {
    paginationState.redirects = page || paginationState.redirects;
    const resp = await apiRequest(
      `/api/v1/admin/redirects?page=${paginationState.redirects}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.redirects = unwrapList(resp.payload, "redirects");
    renderRedirectsTable();
    renderPaginationControls("pagination-redirects", resp.meta, loadRedirects);
  }

  function renderRemindersAndRedirects() {
    renderRemindersTable();
    renderRedirectsTable();
  }

  function renderRemindersTable() {
    const tbodyRem = document.getElementById("tbody-reminders");
    if (!tbodyRem) return;
    const stageMap = {
      "2_months": "2 Ay Kala (60 Gün)",
      "1_month": "1 Ay Kala (30 Gün)",
      "2_weeks": "2 Hafta Kala (14 Gün)",
      "1_week": "1 Hafta Kala (7 Gün)",
      "1_day": "1 Gün Kala (Son Uyarı)",
    };
    if (!state.reminders.length) {
      tbodyRem.innerHTML =
        renderTableState(tbodyRem, "empty", { colCount: 5 });
      return;
    }
    tbodyRem.innerHTML = state.reminders
      .map((r) => {
        const stBadge =
          r.status === "halted_by_renewal"
            ? '<span class="badge-tag badge-paywall-0"><i class="fa fa-stop-circle"></i> Yeniledi — Otomatik Durduruldu</span>'
            : r.status === "sent"
              ? '<span class="badge-tag"><i class="fa fa-check-square-o"></i> Gönderildi</span>'
              : '<span class="badge-tag badge-paywall-2"><i class="fa fa-clock-o"></i> Planlandı</span>';
        return `<tr>
          <td>
            <div style="font-weight:700;">${esc(r.full_name)}</div>
            <div style="font-size:11px; color:var(--text-muted);">${esc(r.user_email)} • ${esc(r.organization_name)}</div>
          </td>
          <td><span class="badge-tag badge-placement">${esc(stageMap[r.stage_label] || r.stage_label)}</span></td>
          <td style="font-family:var(--font-mono); font-weight:700;">${esc(r.days_remaining)} gün</td>
          <td>${stBadge}</td>
          <td style="font-family:var(--font-mono); font-size:11px; color:var(--text-muted);">${esc(r.sent_at)}</td>
        </tr>`;
      })
      .join("");
  }

  function renderRedirectsTable() {
    const tbodySeo = document.getElementById("tbody-redirects");
    if (!tbodySeo) return;
    if (!state.redirects.length) {
      tbodySeo.innerHTML =
        renderTableState(tbodySeo, "empty", { colCount: 5 });
      return;
    }
    tbodySeo.innerHTML = state.redirects
      .map(
        (s) => `<tr>
          <td style="font-family:var(--font-mono); font-size:11.5px; color:var(--color-danger);">${esc(s.old_path)}</td>
          <td style="font-family:var(--font-mono); font-size:11.5px; color:var(--color-success);">${esc(s.new_path)}</td>
          <td><span class="badge-tag badge-paywall-0">${esc(s.status_code || 301)}</span></td>
          <td style="font-family:var(--font-mono); font-weight:700;">${Number(s.hit_count || 0).toLocaleString("tr-TR")}</td>
        </tr>`,
      )
      .join("");
  }

  async function simulateRenewalHalt() {
    const resp = await apiRequest("/api/v1/admin/reminders/halt-renewed", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ action: "halt_renewed" }),
    });
    if (!resp.ok) {
      showToast(resp.errorMessage, true);
      return;
    }
    showToast(
      "Aboneliğini yenileyen kullanıcılar için kalan hatırlatma e-postaları durduruldu (halted_by_renewal)!",
    );
    await refreshAllData();
  }

  async function submitSeoRedirect(e) {
    if (e) e.preventDefault();
    const payload = {
      id: 0,
      old_path: document.getElementById("seo-old-path").value.trim(),
      new_path: document.getElementById("seo-new-path").value.trim(),
      status_code: 301,
      hit_count: 0,
    };
    const resp = await apiRequest("/api/v1/admin/redirects", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast("301 Kalıcı SEO yönlendirme kuralı eklendi!");
      document.getElementById("form-seo-redirect").reset();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  // =========================================================================
  // TAB 8: CENTRAL ENGINES (SETTINGS, NOTIFICATIONS, WAF & 2FA TOTP)
  // =========================================================================

  function toggleAdminTheme() {
    const current = document.documentElement.getAttribute("data-theme") === "light" ? "light" : "dark";
    const next = current === "light" ? "dark" : "light";
    if (next === "light") {
      document.documentElement.setAttribute("data-theme", "light");
      document.body.classList.add("admin-light-mode");
      document.body.classList.add("gw-light-mode");
    } else {
      document.documentElement.removeAttribute("data-theme");
      document.body.classList.remove("admin-light-mode");
      document.body.classList.remove("gw-light-mode");
    }
    try {
      localStorage.setItem("msi_theme", next);
      localStorage.setItem("msi_admin_theme", next);
    } catch (_) {}
  }

  async function logoutOperator() {
    await apiRequest("/api/v1/auth/logout", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    window.location.reload();
  }

  function open2faModal() {
    const modal = document.getElementById("totp-security-modal");
    const initStage = document.getElementById("totp-setup-stage-init");
    const verifyStage = document.getElementById("totp-setup-stage-verify");
    if (initStage) initStage.style.display = "block";
    if (verifyStage) verifyStage.style.display = "none";
    if (modal) modal.style.display = "flex";
  }

  function close2faModal() {
    const modal = document.getElementById("totp-security-modal");
    if (modal) modal.style.display = "none";
  }

  async function start2faSetup() {
    let resp = await apiRequest("/api/v1/auth/totp/setup", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    if (resp.status === 405) {
      resp = await apiRequest("/api/v1/auth/totp/setup", { method: "GET" });
    }
    if (!resp.ok) {
      showToast(resp.errorMessage, true);
      return;
    }
    const d = resp.data || {};
    const qrImg = document.getElementById("totp-qr-img");
    const secretEl = document.getElementById("totp-secret-display");
    const backupWrap = document.getElementById("totp-backup-codes-wrap");
    const backupList = document.getElementById("totp-backup-codes-list");

    if (qrImg && d.qr_svg) {
      qrImg.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(d.qr_svg);
    } else if (qrImg && d.qr_data_url) {
      qrImg.src = d.qr_data_url;
    }
    if (secretEl) secretEl.value = d.secret || "";

    const codes = d.backup_codes || d.recovery_codes || [];
    if (backupWrap && backupList && Array.isArray(codes) && codes.length) {
      backupList.textContent = codes.join("   ");
      backupWrap.style.display = "block";
    } else if (backupWrap) {
      backupWrap.style.display = "none";
    }

    const initStage = document.getElementById("totp-setup-stage-init");
    const verifyStage = document.getElementById("totp-setup-stage-verify");
    if (initStage) initStage.style.display = "none";
    if (verifyStage) verifyStage.style.display = "block";
  }

  async function verify2faSetup(e) {
    if (e) e.preventDefault();
    const code = (document.getElementById("totp-setup-code")?.value || "").trim();
    if (!code) {
      showToast("Lütfen doğrulama kodunu girin.", true);
      return false;
    }
    let resp = await apiRequest("/api/v1/auth/totp/verify-setup", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ code }),
    });
    if (resp.status === 404) {
      resp = await apiRequest("/api/v1/auth/totp/verify", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ code }),
      });
    }
    if (resp.ok) {
      const codes = (resp.data && (resp.data.backup_codes || resp.data.recovery_codes)) || [];
      if (Array.isArray(codes) && codes.length) {
        const backupWrap = document.getElementById("totp-backup-codes-wrap");
        const backupList = document.getElementById("totp-backup-codes-list");
        if (backupWrap && backupList) {
          backupList.textContent = codes.join("   ");
          backupWrap.style.display = "block";
        }
        showToast("2FA etkinleştirildi! Yedek kurtarma kodlarınızı kaydetmeyi unutmayın.");
      } else {
        showToast("İki faktörlü doğrulama (2FA) başarıyla etkinleştirildi!");
        close2faModal();
      }
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function disable2fa(e) {
    if (e) e.preventDefault();
    const password = document.getElementById("totp-disable-password")?.value || "";
    if (!password) {
      showToast("Lütfen mevcut parolanızı girin.", true);
      return false;
    }
    const resp = await apiRequest("/api/v1/auth/totp/disable", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password }),
    });
    if (resp.ok) {
      document.getElementById("form-totp-disable")?.reset();
      showToast("İki faktörlü doğrulama devre dışı bırakıldı.");
      close2faModal();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function refreshCentralEnginesTab() {
    const [settingsRes, secRes, notifRes] = await Promise.all([
      safeFetchJson("/api/v1/admin/settings"),
      safeFetchJson("/api/v1/admin/security-logs"),
      safeFetchJson("/api/v1/admin/notifications"),
    ]);

    const tbodySet = document.getElementById("tbody-site-settings");
    const settingsList = unwrapList(settingsRes, "settings");
    if (tbodySet) {
      tbodySet.innerHTML = settingsList.length
        ? settingsList
            .map(
              (s) => `<tr>
                <td style="font-family:var(--font-mono); color:#0ea5e9;">${esc(s.key)}</td>
                <td>${esc(s.value_tr)}</td>
                <td style="color:var(--text-muted);">${esc(s.value_en)}</td>
                <td><span class="badge-tag">${esc(s.setting_group)}</span></td>
              </tr>`,
            )
            .join("")
        : '<tr><td colspan="4" style="text-align:center; color:var(--text-muted);">Ayar kaydı bulunamadı.</td></tr>';
    }

    const tbodySec = document.getElementById("tbody-security-logs");
    const secList = unwrapList(secRes, "logs");
    if (tbodySec) {
      tbodySec.innerHTML = secList.length
        ? secList
            .map(
              (l) => `<tr>
                <td style="font-family:var(--font-mono); font-size:11px;">${esc(l.created_at)}</td>
                <td style="font-family:var(--font-mono); color:var(--color-danger);">${esc(l.client_ip)}</td>
                <td><span class="badge-tag badge-sponsored">${esc(l.event_type)}</span></td>
                <td><span class="badge-tag badge-paywall-2">${esc(l.threat_level)}</span></td>
                <td style="font-size:11.5px;"><code>${esc(l.request_path)}</code><br><span style="color:var(--text-muted);">${esc(l.details)}</span></td>
              </tr>`,
            )
            .join("")
        : '<tr><td colspan="5" style="text-align:center; color:var(--text-muted);">WAF güvenlik kaydı yok.</td></tr>';
    }

    const tbodyNotif = document.getElementById("tbody-sys-notifications");
    const notifList = unwrapList(notifRes, "notifications");
    if (tbodyNotif) {
      tbodyNotif.innerHTML = notifList.length
        ? notifList
            .map(
              (n) => `<tr>
                <td><span class="badge-tag">${esc(n.target_audience)}</span></td>
                <td><span class="badge-tag badge-paywall-1">${esc(n.severity)}</span></td>
                <td><strong>${esc(n.title_tr)}</strong><br><span style="color:var(--text-muted); font-size:11.5px;">${esc(n.message_tr)}</span></td>
                <td>${n.is_banner || n.is_active_banner ? '<span class="badge-tag badge-paywall-0">ÜST BANT</span>' : '<span class="badge-tag">BİLDİRİM</span>'}</td>
              </tr>`,
            )
            .join("")
        : '<tr><td colspan="4" style="text-align:center; color:var(--text-muted);">Aktif bildirim bulunmuyor.</td></tr>';
    }
  }

  async function submitSiteSetting(e) {
    if (e) e.preventDefault();
    const payload = {
      key: document.getElementById("set-key").value,
      value_tr: document.getElementById("set-val-tr").value.trim(),
      value_en:
        document.getElementById("set-val-en").value.trim() ||
        document.getElementById("set-val-tr").value.trim(),
      setting_group: document.getElementById("set-group").value.trim() || "general",
    };
    const resp = await apiRequest("/api/v1/admin/settings", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast("Merkezi tasarım ayarı güncellendi ve L1 Önbellek yenilendi!");
      await refreshCentralEnginesTab();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function submitSysNotification(e) {
    if (e) e.preventDefault();
    const rawAudience = document.getElementById("notif-audience").value || "all";
    const normalizedAudience = rawAudience.toUpperCase() === "ALL" ? "all" : rawAudience;
    const payload = {
      target_audience: normalizedAudience,
      severity: document.getElementById("notif-severity").value,
      is_banner: document.getElementById("notif-banner").value === "true",
      category: "editorial",
      title_tr: document.getElementById("notif-title-tr").value.trim(),
      message_tr: document.getElementById("notif-msg-tr").value.trim(),
    };
    const resp = await apiRequest("/api/v1/admin/notifications", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast("Merkezi bildirim ve üst bant duyurusu yayınlandı!");
      document.getElementById("form-sys-notif").reset();
      await refreshCentralEnginesTab();
    } else {
      showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function uploadAssetToInput(fileInput, targetInputId) {
    const file = fileInput?.files?.[0];
    if (!file) return;
    const fd = new FormData();
    fd.append("file", file);
    try {
      const resp = await apiRequest("/api/v1/admin/upload", {
        method: "POST",
        body: fd,
      });
      if (!resp.ok) {
        showToast(resp.errorMessage || "Dosya yüklenemedi.", true);
        return;
      }
      const resPayload = resp.payload || {};
      const resolvedUrl =
        resPayload?.data?.url ||
        resPayload?.data?.path ||
        resPayload?.url ||
        resPayload?.path ||
        "";
      const target = document.getElementById(targetInputId);
      if (target && resolvedUrl) {
        target.value = resolvedUrl;
      }
      showToast("Medya başarıyla yüklendi: " + (resolvedUrl || file.name));
      loadMediaLibrary();
    } finally {
      fileInput.value = "";
    }
  }

  async function uploadCoverAsset(fileInput) {
    return uploadAssetToInput(fileInput, "c-cover-image");
  }

  // =========================================================================
  // TAB 9: ANALYTICS CHARTS & SYSTEM HEALTH
  // =========================================================================

  function renderAnalyticsCharts() {
    if (!state.contentAnalytics.length || typeof Chart === "undefined") return;
    const trendCanvas = document.getElementById("chart-content-trend");
    if (trendCanvas) {
      const sorted = [...state.contentAnalytics].sort((a, b) =>
        (a.content_slug || a.slug || "").localeCompare(b.content_slug || b.slug || ""),
      );
      const slice30 = sorted.slice(0, 30);
      const labels = slice30.map((c) => c.content_slug || c.slug || c.title_tr || "?");
      const views = slice30.map((c) => c.total_views ?? c.view_count ?? 0);
      const reads = slice30.map(
        (c) => c.true_reads ?? c.total_true_reads ?? c.true_read_count ?? 0,
      );
      if (trendCanvas._chartInstance) trendCanvas._chartInstance.destroy();
      trendCanvas._chartInstance = new Chart(trendCanvas.getContext("2d"), {
        type: "line",
        data: {
          labels,
          datasets: [
            {
              label: "Görüntülenme",
              data: views,
              borderColor: "#38bdf8",
              backgroundColor: "rgba(56,189,248,0.1)",
              tension: 0.3,
              fill: true,
            },
            {
              label: "Gerçek Okuma",
              data: reads,
              borderColor: "var(--color-success)",
              backgroundColor: "rgba(16,185,129,0.1)",
              tension: 0.3,
              fill: true,
            },
          ],
        },
        options: {
          responsive: true,
          plugins: { legend: { labels: { color: "#cbd5e1" } } },
          scales: {
            x: { ticks: { color: "#64748b", maxRotation: 45 } },
            y: { ticks: { color: "#64748b" }, beginAtZero: true },
          },
        },
      });
    }

    const topCanvas = document.getElementById("chart-top-contents");
    if (topCanvas) {
      const top10 = [...state.contentAnalytics]
        .sort((a, b) => (b.total_views ?? b.view_count ?? 0) - (a.total_views ?? a.view_count ?? 0))
        .slice(0, 10);
      if (topCanvas._chartInstance) topCanvas._chartInstance.destroy();
      topCanvas._chartInstance = new Chart(topCanvas.getContext("2d"), {
        type: "bar",
        data: {
          labels: top10.map((c) => (c.title_tr || c.content_slug || c.slug || "?").substring(0, 32)),
          datasets: [
            {
              label: "Toplam Görüntülenme",
              data: top10.map((c) => c.total_views ?? c.view_count ?? 0),
              backgroundColor: [
                "#c91818",
                "#ea580c",
                "#f59e0b",
                "var(--color-success)",
                "#0ea5e9",
                "#6366f1",
                "#8b5cf6",
                "#ec4899",
                "#14b8a6",
                "#64748b",
              ],
            },
          ],
        },
        options: {
          indexAxis: "y",
          responsive: true,
          plugins: { legend: { display: false } },
          scales: {
            x: { ticks: { color: "#64748b" }, beginAtZero: true },
            y: { ticks: { color: "#cbd5e1" } },
          },
        },
      });
    }
  }

  async function refreshSystemHealth() {
    const panel = document.getElementById("system-health-panel");
    if (!panel) return;
    const resp = await apiRequest("/api/v1/admin/system-health");
    if (!resp.ok || !resp.data) {
      panel.innerHTML = `<div style="color:var(--color-danger);">Sağlık kontrolü başarısız: ${esc(resp.errorMessage)}</div>`;
      return;
    }
    const data = resp.data;
    state.systemHealth = data;
    const dbMB = ((data.db_size_bytes || 0) / 1048576).toFixed(2);
    const statusColor = data.status === "healthy" ? "var(--color-success)" : "var(--color-danger)";
    const ts = data.timestamp ? new Date(data.timestamp).toLocaleString("tr-TR") : "—";
    panel.innerHTML = `<div style="display:grid; gap:8px;">
      <div><span style="color:var(--text-muted);">Durum:</span> <span style="color:${statusColor}; font-weight:700;">${esc((data.status || "unknown").toUpperCase())}</span></div>
      <div><span style="color:var(--text-muted);">DB Boyutu:</span> <strong>${esc(dbMB)} MB</strong></div>
      <div><span style="color:var(--text-muted);">İçerik Sayısı:</span> <strong>${esc(data.tables?.contents ?? 0)}</strong></div>
      <div><span style="color:var(--text-muted);">Kullanıcı Sayısı:</span> <strong>${esc(data.tables?.users ?? 0)}</strong></div>
      <div><span style="color:var(--text-muted);">Analitik Event:</span> <strong>${esc(data.tables?.analytics_events ?? 0)}</strong></div>
      <div><span style="color:var(--text-muted);">Son Kontrol:</span> ${esc(ts)}</div>
    </div>`;
  }

  // =========================================================================
  // TAB 10: MEDIA LIBRARY & SEO PREVIEW
  // =========================================================================

  async function loadMediaLibrary() {
    const resp = await apiRequest("/api/v1/admin/media-library");
    if (!resp.ok || !resp.payload) return;
    state.mediaLibrary = unwrapList(resp.payload, "items");
    renderMediaLibrary(state.mediaLibrary);
  }

  function renderMediaLibrary(items) {
    const grid = document.getElementById("media-library-grid");
    if (!grid) return;
    if (!items.length) {
      grid.innerHTML =
        '<div style="color:var(--text-muted); padding:20px; text-align:center;">Medya bulunamadı.</div>';
      return;
    }
    grid.innerHTML = items
      .map((m) => {
        const mediaPath = m.path || m.url || "";
        const isImage = (m.mime_type || "").startsWith("image/");
        const sizeKB = ((m.file_size || 0) / 1024).toFixed(1);
        const srcUrl = mediaPath.startsWith("/") || mediaPath.startsWith("http") ? mediaPath : "/" + mediaPath;
        const thumb = isImage
          ? `<img src="${esc(srcUrl)}" alt="${esc(m.original_name)}" style="width:100%; height:90px; object-fit:cover; border-radius:2px; margin-bottom:6px;" loading="lazy">`
          : `<div style="width:100%; height:90px; background:rgba(148,163,184,0.08); border-radius:2px; margin-bottom:6px; display:flex; align-items:center; justify-content:center; font-size:24px; color:var(--text-muted);"><i class="fa fa-file"></i></div>`;
        return `<div style="background:var(--color-elevated); border:1px solid var(--color-border); border-radius:2px; padding:8px; cursor:pointer;" data-admin-action="copy-media-url" data-url="${esc(mediaPath)}" title="URL kopyalamak için tıklayın">
          ${thumb}
          <div style="font-size:11px; white-space:nowrap; overflow:hidden; text-overflow:ellipsis;" title="${esc(m.original_name)}">${esc(m.original_name)}</div>
          <div style="font-size:10px; color:var(--text-muted);">${esc(sizeKB)} KB · ${esc(m.mime_type)}</div>
        </div>`;
      })
      .join("");
  }

  function filterMediaLibrary() {
    const q = (document.getElementById("media-search")?.value || "").toLowerCase();
    if (!q) {
      renderMediaLibrary(state.mediaLibrary);
      return;
    }
    const filtered = state.mediaLibrary.filter(
      (m) =>
        (m.original_name || "").toLowerCase().includes(q) ||
        (m.tags || "").toLowerCase().includes(q) ||
        (m.mime_type || "").toLowerCase().includes(q),
    );
    renderMediaLibrary(filtered);
  }

  async function bulkUploadMedia(input) {
    if (!input || !input.files || !input.files.length) return;
    let uploadedCount = 0;
    for (const file of input.files) {
      const fd = new FormData();
      fd.append("file", file);
      const resp = await apiRequest("/api/v1/admin/upload", {
        method: "POST",
        body: fd,
      });
      if (resp.ok) {
        uploadedCount += 1;
      } else {
        showToast(resp.errorMessage, true);
      }
    }
    if (uploadedCount > 0) {
      showToast(`${uploadedCount} medya dosyası yüklendi!`);
      await loadMediaLibrary();
    }
    input.value = "";
  }

  function populateSeoPreviewSelect() {
    const sel = document.getElementById("seo-preview-select");
    if (!sel) return;
    sel.innerHTML =
      '<option value="">-- İçerik seçin --</option>' +
      state.contents
        .map(
          (c) =>
            `<option value="${esc(c.id)}">${esc(c.title_tr || c.slug)} (${esc(c.content_type || "article")})</option>`,
        )
        .join("");
  }

  function updateSeoPreview() {
    const id = parseInt(document.getElementById("seo-preview-select")?.value || "0", 10);
    const content = state.contents.find((c) => Number(c.id) === id);
    if (!content) return;
    const title = content.title_tr || content.slug || "Başlıksız";
    const desc = (content.summary_tr || content.body_tr || "")
      .replace(/<[^>]*>/g, "")
      .substring(0, 160);
    const url = `msidergisi.com/icerik/${content.slug || content.id}`;
    const el = (s) => document.getElementById(s);
    if (el("seo-google-title")) el("seo-google-title").textContent = title;
    if (el("seo-google-url")) el("seo-google-url").textContent = url;
    if (el("seo-google-desc")) el("seo-google-desc").textContent = desc || "Açıklama yok.";
    if (el("seo-og-title")) el("seo-og-title").textContent = title;
    if (el("seo-og-desc")) el("seo-og-desc").textContent = desc || "Açıklama yok.";
    const jsonLd = {
      "@context": "https://schema.org",
      "@type": "NewsArticle",
      headline: title,
      description: desc,
      url: `https://${url}`,
      datePublished: content.published_at || content.created_at || new Date().toISOString(),
      author: { "@type": "Organization", name: content.author_name || "MSI Dergisi" },
      publisher: {
        "@type": "Organization",
        name: "MSI Dergisi",
        logo: { "@type": "ImageObject", url: "https://msidergisi.com/images/logo.png" },
      },
    };
    if (el("seo-jsonld")) el("seo-jsonld").textContent = JSON.stringify(jsonLd, null, 2);
  }

  // =========================================================================
  // TAB 11: ROLES, PERMISSIONS, USER ROLE ASSIGNMENT & AUDIT LOG
  // =========================================================================

  async function loadUserRoles(page) {
    paginationState.userRoles = page || paginationState.userRoles;
    const resp = await apiRequest(
      `/api/v1/admin/user-roles?page=${paginationState.userRoles}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.userRoles = unwrapList(resp.payload, "roles");
    renderUserRoles();
    syncRoleSelectOptions();
    renderAdminUsers();
    renderPaginationControls("pagination-user-roles", resp.meta, loadUserRoles);
  }

  function syncRoleSelectOptions() {
    const manualSel = document.getElementById("manual-assign-role-code");
    if (!manualSel || !state.userRoles.length) return;
    const prev = manualSel.value;
    manualSel.innerHTML = state.userRoles
      .map((r) => `<option value="${esc(r.role_code)}">${esc(r.role_code)} — ${esc(r.name_tr)}</option>`)
      .join("");
    if ([...manualSel.options].some((o) => o.value === prev)) {
      manualSel.value = prev;
    }
  }

  function renderUserRoles() {
    const tbody = document.getElementById("tbody-user-roles");
    if (!tbody) return;
    if (!state.userRoles.length) {
      tbody.innerHTML =
        renderTableState(tbody, "empty", { colCount: 6 });
      return;
    }
    tbody.innerHTML = state.userRoles
      .map((r) => {
        let permCount = 0;
        try {
          permCount = JSON.parse(r.permissions_json || "[]").length;
        } catch (_) {}
        const isSystem = r.is_system == 1 || r.is_system === true;
        const actions = isSystem
          ? '<span style="color:var(--text-muted); font-size:11px;">Korunuyor</span>'
          : `<div style="display:flex; gap:4px;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-role" data-role="${esc(r.role_code)}" title="Düzenle"><i class="fa fa-pencil"></i></button>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-role" data-role="${esc(r.role_code)}" title="Sil"><i class="fa fa-trash"></i></button>
            </div>`;
        return `<tr>
          <td><code>${esc(r.role_code)}</code></td>
          <td>${esc(r.name_tr)}</td>
          <td>${esc(r.name_en)}</td>
          <td><span class="badge-tag">${esc(permCount)} izin</span></td>
          <td>${isSystem ? '<i class="fa fa-lock" style="color:#f59e0b;" title="Sistem rolü"></i>' : '<i class="fa fa-unlock" style="color:var(--text-muted);"></i>'}</td>
          <td>${actions}</td>
        </tr>`;
      })
      .join("");
  }

  function showRoleForm() {
    const f = document.getElementById("role-form-container");
    if (!f) return;
    document.getElementById("role-edit-mode").value = "create";
    const codeEl = document.getElementById("role-code");
    if (codeEl) {
      codeEl.value = "";
      codeEl.readOnly = false;
    }
    document.getElementById("role-name-tr").value = "";
    document.getElementById("role-name-en").value = "";
    document.querySelectorAll("#role-permissions-grid input[type='checkbox']").forEach((cb) => {
      cb.checked = false;
    });
    f.style.display = "block";
  }

  function hideRoleForm() {
    const f = document.getElementById("role-form-container");
    if (f) f.style.display = "none";
  }

  function editRole(roleCode) {
    const role = state.userRoles.find((r) => r.role_code === roleCode);
    const f = document.getElementById("role-form-container");
    if (!role || !f) return;
    document.getElementById("role-edit-mode").value = "edit";
    const codeEl = document.getElementById("role-code");
    if (codeEl) {
      codeEl.value = role.role_code || "";
      codeEl.readOnly = true;
    }
    document.getElementById("role-name-tr").value = role.name_tr || "";
    document.getElementById("role-name-en").value = role.name_en || "";
    let perms = [];
    try {
      perms = JSON.parse(role.permissions_json || "[]");
    } catch (_) {}
    const permSet = new Set(perms);
    document.querySelectorAll("#role-permissions-grid input[type='checkbox']").forEach((cb) => {
      cb.checked = permSet.has(cb.value);
    });
    f.style.display = "block";
  }

  async function saveRole() {
    const mode = document.getElementById("role-edit-mode")?.value || "create";
    const code = document.getElementById("role-code")?.value?.trim();
    const nameTr = document.getElementById("role-name-tr")?.value?.trim();
    const nameEn = document.getElementById("role-name-en")?.value?.trim();
    if (!code || !nameTr) {
      showToast("Rol kodu ve TR adı zorunludur!", true);
      return;
    }
    const perms = [...document.querySelectorAll("#role-permissions-grid input:checked")].map(
      (cb) => cb.value,
    );
    const url =
      mode === "edit"
        ? `/api/v1/admin/user-roles/${encodeURIComponent(code)}`
        : "/api/v1/admin/user-roles";
    const method = mode === "edit" ? "PUT" : "POST";
    const resp = await apiRequest(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        role_code: code,
        name_tr: nameTr,
        name_en: nameEn || nameTr,
        permissions_json: JSON.stringify(perms),
      }),
    });
    if (resp.ok) {
      showToast(mode === "edit" ? `Rol "${code}" güncellendi!` : `Yeni rol "${code}" oluşturuldu!`);
      hideRoleForm();
      await loadUserRoles();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function deleteRole(roleCode) {
    if (!confirm(`"${roleCode}" rolünü silmek istediğinize emin misiniz?`)) return;
    const resp = await apiRequest(`/api/v1/admin/user-roles/${encodeURIComponent(roleCode)}`, {
      method: "DELETE",
    });
    if (resp.ok) {
      showToast(`Rol "${roleCode}" silindi.`);
      await loadUserRoles();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function loadAdminUsers() {
    const resp = await apiRequest("/api/v1/admin/users?per_page=100");
    if (resp.ok && resp.payload) {
      const list = unwrapList(resp.payload, "users");
      if (list.length) {
        state.adminUsers = list;
        renderAdminUsers();
        return;
      }
    }
    state.adminUsers = (state.readerAnalytics || []).map((u) => ({
      id: u.user_id || u.id,
      full_name: u.full_name,
      email: u.email,
      role: u.role || "",
      subscription_tier: u.subscription_tier || "",
      organization_name: u.organization_name || "",
    }));
    renderAdminUsers();
  }

  function renderAdminUsers() {
    const tbody = document.getElementById("tbody-admin-users");
    if (!tbody) return;
    const q = (document.getElementById("user-role-search")?.value || "").toLowerCase().trim();
    const filtered = state.adminUsers.filter((u) => {
      if (!q) return true;
      return (
        (u.full_name || "").toLowerCase().includes(q) ||
        (u.email || "").toLowerCase().includes(q) ||
        (u.role || "").toLowerCase().includes(q) ||
        String(u.id || "").includes(q)
      );
    });

    if (!filtered.length) {
      tbody.innerHTML =
        renderTableState(tbody, "empty", { colCount: 3 });
      return;
    }

    const roleChoices = state.userRoles.length
      ? state.userRoles.map((r) => r.role_code)
      : [
          "site_operator",
          "super_admin",
          "content_editor",
          "content_publisher",
          "ads_manager",
          "finance_viewer",
          "support_agent",
          "subscriber_public",
          "registered",
        ];

    tbody.innerHTML = filtered
      .map((u) => {
        const uid = u.id ?? u.user_id;
        const currentRole = u.role || "";
        const opts = roleChoices
          .map(
            (rc) =>
              `<option value="${esc(rc)}" ${rc === currentRole ? "selected" : ""}>${esc(rc)}</option>`,
          )
          .join("");
        return `<tr>
          <td>
            <div style="font-weight:700;">#${esc(uid)} · ${esc(u.full_name)}</div>
            <div style="font-size:11px; color:var(--text-muted);">${esc(u.email)}</div>
          </td>
          <td>
            ${currentRole ? `<span class="badge-tag badge-placement">${esc(currentRole)}</span>` : ""}
            ${u.subscription_tier ? `<span class="badge-tag">${esc(u.subscription_tier)}</span>` : ""}
          </td>
          <td>
            <div style="display:flex; gap:6px; align-items:center;">
              <select class="form-select" id="assign-role-sel-${esc(uid)}" style="min-height:32px; padding:4px 8px; font-size:11.5px;">
                ${opts}
              </select>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-primary" data-admin-action="assign-user-role" data-user-id="${esc(uid)}">
                Rol Ata
              </button>
            </div>
          </td>
        </tr>`;
      })
      .join("");
  }

  async function assignRoleToUser(userId, roleCode) {
    const uid = parseInt(String(userId || "0"), 10);
    if (!uid || !roleCode) {
      showToast("Geçerli kullanıcı ID ve rol seçilmelidir.", true);
      return;
    }
    const resp = await apiRequest(`/api/v1/admin/users/${encodeURIComponent(uid)}/assign-role`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role_code: roleCode }),
    });
    if (resp.ok) {
      showToast(`Kullanıcı #${uid} için "${roleCode}" rolü başarıyla atandı!`);
      const found = state.adminUsers.find((u) => Number(u.id) === uid);
      if (found) found.role = roleCode;
      renderAdminUsers();
      await loadAuditLogs();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function loadAuditLogs(page) {
    paginationState.auditLogs = page || paginationState.auditLogs;
    const resp = await apiRequest(
      `/api/v1/admin/audit-logs?page=${paginationState.auditLogs}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.auditLogs = unwrapList(resp.payload, "logs");
    renderAuditLogs(state.auditLogs);
    renderPaginationControls("pagination-audit-logs", resp.meta, loadAuditLogs);
  }

  function renderAuditLogs(logs) {
    const tbody = document.getElementById("tbody-audit-logs");
    if (!tbody) return;
    if (!logs.length) {
      renderTableState(tbody, "empty", { colCount: 6 });
      return;
    }
    tbody.innerHTML = logs
      .map((l) => {
        const actionColors = {
          create: "var(--color-success)",
          update: "#0ea5e9",
          delete: "var(--color-danger)",
          login: "#f59e0b",
          flush: "#8b5cf6",
        };
        const color = actionColors[l.action] || "#94a3b8";
        let detail = "";
        try {
          const d = JSON.parse(l.details_json || "{}");
          detail = Object.entries(d)
            .map(([k, v]) => `${k}: ${v}`)
            .join(", ");
        } catch (_) {
          detail = l.details_json || "";
        }
        const ts = l.created_at ? new Date(l.created_at).toLocaleString("tr-TR") : "—";
        return `<tr>
          <td style="white-space:nowrap; font-size:11px;">${esc(ts)}</td>
          <td>${esc(l.actor_email)}</td>
          <td><span style="color:${color}; font-weight:600;">${esc(l.action)}</span></td>
          <td>${esc(l.entity_type)}${l.entity_id ? " #" + esc(l.entity_id) : ""}</td>
          <td style="max-width:220px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;" title="${esc(detail)}">${esc(detail.substring(0, 65))}</td>
          <td style="font-size:11px; color:var(--text-muted);">${esc(l.ip_address || "-")}</td>
        </tr>`;
      })
      .join("");
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
  // TAB 12: BULK OPERATIONS & VERSION HISTORY
  // =========================================================================

  function renderBulkContentsTable() {
    const tbody = document.getElementById("tbody-bulk-contents");
    if (!tbody) return;
    state.bulkSelected.clear();
    updateBulkCount();
    const masterCb = document.getElementById("bulk-select-all-cb");
    if (masterCb) masterCb.checked = false;

    const visLabels = { 0: "Açık", 1: "Kayıtlı", 2: "Abone", 3: "Premium" };
    tbody.innerHTML = state.contents
      .map(
        (c) => `<tr>
          <td><input type="checkbox" class="bulk-cb" data-id="${esc(c.id)}"></td>
          <td>${esc(c.id)}</td>
          <td>${esc(c.title_tr || c.slug)}</td>
          <td><span style="font-size:11px; color:var(--text-muted);">${esc(c.content_type || "article")}</span></td>
          <td><span style="font-size:11px;">${esc(visLabels[c.visibility_level] ?? c.visibility_level)}</span></td>
          <td>${Number(c.view_count || 0).toLocaleString("tr-TR")}</td>
        </tr>`,
      )
      .join("");
    populateVersionHistorySelect();
    populateSeoPreviewSelect();
  }

  function toggleBulkSelect(id, checked) {
    const numId = Number(id);
    if (checked) state.bulkSelected.add(numId);
    else state.bulkSelected.delete(numId);
    updateBulkCount();
  }

  function selectAllBulkContents() {
    state.contents.forEach((c) => state.bulkSelected.add(Number(c.id)));
    document.querySelectorAll(".bulk-cb").forEach((cb) => {
      cb.checked = true;
    });
    updateBulkCount();
  }

  function deselectAllBulkContents() {
    state.bulkSelected.clear();
    document.querySelectorAll(".bulk-cb").forEach((cb) => {
      cb.checked = false;
    });
    updateBulkCount();
  }

  function toggleBulkSelectAll(masterCb) {
    if (masterCb && masterCb.checked) selectAllBulkContents();
    else deselectAllBulkContents();
  }

  function updateBulkCount() {
    const el = document.getElementById("bulk-selected-count");
    if (el) el.textContent = String(state.bulkSelected.size);
  }

  async function executeBulkOperation() {
    if (!state.bulkSelected.size) {
      showToast("Lütfen en az bir içerik seçin!", true);
      return;
    }
    const rawAction = document.getElementById("bulk-action-select")?.value;
    if (!rawAction) {
      showToast("Lütfen bir işlem seçin!", true);
      return;
    }
    let action = rawAction;
    let value = null;
    if (rawAction.startsWith("set_visibility_")) {
      action = "set_visibility";
      value = rawAction.split("_")[2];
    } else if (rawAction === "set_featured") {
      value = "true";
    } else if (rawAction === "unset_featured") {
      action = "set_featured";
      value = "false";
    }
    if (
      action === "delete" &&
      !confirm(`${state.bulkSelected.size} içerik kalıcı olarak silinecek. Emin misiniz?`)
    ) {
      return;
    }

    const resp = await apiRequest("/api/v1/admin/contents/bulk", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ids: [...state.bulkSelected], action, value }),
    });
    if (resp.ok) {
      const affected = (resp.data && resp.data.affected) ?? state.bulkSelected.size;
      showToast(`Toplu işlem tamamlandı: ${affected} içerik güncellendi.`);
      state.bulkSelected.clear();
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  function populateVersionHistorySelect() {
    const sel = document.getElementById("version-history-select");
    if (!sel) return;
    if (!state.contents.length) {
      sel.innerHTML = '<option value="">İçerik bulunamadı. Lütfen önce içerik ekleyin.</option>';
      const container = document.getElementById("version-history-list");
      if (container) {
        container.innerHTML =
          '<div style="color:var(--text-muted); padding:20px; text-align:center; font-size:12px;">İçerik bulunamadı. Lütfen önce içerik ekleyin.</div>';
      }
      return;
    }
    sel.innerHTML =
      '<option value="">-- İçerik seçin --</option>' +
      state.contents
        .map((c) => `<option value="${esc(c.id)}">${esc(c.title_tr || c.slug)}</option>`)
        .join("");
  }

  async function loadVersionHistory(page) {
    const id = parseInt(document.getElementById("version-history-select")?.value || "0", 10);
    const container = document.getElementById("version-history-list");
    if (!container || !id) return;
    paginationState.versionHistory = typeof page === "number" ? page : 1;
    const resp = await apiRequest(
      `/api/v1/admin/contents/${encodeURIComponent(id)}/versions?page=${paginationState.versionHistory}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) {
      container.innerHTML =
        '<div style="color:var(--text-muted); padding:20px; text-align:center; font-size:12px;">Bu içerik için versiyon geçmişi bulunamadı.</div>';
      return;
    }
    const versions = unwrapList(resp.payload, "versions");
    if (!versions.length) {
      container.innerHTML =
        '<div style="color:var(--text-muted); padding:20px; text-align:center; font-size:12px;">Bu içerik için versiyon geçmişi bulunamadı.</div>';
      renderPaginationControls("pagination-version-history", resp.meta, loadVersionHistory);
      return;
    }
    container.innerHTML = versions
      .map((v) => {
        const ts = v.created_at ? new Date(v.created_at).toLocaleString("tr-TR") : "—";
        return `<div style="padding:12px; border-bottom:1px solid var(--color-border); font-size:12px;">
          <div style="display:flex; justify-content:space-between; margin-bottom:4px;">
            <strong style="color:#0ea5e9;">v${esc(v.version)}</strong>
            <span style="color:var(--text-muted); font-size:11px;">${esc(ts)}</span>
          </div>
          <div style="margin-bottom:2px;">${esc(v.title_tr)}</div>
          <div style="color:var(--text-muted); font-size:11px;"><i class="fa fa-edit"></i> ${esc(v.changed_by)} · ${esc(v.change_note || "Not yok")}</div>
        </div>`;
      })
      .join("");
    renderPaginationControls("pagination-version-history", resp.meta, loadVersionHistory);
  }

  // =========================================================================
  // TAB 13: CUSTOM FIELDS & CACHE MANAGEMENT
  // =========================================================================

  async function loadCustomFields(page) {
    paginationState.customFields = page || paginationState.customFields;
    const resp = await apiRequest(
      `/api/v1/admin/custom-fields?applies_to=all&page=${paginationState.customFields}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.customFields = unwrapList(resp.payload, "fields");
    renderCustomFields();
    renderPaginationControls("pagination-custom-fields", resp.meta, loadCustomFields);
  }

  function renderCustomFields() {
    const tbody = document.getElementById("tbody-custom-fields");
    if (!tbody) return;
    if (!state.customFields.length) {
      renderTableState(tbody, "empty", { colCount: 8 });
      return;
    }
    tbody.innerHTML = state.customFields
      .map(
        (f) => `<tr>
          <td><code>${esc(f.field_key)}</code></td>
          <td>${esc(f.label_tr)}</td>
          <td>${esc(f.label_en)}</td>
          <td><span class="badge-tag badge-placement">${esc(f.field_type)}</span></td>
          <td style="font-size:11px;">${esc(f.applies_to)}</td>
          <td>${f.is_required ? '<i class="fa fa-check" style="color:var(--color-success);"></i>' : '<i class="fa fa-minus" style="color:var(--text-muted);"></i>'}</td>
          <td>${esc(f.sort_order)}</td>
          <td><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-custom-field" data-key="${esc(f.field_key)}"><i class="fa fa-trash"></i></button></td>
        </tr>`,
      )
      .join("");
  }

  function showCustomFieldForm() {
    const f = document.getElementById("custom-field-form-container");
    if (f) f.style.display = "block";
  }

  function hideCustomFieldForm() {
    const f = document.getElementById("custom-field-form-container");
    if (f) f.style.display = "none";
  }

  async function saveCustomField() {
    const key = document.getElementById("cf-key")?.value?.trim();
    const labelTr = document.getElementById("cf-label-tr")?.value?.trim();
    const labelEn = document.getElementById("cf-label-en")?.value?.trim();
    if (!key || !labelTr) {
      showToast("Alan anahtarı ve TR etiketi zorunludur!", true);
      return;
    }
    const resp = await apiRequest("/api/v1/admin/custom-fields", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        field_key: key,
        label_tr: labelTr,
        label_en: labelEn || labelTr,
        field_type: document.getElementById("cf-type")?.value || "text",
        options_json: document.getElementById("cf-options")?.value || "[]",
        is_required: document.getElementById("cf-required")?.checked || false,
        sort_order: parseInt(document.getElementById("cf-sort-order")?.value || "0", 10),
        applies_to: document.getElementById("cf-applies-to")?.value || "content",
      }),
    });
    if (resp.ok) {
      showToast("Özel alan kaydedildi!");
      hideCustomFieldForm();
      await loadCustomFields();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function deleteCustomField(fieldKey) {
    if (!confirm(`"${fieldKey}" alanı silinecek. Emin misiniz?`)) return;
    const resp = await apiRequest(
      `/api/v1/admin/custom-fields/${encodeURIComponent(fieldKey)}`,
      { method: "DELETE" },
    );
    if (resp.ok) {
      showToast("Özel alan silindi!");
      await loadCustomFields();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  async function loadCacheOperations(page) {
    paginationState.cacheOperations = page || paginationState.cacheOperations;
    const resp = await apiRequest(
      `/api/v1/admin/cache?page=${paginationState.cacheOperations}&per_page=25`,
    );
    if (!resp.ok || !resp.payload) return;
    state.cacheOperations = unwrapList(resp.payload, "operations");
    renderCacheOperations();
    renderPaginationControls("pagination-cache-operations", resp.meta, loadCacheOperations);
  }

  function renderCacheOperations() {
    const tbody = document.getElementById("tbody-cache-operations");
    if (!tbody) return;
    if (!state.cacheOperations.length) {
      renderTableState(tbody, "empty", { colCount: 5 });
      return;
    }
    tbody.innerHTML = state.cacheOperations
      .map((op) => {
        const ts = op.created_at ? new Date(op.created_at).toLocaleString("tr-TR") : "—";
        return `<tr>
          <td style="white-space:nowrap; font-size:11px;">${esc(ts)}</td>
          <td><code style="font-size:11px;">${esc(op.cache_key)}</code></td>
          <td><span style="color:#f59e0b; font-weight:600;">${esc(op.operation)}</span></td>
          <td>${esc(op.actor_email)}</td>
          <td style="font-size:11px; color:var(--text-muted);">${esc(op.details || "-")}</td>
        </tr>`;
      })
      .join("");
  }

  async function flushAllCache() {
    if (!confirm("Tüm önbellek katmanları temizlenecek. Devam etmek istiyor musunuz?")) return;
    const resp = await apiRequest("/api/v1/admin/cache", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ cache_key: "all" }),
    });
    if (resp.ok) {
      showToast("Tüm önbellek katmanları temizlendi!");
      await loadCacheOperations();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  // =========================================================================
  // TAB 14: REVENUE ANALYTICS DASHBOARD
  // =========================================================================

  let _revenueChartInstance = null;

  async function loadRevenueSummary() {
    const fromInput = document.getElementById("revenue-date-from");
    const toInput = document.getElementById("revenue-date-to");
    const today = new Date().toISOString().slice(0, 10);
    const thirtyDaysAgo = new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10);
    const dateFrom = (fromInput && fromInput.value) || thirtyDaysAgo;
    const dateTo = (toInput && toInput.value) || today;
    if (fromInput && !fromInput.value) fromInput.value = dateFrom;
    if (toInput && !toInput.value) toInput.value = dateTo;

    const resp = await apiRequest(
      `/api/v1/admin/analytics/revenue-summary?from=${encodeURIComponent(dateFrom)}&to=${encodeURIComponent(dateTo)}`,
    );
    if (!resp.ok || !resp.data) return;
    const data = resp.data;

    const totalEl = document.getElementById("kpi-revenue-total");
    const countEl = document.getElementById("kpi-revenue-count");
    const subEl = document.getElementById("kpi-revenue-subscription");
    const prodEl = document.getElementById("kpi-revenue-product");
    if (totalEl) {
      totalEl.textContent =
        "₺" + Number(data.total_amount || 0).toLocaleString("tr-TR", { minimumFractionDigits: 2 });
    }
    if (countEl) countEl.textContent = Number(data.invoice_count || 0).toLocaleString("tr-TR");
    const byType = data.by_item_type || {};
    if (subEl) {
      subEl.textContent =
        "₺" +
        Number(byType.subscription || byType.subscription_plan || 0).toLocaleString("tr-TR", {
          minimumFractionDigits: 2,
        });
    }
    if (prodEl) {
      prodEl.textContent =
        "₺" +
        Number(data.single_item_revenue ?? byType.product ?? byType.commerce_product ?? 0).toLocaleString("tr-TR", {
          minimumFractionDigits: 2,
        });
    }

    renderRevenueChart(data.daily || []);
    renderRevenueBreakdown(byType, data.total_amount || 0);
  }

  function renderRevenueChart(dailyData) {
    const canvas = document.getElementById("chart-revenue-daily");
    if (!canvas || typeof Chart === "undefined") return;
    if (_revenueChartInstance) {
      _revenueChartInstance.destroy();
      _revenueChartInstance = null;
    }
    if (!Array.isArray(dailyData) || !dailyData.length) return;

    const labels = dailyData.map((d) => d.date || "?");
    const amounts = dailyData.map((d) => Number(d.amount || 0));

    _revenueChartInstance = new Chart(canvas.getContext("2d"), {
      type: "line",
      data: {
        labels,
        datasets: [
          {
            label: "Günlük Gelir (₺)",
            data: amounts,
            borderColor: "var(--color-success)",
            backgroundColor: "rgba(16,185,129,0.15)",
            tension: 0.3,
            fill: true,
            pointRadius: 3,
            pointHoverRadius: 6,
          },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { labels: { color: "#cbd5e1" } },
          tooltip: {
            callbacks: {
              label: function (ctx) {
                return "₺" + Number(ctx.parsed.y || 0).toLocaleString("tr-TR", { minimumFractionDigits: 2 });
              },
            },
          },
        },
        scales: {
          x: { ticks: { color: "#64748b", maxRotation: 45 }, grid: { color: "rgba(100,116,139,0.1)" } },
          y: {
            ticks: {
              color: "#64748b",
              callback: function (v) {
                return "₺" + Number(v).toLocaleString("tr-TR");
              },
            },
            beginAtZero: true,
            grid: { color: "rgba(100,116,139,0.1)" },
          },
        },
      },
    });
  }

  function renderRevenueBreakdown(byType, total) {
    const tbody = document.getElementById("tbody-revenue-breakdown");
    if (!tbody) return;
    const entries = Object.entries(byType || {});
    if (!entries.length) {
      renderTableState(tbody, "empty", { colCount: 3 });
      return;
    }
    const typeLabels = {
      subscription: "Abonelik",
      subscription_plan: "Abonelik Planı",
      single_article: "Tekil İçerik / Özel Dosya",
      product: "Ürün Satışı",
      commerce_product: "E-Ticaret Ürünü",
      ad_campaign: "Reklam Kampanyası",
      ad: "Reklam",
    };
    tbody.innerHTML = entries
      .map(([type, amount]) => {
        const amtNum = Number(amount || 0);
        const pct = total > 0 ? ((amtNum / total) * 100).toFixed(1) : "0.0";
        const label = typeLabels[type] || type;
        return `<tr>
          <td>${esc(label)}</td>
          <td style="text-align:right;font-weight:600;">₺${esc(amtNum.toLocaleString("tr-TR", { minimumFractionDigits: 2 }))}</td>
          <td style="text-align:right;">%${esc(pct)}</td>
        </tr>`;
      })
      .join("");
  }

  // =========================================================================
  // TAB 15: USER ANALYTICS DASHBOARD
  // =========================================================================

  let _funnelChartInstance = null;

  async function loadUserAnalytics() {
    const [churnResp, ltvResp, funnelResp, cohortResp] = await Promise.all([
      apiRequest("/api/v1/admin/analytics/churn-rate"),
      apiRequest("/api/v1/admin/analytics/user-ltv"),
      apiRequest("/api/v1/admin/analytics/conversion-funnel"),
      apiRequest("/api/v1/admin/analytics/user-cohorts"),
    ]);

    if (churnResp.ok && churnResp.data) {
      const churn = churnResp.data;
      const churnEl = document.getElementById("kpi-churn-rate");
      const lifetimeEl = document.getElementById("kpi-avg-lifetime");
      if (churnEl) churnEl.textContent = "%" + Number(churn.monthly_churn_pct || 0).toFixed(1);
      if (lifetimeEl) lifetimeEl.textContent = Number(churn.avg_lifetime_days || 0) + " gün";
    }

    if (ltvResp.ok && ltvResp.data) {
      const ltv = ltvResp.data;
      const ltvEl = document.getElementById("kpi-avg-ltv");
      const payingEl = document.getElementById("kpi-paying-users");
      if (ltvEl) {
        ltvEl.textContent =
          "₺" + Number(ltv.avg_ltv_try || 0).toLocaleString("tr-TR", { minimumFractionDigits: 2 });
      }
      if (payingEl) payingEl.textContent = Number(ltv.paying_users || 0).toLocaleString("tr-TR");
    }

    if (funnelResp.ok && funnelResp.data) {
      const funnel = funnelResp.data;
      renderFunnelChart(funnel);
      renderFunnelRates(funnel.conversion_rates || {});
    }

    if (cohortResp.ok && cohortResp.payload) {
      const cohortRows = unwrapList(cohortResp.payload, "cohorts");
      renderCohortTable(cohortRows);
    }
  }

  function renderFunnelChart(funnel) {
    const canvas = document.getElementById("chart-conversion-funnel");
    if (!canvas || typeof Chart === "undefined") return;
    if (_funnelChartInstance) {
      _funnelChartInstance.destroy();
      _funnelChartInstance = null;
    }

    const labels = ["Ziyaretçi", "Kayıtlı", "Abone", "Ödeyen"];
    const data = [
      Number(funnel.visitors || 0),
      Number(funnel.registered || 0),
      Number(funnel.subscribed || 0),
      Number(funnel.paid || 0),
    ];
    const colors = ["#64748b", "#0ea5e9", "#f59e0b", "var(--color-success)"];

    _funnelChartInstance = new Chart(canvas.getContext("2d"), {
      type: "bar",
      data: {
        labels,
        datasets: [
          {
            label: "Kullanıcı Sayısı",
            data,
            backgroundColor: colors,
            borderRadius: 2,
            barPercentage: 0.6,
          },
        ],
      },
      options: {
        indexAxis: "y",
        responsive: true,
        maintainAspectRatio: false,
        plugins: {
          legend: { display: false },
          tooltip: {
            callbacks: {
              label: function (ctx) {
                return Number(ctx.parsed.x || 0).toLocaleString("tr-TR") + " kullanıcı";
              },
            },
          },
        },
        scales: {
          x: { ticks: { color: "#64748b" }, grid: { color: "rgba(100,116,139,0.1)" }, beginAtZero: true },
          y: { ticks: { color: "#cbd5e1", font: { size: 13 } }, grid: { display: false } },
        },
      },
    });
  }

  function renderFunnelRates(rates) {
    const container = document.getElementById("funnel-rates");
    if (!container) return;
    const items = [
      { label: "Ziyaretçi → Kayıtlı", value: Number(rates.visitor_to_registered || 0) },
      { label: "Kayıtlı → Abone", value: Number(rates.registered_to_subscribed || 0) },
      { label: "Abone → Ödeyen", value: Number(rates.subscribed_to_paid || 0) },
    ];
    container.innerHTML = items
      .map((item) => {
        const color =
          item.value >= 10
            ? "var(--color-success)"
            : item.value >= 3
              ? "var(--color-warning)"
              : "var(--color-danger)";
        return `<div style="background:var(--color-bg);border:1px solid var(--color-border);border-radius:2px;padding:10px 16px;text-align:center;flex:1;min-width:140px;">
          <div style="font-size:0.7rem;color:var(--text-muted);text-transform:uppercase;">${esc(item.label)}</div>
          <div style="font-size:1.2rem;font-weight:700;color:${color};margin-top:2px;">%${esc(item.value.toFixed(1))}</div>
        </div>`;
      })
      .join("");
  }

  function renderCohortTable(cohorts) {
    const tbody = document.getElementById("tbody-cohort-analysis");
    if (!tbody) return;
    if (!Array.isArray(cohorts) || !cohorts.length) {
      tbody.innerHTML =
        '<tr><td colspan="8" style="text-align:center;color:var(--text-muted);">Cohort verisi bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = cohorts
      .map((row) => {
        const registered = Number(row.registered || 0);
        let cells =
          `<td>${esc(row.cohort_month || "?")}</td>` +
          `<td style="text-align:right;font-weight:600;">${esc(registered.toLocaleString("tr-TR"))}</td>`;
        for (let m = 1; m <= 6; m++) {
          const active = Number(row["active_m" + m] || 0);
          const pctNum = registered > 0 ? Math.round((active / registered) * 100) : 0;
          const opacity = Math.min(1, pctNum / 100);
          const bgColor = `rgba(16,185,129,${(opacity * 0.45).toFixed(2)})`;
          cells += `<td style="text-align:right;background:${bgColor};">${esc(pctNum)}%</td>`;
        }
        return `<tr>${cells}</tr>`;
      })
      .join("");
  }

  // =========================================================================
  // TAB 16: APPROVAL WORKFLOW (INSIDE IIFE, ZERO REFERENCE ERRORS)
  // =========================================================================

  let _pendingApprovalId = null;
  let _pendingApprovalAction = null;

  async function loadPendingApprovals() {
    const tbody = document.getElementById("tbody-pending-approvals");
    const badge = document.getElementById("approval-badge");
    if (!tbody) return;

    const resp = await apiRequest("/api/v1/admin/approvals/pending");
    if (!resp.ok || !resp.payload) {
      renderTableState(tbody, "error", { colCount: 6, errorMsg: resp.errorMessage || "Onay listesi yüklenemedi." });
      return;
    }
    const items = unwrapList(resp.payload, "approvals");
    if (badge) {
      badge.textContent = String(items.length);
      badge.style.display = items.length > 0 ? "inline-flex" : "none";
    }
    if (!items.length) {
      renderTableState(tbody, "empty", { colCount: 6 });
      return;
    }
    tbody.innerHTML = items
      .map((item) => {
        const date = item.requested_at ? new Date(item.requested_at).toLocaleString("tr-TR") : "—";
        const appId = item.id ?? item.approval_id;
        return `<tr>
          <td><strong>${esc(item.title_tr || "")}</strong><br><small style="color:var(--text-muted);">/${esc(item.slug || "")}</small></td>
          <td>${esc(item.author_name || "—")}</td>
          <td><span class="badge-tag">${esc(item.content_type || "—")}</span></td>
          <td>${esc(date)}</td>
          <td><span class="badge-tag badge-status-pending_review">${esc(item.status || "pending")}</span></td>
          <td style="white-space:nowrap;">
            <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-emerald" data-admin-action="open-approval-modal" data-id="${esc(appId)}" data-decision="approve">
              <i class="fa fa-check"></i> Onayla
            </button>
            <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" style="margin-left:4px;" data-admin-action="open-approval-modal" data-id="${esc(appId)}" data-decision="reject">
              <i class="fa fa-times"></i> Reddet
            </button>
          </td>
        </tr>`;
      })
      .join("");
  }

  function openApprovalModal(approvalId, action) {
    _pendingApprovalId = Number(approvalId);
    _pendingApprovalAction = action;
    const modal = document.getElementById("approval-modal");
    const title = document.getElementById("approval-modal-title");
    const btn = document.getElementById("approval-confirm-btn");
    const note = document.getElementById("approval-note");
    if (!modal) return;
    if (note) note.value = "";
    if (action === "approve") {
      if (title) title.textContent = "İçeriği Onayla ve Yayınla";
      if (btn) {
        btn.textContent = "Onayla ve Yayınla";
        btn.className = "btn-cmd btn-cmd-emerald";
      }
    } else {
      if (title) title.textContent = "İçeriği Reddet";
      if (btn) {
        btn.textContent = "Reddet";
        btn.className = "btn-cmd btn-cmd-danger";
      }
    }
    modal.style.display = "flex";
  }

  function closeApprovalModal() {
    const modal = document.getElementById("approval-modal");
    if (modal) modal.style.display = "none";
    _pendingApprovalId = null;
    _pendingApprovalAction = null;
  }

  async function confirmApprovalAction() {
    if (!_pendingApprovalId || !_pendingApprovalAction) return;
    const note = document.getElementById("approval-note")?.value || "";
    const endpoint =
      _pendingApprovalAction === "approve"
        ? `/api/v1/admin/approvals/${encodeURIComponent(_pendingApprovalId)}/approve`
        : `/api/v1/admin/approvals/${encodeURIComponent(_pendingApprovalId)}/reject`;

    const actionTaken = _pendingApprovalAction;
    const resp = await apiRequest(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ note }),
    });
    if (resp.ok) {
      closeApprovalModal();
      showToast(
        actionTaken === "approve"
          ? "İçerik onaylandı ve yayına alındı!"
          : "İçerik reddedildi ve taslak durumuna döndürüldü.",
      );
      await refreshAllData();
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  // =========================================================================
  // TAB 17: SERVER MONITORING & SECURITY (INSIDE IIFE, ZERO XSS)
  // =========================================================================

  let _monitoringRefreshTimer = null;
  let _systemResourcesChart = null;
  let _trafficDistChart = null;
  const _monitoringHistory = { cpu: [], ram: [], labels: [] };
  const MAX_HISTORY_POINTS = 40;

  async function loadMonitoringBundle() {
    const resp = await apiRequest("/api/v1/admin/monitoring/bundle");
    if (!resp.ok || !resp.data) return;
    const bundle = resp.data;
    renderMonitoringKPIs(bundle);
    renderSystemResourcesChart(bundle.system);
    renderServicesPanel(bundle.services);
    renderDomainsTable(bundle.domains);
    renderTrafficDistChart(bundle.traffic);
    renderAttacksPanel(bundle.attacks);
    renderSeoTable(bundle.seo);
    renderNetworkPanel(bundle.system);
  }

  function renderMonitoringKPIs(bundle) {
    const sys = bundle.system;
    if (sys && sys.cpu && sys.memory && sys.disk) {
      const cpuEl = document.getElementById("mon-cpu");
      const ramEl = document.getElementById("mon-ram");
      const diskEl = document.getElementById("mon-disk");
      if (cpuEl) cpuEl.textContent = Number(sys.cpu.user_pct || 0).toFixed(1) + "%";
      if (ramEl) ramEl.textContent = Number(sys.memory.used_pct || 0).toFixed(1) + "%";
      if (diskEl) diskEl.textContent = Number(sys.disk.used_pct || 0).toFixed(1) + "%";

      const cpuSub = document.getElementById("mon-cpu-sub");
      if (cpuSub) {
        cpuSub.textContent = `Load: ${Number(sys.cpu.load_avg_1m || 0).toFixed(2)} / ${Number(sys.cpu.load_avg_5m || 0).toFixed(2)} / ${Number(sys.cpu.load_avg_15m || 0).toFixed(2)}`;
      }
      const ramSub = document.getElementById("mon-ram-sub");
      if (ramSub) {
        const totalGB = (Number(sys.memory.total_kb || 0) / 1048576).toFixed(1);
        const availGB = (Number(sys.memory.available_kb || 0) / 1048576).toFixed(1);
        ramSub.textContent = `${availGB} GB kullanılabilir / ${totalGB} GB toplam`;
      }
      const diskSub = document.getElementById("mon-disk-sub");
      if (diskSub) {
        const freeGB = (Number(sys.disk.free_bytes || 0) / 1073741824).toFixed(0);
        const totalGB = (Number(sys.disk.total_bytes || 0) / 1073741824).toFixed(0);
        diskSub.textContent = `${freeGB} GB boş / ${totalGB} GB toplam`;
      }
    }
    const domainsEl = document.getElementById("mon-domains");
    if (domainsEl) domainsEl.textContent = Array.isArray(bundle.domains) ? String(bundle.domains.length) : "—";
    const domSub = document.getElementById("mon-domains-sub");
    if (domSub && Array.isArray(bundle.domains)) {
      const healthy = bundle.domains.filter((d) => d.http_status && d.http_status < 400).length;
      domSub.textContent = `${healthy} sağlıklı / ${bundle.domains.length} toplam`;
    }
    const attacksEl = document.getElementById("mon-attacks");
    if (attacksEl) attacksEl.textContent = Array.isArray(bundle.attacks) ? String(bundle.attacks.length) : "0";
    const servicesEl = document.getElementById("mon-services");
    if (servicesEl && Array.isArray(bundle.services)) {
      const ok = bundle.services.filter((s) => s.reachable).length;
      servicesEl.textContent = `${ok}/${bundle.services.length}`;
      const svcSub = document.getElementById("mon-services-sub");
      if (svcSub) svcSub.textContent = `${ok} aktif, ${bundle.services.length - ok} erişilemiyor`;
    }
  }

  function renderSystemResourcesChart(system) {
    const canvas = document.getElementById("chart-system-resources");
    if (!canvas || !system || !system.cpu || !system.memory || typeof Chart === "undefined") return;
    const now = new Date().toLocaleTimeString("tr-TR", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    _monitoringHistory.cpu.push(Number(system.cpu.user_pct || 0));
    _monitoringHistory.ram.push(Number(system.memory.used_pct || 0));
    _monitoringHistory.labels.push(now);
    if (_monitoringHistory.cpu.length > MAX_HISTORY_POINTS) {
      _monitoringHistory.cpu.shift();
      _monitoringHistory.ram.shift();
      _monitoringHistory.labels.shift();
    }
    if (_systemResourcesChart) {
      _systemResourcesChart.data.labels = _monitoringHistory.labels;
      _systemResourcesChart.data.datasets[0].data = _monitoringHistory.cpu;
      _systemResourcesChart.data.datasets[1].data = _monitoringHistory.ram;
      _systemResourcesChart.update("none");
    } else {
      _systemResourcesChart = new Chart(canvas, {
        type: "line",
        data: {
          labels: _monitoringHistory.labels,
          datasets: [
            {
              label: "CPU %",
              data: _monitoringHistory.cpu,
              borderColor: "#0ea5e9",
              backgroundColor: "rgba(14,165,233,0.1)",
              tension: 0.3,
              fill: true,
              pointRadius: 2,
            },
            {
              label: "RAM %",
              data: _monitoringHistory.ram,
              borderColor: "#8b5cf6",
              backgroundColor: "rgba(139,92,246,0.1)",
              tension: 0.3,
              fill: true,
              pointRadius: 2,
            },
          ],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          animation: false,
          scales: {
            y: {
              beginAtZero: true,
              max: 100,
              ticks: { callback: (v) => v + "%" },
            },
          },
          plugins: { legend: { position: "top" } },
        },
      });
    }
  }

  function renderServicesPanel(services) {
    const panel = document.getElementById("monitoring-services-panel");
    if (!panel || !Array.isArray(services)) return;
    if (!services.length) {
      panel.innerHTML = '<div style="text-align:center;color:var(--text-muted);">Servis verisi yok</div>';
      return;
    }
    panel.innerHTML = services
      .map((svc) => {
        const color = svc.reachable ? "var(--color-success)" : "var(--color-danger)";
        const icon = svc.reachable ? "fa-check-circle" : "fa-times-circle";
        const latency = svc.latency_ms != null ? `${esc(svc.latency_ms)}ms` : "—";
        return `<div style="background:var(--color-elevated);border:1px solid var(--color-border);border-radius:var(--radius-sm);padding:10px;text-align:center;">
          <i class="fa ${icon}" style="color:${color};font-size:1.4rem;"></i>
          <div style="font-weight:600;font-size:0.85rem;margin-top:4px;">${esc(svc.name)}</div>
          <div style="font-size:0.75rem;color:var(--text-muted);">${latency}</div>
        </div>`;
      })
      .join("");
  }

  function renderDomainsTable(domains) {
    const tbody = document.getElementById("tbody-monitoring-domains");
    if (!tbody || !Array.isArray(domains)) return;
    if (!domains.length) {
      tbody.innerHTML =
        '<tr><td colspan="7" style="text-align:center;color:var(--text-muted);">Domain verisi yok</td></tr>';
      return;
    }
    tbody.innerHTML = domains
      .map((d) => {
        const statusColor = d.http_status && d.http_status < 400 ? "var(--color-success)" : "var(--color-danger)";
        const sslColor =
          d.ssl_days_left != null && d.ssl_days_left <= 30
            ? "var(--color-danger)"
            : d.ssl_days_left != null && d.ssl_days_left <= 90
              ? "#d97706"
              : "var(--color-success)";
        const sslText = d.ssl_days_left != null ? `${esc(d.ssl_days_left)} gün` : "—";
        const dnsIcon = d.dns_resolves
          ? '<i class="fa fa-check" style="color:var(--color-success);"></i>'
          : '<i class="fa fa-times" style="color:var(--color-danger);"></i>';
        const lastCheck = d.last_check ? new Date(d.last_check).toLocaleTimeString("tr-TR") : "—";
        const sslExp = d.ssl_expiry ? String(d.ssl_expiry).split("T")[0] : "—";
        return `<tr>
          <td style="font-weight:600;">${esc(d.domain)}</td>
          <td><span style="color:${statusColor};font-weight:600;">${esc(d.http_status || "ERR")}</span></td>
          <td>${d.response_ms != null ? esc(d.response_ms) + "ms" : "—"}</td>
          <td>${esc(sslExp)}</td>
          <td style="color:${sslColor};font-weight:600;">${sslText}</td>
          <td style="text-align:center;">${dnsIcon}</td>
          <td>${esc(lastCheck)}</td>
        </tr>`;
      })
      .join("");
  }

  function renderTrafficDistChart(traffic) {
    const canvas = document.getElementById("chart-traffic-dist");
    if (!canvas || !traffic || !Array.isArray(traffic.per_domain) || typeof Chart === "undefined") return;
    const sorted = [...traffic.per_domain]
      .sort((a, b) => (b.requests_per_min || 0) - (a.requests_per_min || 0))
      .slice(0, 12);
    const labels = sorted.map((t) => String(t.domain || ""));
    const data = sorted.map((t) => Number(t.requests_per_min || 0));
    const colors = [
      "#c91818",
      "#0ea5e9",
      "#2563eb",
      "#7c3aed",
      "#d97706",
      "var(--color-danger)",
      "#059669",
      "#ca8a04",
      "#6366f1",
      "#ec4899",
      "#14b8a6",
      "#f59e0b",
    ];
    if (_trafficDistChart) {
      _trafficDistChart.data.labels = labels;
      _trafficDistChart.data.datasets[0].data = data;
      _trafficDistChart.update();
    } else {
      _trafficDistChart = new Chart(canvas, {
        type: "doughnut",
        data: { labels, datasets: [{ data, backgroundColor: colors, borderWidth: 0 }] },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { position: "right", labels: { boxWidth: 12, font: { size: 11 } } } },
        },
      });
    }
  }

  function renderAttacksPanel(attacks) {
    const panel = document.getElementById("monitoring-attacks-panel");
    if (!panel) return;
    if (!Array.isArray(attacks) || !attacks.length) {
      panel.innerHTML =
        '<div style="text-align:center;color:var(--text-muted);padding:2rem;"><i class="fa fa-shield" style="font-size:2rem;margin-bottom:0.5rem;display:block;"></i>Son saldırı tespiti yok — sistem güvenli</div>';
      return;
    }
    panel.innerHTML = attacks
      .map((a) => {
        const sev = String(a.severity || "low").toLowerCase();
        const sevColor = sev === "high" ? "var(--color-danger)" : sev === "medium" ? "#d97706" : "#6366f1";
        const time = a.detected_at ? new Date(a.detected_at).toLocaleString("tr-TR") : "—";
        return `<div style="display:flex;align-items:center;gap:12px;padding:8px 12px;border-bottom:1px solid var(--color-border);">
          <div style="width:8px;height:8px;border-radius:50%;background:${sevColor};flex-shrink:0;"></div>
          <div style="flex:1;min-width:0;">
            <div style="font-weight:600;font-size:0.85rem;">${esc(a.attack_type)}</div>
            <div style="font-size:0.75rem;color:var(--text-muted);">${esc(a.source_ip)} → ${esc(a.target_service || "—")}</div>
          </div>
          <div style="text-align:right;flex-shrink:0;">
            <span style="display:inline-block;padding:2px 8px;border-radius:2px;font-size:0.7rem;font-weight:600;background:${sevColor}22;color:${sevColor};">${esc(sev.toUpperCase())}</span>
            <div style="font-size:0.7rem;color:var(--text-muted);margin-top:2px;">${esc(time)}</div>
          </div>
        </div>`;
      })
      .join("");
  }

  function renderSeoTable(seo) {
    const tbody = document.getElementById("tbody-monitoring-seo");
    if (!tbody || !Array.isArray(seo)) return;
    if (!seo.length) {
      tbody.innerHTML =
        '<tr><td colspan="7" style="text-align:center;color:var(--text-muted);">SEO verisi henüz toplanmadı (saatlik kontrol)</td></tr>';
      return;
    }
    const check = (v) =>
      v
        ? '<i class="fa fa-check" style="color:var(--color-success);"></i>'
        : '<i class="fa fa-times" style="color:var(--color-danger);"></i>';
    tbody.innerHTML = seo
      .map((s) => {
        const issues =
          Array.isArray(s.issues) && s.issues.length
            ? s.issues.map((iss) => esc(iss)).join(", ")
            : '<span style="color:var(--color-success);">Sorun yok</span>';
        return `<tr>
          <td style="font-weight:600;">${esc(s.domain)}</td>
          <td style="text-align:center;">${check(s.robots_txt)}</td>
          <td style="text-align:center;">${check(s.sitemap_xml)}</td>
          <td style="text-align:center;">${check(s.meta_title)}</td>
          <td style="text-align:center;">${check(s.meta_description)}</td>
          <td style="text-align:center;">${check(s.canonical)}</td>
          <td style="font-size:0.8rem;">${issues}</td>
        </tr>`;
      })
      .join("");
  }

  function renderNetworkPanel(system) {
    const panel = document.getElementById("monitoring-network-panel");
    if (!panel || !system) return;
    const rxMB = (Number(system.net_rx_per_sec || 0) / 1048576).toFixed(2);
    const txMB = (Number(system.net_tx_per_sec || 0) / 1048576).toFixed(2);
    panel.innerHTML = `<div style="display:flex;gap:2rem;justify-content:center;align-items:center;padding:1rem;">
      <div style="text-align:center;">
        <i class="fa fa-arrow-down" style="color:#059669;font-size:1.5rem;"></i>
        <div style="font-size:1.5rem;font-weight:700;color:#059669;">${esc(rxMB)} MB/s</div>
        <div style="font-size:0.8rem;color:var(--text-muted);">Gelen (RX)</div>
      </div>
      <div style="text-align:center;">
        <i class="fa fa-arrow-up" style="color:#2563eb;font-size:1.5rem;"></i>
        <div style="font-size:1.5rem;font-weight:700;color:#2563eb;">${esc(txMB)} MB/s</div>
        <div style="font-size:0.8rem;color:var(--text-muted);">Giden (TX)</div>
      </div>
    </div>`;
  }

  async function refreshMonitoring() {
    const resp = await apiRequest("/api/v1/admin/monitoring/refresh", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    if (resp.ok) {
      showToast("Sunucu izleme verileri yenileniyor...");
      setTimeout(loadMonitoringBundle, 1800);
    } else {
      showToast(resp.errorMessage, true);
    }
  }

  function startMonitoringAutoRefresh() {
    if (_monitoringRefreshTimer) clearInterval(_monitoringRefreshTimer);
    loadMonitoringBundle();
    _monitoringRefreshTimer = setInterval(loadMonitoringBundle, 15000);
  }

  function stopMonitoringAutoRefresh() {
    if (_monitoringRefreshTimer) {
      clearInterval(_monitoringRefreshTimer);
      _monitoringRefreshTimer = null;
    }
  }

  // =========================================================================
  // CENTRALIZED CSP-CLEAN EVENT DELEGATION (ZERO INLINE HANDLERS)
  // =========================================================================

  function bindDelegatedEvents() {
    document.addEventListener("click", (ev) => {
      const tabBtn = ev.target.closest("[data-tab]");
      if (tabBtn) {
        ev.preventDefault();
        switchTab(tabBtn.getAttribute("data-tab"));
        return;
      }

      const actionEl = ev.target.closest("[data-admin-action]");
      if (!actionEl) return;

      const action = actionEl.getAttribute("data-admin-action");
      switch (action) {
        case "toggle-theme":
          toggleAdminTheme();
          break;
        case "refresh-all":
          refreshAllData();
          break;
        case "logout-operator":
          logoutOperator();
          break;
        case "open-2fa-modal":
          open2faModal();
          break;
        case "close-2fa-modal":
          close2faModal();
          break;
        case "start-2fa-setup":
          start2faSetup();
          break;
        case "reset-content-form":
          resetContentForm();
          break;
        case "trigger-cover-upload":
          document.getElementById("c-cover-file")?.click();
          break;
        case "submit-form-for-review":
          submitFormAndSendForReview();
          break;
        case "edit-content":
          loadContentIntoForm(actionEl.getAttribute("data-id"));
          break;
        case "submit-content-review":
          submitContentForReview(actionEl.getAttribute("data-id"), false);
          break;
        case "delete-content":
          deleteContent(actionEl.getAttribute("data-id"));
          break;
        case "reset-ad-form":
          resetAdForm();
          break;
        case "check-ad-conflict":
          checkAdConflictLive();
          break;
        case "trigger-ad-upload":
          document.getElementById("ad-image-file")?.click();
          break;
        case "edit-ad":
          loadAdIntoForm(actionEl.getAttribute("data-id"));
          break;
        case "delete-ad":
          deleteAd(actionEl.getAttribute("data-id"));
          break;
        case "switch-analytics-subview":
          switchAnalyticsSubview(actionEl.getAttribute("data-subview"));
          break;
        case "export-analytics-json":
          downloadMonthlyConsolidatedReport();
          break;
        case "export-analytics-csv":
          exportAnalyticsCsv();
          break;
        case "decide-corporate":
          decideCorporateRequest(
            actionEl.getAttribute("data-id"),
            actionEl.getAttribute("data-decision"),
          );
          break;
        case "halt-renewed-reminders":
          simulateRenewalHalt();
          break;
        case "refresh-system-health":
          refreshSystemHealth();
          break;
        case "copy-media-url": {
          const url = actionEl.getAttribute("data-url") || "";
          if (navigator.clipboard && url) {
            navigator.clipboard.writeText(url).then(() => showToast("Medya URL kopyalandı: " + url));
          }
          break;
        }
        case "show-role-form":
          showRoleForm();
          break;
        case "hide-role-form":
          hideRoleForm();
          break;
        case "save-role":
          saveRole();
          break;
        case "edit-role":
          editRole(actionEl.getAttribute("data-role"));
          break;
        case "delete-role":
          deleteRole(actionEl.getAttribute("data-role"));
          break;
        case "assign-user-role": {
          const uid = actionEl.getAttribute("data-user-id");
          const sel = document.getElementById("assign-role-sel-" + uid);
          assignRoleToUser(uid, sel ? sel.value : "");
          break;
        }
        case "assign-manual-user-role": {
          const uid = document.getElementById("manual-assign-user-id")?.value;
          const rc = document.getElementById("manual-assign-role-code")?.value;
          assignRoleToUser(uid, rc);
          break;
        }
        case "bulk-select-all":
          selectAllBulkContents();
          break;
        case "bulk-deselect-all":
          deselectAllBulkContents();
          break;
        case "execute-bulk":
          executeBulkOperation();
          break;
        case "show-custom-field-form":
          showCustomFieldForm();
          break;
        case "hide-custom-field-form":
          hideCustomFieldForm();
          break;
        case "save-custom-field":
          saveCustomField();
          break;
        case "delete-custom-field":
          deleteCustomField(actionEl.getAttribute("data-key"));
          break;
        case "flush-cache":
          flushAllCache();
          break;
        case "load-revenue-summary":
          loadRevenueSummary();
          break;
        case "load-user-analytics":
          loadUserAnalytics();
          break;
        case "load-pending-approvals":
          loadPendingApprovals();
          break;
        case "open-approval-modal":
          openApprovalModal(
            actionEl.getAttribute("data-id"),
            actionEl.getAttribute("data-decision"),
          );
          break;
        case "close-approval-modal":
          closeApprovalModal();
          break;
        case "confirm-approval":
          confirmApprovalAction();
          break;
        case "refresh-monitoring":
          refreshMonitoring();
          break;
        case "paginate": {
          const cid = actionEl.getAttribute("data-container");
          const targetPage = parseInt(actionEl.getAttribute("data-page") || "1", 10);
          if (cid && typeof paginationHandlers[cid] === "function" && targetPage >= 1) {
            paginationHandlers[cid](targetPage);
          }
          break;
        }
        default:
          break;
      }
    });

    document.addEventListener("submit", (ev) => {
      const form = ev.target;
      if (!form || !form.id) return;
      switch (form.id) {
        case "form-content-ssot":
          submitContentForm(ev);
          break;
        case "form-ad-campaign":
          submitAdForm(ev);
          break;
        case "form-new-category":
          submitCategoryForm(ev);
          break;
        case "form-new-ssb":
          submitSsbTagForm(ev);
          break;
        case "form-newsletter":
          submitNewsletterForm(ev);
          break;
        case "form-seo-redirect":
          submitSeoRedirect(ev);
          break;
        case "form-site-setting":
          submitSiteSetting(ev);
          break;
        case "form-sys-notif":
          submitSysNotification(ev);
          break;
        case "form-totp-verify-setup":
          verify2faSetup(ev);
          break;
        case "form-totp-disable":
          disable2fa(ev);
          break;
        default:
          break;
      }
    });

    document.addEventListener("change", (ev) => {
      const target = ev.target;
      if (!target) return;

      if (target.id === "c-cover-file") {
        uploadCoverAsset(target);
      } else if (target.id === "ad-image-file") {
        uploadAssetToInput(target, "ad-image-url");
      } else if (target.id === "media-bulk-upload") {
        bulkUploadMedia(target);
      } else if (target.id === "c-is-sponsored") {
        toggleSponsoredFields();
      } else if (
        target.id === "ad-slot-code" ||
        target.name === "ad_inst" ||
        target.name === "ad_ssb"
      ) {
        checkAdConflictLive();
      } else if (target.id === "seo-preview-select") {
        updateSeoPreview();
      } else if (target.id === "audit-filter-action") {
        filterAuditLogs();
      } else if (target.id === "bulk-select-all-cb") {
        toggleBulkSelectAll(target);
      } else if (target.classList && target.classList.contains("bulk-cb")) {
        toggleBulkSelect(target.getAttribute("data-id"), target.checked);
      } else if (target.id === "version-history-select") {
        loadVersionHistory(1);
      }
    });

    document.addEventListener("input", (ev) => {
      const target = ev.target;
      if (!target) return;
      if (target.id === "filter-content-search") {
        renderContentsTable();
      } else if (target.id === "media-search") {
        filterMediaLibrary();
      } else if (target.id === "audit-filter-actor") {
        filterAuditLogs();
      } else if (target.id === "user-role-search") {
        renderAdminUsers();
      }
    });
  }

  document.addEventListener("DOMContentLoaded", () => {
    if (!document.querySelector(".app-layout, .app-shell")) return;

    // Unified theme initialization — supports both old and new keys
    try {
      const storedTheme = localStorage.getItem("msi_theme") || localStorage.getItem("msi_admin_theme");
      if (storedTheme === "light") {
        document.documentElement.setAttribute("data-theme", "light");
        document.body.classList.add("admin-light-mode");
        document.body.classList.add("gw-light-mode");
      } else if (storedTheme === "dark") {
        document.documentElement.removeAttribute("data-theme");
        document.body.classList.remove("admin-light-mode");
        document.body.classList.remove("gw-light-mode");
      } else if (window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches) {
        document.documentElement.setAttribute("data-theme", "light");
        document.body.classList.add("admin-light-mode");
        document.body.classList.add("gw-light-mode");
      }
    } catch (_) {}

    const ident = getOperatorIdentity();
    const emailBadge = document.getElementById("operator-email-display");
    const roleBadge = document.getElementById("operator-role-display");
    if (emailBadge && ident.email) emailBadge.textContent = ident.email;
    if (roleBadge && ident.role) roleBadge.textContent = ident.role;

    bindDelegatedEvents();
    toggleSponsoredFields();

    // Activate first sidebar nav item by default
    const firstNavItem = document.querySelector(".sidebar-link[data-tab]");
    if (firstNavItem) {
      switchTab(firstNavItem.getAttribute("data-tab"));
    }

    refreshAllData();
  });

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
    submitContentForReview,
    deleteContent,
    checkAdConflictLive,
    loadAdIntoForm,
    resetAdForm,
    submitAdForm,
    deleteAd,
    downloadMonthlyConsolidatedReport,
    exportAnalyticsCsv,
    decideCorporateRequest,
    submitCategoryForm,
    submitSsbTagForm,
    submitNewsletterForm,
    simulateRenewalHalt,
    submitSeoRedirect,
    submitSiteSetting,
    submitSysNotification,
    uploadCoverAsset,
    refreshSystemHealth,
    renderAnalyticsCharts,
    filterMediaLibrary,
    bulkUploadMedia,
    updateSeoPreview,
    showRoleForm,
    hideRoleForm,
    saveRole,
    editRole,
    deleteRole,
    assignRoleToUser,
    filterAuditLogs,
    toggleBulkSelect,
    selectAllBulkContents,
    deselectAllBulkContents,
    toggleBulkSelectAll,
    executeBulkOperation,
    loadVersionHistory,
    showCustomFieldForm,
    hideCustomFieldForm,
    saveCustomField,
    deleteCustomField,
    flushAllCache,
    loadRevenueSummary,
    loadUserAnalytics,
    loadPendingApprovals,
    openApprovalModal,
    closeApprovalModal,
    confirmApprovalAction,
    loadMonitoringBundle,
    refreshMonitoring,
    startMonitoringAutoRefresh,
    stopMonitoringAutoRefresh,
    open2faModal,
    close2faModal,
    start2faSetup,
    _ctx: {
      state,
      paginationState,
      apiRequest,
      showToast,
      esc,
      unwrapList,
      unwrapData,
      renderPaginationControls,
      refreshAllData,
      switchTab,
      loadMediaLibrary,
      loadUserRoles,
      loadAdminUsers,
      loadVersionHistory,
      loadRevenueSummary,
      filterMediaLibrary,
    },
  };
})();
