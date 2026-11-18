/* Shared interaction, motion choreography, live 3D dossier mockup, and commerce/auth layer. */
(() => {
  "use strict";
  const boot = JSON.parse(
    document.getElementById("msi-bootstrap")?.textContent || "{}",
  );
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
    if ($("dialog[open]")) lenis.stop();
    else lenis.start();
  }

  function toast(message) {
    const region = $(".toast-region");
    if (!region) return;
    const el = document.createElement("div");
    el.className = "toast-message";
    el.textContent = message;
    region.replaceChildren(el);
    setTimeout(() => el.remove(), 4500);
  }

  let restoreFocus;
  function openDialog(id) {
    const dialog = document.getElementById(id);
    if (!dialog) return;
    restoreFocus = document.activeElement;
    if (!dialog.open) dialog.showModal();
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
  function applyTheme(mode) {
    document.documentElement.dataset.theme = mode;
    themeButton?.setAttribute("aria-pressed", String(mode === "dark"));
    themeButton?.setAttribute(
      "aria-label",
      mode === "dark"
        ? text("Açık temaya geç", "Switch to light theme")
        : text("Koyu temaya geç", "Switch to dark theme"),
    );
  }
  const media = matchMedia("(prefers-color-scheme: dark)");
  applyTheme(
    storage.get("msi_theme") ||
      (boot.theme === "system"
        ? media.matches
          ? "dark"
          : "light"
        : boot.theme) ||
      "light",
  );
  media.addEventListener("change", (e) => {
    if (boot.theme === "system" && !storage.get("msi_theme"))
      applyTheme(e.matches ? "dark" : "light");
  });
  themeButton?.addEventListener("click", () => {
    const next =
      document.documentElement.dataset.theme === "dark" ? "light" : "dark";
    applyTheme(next);
    storage.set("msi_theme", next);
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
    menu.setAttribute("aria-label", open ? "Menüyü kapat" : "Menüyü aç");
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
      location.href = el.getAttribute("href") || `/switch/${lang}`;
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
    const badge = $(".notification-count");
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
  $$("[data-hero-stage]").forEach((stage) => {
    const issueNum = Number(stage.dataset.issueNumber);
    const stageIssue =
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
      const badgeMap = { "KAPAK DOSYASI": "COVER FILE", "SÖYLEŞİ": "INTERVIEW", "ÖZEL DOSYA": "SPECIAL DOSSIER", "ANALİZ": "ANALYSIS", "DOSYA": "DOSSIER" };
      const rawBadge = (sp.category_badge || "ÖZEL DOSYA").trim();
      const rawPages = sp.page_no || "";
      if (badgeEl) badgeEl.textContent = boot.lang === "en" ? (badgeMap[rawBadge.toUpperCase()] || rawBadge) : rawBadge;
      if (pagesEl) pagesEl.textContent = boot.lang === "en" ? rawPages.replace(/^S\.\s*/i, "P. ") : rawPages;
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

  // Full-screen E-Dergi dialog reader
  let issue,
    spreadIndex = 0;
  function renderSpread() {
    const spreads = issue.spreads || [],
      spread = spreads[spreadIndex],
      root = $("#reader-spread");
    root.replaceChildren();
    $("#reader-position").textContent = spreads.length
      ? `${spreadIndex + 1} / ${spreads.length} · ${spread.page_no}`
      : text("İçindekiler henüz eklenmedi.", "Contents are not yet available.");
    $("[data-reader-prev]").disabled = spreadIndex <= 0;
    $("[data-reader-next]").disabled = spreadIndex >= spreads.length - 1;
    if (spread) {
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
        a.href = "/icerik/" + encodeURIComponent(spread.linked_content_slug);
        a.className = "text-link";
        a.textContent = text("İçeriği oku →", "Read article →");
        root.append(a);
      }
    }
  }
  function openIssue(number) {
    issue =
      boot.issues?.find((i) => i.issue_number === Number(number)) ||
      boot.issues?.find((i) => i.issue_number === boot.featuredIssue) ||
      boot.issues?.[0];
    if (!issue) {
      toast(
        text(
          "Şu anda görüntülenebilecek sayı bulunmuyor.",
          "No issue is available.",
        ),
      );
      return;
    }
    spreadIndex = 0;
    $("#issue-dialog-title").textContent = localized(issue, "title");
    $("#issue-dialog-meta").textContent =
      `SAYI ${issue.issue_number} · ${localized(issue, "publish_month")} ${issue.year}`;
    const image = $("#issue-dialog-cover");
    image.src = issue.cover_image.startsWith("https://")
      ? issue.cover_image
      : "/" + issue.cover_image.replace(/^\/+/, "");
    image.alt = localized(issue, "title");
    const rawSummary = localized(issue, "summary").trim();
    const firstSpreadTitle = issue.spreads?.[0]
      ? localized(issue.spreads[0], "title").trim()
      : "";
    const metaFallback =
      boot.lang === "en"
        ? `${issue.page_count || 96} pages · ${issue.dossier_count || 6} special dossiers · ISSN 1306-0309`
        : `${issue.page_count || 96} sayfa · ${issue.dossier_count || 6} özel dosya · ISSN 1306-0309`;
    $("#issue-dialog-summary").textContent =
      rawSummary.includes("yeni sayısının içerikleri:") ||
      rawSummary.endsWith("...") ||
      (firstSpreadTitle &&
        rawSummary
          .toLowerCase()
          .includes(firstSpreadTitle.slice(0, 24).toLowerCase()))
        ? metaFallback
        : rawSummary || metaFallback;
    $("#issue-download").href =
      `/api/v1/edergi/${issue.issue_number}/download-pdf?pages=all`;
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
        encodeURIComponent("MSI · " + title + " bilgi talebi");
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
        headers: { "Content-Type": "application/json" },
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
      const iframeUrl =
        resPayload.paytr_iframe_url ||
        (resPayload.iframe_token
          ? `https://www.paytr.com/odeme/guvenli/${encodeURIComponent(resPayload.iframe_token)}`
          : "");
      if (iframeUrl && paytrWrap) {
        const iframe = document.createElement("iframe");
        iframe.src = iframeUrl;
        iframe.title = "PayTR Güvenli Ödeme";
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
      toast(invNo ? `Sipariş kaydı oluşturuldu: ${invNo}` : msg);
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
            ),
          )
          .catch(() =>
            toast(
              text(
                "Kodları seçerek kopyalayabilirsiniz.",
                "Select the codes to copy manually.",
              ),
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
    const magazine = e.target.closest(
      '[data-open-issue], [href="#modal-magazine"]',
    );
    if (magazine) {
      e.preventDefault();
      openIssue(magazine.dataset.openIssue);
      return;
    }
    if (e.target.closest('[href="#modal-login"]')) {
      e.preventDefault();
      authView("login");
      openDialog("auth-dialog");
      return;
    }
    if (e.target.closest('[href="#modal-reklam"]')) {
      e.preventDefault();
      location.href = "/kurumsal/iletisim";
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
  const regEmail = $('#register-form [name="email"]');
  const govHint = $("#gov-domain-hint");
  regEmail?.addEventListener("input", () => {
    const val = (regEmail.value || "").trim().toLowerCase();
    const isGov = /(\.gov\.tr|\.tsk\.tr|\.edu\.tr|\.pol\.tr)$/.test(val);
    if (govHint) govHint.hidden = !isGov;
  });

  function authView(name) {
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
    const link = $(".account-link");
    if (link) link.href = "/hesabim";
    toast(msgText);
    $("#auth-dialog")?.close();
    const next = new URL(location.href).searchParams.get("next");
    if (
      typeof next === "string" &&
      next.startsWith("/") &&
      !next.startsWith("//") &&
      !next.startsWith("/\\")
    ) {
      location.href = next;
    } else if (
      location.pathname === "/hesabim" ||
      location.pathname.startsWith("/icerik/")
    ) {
      location.reload();
    }
  }

  // Auto-open auth dialog when redirected with ?next=/... while signed out
  (() => {
    const nextParam = new URL(location.href).searchParams.get("next");
    if (
      typeof nextParam === "string" &&
      nextParam.startsWith("/") &&
      !nextParam.startsWith("//") &&
      !nextParam.startsWith("/\\") &&
      !inIframe &&
      ($('.account-link[href="#modal-login"]') || $(".account-welcome"))
    ) {
      authView("login");
      openDialog("auth-dialog");
    }
  })();

  ["login", "register"].forEach((kind) =>
    $("#" + kind + "-form")?.addEventListener("submit", async (e) => {
      e.preventDefault();
      const form = e.currentTarget,
        button = $("[type=submit]", form),
        feedback = $(".form-feedback", form);
      const reqPayload = Object.fromEntries(new FormData(form));
      if (kind === "register") {
        reqPayload.kvkk_consent = form.elements.kvkk_consent.checked;
        reqPayload.kvkk_acknowledged = form.elements.kvkk_consent.checked;
        reqPayload.email_consent = form.elements.email_consent.checked;
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
          headers: { "Content-Type": "application/json" },
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
        headers: { "Content-Type": "application/json" },
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
      const response = await fetch("/api/v1/auth/forgot-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
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
        headers: { "Content-Type": "application/json" },
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
      toast(msg);
    } catch (err) {
      if (fb) fb.textContent = err.message;
      toast(err.message);
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
        headers: { "Content-Type": "application/json" },
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
        headers: { "Content-Type": "application/json" },
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
        headers: { "Content-Type": "application/json" },
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
        headers: { "Content-Type": "application/json" },
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

  $("#reset-password-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
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
      fb?.classList.add("error");
      const mismatchMsg = text(
        "Yeni şifre ve şifre tekrarı eşleşmiyor.",
        "Passwords do not match.",
      );
      if (fb) fb.textContent = mismatchMsg;
      toast(mismatchMsg);
      return;
    }
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb)
      fb.textContent = text("Şifreniz güncelleniyor…", "Updating password…");
    try {
      const response = await fetch("/api/v1/auth/reset-password", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
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
      if (fb) fb.textContent = msg;
      toast(msg);
      setTimeout(() => {
        location.href = "/hesabim";
      }, 1400);
    } catch (err) {
      fb?.classList.add("error");
      if (fb) fb.textContent = err.message;
    } finally {
      btn.disabled = false;
    }
  });

  $("[data-logout]")?.addEventListener("click", async () => {
    try {
      const response = await fetch("/api/v1/auth/logout", { method: "POST" });
      if (!response.ok) throw Error();
      try {
        localStorage.removeItem("msi_active_user");
        localStorage.removeItem("msi_session_token");
      } catch {}
      location.href = "/hesabim";
    } catch {
      toast("Çıkış yapılamadı. Tekrar deneyin.");
    }
  });
  $("[data-share-article]")?.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(location.origin + location.pathname);
      toast("Bağlantı kopyalandı.");
    } catch {
      toast("Bağlantıyı tarayıcınızın adres çubuğundan kopyalayabilirsiniz.");
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
            track("ad_impression", Number(e.target.dataset.adId));
            adObserver.unobserve(e.target);
          }
        }),
      { threshold: 0.5 },
    );
    $$("[data-ad-id]").forEach((el) => adObserver.observe(el));
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
    sent = false;
  $$("video[data-msi-video]").forEach((v) => {
    v.addEventListener("ended", () => {
      videoCompleted = true;
    });
  });
  function getAnalyticsSessionId() {
    let sid = storage.get("msi_session_id");
    if (!sid) {
      sid = crypto.randomUUID?.() || String(Date.now());
      storage.set("msi_session_id", sid);
    }
    return sid;
  }
  const slug = location.pathname.startsWith("/icerik/")
    ? decodeURIComponent(location.pathname.slice(8))
    : null;
  function track(event, adId) {
    if (event === "article_read" && !cookieConsent.analytics) return;
    if ((event === "ad_impression" || event === "ad_click") && !cookieConsent.marketing)
      return;
    const seconds = Math.round((Date.now() - start) / 1000);
    const payload = {
      id: 0,
      session_id: getAnalyticsSessionId(),
      user_id: null,
      organization_category: "",
      organization_name: "",
      event_type: event,
      content_slug: slug,
      ad_id: adId || null,
      duration_sec: seconds,
      scroll_depth_pct: depth,
      video_completed: videoCompleted,
      is_true_read: seconds >= 25 && depth >= 70,
      ssb_tag: null,
      created_at: "",
    };
    navigator.sendBeacon(
      "/api/v1/analytics/track",
      new Blob([JSON.stringify(payload)], { type: "application/json" }),
    );
  }
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
  addEventListener("pagehide", () => {
    if (slug && !sent) {
      track("article_read");
      sent = true;
    }
  });
  attachAdTelemetry();
  document.addEventListener("click", (e) => {
    const ad = e.target.closest("[data-ad-click-id]");
    if (ad) track("ad_click", Number(ad.dataset.adClickId));
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

  window.MSIUI = { toast, openIssue };
})();
