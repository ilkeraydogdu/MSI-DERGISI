(function () {
  "use strict";

  const C = () => window.MSIAdmin && window.MSIAdmin._ctx;
  const MON_PER_PAGE = 15;
  const MON_PERM = "monitoring:read|system:read";
  const HISTORY_KEY = "msi_mon_history";
  const MAX_POINTS = 40;
  let _timer = null;
  let _followUpTimer = null;
  let _cachedDomains = [];
  let _cachedSeo = [];

  function loadHistory() {
    try {
      const raw = sessionStorage.getItem(HISTORY_KEY);
      if (raw) {
        const parsed = JSON.parse(raw);
        const now = Date.now();
        const tsArr = Array.isArray(parsed?.ts) ? parsed.ts : [];
        const cpu = [], ram = [], labels = [], ts = [];
        for (let i = 0; i < tsArr.length; i++) {
          if (now - Number(tsArr[i] || 0) <= 30 * 60 * 1000) {
            cpu.push(Number(parsed.cpu?.[i] || 0));
            ram.push(Number(parsed.ram?.[i] || 0));
            labels.push(String(parsed.labels?.[i] || ""));
            ts.push(Number(tsArr[i]));
          }
        }
        return { cpu: cpu.slice(-MAX_POINTS), ram: ram.slice(-MAX_POINTS), labels: labels.slice(-MAX_POINTS), ts: ts.slice(-MAX_POINTS) };
      }
    } catch (_) {}
    return { cpu: [], ram: [], labels: [], ts: [] };
  }

  function saveHistory(h) {
    try {
      sessionStorage.setItem(HISTORY_KEY, JSON.stringify(h));
    } catch (_) {}
  }

  const _history = loadHistory();

  function renderMonitoringUnavailable(msg) {
    const c = C();
    const banner = c.byId("monitoring-status-banner");
    const notice = msg || c.t("admin.monitoring.disabled_banner", "Sunucu izleme şu anda devre dışı.", "Server monitoring is currently disabled.");
    if (banner) {
      banner.classList.remove("hidden");
      banner.textContent = notice;
    }
    const shortOff = c.t("admin.monitoring.unavailable_short", "İzleme devre dışı", "Monitoring disabled");
    for (const id of ["mon-cpu-sub", "mon-ram-sub", "mon-disk-sub", "mon-domains-sub", "mon-attacks-sub", "mon-services-sub"]) {
      const el = c.byId(id);
      if (el) el.textContent = shortOff;
    }
    for (const id of ["mon-cpu", "mon-ram", "mon-disk", "mon-domains", "mon-services"]) {
      const el = c.byId(id);
      if (el) el.textContent = "—";
    }
    c.charts.showEmpty("chart-system-resources", notice, "area-chart");
    c.charts.showEmpty("chart-traffic-dist", notice, "pie-chart");
    const srvPanel = c.byId("monitoring-services-panel");
    if (srvPanel) srvPanel.innerHTML = `<div class="text-muted text-sm p-3">${c.esc(notice)}</div>`;
    const netPanel = c.byId("monitoring-network-panel");
    if (netPanel) netPanel.innerHTML = `<div class="text-muted text-sm p-3">${c.esc(notice)}</div>`;
    const atkPanel = c.byId("monitoring-attacks-panel");
    if (atkPanel) atkPanel.innerHTML = `<div class="text-muted text-sm p-3">${c.esc(notice)}</div>`;
    const domTbody = c.byId("tbody-monitoring-domains");
    if (domTbody) c.renderTableState(domTbody, "empty", { colCount: 7, emptyMsg: notice });
    const seoTbody = c.byId("tbody-monitoring-seo");
    if (seoTbody) c.renderTableState(seoTbody, "empty", { colCount: 7, emptyMsg: notice });
  }

  async function loadMonitoringBundle() {
    const c = C();
    const pane = c.byId("tab-17");
    if (!pane || !pane.classList.contains("active") || !c.hasPerm(MON_PERM)) return;
    if (window.MSIAdmin?._me?.monitoring_enabled === false) {
      renderMonitoringUnavailable(c.t("admin.monitoring.disabled_banner", "Sunucu izleme şu anda devre dışı (MONITORING_ENABLED=false).", "Server monitoring is currently disabled (MONITORING_ENABLED=false)."));
      return;
    }
    const resp = await c.apiRequest("/api/v1/admin/monitoring/bundle");
    if (!resp.ok || !resp.data) {
      const msg = resp.status === 503
        ? c.t("admin.monitoring.disabled_banner", "Sunucu izleme şu anda devre dışı (MONITORING_ENABLED=false).", "Server monitoring is currently disabled (MONITORING_ENABLED=false).")
        : (resp.errorMessage || c.t("admin.monitoring.no_telemetry", "Sistem telemetrisi alınamadı.", "System telemetry unavailable."));
      renderMonitoringUnavailable(msg);
      return;
    }
    const banner = c.byId("monitoring-status-banner");
    if (banner) {
      banner.classList.add("hidden");
      banner.textContent = "";
    }
    const b = resp.data;
    _cachedDomains = Array.isArray(b.domains) ? b.domains : [];
    _cachedSeo = Array.isArray(b.seo) ? b.seo : [];
    renderMonitoringKPIs(b);
    renderSystemResourcesChart(b.system);
    renderServicesPanel(b.services);
    renderDomainsTable();
    renderTrafficDistChart(b.traffic);
    renderAttacksPanel(b.attacks);
    renderSeoTable();
    renderNetworkPanel(b.system);
  }

  function renderMonitoringKPIs(bundle) {
    const c = C();
    const setTxt = (id, txt) => { const el = c.byId(id); if (el) el.textContent = txt; };
    const sys = bundle.system;
    if (sys && sys.cpu && sys.memory && sys.disk) {
      setTxt("mon-cpu", Number(sys.cpu.user_pct || 0).toFixed(1) + "%");
      setTxt("mon-ram", Number(sys.memory.used_pct || 0).toFixed(1) + "%");
      setTxt("mon-disk", Number(sys.disk.used_pct || 0).toFixed(1) + "%");
      setTxt("mon-cpu-sub", `${c.t("admin.monitoring.load_label", "Yük:", "Load:")} ${Number(sys.cpu.load_avg_1m || 0).toFixed(2)} / ${Number(sys.cpu.load_avg_5m || 0).toFixed(2)} / ${Number(sys.cpu.load_avg_15m || 0).toFixed(2)}`);
      const tRam = (Number(sys.memory.total_kb || 0) / 1048576).toFixed(1);
      const aRam = (Number(sys.memory.available_kb || 0) / 1048576).toFixed(1);
      setTxt("mon-ram-sub", `${aRam} GB ${c.t("admin.monitoring.free_suffix", "boş", "free")} / ${tRam} GB ${c.t("admin.monitoring.total_suffix", "toplam", "total")}`);
      const fDisk = (Number(sys.disk.free_bytes || 0) / 1073741824).toFixed(0);
      const tDisk = (Number(sys.disk.total_bytes || 0) / 1073741824).toFixed(0);
      setTxt("mon-disk-sub", `${fDisk} GB ${c.t("admin.monitoring.free_suffix", "boş", "free")} / ${tDisk} GB ${c.t("admin.monitoring.total_suffix", "toplam", "total")}`);
    }
    if (Array.isArray(bundle.domains)) {
      setTxt("mon-domains", String(bundle.domains.length));
      const healthy = bundle.domains.filter((d) => d.http_status && d.http_status < 400).length;
      setTxt("mon-domains-sub", `${healthy} ${c.t("admin.monitoring.healthy_suffix", "sağlıklı", "healthy")} / ${bundle.domains.length} ${c.t("admin.monitoring.total_suffix", "toplam", "total")}`);
    }
    setTxt("mon-attacks", Array.isArray(bundle.attacks) ? String(bundle.attacks.length) : "0");
    if (Array.isArray(bundle.services)) {
      const ok = bundle.services.filter((s) => s.reachable).length;
      setTxt("mon-services", `${ok}/${bundle.services.length}`);
      setTxt("mon-services-sub", `${ok} ${c.t("admin.status.active", "aktif", "active")}, ${bundle.services.length - ok} ${c.t("admin.monitoring.unreachable", "erişilemiyor", "unreachable")}`);
    }
  }

  function scheduleFollowUpSample() {
    const c = C();
    if (_followUpTimer || _history.cpu.length !== 1) return;
    _followUpTimer = setTimeout(async () => {
      _followUpTimer = null;
      const pane = c.byId("tab-17");
      if (!pane || !pane.classList.contains("active") || !c.hasPerm(MON_PERM)) return;
      const resp = await c.apiRequest("/api/v1/admin/monitoring/system");
      if (resp.ok && resp.data?.system) renderSystemResourcesChart(resp.data.system);
    }, 2500);
  }

  function renderSystemResourcesChart(system) {
    const c = C();
    if ((!system?.cpu || !system?.memory) && !_history.cpu.length) {
      c.charts.showEmpty("chart-system-resources", c.t("admin.monitoring.no_telemetry", "Sistem telemetrisi alınamadı.", "System telemetry unavailable."), "area-chart");
      return;
    }
    if (system?.cpu && system?.memory) {
      _history.cpu.push(Number(system.cpu.user_pct || 0));
      _history.ram.push(Number(system.memory.used_pct || 0));
      _history.labels.push(new Date().toLocaleTimeString(c.locale(), { hour: "2-digit", minute: "2-digit", second: "2-digit" }));
      _history.ts.push(Date.now());
      if (_history.cpu.length > MAX_POINTS) {
        _history.cpu.shift(); _history.ram.shift(); _history.labels.shift(); _history.ts.shift();
      }
      saveHistory(_history);
    }
    if (_history.cpu.length === 1) scheduleFollowUpSample();

    const pointRad = _history.cpu.length < 3 ? 5 : 2;
    c.charts.mountOrUpdate("chart-system-resources", (pal) => ({
      type: "line",
      data: {
        labels: _history.labels,
        datasets: [
          { label: c.t("admin.chart.cpu_pct", "CPU %", "CPU %"), data: _history.cpu, borderColor: pal.info, backgroundColor: pal.infoSoft, borderWidth: 2.5, tension: 0.3, fill: true, pointRadius: pointRad },
          { label: c.t("admin.chart.ram_pct", "Bellek %", "Memory %"), data: _history.ram, borderColor: pal.accent, backgroundColor: pal.accentSoft, borderWidth: 2.5, tension: 0.3, fill: true, pointRadius: pointRad },
        ],
      },
      options: {
        responsive: true, maintainAspectRatio: false, animation: false,
        plugins: { legend: { position: "top", labels: { color: pal.sec } } },
        scales: {
          x: { ticks: { color: pal.muted }, grid: { color: pal.grid } },
          y: { beginAtZero: true, max: 100, ticks: { color: pal.muted, callback: (v) => v + "%" }, grid: { color: pal.grid } },
        },
      },
    }));
  }

  function renderServicesPanel(services) {
    const c = C();
    const panel = c.byId("monitoring-services-panel");
    if (!panel || !Array.isArray(services)) return;
    if (!services.length) { panel.innerHTML = `<div class="text-center text-muted">${c.esc(c.t("admin.monitoring.no_service_data", "Servis verisi yok", "No service data"))}</div>`; return; }
    panel.innerHTML = services.map((s) => `<div class="service-tile">
      <div class="service-tile-left">
        <span class="service-dot ${s.reachable ? "ok" : "down"}"></span>
        <span class="font-semibold text-xs truncate">${c.esc(s.name)}</span>
      </div>
      <span class="text-mono text-xs ${s.reachable ? "text-success" : "text-danger"}">${s.latency_ms != null ? c.esc(s.latency_ms) + " ms" : (s.reachable ? c.esc(c.t("admin.status.active", "Aktif", "Active")) : c.esc(c.t("admin.status.inactive", "Kapalı", "Down")))}</span>
    </div>`).join("");
  }

  function renderDomainsTable(page) {
    const c = C();
    const tbody = c.byId("tbody-monitoring-domains");
    if (!tbody) return;
    if (typeof page === "number") c.paginationState.monDomains = page;
    const cur = c.paginationState.monDomains || 1;
    const q = c.val("mon-domain-search").toLowerCase();
    const filtered = _cachedDomains.filter((d) => !q || String(d.domain || "").toLowerCase().includes(q));
    if (!filtered.length) {
      c.renderTableState(tbody, "empty", { colCount: 7, emptyMsg: c.t("admin.state.no_match_domain", "Eşleşen alan adı bulunamadı.", "No matching domains found.") });
      c.renderPaginationControls("pagination-mon-domains", null, renderDomainsTable);
      return;
    }
    const tp = Math.max(1, Math.ceil(filtered.length / MON_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = filtered.slice((sp - 1) * MON_PER_PAGE, sp * MON_PER_PAGE);
    const daysUnit = c.t("admin.analytics.days_unit", "gün", "days");
    tbody.innerHTML = slice.map((d) => {
      const stCls = d.http_status && d.http_status < 400 ? "text-success" : "text-danger";
      const sslCls = d.ssl_days_left != null && d.ssl_days_left <= 30 ? "text-danger" : d.ssl_days_left != null && d.ssl_days_left <= 90 ? "text-warning" : "text-success";
      const dns = d.dns_resolves ? c.svgIcon("check", "text-success") : c.svgIcon("times", "text-danger");
      return `<tr>
        <td class="font-semibold">${c.esc(d.domain)}</td>
        <td class="text-mono"><strong class="${stCls}">${c.esc(d.http_status || "ERR")}</strong></td>
        <td class="text-mono text-xs">${d.response_ms != null ? c.esc(d.response_ms) + " ms" : "—"}</td>
        <td class="text-mono text-xs">${c.esc(d.ssl_expiry ? String(d.ssl_expiry).split("T")[0] : "—")}</td>
        <td class="font-semibold text-mono ${sslCls}">${d.ssl_days_left != null ? c.esc(d.ssl_days_left) + " " + c.esc(daysUnit) : "—"}</td>
        <td class="text-center">${dns}</td>
        <td class="text-mono text-xs text-muted">${c.esc(d.last_check ? new Date(d.last_check).toLocaleTimeString(c.locale()) : "—")}</td>
      </tr>`;
    }).join("");
    c.renderPaginationControls("pagination-mon-domains", { page: sp, total_pages: tp, total_count: filtered.length }, renderDomainsTable);
  }

  function renderTrafficDistChart(traffic) {
    const c = C();
    const perDomain = Array.isArray(traffic?.per_domain) ? traffic.per_domain : [];
    const active = perDomain
      .filter((d) => Number(d.requests_per_min || 0) > 0)
      .sort((a, b) => Number(b.requests_per_min || 0) - Number(a.requests_per_min || 0))
      .slice(0, 8);
    if (!active.length) {
      c.charts.showEmpty("chart-traffic-dist", c.t("admin.chart.empty_traffic", "Son 60 saniyede alan adı bazlı trafik kaydı yok.", "No domain traffic recorded in the last 60 seconds."), "pie-chart");
      return;
    }
    const labels = active.map((d) => String(d.domain || "").substring(0, 26));
    const data = active.map((d) => Number(d.requests_per_min || 0));
    c.charts.mountOrUpdate("chart-traffic-dist", (pal) => {
      const bg = [pal.accent, pal.info, pal.success, pal.warning, pal.danger, pal.sec, pal.muted, pal.accent];
      return {
        type: "doughnut",
        data: { labels, datasets: [{ data, backgroundColor: bg, borderWidth: 0 }] },
        options: { responsive: true, maintainAspectRatio: false, plugins: { legend: { position: "right", labels: { boxWidth: 12, color: pal.sec, font: { size: 11 } } } } },
      };
    });
  }

  function renderAttacksPanel(attacks) {
    const c = C();
    const panel = c.byId("monitoring-attacks-panel");
    if (!panel) return;
    if (!Array.isArray(attacks) || !attacks.length) {
      panel.innerHTML = `<div class="empty-state">${c.svgIcon("shield", "text-success")}<p>${c.esc(c.t("admin.monitoring.no_attacks", "Son 24 saatte saldırı tespiti yok — sistem güvenli.", "No attack detections in the last 24h — system secure."))}</p></div>`;
      return;
    }
    panel.innerHTML = attacks.map((a) => {
      const sev = String(a.severity || "low").toLowerCase();
      const cls = sev === "high" ? "danger" : sev === "medium" ? "warning" : "info";
      const time = a.detected_at ? new Date(a.detected_at).toLocaleString(c.locale()) : "—";
      return `<div class="flex items-center justify-between gap-3 p-3 border-bottom">
        <div><div class="font-semibold text-sm">${c.esc(a.attack_type)}</div><div class="text-xs text-muted text-mono">${c.esc(a.source_ip)} → ${c.esc(a.target_service || "—")}</div></div>
        <div class="text-right"><span class="badge-pill ${cls}">${c.esc(sev.toUpperCase())}</span><div class="text-xs text-muted">${c.esc(time)}</div></div>
      </div>`;
    }).join("");
  }

  function renderSeoTable(page) {
    const c = C();
    const tbody = c.byId("tbody-monitoring-seo");
    if (!tbody) return;
    if (typeof page === "number") c.paginationState.monSeo = page;
    const cur = c.paginationState.monSeo || 1;
    const q = c.val("mon-seo-search").toLowerCase();
    const filtered = _cachedSeo.filter((s) => !q || [s.domain, ...(s.issues || [])].some((f) => String(f || "").toLowerCase().includes(q)));
    if (!filtered.length) {
      c.renderTableState(tbody, "empty", { colCount: 7, emptyMsg: c.t("admin.state.no_match_seo", "Eşleşen SEO kaydı bulunamadı.", "No matching SEO records found.") });
      c.renderPaginationControls("pagination-mon-seo", null, renderSeoTable);
      return;
    }
    const tp = Math.max(1, Math.ceil(filtered.length / MON_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = filtered.slice((sp - 1) * MON_PER_PAGE, sp * MON_PER_PAGE);
    const chk = (v) => v ? c.svgIcon("check", "text-success") : c.svgIcon("times", "text-danger");
    tbody.innerHTML = slice.map((s) => `<tr>
      <td class="font-semibold">${c.esc(s.domain)}</td>
      <td class="text-center">${chk(s.robots_txt)}</td><td class="text-center">${chk(s.sitemap_xml)}</td>
      <td class="text-center">${chk(s.meta_title)}</td><td class="text-center">${chk(s.meta_description)}</td><td class="text-center">${chk(s.canonical)}</td>
      <td class="text-xs">${Array.isArray(s.issues) && s.issues.length ? s.issues.map((i) => c.esc(i)).join(", ") : `<span class="badge-pill success">${c.esc(c.t("admin.monitoring.seo_compliant", "Tam Uyumlu", "Compliant"))}</span>`}</td>
    </tr>`).join("");
    c.renderPaginationControls("pagination-mon-seo", { page: sp, total_pages: tp, total_count: filtered.length }, renderSeoTable);
  }

  function renderNetworkPanel(system) {
    const c = C();
    const panel = c.byId("monitoring-network-panel");
    if (!panel || !system) return;
    const rx = (Number(system.net_rx_per_sec || 0) / 1048576).toFixed(2);
    const tx = (Number(system.net_tx_per_sec || 0) / 1048576).toFixed(2);
    panel.innerHTML = `<div class="health-metrics-grid">
      <div class="health-metric-item"><div class="health-metric-label">${c.svgIcon("arrow-down", "text-success")} ${c.esc(c.t("admin.monitoring.incoming_traffic", "Gelen Trafik (RX)", "Incoming Traffic (RX)"))}</div><div class="health-metric-val text-success">${c.esc(rx)} MB/sn</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${c.svgIcon("arrow-up", "text-info")} ${c.esc(c.t("admin.monitoring.outgoing_traffic", "Giden Trafik (TX)", "Outgoing Traffic (TX)"))}</div><div class="health-metric-val text-info">${c.esc(tx)} MB/sn</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${c.esc(c.t("admin.monitoring.load_1m", "Çekirdek Yükü (1dk)", "Core Load (1m)"))}</div><div class="health-metric-val">${Number(system.cpu?.load_avg_1m || 0).toFixed(2)}</div></div>
      <div class="health-metric-item"><div class="health-metric-label">${c.esc(c.t("admin.monitoring.load_15m", "Çekirdek Yükü (15dk)", "Core Load (15m)"))}</div><div class="health-metric-val">${Number(system.cpu?.load_avg_15m || 0).toFixed(2)}</div></div>
    </div>`;
  }

  async function refreshMonitoring() {
    const c = C();
    if (!c.hasPerm("settings:write")) { c.showToast(c.t("admin.error.monitoring_refresh_perm", "İzleme yenileme yetkiniz bulunmuyor.", "You do not have permission to refresh monitoring."), true); return; }
    const resp = await c.apiRequest("/api/v1/admin/monitoring/refresh", {
      method: "POST", headers: { "Content-Type": "application/json" }, body: "{}",
    });
    if (resp.ok) { c.showToast(c.t("admin.toast.monitoring_refreshing", "Sunucu izleme verileri yenileniyor...", "Refreshing server telemetry...")); setTimeout(loadMonitoringBundle, 1500); }
    else c.showToast(resp.errorMessage, true);
  }

  function startMonitoringAutoRefresh(skipImmediate) {
    const c = C();
    if (!c.hasPerm(MON_PERM)) return;
    if (_timer) clearInterval(_timer);
    if (!skipImmediate) loadMonitoringBundle();
    _timer = setInterval(loadMonitoringBundle, 15000);
  }

  function stopMonitoringAutoRefresh() {
    if (_timer) { clearInterval(_timer); _timer = null; }
    if (_followUpTimer) { clearTimeout(_followUpTimer); _followUpTimer = null; }
  }

  const c = C();
  c.registerLoader(loadMonitoringBundle);
  c.registerActions({ "refresh-monitoring": () => refreshMonitoring() });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "mon-domain-search") renderDomainsTable(1);
    else if (ev.target?.id === "mon-seo-search") renderSeoTable(1);
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail?.tabId === "tab-17") startMonitoringAutoRefresh(!!ev.detail?.initial);
    else stopMonitoringAutoRefresh();
  });
})();
