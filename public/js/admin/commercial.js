(function () {
  "use strict";
  const ctx = window.MSIAdmin?._ctx;
  if (!ctx) return;
  const { state, paginationState, byId, val, setVal, esc, apiRequest, unwrapData, unwrapList, uploadFile, showToast, renderTableState, renderPaginationControls, renderChipSlice, charts, locale, t, svgIcon, confirmDialog, loadCustomFieldValues, validateCustomFields, saveCustomFieldValues } = ctx;

  const ADS_PER_PAGE = 12;
  let _pendingAdOverridePayload = null;

  function sponsorTypeLabel(k) {
    const trMap = { defense_oem: "Savunma OEM", auxiliary_service: "Yan Hizmet", in_house: "Öz Tanıtım" };
    const enMap = { defense_oem: "Defence OEM", auxiliary_service: "Auxiliary Service", in_house: "In-House" };
    return t("admin.commercial.sponsor_type." + k, trMap[k] || k, enMap[k] || k);
  }
  function productTypeLabel(k) {
    const trMap = { book: "Prestij Kitabı", report: "Stratejik Rapor", photo_license: "Fotoğraf Lisansı", single_article: "Tekil Dosya" };
    const enMap = { book: "Prestige Book", report: "Strategic Report", photo_license: "Photo License", single_article: "Single Dossier" };
    return t("admin.commercial.product_type." + k, trMap[k] || k, enMap[k] || k);
  }

  async function loadAds() {
    if (!ctx.hasPerm("ads:read")) return;
    const resp = await apiRequest("/api/v1/admin/ads");
    if (!resp.ok) return;
    state.ads = unwrapList(resp.payload, "ads");
    renderAdsTable(1);
    ctx.renderKpis?.();
  }

  function renderAdsTable(page) {
    const tbody = byId("tbody-ads");
    if (!tbody) return;
    if (typeof page === "number") paginationState.ads = page;
    const curPage = paginationState.ads || 1;
    const q = val("filter-ad-search").toLowerCase();
    const stFilter = val("filter-ad-status");

    const filtered = state.ads.filter((a) => {
      if (stFilter === "active" && a.is_active === false) return false;
      if (stFilter === "inactive" && a.is_active !== false) return false;
      if (stFilter === "defense_oem" && a.sponsor_type !== "defense_oem") return false;
      if (!q) return true;
      return [a.sponsor_name, a.title_tr, a.slot_code, a.dimensions, ...(a.target_institutions || [])]
        .some((f) => String(f || "").toLowerCase().includes(q));
    });

    if (!filtered.length) {
      renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_match_ad", "Eşleşen reklam kampanyası bulunamadı.") });
      renderPaginationControls("pagination-ads", null, renderAdsTable);
      return;
    }

    const totalPages = Math.max(1, Math.ceil(filtered.length / ADS_PER_PAGE));
    const safePage = Math.min(curPage, totalPages);
    const slice = filtered.slice((safePage - 1) * ADS_PER_PAGE, safePage * ADS_PER_PAGE);
    const canWrite = ctx.hasPerm("ads:write");

    tbody.innerHTML = slice.map((a) => {
      const imps = Number(a.impressions_count || 0), clicks = Number(a.clicks_count || 0);
      const ctr = imps > 0 ? ((clicks / imps) * 100).toFixed(2) : "0.00";
      const stBadge = a.is_active === false ? `<span class="badge-tag badge-status-draft">${esc(t("admin.status.inactive", "Pasif"))}</span>` : `<span class="badge-tag badge-status-published">${esc(t("admin.status.active", "Aktif"))}</span>`;
      const ops = canWrite
        ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-ad" data-id="${esc(a.id)}" title="${esc(t("admin.action.edit", "Düzenle"))}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-ad" data-id="${esc(a.id)}" title="${esc(t("admin.action.delete", "Sil"))}">${svgIcon("trash")}</button></div>`
        : '<span class="text-muted text-xs">—</span>';
      const instChips = renderChipSlice(a.target_institutions, "") || `<span class="text-muted text-xs">${esc(t("admin.label.all_institutions", "Tüm Kurumlar"))}</span>`;
      const ssbChips = renderChipSlice(a.target_ssb_tags, "badge-ssb") || `<span class="text-muted text-xs">${esc(t("admin.label.general", "Genel"))}</span>`;
      return `<tr>
        <td class="text-mono text-xs"><strong>${esc(a.slot_code)}</strong><div class="text-muted">${esc(a.dimensions)}</div><div class="mt-1">${stBadge}</div></td>
        <td><div class="font-semibold">${esc(a.sponsor_name)}</div><span class="badge-tag ${a.sponsor_type === "defense_oem" ? "badge-sponsored" : "badge-paywall-0"}">${esc(sponsorTypeLabel(a.sponsor_type))}</span></td>
        <td><div class="font-semibold">${esc(a.title_tr)}</div><div class="text-xs text-muted">${esc(t("admin.label.priority", "Öncelik:", "Priority:"))} <strong>${esc(a.budget_score)}</strong>${a.is_default_fallback ? ` · ${esc(t("admin.label.fallback", "Yedek", "Fallback"))}` : ""}</div></td>
        <td><div class="mb-1">${instChips}</div><div>${ssbChips}</div></td>
        <td class="text-mono text-xs text-right"><strong>${imps.toLocaleString(locale())}</strong> ${esc(t("admin.label.impressions_short", "gös.", "imp."))}<div class="text-info">${clicks.toLocaleString(locale())} ${esc(t("admin.label.clicks_short", "tık", "clicks"))} (%${ctr})</div></td>
        <td>${ops}</td>
      </tr>`;
    }).join("");

    renderPaginationControls(
      "pagination-ads",
      { page: safePage, total_pages: totalPages, total_count: filtered.length },
      renderAdsTable,
    );
  }

  function resetAdForm() {
    byId("form-ad-campaign")?.reset();
    setVal("ad-id", "0");
    setVal("ad-start-date", "");
    setVal("ad-end-date", "");
    setVal("ad-rotation-mode", "weighted");
    byId("ad-conflict-banner")?.classList.add("hidden");
    if (byId("ad-form-heading")) byId("ad-form-heading").textContent = t("admin.form.new_ad_campaign", "Yeni Reklam Kampanyası");
    ctx.switchLangTab(byId("form-ad-campaign"), "tr");
    return loadCustomFieldValues("ad-custom-fields", "ad", "0");
  }

  function loadAdIntoForm(id) {
    const a = state.ads.find((x) => Number(x.id) === Number(id));
    if (!a) return;
    setVal("ad-id", a.id); setVal("ad-slot-code", a.slot_code || "ad-mag-top");
    setVal("ad-sponsor-type", a.sponsor_type || "defense_oem"); setVal("ad-sponsor-name", a.sponsor_name || "");
    setVal("ad-dimensions", a.dimensions || "970x90"); setVal("ad-budget-score", a.budget_score || 85);
    setVal("ad-start-date", (a.start_date || "").slice(0, 10));
    setVal("ad-end-date", (a.end_date || "").slice(0, 10));
    setVal("ad-rotation-mode", a.rotation_mode || "weighted");
    setVal("ad-title-tr", a.title_tr || ""); setVal("ad-title-en", a.title_en || "");
    setVal("ad-desc-tr", a.desc_tr || ""); setVal("ad-desc-en", a.desc_en || "");
    setVal("ad-target-url", a.target_url || "https://www.msidergisi.com");
    setVal("ad-image-url", a.image_url || ""); setVal("ad-badge-label", a.badge_label || t("admin.commercial.default_badge_label", "SPONSORLU REKLAM ALANI"));
    setVal("ad-pill-style", a.pill_style || "blue");
    setVal("ad-target-categories", (a.target_categories || []).join(", "));
    if (byId("ad-is-fallback")) byId("ad-is-fallback").checked = !!a.is_default_fallback;
    if (byId("ad-is-popup")) byId("ad-is-popup").checked = !!a.is_floating_popup;
    if (byId("ad-is-newsletter")) byId("ad-is-newsletter").checked = !!a.is_newsletter_ad;
    if (byId("ad-is-active")) byId("ad-is-active").checked = a.is_active !== false;
    const iSet = new Set(a.target_institutions || []), sSet = new Set(a.target_ssb_tags || []);
    document.querySelectorAll('input[name="ad_inst"]').forEach((c) => { c.checked = iSet.has(c.value); });
    document.querySelectorAll('input[name="ad_ssb"]').forEach((c) => { c.checked = sSet.has(c.value); });
    if (byId("ad-form-heading")) byId("ad-form-heading").textContent = `${t("admin.form.edit_ad_campaign", "Kampanya Düzenle")} (#${a.id} — ${a.sponsor_name})`;
    return loadCustomFieldValues("ad-custom-fields", "ad", String(a.id));
  }

  async function checkAdConflictLive() {
    if (!ctx.hasPerm("ads:read") || !byId("ad-slot-code")) return;
    const resp = await apiRequest("/api/v1/admin/ads/check-conflict", {
      method: "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        slot_code: val("ad-slot-code"),
        sponsor_name: val("ad-sponsor-name") || "Yeni Kampanya",
        start_date: val("ad-start-date") || null,
        end_date: val("ad-end-date") || null,
        rotation_mode: val("ad-rotation-mode") || "weighted",
        target_institutions: Array.from(document.querySelectorAll('input[name="ad_inst"]:checked')).map((e) => e.value),
        target_ssb_tags: Array.from(document.querySelectorAll('input[name="ad_ssb"]:checked')).map((e) => e.value),
      }),
    });
    const banner = byId("ad-conflict-banner"), txt = byId("ad-conflict-text");
    const w = unwrapData(resp.payload);
    if (!banner || !txt || !w || typeof w.has_conflict !== "boolean") return;
    banner.classList.remove("hidden");
    banner.classList.toggle("conflict-danger", w.has_conflict);
    const list = (w.conflicting_campaigns || []).map(esc).join(", ");
    txt.innerHTML = w.has_conflict
      ? `<strong>${esc(t("admin.commercial.conflict_warning", "KOTA / ÇAKIŞMA UYARISI"))} (${esc(w.current_count)}/${esc(w.quota_limit)}):</strong> ${esc(w.message_tr)} ${list ? `(${list})` : ""}`
      : `<strong>${esc(t("admin.commercial.quota_available", "KOTA UYGUN"))} (${esc(w.current_count)}/${esc(w.quota_limit)}):</strong> ${esc(w.message_tr || t("admin.commercial.quota_ok_default", "Seçilen reklam kuşağında kota müsait."))}`;
  }

  async function loadRevenueSummary() {
    if (!ctx.hasPerm("revenue:read")) return;
    const today = new Date().toISOString().slice(0, 10);
    const monthAgo = new Date(Date.now() - 30 * 86400000).toISOString().slice(0, 10);
    if (byId("revenue-date-from") && !val("revenue-date-from")) setVal("revenue-date-from", monthAgo);
    if (byId("revenue-date-to") && !val("revenue-date-to")) setVal("revenue-date-to", today);
    const from = val("revenue-date-from") || monthAgo, to = val("revenue-date-to") || today;
    const resp = await apiRequest(`/api/v1/admin/analytics/revenue-summary?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`);
    if (!resp.ok || !resp.data) {
      charts.showEmpty("chart-revenue-daily", resp.errorMessage || t("admin.state.error", "Veri yüklenemedi.", "Failed to load data."), "exclamation-triangle");
      return;
    }
    const d = resp.data, byType = d.by_item_type || {};
    const fmt = (n) => "₺" + Number(n || 0).toLocaleString(locale(), { minimumFractionDigits: 2 });
    if (byId("kpi-revenue-total")) byId("kpi-revenue-total").textContent = fmt(d.total_amount);
    if (byId("kpi-revenue-count")) byId("kpi-revenue-count").textContent = Number(d.invoice_count || 0).toLocaleString(locale());
    if (byId("kpi-revenue-subscription")) byId("kpi-revenue-subscription").textContent = fmt(byType.subscription ?? byType.subscription_plan);
    if (byId("kpi-revenue-product")) byId("kpi-revenue-product").textContent = fmt(d.single_item_revenue ?? byType.product ?? byType.commerce_product);

    const daily = Array.isArray(d.daily) ? d.daily : [];
    if (!daily.length) {
      charts.showEmpty("chart-revenue-daily", t("admin.commercial.revenue_empty", "Seçilen tarih aralığında tahsil edilmiş ödeme kaydı bulunmuyor.", "No collected payment records found in the selected date range."), "line-chart");
    } else {
      charts.mountOrUpdate("chart-revenue-daily", (pal) => ({
        type: "line",
        data: {
          labels: daily.map((x) => x.date || "?"),
          datasets: [{ label: t("admin.chart.daily_collection", "Günlük Tahsilat (₺)", "Daily Collection (₺)"), data: daily.map((x) => Number(x.amount || 0)), borderColor: pal.success, backgroundColor: pal.successSoft, tension: 0.3, fill: true }],
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { labels: { color: pal.sec } } },
          scales: {
            x: { ticks: { color: pal.muted }, grid: { color: pal.grid } },
            y: { beginAtZero: true, ticks: { color: pal.muted }, grid: { color: pal.grid } },
          },
        },
      }));
    }

    const tbody = byId("tbody-revenue-breakdown");
    if (tbody) {
      const entries = Object.entries(byType);
      const labels = { subscription: t("admin.commercial.revenue_subscription", "Abonelik"), subscription_plan: t("admin.commercial.revenue_subscription_plan", "Abonelik Paketi"), single_article: t("admin.commercial.revenue_single_article", "Tekil Dosya"), product: t("admin.commercial.revenue_product", "Ürün Satışı"), commerce_product: t("admin.commercial.revenue_commerce_product", "Mağaza Ürünü"), ad_campaign: t("admin.commercial.revenue_sponsorship", "Sponsorluk") };
      if (!entries.length) renderTableState(tbody, "empty", { colCount: 3, emptyMsg: t("admin.state.no_revenue_period", "Seçili dönemde gelir kalemi kaydı yok.") });
      else {
        const tot = Number(d.total_amount || 0);
        tbody.innerHTML = entries.map(([k, v]) => {
          const amt = Number(v || 0), pct = tot > 0 ? ((amt / tot) * 100).toFixed(1) : "0.0";
          return `<tr><td>${esc(labels[k] || k)}</td><td class="text-right font-semibold text-mono">${esc(fmt(amt))}</td><td class="text-right text-mono">%${esc(pct)}</td></tr>`;
        }).join("");
      }
    }
  }

  async function loadPlans() {
    if (!ctx.hasPerm("subscriptions:read|commerce:read|analytics:read")) return;
    const resp = await apiRequest("/api/v1/admin/subscription-plans");
    if (!resp.ok) return;
    state.plans = unwrapList(resp.payload, "plans");
    const tbody = byId("tbody-sub-plans");
    if (!tbody) return;
    if (!state.plans.length) { renderTableState(tbody, "empty", { colCount: 5 }); return; }
    const canWrite = ctx.hasPerm("subscriptions:write|commerce:manage");
    tbody.innerHTML = state.plans.map((p) => `<tr>
      <td class="text-mono text-xs text-info">${esc(p.code)}</td>
      <td><strong>${esc(p.name_tr)}</strong><div class="text-xs text-muted">${esc(p.name_en)}</div></td>
      <td class="text-mono text-right font-bold">₺${Number(p.price_try || 0).toLocaleString(locale())}</td>
      <td class="text-xs">${esc(p.period_label_tr)}</td>
      <td>${canWrite ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-plan" data-code="${esc(p.code)}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-plan" data-code="${esc(p.code)}">${svgIcon("trash")}</button></div>` : '<span class="text-muted text-xs">—</span>'}</td>
    </tr>`).join("");
  }

  async function loadProducts() {
    if (!ctx.hasPerm("commerce:read|subscriptions:read|analytics:read")) return;
    const resp = await apiRequest("/api/v1/admin/commerce-products");
    if (!resp.ok) return;
    state.products = unwrapList(resp.payload, "products");
    const tbody = byId("tbody-commerce-products");
    if (!tbody) return;
    if (!state.products.length) { renderTableState(tbody, "empty", { colCount: 5 }); return; }
    const canWrite = ctx.hasPerm("commerce:write|commerce:manage");
    tbody.innerHTML = state.products.map((pr) => `<tr>
      <td class="text-mono text-xs text-info">${esc(pr.sku)}</td>
      <td><span class="badge-tag">${esc(productTypeLabel(pr.product_type))}</span></td>
      <td><strong>${esc(pr.title_tr)}</strong><div class="text-xs text-muted">${esc(pr.author_or_source)}</div></td>
      <td class="text-mono text-right font-bold">₺${Number(pr.price_try || 0).toLocaleString(locale())}</td>
      <td>${canWrite ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-product" data-id="${esc(pr.id)}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-product" data-id="${esc(pr.id)}">${svgIcon("trash")}</button></div>` : '<span class="text-muted text-xs">—</span>'}</td>
    </tr>`).join("");
  }

  async function loadInvoices(page) {
    if (!ctx.hasPerm("invoices:read|commerce:read|analytics:read")) return;
    const p = typeof page === "number" ? page : paginationState.invoices || 1;
    paginationState.invoices = p;
    const resp = await apiRequest(`/api/v1/admin/invoices?page=${p}&per_page=20`);
    if (!resp.ok) return;
    const list = unwrapList(resp.payload, "invoices"), tbody = byId("tbody-invoices");
    if (tbody) {
      if (!list.length) renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_invoices", "Kayıtlı sipariş veya fatura bulunmuyor.") });
      else {
        const canWrite = ctx.hasPerm("invoices:write|commerce:manage");
        const invStatusLabel = (raw) => {
          const k = String(raw || "pending").trim().toLowerCase();
          const trMap = { paid: "Ödendi", pending: "Bekliyor", failed: "Başarısız", cancelled: "İptal Edildi", refunded: "İade Edildi" };
          const enMap = { paid: "Paid", pending: "Pending", failed: "Failed", cancelled: "Cancelled", refunded: "Refunded" };
          return t("admin.status.invoice_" + k, trMap[k] || raw, enMap[k] || raw);
        };
        tbody.innerHTML = list.map((inv) => {
          const paid = String(inv.payment_status || "").toLowerCase() === "paid";
          const st = paid
            ? `<span class="badge-tag badge-paywall-0">${esc(t("admin.status.paid", "Ödendi", "Paid"))}</span>`
            : `<span class="badge-tag badge-paywall-2">${esc(invStatusLabel(inv.payment_status))}</span>`;
          const act = paid ? `<span class="text-xs text-muted">${esc(t("admin.status.completed", "Tamamlandı"))}</span>`
            : canWrite ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-primary" data-admin-action="mark-paid" data-id="${esc(inv.id)}">${svgIcon("check")} ${esc(t("admin.action.mark_paid", "Ödendi İşaretle"))}</button>`
            : `<span class="text-xs text-muted">${esc(t("admin.status.pending_invoice", "Bekliyor"))}</span>`;
          return `<tr>
            <td class="text-mono text-xs">${esc(inv.invoice_no)}<br><span class="text-muted">${esc(inv.created_at)}</span></td>
            <td>${esc(inv.full_name)}<div class="text-xs text-muted">${esc(inv.user_email)}</div></td>
            <td>${esc(inv.item_title)}<div class="text-xs text-mono text-muted">${esc(inv.item_code)}</div></td>
            <td class="text-mono text-right font-bold">₺${Number(inv.amount_try || 0).toLocaleString(locale())}</td>
            <td>${st}</td><td>${act}</td>
          </tr>`;
        }).join("");
      }
    }
    renderPaginationControls("pagination-invoices", resp.meta, loadInvoices);
  }

  function renderCorporateTable() {
    const tbody = byId("tbody-corporate-requests");
    if (!tbody) return;
    const stFilter = val("filter-corp-status");
    const list = state.corporateRequests.filter((r) => !stFilter || r.status === stFilter);
    if (!list.length) { renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_corporate_requests", "Bekleyen veya eşleşen kurumsal başvuru bulunmuyor.") }); return; }
    const canDecide = ctx.hasPerm("corporate_requests:decide");
    tbody.innerHTML = list.map((req) => {
      const st = req.status === "approved" ? `<span class="badge-tag badge-paywall-0">${svgIcon("check")} ${esc(t("admin.status.approved", "Onaylandı"))}</span>`
        : req.status === "rejected" ? `<span class="badge-tag badge-sponsored">${svgIcon("times")} ${esc(t("admin.status.rejected", "Reddedildi"))}</span>`
        : `<span class="badge-tag badge-paywall-1">${svgIcon("clock-o")} ${esc(t("admin.status.pending", "Bekliyor"))}</span>`;
      const ops = canDecide ? `<div class="flex gap-2">
        <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-emerald" data-admin-action="decide-corporate" data-id="${esc(req.id)}" data-decision="approved">${svgIcon("check")} ${esc(t("admin.action.approve", "Onayla"))}</button>
        <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="decide-corporate" data-id="${esc(req.id)}" data-decision="rejected">${svgIcon("times")} ${esc(t("admin.action.reject", "Reddet"))}</button>
      </div>` : '<span class="text-xs text-muted">—</span>';
      return `<tr>
        <td class="text-mono text-xs"><strong>#${esc(req.id)}</strong><br><span class="text-muted">${esc(req.created_at || "")}</span></td>
        <td><div class="font-semibold">${esc(req.full_name)}</div><div class="text-xs text-info">${esc(req.email)}</div></td>
        <td><span class="badge-tag">${esc(req.organization_category)}</span><div class="text-xs font-semibold mt-1">${esc(req.organization_name)}</div></td>
        <td class="text-sm text-secondary">${esc(req.request_message)}</td>
        <td>${st}</td><td>${ops}</td>
      </tr>`;
    }).join("");
  }

  async function loadCorporateRequests() {
    if (!ctx.hasPerm("corporate_requests:read")) return;
    const resp = await apiRequest("/api/v1/admin/corporate-requests");
    if (!resp.ok) return;
    state.corporateRequests = unwrapList(resp.payload, "requests");
    ctx.renderKpis?.();
    renderCorporateTable();
  }

  async function submitAdPayload(payload) {
    if (validateCustomFields && !validateCustomFields("ad-custom-fields")) return;
    const r = await apiRequest("/api/v1/admin/ads", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) });
    if (r.ok) {
      const savedId = r.data?.id || payload.id;
      if (savedId) {
        const cfRes = await saveCustomFieldValues("ad-custom-fields", "ad", String(savedId));
        if (cfRes && !cfRes.ok) {
          setVal("ad-id", String(savedId));
          _pendingAdOverridePayload = null;
          ctx.closeModal?.("ad-conflict-modal");
          showToast(cfRes.errorMessage || t("admin.toast.custom_fields_save_failed", "Özel alan değerleri kaydedilemedi.", "Failed to save custom field values."), true);
          await loadAds();
          return;
        }
      }
      _pendingAdOverridePayload = null;
      ctx.closeModal?.("ad-conflict-modal");
      showToast(payload.id > 0 ? t("admin.toast.ad_updated", "Kampanya #${id} güncellendi.").replace("${id}", payload.id) : t("admin.toast.ad_saved", "Kampanya kaydedildi."));
      resetAdForm();
      ctx.closeDrawer("drawer-ad");
      await loadAds();
      return;
    }
    if (r.status === 409) {
      _pendingAdOverridePayload = payload;
      const cc = r.payload?.conflict_check || r.data?.conflict_check || {};
      const msgEl = byId("ad-conflict-modal-msg");
      if (msgEl) msgEl.textContent = cc.message_tr || r.errorMessage || t("admin.commercial.conflict_override_prompt", "Bu reklam kuşağı ve hedefleme kombinasyonunda kota doludur. Yine de kampanyayı zorla kaydetmek istiyor musunuz?");
      const listEl = byId("ad-conflict-modal-list");
      const conflicts = Array.isArray(cc.conflicting_campaigns) ? cc.conflicting_campaigns : [];
      if (listEl) {
        listEl.innerHTML = conflicts.map((name) => `<li>${esc(name)}</li>`).join("");
      }
      ctx.openModal?.("ad-conflict-modal");
      return;
    }
    showToast(r.errorMessage, true);
  }

  ctx.registerActions({
    "open-ad-drawer": () => { resetAdForm(); ctx.openDrawer("drawer-ad"); },
    "reset-ad-form": resetAdForm,
    "check-ad-conflict": checkAdConflictLive,
    "trigger-ad-upload": () => byId("ad-image-file")?.click(),
    "edit-ad": (el) => { loadAdIntoForm(el.getAttribute("data-id")); ctx.openDrawer("drawer-ad"); },
    "close-ad-conflict-modal": () => { _pendingAdOverridePayload = null; ctx.closeModal?.("ad-conflict-modal"); },
    "confirm-ad-override": async () => {
      if (!_pendingAdOverridePayload) { ctx.closeModal?.("ad-conflict-modal"); return; }
      await submitAdPayload({ ..._pendingAdOverridePayload, force_override: true });
    },
    "delete-ad": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_ad", "#${id} reklam kampanyası silinsin mi?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/ads/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.ad_deleted", "Kampanya #${id} silindi.").replace("${id}", id)); await loadAds(); } else showToast(r.errorMessage, true);
    },
    "load-revenue-summary": loadRevenueSummary,
    "reset-sub-plan-form": () => {
      byId("form-sub-plan")?.reset();
      setVal("sp-sort", "10");
      if (byId("sp-featured")) byId("sp-featured").checked = false;
    },
    "edit-plan": (el) => {
      const p = state.plans.find((x) => x.code === el.getAttribute("data-code"));
      if (!p) return;
      const det = byId("form-sub-plan")?.closest("details");
      if (det) det.open = true;
      setVal("sp-code", p.code); setVal("sp-name-tr", p.name_tr); setVal("sp-name-en", p.name_en);
      setVal("sp-price", p.price_try ?? 0); setVal("sp-period-tr", p.period_label_tr); setVal("sp-period-en", p.period_label_en);
      setVal("sp-features-tr", (p.features_tr || []).join("\n")); setVal("sp-features-en", (p.features_en || []).join("\n"));
      if (byId("sp-featured")) byId("sp-featured").checked = !!p.is_featured;
      setVal("sp-sort", p.sort_order ?? 10);
      det?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      byId("sp-name-tr")?.focus();
      showToast(t("admin.toast.plan_editing", "\"${name}\" planı düzenlemeye alındı.", "Plan \"${name}\" loaded for editing.").replace("${name}", p.name_tr));
    },
    "delete-plan": async (el) => {
      const code = el.getAttribute("data-code");
      if (!(await confirmDialog(t("admin.confirm.delete_plan", "\"${code}\" planı silinsin mi?").replace("${code}", code)))) return;
      const r = await apiRequest(`/api/v1/admin/subscription-plans/${encodeURIComponent(code)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.plan_deleted", "Plan \"${code}\" silindi.").replace("${code}", code)); await loadPlans(); } else showToast(r.errorMessage, true);
    },
    "reset-commerce-product-form": () => {
      byId("form-commerce-product")?.reset();
      setVal("cp-id", "0");
      if (byId("cp-available")) byId("cp-available").checked = true;
    },
    "edit-product": (el) => {
      const pr = state.products.find((x) => Number(x.id) === Number(el.getAttribute("data-id")));
      if (!pr) return;
      const det = byId("form-commerce-product")?.closest("details");
      if (det) det.open = true;
      setVal("cp-id", pr.id || 0); setVal("cp-sku", pr.sku); setVal("cp-type", pr.product_type || "book");
      setVal("cp-title-tr", pr.title_tr); setVal("cp-title-en", pr.title_en); setVal("cp-author", pr.author_or_source);
      setVal("cp-price", pr.price_try ?? 0); setVal("cp-res", pr.file_resolution_or_pages); setVal("cp-cover", pr.cover_image);
      setVal("cp-desc-tr", pr.description_tr); setVal("cp-desc-en", pr.description_en);
      if (byId("cp-available")) byId("cp-available").checked = !!pr.is_available;
      det?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      byId("cp-title-tr")?.focus();
      showToast(t("admin.toast.product_editing", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", pr.title_tr));
    },
    "delete-product": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_product", "#${id} ürünü silinsin mi?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/commerce-products/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.product_deleted", "Ürün silindi.")); await loadProducts(); } else showToast(r.errorMessage, true);
    },
    "mark-paid": async (el) => {
      const id = el.getAttribute("data-id");
      const r = await apiRequest(`/api/v1/admin/invoices/${encodeURIComponent(id)}/mark-paid`, { method: "POST" });
      if (r.ok) { showToast(t("admin.toast.invoice_marked_paid", "Fatura #${id} ödendi olarak işaretlendi.").replace("${id}", id)); await Promise.all([loadInvoices(), loadRevenueSummary()]); }
      else showToast(r.errorMessage, true);
    },
    "decide-corporate": async (el) => {
      const id = Number(el.getAttribute("data-id")), status = el.getAttribute("data-decision");
      const r = await apiRequest(`/api/v1/admin/corporate-requests/${id}/decide`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id, status, reviewed_by: ctx.getOperatorIdentity().email || undefined }),
      });
      if (r.ok) { showToast(status === "approved" ? t("admin.toast.corporate_approved", "Başvuru #${id} onaylandı.").replace("${id}", id) : t("admin.toast.corporate_rejected", "Başvuru #${id} reddedildi.").replace("${id}", id)); await loadCorporateRequests(); }
      else showToast(r.errorMessage, true);
    },
  });

  document.addEventListener("submit", async (ev) => {
    const id = ev.target?.id;
    if (id === "form-ad-campaign") {
      ev.preventDefault();
      if (!val("ad-sponsor-name") || !val("ad-title-tr")) {
        ctx.switchLangTab(ev.target, "tr");
        return showToast(t("admin.error.ad_required_fields", "Sponsor adı ve Türkçe başlık zorunludur.", "Sponsor name and Turkish title are required."), true);
      }
      if (validateCustomFields && !validateCustomFields("ad-custom-fields")) return;
      const editId = parseInt(val("ad-id") || "0", 10);
      const ex = editId > 0 ? state.ads.find((x) => Number(x.id) === editId) : null;
      const payload = {
        id: editId, slot_code: val("ad-slot-code"), badge_label: val("ad-badge-label") || "SPONSORLU REKLAM ALANI",
        dimensions: val("ad-dimensions") || "970x90", sponsor_name: val("ad-sponsor-name"), sponsor_type: val("ad-sponsor-type"),
        pill_style: val("ad-pill-style"), title_tr: val("ad-title-tr"), title_en: val("ad-title-en") || val("ad-title-tr"),
        desc_tr: val("ad-desc-tr"), desc_en: val("ad-desc-en") || val("ad-desc-tr"), target_url: val("ad-target-url"),
        image_url: val("ad-image-url") || null, budget_score: parseInt(val("ad-budget-score") || "85", 10),
        start_date: val("ad-start-date") || null, end_date: val("ad-end-date") || null,
        rotation_mode: val("ad-rotation-mode") || "weighted",
        is_default_fallback: !!byId("ad-is-fallback")?.checked, is_floating_popup: !!byId("ad-is-popup")?.checked,
        is_newsletter_ad: !!byId("ad-is-newsletter")?.checked, is_active: !!byId("ad-is-active")?.checked,
        target_categories: val("ad-target-categories").split(",").map((s) => s.trim()).filter(Boolean),
        target_ssb_tags: Array.from(document.querySelectorAll('input[name="ad_ssb"]:checked')).map((e) => e.value),
        target_institutions: Array.from(document.querySelectorAll('input[name="ad_inst"]:checked')).map((e) => e.value),
        impressions_count: ex?.impressions_count ?? 0, clicks_count: ex?.clicks_count ?? 0,
      };
      await submitAdPayload(payload);
    } else if (id === "form-sub-plan") {
      ev.preventDefault();
      const lines = (fid) => val(fid).split("\n").map((s) => s.trim()).filter(Boolean);
      const payload = {
        code: val("sp-code"), name_tr: val("sp-name-tr"), name_en: val("sp-name-en") || val("sp-name-tr"),
        price_try: parseFloat(val("sp-price") || "0"), period_label_tr: val("sp-period-tr") || t("admin.label.period_annual_tr", "Yıllık"),
        period_label_en: val("sp-period-en") || t("admin.label.period_annual_en", "Annual"), features_tr: lines("sp-features-tr"), features_en: lines("sp-features-en"),
        is_featured: !!byId("sp-featured")?.checked, sort_order: parseInt(val("sp-sort") || "10", 10),
      };
      const r = await apiRequest("/api/v1/admin/subscription-plans", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) });
      if (r.ok) { showToast(t("admin.toast.plan_saved", "Plan \"${code}\" kaydedildi.").replace("${code}", payload.code)); ev.target.reset(); await loadPlans(); } else showToast(r.errorMessage, true);
    } else if (id === "form-commerce-product") {
      ev.preventDefault();
      const payload = {
        id: parseInt(val("cp-id") || "0", 10), sku: val("cp-sku"), product_type: val("cp-type") || "book",
        title_tr: val("cp-title-tr"), title_en: val("cp-title-en") || val("cp-title-tr"),
        author_or_source: val("cp-author") || "UMSA Yayıncılık", price_try: parseFloat(val("cp-price") || "0"),
        file_resolution_or_pages: val("cp-res"), cover_image: val("cp-cover") || "images/msi/dergi-286.webp",
        description_tr: val("cp-desc-tr"), description_en: val("cp-desc-en"), is_available: !!byId("cp-available")?.checked,
      };
      const r = await apiRequest("/api/v1/admin/commerce-products", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) });
      if (r.ok) { showToast(t("admin.toast.product_saved", "Ürün \"${sku}\" kaydedildi.").replace("${sku}", payload.sku)); ev.target.reset(); setVal("cp-id", "0"); await loadProducts(); } else showToast(r.errorMessage, true);
    }
  });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "filter-ad-search") renderAdsTable(1);
  });

  document.addEventListener("change", async (ev) => {
    const el = ev.target;
    if (!el) return;
    if (el.id === "filter-ad-status") renderAdsTable(1);
    else if (el.id === "filter-corp-status") renderCorporateTable();
    else if (el.id === "ad-image-file") {
      const u = await uploadFile(el);
      if (u) { setVal("ad-image-url", u); showToast(t("admin.toast.ad_image_uploaded", "Kampanya görseli yüklendi.", "Campaign image uploaded.")); }
    } else if (el.id === "ad-slot-code" || el.id === "ad-start-date" || el.id === "ad-end-date" || el.id === "ad-rotation-mode" || el.name === "ad_inst" || el.name === "ad_ssb") checkAdConflictLive();
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail?.initial) return;
    if (ev.detail?.tabId === "tab-14") loadRevenueSummary();
  });

  ctx.registerLoader(() => Promise.all([loadAds(), loadRevenueSummary(), loadPlans(), loadProducts(), loadInvoices(), loadCorporateRequests()]));
})();
