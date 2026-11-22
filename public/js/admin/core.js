(function () {
  "use strict";

  const THEME_KEY = "msi_admin_theme";
  const LEGACY_THEME_KEY = "msi_theme";
  let _glowBound = false;
  let _storageBound = false;
  let _autoThemeBound = false;
  let _autoClockTimer = null;
  let _i18nParsed = false;

  const byId = (id) => document.getElementById(id);
  const val = (id) => (byId(id)?.value ?? "").trim();
  const setVal = (id, v) => { const el = byId(id); if (el) el.value = v ?? ""; };
  const esc = (v) => String(v ?? "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;");

  function getCookieValue(name) {
    const p = name + "=";
    const m = document.cookie.split(";").map((c) => c.trim()).find((c) => c.startsWith(p));
    if (!m) return "";
    try { return decodeURIComponent(m.substring(p.length)); } catch (_) { return ""; }
  }

  function writeThemeCookie(theme) {
    const sec = typeof location !== "undefined" && location.protocol === "https:" ? "; Secure" : "";
    document.cookie = `${THEME_KEY}=${encodeURIComponent(theme)}; path=/; max-age=31536000; SameSite=Lax${sec}`;
  }

  function resolveAutoTheme() {
    try {
      if (window.matchMedia) {
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
        if (window.matchMedia("(prefers-color-scheme: light)").matches) return "light";
      }
    } catch (_) {}
    const hour = new Date().getHours();
    return hour >= 19 || hour < 7 ? "dark" : "light";
  }

  function getExplicitStoredTheme() {
    try {
      const ls = localStorage.getItem(THEME_KEY);
      if (ls === "dark" || ls === "light") return ls;
    } catch (_) {}
    const ck = getCookieValue(THEME_KEY);
    if (ck === "dark" || ck === "light") return ck;
    try {
      const leg = localStorage.getItem(LEGACY_THEME_KEY);
      if (leg === "dark" || leg === "light") return leg;
    } catch (_) {}
    return "";
  }

  function getStoredTheme() {
    return getExplicitStoredTheme() || resolveAutoTheme();
  }

  function applyTheme(theme) {
    const next = theme === "dark" ? "dark" : "light";
    document.documentElement.setAttribute("data-theme", next);
    if (document.body) {
      document.body.classList.toggle("admin-dark-mode", next === "dark");
      document.body.classList.toggle("admin-light-mode", next === "light");
      document.body.classList.toggle("gw-light-mode", next === "light");
    }
    document.querySelectorAll('[data-admin-action="toggle-theme"]').forEach((btn) => {
      btn.setAttribute("aria-pressed", String(next === "dark"));
    });
    return next;
  }

  function bindAutoThemeWatchers(forceAutoOnly) {
    if (_autoThemeBound) return;
    _autoThemeBound = true;
    const sync = () => {
      if (!forceAutoOnly && getExplicitStoredTheme()) return;
      const next = resolveAutoTheme();
      if (document.documentElement.getAttribute("data-theme") !== next) {
        applyTheme(next);
        window.dispatchEvent(new CustomEvent("msi:admin:theme", { detail: { theme: next } }));
      }
    };
    try {
      if (window.matchMedia) {
        const mqDark = window.matchMedia("(prefers-color-scheme: dark)");
        const mqLight = window.matchMedia("(prefers-color-scheme: light)");
        mqDark.addEventListener?.("change", sync);
        mqLight.addEventListener?.("change", sync);
      }
    } catch (_) {}
    if (typeof window.setInterval === "function") {
      _autoClockTimer = window.setInterval(sync, 60000);
    }
  }

  function initAutoTheme() {
    const current = applyTheme(resolveAutoTheme());
    bindAutoThemeWatchers(true);
    return current;
  }

  function initTheme() {
    const current = applyTheme(getStoredTheme());
    bindAutoThemeWatchers(false);
    if (!_storageBound) {
      _storageBound = true;
      window.addEventListener("storage", (e) => {
        if ((e.key === THEME_KEY || e.key === LEGACY_THEME_KEY) && (e.newValue === "dark" || e.newValue === "light")) {
          const applied = applyTheme(e.newValue);
          window.dispatchEvent(new CustomEvent("msi:admin:theme", { detail: { theme: applied } }));
        }
      });
    }
    return current;
  }

  function toggleTheme() {
    const next = document.documentElement.getAttribute("data-theme") === "dark" ? "light" : "dark";
    applyTheme(next);
    try {
      localStorage.setItem(THEME_KEY, next);
      localStorage.setItem(LEGACY_THEME_KEY, next);
    } catch (_) {}
    writeThemeCookie(next);
    window.dispatchEvent(new CustomEvent("msi:admin:theme", { detail: { theme: next } }));
    return next;
  }

  function bindGlow() {
    if (_glowBound) return;
    _glowBound = true;
    document.addEventListener("pointermove", (ev) => {
      const card = ev.target?.closest?.("[data-glow-card]");
      if (!card) return;
      const r = card.getBoundingClientRect();
      card.style.setProperty("--glow-x", `${Math.round(ev.clientX - r.left)}px`);
      card.style.setProperty("--glow-y", `${Math.round(ev.clientY - r.top)}px`);
    });
  }

  const SVG_PATHS = {
    pencil: '<path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5Z"/><path d="m15 5 4 4"/>',
    edit: '<path d="M12 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.375 2.625a1 1 0 0 1 3 3l-9.013 9.014a2 2 0 0 1-.853.505l-2.873.84a.5.5 0 0 1-.62-.62l.84-2.873a2 2 0 0 1 .506-.852z"/>',
    trash: '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/>',
    "paper-plane": '<path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>',
    star: '<polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"/>',
    shield: '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/>',
    check: '<path d="M20 6 9 17l-5-5"/>',
    "check-circle": '<circle cx="12" cy="12" r="10"/><path d="m9 12 2 2 4-4"/>',
    "exclamation-circle": '<circle cx="12" cy="12" r="10"/><line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>',
    "info-circle": '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
    "exclamation-triangle": '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
    times: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
    undo: '<path d="M3 7v6h6"/><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"/>',
    inbox: '<polyline points="22 12 16 12 14 15 10 15 8 12 2 12"/><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z"/>',
    magic: '<path d="m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72"/><path d="m14 7 3 3"/>',
    "file-pdf-o": '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M10 9H8"/><path d="M16 13H8"/><path d="M16 17H8"/>',
    "file-o": '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/>',
    "external-link": '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
    "clock-o": '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>',
    "arrows-v": '<path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/>',
    "arrow-down": '<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>',
    "arrow-up": '<path d="M12 19V5"/><path d="m5 12 7-7 7 7"/>',
    "arrow-right": '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
    lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
    unlock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 9.9-1"/>',
    minus: '<path d="M5 12h14"/>',
    save: '<path d="M15.2 3a2 2 0 0 1 1.4.6l3.8 3.8a2 2 0 0 1 .6 1.4V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z"/><path d="M17 21v-7a1 1 0 0 0-1-1H8a1 1 0 0 0-1 1v7"/><path d="M7 3v4a1 1 0 0 0 1 1h7"/>',
    "bar-chart": '<path d="M3 3v18h18"/><path d="M18 17V9"/><path d="M13 17V5"/><path d="M8 17v-3"/>',
    "line-chart": '<path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/>',
    "area-chart": '<path d="M3 3v18h18"/><path d="m19 9-5 5-4-4-3 3"/>',
    "pie-chart": '<path d="M21.21 15.89A10 10 0 1 1 8 2.83"/><path d="M22 12A10 10 0 0 0 12 2v10z"/>',
    filter: '<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>',
    eye: '<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>',
    "eye-slash": '<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>',
    spinner: '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
    "sign-in": '<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><polyline points="10 17 15 12 10 7"/><line x1="15" x2="3" y1="12" y2="12"/>',
    envelope: '<rect width="20" height="16" x="2" y="4" rx="2"/><path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7"/>',
  };

  function svgIcon(name, extraCls = "") {
    const raw = String(name || "").trim().replace(/^fa\s+/i, "").replace(/^fa-/, "");
    const inner = SVG_PATHS[raw] || '<circle cx="12" cy="12" r="9"/><path d="M12 8v4"/><path d="M12 16h.01"/>';
    const cls = ["adm-icon", raw === "spinner" ? "adm-spin" : "", extraCls].filter(Boolean).join(" ");
    return `<svg class="${esc(cls)}" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${inner}</svg>`;
  }

  window.MSIAdminTheme = { getCookieValue, resolveAutoTheme, getStoredTheme, applyTheme, initAutoTheme, initTheme, toggleTheme, bindGlow, svgIcon };

  const getLang = () => ((document.body?.dataset?.adminLang || document.documentElement?.lang || "tr").trim().toLowerCase().startsWith("en") ? "en" : "tr");
  const locale = () => (getLang() === "en" ? "en-US" : "tr-TR");

  function t(key, fbTr, fbEn) {
    if (!_i18nParsed && typeof document !== "undefined") {
      const el = document.getElementById("msi-i18n");
      if (el) {
        _i18nParsed = true;
        try { window.__MSI_I18N__ = Object.assign({}, JSON.parse(el.textContent || "{}"), window.__MSI_I18N__ || {}); } catch (_) {}
      }
    }
    const map = window.__MSI_I18N__ || window.__MSI_I18N || window.MSIAdmin?._i18n;
    if (map && typeof map === "object" && Object.prototype.hasOwnProperty.call(map, key) && map[key] !== "") {
      return String(map[key]);
    }
    if (getLang() === "en" && fbEn !== undefined) return fbEn;
    return fbTr !== undefined ? fbTr : key;
  }

  const RESOURCE_ALIASES = {
    content: "contents", category: "categories", approval: "approvals", newsletter: "newsletters",
    notification: "notifications", subscription: "subscriptions", setting: "settings", user: "users",
    role: "roles", ad: "ads", translation: "translations", corporate_request: "corporate_requests",
    issue: "issues", invoice: "invoices", product: "commerce", plan: "subscriptions",
    redirect: "redirects", audit_log: "audit", audit_logs: "audit",
  };

  const ROLE_LABELS_TR = {
    super_admin: "Süper Yönetici", site_operator: "Baş Operatör", admin: "Yönetici",
    editor_in_chief: "Genel Yayın Yönetmeni", managing_editor: "Yazı İşleri Müdürü",
    content_publisher: "İçerik Yayınlayıcı", content_editor: "İçerik Editörü", editor: "Editör",
    business_dev: "İş Geliştirme", ads_manager: "Reklam Yöneticisi", finance_viewer: "Finans Uzmanı", support_agent: "Destek Temsilcisi",
  };

  const ROLE_LABELS_EN = {
    super_admin: "Super Admin", site_operator: "Lead Operator", admin: "Admin",
    editor_in_chief: "Editor-in-Chief", managing_editor: "Managing Editor",
    content_publisher: "Content Publisher", content_editor: "Content Editor", editor: "Editor",
    business_dev: "Business Development", ads_manager: "Ads Manager", finance_viewer: "Finance Specialist", support_agent: "Support Agent",
  };

  const ROLE_LANDING_TABS = {
    ads_manager: ["tab-2", "tab-3", "tab-10"], business_dev: ["tab-2", "tab-14", "tab-3"],
    finance_viewer: ["tab-14", "tab-3", "tab-15"], support_agent: ["tab-4", "tab-7", "tab-11", "tab-8"],
    content_publisher: ["tab-1", "tab-16", "tab-18"], content_editor: ["tab-1", "tab-16", "tab-18"],
    editor: ["tab-1", "tab-16", "tab-18"], managing_editor: ["tab-1", "tab-16", "tab-18"], editor_in_chief: ["tab-1", "tab-16", "tab-18"],
  };

  const ROLE_WORKSPACE_BRIEFS = {
    super_admin: {
      tr: ["Tam Sistem Yetkisi", "Tüm yayın, reklam, B2B analitik, RBAC, yedekleme ve tasarım stüdyosu modülleri tam yetkiyle açık."],
      en: ["Full Platform Authority", "All editorial, commercial, B2B analytics, RBAC, backup and Design Studio modules unlocked."],
    },
    admin: {
      tr: ["Platform Yönetimi", "Yayın operasyonu, ticari yönetim, kullanıcı RBAC ve sistem ayarları tek merkezden kontrol edilir."],
      en: ["Platform Administration", "Manage editorial operations, commercial pipelines, RBAC users and system configuration."],
    },
    site_operator: {
      tr: ["Operasyon Komutası", "Günlük yayın akışı, E-Dergi dönüşümü, kampanya yönetimi ve izleme panelleri aktif."],
      en: ["Operations Command", "Daily publishing pipeline, E-Magazine conversion, campaign operations and monitoring active."],
    },
    editor_in_chief: {
      tr: ["Genel Yayın Masası", "Yayın onay kuyruğu, E-Dergi arşivi, bülten planlama, çeviri ve Tasarım Stüdyosu emrinizde."],
      en: ["Editor-in-Chief Desk", "Approval queue, E-Magazine archive, newsletter scheduling, i18n and Design Studio ready."],
    },
    managing_editor: {
      tr: ["Yazı İşleri Akışı", "İçerik envanteri, editoryal onaylar, medya kütüphanesi ve bülten gönderimleri yönetilir."],
      en: ["Managing Editor Desk", "Control content inventory, editorial approvals, media library and newsletter dispatches."],
    },
    content_publisher: {
      tr: ["Yayın & Onay Masası", "İçerik yayınlama, onay kararları, medya yönetimi ve 301 SEO yönlendirmeleri aktif."],
      en: ["Publishing & Sign-off", "Content publishing, approval sign-off, media management and 301 SEO redirects active."],
    },
    content_editor: {
      tr: ["İçerik Üretim Masası", "Çok dilli makale taslakları, 14 SSB etiketleme, medya kütüphanesi ve toplu düzenlemeler hazır."],
      en: ["Editorial Production", "Bilingual article drafts, 14 SSB tagging, media library and bulk operations ready."],
    },
    editor: {
      tr: ["Editoryal Çalışma Alanı", "Haber ve analiz taslakları oluşturun, medya ve taksonomi eşlemelerini düzenleyin."],
      en: ["Editorial Workspace", "Author news and analysis drafts, and organise media and taxonomy mappings."],
    },
    business_dev: {
      tr: ["Ticari & Gelir Masası", "Sponsorlu reklam kampanyaları, gelir özetleri, abonelik paketleri ve B2B analitik aktif."],
      en: ["Commercial & Revenue", "Sponsored ad campaigns, revenue summaries, subscription plans and B2B analytics active."],
    },
    ads_manager: {
      tr: ["Reklam & Sponsorluk", "Reklam kuşakları, kota çakışma kontrolü, kurum/SSB hedefleme ve kampanya CTR analitiği hazır."],
      en: ["Advertising & Sponsors", "Ad slots, quota conflict detection, institution/SSB targeting and campaign CTR analytics ready."],
    },
    finance_viewer: {
      tr: ["Finans & Tahsilat", "Günlük tahsilat eğilimi, fatura mutabakatı, abonelik yaşam döngüsü ve LTV metrikleri hazır."],
      en: ["Finance & Collections", "Daily collections, invoice reconciliation, subscriber lifecycle and LTV metrics ready."],
    },
    support_agent: {
      tr: ["Kurumsal Destek & Onay", ".gov.tr / .tsk.tr kurumsal erişim başvuruları, üye kayıtları ve yenileme hatırlatmaları aktif."],
      en: ["Corporate Support", "Review .gov.tr / .tsk.tr corporate applications, member accounts and renewal reminders."],
    },
  };

  const ROLE_HERO_PRIORITIES = {
    super_admin: ["hero-open-content-drawer", "editorial_studio", "tab-16", "open-ad-drawer"],
    admin: ["hero-open-content-drawer", "editorial_studio", "tab-16", "open-ad-drawer"],
    site_operator: ["hero-open-content-drawer", "editorial_studio", "tab-16", "open-ad-drawer"],
    editor_in_chief: ["hero-open-content-drawer", "editorial_studio", "tab-16", "tab-18"],
    managing_editor: ["hero-open-content-drawer", "editorial_studio", "tab-16", "tab-18"],
    content_publisher: ["hero-open-content-drawer", "editorial_studio", "tab-16", "tab-18"],
    content_editor: ["hero-open-content-drawer", "editorial_studio", "tab-18"],
    editor: ["hero-open-content-drawer", "editorial_studio", "tab-18"],
    business_dev: ["open-ad-drawer", "tab-14", "tab-3"],
    ads_manager: ["open-ad-drawer", "tab-3"],
    finance_viewer: ["tab-14", "tab-3"],
    support_agent: ["tab-4", "tab-14", "tab-11"],
  };

  const prefersReducedMotion = () => Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches);

  const state = {
    overviewStats: null, totalContentsCount: 0, contents: [], ads: [], contentAnalytics: [],
    adAnalytics: [], readerAnalytics: [], corporateRequests: [], categories: [], ssbTags: [],
    recommendationRules: [], newsletters: [], reminders: [], redirects: [], mediaLibrary: [],
    auditLogs: [], userRoles: [], adminUsers: [], customFields: [], cacheOperations: [],
    securityLogs: [], systemHealth: null, issues: [], plans: [], products: [], translations: [],
    pendingTranslations: [], glossary: [], locales: [], emailGroups: [],
    activeAnalyticsSubview: "content", bulkSelected: new Set(),
  };

  const paginationState = {
    contents: 1, adminUsers: 1, recRules: 1, userRoles: 1, customFields: 1, cacheOperations: 1,
    newsletters: 1, reminders: 1, redirects: 1, auditLogs: 1, versionHistory: 1, mediaLibrary: 1,
    translations: 1, pendingTranslations: 1, invoices: 1, issues: 1, ads: 1,
    analyticsContent: 1, analyticsAd: 1, analyticsReader: 1, securityLogs: 1, monDomains: 1, monSeo: 1,
  };
  const paginationHandlers = {}, _loaders = [], _actions = Object.create(null);
  const _chartRegistry = new Map();
  let _mePromise = null;

  const getOperatorIdentity = () => {
    const me = window.MSIAdmin?._me, ds = document.body?.dataset || {};
    return { email: (me?.email || ds.operatorEmail || "").trim(), role: (me?.role || ds.operatorRole || "").trim(), adminPath: (ds.adminPath || "/podmin").trim() };
  };
  const unwrapData = (p) => (p && typeof p === "object" && "status" in p && "data" in p && p.data != null) ? p.data : p;
  function unwrapList(p, key) {
    if (!p) return [];
    if (Array.isArray(p)) return p;
    if (key && Array.isArray(p[key])) return p[key];
    if (Array.isArray(p.items)) return p.items;
    const d = p.data;
    if (Array.isArray(d)) return d;
    if (d && key && Array.isArray(d[key])) return d[key];
    if (d && Array.isArray(d.items)) return d.items;
    return (d && Array.isArray(d.data)) ? d.data : [];
  }
  function extractMeta(p) {
    if (!p || typeof p !== "object") return null;
    const m = p.meta || p.data?.meta || (typeof p.total_pages === "number" ? p : null);
    if (!m) return null;
    const page = Number(m.page ?? m.current_page ?? 1), tp = Number(m.total_pages ?? 0), tc = Number(m.total_count ?? m.total ?? 0);
    return { page: page > 0 ? page : 1, total_pages: tp >= 0 ? tp : 0, total_count: tc >= 0 ? tc : 0 };
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
      const res = await fetch(url, opts), text = await res.text();
      let payload = null;
      if (text) { try { payload = JSON.parse(text); } catch (_) {} }
      if (!res.ok) {
        const rawErr = payload?.message || (typeof payload?.error === "string" ? payload.error : payload?.error?.message);
        const errorMessage = rawErr ||
          (res.status === 401 ? t("admin.error.session_expired", "Oturum süresi doldu (401).", "Session expired (401).")
            : res.status === 403 ? t("admin.error.forbidden", "Bu işlem için yetkiniz bulunmuyor (403).", "You do not have permission for this action (403).")
            : t("admin.error.http_failed", "İstek başarısız (HTTP ${status}).", "Request failed (HTTP ${status}).").replace("${status}", String(res.status)));
        return { ok: false, status: res.status, payload, data: null, meta: null, errorMessage };
      }
      return { ok: true, status: res.status, payload, data: unwrapData(payload), meta: extractMeta(payload), errorMessage: "" };
    } catch (err) {
      return { ok: false, status: 0, payload: null, data: null, meta: null, errorMessage: t("admin.error.network", "Ağ bağlantı hatası: ", "Network connection error: ") + (err?.message || t("admin.error.unreachable", "Sunucuya ulaşılamadı", "Server unreachable")) };
    }
  }
  const safeFetchJson = async (url, opts = {}) => { const r = await apiRequest(url, opts); return r.ok ? r.payload : null; };

  async function uploadFile(inputEl) {
    const file = inputEl?.files?.[0];
    if (!file) return null;
    const fd = new FormData();
    fd.append("file", file);
    const r = await apiRequest("/api/v1/admin/upload", { method: "POST", body: fd });
    inputEl.value = "";
    if (!r.ok) {
      showToast(r.errorMessage || t("admin.toast.upload_failed", "Dosya yüklenemedi.", "File upload failed."), true);
      return null;
    }
    const d = r.data || r.payload || {};
    return d.path || d.url || d.file_path || null;
  }

  /* ── Centralized Toast System ─────────────────────────────────────────── */
  const TOAST_ICONS = {
    success: "check-circle",
    error: "exclamation-circle",
    warning: "exclamation-triangle",
    info: "info-circle",
  };
  const TOAST_SEMANTIC = { success: "text-success", error: "text-danger", warning: "text-warning", info: "text-info" };
  let _toastRegion = null;
  function _getToastRegion() {
    if (_toastRegion) return _toastRegion;
    _toastRegion = document.querySelector(".ops-toast-region");
    if (!_toastRegion) {
      _toastRegion = document.createElement("div");
      _toastRegion.className = "ops-toast-region";
      _toastRegion.setAttribute("aria-live", "polite");
      _toastRegion.setAttribute("aria-relevant", "additions");
      document.body.appendChild(_toastRegion);
    }
    return _toastRegion;
  }
  function dismissToast(el) {
    if (!el || el.classList.contains("is-leaving")) return;
    el.classList.add("is-leaving");
    el.addEventListener("animationend", () => el.remove(), { once: true });
    setTimeout(() => { if (el.parentNode) el.remove(); }, 250);
  }
  /**
   * @param {string} msg - Toast message text
   * @param {boolean|string} isErrorOrType - true/false for legacy compat, or "success"|"error"|"warning"|"info"
   * @param {number} [duration=4200] - Auto-dismiss ms; pass 0 to persist
   */
  function showToast(msg, isErrorOrType, duration) {
    const region = _getToastRegion();
    let variant = "success";
    if (isErrorOrType === true) variant = "error";
    else if (typeof isErrorOrType === "string" && TOAST_ICONS[isErrorOrType]) variant = isErrorOrType;
    else if (isErrorOrType === false || isErrorOrType == null) variant = "success";
    const icon = TOAST_ICONS[variant];
    const semClass = TOAST_SEMANTIC[variant];
    const toast = document.createElement("div");
    toast.className = `ops-toast toast-${variant}`;
    toast.setAttribute("role", variant === "error" ? "alert" : "status");
    toast.innerHTML = `<span class="ops-toast-icon" aria-hidden="true">${svgIcon(icon, semClass)}</span><span class="ops-toast-body"></span><button type="button" class="ops-toast-close" aria-label="${t("admin.toast.dismiss", "Kapat", "Dismiss")}">${svgIcon("times", "")}</button>`;
    toast.querySelector(".ops-toast-body").textContent = String(msg ?? "");
    toast.querySelector(".ops-toast-close").addEventListener("click", () => dismissToast(toast));
    region.appendChild(toast);
    const ms = typeof duration === "number" ? duration : 4200;
    if (ms > 0) {
      setTimeout(() => dismissToast(toast), ms);
    }
  }

  function openModal(modalOrId, focusEl) {
    const modal = typeof modalOrId === "string" ? byId(modalOrId) : modalOrId;
    if (!modal) return;
    modal._prevFocus = document.activeElement;
    modal.classList.remove("hidden");
    modal.setAttribute("aria-hidden", "false");
    if (window.gsap && !prefersReducedMotion()) {
      const card = modal.querySelector(".modal-content");
      window.gsap.killTweensOf([modal, card].filter(Boolean));
      window.gsap.fromTo(modal, { opacity: 0.5 }, { opacity: 1, duration: 0.16, ease: "power2.out", clearProps: "opacity" });
      if (card) {
        window.gsap.fromTo(card, { y: 10, scale: 0.97, opacity: 0.85 }, { y: 0, scale: 1, opacity: 1, duration: 0.2, ease: "power3.out", clearProps: "transform,opacity" });
      }
    }
    const targetFocus = focusEl || modal.querySelector("button:not([disabled]), input:not([type=hidden]):not([disabled]), select:not([disabled]), textarea:not([disabled])");
    if (targetFocus && typeof targetFocus.focus === "function") {
      requestAnimationFrame(() => { try { targetFocus.focus({ preventScroll: true }); } catch (_) {} });
    }
  }

  function closeModal(modalOrId) {
    const modal = typeof modalOrId === "string" ? byId(modalOrId) : modalOrId;
    if (!modal || modal.classList.contains("hidden")) return;
    const card = modal.querySelector(".modal-content");
    if (window.gsap) window.gsap.killTweensOf([modal, card].filter(Boolean));
    modal.classList.add("hidden");
    modal.setAttribute("aria-hidden", "true");
    const prev = modal._prevFocus;
    modal._prevFocus = null;
    if (prev && typeof prev.focus === "function") {
      try { prev.focus({ preventScroll: true }); } catch (_) {}
    }
  }

  let _confirmResolver = null;
  let _confirmPrevFocus = null;
  function closeConfirmModal(accepted) {
    const modal = byId("ops-confirm-modal");
    if (!modal || modal.classList.contains("hidden")) return;
    const card = modal.querySelector(".modal-content");
    if (window.gsap) window.gsap.killTweensOf([modal, card].filter(Boolean));
    modal.classList.add("hidden");
    modal.setAttribute("aria-hidden", "true");
    const resolve = _confirmResolver;
    _confirmResolver = null;
    const prev = _confirmPrevFocus;
    _confirmPrevFocus = null;
    if (prev && typeof prev.focus === "function") {
      try { prev.focus({ preventScroll: true }); } catch (_) {}
    }
    if (typeof resolve === "function") resolve(!!accepted);
  }

  function confirmDialog(opts) {
    const cfg = typeof opts === "string" ? { message: opts } : (opts || {});
    const modal = byId("ops-confirm-modal");
    if (!modal) return Promise.resolve(false);
    if (_confirmResolver) closeConfirmModal(false);
    _confirmPrevFocus = document.activeElement;
    const titleEl = byId("ops-confirm-title-text");
    const msgEl = byId("ops-confirm-message");
    const acceptBtn = byId("ops-confirm-accept");
    const cancelBtn = byId("ops-confirm-cancel");
    if (titleEl) titleEl.textContent = cfg.title || t("admin.confirm.title", "İşlem Onayı", "Confirm Action");
    if (msgEl) msgEl.textContent = String(cfg.message || "");
    if (cancelBtn) cancelBtn.textContent = cfg.cancelLabel || t("admin.form.cancel", "Vazgeç", "Cancel");
    if (acceptBtn) {
      acceptBtn.textContent = cfg.confirmLabel || t("admin.confirm.accept", "Onayla", "Confirm");
      acceptBtn.className = cfg.tone === "warning" ? "btn-cmd btn-cmd-warning" : cfg.tone === "primary" ? "btn-cmd btn-cmd-primary" : "btn-cmd btn-cmd-danger";
    }
    modal.classList.remove("hidden");
    modal.setAttribute("aria-hidden", "false");
    if (window.gsap && !prefersReducedMotion()) {
      const card = modal.querySelector(".modal-content");
      window.gsap.killTweensOf([modal, card].filter(Boolean));
      window.gsap.fromTo(modal, { opacity: 0.5 }, { opacity: 1, duration: 0.16, ease: "power2.out", clearProps: "opacity" });
      if (card) {
        window.gsap.fromTo(card, { y: 10, scale: 0.97, opacity: 0.85 }, { y: 0, scale: 1, opacity: 1, duration: 0.2, ease: "power3.out", clearProps: "transform,opacity" });
      }
    }
    requestAnimationFrame(() => { (cancelBtn || acceptBtn)?.focus(); });
    return new Promise((resolve) => { _confirmResolver = resolve; });
  }

  function renderTableState(tbody, mode, opts = {}) {
    if (!tbody) return;
    const cols = opts.colCount || tbody.closest("table")?.querySelectorAll("thead th").length || 5;
    if (mode === "loading") tbody.innerHTML = Array.from({ length: 3 }, () => `<tr class="skeleton-row">${"<td></td>".repeat(cols)}</tr>`).join("");
    else if (mode === "empty") tbody.innerHTML = `<tr><td colspan="${cols}" class="empty-state">${svgIcon("inbox")}<p>${esc(opts.emptyMsg || t("admin.state.empty", "Kayıt bulunamadı.", "No records found."))}</p></td></tr>`;
    else if (mode === "error") {
      const retry = opts.retryAction ? `<button type="button" class="btn-cmd btn-cmd-sm mt-2" data-admin-action="${esc(opts.retryAction)}">${esc(t("admin.action.retry", "Tekrar Dene", "Retry"))}</button>` : "";
      tbody.innerHTML = `<tr><td colspan="${cols}" class="error-state">${svgIcon("exclamation-triangle")}<p>${esc(opts.errorMsg || t("admin.state.error", "Veri yüklenemedi.", "Failed to load data."))}</p>${retry}</td></tr>`;
    }
  }

  function renderPaginationControls(containerId, rawMeta, fetchPageFn) {
    const container = byId(containerId);
    const meta = rawMeta && typeof rawMeta.total_pages === "number" ? rawMeta : extractMeta({ meta: rawMeta });
    if (!container || !meta || meta.total_pages <= 1) { if (container) container.innerHTML = ""; return; }
    paginationHandlers[containerId] = fetchPageFn;
    const prevLbl = t("admin.pagination.prev", "← Önceki", "← Previous");
    const nextLbl = t("admin.pagination.next", "Sonraki →", "Next →");
    const summary = t("admin.pagination.summary", "Sayfa ${page} / ${total_pages} · Toplam ${total} kayıt", "Page ${page} / ${total_pages} · ${total} total")
      .replace("${page}", String(meta.page))
      .replace("${total_pages}", String(meta.total_pages))
      .replace("${total}", Number(meta.total_count ?? 0).toLocaleString(locale()));
    container.innerHTML = `<div class="pagination-controls">
      <button type="button" class="btn-cmd btn-cmd-sm" ${meta.page <= 1 ? "disabled" : ""} data-admin-action="paginate" data-container="${esc(containerId)}" data-page="${meta.page - 1}">${esc(prevLbl)}</button>
      <span class="text-xs text-secondary font-semibold">${esc(summary)}</span>
      <button type="button" class="btn-cmd btn-cmd-sm" ${meta.page >= meta.total_pages ? "disabled" : ""} data-admin-action="paginate" data-container="${esc(containerId)}" data-page="${meta.page + 1}">${esc(nextLbl)}</button>
    </div>`;
  }

  function renderChipSlice(items, cls = "", limit = 2) {
    const list = Array.isArray(items) ? items.filter(Boolean) : [];
    if (!list.length) return "";
    const badgeCls = cls ? `badge-tag ${esc(cls)}` : "badge-tag";
    const head = list.slice(0, limit).map((x) => `<span class="${badgeCls}">${esc(x)}</span>`).join(" ");
    return head + (list.length > limit ? ` <span class="badge-tag">+${list.length - limit}</span>` : "");
  }

  function downloadBlob(content, filename, mimeType) {
    const url = URL.createObjectURL(new Blob([content], { type: mimeType })), a = document.createElement("a");
    a.href = url; a.download = filename; document.body.appendChild(a); a.click(); document.body.removeChild(a); URL.revokeObjectURL(url);
  }

  // ── Centralized Chart.js Lifecycle & Empty-State Manager ────────────────
  const charts = {
    palette() {
      const cs = getComputedStyle(document.documentElement);
      const v = (n, fb) => (cs.getPropertyValue(n) || "").trim() || fb;
      return {
        text: v("--adm-text", "#1e293b"),
        sec: v("--adm-text-secondary", "#475569"),
        muted: v("--adm-text-muted", "#556377"),
        grid: v("--adm-border", "rgba(30, 41, 59, 0.09)"),
        accent: v("--adm-accent", "#3b66c4"),
        accentSoft: v("--adm-accent-soft", "rgba(59, 102, 196, 0.10)"),
        success: v("--adm-success", "#176b50"),
        successSoft: v("--adm-success-soft", "rgba(23, 107, 80, 0.10)"),
        info: v("--adm-info", "#1d5fa3"),
        infoSoft: v("--adm-info-soft", "rgba(29, 95, 163, 0.10)"),
        warning: v("--adm-warning", "#8f4f12"),
        warningSoft: v("--adm-warning-soft", "rgba(143, 79, 18, 0.10)"),
        danger: v("--adm-danger", "#a83333"),
        dangerSoft: v("--adm-danger-soft", "rgba(168, 51, 51, 0.10)"),
      };
    },
    _getEmptyEl(canvas, canvasId) {
      if (!canvas) return null;
      if (canvasId === "chart-revenue-daily") {
        const rev = byId("revenue-chart-empty");
        if (rev) { rev.classList.add("chart-empty-state"); return rev; }
      }
      const parent = canvas.parentElement;
      if (!parent) return null;
      let el = parent.querySelector(`.chart-empty-state[data-for="${canvasId}"]`);
      if (!el) {
        el = document.createElement("div");
        el.className = "chart-empty-state hidden";
        el.setAttribute("data-for", canvasId);
        parent.appendChild(el);
      }
      return el;
    },
    destroy(canvasId) {
      const entry = _chartRegistry.get(canvasId);
      if (entry?.instance) {
        try { entry.instance.destroy(); } catch (_) {}
      }
      _chartRegistry.delete(canvasId);
      const canvas = byId(canvasId);
      if (canvas?._chartInstance) {
        try { canvas._chartInstance.destroy(); } catch (_) {}
        canvas._chartInstance = null;
      }
    },
    showEmpty(canvasId, msg, iconCls = "fa-bar-chart") {
      this.destroy(canvasId);
      const canvas = byId(canvasId);
      if (!canvas) return;
      canvas.classList.add("hidden");
      const emptyEl = this._getEmptyEl(canvas, canvasId);
      if (emptyEl) {
        if (emptyEl.id === "revenue-chart-empty") {
          const p = emptyEl.querySelector("p");
          if (p && msg) p.textContent = msg;
        } else {
          emptyEl.innerHTML = `${svgIcon(iconCls)}<div class="chart-empty-state-title">${esc(msg || t("admin.state.empty", "Kayıt bulunamadı.", "No records found."))}</div>`;
        }
        emptyEl.classList.remove("hidden");
      }
    },
    mountOrUpdate(canvasId, configBuilder) {
      const canvas = byId(canvasId);
      if (!canvas || typeof Chart === "undefined" || typeof configBuilder !== "function") return null;
      const emptyEl = this._getEmptyEl(canvas, canvasId);
      if (emptyEl) emptyEl.classList.add("hidden");
      canvas.classList.remove("hidden");
      const prev = _chartRegistry.get(canvasId);
      if (prev?.instance) {
        try { prev.instance.destroy(); } catch (_) {}
      }
      if (canvas._chartInstance && canvas._chartInstance !== prev?.instance) {
        try { canvas._chartInstance.destroy(); } catch (_) {}
      }
      const cfg = configBuilder(this.palette());
      const inst = new Chart(canvas.getContext("2d"), cfg);
      canvas._chartInstance = inst;
      _chartRegistry.set(canvasId, { builder: configBuilder, instance: inst });
      return inst;
    },
    refreshTheme() {
      const entries = Array.from(_chartRegistry.entries());
      entries.forEach(([canvasId, entry]) => {
        const canvas = byId(canvasId);
        if (canvas && !canvas.classList.contains("hidden") && typeof entry?.builder === "function") {
          this.mountOrUpdate(canvasId, entry.builder);
        }
      });
    },
    resize(scope) {
      requestAnimationFrame(() => {
        _chartRegistry.forEach((entry, canvasId) => {
          const canvas = byId(canvasId);
          if (entry?.instance && canvas && (!scope || scope.contains(canvas))) {
            try { entry.instance.resize(); } catch (_) {}
          }
        });
        if (typeof Chart !== "undefined" && Chart.instances) {
          Object.values(Chart.instances).forEach((inst) => {
            if (!inst?.canvas) return;
            if (!scope || scope.contains(inst.canvas)) {
              try { inst.resize(); } catch (_) {}
            }
          });
        }
      });
    },
  };

  const canonRes = (r) => RESOURCE_ALIASES[String(r || "").trim().toLowerCase()] || String(r || "").trim().toLowerCase();
  function permMatches(grantedRaw, requiredRaw) {
    const g = String(grantedRaw || "").trim().toLowerCase().replace(/\./g, ":"), r = String(requiredRaw || "").trim().toLowerCase().replace(/\./g, ":");
    if (!g || !r) return false;
    if (g === "*" || g === r) return true;
    const [gResRaw, gAct = "*"] = g.includes(":") ? g.split(":", 2) : [g, "*"], [rResRaw, rAct = "*"] = r.includes(":") ? r.split(":", 2) : [r, "*"];
    const gRes = canonRes(gResRaw), rRes = canonRes(rResRaw);
    if (gRes === "users" && ["*", "manage", "verify"].includes(gAct) && rRes === "corporate_requests" && ["read", "view", "decide", "write"].includes(rAct)) return true;
    if (gRes === "corporate_requests" && ["*", "manage", "decide"].includes(gAct) && rRes === "users" && rAct === "verify") return true;
    if (gRes !== rRes) return false;
    const ga = gAct === "view" ? "read" : gAct, ra = rAct === "view" ? "read" : rAct;
    if (gAct === "*" || ga === "manage" || ga === ra) return true;
    if (ra === "read") return ["write", "publish", "upload"].includes(ga);
    if (ra === "write") return ga === "publish";
    return false;
  }

  function hasPerm(expr) {
    if (!expr) return true;
    const me = window.MSIAdmin?._me;
    if (!me) return false;
    if (me.is_wildcard) return true;
    const perms = Array.isArray(me.permissions) ? me.permissions : [], reqs = String(expr).split("|").map((s) => s.trim()).filter(Boolean);
    return !reqs.length || reqs.some((req) => perms.some((g) => permMatches(g, req)));
  }

  async function ensureMe() {
    if (window.MSIAdmin?._me) return window.MSIAdmin._me;
    if (!_mePromise) {
      _mePromise = apiRequest("/api/v1/admin/me").then((res) => {
        if (res.ok && res.data) { if (window.MSIAdmin) window.MSIAdmin._me = res.data; return res.data; }
        _mePromise = null; return null;
      });
    }
    return _mePromise;
  }

  async function loadSsbTags() {
    const ssbUrl = hasPerm("categories:read|contents:read|ads:read") ? "/api/v1/admin/ssb-tags" : "/api/v1/ssb-tags";
    const res = await apiRequest(ssbUrl);
    if (res.ok) state.ssbTags = unwrapList(res.payload, "ssb_tags");
    return state.ssbTags;
  }

  function resizeVisibleCharts(scope) {
    charts.resize(scope);
  }

  function updateGlobalKpiVisibility(tabId) {
    const strip = byId("global-kpi-strip");
    if (!strip) return;
    const visible = strip.querySelectorAll('.kpi-card:not([data-rbac-hidden="1"]):not(.hidden)').length;
    strip.dataset.kpiCount = String(visible);
    const hideTabs = (strip.dataset.hideTabs || "tab-14,tab-15,tab-17").split(",").map((s) => s.trim());
    strip.classList.toggle("hidden", visible === 0 || hideTabs.includes(tabId));
  }

  function switchSubtab(targetId, triggerBtn) {
    if (!targetId) return;
    const bar = triggerBtn?.closest(".subtab-bar") ||
      document.querySelector(`[data-subtab="${targetId}"], [data-subtab-target="${targetId}"]`)?.closest(".subtab-bar");
    const buttons = bar ? bar.querySelectorAll("[data-subtab], [data-subtab-target]") : [];
    let activePane = null;
    buttons.forEach((btn) => {
      const id = btn.getAttribute("data-subtab") || btn.getAttribute("data-subtab-target");
      const active = id === targetId;
      btn.classList.toggle("active", active);
      if (btn.hasAttribute("role")) btn.setAttribute("aria-selected", String(active));
      const pane = id ? byId(id) : null;
      if (pane) {
        pane.classList.toggle("active", active);
        pane.classList.toggle("hidden", !active);
        if (active) {
          activePane = pane;
          charts.resize(pane);
        }
      }
    });
    if (activePane && window.gsap && !prefersReducedMotion()) {
      window.gsap.killTweensOf(activePane);
      window.gsap.fromTo(
        activePane,
        { opacity: 0.78, y: 4 },
        { opacity: 1, y: 0, duration: 0.16, ease: "power2.out", clearProps: "transform,opacity" },
      );
    }
    window.dispatchEvent(new CustomEvent("msi:admin:subtab", { detail: { subtabId: targetId } }));
  }

  function switchLangTab(scopeEl, lang) {
    const root = scopeEl?.querySelectorAll ? scopeEl : document;
    let activePane = null;
    root.querySelectorAll("[data-lang-tab]").forEach((btn) => {
      const m = btn.getAttribute("data-lang-tab") === lang;
      btn.classList.toggle("active", m);
      btn.setAttribute("aria-selected", String(m));
    });
    root.querySelectorAll("[data-lang-pane], [data-lang-panel], [data-lang]:not([data-lang-tab])").forEach((p) => {
      const m = (p.getAttribute("data-lang-pane") || p.getAttribute("data-lang-panel") || p.getAttribute("data-lang")) === lang;
      p.classList.toggle("active", m);
      p.classList.toggle("hidden", !m);
      if (m) activePane = p;
    });
    if (activePane && window.gsap && !prefersReducedMotion()) {
      window.gsap.killTweensOf(activePane);
      window.gsap.fromTo(
        activePane,
        { opacity: 0.78, y: 4 },
        { opacity: 1, y: 0, duration: 0.16, ease: "power2.out", clearProps: "transform,opacity" },
      );
    }
  }

  function requireField(form, id, msg, lang) {
    const el = byId(id);
    if (!el || el.value.trim()) return true;
    if (lang) switchLangTab(form || el.closest("form, .admin-drawer"), lang);
    el.focus();
    showToast(msg, true);
    return false;
  }

  let _drawerPrevFocus = null;
  function openDrawer(drawerId) {
    const d = typeof drawerId === "string" ? byId(drawerId) : drawerId;
    if (!d) return;
    const parentTab = d.closest(".tab-pane");
    if (parentTab) {
      if (parentTab.getAttribute("data-rbac-hidden") === "1") return;
      if (parentTab.id && !parentTab.classList.contains("active")) {
        switchTab(parentTab.id);
      }
    }
    _drawerPrevFocus = document.activeElement;
    d.classList.remove("hidden");
    d.classList.add("open");
    d.setAttribute("aria-hidden", "false");
    if (window.gsap && !prefersReducedMotion()) {
      const panel = d.querySelector(".admin-drawer") || d;
      window.gsap.killTweensOf([d, panel]);
      window.gsap.fromTo(d, { opacity: 0.45 }, { opacity: 1, duration: 0.18, ease: "power2.out", clearProps: "opacity" });
      if (panel !== d) {
        window.gsap.fromTo(panel, { x: 28, opacity: 0.85 }, { x: 0, opacity: 1, duration: 0.24, ease: "power3.out", clearProps: "transform,opacity" });
      }
    }
    const firstCtrl = d.querySelector(".drawer-header button, input:not([type=hidden]):not([disabled]), select:not([disabled]), textarea:not([disabled])");
    if (firstCtrl && typeof firstCtrl.focus === "function") {
      requestAnimationFrame(() => { try { firstCtrl.focus({ preventScroll: true }); } catch (_) {} });
    }
  }

  function closeDrawer(drawerId) {
    const list = drawerId
      ? [typeof drawerId === "string" ? byId(drawerId) : drawerId]
      : Array.from(document.querySelectorAll(".admin-drawer-backdrop:not(.hidden), .drawer.open, .admin-drawer.open"));
    let closedAny = false;
    list.forEach((d) => {
      if (d) {
        if (window.gsap) {
          const panel = d.querySelector(".admin-drawer");
          window.gsap.killTweensOf([d, panel].filter(Boolean));
        }
        d.classList.remove("open");
        d.classList.add("hidden");
        d.setAttribute("aria-hidden", "true");
        closedAny = true;
      }
    });
    if (closedAny) {
      const prev = _drawerPrevFocus;
      _drawerPrevFocus = null;
      if (prev && typeof prev.focus === "function") {
        try { prev.focus({ preventScroll: true }); } catch (_) {}
      }
    }
  }

  function switchTab(tabId, opts = {}) {
    if (!tabId) return;
    document.querySelectorAll(".sidebar-link[data-tab], .tab-btn[data-tab]").forEach((item) => {
      const active = item.getAttribute("data-tab") === tabId;
      item.classList.toggle("active", active);
      if (item.hasAttribute("role")) item.setAttribute("aria-selected", String(active));
      if (active) {
        const lblEl = item.querySelector(".sidebar-link-label");
        const txt = lblEl ? lblEl.textContent.trim() : item.textContent.trim();
        if (txt && byId("page-title")) byId("page-title").textContent = txt;
        const grpLabel = item.closest(".sidebar-group")?.querySelector(".sidebar-group-label")?.textContent.trim();
        const grp = grpLabel || item.getAttribute("data-group") || item.closest(".sidebar-group")?.getAttribute("data-group-name") || "";
        if (grp && byId("page-group-label")) byId("page-group-label").textContent = grp;
      }
    });
    let activePane = null;
    document.querySelectorAll(".tab-pane").forEach((p) => {
      const isAct = p.id === tabId && p.getAttribute("data-rbac-hidden") !== "1";
      p.classList.toggle("active", isAct);
      if (isAct) p.classList.remove("hidden");
      if (isAct) activePane = p;
    });
    byId("admin-sidebar")?.classList.remove("open");
    byId("sidebar-overlay")?.classList.remove("open");
    byId("sidebar-toggle")?.setAttribute("aria-expanded", "false");
    updateGlobalKpiVisibility(tabId);
    if (activePane) {
      charts.resize(activePane);
      if (window.gsap && !prefersReducedMotion()) {
        window.gsap.killTweensOf(activePane);
        window.gsap.fromTo(
          activePane,
          { opacity: 0.72, y: 6 },
          { opacity: 1, y: 0, duration: 0.2, ease: "power2.out", clearProps: "transform,opacity" },
        );
      }
    }
    window.dispatchEvent(new CustomEvent("msi:admin:tab", { detail: { tabId, initial: !!opts.initial } }));
  }

  function roleDisplayName(roleCode) {
    if (!roleCode) return "";
    return t("admin.role." + roleCode, ROLE_LABELS_TR[roleCode] || roleCode, ROLE_LABELS_EN[roleCode] || roleCode);
  }

  function applyRoleWorkspaceBrief(roleCode) {
    const entry = ROLE_WORKSPACE_BRIEFS[roleCode];
    if (!entry) return;
    const pair = getLang() === "en" ? entry.en : entry.tr;
    if (byId("hero-role-brief")) byId("hero-role-brief").textContent = pair[0];
    if (byId("hero-role-subtitle")) byId("hero-role-subtitle").textContent = pair[1];
  }

  function applyRbacVisibility(me) {
    if (!me) return;
    if (me.email) {
      if (byId("operator-email-display")) byId("operator-email-display").textContent = me.email;
      if (byId("hero-operator-name")) byId("hero-operator-name").textContent = me.email;
    }
    if (me.role) {
      const roleTxt = roleDisplayName(me.role);
      if (byId("operator-role-display")) byId("operator-role-display").textContent = roleTxt;
      if (byId("hero-role-badge")) byId("hero-role-badge").textContent = roleTxt;
      applyRoleWorkspaceBrief(me.role);
    }
    document.querySelectorAll("[data-required-perm]").forEach((el) => {
      const ok = hasPerm(el.getAttribute("data-required-perm"));
      el.classList.toggle("hidden", !ok);
      if (!ok) el.setAttribute("data-rbac-hidden", "1"); else el.removeAttribute("data-rbac-hidden");
      if (el.classList.contains("sidebar-link") && el.hasAttribute("data-tab")) {
        const pane = byId(el.getAttribute("data-tab"));
        if (pane) {
          pane.classList.toggle("hidden", !ok);
          if (!ok) pane.setAttribute("data-rbac-hidden", "1"); else pane.removeAttribute("data-rbac-hidden");
        }
      }
    });
    document.querySelectorAll("[data-readonly-notice]").forEach((el) => {
      const canWrite = hasPerm(el.getAttribute("data-readonly-notice"));
      el.classList.toggle("hidden", canWrite);
    });
    document.querySelectorAll(".workspace-grid, .media-seo-grid").forEach((grid) => {
      const visibleChildren = Array.from(grid.children).filter(
        (ch) => !ch.classList.contains("hidden") && ch.getAttribute("data-rbac-hidden") !== "1",
      );
      grid.classList.toggle("single-col", visibleChildren.length <= 1);
    });
    document.querySelectorAll(".sidebar-group, .sidebar-studio-wrap").forEach((g) => {
      g.classList.toggle("hidden", g.querySelectorAll('.sidebar-link:not([data-rbac-hidden="1"])').length === 0);
    });
    if (byId("hero-module-count")) {
      const visibleModules = document.querySelectorAll('.sidebar-link[data-tab]:not([data-rbac-hidden="1"])').length;
      byId("hero-module-count").textContent = String(visibleModules);
    }
    document.querySelectorAll(".subtab-bar").forEach((bar) => {
      const cur = bar.querySelector('.subtab-btn.active:not([data-rbac-hidden="1"])');
      if (!cur) {
        const first = bar.querySelector('.subtab-btn:not([data-rbac-hidden="1"])');
        if (first) switchSubtab(first.getAttribute("data-subtab") || first.getAttribute("data-subtab-target"), first);
      }
    });
    const heroStrip = byId("hero-quick-actions");
    if (heroStrip) {
      const preferred = ROLE_HERO_PRIORITIES[me.role] || null;
      let shown = 0;
      heroStrip.querySelectorAll("button, a").forEach((el) => {
        if (el.getAttribute("data-rbac-hidden") === "1") return;
        const href = el.getAttribute("href") || "";
        const key = el.getAttribute("data-admin-action") ||
          el.getAttribute("data-tab") ||
          (href.endsWith("/editor") ? "editorial_studio" : (href.endsWith("/tasarim") ? "studio" : ""));
        const keep = preferred ? (preferred.includes(key) && shown < 4) : (shown < 4);
        el.classList.toggle("hidden", !keep);
        if (keep) shown++;
      });
    }
    const active = document.querySelector(".sidebar-link.active[data-tab], .tab-btn.active[data-tab]");
    const pref = (ROLE_LANDING_TABS[me.role] || []).map((tid) => document.querySelector(`.sidebar-link[data-tab="${tid}"]:not([data-rbac-hidden="1"])`)).find(Boolean);
    const first = document.querySelector('.sidebar-link[data-tab]:not([data-rbac-hidden="1"])');
    const target = pref || (active?.getAttribute("data-rbac-hidden") === "1" ? first : active || first);
    if (target) switchTab(target.getAttribute("data-tab"), { initial: true });
    if (window.gsap && !prefersReducedMotion()) {
      const entranceEls = Array.from(document.querySelectorAll("#command-hero-strip > *, #global-kpi-strip:not(.hidden) .kpi-card:not([data-rbac-hidden=\"1\"]):not(.hidden)"));
      if (entranceEls.length) {
        window.gsap.killTweensOf(entranceEls);
        window.gsap.fromTo(
          entranceEls,
          { y: 8, opacity: 0.85 },
          { y: 0, opacity: 1, duration: 0.24, stagger: 0.04, ease: "power2.out", clearProps: "transform,opacity" },
        );
      }
    }
  }

  async function logoutOperator() {
    await apiRequest("/api/v1/auth/logout", { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
    window.location.reload();
  }
  const registerLoader = (fn) => { if (typeof fn === "function") _loaders.push(fn); };
  const registerActions = (map) => { if (map && typeof map === "object") Object.assign(_actions, map); };
  async function refreshAllData() {
    await ensureMe();
    await Promise.allSettled([loadSsbTags(), ..._loaders.map((fn) => fn())]);
    window.dispatchEvent(new CustomEvent("msi:admin:refreshed"));
  }

  async function loadCustomFieldValues(containerId, entityType, entityId) {
    const box = byId(containerId);
    if (!box) return [];
    const seq = String(Number(box.dataset.cfSeq || "0") + 1);
    box.dataset.cfSeq = seq;
    box.innerHTML = "";
    const eid = String(entityId ?? "").trim() || "0";
    const r = await apiRequest(`/api/v1/admin/custom-field-values?entity_type=${encodeURIComponent(entityType)}&entity_id=${encodeURIComponent(eid)}`);
    if (box.dataset.cfSeq !== seq) return [];
    if (!r.ok) {
      box.innerHTML = `<div class="custom-fields-empty text-danger">${esc(r.errorMessage || t("admin.custom_fields.load_error", "Özel alanlar yüklenemedi.", "Failed to load custom fields."))}</div>`;
      return [];
    }
    const items = unwrapList(r.payload, "values");
    if (!items.length) {
      box.innerHTML = `<div class="custom-fields-empty">${esc(t("admin.custom_fields.none", "Bu varlık tipi için tanımlı özel alan yok.", "No custom fields defined for this entity type."))}</div>`;
      return [];
    }
    const lang = getLang();
    const heading = esc(t("admin.custom_fields.heading", "Özel Alanlar", "Custom Fields"));
    const itemsHtml = items.map((f) => {
      const fid = Number(f.field_id || f.id || 0);
      const labelText = (lang === "en" ? f.label_en : f.label_tr) || f.label_tr || f.field_key || "";
      const label = esc(labelText);
      const req = f.is_required ? " *" : "";
      const reqAttrs = f.is_required ? ' data-cf-required="1" required' : "";
      const ftype = String(f.field_type || "text").toLowerCase();
      const valText = f.value_text ?? "";
      const inputId = `${containerId}-cf-${fid}`;
      if (ftype === "textarea") {
        return `<div class="form-group custom-field-item custom-field-item-wide"><label class="form-label" for="${inputId}">${label}${req}</label><textarea id="${inputId}" class="form-textarea textarea-sm" rows="2" data-cf-field-id="${fid}" data-cf-type="textarea" data-cf-label="${label}"${reqAttrs}>${esc(valText)}</textarea></div>`;
      }
      if (ftype === "select") {
        let opts = [];
        if (Array.isArray(f.options_json)) {
          opts = f.options_json;
        } else if (typeof f.options_json === "string" && f.options_json.trim()) {
          const rawOpts = f.options_json.trim();
          try {
            const parsed = JSON.parse(rawOpts);
            if (Array.isArray(parsed)) opts = parsed;
            else opts = rawOpts.split(",").map((s) => s.trim()).filter(Boolean);
          } catch (_) {
            opts = rawOpts.split(",").map((s) => s.trim()).filter(Boolean);
          }
        }
        const optHtml = opts.map((o) => {
          const v = typeof o === "object" && o !== null ? String(o.value ?? o.code ?? "") : String(o);
          const lbl = typeof o === "object" && o !== null ? String((lang === "en" ? o.label_en : o.label_tr) || o.label || v) : String(o);
          return `<option value="${esc(v)}"${v === String(valText) ? " selected" : ""}>${esc(lbl)}</option>`;
        }).join("");
        return `<div class="form-group custom-field-item"><label class="form-label" for="${inputId}">${label}${req}</label><select id="${inputId}" class="form-select" data-cf-field-id="${fid}" data-cf-type="select" data-cf-label="${label}"${reqAttrs}><option value="">--</option>${optHtml}</select></div>`;
      }
      if (ftype === "checkbox") {
        const checked = valText === "true" || valText === "1" || valText === "on";
        return `<div class="form-group form-group-end custom-field-item"><label class="check-pill" for="${inputId}"><input type="checkbox" id="${inputId}" data-cf-field-id="${fid}" data-cf-type="checkbox" data-cf-label="${label}"${reqAttrs}${checked ? " checked" : ""}><span>${label}${req}</span></label></div>`;
      }
      const htmlType = ftype === "number" ? "number" : (ftype === "date" ? "date" : (ftype === "url" ? "url" : "text"));
      const extraAttrs = htmlType === "number" ? ' step="any"' : (htmlType === "url" ? ' placeholder="https://..."' : "");
      return `<div class="form-group custom-field-item"><label class="form-label" for="${inputId}">${label}${req}</label><input type="${htmlType}" id="${inputId}" class="form-control" data-cf-field-id="${fid}" data-cf-type="${htmlType}" data-cf-label="${label}"${extraAttrs}${reqAttrs} value="${esc(valText)}"></div>`;
    }).join("");
    box.innerHTML = `<div class="custom-fields-heading">${heading}</div><div class="custom-fields-grid">${itemsHtml}</div>`;
    return items;
  }

  function validateCustomFields(containerId) {
    const box = byId(containerId);
    if (!box) return true;
    const requiredEls = box.querySelectorAll('[data-cf-field-id][data-cf-required="1"]');
    for (const el of requiredEls) {
      const ftype = el.getAttribute("data-cf-type");
      const ok = ftype === "checkbox" ? !!el.checked : String(el.value ?? "").trim() !== "";
      if (!ok) {
        if (typeof el.focus === "function") el.focus();
        const label = el.getAttribute("data-cf-label") || el.id || "";
        showToast(
          t("admin.custom_fields.required_error", "\"${label}\" özel alanı zorunludur.", "Custom field \"${label}\" is required.").replace("${label}", label),
          true,
        );
        return false;
      }
    }
    return true;
  }

  async function saveCustomFieldValues(containerId, entityType, entityId) {
    const box = byId(containerId);
    const eid = String(entityId ?? "").trim();
    if (!box || !eid || eid === "0") return { ok: true, skipped: true };
    const inputs = box.querySelectorAll("[data-cf-field-id]");
    if (!inputs.length) return { ok: true, skipped: true };
    const values = Array.from(inputs).map((el) => {
      const fieldId = parseInt(el.getAttribute("data-cf-field-id") || "0", 10);
      const ftype = el.getAttribute("data-cf-type");
      const valueText = ftype === "checkbox" ? (el.checked ? "true" : "false") : String(el.value ?? "");
      return { field_id: fieldId, value_text: valueText };
    }).filter((v) => v.field_id > 0);
    if (!values.length) return { ok: true, skipped: true };
    return apiRequest("/api/v1/admin/custom-field-values", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ entity_type: entityType, entity_id: eid, values }),
    });
  }

  const MODAL_FOCUSABLE_SEL = 'a[href], button:not([disabled]), input:not([type="hidden"]):not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  function getVisibleFocusable(container) {
    if (!container) return [];
    return Array.from(container.querySelectorAll(MODAL_FOCUSABLE_SEL)).filter((el) => {
      if (el.closest("[hidden], .hidden")) return false;
      const st = window.getComputedStyle(el);
      if (st.display === "none" || st.visibility === "hidden") return false;
      return el.getClientRects().length > 0;
    });
  }

  function getActiveModalContainer() {
    const confirmModal = byId("ops-confirm-modal");
    if (confirmModal && !confirmModal.classList.contains("hidden")) {
      return confirmModal;
    }
    const openModals = Array.from(document.querySelectorAll('.modal-overlay[aria-modal="true"]:not(.hidden)'))
      .filter((el) => window.getComputedStyle(el).display !== "none");
    if (openModals.length) {
      return openModals[openModals.length - 1];
    }
    const openDrawers = Array.from(document.querySelectorAll('.admin-drawer-backdrop[aria-modal="true"]:not(.hidden), .drawer.open[aria-modal="true"], .admin-drawer.open[aria-modal="true"]'))
      .filter((el) => window.getComputedStyle(el).display !== "none");
    if (openDrawers.length) {
      return openDrawers[openDrawers.length - 1];
    }
    return null;
  }

  function bindCoreEvents() {
    byId("sidebar-toggle")?.addEventListener("click", () => {
      const open = !byId("admin-sidebar")?.classList.contains("open");
      byId("admin-sidebar")?.classList.toggle("open", open);
      byId("sidebar-overlay")?.classList.toggle("open", open);
      byId("sidebar-toggle")?.setAttribute("aria-expanded", String(open));
      if (open) {
        requestAnimationFrame(() => { try { byId("sidebar-close-btn")?.focus({ preventScroll: true }); } catch (_) {} });
      }
    });
    const closeMobileSidebar = (restoreFocus = false) => {
      const sb = byId("admin-sidebar");
      const wasOpen = Boolean(sb?.classList.contains("open"));
      sb?.classList.remove("open");
      byId("sidebar-overlay")?.classList.remove("open");
      byId("sidebar-toggle")?.setAttribute("aria-expanded", "false");
      if (wasOpen && restoreFocus) {
        try { byId("sidebar-toggle")?.focus({ preventScroll: true }); } catch (_) {}
      }
    };
    byId("sidebar-overlay")?.addEventListener("click", () => closeMobileSidebar(true));
    byId("sidebar-close-btn")?.addEventListener("click", () => closeMobileSidebar(true));

    byId("sidebar-quick-filter")?.addEventListener("input", (ev) => {
      const q = (ev.target.value || "").trim().toLowerCase();
      document.querySelectorAll(".sidebar-group").forEach((grp) => {
        let anyVisible = false;
        grp.querySelectorAll(".sidebar-link[data-tab]").forEach((btn) => {
          if (btn.getAttribute("data-rbac-hidden") === "1") return;
          const match = !q || btn.textContent.toLowerCase().includes(q) || (grp.getAttribute("data-group-name") || "").toLowerCase().includes(q);
          btn.classList.toggle("hidden", !match);
          if (match) anyVisible = true;
        });
        grp.classList.toggle("hidden", !anyVisible);
      });
    });

    bindGlow();

    byId("ops-confirm-cancel")?.addEventListener("click", () => closeConfirmModal(false));
    byId("ops-confirm-close")?.addEventListener("click", () => closeConfirmModal(false));
    byId("ops-confirm-accept")?.addEventListener("click", () => closeConfirmModal(true));

    document.addEventListener("keydown", (ev) => {
      if (ev.key === "Tab") {
        const activeModal = getActiveModalContainer();
        if (activeModal) {
          const focusables = getVisibleFocusable(activeModal);
          if (!focusables.length) {
            ev.preventDefault();
            return;
          }
          const first = focusables[0];
          const last = focusables[focusables.length - 1];
          const cur = document.activeElement;
          if (!activeModal.contains(cur)) {
            ev.preventDefault();
            (ev.shiftKey ? last : first).focus({ preventScroll: true });
            return;
          }
          if (ev.shiftKey) {
            if (cur === first || cur === activeModal) {
              ev.preventDefault();
              last.focus({ preventScroll: true });
            }
          } else if (cur === last) {
            ev.preventDefault();
            first.focus({ preventScroll: true });
          }
        }
        return;
      }
      if (ev.key === "Escape") {
        if (byId("ops-confirm-modal") && !byId("ops-confirm-modal").classList.contains("hidden")) {
          closeConfirmModal(false);
          return;
        }
        const openModalEl = ["totp-security-modal", "approval-modal", "ad-conflict-modal"]
          .map((id) => byId(id))
          .find((el) => el && !el.classList.contains("hidden"));
        if (openModalEl) {
          closeModal(openModalEl);
          return;
        }
        const openDrawerEl = document.querySelector(".admin-drawer-backdrop:not(.hidden), .drawer.open, .admin-drawer.open");
        if (openDrawerEl) {
          closeDrawer();
          return;
        }
        if (byId("admin-sidebar")?.classList.contains("open")) {
          closeMobileSidebar(true);
        }
      }
    });

    document.addEventListener("click", (ev) => {
      const targetEl = ev.target;
      if (targetEl.classList?.contains("admin-drawer-backdrop")) { closeDrawer(targetEl.id); return; }
      if (targetEl.id === "ops-confirm-modal") { closeConfirmModal(false); return; }
      if (targetEl.classList?.contains("modal-overlay")) { closeModal(targetEl); return; }

      const tab = targetEl.closest("[data-tab]");
      if (tab) { ev.preventDefault(); switchTab(tab.getAttribute("data-tab")); return; }

      const sub = targetEl.closest("[data-subtab], [data-subtab-target]");
      if (sub) { ev.preventDefault(); switchSubtab(sub.getAttribute("data-subtab") || sub.getAttribute("data-subtab-target"), sub); return; }

      const lang = targetEl.closest("[data-lang-tab]");
      if (lang) { ev.preventDefault(); switchLangTab(lang.closest("form, .admin-drawer, .modal-content") || document, lang.getAttribute("data-lang-tab")); return; }

      const el = targetEl.closest("[data-admin-action], [data-ext-action], [data-ext-op], [data-com-action]");
      if (!el) return;
      const act = el.getAttribute("data-admin-action") || el.getAttribute("data-ext-action") || el.getAttribute("data-ext-op") || el.getAttribute("data-com-action");
      if (act === "close-drawer") {
        closeDrawer(el.getAttribute("data-drawer-target") || el.closest(".admin-drawer-backdrop, .admin-drawer, .drawer")?.id);
      } else if (act === "toggle-theme") toggleTheme();
      else if (act === "refresh-all") { refreshAllData(); showToast(t("admin.toast.all_refreshed", "Tüm modül verileri güncellendi.", "All module data refreshed.")); }
      else if (act === "logout-operator") logoutOperator();
      else if (act === "paginate") {
        const cid = el.getAttribute("data-container"), p = parseInt(el.getAttribute("data-page") || "1", 10);
        if (cid && typeof paginationHandlers[cid] === "function" && p >= 1) paginationHandlers[cid](p);
      } else if (typeof _actions[act] === "function") _actions[act](el, ev);
    });

    window.addEventListener("msi:admin:theme", () => charts.refreshTheme());
  }

  document.addEventListener("DOMContentLoaded", async () => {
    if (!document.querySelector(".app-layout, .app-shell")) return;

    // Self-XSS console warning (dashboard only)
    if (typeof console !== "undefined" && console.warn) {
      console.warn(
        "%c🛑 DURUN! / STOP!",
        "color:#3b66c4;font-size:30px;font-weight:bold;line-height:1.4;padding:8px 0",
      );
      console.warn(
        "%cBu tarayıcı konsolu yalnızca yetkili UMSA / MSI Dergisi geliştiricileri içindir.\n\n" +
        "Eğer birisi size buraya herhangi bir kod veya komut kopyalayıp yapıştırmanızı söylediyse,\n" +
        "bu bir dolandırıcılık saldırısıdır (Self-XSS) ve hesabınızın ya da yönetim panelinizin\n" +
        "ele geçirilmesine yol açabilir.\n\n" +
        "🔒 MSI Podmin — Güvenlik Ekibi",
        "color:#556377;font-size:13px;line-height:1.7",
      );
    }

    initTheme();
    const ident = getOperatorIdentity();
    if (ident.email) {
      if (byId("operator-email-display")) byId("operator-email-display").textContent = ident.email;
      if (byId("hero-operator-name")) byId("hero-operator-name").textContent = ident.email;
    }
    if (ident.role) {
      const roleTxt = roleDisplayName(ident.role);
      if (byId("operator-role-display")) byId("operator-role-display").textContent = roleTxt;
      if (byId("hero-role-badge")) byId("hero-role-badge").textContent = roleTxt;
      applyRoleWorkspaceBrief(ident.role);
    }
    bindCoreEvents();
    const me = await ensureMe();
    if (me) applyRbacVisibility(me);
    await refreshAllData();
  });

  window.MSIAdmin = {
    t, getLang, locale, theme: window.MSIAdminTheme, charts, _me: null, state,
    switchTab, switchSubtab, switchLangTab, openDrawer, closeDrawer, openModal, closeModal,
    refreshAllData, registerLoader, registerActions, hasPerm, ensureMe,
    toggleAdminTheme: toggleTheme, loadSsbTags, renderChipSlice, svgIcon, confirmDialog,
    loadCustomFieldValues, validateCustomFields, saveCustomFieldValues,
    _ctx: {
      t, getLang, locale, ROLE_LABELS_TR, ROLE_LABELS_EN, state, paginationState, byId, val, setVal, esc,
      getCookieValue, getOperatorIdentity, unwrapData, unwrapList, extractMeta, apiRequest,
      safeFetchJson, uploadFile, showToast, renderTableState, renderPaginationControls, renderChipSlice,
      charts, theme: window.MSIAdminTheme, downloadBlob, hasPerm, ensureMe, applyRbacVisibility,
      switchTab, switchSubtab, switchLangTab, requireField, openDrawer, closeDrawer, openModal, closeModal,
      refreshAllData, registerLoader, registerActions, loadSsbTags, svgIcon, confirmDialog,
      loadCustomFieldValues, validateCustomFields, saveCustomFieldValues,
    },
  };
})();
