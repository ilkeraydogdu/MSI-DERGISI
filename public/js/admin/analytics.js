(function () {
  "use strict";
  const ctx = window.MSIAdmin?._ctx;
  if (!ctx) return;
  const { state, paginationState, byId, val, esc, apiRequest, unwrapList, downloadBlob, showToast, renderTableState, renderPaginationControls, renderChipSlice, charts, locale, t, svgIcon } = ctx;

  const OVERVIEW_PERMS = "analytics:read|analytics:export|contents:read|contents:delete|ads:read|categories:read|users:read|issues:read|issues:delete|approvals:read|approvals:approve|approvals:reject|commerce:read|subscriptions:read|invoices:read|revenue:read|notifications:read|corporate_requests:read|corporate_requests:decide|audit:read|settings:read|media:read|media:delete|monitoring:read|translations:read|glossary:write|glossary:manage|locales:manage|newsletters:read|email_groups:write|email_groups:manage|backup:manage|roles:manage|redirects:write|redirects:manage|system:read";
  const ANALYTICS_PERMS_CHECK = OVERVIEW_PERMS;
  const ANALYTICS_PER_PAGE = 15;

  function renderKpis() {
    const ov = state.overviewStats || {};
    const totalContents = (typeof ov.total_contents === "number" && ov.total_contents > 0 ? ov.total_contents : state.totalContentsCount) || state.contents.length;
    const totalPlacements = typeof ov.total_placements === "number"
      ? ov.total_placements
      : state.contents.reduce((a, c) => a + (Array.isArray(c.placements) ? c.placements.length : 0), 0);
    if (byId("kpi-total-contents")) byId("kpi-total-contents").textContent = totalContents.toLocaleString(locale());
    if (byId("kpi-placements-sub")) byId("kpi-placements-sub").textContent = `${totalPlacements.toLocaleString(locale())} ${t("admin.analytics.multi_section_display", "çoklu bölüm gösterimi", "multi-section placements")}`;

    // Only use real analytics event totals from overviewStats — never fall back to
    // seed/harvested view_count or true_read_count which represent the original site's traffic.
    const views = typeof ov.total_views === "number" ? ov.total_views : 0;
    const reads = typeof ov.total_true_reads === "number" ? ov.total_true_reads : 0;
    const ratio = views > 0 ? Math.round((reads / views) * 100) : 0;
    if (byId("kpi-true-reads")) byId("kpi-true-reads").textContent = `${reads.toLocaleString(locale())} (%${ratio})`;
    if (byId("kpi-views-sub")) byId("kpi-views-sub").textContent = `${t("admin.analytics.total_views_prefix", "Toplam", "Total")} ${views.toLocaleString(locale())} ${t("admin.analytics.views_label", "görüntülenme", "views")}`;

    const activeAds = typeof ov.active_ads === "number" ? ov.active_ads : state.ads.filter((a) => a.is_active !== false).length;
    // Only use real analytics event totals — never fall back to seed/harvested impressions_count.
    const totalImps = typeof ov.total_impressions === "number" ? ov.total_impressions : 0;
    if (byId("kpi-active-ads")) byId("kpi-active-ads").textContent = activeAds.toLocaleString(locale());
    if (byId("kpi-ads-sub")) byId("kpi-ads-sub").textContent = `${totalImps.toLocaleString(locale())} ${t("admin.analytics.corporate_impressions", "kurumsal gösterim", "corporate impressions")}`;

    const pendingReqs = typeof ov.pending_corporate_requests === "number" ? ov.pending_corporate_requests : state.corporateRequests.filter((r) => r.status === "pending").length;
    if (byId("kpi-gov-requests")) byId("kpi-gov-requests").textContent = pendingReqs.toLocaleString(locale());
    if (byId("kpi-gov-sub")) byId("kpi-gov-sub").textContent = `${t("admin.analytics.total_prefix", "Toplam", "Total")} ${state.corporateRequests.length.toLocaleString(locale())} ${t("admin.analytics.corporate_requests", "kurumsal başvuru", "corporate requests")}`;
  }

  ctx.renderKpis = renderKpis;

  async function loadOverviewStats() {
    if (!ctx.hasPerm(ANALYTICS_PERMS_CHECK)) return;
    const resp = await apiRequest("/api/v1/admin/overview");
    if (!resp.ok || !resp.data) return;
    state.overviewStats = resp.data;
    if (typeof resp.data.total_contents === "number") state.totalContentsCount = Math.max(state.totalContentsCount, resp.data.total_contents);
    renderKpis();
  }

  function renderInstitutionBars(list, isCyan) {
    if (!Array.isArray(list) || !list.length) return `<span class="text-muted text-xs">${esc(t("admin.analytics.data_collecting", "Veri toplanıyor", "Collecting data"))}</span>`;
    return list.slice(0, 4).map((it) => {
      const pct = Number(it.percentage || 0);
      const qPct = pct <= 2 ? 4 : Math.min(100, Math.max(5, Math.round(pct / 5) * 5));
      return `<div class="inst-bar-row">
        <div class="inst-bar-meta"><span><strong>${esc(it.organization_category)}</strong></span><span>${esc(it.count)} (%${pct.toFixed(1)})</span></div>
        <div class="inst-bar-track"><div class="inst-bar-fill w-p-${qPct} ${isCyan ? "cyan" : ""}"></div></div>
      </div>`;
    }).join("");
  }

  function renderContentAnalyticsPage(page) {
    const tc = byId("tbody-analytics-content");
    if (!tc) return;
    if (typeof page === "number") paginationState.analyticsContent = page;
    const cur = paginationState.analyticsContent || 1;
    const q = val("analytics-search").toLowerCase();
    const list = state.contentAnalytics.filter((r) => !q || [r.title_tr, r.content_slug, r.slug].some((f) => String(f || "").toLowerCase().includes(q)));
    if (state.activeAnalyticsSubview === "content" && byId("analytics-count-label")) {
      byId("analytics-count-label").textContent = `${list.length.toLocaleString(locale())} ${t("admin.analytics.content_analysis", "yayın analizi", "content reports")}`;
    }
    if (!list.length) {
      renderTableState(tc, "empty", { colCount: 5, emptyMsg: t("admin.state.no_match_content_analysis", "Eşleşen içerik analizi bulunamadı.", "No matching content analytics found.") });
      renderPaginationControls("pagination-analytics-content", null, renderContentAnalyticsPage);
      return;
    }
    const tp = Math.max(1, Math.ceil(list.length / ANALYTICS_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = list.slice((sp - 1) * ANALYTICS_PER_PAGE, sp * ANALYTICS_PER_PAGE);
    tc.innerHTML = slice.map((r) => `<tr>
      <td><div class="font-semibold">${esc(r.title_tr)}</div><div class="text-mono text-xs text-muted">/${esc(r.content_slug || r.slug || "")}</div></td>
      <td class="text-mono font-bold text-right">${Number(r.total_views || 0).toLocaleString(locale())}</td>
      <td class="text-mono font-bold text-success text-right">${Number(r.true_reads ?? r.true_read_count ?? 0).toLocaleString(locale())}</td>
      <td class="text-mono text-xs"><div>${svgIcon("clock-o")} ${Math.round(r.avg_duration_sec || 0)} ${esc(t("admin.analytics.time_unit_sec", "sn", "s"))}</div><div class="text-info">${svgIcon("arrows-v")} %${Math.round(r.avg_scroll_pct || 0)} ${esc(t("admin.analytics.scroll_label", "kaydırma", "scroll"))}</div></td>
      <td>${renderInstitutionBars(r.institution_breakdown, false)}</td>
    </tr>`).join("");
    renderPaginationControls("pagination-analytics-content", { page: sp, total_pages: tp, total_count: list.length }, renderContentAnalyticsPage);
  }

  function renderAdAnalyticsPage(page) {
    const ta = byId("tbody-analytics-ad");
    if (!ta) return;
    if (typeof page === "number") paginationState.analyticsAd = page;
    const cur = paginationState.analyticsAd || 1;
    const q = val("analytics-search").toLowerCase();
    const list = state.adAnalytics.filter((a) => !q || [a.sponsor_name, a.slot_code].some((f) => String(f || "").toLowerCase().includes(q)));
    if (state.activeAnalyticsSubview === "ad" && byId("analytics-count-label")) {
      byId("analytics-count-label").textContent = `${list.length.toLocaleString(locale())} ${t("admin.analytics.campaign_report", "kampanya raporu", "campaign reports")}`;
    }
    if (!list.length) {
      renderTableState(ta, "empty", { colCount: 4, emptyMsg: t("admin.state.no_match_campaign_analysis", "Eşleşen kampanya analizi bulunamadı.", "No matching campaign analytics found.") });
      renderPaginationControls("pagination-analytics-ad", null, renderAdAnalyticsPage);
      return;
    }
    const tp = Math.max(1, Math.ceil(list.length / ANALYTICS_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = list.slice((sp - 1) * ANALYTICS_PER_PAGE, sp * ANALYTICS_PER_PAGE);
    ta.innerHTML = slice.map((a) => `<tr>
      <td><div class="font-semibold">${esc(a.sponsor_name)}</div><span class="badge-tag">${esc(a.slot_code)}</span></td>
      <td class="text-mono text-xs"><div><strong>${Number(a.impressions || 0).toLocaleString(locale())}</strong> ${esc(t("admin.label.impressions", "gösterim", "impressions"))}</div><div class="text-success"><strong>${Number(a.clicks || 0).toLocaleString(locale())}</strong> ${esc(t("admin.label.clicks", "tıklama", "clicks"))}</div><div class="text-warning">CTR: %${Number(a.ctr_pct || 0).toFixed(2)}</div></td>
      <td>${renderInstitutionBars(a.impression_institutions, false)}</td>
      <td>${renderInstitutionBars(a.click_institutions, true)}</td>
    </tr>`).join("");
    renderPaginationControls("pagination-analytics-ad", { page: sp, total_pages: tp, total_count: list.length }, renderAdAnalyticsPage);
  }

  function renderReaderAnalyticsPage(page) {
    const tr = byId("tbody-analytics-reader");
    if (!tr) return;
    if (typeof page === "number") paginationState.analyticsReader = page;
    const cur = paginationState.analyticsReader || 1;
    const q = val("analytics-search").toLowerCase();
    const list = state.readerAnalytics.filter((u) => !q || [u.full_name, u.email, u.organization_name, u.organization_category].some((f) => String(f || "").toLowerCase().includes(q)));
    if (state.activeAnalyticsSubview === "reader" && byId("analytics-count-label")) {
      byId("analytics-count-label").textContent = `${list.length.toLocaleString(locale())} ${t("admin.analytics.corporate_readers", "kurumsal okuyucu", "corporate readers")}`;
    }
    if (!list.length) {
      renderTableState(tr, "empty", { colCount: 6, emptyMsg: t("admin.state.no_match_reader", "Eşleşen okuyucu kaydı bulunamadı.", "No matching reader records found.") });
      renderPaginationControls("pagination-analytics-reader", null, renderReaderAnalyticsPage);
      return;
    }
    const tp = Math.max(1, Math.ceil(list.length / ANALYTICS_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = list.slice((sp - 1) * ANALYTICS_PER_PAGE, sp * ANALYTICS_PER_PAGE);
    tr.innerHTML = slice.map((u) => `<tr>
      <td><div class="font-semibold">${esc(u.full_name)}</div><div class="text-xs text-muted">${esc(u.email)}</div></td>
      <td><span class="badge-tag">${esc(u.organization_category)}</span><div class="text-xs text-secondary mt-1">${esc(u.organization_name)}</div></td>
      <td><span class="badge-tag badge-paywall-0">${esc(u.subscription_tier)}</span></td>
      <td>${renderChipSlice(u.articles_read, "badge-placement", 3) || "—"}</td>
      <td>${renderChipSlice(u.ssb_keywords_engaged, "badge-ssb", 3) || "—"}</td>
      <td class="text-mono text-xs">${Number(u.ad_impressions_count || 0)} ${esc(t("admin.label.impressions_short", "gös.", "imp."))} / <strong>${Number(u.ad_clicks_count || 0)} ${esc(t("admin.label.clicks_short", "tık", "clicks"))}</strong></td>
    </tr>`).join("");
    renderPaginationControls("pagination-analytics-reader", { page: sp, total_pages: tp, total_count: list.length }, renderReaderAnalyticsPage);
  }

  function renderAnalyticsTables() {
    renderContentAnalyticsPage();
    renderAdAnalyticsPage();
    renderReaderAnalyticsPage();
  }

  function renderAnalyticsCharts() {
    if (!ctx.hasPerm("analytics:read")) return;
    if (!state.contentAnalytics.length) {
      charts.showEmpty("chart-content-trend", t("admin.chart.empty_content_trend", "İçerik okuma eğilimi için henüz analitik verisi yok.", "No content trend data available yet."), "area-chart");
      charts.showEmpty("chart-top-contents", t("admin.chart.empty_top_contents", "En yüksek erişimli yayınlar için veri yok.", "No top publications data available yet."), "bar-chart");
      return;
    }

    const slice20 = [...state.contentAnalytics].slice(0, 20);
    charts.mountOrUpdate("chart-content-trend", (pal) => ({
      type: "line",
      data: {
        labels: slice20.map((c, idx) => `#${idx + 1} ` + String(c.title_tr || c.slug || "").substring(0, 18)),
        datasets: [
          { label: t("admin.chart.views", "Görüntülenme", "Views"), data: slice20.map((c) => Number(c.total_views ?? c.view_count ?? 0)), borderColor: pal.info, backgroundColor: pal.infoSoft, borderWidth: 2, pointRadius: 2.5, tension: 0.32, fill: true },
          { label: t("admin.chart.verified_reads", "Doğrulanmış Okuma", "Verified Reads"), data: slice20.map((c) => Number(c.true_reads ?? c.true_read_count ?? 0)), borderColor: pal.success, backgroundColor: pal.successSoft, borderWidth: 2, pointRadius: 2.5, tension: 0.32, fill: true },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        plugins: { legend: { position: "top", labels: { color: pal.sec, font: { weight: "600", size: 11.5 } } } },
        scales: {
          x: { ticks: { color: pal.muted, maxRotation: 28, font: { size: 10.5 } }, grid: { color: pal.grid } },
          y: { beginAtZero: true, ticks: { color: pal.muted, font: { size: 11 } }, grid: { color: pal.grid } },
        },
      },
    }));

    const top10 = [...state.contentAnalytics].sort((a, b) => (b.total_views ?? 0) - (a.total_views ?? 0)).slice(0, 10);
    charts.mountOrUpdate("chart-top-contents", (pal) => {
      const barColors = [
        pal.accent, pal.info, pal.success, pal.warning, pal.danger,
        pal.sec, pal.muted, pal.accent, pal.info, pal.success,
      ];
      return {
        type: "bar",
        data: {
          labels: top10.map((c) => {
            const raw = String(c.title_tr || c.slug || "?");
            return raw.length > 44 ? raw.substring(0, 44) + "…" : raw;
          }),
          datasets: [{ label: t("admin.chart.total_views", "Toplam Görüntülenme", "Total Views"), data: top10.map((c) => Number(c.total_views ?? 0)), backgroundColor: barColors, borderRadius: 5, barThickness: 18 }],
        },
        options: {
          indexAxis: "y",
          responsive: true,
          maintainAspectRatio: false,
          plugins: { legend: { display: false } },
          scales: {
            x: { beginAtZero: true, ticks: { color: pal.muted, font: { size: 11 } }, grid: { color: pal.grid } },
            y: { ticks: { color: pal.sec, font: { size: 11.5, weight: "500" } }, grid: { display: false } },
          },
        },
      };
    });
  }

  async function loadAnalyticsBundle() {
    const canC = ctx.hasPerm("analytics:read");
    const canA = ctx.hasPerm("analytics:read|ads:read");
    const canR = ctx.hasPerm("analytics:read|users:read");
    const [cr, ar, rr] = await Promise.all([
      canC ? apiRequest("/api/v1/admin/analytics/contents") : Promise.resolve({ ok: false }),
      canA ? apiRequest("/api/v1/admin/analytics/ads") : Promise.resolve({ ok: false }),
      canR ? apiRequest("/api/v1/admin/analytics/readers") : Promise.resolve({ ok: false }),
    ]);
    if (cr.ok) state.contentAnalytics = unwrapList(cr.payload, "reports");
    if (ar.ok) state.adAnalytics = unwrapList(ar.payload, "reports");
    if (rr.ok) state.readerAnalytics = unwrapList(rr.payload, "reports");
    renderAnalyticsTables();
    renderAnalyticsCharts();
  }

  async function refreshSystemHealth() {
    if (!ctx.hasPerm("settings:read|analytics:read|system:read")) return;
    const panel = byId("system-health-panel");
    if (!panel) return;
    const resp = await apiRequest("/api/v1/admin/system-health");
    if (!resp.ok || !resp.data) { panel.innerHTML = `<div class="text-danger text-sm">${esc(t("admin.health.check_failed", "Sağlık kontrolü alınamadı.", "Health check unavailable."))}</div>`; return; }
    const d = resp.data, ok = d.status === "healthy";
    const dbMB = ((d.db_size_bytes || 0) / 1048576).toFixed(2);
    const ts = d.timestamp ? new Date(d.timestamp).toLocaleTimeString(locale()) : "—";
    panel.innerHTML = `<div class="health-metrics-grid">
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.engine_status", "Motor Durumu", "Engine Status"))}</div><div class="health-metric-val ${ok ? "text-success" : "text-danger"}">${ok ? esc(t("admin.health.healthy", "SAĞLIKLI", "HEALTHY")) : esc(String(d.status || "").toUpperCase())}</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.sqlite_wal_size", "SQLite WAL Boyutu", "SQLite WAL Size"))}</div><div class="health-metric-val">${esc(dbMB)} MB</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.total_contents", "Toplam Yayın", "Total Content"))}</div><div class="health-metric-val">${Number(d.tables?.contents ?? 0).toLocaleString(locale())}</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.registered_users", "Kayıtlı Kullanıcı", "Registered Users"))}</div><div class="health-metric-val">${Number(d.tables?.users ?? 0).toLocaleString(locale())}</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.analytics_events", "Analitik Olayı", "Analytics Events"))}</div><div class="health-metric-val">${Number(d.tables?.analytics_events ?? 0).toLocaleString(locale())}</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${esc(t("admin.health.last_audit", "Son Denetim", "Last Audit"))}</div><div class="health-metric-val">${esc(ts)}</div></div>
    </div>`;
  }

  async function loadUserAnalytics() {
    const canChurn = ctx.hasPerm("analytics:read|revenue:read|subscriptions:read");
    const canRev = ctx.hasPerm("analytics:read|revenue:read");
    const canCohort = ctx.hasPerm("analytics:read|users:read");
    const [churnR, ltvR, funnelR, cohortR] = await Promise.all([
      canChurn ? apiRequest("/api/v1/admin/analytics/churn-rate") : Promise.resolve({ ok: false }),
      canRev ? apiRequest("/api/v1/admin/analytics/user-ltv") : Promise.resolve({ ok: false }),
      canRev ? apiRequest("/api/v1/admin/analytics/conversion-funnel") : Promise.resolve({ ok: false }),
      canCohort ? apiRequest("/api/v1/admin/analytics/user-cohorts") : Promise.resolve({ ok: false }),
    ]);

    if (churnR.ok && churnR.data) {
      if (byId("kpi-churn-rate")) byId("kpi-churn-rate").textContent = "%" + Number(churnR.data.monthly_churn_pct || 0).toFixed(1);
      if (byId("kpi-avg-lifetime")) byId("kpi-avg-lifetime").textContent = Number(churnR.data.avg_lifetime_days || 0).toLocaleString(locale()) + " " + t("admin.analytics.days_unit", "gün", "days");
    }
    if (ltvR.ok && ltvR.data) {
      if (byId("kpi-avg-ltv")) byId("kpi-avg-ltv").textContent = "₺" + Number(ltvR.data.avg_ltv_try || 0).toLocaleString(locale(), { minimumFractionDigits: 2 });
      if (byId("kpi-paying-users")) byId("kpi-paying-users").textContent = Number(ltvR.data.paying_users || 0).toLocaleString(locale());
    }
    if (canRev) {
      const ratesEl = byId("funnel-rates");
      if (!funnelR.ok || !funnelR.data) {
        charts.showEmpty("chart-conversion-funnel", funnelR.errorMessage || t("admin.state.error", "Veri yüklenemedi.", "Failed to load data."), "filter");
        if (ratesEl) ratesEl.innerHTML = `<div class="text-muted text-sm">${esc(t("admin.state.empty", "Kayıt bulunamadı.", "No records found."))}</div>`;
      } else {
        const f = funnelR.data;
        const visitors = Number(f.visitors || 0);
        const registered = Number(f.registered || 0);
        const verified = Number(f.verified || 0);
        const activeSubs = Number(f.active_subscribers ?? f.subscribed ?? 0);
        const paidCustomers = Number(f.paid_invoice_customers ?? f.paid ?? 0);
        const stages = [visitors, registered, verified, activeSubs, paidCustomers];
        if (stages.every((v) => v === 0)) {
          charts.showEmpty("chart-conversion-funnel", t("admin.chart.empty_funnel", "Dönüşüm hunisi için henüz kayıtlı veri yok.", "No conversion funnel data recorded yet."), "filter");
        } else {
          charts.mountOrUpdate("chart-conversion-funnel", (pal) => ({
            type: "bar",
            data: {
              labels: [
                t("admin.chart.funnel_visitors", "Tekil Ziyaretçi", "Unique Visitors"),
                t("admin.chart.funnel_registered", "Kayıtlı Okuyucu", "Registered Readers"),
                t("admin.chart.funnel_verified", "Doğrulanmış / Kamu", "Verified / Gov Readers"),
                t("admin.chart.funnel_active_subs", "Aktif Abone", "Active Subscribers"),
                t("admin.chart.funnel_paid", "Faturalı Üye", "Invoiced Customers"),
              ],
              datasets: [{
                label: t("admin.chart.user_count", "Kullanıcı Sayısı", "User Count"),
                data: stages,
                backgroundColor: [pal.muted, pal.info, pal.accent, pal.warning, pal.success],
                borderRadius: 6,
                barThickness: 22,
              }],
            },
            options: {
              indexAxis: "y",
              responsive: true,
              maintainAspectRatio: false,
              plugins: { legend: { display: false } },
              scales: {
                x: { beginAtZero: true, ticks: { color: pal.muted }, grid: { color: pal.grid } },
                y: { ticks: { color: pal.sec, font: { weight: "600", size: 12 } }, grid: { display: false } },
              },
            },
          }));
        }
        if (ratesEl) {
          const rt = f.conversion_rates || {};
          const clampPct = (v) => Math.min(100, Math.max(0, Number(v || 0)));
          ratesEl.innerHTML = [
            [t("admin.chart.funnel_v_to_r", "Ziyaretçi → Kayıtlı Okuyucu", "Visitor → Registered Reader"), clampPct(rt.visitor_to_registered), "tone-info"],
            [t("admin.chart.funnel_r_to_v", "Kayıtlı → Doğrulanmış / Kamu", "Registered → Verified / Gov"), clampPct(rt.registered_to_verified), "tone-accent"],
            [t("admin.chart.funnel_v_to_s", "Doğrulanmış → Aktif Abone", "Verified → Active Subscriber"), clampPct(rt.verified_to_active_subscriber), "tone-warning"],
            [t("admin.chart.funnel_s_to_p", "Aktif Abone → Faturalı Üye", "Active Subscriber → Invoiced"), clampPct(rt.active_subscriber_to_paid ?? rt.subscribed_to_paid), "tone-success"],
          ].map(([lbl, v, tone]) => `<div class="kpi-card ${tone}"><div class="kpi-label"><span>${esc(lbl)}</span></div><div class="kpi-value text-success">%${esc(v.toFixed(1))}</div></div>`).join("");
        }
      }
    }
    if (cohortR.ok && cohortR.payload) {
      const rows = unwrapList(cohortR.payload, "cohorts"), tbody = byId("tbody-cohort-analysis");
      if (tbody) {
        if (!rows.length) renderTableState(tbody, "empty", { colCount: 8 });
        else tbody.innerHTML = rows.map((row) => {
          const reg = Number(row.registered || 0);
          let tds = `<td class="font-semibold text-mono">${esc(row.cohort_month || "?")}</td><td class="text-right font-bold text-mono">${reg.toLocaleString(locale())}</td>`;
          for (let m = 1; m <= 6; m++) {
            const pct = reg > 0 ? Math.round((Number(row["active_m" + m] || 0) / reg) * 100) : 0;
            const heatCls = `cohort-heat-${Math.min(10, Math.max(0, Math.round(pct / 10)))}`;
            tds += `<td class="text-right text-mono cohort-heat-cell ${heatCls}">${pct}%</td>`;
          }
          return `<tr>${tds}</tr>`;
        }).join("");
      }
    }
  }

  ctx.registerActions({
    "switch-analytics-subview": (el) => {
      const view = el.getAttribute("data-subview") || "content";
      state.activeAnalyticsSubview = view;
      ["content", "ad", "reader"].forEach((v) => {
        byId("btn-subview-" + v)?.classList.toggle("btn-cmd-primary", v === view);
        byId("subview-" + v)?.classList.toggle("active", v === view);
      });
      const activeView = byId("subview-" + view);
      const reduced = Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches);
      if (activeView && window.gsap && !reduced) {
        window.gsap.killTweensOf(activeView);
        window.gsap.fromTo(
          activeView,
          { opacity: 0.78, y: 4 },
          { opacity: 1, y: 0, duration: 0.16, ease: "power2.out", clearProps: "transform,opacity" },
        );
      }
      renderAnalyticsTables();
    },
    "export-analytics-json": () => {
      const payload = {
        generated_at: new Date().toISOString(),
        generated_by: ctx.getOperatorIdentity().email || "operator",
        summary: { total_contents: state.totalContentsCount || state.contents.length, total_ads: state.ads.length, total_readers: state.readerAnalytics.length },
        content_analytics: state.contentAnalytics, ad_analytics: state.adAnalytics, reader_analytics: state.readerAnalytics,
      };
      downloadBlob(JSON.stringify(payload, null, 2), `${t("admin.export.analytics_report_prefix", "MSI_Analitik_Raporu", "MSI_Analytics_Report")}_${new Date().toISOString().slice(0, 10)}.json`, "application/json;charset=utf-8");
      showToast(t("admin.toast.json_report_downloaded", "Konsolide JSON analitik raporu indirildi.", "Consolidated JSON analytics report downloaded."));
    },
    "export-analytics-csv": () => {
      const cell = (v) => `"${String(v ?? "").replace(/"/g, '""')}"`;
      const rows = state.activeAnalyticsSubview === "ad"
        ? [[t("admin.csv.sponsor", "Sponsor", "Sponsor"), t("admin.csv.slot", "Slot", "Slot"), t("admin.csv.impressions", "Gosterim", "Impressions"), t("admin.csv.clicks", "Tiklama", "Clicks"), "CTR_Pct"], ...state.adAnalytics.map((a) => [a.sponsor_name, a.slot_code, a.impressions || 0, a.clicks || 0, Number(a.ctr_pct || 0).toFixed(2)])]
        : state.activeAnalyticsSubview === "reader"
          ? [[t("admin.csv.full_name", "Ad_Soyad", "Full_Name"), t("admin.csv.email", "Eposta", "Email"), t("admin.csv.org_category", "Kurum_Kategorisi", "Org_Category"), t("admin.csv.org_name", "Kurum_Adi", "Org_Name"), t("admin.csv.subscription", "Abonelik", "Subscription"), t("admin.csv.ad_clicks", "Reklam_Tik", "Ad_Clicks")], ...state.readerAnalytics.map((u) => [u.full_name, u.email, u.organization_category, u.organization_name, u.subscription_tier, u.ad_clicks_count || 0])]
          : [["Slug", t("admin.csv.title_tr", "Baslik_TR", "Title_TR"), t("admin.csv.views", "Goruntulenme", "Views"), t("admin.csv.true_reads", "Gercek_Okuma", "Verified_Reads"), t("admin.csv.avg_duration_sec", "Ort_Sure_Sn", "Avg_Duration_Sec"), t("admin.csv.avg_scroll_pct", "Ort_Scroll_Pct", "Avg_Scroll_Pct")], ...state.contentAnalytics.map((r) => [r.content_slug || r.slug || "", r.title_tr || "", r.total_views || 0, r.true_reads ?? r.true_read_count ?? 0, Math.round(r.avg_duration_sec || 0), Math.round(r.avg_scroll_pct || 0)])];
      downloadBlob("\uFEFF" + rows.map((r) => r.map(cell).join(",")).join("\r\n"), `${t("admin.export.analytics_prefix", "MSI_Analitik", "MSI_Analytics")}_${state.activeAnalyticsSubview}_${new Date().toISOString().slice(0, 10)}.csv`, "text/csv;charset=utf-8");
      showToast(t("admin.toast.csv_report_downloaded", "CSV analitik raporu indirildi.", "CSV analytics report downloaded."));
    },
    "refresh-system-health": refreshSystemHealth,
    "load-user-analytics": loadUserAnalytics,
  });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "analytics-search") {
      paginationState.analyticsContent = 1;
      paginationState.analyticsAd = 1;
      paginationState.analyticsReader = 1;
      renderAnalyticsTables();
    }
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail?.initial) return;
    const tabId = ev.detail?.tabId;
    if (tabId === "tab-9") { if (ctx.hasPerm("analytics:read")) renderAnalyticsCharts(); refreshSystemHealth(); }
    else if (tabId === "tab-15") loadUserAnalytics();
  });

  ctx.registerLoader(() => Promise.all([loadOverviewStats(), loadAnalyticsBundle(), refreshSystemHealth(), loadUserAnalytics()]));
})();
