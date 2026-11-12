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

  let lenis = null;
  if (window.Lenis && !reducedMotion && !inIframe) {
    try {
      lenis = new window.Lenis({
        duration: 1.05,
        smoothWheel: true,
        wheelMultiplier: 0.95,
      });
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
      mode === "dark" ? "Açık temaya geç" : "Koyu temaya geç",
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

  function portal(mode) {
    document.body.classList.toggle("show-news", mode === "news");
    $$("[data-portal]").forEach((a) => {
      const active = a.dataset.portal === mode;
      a.classList.toggle("active", active);
      if (active) a.setAttribute("aria-current", "page");
      else a.removeAttribute("aria-current");
    });
    if (window.ScrollTrigger) {
      setTimeout(() => window.ScrollTrigger.refresh(), 50);
    }
  }
  const hasPortals = !!$("#view-magazine");
  function fromURL() {
    const mode =
      location.hash === "#haber"
        ? "news"
        : location.hash === "#dergi"
          ? "magazine"
          : new URL(location.href).searchParams.get("mode") || boot.mode;
    portal(mode);
  }
  fromURL();
  window.addEventListener("popstate", fromURL);
  window.addEventListener("hashchange", fromURL);
  $$("[data-portal]").forEach((a) =>
    a.addEventListener("click", (e) => {
      if (!hasPortals) return;
      e.preventDefault();
      const mode = a.dataset.portal;
      const url = new URL(location.href);
      url.searchParams.set("mode", mode);
      url.hash = mode === "news" ? "haber" : "dergi";
      history.pushState(null, "", url);
      portal(mode);
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
  $$(".navigation>a").forEach((a) => {
    if (
      new URL(a.href).pathname === location.pathname &&
      location.pathname !== "/"
    )
      a.setAttribute("aria-current", "page");
  });
  $("[data-language]")?.addEventListener("click", (e) => {
    e.preventDefault();
    const lang = e.currentTarget.dataset.language;
    document.cookie = `msi_lang=${lang};path=/;SameSite=Lax;max-age=31536000`;
    const url = new URL(location.href);
    url.searchParams.set("lang", lang);
    location.href = url;
  });

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
      if (badgeEl) badgeEl.textContent = sp.category_badge || "ÖZEL DOSYA";
      if (pagesEl) pagesEl.textContent = sp.page_no || "";
      if (titleEl) titleEl.textContent = localized(sp, "title");
      if (excerptEl) excerptEl.textContent = localized(sp, "excerpt");
      if (actionsEl) {
        const oldLink = $("[data-dossier-link], .dossier-link", actionsEl);
        if (oldLink) {
          if (sp.linked_content_slug) {
            const a = document.createElement("a");
            a.className = "dossier-link";
            a.dataset.dossierLink = "";
            a.href = "/icerik/" + encodeURIComponent(sp.linked_content_slug);
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
      const title = document.createElement("h3");
      title.textContent = localized(spread, "title");
      const description = document.createElement("p");
      description.textContent = localized(spread, "excerpt");
      root.append(title, description);
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
    $("#issue-dialog-summary").textContent = localized(issue, "summary");
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

  // Subscription & Publication Offer Dialog + Online Checkout
  async function openOffer(trigger) {
    try {
      const response = await fetch("/api/v1/commerce/catalog");
      if (!response.ok) throw Error();
      const catalog = await response.json();
      const plan = trigger.dataset.selectPlan;
      const item = plan
        ? catalog.subscription_plans.find((p) => p.code === plan)
        : catalog.commerce_products.find(
            (p) => p.sku === trigger.dataset.selectProduct,
          );
      if (!item) throw Error();
      const title = localized(item, plan ? "name" : "title");
      $("#offer-title").textContent = title;
      $("#offer-description").textContent = plan
        ? localized(item, "period_label")
        : localized(item, "description");
      $("#offer-price").textContent = new Intl.NumberFormat("tr-TR", {
        style: "currency",
        currency: "TRY",
        maximumFractionDigits: 0,
      }).format(item.price_try);

      const featList = $("#offer-features");
      if (featList) {
        featList.replaceChildren();
        const feats = plan
          ? boot.lang === "en"
            ? item.features_en || []
            : item.features_tr || []
          : item.file_resolution_or_pages
            ? [item.file_resolution_or_pages]
            : [];
        feats.forEach((f) => {
          const li = document.createElement("li");
          li.textContent = f;
          featList.append(li);
        });
        featList.hidden = !feats.length;
      }

      const checkoutForm = $("#offer-checkout-form");
      if (checkoutForm) {
        checkoutForm.elements.item_type.value = plan
          ? "subscription"
          : "product";
        checkoutForm.elements.item_code.value = plan
          ? item.code
          : item.sku;
        try {
          const savedUser = JSON.parse(storage.get("msi_active_user") || "null");
          if (savedUser) {
            if (!checkoutForm.elements.full_name.value && savedUser.full_name)
              checkoutForm.elements.full_name.value = savedUser.full_name;
            if (!checkoutForm.elements.email.value && savedUser.email)
              checkoutForm.elements.email.value = savedUser.email;
            if (
              !checkoutForm.elements.organization_name.value &&
              savedUser.organization_name
            )
              checkoutForm.elements.organization_name.value =
                savedUser.organization_name;
          }
        } catch {}
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
      $("#offer-contact").href =
        (contact?.href || "mailto:info@msidergisi.com") +
        "?subject=" +
        encodeURIComponent("MSI · " + title + " bilgi talebi");
      openDialog("offer-dialog");
    } catch {
      toast("Yayın bilgileri yüklenemedi. Lütfen tekrar deneyin.");
    }
  }

  $("#offer-checkout-form")?.addEventListener("submit", async (e) => {
    e.preventDefault();
    const form = e.currentTarget;
    const btn = $("[type=submit]", form);
    const fb = $("#offer-feedback");
    const paytrWrap = $("#paytr-iframe-wrap");
    const payload = Object.fromEntries(new FormData(form));
    payload.distance_sales_consent = !!form.elements.distance_sales_consent?.checked;
    btn.disabled = true;
    fb?.classList.remove("error");
    if (fb) fb.textContent = text("Sipariş oluşturuluyor…", "Creating order…");
    try {
      const response = await fetch("/api/v1/subscriptions/checkout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      const data = await response.json();
      if (!response.ok) {
        throw new Error(
          data.error ||
            text("Sipariş oluşturulamadı.", "Order could not be created."),
        );
      }
      const iframeUrl =
        data.paytr_iframe_url ||
        (data.iframe_token
          ? `https://www.paytr.com/odeme/guvenli/${encodeURIComponent(data.iframe_token)}`
          : "");
      if (iframeUrl && paytrWrap) {
        const iframe = document.createElement("iframe");
        iframe.src = iframeUrl;
        iframe.title = "PayTR Güvenli Ödeme";
        paytrWrap.replaceChildren(iframe);
        paytrWrap.hidden = false;
      }
      const invNo = data.invoice_no || data.invoice?.invoice_no || "";
      const msg =
        data.message ||
        data.message_tr ||
        text(
          `Siparişiniz kaydedildi (${invNo}).`,
          `Your order has been registered (${invNo}).`,
        );
      if (fb) fb.textContent = invNo ? `${msg} [${invNo}]` : msg;
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
    const offer = e.target.closest("[data-select-plan],[data-select-product]");
    if (offer) {
      e.preventDefault();
      openOffer(offer);
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

  function authTab(name) {
    $$("[data-auth-tab]").forEach((t) => {
      const active = t.dataset.authTab === name;
      t.setAttribute("aria-selected", String(active));
      t.tabIndex = active ? 0 : -1;
    });
    $("#login-form").hidden = name !== "login";
    $("#register-form").hidden = name !== "register";
  }
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

  ["login", "register"].forEach((kind) =>
    $("#" + kind + "-form")?.addEventListener("submit", async (e) => {
      e.preventDefault();
      const form = e.currentTarget,
        button = $("[type=submit]", form),
        feedback = $(".form-feedback", form);
      const payload = Object.fromEntries(new FormData(form));
      if (kind === "register") {
        payload.kvkk_consent = form.elements.kvkk_consent.checked;
        payload.kvkk_acknowledged = form.elements.kvkk_consent.checked;
        payload.email_consent = form.elements.email_consent.checked;
        payload.ssb_interests = $$("#register-ssb-tags input:checked").map(
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
          body: JSON.stringify(payload),
        });
        const data = await response.json();
        if (!response.ok)
          throw new Error(
            data.error ||
              text("İşlem tamamlanamadı.", "Unable to complete request."),
          );
        feedback.textContent =
          kind === "login"
            ? text("Giriş başarılı.", "Signed in.")
            : data.message_tr ||
              text(
                "Kaydınız oluşturuldu. Giriş yapabilirsiniz.",
                "Account created. You can sign in.",
              );
        if (kind === "login") {
          // SECURITY FIX (C4/H8): Removed session token and PII from localStorage.
          // Session is managed exclusively via HttpOnly cookie server-side.
          const link = $(".account-link");
          if (link) link.href = "/hesabim";
          toast(feedback.textContent);
          $("#auth-dialog").close();
          const next = new URL(location.href).searchParams.get("next");
          if (
            next &&
            /^\/(admin(?:\/|$)|yonetim$)/.test(next) &&
            ["site_operator", "admin"].includes(data.user.role)
          )
            location.href = next;
          else if (location.pathname === "/hesabim") location.reload();
        } else {
          authTab("login");
          $("#login-form").elements.email.value = payload.email;
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

  $("[data-logout]")?.addEventListener("click", async () => {
    try {
      const response = await fetch("/api/v1/auth/logout", { method: "POST" });
      if (!response.ok) throw Error();
      // SECURITY FIX (C4/H8): Session token no longer stored in localStorage.
      // Only clear any legacy PII that may exist from older sessions.
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
  window.MSIUI = { toast, openIssue };
})();
