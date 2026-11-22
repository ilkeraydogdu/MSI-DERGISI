/* Shared interaction, motion choreography, live 3D dossier mockup, and commerce/auth layer. */
(() => {
  "use strict";
  const boot = JSON.parse(
    document.getElementById("msi-bootstrap")?.textContent || "{}",
  );
  try {
    const i18nEl = document.getElementById("msi-i18n");
    if (i18nEl) window.__MSI_I18N__ = JSON.parse(i18nEl.textContent || "{}");
  } catch {}
  const $ = (s, root = document) => root.querySelector(s);
  const $$ = (s, root = document) => [...root.querySelectorAll(s)];
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const inIframe = (() => {
    try {
      return window.self !== window.top;
    } catch {
      return true;
    }
  })();

  const storage = {
    get: (k) => {
      try {
        return localStorage.getItem(k);
      } catch {
        return null;
      }
    },
    set: (k, v) => {
      try {
        localStorage.setItem(k, v);
      } catch {}
    },
  };
  const text = (tr, en) => (boot.lang === "en" ? en : tr);
  const localized = (obj, field) =>
    obj?.[`${field}_${boot.lang || "tr"}`] || obj?.[`${field}_tr`] || "";
  const unwrapPayload = (data) =>
    data &&
    typeof data === "object" &&
    data.data &&
    typeof data.data === "object"
      ? data.data
      : data || {};
  const extractError = (data, fallback) =>
    data?.message ||
    (typeof data?.error === "string" ? data.error : data?.error?.message) ||
    data?.data?.message ||
    data?.data?.error ||
    fallback;
  const extractMessage = (data, payload, fallback) =>
    payload?.message ||
    payload?.message_tr ||
    data?.message ||
    data?.message_tr ||
    fallback;
  const getCookie = (name) => {
    try {
      const prefix = name + "=";
      for (const raw of document.cookie.split(";")) {
        const part = raw.trim();
        if (part.startsWith(prefix)) {
          return decodeURIComponent(part.slice(prefix.length));
        }
      }
    } catch {}
    return "";
  };
  const csrfHeaders = (extra = {}) => {
    const out = Object.assign({}, extra);
    const tok = getCookie("csrf_token");
    if (tok && !out["X-CSRF-Token"]) {
      out["X-CSRF-Token"] = tok;
    }
    return out;
  };
  const safeRelativePath = (raw) => {
    if (typeof raw !== "string") return "";
    if (/[\u0000-\u001f\u007f]/.test(raw)) return "";
    const trimmed = raw.trim();
    if (!trimmed) return "";
    if (
      !trimmed.startsWith("/") ||
      trimmed.startsWith("//") ||
      trimmed.startsWith("/\\")
    ) {
      return "";
    }
    const baseOrigin =
      typeof window !== "undefined" &&
      window.location &&
      window.location.origin &&
      window.location.origin !== "null"
        ? window.location.origin
        : "https://localhost";
    try {
      const parsed = new URL(trimmed, baseOrigin);
      return parsed.origin === baseOrigin ? trimmed : "";
    } catch {
      return "";
    }
  };
  const safeIframeUrl = (url) => {
    if (typeof url !== "string") return "";
    if (/[\u0000-\u001f\u007f]/.test(url)) return "";
    const trimmed = url.trim();
    if (!trimmed) return "";
    if (trimmed.startsWith("/")) {
      return safeRelativePath(trimmed);
    }
    try {
      const parsed = new URL(trimmed);
      return parsed.protocol === "https:" ? trimmed : "";
    } catch {
      return "";
    }
  };

  let lenis = null;
  if (window.Lenis && !reducedMotion && !inIframe) {
    try {
      lenis = new window.Lenis({
        duration: 1.05,
        smoothWheel: true,
        wheelMultiplier: 0.95,
      });
      window.__msiLenis = lenis;
      if (window.gsap && window.ScrollTrigger) {
        window.gsap.registerPlugin(window.ScrollTrigger);
        lenis.on("scroll", window.ScrollTrigger.update);
        window.gsap.ticker.add((time) => {
          lenis.raf(time * 1000);
        });
        window.gsap.ticker.lagSmoothing(0);
      } else {
        const raf = (time) => {
          lenis.raf(time);
          requestAnimationFrame(raf);
        };
        requestAnimationFrame(raf);
      }
    } catch {
      lenis = null;
    }
  } else if (window.gsap && window.ScrollTrigger) {
    window.gsap.registerPlugin(window.ScrollTrigger);
  }

  function syncScrollLock() {
    if (!lenis) return;
    if ($("dialog[open]") || $("#cart-drawer")?.classList.contains("is-open")) lenis.stop();
    else lenis.start();
  }

  function createToastIcon(type) {
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "2");
    svg.setAttribute("width", "18");
    svg.setAttribute("height", "18");
    svg.setAttribute("aria-hidden", "true");
    const addPath = (d) => {
      const p = document.createElementNS(ns, "path");
      p.setAttribute("d", d);
      svg.appendChild(p);
    };
    const addCircle = (cx, cy, r) => {
      const c = document.createElementNS(ns, "circle");
      c.setAttribute("cx", String(cx));
      c.setAttribute("cy", String(cy));
      c.setAttribute("r", String(r));
      svg.appendChild(c);
    };
    if (type === "success") {
      addCircle(12, 12, 9);
      addPath("m9 12 2 2 4-4");
    } else if (type === "error") {
      addCircle(12, 12, 9);
      addPath("m15 9-6 6M9 9l6 6");
    } else if (type === "warning") {
      addPath("M10.3 3.9 1.8 18.2A2 2 0 0 0 3.5 21h17a2 2 0 0 0 1.7-2.8L13.7 3.9a2 2 0 0 0-3.4 0Z");
      addPath("M12 9v4M12 17h.01");
    } else {
      addCircle(12, 12, 9);
      addPath("M12 16v-4M12 8h.01");
    }
    return svg;
  }

  function toast(message, options = {}) {
    const region = $(".toast-region");
    if (!region) return;
    let type = "info";
    let duration = 4200;
    if (typeof options === "string") {
      type = options;
    } else if (options === true) {
      type = "error";
    } else if (options && typeof options === "object") {
      if (options.type) type = options.type;
      if (Number.isFinite(options.duration) && options.duration > 0) {
        duration = options.duration;
      }
    }
    if (!["success", "error", "warning", "info"].includes(type)) type = "info";

    while (region.children.length >= 4) {
      region.firstElementChild?.remove();
    }

    const el = document.createElement("div");
    el.className = `toast-message is-${type}`;
    el.setAttribute("role", type === "error" ? "alert" : "status");

    const iconWrap = document.createElement("span");
    iconWrap.className = "toast-icon";
    iconWrap.setAttribute("aria-hidden", "true");
    iconWrap.appendChild(createToastIcon(type));

    const body = document.createElement("span");
    body.className = "toast-body";
    body.textContent = String(message || "");

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "toast-close";
    closeBtn.setAttribute("aria-label", text("Bildirimi kapat", "Dismiss notification"));
    closeBtn.textContent = "×";

    let timer = null;
    const dismiss = () => {
      if (timer) clearTimeout(timer);
      el.remove();
    };
    closeBtn.addEventListener("click", dismiss);

    el.append(iconWrap, body, closeBtn);
    region.appendChild(el);
    timer = setTimeout(dismiss, duration);
  }

  function msiConfirm(options = {}) {
    return new Promise((resolve) => {
      const prevFocus = document.activeElement;
      let dlg = document.getElementById("msi-confirm-dialog");
      if (!dlg) {
        dlg = document.createElement("dialog");
        dlg.id = "msi-confirm-dialog";
        dlg.className = "ui-dialog ui-confirm-dialog";
        dlg.setAttribute("aria-modal", "true");
        dlg.setAttribute("aria-labelledby", "msi-confirm-title");
        dlg.setAttribute("aria-describedby", "msi-confirm-desc");

        const head = document.createElement("div");
        head.className = "dialog-heading";
        const titleWrap = document.createElement("div");
        const eyebrow = document.createElement("div");
        eyebrow.className = "eyebrow";
        eyebrow.id = "msi-confirm-eyebrow";
        const h2 = document.createElement("h2");
        h2.id = "msi-confirm-title";
        titleWrap.append(eyebrow, h2);

        const xBtn = document.createElement("button");
        xBtn.type = "button";
        xBtn.className = "icon-button";
        xBtn.dataset.confirmCancel = "1";
        xBtn.setAttribute("aria-label", text("Kapat", "Close"));
        xBtn.textContent = "×";
        head.append(titleWrap, xBtn);

        const body = document.createElement("p");
        body.id = "msi-confirm-desc";
        body.className = "ui-confirm-body";

        const actions = document.createElement("div");
        actions.className = "ui-confirm-actions";
        const cancelBtn = document.createElement("button");
        cancelBtn.type = "button";
        cancelBtn.className = "button secondary";
        cancelBtn.id = "msi-confirm-cancel";
        cancelBtn.dataset.confirmCancel = "1";
        const okBtn = document.createElement("button");
        okBtn.type = "button";
        okBtn.className = "button primary";
        okBtn.id = "msi-confirm-ok";
        actions.append(cancelBtn, okBtn);

        dlg.append(head, body, actions);
        document.body.appendChild(dlg);
      }

      const danger = Boolean(options.danger);
      dlg.classList.toggle("is-danger", danger);
      const eyebrowEl = document.getElementById("msi-confirm-eyebrow");
      const titleEl = document.getElementById("msi-confirm-title");
      const descEl = document.getElementById("msi-confirm-desc");
      const cancelBtn = document.getElementById("msi-confirm-cancel");
      const okBtn = document.getElementById("msi-confirm-ok");

      if (eyebrowEl) {
        eyebrowEl.textContent =
          options.eyebrow || text("ONAY GEREKLİ", "CONFIRMATION REQUIRED");
      }
      if (titleEl) {
        titleEl.textContent =
          options.title || text("İşlemi Onaylayın", "Confirm Action");
      }
      if (descEl) {
        descEl.textContent =
          options.message ||
          text("Bu işlemi gerçekleştirmek istiyor musunuz?", "Do you want to proceed with this action?");
      }
      if (cancelBtn) {
        cancelBtn.textContent = options.cancelLabel || text("Vazgeç", "Cancel");
      }
      if (okBtn) {
        okBtn.textContent = options.confirmLabel || text("Onayla", "Confirm");
        okBtn.className = danger ? "button danger" : "button primary";
      }

      let settled = false;
      const finish = (val) => {
        if (settled) return;
        settled = true;
        dlg.removeEventListener("close", onClose);
        dlg.removeEventListener("click", onBackdrop);
        dlg.removeEventListener("cancel", onCancel);
        okBtn?.removeEventListener("click", onOk);
        dlg.querySelectorAll("[data-confirm-cancel]").forEach((b) =>
          b.removeEventListener("click", onCancelBtn),
        );
        if (dlg.open) dlg.close();
        syncScrollLock();
        if (prevFocus && document.contains(prevFocus)) prevFocus.focus?.();
        resolve(val);
      };
      const onOk = () => finish(true);
      const onCancelBtn = () => finish(false);
      const onCancel = (ev) => {
        ev.preventDefault();
        finish(false);
      };
      const onClose = () => finish(false);
      const onBackdrop = (ev) => {
        if (ev.target !== dlg) return;
        const r = dlg.getBoundingClientRect();
        if (
          ev.clientX < r.left ||
          ev.clientX > r.right ||
          ev.clientY < r.top ||
          ev.clientY > r.bottom
        ) {
          finish(false);
        }
      };

      okBtn?.addEventListener("click", onOk);
      dlg.querySelectorAll("[data-confirm-cancel]").forEach((b) =>
        b.addEventListener("click", onCancelBtn),
      );
      dlg.addEventListener("cancel", onCancel);
      dlg.addEventListener("close", onClose);
      dlg.addEventListener("click", onBackdrop);

      try {
        dlg.showModal();
        syncScrollLock();
        (danger ? cancelBtn : okBtn)?.focus?.();
      } catch {
        finish(false);
      }
    });
  }

  function animateAuthSurface(el, fromVars, durationSec = 0.22) {
    if (!el || reducedMotion || matchMedia?.("(prefers-reduced-motion: reduce)")?.matches) return;
    const isDialog = el.tagName === "DIALOG";
    if (!isDialog && window.gsap && typeof window.gsap.fromTo === "function") {
      window.gsap.fromTo(
        el,
        fromVars,
        {
          opacity: 1,
          y: 0,
          scale: 1,
          duration: durationSec,
          ease: "power2.out",
          clearProps: "opacity,transform",
        },
      );
      return;
    }
    if (typeof el.animate === "function") {
      const y = typeof fromVars.y === "number" ? fromVars.y : 0;
      const scale = typeof fromVars.scale === "number" ? fromVars.scale : 1;
      const opacity = typeof fromVars.opacity === "number" ? fromVars.opacity : 1;
      try {
        el.animate(
          [
            { opacity, transform: `translate3d(0, ${y}px, 0) scale(${scale})` },
            { opacity: 1, transform: "translate3d(0, 0, 0) scale(1)" },
          ],
          {
            duration: Math.round(durationSec * 1000),
            easing: "cubic-bezier(0.22, 1, 0.36, 1)",
            fill: "none",
          },
        );
      } catch {}
    }
  }

  let restoreFocus;
  function openDialog(id) {
    const dialog = document.getElementById(id);
    if (!dialog) return;
    restoreFocus = document.activeElement;
    const wasOpen = dialog.open;
    if (!wasOpen) dialog.showModal();
    if (!wasOpen && id === "auth-dialog") {
      animateAuthSurface(dialog, { opacity: 0, y: 12, scale: 0.985 }, 0.24);
    }
    syncScrollLock();
  }
  $$("dialog").forEach((d) => {
    d.addEventListener("click", (e) => {
      if (e.target === d) {
        const r = d.getBoundingClientRect();
        if (
          e.clientX < r.left ||
          e.clientX > r.right ||
          e.clientY < r.top ||
          e.clientY > r.bottom
        )
          d.close();
      }
    });
    d.addEventListener("close", () => {
      syncScrollLock();
      restoreFocus?.focus?.();
    });
  });

  const themeButton = $("#btnToggleThemeMode");
  function resolveAutoTheme() {
    try {
      if (window.matchMedia) {
        if (matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
        if (matchMedia("(prefers-color-scheme: light)").matches) return "light";
      }
    } catch {}
    const hour = new Date().getHours();
    return hour >= 19 || hour < 7 ? "dark" : "light";
  }
  function getExplicitTheme() {
    const saved = storage.get("msi_theme") || storage.get("msi_admin_theme");
    return saved === "dark" || saved === "light" ? saved : "";
  }
  function applyTheme(mode) {
    const next = mode === "dark" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    themeButton?.setAttribute("aria-pressed", String(next === "dark"));
    themeButton?.setAttribute(
      "aria-label",
      next === "dark"
        ? text("Açık temaya geç", "Switch to light theme")
        : text("Koyu temaya geç", "Switch to dark theme"),
    );
  }
  const mqDark = window.matchMedia ? matchMedia("(prefers-color-scheme: dark)") : null;
  const mqLight = window.matchMedia ? matchMedia("(prefers-color-scheme: light)") : null;
  const syncAutoTheme = () => {
    if (!getExplicitTheme()) applyTheme(resolveAutoTheme());
  };
  applyTheme(getExplicitTheme() || resolveAutoTheme());
  mqDark?.addEventListener?.("change", syncAutoTheme);
  mqLight?.addEventListener?.("change", syncAutoTheme);
  window.setInterval(syncAutoTheme, 60000);
  themeButton?.addEventListener("click", () => {
    const next =
      document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    applyTheme(next);
    storage.set("msi_theme", next);
    storage.set("msi_admin_theme", next);
  });

  let currentPortalMode = null;
  function portal(mode, animate = false) {
    const normalized = mode === "news" ? "news" : "magazine";
    const changed =
      currentPortalMode !== null && currentPortalMode !== normalized;
    currentPortalMode = normalized;
    document.body.classList.toggle("show-news", normalized === "news");
    $$("[data-mode-switch]").forEach((sw) => {
      sw.dataset.activeMode = normalized;
    });
    $$("[data-portal]").forEach((a) => {
      const active = a.dataset.portal === normalized;
      a.classList.toggle("active", active);
      if (active) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    });
    if (animate && changed) {
      if (window.scrollY > 40) {
        if (lenis) lenis.scrollTo(0, { immediate: true, force: true });
        window.scrollTo({ top: 0, left: 0, behavior: "instant" });
        document.documentElement.scrollTop = 0;
      }
      if (!reducedMotion) {
        const activeView = $(
          normalized === "news" ? "#view-news" : "#view-magazine",
        );
        if (activeView) {
          if (window.gsap) {
            window.gsap.killTweensOf(activeView);
            window.gsap.fromTo(
              activeView,
              { opacity: 0.01, y: 14 },
              {
                opacity: 1,
                y: 0,
                duration: 0.36,
                ease: "power3.out",
                clearProps: "transform,opacity",
              },
            );
            const leadCards = $$(
              ".magazine-hero, .news-lead, .story-card",
              activeView,
            ).slice(0, 6);
            if (leadCards.length) {
              window.gsap.killTweensOf(leadCards);
              window.gsap.fromTo(
                leadCards,
                { opacity: 0.01, y: 12 },
                {
                  opacity: 1,
                  y: 0,
                  duration: 0.34,
                  stagger: 0.035,
                  ease: "power2.out",
                  clearProps: "transform,opacity",
                },
              );
            }
          } else if (activeView.animate) {
            activeView.animate(
              [
                { opacity: 0.01, transform: "translate3d(0, 12px, 0)" },
                { opacity: 1, transform: "translate3d(0, 0, 0)" },
              ],
              { duration: 320, easing: "cubic-bezier(0.22, 1, 0.36, 1)" },
            );
          }
        }
      }
    }
    if (window.ScrollTrigger) {
      setTimeout(() => window.ScrollTrigger.refresh(), 60);
    }
  }
  const hasPortals = !!$("#view-magazine");
  function fromURL(animate = false) {
    const mode =
      location.hash === "#haber"
        ? "news"
        : location.hash === "#dergi"
          ? "magazine"
          : new URL(location.href).searchParams.get("mode") || boot.mode;
    portal(mode, animate);
  }
  fromURL(false);
  window.addEventListener("popstate", () => fromURL(true));
  window.addEventListener("hashchange", () => fromURL(true));
  $$("[data-portal]").forEach((a) =>
    a.addEventListener("click", (e) => {
      const mode = a.dataset.portal;
      $$("[data-mode-switch]").forEach((sw) => {
        sw.dataset.activeMode = mode === "news" ? "news" : "magazine";
      });
      if (!hasPortals) return;
      e.preventDefault();
      const url = new URL(location.href);
      url.searchParams.set("mode", mode);
      url.hash = "";
      history.pushState(null, "", url);
      portal(mode, true);
    }),
  );

  const menu = $(".mobile-toggle"),
    nav = $("#site-navigation");
  menu?.addEventListener("click", () => {
    const open = nav.classList.toggle("is-open");
    menu.setAttribute("aria-expanded", String(open));
    menu.setAttribute(
      "aria-label",
      open ? text("Menüyü kapat", "Close menu") : text("Menüyü aç", "Open menu"),
    );
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      $$(".nav-more[open]").forEach((d) => (d.open = false));
      nav?.classList.remove("is-open");
      menu?.setAttribute("aria-expanded", "false");
    }
  });
  document.addEventListener("click", (e) => {
    const more = $(".nav-more");
    if (more && !more.contains(e.target)) more.open = false;
  });
  $$(".navigation>a:not(.language-link)").forEach((a) => {
    const rawHref = a.getAttribute("href") || "";
    if (
      rawHref.startsWith("/") &&
      new URL(a.href).pathname === location.pathname &&
      location.pathname !== "/"
    )
      a.setAttribute("aria-current", "page");
  });
  $$("[data-language]").forEach((el) =>
    el.addEventListener("click", (e) => {
      e.preventDefault();
      const lang = el.dataset.language || "tr";
      document.cookie = `msi_lang=${lang};path=/;SameSite=Lax;max-age=31536000`;
      const baseHref = el.getAttribute("href") || `/switch/${lang}`;
      const rawHash = (location.hash || "").replace(/^#/, "");
      if (/^[a-zA-Z0-9_-]{1,40}$/.test(rawHash)) {
        const sep = baseHref.includes("?") ? "&" : "?";
        location.href = `${baseHref}${sep}hash=${encodeURIComponent(rawHash)}`;
      } else {
        location.href = baseHref;
      }
    }),
  );

  // Notifications state
  let readIds;
  try {
    readIds = JSON.parse(storage.get("msi_read_notifications") || "[]");
  } catch {
    readIds = [];
  }
  if (!Array.isArray(readIds)) readIds = [];
  function refreshNotifications() {
    let count = 0;
    $$("[data-notification-id]").forEach((el) => {
      const read = readIds.includes(el.dataset.notificationId);
      el.classList.toggle("is-read", read);
      if (!read) count++;
    });
    const badge = $(".notification-count:not(.cart-count)");
    if (badge) {
      badge.textContent = count;
      badge.hidden = !count;
    }
  }
  refreshNotifications();
  $("[data-read-notifications]")?.addEventListener("click", () => {
    readIds = $$("[data-notification-id]").map(
      (el) => el.dataset.notificationId,
    );
    storage.set("msi_read_notifications", JSON.stringify(readIds));
    refreshNotifications();
    toast(
      text(
        "Bildirimler bu cihazda okundu olarak işaretlendi.",
        "Notifications marked as read on this device.",
      ),
      "success",
    );
  });
  $("[data-dismiss-announcement]")?.addEventListener("click", () => {
    const bar = $("#site-announcement");
    storage.set("msi_dismissed_announcement", bar?.textContent.trim());
    bar?.remove();
  });
  if (
    storage.get("msi_dismissed_announcement") ===
    $("#site-announcement")?.textContent.trim()
  )
    $("#site-announcement")?.remove();

  // Top Laser Scroll Telemetry Progress Bar
  const telemetryBar = $("#scroll-telemetry");
  if (telemetryBar) {
    let ticking = false;
    const syncTelemetry = () => {
      ticking = false;
      const doc = document.documentElement;
      const max = Math.max(1, doc.scrollHeight - window.innerHeight);
      const p = Math.min(1, Math.max(0, window.scrollY / max));
      telemetryBar.style.setProperty("--scroll-progress", p.toFixed(4));
    };
    window.addEventListener(
      "scroll",
      () => {
        if (!ticking) {
          ticking = true;
          requestAnimationFrame(syncTelemetry);
        }
      },
      { passive: true },
    );
    syncTelemetry();
  }

  // Live Breaking News Ticker (Seamless Marquee + Manual Step Controls)
  $$("[data-news-ticker]").forEach((ticker) => {
    const items = $$("[data-ticker-item]", ticker);
    if (items.length < 2) return;
    let idx = 0;
    let resumeTimer = null;
    if (reducedMotion || inIframe) {
      ticker.classList.remove("is-marquee");
      ticker.classList.add("is-manual");
    }
    const showManual = (next) => {
      ticker.classList.remove("is-marquee");
      ticker.classList.add("is-manual");
      idx = ((next % items.length) + items.length) % items.length;
      items.forEach((el, i) => el.classList.toggle("is-active", i === idx));
      clearTimeout(resumeTimer);
      if (!reducedMotion && !inIframe) {
        resumeTimer = setTimeout(() => {
          ticker.classList.remove("is-manual");
          ticker.classList.add("is-marquee");
        }, 9000);
      }
    };
    $("[data-ticker-prev]", ticker)?.addEventListener("click", () => showManual(idx - 1));
    $("[data-ticker-next]", ticker)?.addEventListener("click", () => showManual(idx + 1));
  });

  // Live Interactive 3D Hero Mockup & Auto-Advancing Dossier Switcher
  const normHeroPubType = (iss) =>
    String(iss?.publication_type || "msi").trim().toLowerCase() === "msi_tdr"
      ? "msi_tdr"
      : "msi";
  $$("[data-hero-stage]").forEach((stage) => {
    const issueNum = Number(stage.dataset.issueNumber);
    const stagePub = (stage.dataset.publicationType || "").trim().toLowerCase();
    const stageIssue =
      boot.issues?.find(
        (i) => i.issue_number === issueNum && (!stagePub || normHeroPubType(i) === stagePub),
      ) ||
      boot.issues?.find((i) => i.issue_number === issueNum) ||
      boot.issues?.[0];
    const spreads = (stageIssue?.spreads || []).slice(0, 4);
    const progressEl = $("[data-dossier-progress]", stage);
    let activeIdx = 0;
    let stagePaused = false;

    const restartProgressBar = () => {
      if (!progressEl || reducedMotion || inIframe || spreads.length < 2) return;
      progressEl.classList.remove("is-running");
      void progressEl.offsetWidth;
      progressEl.classList.add("is-running");
    };

    function selectHeroSpread(nextIdx) {
      if (!spreads.length) return;
      activeIdx = ((nextIdx % spreads.length) + spreads.length) % spreads.length;
      const sp = spreads[activeIdx];
      $$("[data-hero-spread]", stage).forEach((tab, idx) => {
        const on = idx === activeIdx;
        tab.classList.toggle("is-active", on);
        tab.setAttribute("aria-selected", String(on));
      });
      const badgeEl = $("[data-dossier-badge]", stage);
      const pagesEl = $("[data-dossier-pages]", stage);
      const titleEl = $("[data-dossier-title]", stage);
      const excerptEl = $("[data-dossier-excerpt]", stage);
      const actionsEl = $(".dossier-actions", stage);
      const ttl = localized(sp, "title").trim();
      const exc = localized(sp, "excerpt").trim();
      const showExc = Boolean(exc) && exc.toLowerCase() !== ttl.toLowerCase();
      const badgeMap = {
        "KAPAK DOSYASI": "COVER FILE",
        "KAPAK DOSYALARI": "COVER FILE",
        "SÖYLEŞİ": "INTERVIEW",
        "SÖYLEŞİLER": "INTERVIEW",
        "ÖZEL DOSYA": "SPECIAL DOSSIER",
        "ÖZEL DOSYALAR": "SPECIAL DOSSIER",
        "ÖZEL HABER": "EXCLUSIVE REPORT",
        "ÖZEL HABERLER": "EXCLUSIVE REPORT",
        "BAŞYAZI": "EDITORIAL",
        "BAŞYAZILAR": "EDITORIAL",
        "MESAJ": "EXECUTIVE MESSAGE",
        "MESAJLAR": "EXECUTIVE MESSAGE",
        "TANITICI MAKALE": "SPECIAL COVERAGE",
        "TANITICI MAKALELER": "SPECIAL COVERAGE",
        "ANALİZ": "ANALYSIS",
        "MAKALE / ANALİZ": "ANALYSIS",
        "HABER": "NEWS",
        "HABERLER": "NEWS",
        "YENİ SAYI": "NEW ISSUE",
        "DOSYA": "DOSSIER",
        "ADVERTORIALS": "ADVERTORIALS",
      };
      const rawBadge = (sp.category_badge || "ÖZEL DOSYA").trim();
      const rawPages = sp.page_no || "";
      const mappedBadge =
        badgeMap[rawBadge.toLocaleUpperCase("tr-TR")] ||
        badgeMap[rawBadge.toUpperCase()] ||
        rawBadge;
      if (badgeEl) badgeEl.textContent = boot.lang === "en" ? mappedBadge : rawBadge;
      if (pagesEl)
        pagesEl.textContent =
          boot.lang === "en"
            ? rawPages.replace(/^S\.\s*/i, "P. ").replace(/^DOSYA\s+/i, "DOSSIER ")
            : rawPages;
      if (titleEl) titleEl.textContent = ttl;
      if (excerptEl) {
        excerptEl.textContent = showExc ? exc : "";
        excerptEl.hidden = !showExc;
      }
      if (actionsEl) {
        const oldLink = $("[data-dossier-link], .dossier-link", actionsEl);
        if (oldLink) {
          if (sp.linked_content_slug) {
            const a = document.createElement("a");
            a.className = "dossier-link";
            a.dataset.dossierLink = "";
            a.href = "/icerik/" + encodeURIComponent(sp.linked_content_slug) + (boot.lang === "en" ? "?lang=en" : "");
            a.textContent = text("Dosyayı oku →", "Read dossier →");
            oldLink.replaceWith(a);
          } else {
            const btn = document.createElement("button");
            btn.type = "button";
            btn.className = "dossier-link";
            btn.dataset.openIssue = String(issueNum);
            const resolvedPub = stageIssue ? normHeroPubType(stageIssue) : stagePub;
            if (resolvedPub) btn.dataset.publicationType = resolvedPub;
            btn.textContent = text("Sayıda aç →", "Open in issue →");
            oldLink.replaceWith(btn);
          }
        }
      }
      restartProgressBar();
      if (window.gsap && !reducedMotion) {
        const card = $("[data-live-dossier]", stage);
        const cover = $(".hero-cover", stage);
        if (card) {
          window.gsap.fromTo(
            [badgeEl, titleEl, excerptEl].filter(Boolean),
            { y: 8, opacity: 0.45 },
            {
              y: 0,
              opacity: 1,
              duration: 0.32,
              stagger: 0.04,
              ease: "power2.out",
              clearProps: "transform,opacity",
            },
          );
        }
        if (cover) {
          window.gsap.fromTo(
            cover,
            { scale: 0.988 },
            { scale: 1, duration: 0.36, ease: "power2.out", clearProps: "scale" },
          );
        }
      }
    }

    $$("[data-hero-spread]", stage).forEach((tab) => {
      tab.addEventListener("click", () =>
        selectHeroSpread(Number(tab.dataset.heroSpread || 0)),
      );
    });
    $("[data-hero-next-spread]", stage)?.addEventListener("click", () =>
      selectHeroSpread(activeIdx + 1),
    );
    stage.addEventListener("pointerenter", () => {
      stagePaused = true;
    });
    stage.addEventListener("pointerleave", () => {
      stagePaused = false;
    });
    stage.addEventListener("focusin", () => {
      stagePaused = true;
    });
    stage.addEventListener("focusout", () => {
      stagePaused = false;
    });
    restartProgressBar();
    if (!reducedMotion && !inIframe && spreads.length > 1) {
      setInterval(() => {
        if (!stagePaused && !document.hidden) selectHeroSpread(activeIdx + 1);
      }, 5400);
    }

    const tiltWrap = $("[data-tilt-card]", stage);
    const coverBtn = $(".hero-cover", stage);
    if (tiltWrap && coverBtn && !reducedMotion) {
      tiltWrap.addEventListener("pointermove", (e) => {
        const r = tiltWrap.getBoundingClientRect();
        if (!r.width || !r.height) return;
        const px = (e.clientX - r.left) / r.width - 0.5;
        const py = (e.clientY - r.top) / r.height - 0.5;
        coverBtn.style.setProperty("--ry", `${(px * 6).toFixed(2)}deg`);
        coverBtn.style.setProperty("--rx", `${(-py * 5).toFixed(2)}deg`);
        coverBtn.style.setProperty(
          "--sheen",
          `${Math.round(30 + (px + 0.5) * 45)}%`,
        );
      });
      tiltWrap.addEventListener("pointerleave", () => {
        coverBtn.style.removeProperty("--ry");
        coverBtn.style.removeProperty("--rx");
        coverBtn.style.removeProperty("--sheen");
      });
    }
  });

  // Cursor glow border coordinates
  if (!reducedMotion) {
    document.addEventListener(
      "pointermove",
      (e) => {
        const card = e.target.closest(
          ".plan-card, .membership-banner, .hero-cover-stage, .subdossier-card, .ad-visual-banner",
        );
        if (!card) return;
        const r = card.getBoundingClientRect();
        card.style.setProperty("--mx", `${Math.round(e.clientX - r.left)}px`);
        card.style.setProperty("--my", `${Math.round(e.clientY - r.top)}px`);
      },
      { passive: true },
    );
  }

  // Live Hero Telemetry Counter Roll-Up (non-destructive)
  if (!reducedMotion && !inIframe) {
    $$(".hero-metrics strong").forEach((el) => {
      const raw = el.textContent.trim();
      const match = raw.match(/^(\d+)(\+.*)$/);
      if (!match) return;
      const target = Number(match[1]);
      const suffix = match[2];
      if (!target || target > 9999) return;
      const start = performance.now();
      const dur = 950;
      const tick = (now) => {
        const t = Math.min(1, (now - start) / dur);
        const eased = 1 - Math.pow(1 - t, 3);
        el.textContent = `${Math.max(1, Math.round(target * eased))}${suffix}`;
        if (t < 1) requestAnimationFrame(tick);
        else el.textContent = raw;
      };
      requestAnimationFrame(tick);
    });
  }

  // GSAP + ScrollTrigger Motion Choreography (non-destructive for full-page screenshots)
  if (window.gsap && !reducedMotion && !inIframe) {
    const heroItems = $$(".magazine-hero .hero-copy > *");
    if (heroItems.length) {
      window.gsap.from(heroItems, {
        y: 20,
        duration: 0.68,
        stagger: 0.065,
        ease: "power3.out",
        clearProps: "transform",
      });
    }
    const heroStage = $(".magazine-hero .hero-cover-stage");
    if (heroStage) {
      window.gsap.from(heroStage, {
        y: 22,
        duration: 0.78,
        ease: "power3.out",
        clearProps: "transform",
      });
    }
    if (window.ScrollTrigger) {
      $$("[data-section], .category-page, .article-page").forEach((sec) => {
        const headingEls = $$(
          ".section-heading > div, .msi-section-ribbon",
          sec,
        );
        const cards = $$(
          ".story-card, .subdossier-card, .ad-visual-banner, .issue-card, .plan-card, .topics-grid > a",
          sec,
        ).slice(0, 14);
        if (!cards.length && !headingEls.length) return;
        window.ScrollTrigger.create({
          trigger: sec,
          start: "top 89%",
          once: true,
          onEnter: () => {
            if (headingEls.length) {
              window.gsap.fromTo(
                headingEls,
                { x: -12 },
                {
                  x: 0,
                  duration: 0.45,
                  ease: "power2.out",
                  overwrite: "auto",
                  clearProps: "transform",
                },
              );
            }
            if (cards.length) {
              window.gsap.fromTo(
                cards,
                { y: 22 },
                {
                  y: 0,
                  duration: 0.56,
                  stagger: 0.045,
                  ease: "power3.out",
                  overwrite: "auto",
                  clearProps: "transform",
                },
              );
            }
          },
        });
      });
    }
  }

  // Full-screen E-Dergi dialog reader, interactive flipbook & partial/full watermarked export
  let issue,
    spreadIndex = 0;
  const fbState = {
    page: 1,
    totalPages: 1,
    verifiedTotalPages: null,
    renderSeq: 0,
    zoom: 1,
    activeView: "toc",
    touchStartX: 0,
    touchStartY: 0,
    pageCache: new Map(),
    inflight: new Map(),
    abortControllers: new Set(),
  };
  const canReadPdf = () => Boolean($("#issue-pdf-frame"));
  const isMobileFlipbook = () => matchMedia("(max-width: 768px)").matches;
  const issuePubType = (iss) =>
    String(iss?.publication_type || "msi").toLowerCase() === "msi_tdr"
      ? "msi_tdr"
      : "msi";

  function resetFlipbookCache() {
    fbState.renderSeq += 1;
    fbState.abortControllers.forEach((ctrl) => {
      try {
        ctrl.abort();
      } catch {}
    });
    fbState.abortControllers.clear();
    fbState.inflight.clear();
    [
      ["edergi-slot-left", "edergi-page-left"],
      ["edergi-slot-right", "edergi-page-right"],
    ].forEach(([slotId, imgId]) => {
      const slot = $("#" + slotId);
      const img = $("#" + imgId);
      if (slot) slot.classList.remove("is-loading");
      if (img) {
        img.onload = null;
        img.onerror = null;
        img.removeAttribute("src");
        delete img.dataset.loadedSrc;
        delete img.dataset.cacheKey;
      }
    });
    fbState.pageCache.forEach((entry) => {
      if (entry?.objectUrl) {
        try {
          URL.revokeObjectURL(entry.objectUrl);
        } catch {}
      }
    });
    fbState.pageCache.clear();
    fbState.verifiedTotalPages = null;
  }

  function parseSpreadStartPage(sp, fallbackIdx) {
    const m = String(sp?.page_no || "").match(/(\d+)/);
    const parsed = m ? Number(m[1]) : NaN;
    return Number.isFinite(parsed) && parsed >= 1 ? parsed : fallbackIdx + 1;
  }

  function getFlipbookSlots() {
    const total = Math.max(1, fbState.totalPages || 1);
    const p = Math.max(1, Math.min(total, fbState.page || 1));
    if (isMobileFlipbook()) {
      return [p, null];
    }
    if (p === 1) {
      return [null, 1];
    }
    const left = p % 2 === 0 ? p : p - 1;
    const right = left + 1 <= total ? left + 1 : null;
    return [left, right];
  }

  function getCurrentExportRange() {
    if (fbState.activeView === "flipbook") {
      const [left, right] = getFlipbookSlots();
      if (left && right) return `${left}-${right}`;
      return String(left || right || 1);
    }
    const spreads = issue?.spreads || [];
    const sp = spreads[spreadIndex];
    if (sp) {
      const nums = String(sp.page_no || "").match(/\d+/g);
      if (nums && nums.length >= 2) {
        const a = Math.max(1, Number(nums[0]));
        const b = Math.max(a, Number(nums[1]));
        return `${a}-${b}`;
      }
      if (nums && nums.length === 1) {
        return String(Math.max(1, Number(nums[0])));
      }
    }
    return "1";
  }

  function validateCustomPageSelection(raw) {
    const cleaned = String(raw || "").trim();
    if (!cleaned) {
      return {
        ok: false,
        msg: text(
          "Lütfen sayfa veya aralık girin (örn: 1-5, 8).",
          "Please enter pages or ranges (e.g. 1-5, 8).",
        ),
      };
    }
    const parts = cleaned.split(",");
    const total = Math.max(1, fbState.totalPages || issue?.page_count || 999);
    for (const part of parts) {
      const tok = part.trim();
      if (!tok) {
        return {
          ok: false,
          msg: text(
            "Virgül sonrası boş sayfa değeri olamaz.",
            "Empty segment after comma is not allowed.",
          ),
        };
      }
      if (tok.includes("-")) {
        const bounds = tok.split("-");
        if (bounds.length !== 2) {
          return {
            ok: false,
            msg: text("Geçersiz sayfa aralığı: " + tok, "Invalid page range: " + tok),
          };
        }
        const a = Number(bounds[0].trim());
        const b = Number(bounds[1].trim());
        if (!Number.isInteger(a) || !Number.isInteger(b) || a < 1 || b < 1 || a > b) {
          return {
            ok: false,
            msg: text(
              "Aralık başlangıcı bitişten küçük ve ≥ 1 olmalıdır.",
              "Range start must be ≥ 1 and ≤ end.",
            ),
          };
        }
        if (b > total) {
          return {
            ok: false,
            msg: text(
              `Sayfa numarası toplam sayfa sayısını (${total}) aşamaz.`,
              `Page number cannot exceed total pages (${total}).`,
            ),
          };
        }
      } else {
        const p = Number(tok);
        if (!Number.isInteger(p) || p < 1) {
          return {
            ok: false,
            msg: text("Geçersiz sayfa numarası: " + tok, "Invalid page number: " + tok),
          };
        }
        if (p > total) {
          return {
            ok: false,
            msg: text(
              `Sayfa ${p} toplam sayfa sayısını (${total}) aşıyor.`,
              `Page ${p} exceeds total pages (${total}).`,
            ),
          };
        }
      }
    }
    return { ok: true, value: cleaned.replace(/\s+/g, "") };
  }

  function syncExportLink() {
    const downloadEl = $("#issue-download");
    const pagesInput = $("#edergi-pages-input");
    const fbEl = $("#edergi-export-feedback");
    const curLbl = $("#edergi-export-current-label");
    const mode = $('input[name="edergi_export_mode"]:checked')?.value || "all";
    const curRange = getCurrentExportRange();
    if (curLbl) {
      curLbl.textContent = `${text("Bu Sayfa / Çift Sayfa", "Current Page / Spread")} (${curRange})`;
    }
    if (pagesInput) {
      pagesInput.disabled = mode !== "custom";
    }
    if (!issue || !downloadEl) return;
    const pubType = issuePubType(issue);
    let pagesParam = "all";
    if (mode === "current") {
      pagesParam = curRange;
      if (fbEl) {
        fbEl.classList.remove("error");
        fbEl.textContent = "";
      }
    } else if (mode === "custom") {
      const check = validateCustomPageSelection(pagesInput?.value || "");
      if (check.ok) {
        pagesParam = check.value;
        if (fbEl) {
          fbEl.classList.remove("error");
          fbEl.textContent = "";
        }
      } else {
        pagesParam = "all";
      }
    } else if (fbEl) {
      fbEl.classList.remove("error");
      fbEl.textContent = "";
    }
    downloadEl.href = `/api/v1/edergi/${encodeURIComponent(issue.issue_number)}/download-pdf?pages=${encodeURIComponent(pagesParam)}&publication_type=${encodeURIComponent(pubType)}`;
  }

  function showFlipbookGate(message, showAuthActions) {
    const gate = $("#edergi-fb-gate");
    const msgEl = $("#edergi-fb-gate-msg");
    const actEl = $("#edergi-fb-gate-actions");
    if (!gate || !msgEl || !actEl) return;
    msgEl.textContent = message;
    actEl.replaceChildren();
    if (showAuthActions) {
      const loginBtn = document.createElement("a");
      loginBtn.href = "#modal-login";
      loginBtn.className = "button primary";
      loginBtn.textContent = text("Giriş yap / Hesap oluştur →", "Sign in / Create account →");
      const subBtn = document.createElement("a");
      subBtn.href = boot.lang === "en" ? "/abonelik-ve-satis?lang=en" : "/abonelik-ve-satis";
      subBtn.className = "button secondary";
      subBtn.textContent = text("Abonelikleri incele", "Explore subscriptions");
      actEl.append(loginBtn, subBtn);
    } else {
      const retryBtn = document.createElement("button");
      retryBtn.type = "button";
      retryBtn.className = "button secondary";
      retryBtn.textContent = text("Tekrar dene", "Retry");
      retryBtn.addEventListener("click", () => {
        gate.hidden = true;
        void renderFlipbook();
      });
      actEl.append(retryBtn);
    }
    gate.hidden = false;
  }

  function syncFlipbookControls() {
    const total = Math.max(1, fbState.totalPages || 1);
    const pageInput = $("#edergi-page-input");
    const pageTotal = $("#edergi-page-total");
    const prevBtn = $("[data-fb-prev]");
    const nextBtn = $("[data-fb-next]");
    const [leftPage, rightPage] = getFlipbookSlots();

    if (pageInput) {
      pageInput.max = String(total);
      pageInput.value = String(Math.max(1, Math.min(total, fbState.page || 1)));
    }
    if (pageTotal) pageTotal.textContent = String(total);
    if (prevBtn) prevBtn.disabled = fbState.page <= 1;
    if (nextBtn) {
      const highestShown = rightPage || leftPage || fbState.page;
      nextBtn.disabled = highestShown >= total;
    }
    syncExportLink();
  }

  function applyVerifiedTotalPages(count) {
    if (!Number.isFinite(count) || count < 1) return;
    fbState.verifiedTotalPages = count;
    fbState.totalPages = count;
    if (fbState.page > count) {
      fbState.page = count;
    }
    syncFlipbookControls();
  }

  async function fetchFlipbookPage(pageNum) {
    if (!issue || !canReadPdf() || !Number.isInteger(pageNum) || pageNum < 1) {
      return { ok: false, status: 0 };
    }
    if (fbState.verifiedTotalPages !== null && pageNum > fbState.verifiedTotalPages) {
      return { ok: false, status: 404, outOfBounds: true };
    }
    const pubType = issuePubType(issue);
    const targetIssue = issue;
    const cacheKey = `${issue.issue_number}:${pubType}:${pageNum}`;
    if (fbState.pageCache.has(cacheKey)) {
      const cached = fbState.pageCache.get(cacheKey);
      if (cached?.totalPages >= 1 && fbState.verifiedTotalPages == null) {
        applyVerifiedTotalPages(cached.totalPages);
      }
      return {
        ok: true,
        objectUrl: cached.objectUrl,
        url: cached.url,
        totalPages: cached.totalPages,
      };
    }
    if (fbState.inflight.has(cacheKey)) {
      return fbState.inflight.get(cacheKey);
    }

    const url = `/api/v1/edergi/${encodeURIComponent(issue.issue_number)}/pages/${pageNum}?publication_type=${encodeURIComponent(pubType)}`;
    const ctrl = typeof AbortController !== "undefined" ? new AbortController() : null;
    if (ctrl) fbState.abortControllers.add(ctrl);

    const promise = (async () => {
      try {
        const res = await fetch(url, {
          credentials: "same-origin",
          signal: ctrl?.signal,
        });
        if (ctrl?.signal.aborted || issue !== targetIssue) {
          return { ok: false, aborted: true };
        }
        const hdrCount = Number(res.headers.get("X-MSI-Page-Count") || "0");
        if (res.ok) {
          if (hdrCount >= 1) {
            applyVerifiedTotalPages(hdrCount);
          }
          const blob = await res.blob();
          if (ctrl?.signal.aborted || issue !== targetIssue) {
            return { ok: false, aborted: true };
          }
          const objectUrl = URL.createObjectURL(blob);
          const entry = {
            objectUrl,
            url,
            totalPages: hdrCount >= 1 ? hdrCount : null,
          };
          fbState.pageCache.set(cacheKey, entry);
          return { ok: true, ...entry };
        }
        if (res.status === 401 || res.status === 403) {
          let payload = {};
          try {
            payload = await res.json();
          } catch {}
          return {
            ok: false,
            status: res.status,
            authError: true,
            message:
              payload?.message ||
              text(
                "Bu sayıyı okuyabilmek için doğrulanmış aktif abonelik gereklidir.",
                "An active verified subscription is required to read this issue.",
              ),
          };
        }
        if (res.status === 404 && hdrCount >= 1) {
          applyVerifiedTotalPages(hdrCount);
          return {
            ok: false,
            status: 404,
            outOfBounds: true,
            totalPages: hdrCount,
          };
        }
        return { ok: false, status: res.status };
      } catch (err) {
        if (err?.name === "AbortError" || ctrl?.signal.aborted) {
          return { ok: false, aborted: true };
        }
        return { ok: false, status: 0, error: err };
      } finally {
        if (ctrl) fbState.abortControllers.delete(ctrl);
        if (fbState.inflight.get(cacheKey) === promise) {
          fbState.inflight.delete(cacheKey);
        }
      }
    })();

    fbState.inflight.set(cacheKey, promise);
    return promise;
  }

  async function renderFlipbookSlot(slotId, imgId, badgeId, pageNum, seq) {
    const slot = $("#" + slotId);
    const img = $("#" + imgId);
    const badge = $("#" + badgeId);
    if (!slot || !img) return { ok: true };
    if (!pageNum) {
      slot.classList.add("is-empty");
      slot.classList.remove("is-loading");
      img.onload = null;
      img.onerror = null;
      img.removeAttribute("src");
      delete img.dataset.loadedSrc;
      delete img.dataset.cacheKey;
      img.alt = "";
      if (badge) badge.textContent = "";
      return { ok: true };
    }
    slot.classList.remove("is-empty");
    if (badge) badge.textContent = String(pageNum);
    const pubType = issuePubType(issue);
    const cacheKey = `${issue.issue_number}:${pubType}:${pageNum}`;
    if (
      img.dataset.cacheKey === cacheKey &&
      img.src &&
      img.complete &&
      img.naturalWidth > 0
    ) {
      slot.classList.remove("is-loading");
      return { ok: true };
    }
    slot.classList.add("is-loading");
    img.alt = `${localized(issue, "title")} — ${text("Sayfa", "Page")} ${pageNum}`;

    const res = await fetchFlipbookPage(pageNum);
    if (seq !== fbState.renderSeq || !issue) {
      slot.classList.remove("is-loading");
      return { ok: false, aborted: true };
    }
    if (res.ok && res.objectUrl) {
      img.onload = () => {
        slot.classList.remove("is-loading");
        img.dataset.loadedSrc = res.url;
        img.dataset.cacheKey = cacheKey;
      };
      img.onerror = () => {
        slot.classList.remove("is-loading");
        delete img.dataset.loadedSrc;
        delete img.dataset.cacheKey;
      };
      img.src = res.objectUrl;
      if (img.complete && img.naturalWidth > 0) {
        slot.classList.remove("is-loading");
        img.dataset.loadedSrc = res.url;
        img.dataset.cacheKey = cacheKey;
      }
      return res;
    }
    slot.classList.remove("is-loading");
    delete img.dataset.loadedSrc;
    delete img.dataset.cacheKey;
    return res;
  }

  function preloadFlipbookPage(pageNum) {
    if (
      !issue ||
      !canReadPdf() ||
      fbState.verifiedTotalPages == null ||
      pageNum < 1 ||
      pageNum > fbState.verifiedTotalPages
    ) {
      return;
    }
    void fetchFlipbookPage(pageNum);
  }

  async function renderFlipbook(turnDir) {
    if (!issue) return;
    const seq = ++fbState.renderSeq;
    try {
      if (canReadPdf() && fbState.verifiedTotalPages == null && fbState.page > 1) {
        const probe = await fetchFlipbookPage(1);
        if (seq !== fbState.renderSeq || !issue) return;
        if (probe?.authError) {
          showFlipbookGate(probe.message, true);
          syncFlipbookControls();
          return;
        }
        if (fbState.verifiedTotalPages != null) {
          fbState.page = Math.max(1, Math.min(fbState.verifiedTotalPages, fbState.page));
        }
      }

      const total = Math.max(1, fbState.totalPages || 1);
      fbState.page = Math.max(1, Math.min(total, fbState.page || 1));

      const zoomEl = $("#edergi-zoom-level");
      const book = $("#edergi-book");
      const gate = $("#edergi-fb-gate");

      if (zoomEl) zoomEl.textContent = `${Math.round(fbState.zoom * 100)}%`;
      if (book) {
        book.style.setProperty("--edergi-zoom", String(fbState.zoom));
        book.classList.toggle("is-zoomed", fbState.zoom > 1.01);
        if (turnDir && !reducedMotion) {
          const cls = turnDir === "next" ? "is-turning-next" : "is-turning-prev";
          book.classList.remove("is-turning-next", "is-turning-prev");
          void book.offsetWidth;
          book.classList.add(cls);
          setTimeout(() => book.classList.remove(cls), 360);
        }
      }

      syncFlipbookControls();
      const [leftPage, rightPage] = getFlipbookSlots();

      if (!canReadPdf()) {
        await Promise.all([
          renderFlipbookSlot("edergi-slot-left", "edergi-page-left", "edergi-badge-left", null, seq),
          renderFlipbookSlot("edergi-slot-right", "edergi-page-right", "edergi-badge-right", null, seq),
        ]);
        if (seq !== fbState.renderSeq || !issue) return;
        showFlipbookGate(
          text(
            "İnteraktif E-Dergi okuyucusu aktif aboneler ve doğrulanmış kamu personeli içindir. Giriş yaparak tüm sayfaları filigranlı okuyabilirsiniz.",
            "The interactive E-Magazine reader is available to active subscribers and verified public-sector personnel. Sign in to read all pages.",
          ),
          true,
        );
        syncExportLink();
        return;
      }

      if (gate) gate.hidden = true;
      const [leftRes, rightRes] = await Promise.all([
        renderFlipbookSlot("edergi-slot-left", "edergi-page-left", "edergi-badge-left", leftPage, seq),
        renderFlipbookSlot("edergi-slot-right", "edergi-page-right", "edergi-badge-right", rightPage, seq),
      ]);
      if (seq !== fbState.renderSeq || !issue) return;

      const authFailure = [leftRes, rightRes].find((r) => r && r.authError);
      if (authFailure) {
        showFlipbookGate(authFailure.message, true);
        syncFlipbookControls();
        return;
      }

      const otherFailure = [leftRes, rightRes].find(
        (r) => r && !r.ok && !r.aborted && !r.outOfBounds,
      );
      if (otherFailure) {
        showFlipbookGate(
          text(
            "Sayfa görüntüsü yüklenemedi. Lütfen tekrar deneyin.",
            "Could not load page image. Please try again.",
          ),
          false,
        );
        syncFlipbookControls();
        return;
      }

      if (fbState.verifiedTotalPages != null && fbState.page > fbState.verifiedTotalPages) {
        fbState.page = fbState.verifiedTotalPages;
        void renderFlipbook();
        return;
      }

      syncFlipbookControls();
      const [curLeft, curRight] = getFlipbookSlots();
      const maxVisible = curRight || curLeft || fbState.page;
      preloadFlipbookPage(maxVisible + 1);
      preloadFlipbookPage(maxVisible + 2);
    } catch {
      // Catch cleanly so browser teardown/aborts never cause unhandled rejections
    }
  }

  async function stepFlipbook(delta) {
    if (!issue) return;
    try {
      let preempted = false;
      if (canReadPdf() && fbState.verifiedTotalPages == null && delta > 0) {
        const seq = ++fbState.renderSeq;
        preempted = true;
        const probe = await fetchFlipbookPage(1);
        if (seq !== fbState.renderSeq || !issue) return;
        if (probe?.authError) {
          showFlipbookGate(probe.message, true);
          syncFlipbookControls();
          return;
        }
      }
      const total = Math.max(1, fbState.verifiedTotalPages ?? fbState.totalPages ?? 1);
      let next = fbState.page;
      if (isMobileFlipbook()) {
        next = Math.max(1, Math.min(total, fbState.page + delta));
      } else if (delta > 0) {
        next = fbState.page === 1 ? Math.min(total, 2) : Math.min(total, fbState.page + 2);
      } else {
        next = fbState.page <= 2 ? 1 : Math.max(1, fbState.page - 2);
      }
      if (next !== fbState.page) {
        fbState.page = next;
        void renderFlipbook(delta > 0 ? "next" : "prev");
      } else if (preempted) {
        void renderFlipbook();
      } else {
        syncFlipbookControls();
      }
    } catch {}
  }

  async function jumpToFlipbookPage(targetPage) {
    if (!issue) return;
    try {
      const seq = ++fbState.renderSeq;
      const rawTarget = Math.max(1, Number(targetPage) || 1);
      if (canReadPdf() && fbState.verifiedTotalPages == null && rawTarget > 1) {
        const probe = await fetchFlipbookPage(1);
        if (seq !== fbState.renderSeq || !issue) return;
        if (probe?.authError) {
          fbState.page = 1;
          setIssueView("flipbook");
          return;
        }
      }
      const maxAllowed = Math.max(
        1,
        fbState.verifiedTotalPages ?? fbState.totalPages ?? 1,
      );
      const p = Math.max(1, Math.min(maxAllowed, rawTarget));
      fbState.page = isMobileFlipbook() || p === 1 ? p : p % 2 === 0 ? p : p - 1;
      setIssueView("flipbook");
    } catch {}
  }

  function populateIssueTocLists() {
    const spreads = issue?.spreads || [];
    const tocList = $("#edergi-toc-list");
    const tocJump = $("#edergi-toc-jump");
    if (tocList) {
      tocList.replaceChildren();
      spreads.forEach((sp, idx) => {
        const startPage = parseSpreadStartPage(sp, idx);
        const btn = document.createElement("button");
        btn.type = "button";
        btn.className = "edergi-toc-item";
        const badge = document.createElement("span");
        badge.className = "edergi-toc-page";
        badge.textContent = sp.page_no || `S. ${startPage}`;
        const label = document.createElement("span");
        label.className = "edergi-toc-title";
        label.textContent = localized(sp, "title");
        btn.append(badge, label);
        btn.addEventListener("click", () => {
          spreadIndex = idx;
          renderSpread();
          void jumpToFlipbookPage(startPage);
        });
        tocList.append(btn);
      });
    }
    if (tocJump) {
      tocJump.replaceChildren();
      const ph = document.createElement("option");
      ph.value = "";
      ph.textContent = text("Bölüme atla...", "Jump to section...");
      tocJump.append(ph);
      spreads.forEach((sp, idx) => {
        const startPage = parseSpreadStartPage(sp, idx);
        const opt = document.createElement("option");
        opt.value = String(startPage);
        opt.textContent = `${sp.page_no || startPage} · ${localized(sp, "title")}`;
        tocJump.append(opt);
      });
    }
  }

  function renderSpread() {
    if (!issue) return;
    const spreads = issue.spreads || [],
      spread = spreads[spreadIndex],
      root = $("#reader-spread"),
      posEl = $("#reader-position"),
      prevBtn = $("[data-reader-prev]"),
      nextBtn = $("[data-reader-next]");
    if (root) root.replaceChildren();
    if (posEl) {
      posEl.textContent = spreads.length
        ? `${spreadIndex + 1} / ${spreads.length} · ${spread?.page_no || ""}`
        : text("İçindekiler henüz eklenmedi.", "Contents are not yet available.");
    }
    if (prevBtn) prevBtn.disabled = spreadIndex <= 0;
    if (nextBtn) nextBtn.disabled = spreadIndex >= spreads.length - 1;
    if (spread && root) {
      const spTitle = localized(spread, "title").trim();
      const spExcerpt = localized(spread, "excerpt").trim();
      const title = document.createElement("h3");
      title.textContent = spTitle;
      root.append(title);
      if (spExcerpt && spExcerpt.toLowerCase() !== spTitle.toLowerCase()) {
        const description = document.createElement("p");
        description.textContent = spExcerpt;
        root.append(description);
      }
      if (spread.linked_content_slug) {
        const a = document.createElement("a");
        a.href =
          "/icerik/" +
          encodeURIComponent(spread.linked_content_slug) +
          (boot.lang === "en" ? "?lang=en" : "");
        a.className = "text-link";
        a.textContent = text("İçeriği oku →", "Read article →");
        root.append(a);
      }
    }
    syncExportLink();
  }

  function setIssueView(mode) {
    fbState.activeView = mode || "toc";
    $$("[data-issue-view]").forEach((btn) => {
      if (!btn.classList.contains("issue-view-tab")) return;
      const on = btn.dataset.issueView === fbState.activeView;
      btn.classList.toggle("is-active", on);
      btn.setAttribute("aria-selected", String(on));
    });
    const dialog = $("#issue-dialog");
    const toc = $("#issue-toc-panel");
    const fb = $("#issue-flipbook-panel");
    const pdf = $("#issue-pdf-panel");
    if (dialog) dialog.classList.toggle("is-flipbook-view", fbState.activeView === "flipbook");
    if (toc) toc.hidden = fbState.activeView !== "toc";
    if (fb) fb.hidden = fbState.activeView !== "flipbook";
    if (pdf) pdf.hidden = fbState.activeView !== "pdf";
    if (fbState.activeView === "flipbook") {
      void renderFlipbook();
    } else {
      syncExportLink();
    }
  }

  function resolveIssueRecord(number, pubType) {
    const list = Array.isArray(boot.issues) ? boot.issues : [];
    const num = Number(number);
    const reqPub = String(pubType || "").trim().toLowerCase();
    if (Number.isFinite(num) && num > 0) {
      if (reqPub) {
        const exact = list.find(
          (i) => i.issue_number === num && issuePubType(i) === reqPub,
        );
        if (exact) return exact;
      }
      const preferredPub = boot.lang === "en" ? "msi_tdr" : "msi";
      const langMatch = list.find(
        (i) => i.issue_number === num && issuePubType(i) === preferredPub,
      );
      if (langMatch) return langMatch;
      const numMatch = list.find((i) => i.issue_number === num);
      if (numMatch) return numMatch;
    }
    return (
      list.find((i) => i.issue_number === boot.featuredIssue) ||
      list[0]
    );
  }

  function openIssue(number, pubType) {
    const resolved = resolveIssueRecord(number, pubType);
    if (!resolved) {
      toast(
        text(
          "Şu anda görüntülenebilecek sayı bulunmuyor.",
          "No issue is available.",
        ),
        "warning",
      );
      return;
    }
    resetFlipbookCache();
    issue = resolved;
    spreadIndex = 0;
    fbState.page = 1;
    fbState.zoom = 1;
    fbState.totalPages = Math.max(1, Number(issue.page_count) || 1);
    fbState.verifiedTotalPages = null;
    const titleEl = $("#issue-dialog-title");
    if (titleEl) titleEl.textContent = localized(issue, "title");
    const metaEl = $("#issue-dialog-meta");
    if (metaEl) {
      const pubTag = issuePubType(issue) === "msi_tdr" ? "MSI TDR" : "MSI";
      metaEl.textContent =
        `${pubTag} · ${text("SAYI", "ISSUE")} ${issue.issue_number} · ${localized(issue, "publish_month")} ${issue.year}`;
    }
    const image = $("#issue-dialog-cover");
    if (image) {
      const cover = String(issue.cover_image || "");
      if (cover) {
        image.src = cover.startsWith("https://")
          ? cover
          : "/" + cover.replace(/^\/+/, "");
      }
      image.alt = localized(issue, "title");
    }
    const rawSummary = localized(issue, "summary").trim();
    const firstSpreadTitle = issue.spreads?.[0]
      ? localized(issue.spreads[0], "title").trim()
      : "";
    const metaFallback =
      boot.lang === "en"
        ? `${issue.page_count || 96} pages · ${issue.dossier_count || 6} special dossiers · ISSN 1306-0309`
        : `${issue.page_count || 96} sayfa · ${issue.dossier_count || 6} özel dosya · ISSN 1306-0309`;
    const summaryEl = $("#issue-dialog-summary");
    if (summaryEl) {
      summaryEl.textContent =
        rawSummary.includes("yeni sayısının içerikleri:") ||
        rawSummary.endsWith("...") ||
        (firstSpreadTitle &&
          rawSummary
            .toLowerCase()
            .includes(firstSpreadTitle.slice(0, 24).toLowerCase()))
          ? metaFallback
          : rawSummary || metaFallback;
    }
    const allRadio = $('input[name="edergi_export_mode"][value="all"]');
    if (allRadio) allRadio.checked = true;
    const pagesInput = $("#edergi-pages-input");
    if (pagesInput) {
      pagesInput.value = "";
      pagesInput.disabled = true;
    }
    const pdfFrame = $("#issue-pdf-frame");
    if (pdfFrame) {
      pdfFrame.src = `/api/v1/edergi/${encodeURIComponent(issue.issue_number)}/download-pdf?publication_type=${encodeURIComponent(issuePubType(issue))}#toolbar=0`;
    }
    populateIssueTocLists();
    setIssueView("toc");
    renderSpread();
    openDialog("issue-dialog");
  }

  $("[data-reader-prev]")?.addEventListener("click", () => {
    if (spreadIndex > 0) {
      spreadIndex--;
      renderSpread();
    }
  });
  $("[data-reader-next]")?.addEventListener("click", () => {
    if (spreadIndex < (issue?.spreads || []).length - 1) {
      spreadIndex++;
      renderSpread();
    }
  });

  $("#issue-dialog")?.addEventListener("close", () => {
    resetFlipbookCache();
  });

  $("[data-fb-prev]")?.addEventListener("click", () => {
    void stepFlipbook(-1);
  });
  $("[data-fb-next]")?.addEventListener("click", () => {
    void stepFlipbook(1);
  });
  $("#edergi-page-input")?.addEventListener("change", (ev) => {
    const total = Math.max(1, fbState.verifiedTotalPages ?? fbState.totalPages ?? 1);
    const raw = Number(ev.target.value);
    const clamped = Math.max(1, Math.min(total, Number.isFinite(raw) ? Math.floor(raw) : 1));
    ev.target.value = String(clamped);
    void jumpToFlipbookPage(clamped);
  });
  $("#edergi-toc-jump")?.addEventListener("change", (ev) => {
    const p = Number(ev.target.value);
    if (p >= 1) void jumpToFlipbookPage(p);
  });
  $$("[data-fb-zoom]").forEach((btn) => {
    btn.addEventListener("click", () => {
      const action = btn.dataset.fbZoom;
      if (action === "in") fbState.zoom = Math.min(2.5, Number((fbState.zoom + 0.25).toFixed(2)));
      else if (action === "out") fbState.zoom = Math.max(0.75, Number((fbState.zoom - 0.25).toFixed(2)));
      else fbState.zoom = 1;
      void renderFlipbook();
    });
  });
  $("#edergi-fullscreen-btn")?.addEventListener("click", async () => {
    const dlg = $("#issue-dialog");
    if (!dlg) return;
    try {
      if (document.fullscreenElement === dlg) {
        await document.exitFullscreen();
        dlg.classList.remove("is-fullscreen");
      } else if (dlg.requestFullscreen) {
        await dlg.requestFullscreen();
        dlg.classList.add("is-fullscreen");
      } else {
        dlg.classList.toggle("is-fullscreen");
      }
    } catch {
      dlg.classList.toggle("is-fullscreen");
    }
  });
  document.addEventListener("fullscreenchange", () => {
    const dlg = $("#issue-dialog");
    if (dlg) {
      dlg.classList.toggle("is-fullscreen", document.fullscreenElement === dlg);
    }
  });
  $$('input[name="edergi_export_mode"]').forEach((rad) => {
    rad.addEventListener("change", syncExportLink);
  });
  $("#edergi-pages-input")?.addEventListener("input", syncExportLink);
  $("#issue-download")?.addEventListener("click", (ev) => {
    const mode = $('input[name="edergi_export_mode"]:checked')?.value || "all";
    if (mode !== "custom") return;
    const pagesInput = $("#edergi-pages-input");
    const fbEl = $("#edergi-export-feedback");
    const check = validateCustomPageSelection(pagesInput?.value || "");
    if (!check.ok) {
      ev.preventDefault();
      if (fbEl) {
        fbEl.classList.add("error");
        fbEl.textContent = check.msg;
      }
      pagesInput?.focus();
    } else {
      syncExportLink();
    }
  });

  const fbStage = $("[data-fb-stage]");
  if (fbStage) {
    fbStage.addEventListener(
      "touchstart",
      (ev) => {
        if (ev.touches.length !== 1) return;
        fbState.touchStartX = ev.touches[0].clientX;
        fbState.touchStartY = ev.touches[0].clientY;
      },
      { passive: true },
    );
    fbStage.addEventListener(
      "touchend",
      (ev) => {
        if (fbState.zoom > 1.05 || ev.changedTouches.length !== 1) return;
        const dx = ev.changedTouches[0].clientX - fbState.touchStartX;
        const dy = ev.changedTouches[0].clientY - fbState.touchStartY;
        if (Math.abs(dx) >= 44 && Math.abs(dx) > Math.abs(dy) * 1.25) {
          void stepFlipbook(dx < 0 ? 1 : -1);
        }
      },
      { passive: true },
    );
  }
  document.addEventListener("keydown", (ev) => {
    const dlg = $("#issue-dialog");
    if (!dlg?.open || fbState.activeView !== "flipbook") return;
    const tag = document.activeElement?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    if (ev.key === "ArrowLeft") {
      ev.preventDefault();
      void stepFlipbook(-1);
    } else if (ev.key === "ArrowRight") {
      ev.preventDefault();
      void stepFlipbook(1);
    } else if (ev.key === "+" || ev.key === "=") {
      ev.preventDefault();
      fbState.zoom = Math.min(2.5, Number((fbState.zoom + 0.25).toFixed(2)));
      void renderFlipbook();
    } else if (ev.key === "-") {
      ev.preventDefault();
      fbState.zoom = Math.max(0.75, Number((fbState.zoom - 0.25).toFixed(2)));
      void renderFlipbook();
    } else if (ev.key === "0") {
      ev.preventDefault();
      fbState.zoom = 1;
      void renderFlipbook();
    } else if (ev.key === "f" || ev.key === "F") {
      ev.preventDefault();
      $("#edergi-fullscreen-btn")?.click();
    }
  });
  matchMedia("(max-width: 768px)").addEventListener("change", () => {
    if ($("#issue-dialog")?.open && fbState.activeView === "flipbook") {
      void renderFlipbook();
    }
  });

  // /e-dergi?issue=N&publication_type=... deep-link auto-open
  try {
    if (location.pathname.includes("/e-dergi")) {
      const qs = new URLSearchParams(location.search);
      const deepIssue = qs.get("issue");
      if (deepIssue) {
        const deepPub = qs.get("publication_type") || "";
        const deepView = qs.get("view") || "";
        openIssue(deepIssue, deepPub);
        if (deepView === "flipbook") {
          setIssueView("flipbook");
        }
      }
    }
  } catch {}

  // Subscription, Publication & Single-Article Dossier Offer Dialog + Online Checkout
  function populateOfferDialog({
    title,
    description,
    priceTry,
    features,
    itemType,
    itemCode,
  }) {
    $("#offer-title").textContent = title;
    $("#offer-description").textContent = description;
    $("#offer-price").textContent = new Intl.NumberFormat("tr-TR", {
      style: "currency",
      currency: "TRY",
      maximumFractionDigits: 0,
    }).format(Number(priceTry || 0));

    const featList = $("#offer-features");
    if (featList) {
      featList.replaceChildren();
      (features || []).forEach((f) => {
        const li = document.createElement("li");
        li.textContent = f;
        featList.append(li);
      });
      featList.hidden = !(features && features.length);
    }

    const checkoutForm = $("#offer-checkout-form");
    if (checkoutForm) {
      checkoutForm.elements.item_type.value = itemType;
      checkoutForm.elements.item_code.value = itemCode;
    }
    const fb = $("#offer-feedback");
    if (fb) {
      fb.classList.remove("error");
      fb.textContent = "";
    }
    const paytrWrap = $("#paytr-iframe-wrap");
    if (paytrWrap) {
      paytrWrap.hidden = true;
      paytrWrap.replaceChildren();
    }
    const contact = $('.site-footer a[href^="mailto:"]');
    if ($("#offer-contact")) {
      $("#offer-contact").href =
        (contact?.href || "mailto:info@msidergisi.com") +
        "?subject=" +
        encodeURIComponent(
          "MSI · " + title + text(" bilgi talebi", " information request"),
        );
    }
    openDialog("offer-dialog");
  }

  async function openOffer(trigger) {
    try {
      const response = await fetch("/api/v1/commerce/catalog");
      if (!response.ok) throw Error();
      const rawCatalog = await response.json();
      const catalog = unwrapPayload(rawCatalog);
      const plan = trigger.dataset.selectPlan;
      const item = plan
        ? (catalog.subscription_plans || []).find((p) => p.code === plan)
        : (catalog.commerce_products || []).find(
            (p) => p.sku === trigger.dataset.selectProduct,
          );
      if (!item) throw Error();
      const title = localized(item, plan ? "name" : "title");
      const description = plan
        ? localized(item, "period_label")
        : localized(item, "description");
      const feats = plan
        ? boot.lang === "en"
          ? item.features_en || []
          : item.features_tr || []
        : item.file_resolution_or_pages
          ? [item.file_resolution_or_pages]
          : [];
      populateOfferDialog({
        title,
        description,
        priceTry: item.price_try,
        features: feats,
        itemType: plan ? "subscription" : "product",
        itemCode: plan ? item.code : item.sku,
      });
    } catch {
      toast(
        text(
          "Yayın bilgileri yüklenemedi. Lütfen tekrar deneyin.",
          "Publication details could not be loaded. Please try again.",
        ),
        "error",
      );
    }
  }

  function openSingleArticleOffer(trigger) {
    const articleSlug = trigger.dataset.buyArticle || "";
    const articleTitle =
      trigger.dataset.articleTitle ||
      $(".article-header h1")?.textContent?.trim() ||
      text("Özel Dosya Erişimi", "Special Dossier Access");
    const articlePrice = Number(trigger.dataset.articlePrice || 0);
    if (!articleSlug || !(articlePrice > 0)) return;
    populateOfferDialog({
      title: articleTitle,
      description: text(
        "Tekil özel dosya / makale kalıcı tam metin dijital erişimi",
        "Permanent full-text digital access to this special dossier",
      ),
      priceTry: articlePrice,
      features: [
        text(
          "Ödeme onayıyla anında tam metin erişimi",
          "Instant full-text unlock upon payment confirmation",
        ),
        text(
          "Kurumsal proforma ve fatura kaydı",
          "Official corporate proforma and invoice record",
        ),
      ],
      itemType: "single_article",
      itemCode: articleSlug,
    });
  }

  $("#offer-checkout-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $("#offer-feedback");
    const paytrWrap = $("#paytr-iframe-wrap");
    const reqBody = Object.fromEntries(new FormData(form));
    reqBody.distance_sales_consent =
      !!form.elements.distance_sales_consent?.checked;
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("Sipariş oluşturuluyor…", "Creating order…");
    try {
      const response = await fetch("/api/v1/subscriptions/checkout", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(reqBody),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text("Sipariş oluşturulamadı.", "Order could not be created."),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const iframeUrl = safeIframeUrl(
        resPayload.paytr_iframe_url ||
          (resPayload.iframe_token
            ? `https://www.paytr.com/odeme/guvenli/${encodeURIComponent(resPayload.iframe_token)}`
            : ""),
      );
      if (iframeUrl && paytrWrap) {
        const iframe = document.createElement("iframe");
        iframe.src = iframeUrl;
        iframe.title = text("PayTR Güvenli Ödeme", "PayTR Secure Payment");
        paytrWrap.replaceChildren(iframe);
        paytrWrap.hidden = false;
      }
      const invNo = resPayload.invoice_no || resPayload.invoice?.invoice_no || "";
      const msg = extractMessage(
        data,
        resPayload,
        text(
          `Siparişiniz kaydedildi (${invNo}).`,
          `Your order has been registered (${invNo}).`,
        ),
      );
      if (fb)
        fb.textContent =
          invNo && !msg.includes(invNo) ? `${msg} [${invNo}]` : msg;
      toast(
        invNo
          ? text(
              `Sipariş kaydı oluşturuldu: ${invNo}`,
              `Order created: ${invNo}`,
            )
          : msg,
      );
    } catch (err) {
      fb?.classList.add("error");
      if (fb)
        fb.textContent =
          err.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred.");
    } finally {
      btn.disabled = false;
    }
  });

  let authStatePromise = null;
  function checkAuthState() {
    if (typeof boot.authenticated === "boolean") {
      return Promise.resolve(boot.authenticated);
    }
    if ($(".account-welcome") || $('.account-link[href="#modal-login"]')) {
      boot.authenticated = false;
      return Promise.resolve(false);
    }
    if (!authStatePromise) {
      authStatePromise = fetch("/api/v1/health", { credentials: "same-origin" })
        .then((r) => (r.ok ? r.json() : null))
        .then((data) => {
          if (data && typeof data.authenticated === "boolean") {
            boot.authenticated = data.authenticated;
            $$(".account-link").forEach((link) => {
              link.dataset.authState = data.authenticated ? "authenticated" : "guest";
            });
            return data.authenticated;
          }
          return null;
        })
        .catch(() => null);
    }
    return authStatePromise;
  }
  if (!inIframe && $("#auth-dialog") && $(".account-link")) {
    void checkAuthState();
  }

  document.addEventListener("click", (e) => {
    const buyArticle = e.target.closest("[data-buy-article]");
    if (buyArticle) {
      e.preventDefault();
      openSingleArticleOffer(buyArticle);
      return;
    }
    const offer = e.target.closest("[data-select-plan],[data-select-product]");
    if (offer) {
      e.preventDefault();
      openOffer(offer);
      return;
    }
    const viewBtn = e.target.closest("[data-auth-view]");
    if (viewBtn) {
      e.preventDefault();
      authView(viewBtn.dataset.authView);
      return;
    }
    const copyBackup = e.target.closest("[data-copy-backup-codes]");
    if (copyBackup) {
      e.preventDefault();
      const codes = $$("#totp-backup-codes-list code")
        .map((el) => el.textContent.trim())
        .filter(Boolean)
        .join("\n");
      if (codes) {
        navigator.clipboard
          ?.writeText(codes)
          .then(() =>
            toast(
              text(
                "Yedek kurtarma kodları panoya kopyalandı.",
                "Backup recovery codes copied to clipboard.",
              ),
              "success",
            ),
          )
          .catch(() =>
            toast(
              text(
                "Kodları seçerek kopyalayabilirsiniz.",
                "Select the codes to copy manually.",
              ),
              "info",
            ),
          );
      }
      return;
    }
    if (e.target.closest("[data-reload-page]")) {
      e.preventDefault();
      location.reload();
      return;
    }
    const close = e.target.closest("[data-close-dialog]");
    if (close) {
      close.closest("dialog")?.close();
      return;
    }
    const trigger = e.target.closest("[data-open-dialog]");
    if (trigger) {
      openDialog(trigger.dataset.openDialog);
      return;
    }
    const issueViewTab = e.target.closest("[data-issue-view]");
    if (issueViewTab) {
      setIssueView(issueViewTab.dataset.issueView || "toc");
      return;
    }
    const magazine = e.target.closest(
      '[data-open-issue], [href="#modal-magazine"]',
    );
    if (magazine) {
      e.preventDefault();
      openIssue(magazine.dataset.openIssue, magazine.dataset.publicationType || "");
      return;
    }
    if (e.target.closest('[href="#modal-login"]')) {
      e.preventDefault();
      authView("login");
      openDialog("auth-dialog");
      return;
    }
    const acctLink = e.target.closest(".account-link");
    if (
      acctLink &&
      !e.metaKey &&
      !e.ctrlKey &&
      !e.shiftKey &&
      !e.altKey &&
      e.button === 0 &&
      acctLink.getAttribute("href") !== "#modal-login" &&
      $("#auth-dialog")
    ) {
      if (boot.authenticated === false) {
        e.preventDefault();
        authView("login");
        openDialog("auth-dialog");
        return;
      }
      if (typeof boot.authenticated !== "boolean") {
        e.preventDefault();
        const targetHref = acctLink.href;
        checkAuthState().then((authed) => {
          if (authed === false) {
            authView("login");
            openDialog("auth-dialog");
          } else if (targetHref) {
            location.href = targetHref;
          }
        });
        return;
      }
    }
    if (e.target.closest('[href="#modal-reklam"]')) {
      e.preventDefault();
      location.href = boot.lang === "en" ? "/kurumsal/iletisim?lang=en" : "/kurumsal/iletisim";
    }
  });

  // Populate SSB interest tags in registration modal & detect .gov.tr/.tsk.tr
  const ssbRoot = $("#register-ssb-tags");
  if (ssbRoot && Array.isArray(boot.tags)) {
    boot.tags.forEach((tag) => {
      const label = document.createElement("label");
      label.className = "ssb-chip";
      const input = document.createElement("input");
      input.type = "checkbox";
      input.value = tag.code;
      const span = document.createElement("span");
      span.textContent = localized(tag, "name");
      label.append(input, span);
      ssbRoot.append(label);
    });
  }
  const regFormEl = $("#register-form");
  const regPills = $$("[data-reg-jump]", regFormEl || document);
  function syncRegProgress(activeStep) {
    if (!regFormEl || !regPills.length) return;
    const step1Done = Boolean(
      regFormEl.elements.full_name?.value?.trim() &&
        regFormEl.elements.email?.value?.trim() &&
        (regFormEl.elements.password?.value || "").length >= 8,
    );
    const step2Done = Boolean(regFormEl.elements.organization_name?.value?.trim());
    const step3Done = $$("#register-ssb-tags input:checked", regFormEl).length > 0;
    const step4Done = Boolean(
      regFormEl.elements.kvkk_consent?.checked && regFormEl.elements.tos_accepted?.checked,
    );
    const doneMap = { 1: step1Done, 2: step2Done, 3: step3Done, 4: step4Done };
    regPills.forEach((pill) => {
      const n = Number(pill.dataset.regJump || "1");
      if (activeStep) pill.classList.toggle("is-active", n === Number(activeStep));
      pill.classList.toggle("is-complete", Boolean(doneMap[n]));
    });
  }
  regPills.forEach((pill) => {
    pill.addEventListener("click", () => {
      const step = pill.dataset.regJump;
      const section = $(`[data-reg-step="${step}"]`, regFormEl || document);
      if (!section) return;
      syncRegProgress(step);
      section.scrollIntoView({ behavior: reducedMotion ? "auto" : "smooth", block: "nearest" });
      const firstInput = $("input:not([type=hidden]), select", section);
      firstInput?.focus?.();
    });
  });
  regFormEl?.addEventListener("focusin", (e) => {
    const sec = e.target.closest("[data-reg-step]");
    if (sec) syncRegProgress(sec.dataset.regStep);
  });
  regFormEl?.addEventListener("input", () => syncRegProgress());
  regFormEl?.addEventListener("change", () => syncRegProgress());
  const regEmail = $('#register-form [name="email"]');
  const govHint = $("#gov-domain-hint");
  regEmail?.addEventListener("input", () => {
    const val = (regEmail.value || "").trim().toLowerCase();
    const isGov = /(\.gov\.tr|\.tsk\.tr|\.edu\.tr|\.pol\.tr)$/.test(val);
    if (govHint) govHint.hidden = !isGov;
  });

  let currentAuthView = "login";
  function authView(name) {
    const dialogOpen = Boolean($("#auth-dialog")?.open);
    const viewChanged = currentAuthView !== name;
    const tabBar = $("#auth-dialog .auth-tabs");
    const isTabView = name === "login" || name === "register";
    if (tabBar) tabBar.hidden = !isTabView;
    $$("[data-auth-tab]").forEach((t) => {
      const active = t.dataset.authTab === name;
      t.setAttribute("aria-selected", String(active));
      t.tabIndex = active ? 0 : -1;
    });
    const loginForm = $("#login-form");
    const regForm = $("#register-form");
    const forgotForm = $("#forgot-password-form");
    const totpForm = $("#totp-login-form");
    if (loginForm) loginForm.hidden = name !== "login";
    if (regForm) regForm.hidden = name !== "register";
    if (forgotForm) {
      forgotForm.hidden = name !== "forgot";
      if (
        name === "forgot" &&
        loginForm?.elements?.email?.value &&
        !forgotForm.elements.email.value
      ) {
        forgotForm.elements.email.value = loginForm.elements.email.value;
      }
    }
    if (totpForm) totpForm.hidden = name !== "totp";
    const authTitle = $("#auth-title");
    if (authTitle) {
      authTitle.textContent =
        name === "forgot"
          ? text("Şifremi unuttum", "Reset password")
          : name === "totp"
            ? text("İki Aşamalı Doğrulama", "Two-Factor Verification")
            : name === "register"
              ? text("Hesap oluşturun", "Create account")
              : text("Hoş geldiniz", "Welcome");
    }
    const targetForm =
      name === "register"
        ? regForm
        : name === "forgot"
          ? forgotForm
          : name === "totp"
            ? totpForm
            : loginForm;
    if (dialogOpen && viewChanged && targetForm) {
      animateAuthSurface(targetForm, { opacity: 0, y: 8 }, 0.22);
    }
    currentAuthView = name;
  }
  const authTab = authView;

  $$("[data-auth-tab]").forEach((t) => {
    t.addEventListener("click", () => authTab(t.dataset.authTab));
    t.addEventListener("keydown", (e) => {
      if (["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) {
        e.preventDefault();
        const next = t.dataset.authTab === "login" ? "register" : "login";
        authTab(next);
        $(`[data-auth-tab="${next}"]`).focus();
      }
    });
  });

  function completeAuthenticatedLogin(msgText) {
    boot.authenticated = true;
    const accountTarget = boot.lang === "en" ? "/hesabim?lang=en" : "/hesabim";
    $$(".account-link").forEach((link) => {
      link.href = accountTarget;
      link.dataset.authState = "authenticated";
    });
    toast(msgText, "success");
    $("#auth-dialog")?.close();
    const next = safeRelativePath(new URL(location.href).searchParams.get("next"));
    if (next) {
      location.href = next;
    } else if (
      location.pathname === "/hesabim" ||
      location.pathname.startsWith("/icerik/") ||
      location.pathname === "/e-dergi"
    ) {
      location.reload();
    }
  }

  // Auto-open auth dialog when redirected with ?next=/... while signed out
  (() => {
    const nextParam = safeRelativePath(
      new URL(location.href).searchParams.get("next"),
    );
    if (nextParam && !inIframe) {
      checkAuthState().then((authed) => {
        if (authed === false) {
          authView("login");
          openDialog("auth-dialog");
        }
      });
    }
  })();

  // ── Anti-Spam: Initialize form timing tokens when forms become visible ──
  function initFormTimingToken(form) {
    if (!form) return;
    const tsField = form.querySelector("[data-form-timing]");
    if (tsField && !tsField.value) {
      tsField.value = String(Math.floor(Date.now() / 1000));
    }
  }
  // Set timing tokens on all auth & newsletter forms at boot and ensure initialized on view switch
  [
    "#login-form",
    "#register-form",
    "#forgot-password-form",
    "#totp-login-form",
    "#public-newsletter-form",
  ].forEach((sel) => initFormTimingToken($(sel)));
  const _origAuthView = authView;
  authView = function (name) {
    _origAuthView(name);
    const formMap = { login: "#login-form", register: "#register-form", forgot: "#forgot-password-form", totp: "#totp-login-form" };
    initFormTimingToken($(formMap[name]));
  };

  ["login", "register"].forEach((kind) =>
    $("#" + kind + "-form")?.addEventListener("submit", async (e) => {
      e.preventDefault();
      const form = e.currentTarget,
        button = $("[type=submit]", form),
        feedback = $(".form-feedback", form);
      const reqPayload = Object.fromEntries(new FormData(form));
      // Anti-spam: ensure honeypot is string and timing field is numeric when present
      if (!reqPayload.website_url) reqPayload.website_url = "";
      if (reqPayload._form_ts) {
        reqPayload._form_ts = Number(reqPayload._form_ts) || undefined;
      } else {
        delete reqPayload._form_ts;
      }
      if (kind === "register") {
        const kvkkInput = form.elements.kvkk_consent;
        const tosInput = form.elements.tos_accepted;
        const emailInput = form.elements.email_consent;
        if (tosInput && !tosInput.checked) {
          feedback.classList.add("error");
          feedback.textContent = text(
            "Devam etmek için Üyelik ve Kullanım Koşulları'nı onaylamanız gerekmektedir.",
            "Please accept the Terms of Use to create your account.",
          );
          tosInput.focus();
          return;
        }
        reqPayload.kvkk_consent = !!kvkkInput?.checked;
        reqPayload.kvkk_acknowledged = !!kvkkInput?.checked;
        reqPayload.tos_accepted = tosInput
          ? !!tosInput.checked
          : !!kvkkInput?.checked;
        reqPayload.tos_version = String(
          form.elements.tos_version?.value ||
            form.dataset.tosVersion ||
            "2026-01",
        );
        reqPayload.email_consent = !!emailInput?.checked;
        reqPayload.ssb_interests = $$("#register-ssb-tags input:checked").map(
          (i) => i.value,
        );
      }
      button.disabled = true;
      form.setAttribute("aria-busy", "true");
      feedback.classList.remove("error");
      feedback.textContent = text("İşleniyor…", "Processing…");
      try {
        const response = await fetch("/api/v1/auth/" + kind, {
          method: "POST",
          headers: csrfHeaders({ "Content-Type": "application/json" }),
          credentials: "same-origin",
          body: JSON.stringify(reqPayload),
        });
        const data = await response.json();
        if (!response.ok)
          throw new Error(
            extractError(
              data,
              text("İşlem tamamlanamadı.", "Unable to complete request."),
            ),
          );
        const resPayload = unwrapPayload(data);
        if (
          kind === "login" &&
          (resPayload.requires_totp || data.requires_totp)
        ) {
          const tempToken = resPayload.temp_token || data.temp_token || "";
          const totpForm = $("#totp-login-form");
          if (totpForm) {
            totpForm.elements.temp_token.value = tempToken;
            totpForm.elements.code.value = "";
            const totpFb = $(".form-feedback", totpForm);
            if (totpFb) {
              totpFb.classList.remove("error");
              totpFb.textContent = "";
            }
            feedback.textContent = "";
            authView("totp");
            totpForm.elements.code.focus();
            return;
          }
        }
        feedback.textContent =
          kind === "login"
            ? extractMessage(
                data,
                resPayload,
                text("Giriş başarılı.", "Signed in."),
              )
            : extractMessage(
                data,
                resPayload,
                text(
                  "Kaydınız oluşturuldu. Giriş yapabilirsiniz.",
                  "Account created. You can sign in.",
                ),
              );
        if (kind === "login") {
          completeAuthenticatedLogin(feedback.textContent);
        } else {
          authView("login");
          $("#login-form").elements.email.value = reqPayload.email;
          $(".form-feedback", $("#login-form")).textContent =
            feedback.textContent;
        }
      } catch (error) {
        feedback.classList.add("error");
        feedback.textContent =
          error.message ||
          text(
            "Bağlantı kurulamadı. Tekrar deneyin.",
            "Connection failed. Try again.",
          );
      } finally {
        button.disabled = false;
        form.removeAttribute("aria-busy");
      }
    }),
  );

  $("#totp-login-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const button = $("[type=submit]", form);
    const feedback = $(".form-feedback", form);
    const tempToken = String(form.elements.temp_token?.value || "").trim();
    const code = String(form.elements.code?.value || "").trim();
    button.disabled = true;
    form.setAttribute("aria-busy", "true");
    feedback?.classList.remove("error");
    if (feedback)
      feedback.textContent = text("Doğrulanıyor…", "Verifying code…");
    try {
      const response = await fetch("/api/v1/auth/totp/login", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({ temp_token: tempToken, code }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Doğrulama kodu geçersiz veya süresi dolmuş.",
              "Verification code is invalid or expired.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text("Doğrulama başarılı. Giriş yapıldı.", "Verified and signed in."),
      );
      if (feedback) feedback.textContent = msg;
      completeAuthenticatedLogin(msg);
    } catch (err) {
      feedback?.classList.add("error");
      if (feedback)
        feedback.textContent =
          err.message ||
          text("Doğrulama tamamlanamadı.", "Verification failed.");
    } finally {
      button.disabled = false;
      form.removeAttribute("aria-busy");
    }
  });

  $("#forgot-password-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const button = $("[type=submit]", form);
    const feedback = $(".form-feedback", form);
    const email = String(form.elements.email?.value || "").trim();
    button.disabled = true;
    form.setAttribute("aria-busy", "true");
    feedback?.classList.remove("error");
    if (feedback)
      feedback.textContent = text(
        "Bağlantı gönderiliyor…",
        "Sending reset link…",
      );
    try {
      // Anti-spam: include honeypot and timing fields
      const rawForgotTs = Number(form.elements._form_ts?.value);
      const forgotPayload = {
        email,
        website_url: String(form.elements.website_url?.value || ""),
        ...(rawForgotTs > 0 ? { _form_ts: rawForgotTs } : {}),
      };
      const response = await fetch("/api/v1/auth/forgot-password", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(forgotPayload),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Şifre sıfırlama talebi gönderilemedi.",
              "Unable to submit password reset request.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "E-posta adresiniz kayıtlıysa şifre sıfırlama bağlantısı gönderilmiştir.",
          "If this email is registered, a password reset link has been sent.",
        ),
      );
      if (feedback) feedback.textContent = msg;
      toast(msg);
    } catch (err) {
      feedback?.classList.add("error");
      if (feedback)
        feedback.textContent =
          err.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred.");
    } finally {
      button.disabled = false;
      form.removeAttribute("aria-busy");
    }
  });

  // Account Verification, Profile, 2FA TOTP & Reset-Password Delegated Handlers
  $("#account-resend-verify")?.addEventListener("click", async (e) => {
    const btn = e.currentTarget;
    const fb = $("#account-verify-feedback");
    const email = btn.dataset.userEmail || "";
    btn.disabled = true;
    if (fb) fb.textContent = text("Gönderiliyor…", "Sending…");
    try {
      const response = await fetch("/api/v1/auth/resend-verification", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({ email }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Doğrulama e-postası gönderilemedi.",
              "Verification email could not be sent.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "Doğrulama e-postası adresinize tekrar gönderildi.",
          "Verification email has been resent.",
        ),
      );
      if (fb) fb.textContent = msg;
      toast(msg, "success");
    } catch (err) {
      if (fb) fb.textContent = err.message;
      toast(err.message, "error");
    } finally {
      btn.disabled = false;
    }
  });

  $("#account-profile-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $(".form-feedback", form);
    const fd = new FormData(form);
    const reqBody = {
      full_name: String(fd.get("full_name") || "").trim(),
      job_title: String(fd.get("job_title") || "").trim(),
      organization_name: String(fd.get("organization_name") || "").trim(),
      organization_category: String(
        fd.get("organization_category") || "",
      ).trim(),
      ssb_interests: $$('input[name="ssb_interests"]:checked', form).map(
        (i) => i.value,
      ),
      email_consent: !!form.elements.email_consent?.checked,
      marketing_emails_opted_in: !!form.elements.email_consent?.checked,
    };
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("Kaydediliyor…", "Saving…");
    try {
      const response = await fetch("/api/v1/account/profile", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(reqBody),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text("Profil güncellenemedi.", "Profile could not be updated."),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "Profiliniz ve SSB ilgi alanlarınız güncellendi.",
          "Your profile and SSB interests have been updated.",
        ),
      );
      if (fb) fb.textContent = msg;
      toast(msg);
      setTimeout(() => location.reload(), 800);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
    } finally {
      btn.disabled = false;
    }
  });

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
    if (fb)
      fb.textContent = text(
        "Güvenli 2FA anahtarı üretiliyor…",
        "Generating secure 2FA secret…",
      );
    try {
      const response = await fetch("/api/v1/auth/totp/setup", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({}),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "2FA kurulumu başlatılamadı.",
              "2FA setup could not be started.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const qrRaw =
        resPayload.qr_svg ||
        resPayload.qr_code_svg ||
        resPayload.qr_code ||
        resPayload.svg ||
        "";
      const secret =
        resPayload.secret ||
        resPayload.totp_secret ||
        resPayload.manual_entry_key ||
        "";
      const qrImg = $("#account-totp-qr");
      if (qrImg && qrRaw) {
        qrImg.src = qrRaw.startsWith("data:image/")
          ? qrRaw
          : "data:image/svg+xml;utf8," + encodeURIComponent(qrRaw);
      }
      const secretCodeEl = $("#account-totp-secret");
      if (secretCodeEl) secretCodeEl.textContent = secret;
      const secretInput = $("#account-totp-secret-input");
      if (secretInput) secretInput.value = secret;
      if (Array.isArray(resPayload.backup_codes) && resPayload.backup_codes.length) {
        renderBackupCodes(resPayload.backup_codes);
      }
      if (panel) panel.hidden = false;
      btn.hidden = true;
      if (fb) fb.textContent = "";
    } catch (err) {
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
      btn.disabled = false;
    }
  });

  $("#account-totp-verify-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $(".form-feedback", form);
    const code = String(form.elements.code?.value || "").trim();
    const secret = String(form.elements.secret?.value || "").trim();
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("Doğrulanıyor…", "Verifying…");
    try {
      const response = await fetch("/api/v1/auth/totp/verify", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(secret ? { code, secret } : { code }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Doğrulama kodu hatalı. Lütfen tekrar deneyin.",
              "Invalid verification code. Please try again.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "İki aşamalı doğrulama (2FA) etkinleştirildi.",
          "Two-factor authentication (2FA) is now enabled.",
        ),
      );
      if (fb) fb.textContent = msg;
      toast(msg);
      const shownBackup = renderBackupCodes(resPayload.backup_codes);
      if (shownBackup) {
        form.hidden = true;
      } else {
        setTimeout(() => location.reload(), 900);
      }
    } catch (err) {
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
    } finally {
      btn.disabled = false;
    }
  });

  $("#account-totp-disable-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $(".form-feedback", form);
    const password = String(form.elements.password?.value || "");
    const code = String(form.elements.code?.value || "").trim();
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("İşleniyor…", "Processing…");
    try {
      const response = await fetch("/api/v1/auth/totp/disable", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({ password, code }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "2FA devre dışı bırakılamadı.",
              "2FA could not be disabled.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "İki aşamalı doğrulama devre dışı bırakıldı.",
          "Two-factor authentication has been disabled.",
        ),
      );
      if (fb) fb.textContent = msg;
      toast(msg);
      setTimeout(() => location.reload(), 800);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
    } finally {
      btn.disabled = false;
    }
  });

  const resetPasswordForm = $("#reset-password-form");
  if (resetPasswordForm) {
    animateAuthSurface(
      resetPasswordForm.closest(".reset-password-card"),
      { opacity: 0, y: 10 },
      0.26,
    );
  }
  resetPasswordForm?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const card = form.closest(".reset-password-card");
    const btn = $("[type=submit]", form);
    const fb =
      $(".form-feedback, [data-form-feedback]", form) ||
      $("#reset-password-feedback");
    const token = String(
      form.elements.token?.value ||
        new URL(location.href).searchParams.get("token") ||
        "",
    ).trim();
    const newPassword = String(
      form.elements.new_password?.value || form.elements.password?.value || "",
    );
    const confirmInput =
      form.elements.confirm_password || form.elements.password_confirm;
    if (confirmInput && confirmInput.value !== newPassword) {
      fb?.classList.remove("is-success");
      card?.classList.remove("is-success");
      fb?.classList.add("error");
      const mismatchMsg = text(
        "Yeni şifre ve şifre tekrarı eşleşmiyor.",
        "Passwords do not match.",
      );
      if (fb) fb.textContent = mismatchMsg;
      toast(mismatchMsg, "error");
      return;
    }
    btn.disabled = true;
    fb?.classList.remove("error", "is-success");
    card?.classList.remove("is-success");
    if (fb)
      fb.textContent = text("Şifreniz güncelleniyor…", "Updating password…");
    try {
      const response = await fetch("/api/v1/auth/reset-password", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({
          token,
          new_password: newPassword,
        }),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Şifre sıfırlama bağlantısı geçersiz veya süresi dolmuş.",
              "Password reset link is invalid or expired.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "Şifreniz başarıyla güncellendi. Yeni şifrenizle giriş yapabilirsiniz.",
          "Your password has been updated. You can now sign in.",
        ),
      );
      fb?.classList.add("is-success");
      card?.classList.add("is-success");
      if (fb) fb.textContent = msg;
      toast(msg, "success");
      setTimeout(() => {
        location.href = boot.lang === "en" ? "/hesabim?lang=en" : "/hesabim";
      }, 1400);
    } catch (err) {
      fb?.classList.remove("is-success");
      card?.classList.remove("is-success");
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
    } finally {
      btn.disabled = false;
    }
  });

  $("#account-delete-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $(".form-feedback", form);
    const confirmInput = form.elements.confirm_erasure;
    if (confirmInput && !confirmInput.checked) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent = text(
          "Kalıcı hesap silme işlemini onaylamanız gerekmektedir.",
          "Please confirm permanent account erasure.",
        );
      }
      confirmInput.focus();
      return;
    }
    const password = String(form.elements.password?.value || "");
    if (!password.trim()) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent = text(
          "Devam etmek için mevcut şifrenizi girin.",
          "Please enter your current password to continue.",
        );
      }
      form.elements.password?.focus?.();
      return;
    }
    const confirmed = await msiConfirm({
      eyebrow: text("KVKK HESAP SİLME", "ACCOUNT ERASURE"),
      title: text("Hesabınızı kalıcı olarak silmek istiyor musunuz?", "Permanently delete your account?"),
      message: text(
        "Bu işlem geri alınamaz. Profiliniz, okuma geçmişiniz ve dijital erişim yetkileriniz kalıcı olarak kaldırılacaktır.",
        "This action cannot be undone. Your profile, reading history, and digital access entitlements will be permanently erased.",
      ),
      confirmLabel: text("Hesabı Kalıcı Olarak Sil", "Delete Account Permanently"),
      cancelLabel: text("Vazgeç", "Cancel"),
      danger: true,
    });
    if (!confirmed) return;
    if (btn) btn.disabled = true;
    form.setAttribute("aria-busy", "true");
    fb?.classList.remove("error");
    if (fb) {
      fb.textContent = text(
        "Hesabınız kalıcı olarak siliniyor…",
        "Permanently deleting your account…",
      );
    }
    try {
      const response = await fetch("/api/v1/account/delete", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(password ? { password } : {}),
      });
      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Hesap silme işlemi tamamlanamadı. Şifrenizi kontrol edin.",
              "Account deletion failed. Please check your password.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "Hesabınız KVKK kapsamında kalıcı olarak silindi.",
          "Your account has been permanently deleted.",
        ),
      );
      if (fb) fb.textContent = msg;
      try {
        localStorage.removeItem("msi_active_user");
        // SECURITY FIX: Removed legacy msi_session_token cleanup — sessions are HttpOnly cookies only
      } catch {}
      form.closest("dialog")?.close();
      toast(msg, "success");
      setTimeout(() => {
        location.href = boot.lang === "en" ? "/en" : "/";
      }, 900);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent =
          err?.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred.");
      }
    } finally {
      if (btn) btn.disabled = false;
      form.removeAttribute("aria-busy");
    }
  });

  $("[data-logout]")?.addEventListener("click", async () => {
    try {
      const response = await fetch("/api/v1/auth/logout", {
        method: "POST",
        headers: csrfHeaders({}),
        credentials: "same-origin",
      });
      if (!response.ok) throw Error();
      try {
        localStorage.removeItem("msi_active_user");
        // SECURITY FIX: Removed legacy msi_session_token cleanup — sessions are HttpOnly cookies only
      } catch {}
      location.href = boot.lang === "en" ? "/hesabim?lang=en" : "/hesabim";
    } catch {
      toast(
        text(
          "Çıkış yapılamadı. Tekrar deneyin.",
          "Sign-out failed. Please try again.",
        ),
        "error",
      );
    }
  });
  $("[data-share-article]")?.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(location.origin + location.pathname);
      toast(text("Bağlantı kopyalandı.", "Link copied."), "success");
    } catch {
      toast(
        text(
          "Bağlantıyı tarayıcınızın adres çubuğundan kopyalayabilirsiniz.",
          "Please copy the link from your browser's address bar.",
        ),
        "info",
      );
    }
  });

  // KVKK Opt-In Cookie Consent & First-Party Telemetry Gate
  const cookieBar = $("#cookie-consent-bar");
  const cookieDialog = $("#cookie-dialog");
  const cookieForm = $("#cookie-preferences-form");
  function loadCookieConsent() {
    try {
      const raw = JSON.parse(storage.get("msi_cookie_consent") || "null");
      if (raw && typeof raw === "object") {
        return {
          decided: true,
          necessary: true,
          analytics: !!raw.analytics,
          marketing: !!raw.marketing,
          updated_at: raw.updated_at || "",
        };
      }
    } catch {}
    return {
      decided: false,
      necessary: true,
      analytics: false,
      marketing: false,
      updated_at: "",
    };
  }
  let cookieConsent = loadCookieConsent();
  function syncCookieForm() {
    if (!cookieForm) return;
    if (cookieForm.elements.analytics)
      cookieForm.elements.analytics.checked = !!cookieConsent.analytics;
    if (cookieForm.elements.marketing)
      cookieForm.elements.marketing.checked = !!cookieConsent.marketing;
  }
  syncCookieForm();
  if (cookieBar && !cookieConsent.decided && !inIframe) {
    cookieBar.hidden = false;
  }

  let adObserver = null;
  function attachAdTelemetry() {
    if (!cookieConsent.marketing || adObserver || !("IntersectionObserver" in window))
      return;
    adObserver = new IntersectionObserver(
      (entries) =>
        entries.forEach((e) => {
          if (e.isIntersecting) {
            track("ad_impression", Number(e.target.dataset.adId), {
              el: e.target,
            });
            adObserver.unobserve(e.target);
          }
        }),
      { threshold: 0.5 },
    );
    $$("[data-ad-id]").forEach((el) => {
      if (!el.hidden) adObserver.observe(el);
    });
  }

  function saveCookieConsent(analytics, marketing) {
    cookieConsent = {
      decided: true,
      necessary: true,
      analytics: !!analytics,
      marketing: !!marketing,
      updated_at: new Date().toISOString(),
    };
    storage.set(
      "msi_cookie_consent",
      JSON.stringify({
        necessary: true,
        analytics: cookieConsent.analytics,
        marketing: cookieConsent.marketing,
        updated_at: cookieConsent.updated_at,
      }),
    );
    if (!cookieConsent.analytics && !cookieConsent.marketing) {
      try {
        localStorage.removeItem("msi_session_id");
      } catch {}
    }
    syncCookieForm();
    if (cookieBar) cookieBar.hidden = true;
    if (cookieDialog?.open) cookieDialog.close();
    const deferredPopup = $("#ad-floating-popup");
    if (deferredPopup && deferredPopup.dataset.deferredForCookie === "1") {
      delete deferredPopup.dataset.deferredForCookie;
      const deferredAdId = deferredPopup.dataset.adId || "0";
      let dismissed = false;
      try {
        dismissed = sessionStorage.getItem("msi_dismiss_ad_" + deferredAdId) === "1";
      } catch {}
      if (!dismissed) {
        deferredPopup.hidden = false;
        if (adObserver && cookieConsent.marketing) {
          adObserver.observe(deferredPopup);
        }
      }
    }
    attachAdTelemetry();
    toast(
      text(
        "Çerez ve gizlilik tercihleriniz kaydedildi.",
        "Your cookie and privacy preferences have been saved.",
      ),
    );
  }

  document.addEventListener("click", (e) => {
    const cookieAction = e.target.closest("[data-cookie-accept]");
    if (cookieAction) {
      const mode = cookieAction.dataset.cookieAccept;
      if (mode === "all") saveCookieConsent(true, true);
      else saveCookieConsent(false, false);
    }
  });
  cookieForm?.addEventListener("submit", (e) => {
    e.preventDefault();
    saveCookieConsent(
      !!cookieForm.elements.analytics?.checked,
      !!cookieForm.elements.marketing?.checked,
    );
  });

  // Telemetry & True-Read Analytics Engine (strictly gated on KVKK opt-in consent)
  const start = Date.now();
  let depth = 0,
    videoCompleted = false,
    maxVideoWatchPct = 0,
    sent = false;
  const videoStates = new Map();
  const VIDEO_QUARTILES = [25, 50, 75, 100];

  function getAnalyticsSessionId() {
    let sid = storage.get("msi_session_id");
    if (!sid) {
      sid = crypto.randomUUID?.() || String(Date.now());
      storage.set("msi_session_id", sid);
    }
    return sid;
  }
  const articleRoot = $(
    ".article-page[data-msi-content-slug], article.msi-article-body",
  );
  const slug =
    articleRoot?.dataset?.msiContentSlug ||
    (location.pathname.startsWith("/icerik/")
      ? decodeURIComponent(location.pathname.slice(8))
      : null);

  function firstValidSsbTag(single, csv) {
    const direct = String(single || "").trim();
    if (direct && direct !== "*") return direct;
    return (
      String(csv || "")
        .split(",")
        .map((s) => s.trim())
        .find((s) => s && s !== "*") || ""
    );
  }

  function resolveSsbTag(el, overrides = {}) {
    if (overrides.ssb_tag !== undefined) {
      const explicit = String(overrides.ssb_tag || "").trim();
      return explicit && explicit !== "*" ? explicit : null;
    }
    const scoped = el?.closest?.("[data-ssb-tag], [data-ssb-tags]");
    const scopedTag = firstValidSsbTag(
      scoped?.dataset?.ssbTag,
      scoped?.dataset?.ssbTags,
    );
    if (scopedTag) return scopedTag;
    const rootTag = firstValidSsbTag(
      articleRoot?.dataset?.ssbTag,
      articleRoot?.dataset?.ssbTags,
    );
    return rootTag || null;
  }

  function buildAnalyticsPayload(event, adId, overrides = {}) {
    const seconds = Math.round((Date.now() - start) / 1000);
    const ssbTag = resolveSsbTag(overrides.el, overrides);
    return {
      id: 0,
      session_id: getAnalyticsSessionId(),
      user_id: null,
      organization_category: "",
      organization_name: "",
      event_type: event,
      content_slug:
        overrides.content_slug !== undefined ? overrides.content_slug : slug,
      ad_id: adId || null,
      duration_sec:
        overrides.duration_sec !== undefined ? overrides.duration_sec : seconds,
      scroll_depth_pct: depth,
      video_completed:
        overrides.video_completed !== undefined
          ? overrides.video_completed
          : videoCompleted,
      video_watch_pct:
        overrides.video_watch_pct !== undefined
          ? overrides.video_watch_pct
          : maxVideoWatchPct,
      is_true_read: seconds >= 25 && depth >= 70,
      ssb_tag: ssbTag,
      created_at: "",
    };
  }

  function sendAnalyticsBeacon(payload) {
    try {
      navigator.sendBeacon(
        "/api/v1/analytics/track",
        new Blob([JSON.stringify(payload)], { type: "application/json" }),
      );
    } catch {}
  }

  function track(event, adId, overrides = {}) {
    if (
      (event === "article_read" || event === "video_progress") &&
      !cookieConsent.analytics
    )
      return;
    if (
      (event === "ad_impression" || event === "ad_click") &&
      !cookieConsent.marketing
    )
      return;
    sendAnalyticsBeacon(buildAnalyticsPayload(event, adId, overrides));
  }

  function flushVideoProgress(videoEl, force = false) {
    if (!cookieConsent.analytics) return;
    const st = videoStates.get(videoEl);
    if (!st || st.maxPct <= 0) return;
    if (!force && st.maxPct <= st.lastSentPct) return;
    if (st.maxPct === st.lastSentPct && st.sentFinal) return;
    st.lastSentPct = st.maxPct;
    if (st.maxPct >= 100) st.sentFinal = true;
    sendAnalyticsBeacon(
      buildAnalyticsPayload("video_progress", null, {
        el: videoEl,
        content_slug: st.slug,
        duration_sec: Math.max(0, Math.round(videoEl.currentTime || 0)),
        video_completed: st.maxPct >= 100 || videoEl.ended,
        video_watch_pct: st.maxPct,
      }),
    );
  }

  $$("video[data-msi-video]").forEach((v) => {
    const vSlug = (v.dataset.msiVideo || "").trim() || slug || null;
    videoStates.set(v, {
      slug: vSlug,
      maxPct: 0,
      lastSentPct: 0,
      quartiles: new Set(),
      sentFinal: false,
    });
    v.addEventListener("timeupdate", () => {
      const st = videoStates.get(v);
      if (!st || !Number.isFinite(v.duration) || v.duration <= 0) return;
      const pct = Math.min(
        100,
        Math.max(0, Math.round((v.currentTime / v.duration) * 100)),
      );
      if (pct > st.maxPct) st.maxPct = pct;
      if (st.maxPct > maxVideoWatchPct) maxVideoWatchPct = st.maxPct;
      if (st.maxPct >= 100) videoCompleted = true;
      let hitNewQuartile = false;
      for (const q of VIDEO_QUARTILES) {
        if (st.maxPct >= q && !st.quartiles.has(q)) {
          st.quartiles.add(q);
          hitNewQuartile = true;
        }
      }
      if (hitNewQuartile) {
        flushVideoProgress(v, false);
      }
    });
    v.addEventListener("pause", () => {
      if (!v.ended) flushVideoProgress(v, false);
    });
    v.addEventListener("ended", () => {
      const st = videoStates.get(v);
      videoCompleted = true;
      if (st) {
        st.maxPct = 100;
        maxVideoWatchPct = 100;
        st.quartiles.add(100);
      }
      flushVideoProgress(v, true);
    });
  });

  addEventListener(
    "scroll",
    () => {
      const range = document.documentElement.scrollHeight - innerHeight;
      depth = Math.max(
        depth,
        range > 0 ? Math.min(100, Math.round((scrollY / range) * 100)) : 100,
      );
    },
    { passive: true },
  );
  function flushPageTelemetry() {
    videoStates.forEach((_st, v) => flushVideoProgress(v, false));
    if (slug && !sent) {
      track("article_read");
      sent = true;
    }
  }
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") {
      videoStates.forEach((_st, v) => flushVideoProgress(v, false));
    }
  });
  addEventListener("pagehide", flushPageTelemetry);

  // Personalized Feed Hydration (B4 — silent-safe before M6-B_API)
  async function hydratePersonalizedFeed() {
    const containers = $$("[data-personalized-feed]");
    if (!containers.length) return;
    for (const root of containers) {
      if (root.dataset.hydrated === "true") continue;
      const limit = Number(root.dataset.limit || 6) || 6;
      const band = root.closest("[data-personalized-band], .personalized-band");
      try {
        const res = await fetch(
          `/api/v1/public/personalized?lang=${encodeURIComponent(boot.lang || "tr")}&limit=${limit}`,
          { credentials: "same-origin" },
        );
        if (!res.ok) continue;
        const raw = await res.json().catch(() => null);
        const payload = unwrapPayload(raw);
        const items = Array.isArray(payload?.items)
          ? payload.items
          : Array.isArray(raw?.items)
            ? raw.items
            : Array.isArray(payload)
              ? payload
              : [];
        if (!items.length) continue;
        const frag = document.createDocumentFragment();
        const langQ = boot.lang === "en" ? "?lang=en" : "";
        const minLabel = text("dk okuma", "min read");
        items.slice(0, limit).forEach((item) => {
          const card = document.createElement("article");
          card.className = "story-card";
          if (item.id != null) card.dataset.contentId = String(item.id);
          if (item.content_type) card.dataset.categorySlug = String(item.content_type);
          if (Array.isArray(item.ssb_tags) && item.ssb_tags.length) {
            card.dataset.ssbTags = item.ssb_tags.join(",");
            const primaryTag = card.dataset.ssbTags
              .split(",")
              .map((s) => s.trim())
              .find((s) => s && s !== "*");
            if (primaryTag) card.dataset.ssbTag = primaryTag;
          }
          const href = `/icerik/${encodeURIComponent(item.slug || "")}${langQ}`;
          const title = localized(item, "title") || item.title || "";
          const summary = localized(item, "summary") || item.summary || "";
          const cover = item.cover_image
            ? item.cover_image.startsWith("https://")
              ? item.cover_image
              : "/" + String(item.cover_image).replace(/^\/+/, "")
            : "/images/logo.svg";

          const imgLink = document.createElement("a");
          imgLink.className = "story-image";
          imgLink.href = href;
          imgLink.tabIndex = -1;
          imgLink.setAttribute("aria-hidden", "true");
          const img = document.createElement("img");
          img.src = cover;
          img.alt = title;
          img.loading = "lazy";
          img.decoding = "async";
          imgLink.append(img);

          const copy = document.createElement("div");
          copy.className = "story-copy";
          const eb = document.createElement("div");
          eb.className = "eyebrow";
          eb.textContent = String(
            item.category_label || item.content_type || "MSI",
          ).replace(/-/g, " ");
          const h3 = document.createElement("h3");
          const titleLink = document.createElement("a");
          titleLink.href = href;
          titleLink.textContent = title;
          h3.append(titleLink);
          const p = document.createElement("p");
          p.textContent = summary;
          const meta = document.createElement("div");
          meta.className = "story-meta";
          const timeEl = document.createElement("time");
          const pubDate = String(item.published_at || "").slice(0, 10);
          timeEl.dateTime = pubDate;
          timeEl.textContent = pubDate;
          const minSpan = document.createElement("span");
          minSpan.textContent = `${Math.max(1, Number(item.read_time_min || 3))} ${minLabel}`;
          meta.append(timeEl, minSpan);
          copy.append(eb, h3, p, meta);
          card.append(imgLink, copy);
          frag.append(card);
        });
        root.replaceChildren(frag);
        root.dataset.hydrated = "true";
        if (band) band.hidden = false;
      } catch {
        // Silent no-op before M6-B_API wires /api/v1/public/personalized
      }
    }
  }
  hydratePersonalizedFeed();

  // Public Newsletter Opt-In Handler (B7)
  $("#public-newsletter-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $(".form-feedback", form);
    const email = String(form.elements.email?.value || "").trim();
    const kvkkInput = form.elements.kvkk_consent;
    if (!email) {
      fb?.classList.add("error");
      if (fb)
        fb.textContent = text(
          "Lütfen geçerli bir e-posta adresi girin.",
          "Please enter a valid email address.",
        );
      return;
    }
    if (kvkkInput && !kvkkInput.checked) {
      fb?.classList.add("error");
      if (fb)
        fb.textContent = text(
          "KVKK aydınlatma onayı gereklidir.",
          "Privacy consent is required.",
        );
      kvkkInput.focus();
      return;
    }
    const rawTs = Number(form.elements._form_ts?.value);
    const reqBody = {
      email,
      full_name: String(form.elements.full_name?.value || "").trim() || undefined,
      email_group_code:
        String(form.elements.email_group_code?.value || "general").trim() ||
        "general",
      kvkk_consent: kvkkInput ? !!kvkkInput.checked : true,
      email_consent: true,
      lang: boot.lang || "tr",
      website_url: String(form.elements.website_url?.value || ""),
      ...(rawTs > 0 ? { _form_ts: rawTs } : {}),
    };
    if (btn) btn.disabled = true;
    form.setAttribute("aria-busy", "true");
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("Kaydediliyor…", "Subscribing…");
    try {
      const endpoint =
        form.getAttribute("action") ||
        form.dataset.endpoint ||
        "/api/v1/public/newsletter/subscribe";
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(reqBody),
      });
      const data = await response.json().catch(() => ({}));
      if (
        response.status === 404 ||
        response.status === 405 ||
        response.status === 501
      ) {
        fb?.classList.add("error");
        if (fb) {
          fb.textContent = text(
            "E-bülten servisi şu anda bakımdadır. Hesabım sayfasından e-posta tercihlerinizi yönetebilirsiniz.",
            "Newsletter signup is temporarily unavailable. Please manage email preferences from your account.",
          );
        }
        return;
      }
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Bülten kaydı oluşturulamadı.",
              "Unable to subscribe to the newsletter.",
            ),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "E-bülten aboneliğiniz kaydedildi.",
          "Your newsletter subscription has been saved.",
        ),
      );
      if (fb) fb.textContent = msg;
      form.reset();
      initFormTimingToken(form);
      toast(msg);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent =
          err?.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred.");
      }
    } finally {
      if (btn) btn.disabled = false;
      form.removeAttribute("aria-busy");
    }
  });

  // Multi-Item LocalStorage Cart Drawer (B14)
  const CART_KEY = "msi_cart_items";
  const formatTry = (n) => {
    const num = Number(n);
    return new Intl.NumberFormat("tr-TR", {
      style: "currency",
      currency: "TRY",
      maximumFractionDigits: 0,
    }).format(Number.isFinite(num) && num >= 0 ? num : 0);
  };

  function readCart() {
    try {
      const parsed = JSON.parse(storage.get(CART_KEY) || "[]");
      if (!Array.isArray(parsed)) return [];
      const clean = [];
      for (const x of parsed) {
        if (!x || typeof x !== "object" || typeof x.item_code !== "string") {
          continue;
        }
        const item_code = x.item_code.trim();
        if (!item_code) continue;
        if (typeof x.qty === "boolean") continue;
        const q = Number(x.qty);
        if (!Number.isFinite(q) || q <= 0) continue;
        const p = Number(x.price_try);
        clean.push({
          item_type: String(x.item_type || "product").trim() || "product",
          item_code,
          title: String(x.title || item_code).trim() || item_code,
          price_try: Number.isFinite(p) && p >= 0 ? p : 0,
          qty: Math.max(1, Math.floor(q)),
        });
      }
      return clean;
    } catch {
      return [];
    }
  }
  function writeCart(items) {
    storage.set(CART_KEY, JSON.stringify(items));
    renderCart();
  }
  function clearCartFeedback() {
    const cartFb = $("#cart-feedback");
    if (cartFb) {
      cartFb.classList.remove("error");
      cartFb.textContent = "";
    }
    const formFb = $("#cart-checkout-form .form-feedback");
    if (formFb) {
      formFb.textContent = "";
      formFb.classList.remove("error");
    }
  }
  function isCartOpen() {
    const drawer = $("#cart-drawer");
    if (!drawer) return false;
    return drawer.tagName === "DIALOG"
      ? drawer.open
      : drawer.classList.contains("is-open");
  }
  let cartRestoreFocus = null;
  function setCartOpen(open) {
    const drawer = $("#cart-drawer");
    if (!drawer) return;
    if (drawer.tagName === "DIALOG") {
      if (open && !drawer.open) openDialog("cart-drawer");
      else if (!open && drawer.open) drawer.close();
      return;
    }
    if (open) {
      if (!isCartOpen()) cartRestoreFocus = document.activeElement;
      drawer.hidden = false;
      requestAnimationFrame(() => drawer.classList.add("is-open"));
      drawer.setAttribute("aria-hidden", "false");
      $("#cart-backdrop")?.removeAttribute("hidden");
      syncScrollLock();
      const focusTarget = $("[data-cart-close]", drawer) || drawer;
      if (focusTarget === drawer && !drawer.hasAttribute("tabindex")) {
        drawer.setAttribute("tabindex", "-1");
      }
      focusTarget?.focus?.();
    } else {
      drawer.classList.remove("is-open");
      drawer.hidden = true;
      drawer.setAttribute("aria-hidden", "true");
      $("#cart-backdrop")?.setAttribute("hidden", "");
      syncScrollLock();
      if (cartRestoreFocus && document.contains(cartRestoreFocus)) {
        cartRestoreFocus.focus?.();
      }
      cartRestoreFocus = null;
    }
  }
  function renderCart() {
    const items = readCart();
    const totalCount = items.reduce((s, x) => {
      const q = Number(x.qty);
      return s + (Number.isFinite(q) && q > 0 ? q : 1);
    }, 0);
    const totalAmount = items.reduce((s, x) => {
      const p = Number(x.price_try);
      const q = Number(x.qty);
      return (
        s +
        (Number.isFinite(p) && p >= 0 ? p : 0) *
          (Number.isFinite(q) && q > 0 ? q : 1)
      );
    }, 0);
    $$("[data-cart-count]").forEach((el) => {
      el.textContent = String(totalCount);
      el.hidden = totalCount === 0;
    });
    const list = $("#cart-items-list");
    const emptyEl = $("#cart-empty-state");
    const subtotalEl = $("#cart-subtotal");
    const checkoutForm = $("#cart-checkout-form");
    if (subtotalEl) subtotalEl.textContent = formatTry(totalAmount);
    if (emptyEl) emptyEl.hidden = items.length > 0;
    if (checkoutForm) checkoutForm.hidden = items.length === 0;
    if (!list) return;
    list.replaceChildren();
    items.forEach((item) => {
      const row = document.createElement("div");
      row.className = "cart-item-row";
      const info = document.createElement("div");
      info.className = "cart-item-info";
      const title = document.createElement("strong");
      title.textContent = item.title || item.item_code;
      const meta = document.createElement("span");
      meta.className = "cart-item-price";
      meta.textContent = `${formatTry(item.price_try)} × ${item.qty}`;
      info.append(title, meta);

      const itemType = String(item.item_type || "").trim();
      const ctrls = document.createElement("div");
      ctrls.className = "cart-qty-control";
      const dec = document.createElement("button");
      dec.type = "button";
      dec.className = "cart-qty-btn";
      dec.textContent = "−";
      dec.setAttribute("aria-label", text("Azalt", "Decrease quantity"));
      dec.dataset.cartQty = "dec";
      dec.dataset.itemCode = item.item_code;
      if (itemType) dec.dataset.itemType = itemType;

      const qtyVal = document.createElement("span");
      qtyVal.textContent = String(item.qty);

      const inc = document.createElement("button");
      inc.type = "button";
      inc.className = "cart-qty-btn";
      inc.textContent = "+";
      inc.setAttribute("aria-label", text("Artır", "Increase quantity"));
      inc.dataset.cartQty = "inc";
      inc.dataset.itemCode = item.item_code;
      if (itemType) inc.dataset.itemType = itemType;

      const rm = document.createElement("button");
      rm.type = "button";
      rm.className = "cart-remove-btn";
      rm.textContent = "×";
      rm.setAttribute("aria-label", text("Sepetten çıkar", "Remove item"));
      rm.dataset.cartRemove = item.item_code;
      if (itemType) rm.dataset.itemType = itemType;

      ctrls.append(dec, qtyVal, inc, rm);
      row.append(info, ctrls);
      list.append(row);
    });
  }
  renderCart();

  document.addEventListener("click", (e) => {
    const addBtn = e.target.closest("[data-add-to-cart]");
    if (addBtn) {
      e.preventDefault();
      const item_code = String(addBtn.dataset.itemCode || "").trim();
      if (!item_code) return;
      const item_type = String(addBtn.dataset.itemType || "product").trim();
      const title = String(addBtn.dataset.itemTitle || item_code).trim();
      const rawPrice = Number(addBtn.dataset.itemPrice);
      const price_try =
        Number.isFinite(rawPrice) && rawPrice >= 0 ? rawPrice : 0;
      const items = readCart();
      const existing = items.find(
        (x) => x.item_code === item_code && x.item_type === item_type,
      );
      if (existing) {
        const curQty = Number(existing.qty);
        existing.qty =
          (Number.isFinite(curQty) && curQty > 0 ? Math.floor(curQty) : 0) + 1;
      } else {
        items.push({ item_type, item_code, title, price_try, qty: 1 });
      }
      clearCartFeedback();
      writeCart(items);
      toast(text(`${title} sepete eklendi.`, `${title} added to cart.`), "success");
      setCartOpen(true);
      return;
    }
    const qtyBtn = e.target.closest("[data-cart-qty]");
    if (qtyBtn) {
      const code = qtyBtn.dataset.itemCode;
      const type = String(qtyBtn.dataset.itemType || "").trim();
      const delta = qtyBtn.dataset.cartQty === "inc" ? 1 : -1;
      const matches = (x) =>
        x.item_code === code && (!type || x.item_type === type);
      const items = readCart()
        .map((x) => {
          if (!matches(x)) return x;
          const curQty = Number(x.qty);
          const nextQty =
            (Number.isFinite(curQty) && curQty > 0 ? Math.floor(curQty) : 0) +
            delta;
          return { ...x, qty: nextQty };
        })
        .filter((x) => Number.isFinite(x.qty) && x.qty > 0);
      clearCartFeedback();
      writeCart(items);
      return;
    }
    const rmBtn = e.target.closest("[data-cart-remove]");
    if (rmBtn) {
      const code = rmBtn.dataset.cartRemove;
      const type = String(rmBtn.dataset.itemType || "").trim();
      clearCartFeedback();
      writeCart(
        readCart().filter(
          (x) => !(x.item_code === code && (!type || x.item_type === type)),
        ),
      );
      return;
    }
    if (e.target.closest("[data-cart-toggle]")) {
      e.preventDefault();
      setCartOpen(!isCartOpen());
      return;
    }
    if (
      e.target.closest("[data-cart-close]") ||
      e.target.id === "cart-backdrop"
    ) {
      setCartOpen(false);
    }
  });
  document.addEventListener("keydown", (e) => {
    if (!isCartOpen()) return;
    if (e.key === "Escape") {
      setCartOpen(false);
      return;
    }
    if (e.key === "Tab") {
      const drawer = $("#cart-drawer");
      if (!drawer || drawer.tagName === "DIALOG") return;
      const focusables = $$(
        'a[href], button:not([disabled]), input:not([type="hidden"]):not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        drawer,
      ).filter((el) => {
        if (el.closest("[hidden]")) return false;
        const st = getComputedStyle(el);
        return st.display !== "none" && st.visibility !== "hidden" && el.getClientRects().length > 0;
      });
      if (!focusables.length) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const cur = document.activeElement;
      if (!drawer.contains(cur)) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
        return;
      }
      if (e.shiftKey) {
        if (cur === first || cur === drawer) {
          e.preventDefault();
          last.focus();
        }
      } else if (cur === last) {
        e.preventDefault();
        first.focus();
      }
    }
  });

  $("#cart-checkout-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const items = readCart();
    const fb = $(".form-feedback", form);
    const cartFb = $("#cart-feedback");
    const btn = $("[type=submit]", form);
    const paytrWrap = $("#cart-paytr-wrap");
    if (!items.length) return;
    const fd = Object.fromEntries(new FormData(form));
    const reqBody = {
      user_email: String(fd.email || fd.user_email || "").trim(),
      email: String(fd.email || fd.user_email || "").trim(),
      full_name: String(fd.full_name || "").trim(),
      organization_name: String(fd.organization_name || "").trim(),
      distance_sales_consent: !!form.elements.distance_sales_consent?.checked,
      items: items.map((x) => ({
        item_type: x.item_type,
        item_code: x.item_code,
        qty: x.qty,
      })),
    };
    if (btn) btn.disabled = true;
    fb?.classList.remove("error");
    clearCartFeedback();
    if (fb)
      fb.textContent = text(
        "Sepet siparişi oluşturuluyor…",
        "Creating cart order…",
      );
    try {
      const res = await fetch("/api/v1/commerce/cart-checkout", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify(reqBody),
      });
      const data = await res.json().catch(() => ({}));
      if (res.status === 404 || res.status === 405 || res.status === 501) {
        fb?.classList.add("error");
        if (fb) {
          fb.textContent = text(
            "Çoklu sepet ödemesi yakında aktif olacaktır. Tekil ürünler için 'Yayını incele' butonunu kullanabilirsiniz.",
            "Multi-item cart checkout is coming soon. Please use single-item checkout on individual items.",
          );
        }
        return;
      }
      if (!res.ok) {
        throw new Error(
          extractError(
            data,
            text("Sipariş oluşturulamadı.", "Order could not be created."),
          ),
        );
      }
      const resPayload = unwrapPayload(data);
      const iframeUrl = safeIframeUrl(
        resPayload.paytr_iframe_url ||
          (resPayload.iframe_token
            ? `https://www.paytr.com/odeme/guvenli/${encodeURIComponent(resPayload.iframe_token)}`
            : ""),
      );
      if (iframeUrl && paytrWrap) {
        const iframe = document.createElement("iframe");
        iframe.src = iframeUrl;
        iframe.title = text("PayTR Güvenli Ödeme", "PayTR Secure Payment");
        paytrWrap.replaceChildren(iframe);
        paytrWrap.hidden = false;
      }
      writeCart([]);
      const msg = extractMessage(
        data,
        resPayload,
        text(
          "Sepet siparişiniz kaydedildi.",
          "Your cart order has been registered.",
        ),
      );
      if (fb) fb.textContent = msg;
      if (cartFb) {
        cartFb.classList.remove("error");
        cartFb.textContent = msg;
      }
      toast(msg);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) {
        fb.textContent =
          err?.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred.");
      }
    } finally {
      if (btn) btn.disabled = false;
    }
  });
  const floatingPopup = $("#ad-floating-popup");
  if (floatingPopup) {
    const isUtilityPath = [
      "/hesabim",
      "/sifre-sifirla",
      "/sifre-yenile",
      "/hesap-dogrula",
      "/abonelik-iptal",
      "/coming-soon",
      "/yakinda",
      "/podmin",
    ].some(
      (p) =>
        location.pathname === p || location.pathname.startsWith(p + "/"),
    );
    if (isUtilityPath) {
      floatingPopup.hidden = true;
    } else {
      const adId = floatingPopup.dataset.adId || "0";
      let dismissed = false;
      try {
        dismissed = sessionStorage.getItem("msi_dismiss_ad_" + adId) === "1";
      } catch {}
      if (dismissed) {
        floatingPopup.hidden = true;
      } else if (cookieBar && !cookieBar.hidden) {
        floatingPopup.hidden = true;
        floatingPopup.dataset.deferredForCookie = "1";
      }
      floatingPopup
        .querySelector("[data-dismiss-floating-ad]")
        ?.addEventListener("click", () => {
          floatingPopup.hidden = true;
          delete floatingPopup.dataset.deferredForCookie;
          try {
            sessionStorage.setItem("msi_dismiss_ad_" + adId, "1");
          } catch {}
        });
    }
  }
  attachAdTelemetry();
  document.addEventListener("click", (e) => {
    const ad = e.target.closest("[data-ad-click-id]");
    if (ad) track("ad_click", Number(ad.dataset.adClickId), { el: ad });
  });

  // Smart Portrait/Document Framing & Interactive Lightbox for Article Galleries
  const galleryItems = $$(".msi-gallery-item");
  if (galleryItems.length) {
    const lbDialog = $("#gallery-lightbox");
    const lbImg = $("#lightbox-img");
    const lbCap = $("#lightbox-caption");
    const lbCounter = $("#lightbox-counter");
    const lbRaw = $("#lightbox-raw-link");
    let lbIndex = 0;

    const frameGalleryImg = (fig, img) => {
      if (!img.naturalWidth || !img.naturalHeight) return;
      if (img.naturalHeight >= img.naturalWidth * 0.84) {
        fig.classList.add("is-contain");
        fig.style.setProperty("--gal-bg", `url("${img.currentSrc || img.src}")`);
      }
    };

    const showLightboxAt = (nextIdx) => {
      if (!lbDialog || !lbImg || !galleryItems.length) return;
      lbIndex =
        ((nextIdx % galleryItems.length) + galleryItems.length) %
        galleryItems.length;
      const fig = galleryItems[lbIndex];
      const link = $("a", fig);
      const img = $("img", fig);
      const fullHref = link?.getAttribute("href") || img?.src || "";
      lbImg.src = fullHref;
      lbImg.alt = img?.alt || "";
      if (lbCap) lbCap.textContent = img?.alt || "";
      if (lbCounter)
        lbCounter.textContent = `${text("GÖRSEL", "IMAGE")} ${lbIndex + 1} / ${galleryItems.length}`;
      if (lbRaw) lbRaw.href = fullHref;
      const prevBtn = $("[data-lightbox-prev]", lbDialog);
      const nextBtn = $("[data-lightbox-next]", lbDialog);
      if (prevBtn) prevBtn.hidden = galleryItems.length < 2;
      if (nextBtn) nextBtn.hidden = galleryItems.length < 2;
      openDialog("gallery-lightbox");
    };

    galleryItems.forEach((fig, idx) => {
      const img = $("img", fig);
      const link = $("a", fig);
      if (img) {
        if (img.complete && img.naturalWidth) frameGalleryImg(fig, img);
        else img.addEventListener("load", () => frameGalleryImg(fig, img), { once: true });
      }
      link?.addEventListener("click", (ev) => {
        if (ev.metaKey || ev.ctrlKey || ev.shiftKey || !lbDialog) return;
        ev.preventDefault();
        showLightboxAt(idx);
      });
    });

    $("[data-lightbox-prev]")?.addEventListener("click", () =>
      showLightboxAt(lbIndex - 1),
    );
    $("[data-lightbox-next]")?.addEventListener("click", () =>
      showLightboxAt(lbIndex + 1),
    );
    document.addEventListener("keydown", (ev) => {
      if (!lbDialog?.open) return;
      if (ev.key === "ArrowLeft") showLightboxAt(lbIndex - 1);
      else if (ev.key === "ArrowRight") showLightboxAt(lbIndex + 1);
    });
  }

  $("#prefs-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const token = String(form.elements.token?.value || "").trim();
    const newsletter_frequency = String(
      form.elements.newsletter_frequency?.value || "weekly",
    ).trim();
    const categories = String(form.elements.categories?.value || "");
    if (btn) btn.disabled = true;
    try {
      const response = await fetch("/api/v1/email-preferences", {
        method: "POST",
        headers: csrfHeaders({ "Content-Type": "application/json" }),
        credentials: "same-origin",
        body: JSON.stringify({ token, newsletter_frequency, categories }),
      });
      const data = await response.json().catch(() => ({}));
      if (!response.ok) {
        throw new Error(
          extractError(
            data,
            text(
              "Tercihler kaydedilemedi.",
              "Preferences could not be saved.",
            ),
          ),
        );
      }
      toast(
        text(
          "E-posta tercihleriniz kaydedildi.",
          "Your email preferences have been saved.",
        ),
        "success",
      );
    } catch (err) {
      toast(
        err?.message ||
          text("Bağlantı hatası oluştu.", "A connection error occurred."),
        "error",
      );
    } finally {
      if (btn) btn.disabled = false;
    }
  });

  window.MSIUI = { toast, openIssue, msiConfirm };
})();
