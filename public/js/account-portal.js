(function () {
  "use strict";

  if (window.__MSI_ACCOUNT_PORTAL_INIT__) return;
  window.__MSI_ACCOUNT_PORTAL_INIT__ = true;

  const $ = (sel, root = document) => root.querySelector(sel);
  const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

  const lang = (document.documentElement.lang || "tr").toLowerCase().startsWith("en")
    ? "en"
    : "tr";
  const isEn = lang === "en";

  let i18nMap = {};
  try {
    const i18nEl = $("#msi-i18n");
    if (i18nEl && i18nEl.textContent) {
      i18nMap = JSON.parse(i18nEl.textContent) || {};
    }
  } catch {
    i18nMap = {};
  }

  function t(key, trFallback, enFallback) {
    if (key && typeof i18nMap[key] === "string" && i18nMap[key].trim()) {
      return i18nMap[key];
    }
    return isEn ? enFallback : trFallback;
  }

  function getCookie(name) {
    try {
      const prefix = name + "=";
      const parts = document.cookie.split(";");
      for (const raw of parts) {
        const part = raw.trim();
        if (part.startsWith(prefix)) {
          return decodeURIComponent(part.slice(prefix.length));
        }
      }
    } catch {
      return "";
    }
    return "";
  }

  function unwrapPayload(raw) {
    if (raw && typeof raw === "object" && raw.data && typeof raw.data === "object") {
      return raw.data;
    }
    return raw || {};
  }

  function extractError(raw, fallback) {
    if (raw && typeof raw === "object") {
      if (typeof raw.message === "string" && raw.message.trim()) return raw.message;
      if (typeof raw.error === "string" && raw.error.trim()) return raw.error;
      if (raw.data && typeof raw.data.message === "string" && raw.data.message.trim()) {
        return raw.data.message;
      }
    }
    return fallback;
  }

  function extractMessage(raw, payload, fallback) {
    if (payload && typeof payload.message === "string" && payload.message.trim()) {
      return payload.message;
    }
    if (raw && typeof raw.message === "string" && raw.message.trim()) {
      return raw.message;
    }
    return fallback;
  }

  async function apiFetch(url, options = {}) {
    const headers = Object.assign({}, options.headers || {});
    if (options.body !== undefined && !headers["Content-Type"]) {
      headers["Content-Type"] = "application/json";
    }
    const csrf = getCookie("csrf_token");
    if (csrf && !headers["X-CSRF-Token"]) {
      headers["X-CSRF-Token"] = csrf;
    }
    const res = await fetch(url, {
      ...options,
      headers,
      credentials: "same-origin",
    });
    const json = await res.json().catch(() => ({}));
    return { res, json, payload: unwrapPayload(json) };
  }

  // ── Toast Region (Queued Stack, 4200ms, Semantic Variants) ───────────────
  const toastRegion = $(".ap-toast-region");
  const MAX_VISIBLE_TOASTS = 4;
  function toastIconSvg(variant) {
    switch (variant) {
      case "success":
        return '<svg class="ap-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m8.5 12.5 2.5 2.5 4.5-5"/></svg>';
      case "error":
        return '<svg class="ap-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m15 9-6 6M9 9l6 6"/></svg>';
      case "warning":
        return '<svg class="ap-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 3 2.5 20h19L12 3Z"/><path d="M12 9v5M12 17h.01"/></svg>';
      default:
        return '<svg class="ap-toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 10v6M12 7h.01"/></svg>';
    }
  }
  function showToast(message, variantOrIsError = "success") {
    if (!toastRegion || !message) return;
    let variant = "success";
    if (variantOrIsError === true) {
      variant = "error";
    } else if (variantOrIsError === false) {
      variant = "success";
    } else if (
      variantOrIsError === "success" ||
      variantOrIsError === "error" ||
      variantOrIsError === "warning" ||
      variantOrIsError === "info"
    ) {
      variant = variantOrIsError;
    }

    while (toastRegion.children.length >= MAX_VISIBLE_TOASTS) {
      toastRegion.firstElementChild?.remove();
    }

    const item = document.createElement("div");
    item.className = `ap-toast is-${variant}`;
    item.setAttribute("role", variant === "error" ? "alert" : "status");

    const iconWrap = document.createElement("span");
    iconWrap.innerHTML = toastIconSvg(variant);
    if (iconWrap.firstElementChild) {
      item.append(iconWrap.firstElementChild);
    }

    const msgEl = document.createElement("span");
    msgEl.className = "ap-toast-msg";
    msgEl.textContent = message;
    item.append(msgEl);

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "ap-toast-close";
    closeBtn.setAttribute("aria-label", t("portal.close", "Kapat", "Close"));
    closeBtn.textContent = "×";
    item.append(closeBtn);

    toastRegion.append(item);

    let dismissed = false;
    const dismiss = () => {
      if (dismissed) return;
      dismissed = true;
      window.clearTimeout(timerId);
      if (item.parentNode === toastRegion) {
        item.remove();
      }
    };
    const timerId = window.setTimeout(dismiss, 4200);
    closeBtn.addEventListener("click", dismiss);
  }

  const prefersReducedMotion = () =>
    Boolean(window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches);

  function animateEntrance(el, fromVars, durationMs = 200) {
    if (!el || prefersReducedMotion()) return;
    const fromOpacity = typeof fromVars.opacity === "number" ? fromVars.opacity : 1;
    const fromX = typeof fromVars.x === "number" ? fromVars.x : 0;
    const fromY = typeof fromVars.y === "number" ? fromVars.y : 0;
    const fromScale = typeof fromVars.scale === "number" ? fromVars.scale : 1;
    const isDialog = el.tagName === "DIALOG";

    if (!isDialog && window.gsap && typeof window.gsap.fromTo === "function") {
      window.gsap.fromTo(
        el,
        { opacity: fromOpacity, x: fromX, y: fromY, scale: fromScale },
        {
          opacity: 1,
          x: 0,
          y: 0,
          scale: 1,
          duration: durationMs / 1000,
          ease: "power2.out",
          clearProps: "opacity,transform",
        }
      );
      return;
    }

    if (typeof el.animate === "function") {
      const fromTransform = `translate3d(${fromX}px, ${fromY}px, 0) scale(${fromScale})`;
      try {
        el.animate(
          [
            { opacity: fromOpacity, transform: fromTransform },
            { opacity: 1, transform: "translate3d(0, 0, 0) scale(1)" },
          ],
          {
            duration: durationMs,
            easing: "cubic-bezier(0.22, 1, 0.36, 1)",
            fill: "none",
          }
        );
      } catch {}
    }
  }

  // ── Autonomous OS / Dynamic Clock Theme + Manual Toggle ─────────────────
  const themeBtn = $("#ap-theme-toggle");
  function resolveAutoTheme() {
    try {
      if (window.matchMedia) {
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
        if (window.matchMedia("(prefers-color-scheme: light)").matches) return "light";
      }
    } catch {}
    const hour = new Date().getHours();
    return hour >= 19 || hour < 7 ? "dark" : "light";
  }
  function getExplicitTheme() {
    try {
      const t1 = localStorage.getItem("msi_theme");
      if (t1 === "dark" || t1 === "light") return t1;
      const t2 = localStorage.getItem("msi_admin_theme");
      if (t2 === "dark" || t2 === "light") return t2;
    } catch {}
    const c = getCookie("msi_admin_theme");
    if (c === "dark" || c === "light") return c;
    return "";
  }
  function getActiveTheme() {
    const attr = document.documentElement.getAttribute("data-theme");
    return attr === "dark" || attr === "light" ? attr : resolveAutoTheme();
  }

  function syncThemeButton() {
    const current = getActiveTheme();
    document.documentElement.setAttribute("data-theme", current);
    if (!themeBtn) return;
    const isDark = current === "dark";
    themeBtn.setAttribute("aria-pressed", isDark ? "true" : "false");
    themeBtn.setAttribute(
      "aria-label",
      isDark
        ? t("portal.theme_to_light", "Açık temaya geç", "Switch to light theme")
        : t("portal.theme_to_dark", "Koyu temaya geç", "Switch to dark theme")
    );
    const labelEl = $("[data-theme-label]", themeBtn);
    if (labelEl) {
      labelEl.textContent = isDark
        ? t("portal.theme_light", "Açık", "Light")
        : t("portal.theme_dark", "Koyu", "Dark");
    }
  }
  document.documentElement.setAttribute("data-theme", getExplicitTheme() || resolveAutoTheme());
  syncThemeButton();

  const syncPortalAutoTheme = () => {
    if (getExplicitTheme()) return;
    const next = resolveAutoTheme();
    if (document.documentElement.getAttribute("data-theme") !== next) {
      document.documentElement.setAttribute("data-theme", next);
      syncThemeButton();
    }
  };
  try {
    if (window.matchMedia) {
      window.matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", syncPortalAutoTheme);
      window.matchMedia("(prefers-color-scheme: light)").addEventListener?.("change", syncPortalAutoTheme);
    }
  } catch {}
  window.setInterval(syncPortalAutoTheme, 60000);

  themeBtn?.addEventListener("click", () => {
    const current = getActiveTheme();
    const next = current === "dark" ? "light" : "dark";
    document.documentElement.setAttribute("data-theme", next);
    try {
      localStorage.setItem("msi_theme", next);
      localStorage.setItem("msi_admin_theme", next);
    } catch {}
    syncThemeButton();
  });

  window.addEventListener("storage", (e) => {
    if (e.key !== "msi_theme" && e.key !== "msi_admin_theme") return;
    if (e.newValue === "dark" || e.newValue === "light") {
      document.documentElement.setAttribute("data-theme", e.newValue);
      syncThemeButton();
    }
  });

  // ── Mobile Sidebar Drawer ───────────────────────────────────────────────
  const sidebar = $("#ap-sidebar");
  const sidebarToggle = $("#ap-sidebar-toggle");
  const sidebarClose = $("#ap-sidebar-close");
  const sidebarBackdrop = $("#ap-sidebar-backdrop");

  function openSidebar() {
    if (!sidebar) return;
    sidebar.classList.add("is-open", "open");
    if (sidebarBackdrop) {
      sidebarBackdrop.hidden = false;
      animateEntrance(sidebarBackdrop, { opacity: 0 }, 180);
    }
    if (!prefersReducedMotion()) {
      $$(".ap-nav-btn", sidebar).forEach((navBtn, idx) => {
        if (window.gsap && typeof window.gsap.fromTo === "function") {
          window.gsap.fromTo(
            navBtn,
            { opacity: 0.55, x: -8 },
            {
              opacity: 1,
              x: 0,
              duration: 0.18,
              delay: idx * 0.025,
              ease: "power2.out",
              clearProps: "opacity,transform",
            }
          );
        } else if (typeof navBtn.animate === "function") {
          try {
            navBtn.animate(
              [
                { opacity: 0.55, transform: "translate3d(-8px, 0, 0)" },
                { opacity: 1, transform: "translate3d(0, 0, 0)" },
              ],
              {
                duration: 180,
                delay: idx * 22,
                easing: "cubic-bezier(0.22, 1, 0.36, 1)",
                fill: "none",
              }
            );
          } catch {}
        }
      });
    }
    sidebarToggle?.setAttribute("aria-expanded", "true");
    void sidebar.offsetWidth;
    sidebarClose?.focus();
    requestAnimationFrame(() => {
      if (
        (sidebar.classList.contains("is-open") || sidebar.classList.contains("open")) &&
        document.activeElement !== sidebarClose
      ) {
        sidebarClose?.focus();
      }
    });
  }

  function closeSidebar(restoreFocus = false) {
    if (!sidebar) return;
    const wasOpen = sidebar.classList.contains("is-open") || sidebar.classList.contains("open");
    const hasBackdrop = Boolean(sidebarBackdrop && !sidebarBackdrop.hidden);
    if (!wasOpen && !hasBackdrop) return;
    sidebar.classList.remove("is-open", "open");
    if (sidebarBackdrop) sidebarBackdrop.hidden = true;
    sidebarToggle?.setAttribute("aria-expanded", "false");
    if (restoreFocus && wasOpen) sidebarToggle?.focus();
  }

  sidebarToggle?.addEventListener("click", () => {
    if (sidebar?.classList.contains("is-open") || sidebar?.classList.contains("open")) {
      closeSidebar(true);
    } else {
      openSidebar();
    }
  });
  sidebarClose?.addEventListener("click", () => closeSidebar(true));
  sidebarBackdrop?.addEventListener("click", () => closeSidebar(true));

  const desktopMq =
    typeof window.matchMedia === "function" ? window.matchMedia("(min-width: 960px)") : null;
  if (desktopMq) {
    const onDesktopTransition = (e) => {
      if (e.matches) {
        closeSidebar(false);
      }
    };
    if (typeof desktopMq.addEventListener === "function") {
      desktopMq.addEventListener("change", onDesktopTransition);
    } else if (typeof desktopMq.addListener === "function") {
      desktopMq.addListener(onDesktopTransition);
    }
    if (desktopMq.matches) {
      closeSidebar(false);
    }
  }

  // ── Multi-Section Tab Navigation ────────────────────────────────────────
  const VALID_TABS = ["overview", "edergi", "profile", "security", "notifications"];
  const tabButtons = $$("[data-ap-tab]");
  const panels = $$("[data-ap-panel]");
  let lastActiveTab = null;

  function normalizeTab(raw) {
    if (!raw) return "";
    const cleaned = String(raw)
      .trim()
      .replace(/^#/, "")
      .replace(/^sec-/, "")
      .replace(/^ap-panel-/, "")
      .toLowerCase();
    return VALID_TABS.includes(cleaned) ? cleaned : "";
  }

  function activateTab(tabName, updateUrl = true) {
    const target = normalizeTab(tabName) || "overview";
    const shouldAnimate = lastActiveTab !== null && lastActiveTab !== target;

    tabButtons.forEach((btn) => {
      const active = btn.dataset.apTab === target;
      btn.setAttribute("aria-selected", active ? "true" : "false");
      btn.classList.toggle("is-active", active);
      btn.tabIndex = active ? 0 : -1;
    });

    let activePanel = null;
    panels.forEach((panel) => {
      const active = panel.dataset.apPanel === target;
      panel.hidden = !active;
      if (active) activePanel = panel;
    });

    if (shouldAnimate && activePanel) {
      animateEntrance(activePanel, { opacity: 0.45, y: 8 }, 200);
    }
    lastActiveTab = target;

    try {
      sessionStorage.setItem("msi_ap_active_tab", target);
    } catch {}

    if (updateUrl && window.history && typeof window.history.replaceState === "function") {
      const url = new URL(window.location.href);
      if (target === "overview") {
        url.searchParams.delete("tab");
        url.hash = "";
      } else {
        url.searchParams.set("tab", target);
        url.hash = "#" + target;
      }
      window.history.replaceState(null, "", url.toString());
    }
  }

  tabButtons.forEach((btn) => {
    btn.addEventListener("click", () => {
      activateTab(btn.dataset.apTab, true);
      closeSidebar(false);
    });

    btn.addEventListener("keydown", (e) => {
      const list = btn.closest('[role="tablist"]');
      const groupTabs = list ? $$('[role="tab"][data-ap-tab]', list) : tabButtons;
      const idx = groupTabs.indexOf(btn);
      if (idx < 0 || !groupTabs.length) return;
      let nextIdx = -1;
      if (e.key === "ArrowDown" || e.key === "ArrowRight") {
        nextIdx = (idx + 1) % groupTabs.length;
      } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
        nextIdx = (idx - 1 + groupTabs.length) % groupTabs.length;
      } else if (e.key === "Home") {
        nextIdx = 0;
      } else if (e.key === "End") {
        nextIdx = groupTabs.length - 1;
      }
      if (nextIdx >= 0) {
        e.preventDefault();
        const nextBtn = groupTabs[nextIdx];
        activateTab(nextBtn.dataset.apTab, true);
        nextBtn.focus();
      }
    });
  });

  $$("[data-ap-jump]:not(.ap-mobile-tab)").forEach((btn) => {
    btn.addEventListener("click", () => {
      const target = btn.dataset.apJump;
      activateTab(target, true);
      const panel = $(`[data-ap-panel="${target}"]`);
      if (panel) {
        const rect = panel.getBoundingClientRect();
        if (rect.top < 72 || rect.top > window.innerHeight * 0.72) {
          panel.scrollIntoView({
            behavior: prefersReducedMotion() ? "auto" : "smooth",
            block: "start",
          });
        }
      }
    });
  });

  $$("[data-ap-lang-switch]").forEach((link) => {
    link.addEventListener("click", () => {
      try {
        sessionStorage.setItem("msi_ap_lang_switching", "1");
        sessionStorage.setItem("msi_ap_active_tab", lastActiveTab || "overview");
      } catch {}
    });
  });

  let storedLangTab = "";
  try {
    if (sessionStorage.getItem("msi_ap_lang_switching") === "1") {
      storedLangTab = normalizeTab(sessionStorage.getItem("msi_ap_active_tab"));
      sessionStorage.removeItem("msi_ap_lang_switching");
    }
  } catch {}

  const queryTab = normalizeTab(new URLSearchParams(window.location.search).get("tab"));
  const initialTab =
    normalizeTab(window.location.hash) ||
    queryTab ||
    storedLangTab ||
    normalizeTab(document.body.dataset.initialTab) ||
    "overview";
  activateTab(initialTab, Boolean(storedLangTab && !queryTab && !window.location.hash));

  window.addEventListener("hashchange", () => {
    const hashed = normalizeTab(window.location.hash);
    if (hashed) activateTab(hashed, false);
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      closeSidebar(true);
    }
  });

  // Dismissible Hint Alerts
  $$("[data-dismiss-alert]").forEach((btn) => {
    btn.addEventListener("click", () => {
      btn.closest(".ap-alert-banner")?.remove();
    });
  });

  // ── SSB Topic Chips Interactive Toolbar ─────────────────────────────────
  const ssbContainer = $("#account-ssb-tags");
  const ssbCountEl = $("#ap-ssb-count");
  function syncSsbChips() {
    if (!ssbContainer) return;
    const inputs = $$('input[name="ssb_interests"]', ssbContainer);
    let checkedCount = 0;
    inputs.forEach((inp) => {
      if (inp.checked) checkedCount += 1;
      inp.closest(".ssb-chip")?.classList.toggle("is-checked", inp.checked);
    });
    if (ssbCountEl) {
      ssbCountEl.textContent = isEn
        ? `${checkedCount} / ${inputs.length} selected`
        : `${checkedCount} / ${inputs.length} seçili`;
    }
  }
  ssbContainer?.addEventListener("change", syncSsbChips);
  $("[data-ssb-select-all]")?.addEventListener("click", () => {
    if (!ssbContainer) return;
    $$('input[name="ssb_interests"]', ssbContainer).forEach((i) => {
      i.checked = true;
    });
    syncSsbChips();
  });
  $("[data-ssb-clear-all]")?.addEventListener("click", () => {
    if (!ssbContainer) return;
    $$('input[name="ssb_interests"]', ssbContainer).forEach((i) => {
      i.checked = false;
    });
    syncSsbChips();
  });
  syncSsbChips();

  // ── Resend Email Verification ───────────────────────────────────────────
  $("#account-resend-verify")?.addEventListener("click", async (e) => {
    const btn = e.currentTarget;
    const fb = $("#account-verify-feedback");
    const email = btn.dataset.userEmail || "";
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = t("portal.sending", "Gönderiliyor…", "Sending…");
    try {
      const { res, json, payload } = await apiFetch("/api/v1/auth/resend-verification", {
        method: "POST",
        body: JSON.stringify({ email }),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t(
              "portal.verify_err",
              "Doğrulama e-postası gönderilemedi.",
              "Verification email could not be sent."
            )
          )
        );
      }
      const msg = extractMessage(
        json,
        payload,
        t(
          "portal.verify_ok",
          "Doğrulama e-postası adresinize tekrar gönderildi.",
          "Verification email has been resent."
        )
      );
      if (fb) fb.textContent = msg;
      showToast(msg);
    } catch (err) {
      fb?.classList.add("error");
      const msg = err?.message || t("portal.conn_err", "Bağlantı hatası.", "Connection error.");
      if (fb) fb.textContent = msg;
      showToast(msg, true);
    } finally {
      btn.disabled = false;
    }
  });

  // ── Profile & SSB Interests Update ──────────────────────────────────────
  $("#account-profile-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $('[type="submit"]', form);
    const fb = $(".form-feedback", form);
    const fd = new FormData(form);
    const reqBody = {
      full_name: String(fd.get("full_name") || "").trim(),
      job_title: String(fd.get("job_title") || "").trim(),
      ssb_interests: $$('input[name="ssb_interests"]:checked', form).map((i) => i.value),
      email_consent: !!form.elements.email_consent?.checked,
      marketing_emails_opted_in: !!form.elements.email_consent?.checked,
    };
    const orgNameEl = form.elements.organization_name;
    if (orgNameEl && !orgNameEl.disabled) {
      reqBody.organization_name = String(fd.get("organization_name") || "").trim();
    }
    const orgCatEl = form.elements.organization_category;
    if (orgCatEl && !orgCatEl.disabled) {
      reqBody.organization_category = String(fd.get("organization_category") || "").trim();
    }

    if (btn) btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = t("portal.saving", "Kaydediliyor…", "Saving…");
    try {
      const { res, json, payload } = await apiFetch("/api/v1/account/profile", {
        method: "POST",
        body: JSON.stringify(reqBody),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t("portal.profile_err", "Profil güncellenemedi.", "Profile could not be updated.")
          )
        );
      }
      const msg = extractMessage(
        json,
        payload,
        t(
          "portal.profile_ok",
          "Profiliniz ve SSB ilgi alanlarınız güncellendi.",
          "Your profile and SSB interests have been updated."
        )
      );
      if (fb) fb.textContent = msg;
      showToast(msg);
      window.setTimeout(() => {
        window.location.hash = "profile";
        window.location.reload();
      }, 850);
    } catch (err) {
      fb?.classList.add("error");
      const msg = err?.message || t("portal.conn_err", "Bağlantı hatası.", "Connection error.");
      if (fb) fb.textContent = msg;
      showToast(msg, true);
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  // ── 2FA TOTP Setup, Verify, Backup Codes & Disable ──────────────────────
  function renderBackupCodes(codes) {
    const box = $("#totp-backup-codes-box");
    const list = $("#totp-backup-codes-list");
    if (!box || !list || !Array.isArray(codes) || !codes.length) return false;
    list.replaceChildren();
    codes.forEach((c) => {
      const codeEl = document.createElement("code");
      codeEl.textContent = String(c);
      list.append(codeEl);
    });
    box.hidden = false;
    return true;
  }

  $("#account-totp-setup-btn")?.addEventListener("click", async (e) => {
    const btn = e.currentTarget;
    const fb = $("#account-totp-setup-feedback");
    const panel = $("#account-totp-setup-panel");
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) {
      fb.textContent = t(
        "portal.totp_gen",
        "Güvenli 2FA anahtarı üretiliyor…",
        "Generating secure 2FA secret…"
      );
    }
    try {
      const { res, json, payload } = await apiFetch("/api/v1/auth/totp/setup", {
        method: "POST",
        body: JSON.stringify({}),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t("portal.totp_setup_err", "2FA kurulumu başlatılamadı.", "2FA setup could not be started.")
          )
        );
      }
      const qrRaw =
        payload.qr_svg || payload.qr_code_svg || payload.qr_code || payload.svg || "";
      const secret =
        payload.secret || payload.totp_secret || payload.manual_entry_key || "";
      const qrImg = $("#account-totp-qr");
      if (qrImg && qrRaw) {
        qrImg.src = qrRaw.startsWith("data:image/")
          ? qrRaw
          : "data:image/svg+xml;utf8," + encodeURIComponent(qrRaw);
      }
      const secretEl = $("#account-totp-secret");
      if (secretEl) secretEl.textContent = secret;
      const secretInput = $("#account-totp-secret-input");
      if (secretInput) secretInput.value = secret;
      if (Array.isArray(payload.backup_codes) && payload.backup_codes.length) {
        renderBackupCodes(payload.backup_codes);
      }
      if (panel) panel.hidden = false;
      btn.hidden = true;
      if (fb) fb.textContent = "";
    } catch (err) {
      fb?.classList.add("error");
      const msg = err?.message || t("portal.conn_err", "Bağlantı hatası.", "Connection error.");
      if (fb) fb.textContent = msg;
      btn.disabled = false;
    }
  });

  $("#account-totp-verify-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $('[type="submit"]', form);
    const fb = $(".form-feedback", form);
    const code = String(form.elements.code?.value || "").trim();
    const secret = String(form.elements.secret?.value || "").trim();
    if (btn) btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = t("portal.verifying", "Doğrulanıyor…", "Verifying…");
    try {
      const { res, json, payload } = await apiFetch("/api/v1/auth/totp/verify", {
        method: "POST",
        body: JSON.stringify(secret ? { code, secret } : { code }),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t(
              "portal.totp_code_err",
              "Doğrulama kodu hatalı. Lütfen tekrar deneyin.",
              "Invalid verification code. Please try again."
            )
          )
        );
      }
      const msg = extractMessage(
        json,
        payload,
        t(
          "portal.totp_ok",
          "İki aşamalı doğrulama (2FA) etkinleştirildi.",
          "Two-factor authentication (2FA) is now enabled."
        )
      );
      if (fb) fb.textContent = msg;
      showToast(msg);
      const codes = payload.backup_codes || payload.recovery_codes;
      if (renderBackupCodes(codes)) {
        form.hidden = true;
      } else {
        window.setTimeout(() => {
          window.location.hash = "security";
          window.location.reload();
        }, 900);
      }
    } catch (err) {
      fb?.classList.add("error");
      const msg = err?.message || t("portal.conn_err", "Bağlantı hatası.", "Connection error.");
      if (fb) fb.textContent = msg;
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  $("[data-copy-backup-codes]")?.addEventListener("click", async () => {
    const codes = $$("#totp-backup-codes-list code")
      .map((el) => el.textContent.trim())
      .filter(Boolean)
      .join("\n");
    if (!codes) return;
    try {
      await navigator.clipboard.writeText(codes);
      showToast(t("portal.codes_copied", "Kurtarma kodları kopyalandı.", "Recovery codes copied."));
    } catch {
      showToast(
        t("portal.copy_manual", "Kodları manuel olarak kopyalayın.", "Please copy the codes manually."),
        true
      );
    }
  });

  $("[data-reload-page]")?.addEventListener("click", () => {
    window.location.hash = "security";
    window.location.reload();
  });

  $("#account-totp-disable-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $('[type="submit"]', form);
    const fb = $(".form-feedback", form);
    const password = String(form.elements.password?.value || "");
    const code = String(form.elements.code?.value || "").trim();
    if (btn) btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = t("portal.processing", "İşleniyor…", "Processing…");
    try {
      const { res, json, payload } = await apiFetch("/api/v1/auth/totp/disable", {
        method: "POST",
        body: JSON.stringify({ password, code }),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t("portal.totp_dis_err", "2FA devre dışı bırakılamadı.", "2FA could not be disabled.")
          )
        );
      }
      const msg = extractMessage(
        json,
        payload,
        t(
          "portal.totp_dis_ok",
          "İki aşamalı doğrulama devre dışı bırakıldı.",
          "Two-factor authentication has been disabled."
        )
      );
      if (fb) fb.textContent = msg;
      showToast(msg);
      window.setTimeout(() => {
        window.location.hash = "security";
        window.location.reload();
      }, 850);
    } catch (err) {
      fb?.classList.add("error");
      const msg = err?.message || t("portal.conn_err", "Bağlantı hatası.", "Connection error.");
      if (fb) fb.textContent = msg;
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  // ── KVKK Account Erasure with Accessible Custom Confirmation Modal ──────
  const deleteForm = $("#account-delete-form");
  const deleteDialog = $("#account-delete-confirm-dialog");
  const deleteConfirmBtn = $("#account-delete-confirm-btn");
  const deleteCancelBtn = $("#account-delete-cancel-btn");
  const deleteCloseBtn = $("#account-delete-close-btn");
  let pendingDeletePassword = "";
  let deleteInFlight = false;
  let deletionSucceeded = false;

  async function executeAccountDeletion() {
    if (!deleteForm || deleteInFlight) return;
    deleteInFlight = true;
    const submitBtn = $('[type="submit"]', deleteForm);
    const fb = $(".form-feedback", deleteForm);
    if (deleteConfirmBtn) deleteConfirmBtn.disabled = true;
    if (deleteCancelBtn) deleteCancelBtn.disabled = true;
    if (deleteCloseBtn) deleteCloseBtn.disabled = true;
    if (submitBtn) submitBtn.disabled = true;
    deleteForm.setAttribute("aria-busy", "true");
    fb?.classList.remove("error");
    if (fb) {
      fb.textContent = t(
        "portal.deleting",
        "Hesabınız kalıcı olarak siliniyor…",
        "Permanently deleting your account…"
      );
    }
    try {
      const { res, json, payload } = await apiFetch("/api/v1/account/delete", {
        method: "POST",
        body: JSON.stringify(pendingDeletePassword ? { password: pendingDeletePassword } : {}),
      });
      if (!res.ok) {
        throw new Error(
          extractError(
            json,
            t(
              "portal.delete_err",
              "Hesap silme işlemi tamamlanamadı. Şifrenizi kontrol edin.",
              "Account deletion failed. Please check your password."
            )
          )
        );
      }
      deletionSucceeded = true;
      const msg = extractMessage(
        json,
        payload,
        t(
          "portal.delete_ok",
          "Hesabınız KVKK kapsamında kalıcı olarak silindi.",
          "Your account has been permanently deleted."
        )
      );
      if (fb) fb.textContent = msg;
      try {
        localStorage.removeItem("msi_active_user");
        // SECURITY FIX: Removed legacy msi_session_token cleanup — sessions are HttpOnly cookies only
      } catch {}
      if (deleteDialog?.open) deleteDialog.close();
      showToast(msg, "success");
      window.setTimeout(() => {
        window.location.href = isEn ? "/en" : "/";
      }, 900);
    } catch (err) {
      if (deleteDialog?.open) deleteDialog.close();
      fb?.classList.add("error");
      const msg =
        err?.message || t("portal.conn_err", "Bağlantı hatası oluştu.", "A connection error occurred.");
      if (fb) fb.textContent = msg;
      showToast(msg, "error");
    } finally {
      deleteInFlight = false;
      if (deleteConfirmBtn) deleteConfirmBtn.disabled = false;
      if (deleteCancelBtn) deleteCancelBtn.disabled = false;
      if (deleteCloseBtn) deleteCloseBtn.disabled = false;
      if (submitBtn) submitBtn.disabled = false;
      deleteForm.removeAttribute("aria-busy");
    }
  }

  deleteForm?.addEventListener("submit", (e) => {
    e.preventDefault();
    const fb = $(".form-feedback", deleteForm);
    const confirmInput = deleteForm.elements.confirm_erasure;
    const password = String(deleteForm.elements.password?.value || "");

    if (confirmInput && !confirmInput.checked) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent = t(
          "portal.confirm_erasure_err",
          "Kalıcı hesap silme işlemini onaylamanız gerekmektedir.",
          "Please confirm permanent account erasure."
        );
      }
      confirmInput.focus();
      return;
    }
    if (!password) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent = t(
          "portal.pw_required",
          "Lütfen mevcut şifrenizi girin.",
          "Please enter your current password."
        );
      }
      deleteForm.elements.password?.focus();
      return;
    }

    fb?.classList.remove("error");
    if (fb) fb.textContent = "";
    pendingDeletePassword = password;
    deletionSucceeded = false;

    if (deleteDialog && typeof deleteDialog.showModal === "function") {
      deleteDialog.showModal();
      animateEntrance(deleteDialog, { opacity: 0.2, y: 8, scale: 0.98 }, 200);
      deleteCancelBtn?.focus();
    } else {
      executeAccountDeletion();
    }
  });

  function closeDeleteDialog() {
    if (deleteInFlight) return;
    if (deleteDialog?.open) deleteDialog.close();
  }

  deleteCancelBtn?.addEventListener("click", closeDeleteDialog);
  deleteCloseBtn?.addEventListener("click", closeDeleteDialog);

  deleteDialog?.addEventListener("click", (e) => {
    if (e.target !== deleteDialog || deleteInFlight) return;
    const r = deleteDialog.getBoundingClientRect();
    if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) {
      closeDeleteDialog();
    }
  });

  deleteDialog?.addEventListener("cancel", (e) => {
    if (deleteInFlight) {
      e.preventDefault();
      return;
    }
    pendingDeletePassword = "";
  });

  deleteDialog?.addEventListener("close", () => {
    pendingDeletePassword = "";
    if (!deletionSucceeded) {
      $('[type="submit"]', deleteForm)?.focus();
    }
  });

  deleteConfirmBtn?.addEventListener("click", () => {
    executeAccountDeletion();
  });

  // ── Notifications Mark-Read, Bounded Batch & Header localStorage Sync ───
  const READ_STORAGE_KEY = "msi_read_notifications";
  function syncReadStorage(ids) {
    if (!Array.isArray(ids) || !ids.length) return;
    try {
      const existing = JSON.parse(localStorage.getItem(READ_STORAGE_KEY) || "[]");
      const merged = new Set(Array.isArray(existing) ? existing.map(String) : []);
      ids.forEach((id) => {
        if (id !== undefined && id !== null && String(id).trim()) {
          merged.add(String(id).trim());
        }
      });
      localStorage.setItem(READ_STORAGE_KEY, JSON.stringify(Array.from(merged)));
    } catch {}
  }

  // Initial sync of DB-read notifications into Surface 1 localStorage
  const initialReadIds = $$('.notification-item[data-is-read="true"]')
    .map((el) => el.dataset.notificationId)
    .filter(Boolean);
  if (initialReadIds.length) {
    syncReadStorage(initialReadIds);
  }

  function refreshNotificationCounters() {
    const unreadItems = $$('.notification-item[data-is-read="false"]');
    const count = unreadItems.length;
    $$("#ap-notif-badge, [data-notif-badge]").forEach((badge) => {
      badge.textContent = String(count);
      badge.dataset.unreadCount = String(count);
      badge.hidden = count === 0;
    });
    const kpiCount = $("#ap-kpi-unread");
    if (kpiCount) {
      kpiCount.textContent = String(count);
    }
    const markAllBtn = $("[data-mark-all-read]");
    if (markAllBtn) {
      markAllBtn.hidden = count === 0;
    }
  }

  function applyItemReadUi(itemEl) {
    if (!itemEl) return;
    itemEl.dataset.isRead = "true";
    itemEl.classList.remove("is-unread");
    itemEl.classList.add("is-read");
    const statusBadge = $("[data-notif-state]", itemEl);
    if (statusBadge) {
      statusBadge.className = "status-badge is-muted";
      statusBadge.textContent = t("portal.notif_status_read", "Okundu", "Read");
    }
    $("[data-mark-read]", itemEl)?.remove();
  }

  $$("[data-mark-read]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const id = btn.dataset.markRead;
      if (!id) return;
      btn.disabled = true;
      try {
        const { res, payload } = await apiFetch(`/api/v1/notifications/${encodeURIComponent(id)}/read`, {
          method: "POST",
        });
        if (!res.ok || payload?.marked_read === false) throw new Error("mark_read_failed");
        applyItemReadUi(btn.closest(".notification-item"));
        syncReadStorage([id]);
        refreshNotificationCounters();
        showToast(
          t("portal.notif_read", "Bildirim okundu olarak işaretlendi.", "Notification marked as read."),
          "success"
        );
      } catch {
        btn.disabled = false;
        showToast(
          t("portal.notif_err", "Bildirim güncellenemedi.", "Could not update notification."),
          "error"
        );
      }
    });
  });

  $("[data-mark-all-read]")?.addEventListener("click", async (e) => {
    const btn = e.currentTarget;
    const unreadButtons = $$('.notification-item[data-is-read="false"] [data-mark-read]');
    if (!unreadButtons.length) return;
    const originalLabel = btn.textContent;
    btn.disabled = true;
    btn.setAttribute("aria-busy", "true");
    btn.textContent = t("portal.processing", "İşleniyor…", "Processing…");
    unreadButtons.forEach((b) => {
      b.disabled = true;
    });

    const succeededIds = [];
    let failedCount = 0;
    const queue = unreadButtons.slice();
    const concurrency = Math.min(3, queue.length);

    try {
      const workers = Array.from({ length: concurrency }, async () => {
        while (queue.length > 0) {
          const itemBtn = queue.shift();
          if (!itemBtn) break;
          const id = itemBtn.dataset.markRead;
          if (!id) continue;
          try {
            const { res, payload } = await apiFetch(
              `/api/v1/notifications/${encodeURIComponent(id)}/read`,
              { method: "POST" }
            );
            if (res.ok && payload?.marked_read !== false) {
              succeededIds.push(id);
              applyItemReadUi(itemBtn.closest(".notification-item"));
            } else {
              failedCount += 1;
            }
          } catch {
            failedCount += 1;
          }
        }
      });
      await Promise.all(workers);

      if (succeededIds.length > 0) {
        syncReadStorage(succeededIds);
        refreshNotificationCounters();
      }
      $$('.notification-item[data-is-read="false"] [data-mark-read]').forEach((b) => {
        b.disabled = false;
      });

      if (succeededIds.length > 0 && failedCount === 0) {
        showToast(
          t("portal.notif_all_read", "Tüm bildirimler okundu işaretlendi.", "All notifications marked as read."),
          "success"
        );
      } else if (succeededIds.length > 0 && failedCount > 0) {
        showToast(
          t(
            "portal.notif_partial",
            "Bazı bildirimler güncellenemedi. Lütfen tekrar deneyin.",
            "Some notifications could not be updated. Please try again."
          ),
          "warning"
        );
      } else {
        showToast(
          t("portal.notif_err", "Bildirimler güncellenemedi.", "Could not update notifications."),
          "error"
        );
      }
    } finally {
      btn.disabled = false;
      btn.removeAttribute("aria-busy");
      btn.textContent = originalLabel;
    }
  });

  // ── Quick Sign-Out ──────────────────────────────────────────────────────
  $$("[data-logout]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      btn.disabled = true;
      try {
        const { res } = await apiFetch("/api/v1/auth/logout", { method: "POST" });
        if (!res.ok) throw new Error("logout_failed");
        try {
          localStorage.removeItem("msi_active_user");
          // SECURITY FIX: Removed legacy msi_session_token cleanup — sessions are HttpOnly cookies only
        } catch {}
        window.location.href = "/hesabim";
      } catch {
        btn.disabled = false;
        showToast(
          t("portal.logout_err", "Çıkış yapılamadı. Tekrar deneyin.", "Sign-out failed. Please try again."),
          true
        );
      }
    });
  });
})();
