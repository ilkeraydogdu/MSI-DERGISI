(() => {
  "use strict";
  const $ = (s) => document.querySelector(s),
    $$ = (s) => [...document.querySelectorAll(s)];
  const adminBase =
    (
      document.body?.dataset?.adminPath ||
      location.pathname.replace(/\/tasarim\/?$/, "") ||
      "/podmin"
    ).replace(/\/+$/, "") || "/podmin";
  const adminLink = $("#editor-admin-link");
  if (adminLink) adminLink.href = adminBase;

  function readCookie(name) {
    const prefix = name + "=";
    const hit = document.cookie
      .split(";")
      .map((c) => c.trim())
      .find((c) => c.startsWith(prefix));
    return hit ? decodeURIComponent(hit.slice(prefix.length)) : "";
  }
  function writeThemeCookie(theme) {
    const sec = location.protocol === "https:" ? "; Secure" : "";
    document.cookie = `msi_admin_theme=${encodeURIComponent(theme)}; path=/; max-age=31536000; SameSite=Lax${sec}`;
  }
  function resolveAutoStudioTheme() {
    try {
      if (window.matchMedia) {
        if (window.matchMedia("(prefers-color-scheme: dark)").matches) return "dark";
        if (window.matchMedia("(prefers-color-scheme: light)").matches) return "light";
      }
    } catch {}
    const hour = new Date().getHours();
    return hour >= 19 || hour < 7 ? "dark" : "light";
  }
  function getExplicitStudioTheme() {
    try {
      const ls = localStorage.getItem("msi_admin_theme");
      if (ls === "dark" || ls === "light") return ls;
    } catch {}
    const ck = readCookie("msi_admin_theme");
    if (ck === "dark" || ck === "light") return ck;
    try {
      const pub = localStorage.getItem("msi_theme");
      if (pub === "dark" || pub === "light") return pub;
    } catch {}
    return "";
  }
  function getStoredStudioTheme() {
    return getExplicitStudioTheme() || resolveAutoStudioTheme();
  }
  function applyStudioTheme(theme) {
    const next = theme === "dark" ? "dark" : "light";
    document.documentElement.dataset.theme = next;
    document.body.classList.toggle("admin-dark-mode", next === "dark");
    document.body.classList.toggle("admin-light-mode", next === "light");
    const btn = $("#studio-theme-toggle");
    if (btn) {
      btn.setAttribute("aria-pressed", String(next === "dark"));
      const lbl = btn.querySelector(".studio-theme-label");
      if (lbl) lbl.textContent = next === "dark" ? "Açık Stüdyo" : "Koyu Stüdyo";
    }
  }
  function toggleStudioTheme() {
    const current = document.documentElement.dataset.theme === "dark" ? "dark" : "light";
    const next = current === "dark" ? "light" : "dark";
    applyStudioTheme(next);
    try {
      localStorage.setItem("msi_admin_theme", next);
      localStorage.setItem("msi_theme", next);
    } catch {}
    writeThemeCookie(next);
  }
  const syncAutoStudioTheme = () => {
    if (!getExplicitStudioTheme()) applyStudioTheme(resolveAutoStudioTheme());
  };
  applyStudioTheme(getStoredStudioTheme());
  try {
    if (window.matchMedia) {
      window.matchMedia("(prefers-color-scheme: dark)").addEventListener?.("change", syncAutoStudioTheme);
      window.matchMedia("(prefers-color-scheme: light)").addEventListener?.("change", syncAutoStudioTheme);
    }
  } catch {}
  window.setInterval(syncAutoStudioTheme, 60000);
  $("#studio-theme-toggle")?.addEventListener("click", toggleStudioTheme);
  window.addEventListener("storage", (e) => {
    if ((e.key === "msi_admin_theme" || e.key === "msi_theme") && (e.newValue === "dark" || e.newValue === "light")) {
      applyStudioTheme(e.newValue);
    }
  });
  const kinds = {
    magazine_hero: "Dergi kapağı",
    news_hero: "Haber manşeti",
    articles: "İçerik listesi",
    issues: "Dergi arşivi",
    topics: "Konu başlıkları",
    banner: "Reklam alanı",
    text: "Metin",
    cta: "Çağrı alanı",
    plans: "Abonelik planları",
    products: "Yayınlar",
  };
  const colors = {
    background: "Sayfa zemini",
    surface: "Kart ve pencere",
    subtle: "İkincil zemin",
    text: "Ana metin",
    muted: "İkincil metin",
    border: "Çizgiler",
    accent: "Marka vurgusu",
  };
  let config,
    defaults,
    saved,
    pageIndex = 0,
    dirty = false,
    canSave = document.body?.dataset?.canSave !== "false",
    previewTimer,
    revision = 0,
    requestId = 0,
    previewDark = false;
  const clone = (o) => (o === undefined ? undefined : JSON.parse(JSON.stringify(o))),
    page = () => config?.pages?.[pageIndex],
    prefersReducedMotion = () => Boolean(window.matchMedia?.("(prefers-reduced-motion: reduce)")?.matches);
  const FOCUSABLE_SEL = 'a[href], button:not([disabled]), input:not([type="hidden"]):not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
  function applySavePermissionUi() {
    const saveBtn = $("#save-button");
    const banner = $("#studio-readonly-banner");
    if (saveBtn) {
      saveBtn.disabled = !canSave;
      saveBtn.setAttribute("aria-disabled", String(!canSave));
      if (!canSave) {
        saveBtn.title = "Salt okunur erişim: Kaydetmek için settings:write yetkisi gerekir.";
      } else {
        saveBtn.removeAttribute("title");
      }
    }
    if (banner) {
      banner.hidden = canSave;
    }
  }
  applySavePermissionUi();
  let studioConfirmResolve = null;
  let studioConfirmPrevFocus = null;
  function resolveStudioFocusTarget(el) {
    if (!el || !el.isConnected) return null;
    const target =
      el.closest(".studio-menu-panel") || el.closest("[hidden]")
        ? el.closest(".studio-dropdown")?.querySelector(".studio-dropdown-trigger")
        : el;
    if (
      target &&
      target.isConnected &&
      !target.closest("[hidden]") &&
      typeof target.focus === "function"
    ) {
      return target;
    }
    return null;
  }
  function closeStudioConfirm(accepted) {
    const modal = $("#studio-confirm-modal");
    const card = modal?.querySelector(".studio-modal-card");
    if (card && window.gsap) window.gsap.killTweensOf(card);
    if (modal) modal.hidden = true;
    const prev = studioConfirmPrevFocus;
    studioConfirmPrevFocus = null;
    const focusTarget = resolveStudioFocusTarget(prev);
    if (focusTarget) focusTarget.focus();
    const res = studioConfirmResolve;
    studioConfirmResolve = null;
    if (typeof res === "function") res(!!accepted);
  }
  function studioConfirm(message, titleOrOpts = "İşlem Onayı", extraOpts = {}) {
    const opts =
      typeof titleOrOpts === "object" && titleOrOpts !== null
        ? { ...titleOrOpts, ...extraOpts }
        : { title: titleOrOpts || "İşlem Onayı", ...extraOpts };
    const title = opts.title || "İşlem Onayı";
    const confirmLabel = opts.confirmLabel || "Onayla";
    const cancelLabel = opts.cancelLabel || "Vazgeç";
    const tone =
      opts.tone === "danger" || opts.tone === "warning" ? opts.tone : "primary";

    const modal = $("#studio-confirm-modal");
    const card = modal?.querySelector(".studio-modal-card");
    const msgEl = $("#studio-confirm-message");
    const titleEl = $("#studio-confirm-title");
    const acceptBtn = $("#studio-confirm-accept");
    const cancelBtn = $("#studio-confirm-cancel");
    if (!modal || !msgEl) return Promise.resolve(false);
    if (studioConfirmResolve) closeStudioConfirm(false);
    studioConfirmPrevFocus = resolveStudioFocusTarget(
      opts.returnFocusTo || document.activeElement,
    );
    if (card) card.dataset.tone = tone;
    if (titleEl) titleEl.textContent = title;
    msgEl.textContent = String(message || "");
    if (cancelBtn) cancelBtn.textContent = cancelLabel;
    if (acceptBtn) {
      acceptBtn.textContent = confirmLabel;
      acceptBtn.className = tone === "danger" ? "button danger-solid" : "button primary";
    }
    modal.hidden = false;
    if (card && window.gsap && !prefersReducedMotion()) {
      window.gsap.killTweensOf(card);
      window.gsap.fromTo(
        card,
        { opacity: 0.85, y: 10, scale: 0.98 },
        { opacity: 1, y: 0, scale: 1, duration: 0.2, ease: "power2.out", clearProps: "transform,opacity" },
      );
    }
    const initialFocus =
      tone === "danger" || tone === "warning" ? cancelBtn || acceptBtn : acceptBtn || cancelBtn;
    initialFocus?.focus();
    return new Promise((resolve) => {
      studioConfirmResolve = resolve;
    });
  }
  $("#studio-confirm-cancel")?.addEventListener("click", () => closeStudioConfirm(false));
  $("#studio-confirm-close")?.addEventListener("click", () => closeStudioConfirm(false));
  $("#studio-confirm-accept")?.addEventListener("click", () => closeStudioConfirm(true));
  $("#studio-confirm-modal")?.addEventListener("click", (e) => {
    if (e.target === $("#studio-confirm-modal")) closeStudioConfirm(false);
  });
  document.addEventListener("keydown", (e) => {
    const modal = $("#studio-confirm-modal");
    if (!modal || modal.hidden) return;
    if (e.key === "Escape") {
      e.preventDefault();
      closeStudioConfirm(false);
      return;
    }
    if (e.key === "Tab") {
      const focusables = Array.from(modal.querySelectorAll(FOCUSABLE_SEL)).filter((el) => {
        if (el.closest("[hidden]")) return false;
        const st = window.getComputedStyle(el);
        return st.display !== "none" && st.visibility !== "hidden" && el.getClientRects().length > 0;
      });
      if (!focusables.length) {
        e.preventDefault();
        return;
      }
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      const cur = document.activeElement;
      if (!modal.contains(cur)) {
        e.preventDefault();
        (e.shiftKey ? last : first).focus();
        return;
      }
      if (e.shiftKey) {
        if (cur === first || cur === modal) {
          e.preventDefault();
          last.focus();
        }
      } else if (cur === last) {
        e.preventDefault();
        first.focus();
      }
    }
  });

  function ensureToastRegion() {
    let region = $("#studio-toast-region");
    if (!region) {
      region = document.createElement("div");
      region.id = "studio-toast-region";
      region.className = "ops-toast-region";
      region.setAttribute("aria-live", "polite");
      region.setAttribute("aria-relevant", "additions");
      document.body.appendChild(region);
    }
    return region;
  }
  function createToastIconSvg(variant) {
    const ns = "http://www.w3.org/2000/svg";
    const svg = document.createElementNS(ns, "svg");
    svg.setAttribute("class", "adm-icon");
    svg.setAttribute("width", "18");
    svg.setAttribute("height", "18");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("fill", "none");
    svg.setAttribute("stroke", "currentColor");
    svg.setAttribute("stroke-width", "2");
    svg.setAttribute("stroke-linecap", "round");
    svg.setAttribute("stroke-linejoin", "round");
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
    if (variant === "success") {
      addCircle(12, 12, 10);
      addPath("m9 12 2 2 4-4");
    } else if (variant === "error") {
      addCircle(12, 12, 10);
      addPath("m15 9-6 6");
      addPath("m9 9 6 6");
    } else if (variant === "warning") {
      addPath("m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z");
      addPath("M12 9v4");
      addPath("M12 17h.01");
    } else {
      addCircle(12, 12, 10);
      addPath("M12 16v-4");
      addPath("M12 8h.01");
    }
    return svg;
  }
  function showStudioToast(message, variant = "info", duration = 4200) {
    const text = String(message ?? "").trim();
    if (!text) return null;
    const safeVariant = ["success", "error", "warning", "info"].includes(variant) ? variant : "info";
    const region = ensureToastRegion();
    const toast = document.createElement("div");
    toast.className = `ops-toast toast-${safeVariant}`;
    toast.setAttribute("role", safeVariant === "error" ? "alert" : "status");

    const iconWrap = document.createElement("span");
    iconWrap.className = "ops-toast-icon";
    iconWrap.setAttribute("aria-hidden", "true");
    iconWrap.appendChild(createToastIconSvg(safeVariant));

    const body = document.createElement("div");
    body.className = "ops-toast-body";
    body.textContent = text;

    const closeBtn = document.createElement("button");
    closeBtn.type = "button";
    closeBtn.className = "ops-toast-close";
    closeBtn.setAttribute("aria-label", "Kapat");
    closeBtn.textContent = "×";

    let timerId = null;
    let dismissed = false;
    function dismiss() {
      if (dismissed) return;
      dismissed = true;
      if (timerId) window.clearTimeout(timerId);
      if (prefersReducedMotion()) {
        toast.remove();
        return;
      }
      toast.classList.add("is-leaving");
      window.setTimeout(() => toast.remove(), 180);
    }
    closeBtn.addEventListener("click", dismiss);

    toast.append(iconWrap, body, closeBtn);
    region.appendChild(toast);
    if (duration > 0) {
      timerId = window.setTimeout(dismiss, duration);
    }
    return toast;
  }
  function feedback(message, error = false, variantOverride) {
    const el = $("#editor-feedback");
    const text = String(message ?? "").trim();
    if (el) {
      el.textContent = text;
      el.dataset.state = !text ? "" : error ? "error" : "success";
    }
    if (!text) return;
    showStudioToast(text, variantOverride || (error ? "error" : "success"));
  }

  const studioMenus = [];
  function closeAllStudioMenus(restoreFocus = false) {
    studioMenus.forEach(({ trigger, menu, setOpen }) => {
      if (!menu.hidden) {
        setOpen(false);
        if (restoreFocus) trigger.focus();
      }
    });
  }
  function bindStudioMenu(triggerId, menuId) {
    const trigger = $(triggerId);
    const menu = $(menuId);
    if (!trigger || !menu) return;
    const items = () =>
      Array.from(menu.querySelectorAll('[role="menuitem"]:not([disabled])')).filter(
        (el) => !el.hidden && !el.closest("[hidden]"),
      );
    function setOpen(open) {
      menu.hidden = !open;
      trigger.setAttribute("aria-expanded", String(open));
    }
    studioMenus.push({ trigger, menu, setOpen });

    trigger.addEventListener("click", (e) => {
      e.stopPropagation();
      const willOpen = menu.hidden;
      closeAllStudioMenus(false);
      setOpen(willOpen);
    });
    trigger.addEventListener("keydown", (e) => {
      if (e.key === "ArrowDown" || e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        closeAllStudioMenus(false);
        setOpen(true);
        items()[0]?.focus();
      } else if (e.key === "Escape" && !menu.hidden) {
        e.preventDefault();
        setOpen(false);
        trigger.focus();
      }
    });
    menu.addEventListener("keydown", (e) => {
      const list = items();
      if (!list.length) return;
      const idx = list.indexOf(document.activeElement);
      if (e.key === "ArrowDown") {
        e.preventDefault();
        list[(idx + 1) % list.length]?.focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        list[(idx - 1 + list.length) % list.length]?.focus();
      } else if (e.key === "Home") {
        e.preventDefault();
        list[0]?.focus();
      } else if (e.key === "End") {
        e.preventDefault();
        list[list.length - 1]?.focus();
      } else if (e.key === "Escape") {
        e.preventDefault();
        setOpen(false);
        trigger.focus();
      } else if (e.key === "Tab") {
        setOpen(false);
      }
    });
    menu.addEventListener("click", (e) => {
      if (e.target.closest('[role="menuitem"]')) {
        setOpen(false);
      }
    });
  }
  bindStudioMenu("#studio-workspace-btn", "#studio-workspace-menu");
  bindStudioMenu("#studio-tools-btn", "#studio-tools-menu");
  document.addEventListener("pointerdown", (e) => {
    if (!e.target.closest(".studio-dropdown")) {
      closeAllStudioMenus(false);
    }
  });
  window.addEventListener("blur", () => closeAllStudioMenus(false));
  function setSaveState(text, isDirty) {
    const pill = $("#save-state");
    if (!pill) return;
    pill.textContent = text;
    pill.dataset.dirty = isDirty ? "true" : "false";
  }
  function mark() {
    dirty = true;
    revision++;
    setSaveState("Kaydedilmemiş değişiklikler", true);
    try {
      localStorage.setItem("msi_design_draft", JSON.stringify(config));
    } catch {}
    clearTimeout(previewTimer);
    previewTimer = setTimeout(preview, 700);
  }
  function input(label, value, change, type = "text", options) {
    const wrap = document.createElement("label");
    wrap.append(document.createTextNode(label));
    const el = document.createElement(
      options ? "select" : type === "textarea" ? "textarea" : "input",
    );
    if (options) {
      Object.entries(options).forEach(([v, t]) => el.add(new Option(t, v)));
    } else if (type === "textarea") {
      el.rows = 3;
    } else el.type = type;
    if (type === "checkbox") {
      wrap.className = "check-label";
      el.checked = !!value;
    } else el.value = value ?? "";
    el.addEventListener(
      options || type === "checkbox" ? "change" : "input",
      () =>
        change(
          type === "checkbox"
            ? el.checked
            : type === "number"
              ? Number(el.value)
              : el.value,
        ),
    );
    wrap.append(el);
    return wrap;
  }
  function button(label, action, cls = "") {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = label;
    b.className = cls;
    b.addEventListener("click", action);
    return b;
  }
  function pathFor(p) {
    if (p.slug === "magazine") return "/?mode=magazine#dergi";
    if (p.slug === "news") return "/?mode=news#haber";
    if (["e-dergi", "abonelik-ve-satis"].includes(p.slug)) return "/" + p.slug;
    if (p.slug.startsWith("kurumsal-")) return "/kurumsal/" + p.slug.slice(9);
    return "/sayfa/" + p.slug;
  }
  function pageFields() {
    const p = page();
    $("#page-title").value = p.title;
    $("#page-description").value = p.description;
    $("#page-mode").value = p.mode;
    $("#page-slug").value = p.slug;
    $("#page-slug").disabled = ["magazine", "news"].includes(p.slug);
    $("#delete-page").hidden = ["magazine", "news"].includes(p.slug);
    $("#page-url").textContent = pathFor(p);
    $("#open-page").href = pathFor(p);
  }
  function selectPages() {
    const select = $("#page-select");
    select.replaceChildren();
    config.pages.forEach((p, i) =>
      select.add(new Option(p.title || p.slug, i)),
    );
    select.value = pageIndex;
  }
  function renderSections() {
    const list = $("#section-list");
    list.replaceChildren();
    page().sections.forEach((s, index) => {
      const details = document.createElement("details");
      details.className = "section-editor" + (s.enabled ? "" : " is-disabled");
      const summary = document.createElement("summary");
      summary.textContent = `${String(index + 1).padStart(2, "0")} · ${s.title || kinds[s.kind]}`;
      details.append(summary);
      const form = document.createElement("div");
      form.className = "ui-form";
      const field = (key, label, type = "text", options) =>
        form.append(
          input(
            label,
            s[key],
            (v) => {
              s[key] = v;
              mark();
              if (key === "enabled")
                details.classList.toggle("is-disabled", !v);
            },
            type,
            options,
          ),
        );
      field("enabled", "Bölümü göster", "checkbox");
      field("kind", "Bölüm tipi", "text", kinds);
      field("title", "Başlık");
      field("eyebrow", "Üst etiket");
      field("description", "Açıklama / metin", "textarea");
      field("source", "İçerik kaynağı", "text", {
        all: "Tüm içerikler",
        magazine: "Dergi içerikleri",
        news: "Haberler",
      });
      field("category", "Kategori, konu veya reklam slotu");
      field("layout", "Kart düzeni", "text", {
        grid: "Izgara",
        list: "Liste",
        feature: "Editoryal",
      });
      field("limit", "İçerik sayısı (1–24)", "number");
      field("offset", "Atlanacak içerik sayısı", "number");
      field("link", "Bağlantı adresi");
      field("link_label", "Bağlantı metni");
      details.append(form);
      const controls = document.createElement("div");
      controls.className = "section-controls";
      const up = button("↑ Yukarı", () => moveSection(index, -1));
      up.disabled = index === 0;
      const down = button("↓ Aşağı", () => moveSection(index, 1));
      down.disabled = index === page().sections.length - 1;
      controls.append(
        up,
        down,
        button("Kopyala", () => {
          page().sections.splice(index + 1, 0, clone(s));
          renderSections();
          mark();
        }),
        button(
          "Sil",
          async () => {
            if (page().sections.length <= 1) {
              showStudioToast("Bir sayfada en az bir bölüm kalmalıdır.", "error");
              return;
            }
            const secName = s.title || kinds[s.kind] || "Bu bölüm";
            if (
              !(await studioConfirm(
                `"${secName}" bölümünü bu sayfadan silmek istediğinize emin misiniz?`,
                { title: "Bölümü Sil", confirmLabel: "Bölümü Sil", tone: "danger" },
              ))
            ) {
              return;
            }
            page().sections.splice(index, 1);
            renderSections();
            mark();
            showStudioToast("Bölüm sayfadan kaldırıldı.", "info");
          },
          "remove",
        ),
      );
      details.append(controls);
      list.append(details);
    });
  }
  function moveSection(index, delta) {
    const a = page().sections;
    [a[index], a[index + delta]] = [a[index + delta], a[index]];
    renderSections();
    mark();
  }
  function renderTheme() {
    const templateRoot = $("#template-fields");
    templateRoot.replaceChildren();
    config.templates ||= {
      article_layout: "sidebar",
      category_layout: "grid",
      search_layout: "grid",
      reading_font: "serif",
    };
    Object.entries({
      article_layout: [
        "Haber / makale sayfası",
        { sidebar: "Konu kenar sütunuyla", centered: "Ortalanmış okuma" },
      ],
      category_layout: ["Kategori kartları", { grid: "Izgara", list: "Liste" }],
      search_layout: ["Arama kartları", { grid: "Izgara", list: "Liste" }],
      reading_font: [
        "Uzun metin yazı tipi",
        { serif: "Editoryal (serif)", sans: "Kurumsal (sans serif)" },
      ],
    }).forEach(([key, [label, options]]) =>
      templateRoot.append(
        input(
          label,
          config.templates[key],
          (value) => {
            config.templates[key] = value;
            mark();
          },
          "text",
          options,
        ),
      ),
    );
    const t = config.theme;
    $("#theme-default").value = t.default_mode;
    $("#theme-width").value = t.width;
    $("#theme-radius").value = t.radius;
    $("#theme-font-size").value = t.font_size;
    const root = $("#palette-fields");
    root.replaceChildren();
    for (const mode of ["light", "dark"]) {
      const group = document.createElement("div");
      group.className = "palette-group";
      const title = document.createElement("h2");
      title.textContent = mode === "light" ? "Açık tema" : "Yumuşak koyu tema";
      group.append(title);
      Object.entries(colors).forEach(([key, label]) => {
        const row = document.createElement("label");
        row.className = "palette-row";
        const span = document.createElement("span");
        span.textContent = label;
        const color = document.createElement("input");
        color.type = "color";
        color.value = t[mode][key];
        color.setAttribute("aria-label", label);
        const hex = document.createElement("input");
        hex.type = "text";
        hex.value = t[mode][key];
        hex.setAttribute("aria-label", label + " HEX");
        hex.maxLength = 7;
        color.addEventListener("input", () => {
          hex.value = color.value;
          t[mode][key] = color.value;
          mark();
        });
        hex.addEventListener("input", () => {
          t[mode][key] = hex.value;
          if (/^#[0-9a-f]{6}$/i.test(hex.value)) color.value = hex.value;
          mark();
        });
        row.append(span, color, hex);
        group.append(row);
      });
      root.append(group);
    }
  }
  function renderNavigation() {
    const root = $("#navigation-list");
    root.replaceChildren();
    config.navigation.forEach((n, i) => {
      const box = document.createElement("div");
      box.className = "navigation-editor ui-form";
      box.append(
        input("Menü başlığı", n.label, (v) => {
          n.label = v;
          mark();
        }),
        input("Adres", n.href, (v) => {
          n.href = v;
          mark();
        }),
      );
      const row = document.createElement("div");
      row.className = "editor-row";
      const up = button(
        "↑ Yukarı",
        () => {
          [config.navigation[i - 1], config.navigation[i]] = [
            n,
            config.navigation[i - 1],
          ];
          renderNavigation();
          mark();
        },
        "button secondary",
      );
      up.disabled = i === 0;
      row.append(
        up,
        button(
          "Kaldır",
          async () => {
            const navName = n.label || n.href || "Bu bağlantı";
            if (
              !(await studioConfirm(
                `"${navName}" bağlantısını üst menüden kaldırmak istediğinize emin misiniz?`,
                { title: "Menü Bağlantısını Kaldır", confirmLabel: "Kaldır", tone: "danger" },
              ))
            ) {
              return;
            }
            config.navigation.splice(i, 1);
            renderNavigation();
            mark();
            showStudioToast("Menü bağlantısı kaldırıldı.", "info");
          },
          "button danger",
        ),
      );
      box.append(row);
      root.append(box);
    });
  }
  function render() {
    selectPages();
    pageFields();
    renderSections();
    renderTheme();
    renderNavigation();
  }
  function jsonCsrfHeaders() {
    const headers = { "Content-Type": "application/json" };
    const match = document.cookie
      .split(";")
      .map((c) => c.trim())
      .find((c) => c.startsWith("csrf_token="));
    if (match) {
      headers["X-CSRF-Token"] = decodeURIComponent(match.substring("csrf_token=".length));
    }
    return headers;
  }
  async function preview() {
    if (!config) return;
    const request = ++requestId;
    const selected = page().slug;
    try {
      const response = await fetch(
        `${adminBase}/tasarim/onizleme?page=${encodeURIComponent(selected)}`,
        {
          method: "POST",
          headers: jsonCsrfHeaders(),
          body: JSON.stringify(config),
        },
      );
      if (request !== requestId) return;
      if (!response.ok) {
        const data = await response
          .json()
          .catch(() => ({ error: "Önizleme oluşturulamadı." }));
        throw Error(data.message || data.error || "Önizleme oluşturulamadı.");
      }
      const html = await response.text();
      if (request !== requestId) return;
      $("#design-preview").srcdoc = html;
      feedback("");
    } catch (error) {
      if (request === requestId) feedback(error.message, true);
    }
  }
  $("#design-preview").addEventListener("load", () => {
    const doc = $("#design-preview").contentDocument;
    if (doc) {
      doc.documentElement.dataset.theme = previewDark ? "dark" : "light";
      doc.addEventListener("click", (e) => {
        if (e.target.closest("a,button,summary")) e.preventDefault();
      });
    }
  });
  async function save() {
    if (!canSave) return;
    if (!config) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    const b = $("#save-button");
    b.disabled = true;
    const atRevision = revision;
    const submitted = clone(config);
    setSaveState("Kaydediliyor…", dirty);
    try {
      const response = await fetch("/api/v1/admin/design", {
        method: "POST",
        headers: jsonCsrfHeaders(),
        body: JSON.stringify(submitted),
      });
      const data = await response.json();
      if (!response.ok)
        throw Error(data.message || data.error || "Kaydetme başarısız.");
      saved = clone(submitted);
      if (revision === atRevision) {
        dirty = false;
        try {
          localStorage.removeItem("msi_design_draft");
        } catch {}
        setSaveState("Tüm değişiklikler kaydedildi", false);
      }
      feedback("Tasarım kaydedildi. Sayfalar yeni ayarlarla üretiliyor.");
    } catch (error) {
      setSaveState(
        dirty ? "Kaydedilmemiş değişiklikler" : "Kayıtlı tasarım",
        dirty,
      );
      feedback(error.message, true);
    } finally {
      b.disabled = !canSave;
    }
  }
  $("#save-button").addEventListener("click", save);
  $("#preview-button").addEventListener("click", () => {
    if (!config) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    preview();
  });
  $("#page-select").addEventListener("change", (e) => {
    if (!config) return;
    pageIndex = Number(e.target.value);
    pageFields();
    renderSections();
    preview();
  });
  ["title", "slug", "description", "mode"].forEach((key) =>
    $("#page-" + key).addEventListener(
      key === "mode" ? "change" : "input",
      (e) => {
        if (!config || !page()) return;
        page()[key] = e.target.value;
        $("#page-url").textContent = pathFor(page());
        $("#open-page").href = pathFor(page());
        if (key === "title" || key === "slug") selectPages();
        mark();
      },
    ),
  );
  Object.entries({
    default: "default_mode",
    width: "width",
    radius: "radius",
    "font-size": "font_size",
  }).forEach(([id, key]) =>
    $("#theme-" + id).addEventListener("input", (e) => {
      if (!config?.theme) return;
      config.theme[key] =
        key === "default_mode" ? e.target.value : Number(e.target.value);
      mark();
    }),
  );
  function uniqueSlug(prefix) {
    let i = 1;
    while (config.pages.some((p) => p.slug === prefix + "-" + i)) i++;
    return prefix + "-" + i;
  }
  $("#add-page").addEventListener("click", () => {
    if (!config) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    config.pages.push({
      slug: uniqueSlug("yeni-sayfa"),
      title: "Yeni sayfa",
      description: "",
      mode: "magazine",
      sections: [newSection("text")],
    });
    pageIndex = config.pages.length - 1;
    render();
    mark();
  });
  $("#duplicate-page").addEventListener("click", () => {
    if (!config || !page()) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    const copy = clone(page());
    copy.slug = uniqueSlug(page().slug);
    copy.title += " · Kopya";
    config.pages.push(copy);
    pageIndex = config.pages.length - 1;
    render();
    mark();
  });
  $("#delete-page").addEventListener("click", async () => {
    if (!config || !page()) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    if (config.pages.length <= 1) {
      feedback("En az bir sayfa kalmalıdır.", true);
      return;
    }
    if (
      !(await studioConfirm(
        `"${page().title || page().slug}" sayfasını silmek istediğinize emin misiniz?`,
        { title: "Sayfayı Sil", confirmLabel: "Sayfayı Sil", tone: "danger" },
      ))
    ) {
      return;
    }
    config.pages.splice(pageIndex, 1);
    pageIndex = 0;
    render();
    mark();
    showStudioToast("Sayfa taslaktan silindi.", "info");
  });
  function newSection(kind) {
    return {
      kind,
      title: kinds[kind],
      eyebrow: "",
      description: "",
      source: "all",
      category: "",
      layout: "grid",
      limit: 6,
      offset: 0,
      enabled: true,
      link: "",
      link_label: "",
    };
  }
  Object.entries(kinds).forEach(([key, label]) =>
    $("#new-section-kind").add(new Option(label, key)),
  );
  $("#add-section").addEventListener("click", () => {
    if (!config || !page()) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    page().sections.push(newSection($("#new-section-kind").value));
    renderSections();
    $("#section-list").lastElementChild.open = true;
    mark();
  });
  $("#add-navigation").addEventListener("click", () => {
    if (!config) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    config.navigation.push({ label: "Yeni bağlantı", href: "/" });
    renderNavigation();
    mark();
  });
  $$("[data-editor-tab]").forEach((tab) => {
    function activate() {
      const name = tab.dataset.editorTab;
      const panels = $$("[data-editor-tab]")
        .map((t) => $("#" + t.dataset.editorTab + "-panel"))
        .filter(Boolean);
      let activePanel = null;
      $$("[data-editor-tab]").forEach((t) => {
        const active = t === tab;
        t.setAttribute("aria-selected", String(active));
        t.tabIndex = active ? 0 : -1;
        const p = $("#" + t.dataset.editorTab + "-panel");
        if (p) {
          p.hidden = !active;
          if (active && t.dataset.editorTab === name) activePanel = p;
        }
      });
      if (activePanel && window.gsap && !prefersReducedMotion()) {
        window.gsap.killTweensOf(panels);
        window.gsap.fromTo(
          activePanel,
          { opacity: 0.78, y: 6 },
          { opacity: 1, y: 0, duration: 0.22, ease: "power2.out", clearProps: "transform,opacity" },
        );
      }
    }
    tab.addEventListener("click", activate);
    tab.addEventListener("keydown", (e) => {
      if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
        e.preventDefault();
        const tabs = $$("[data-editor-tab]"),
          i = tabs.indexOf(tab);
        const next = tabs[(i + (e.key === "ArrowRight" ? 1 : 2)) % 3];
        next.click();
        next.focus();
      }
    });
  });
  $$("[data-mobile-view]").forEach((btn) =>
    btn.addEventListener("click", () => {
      const ws = $(".editor-workspace");
      if (!ws) return;
      const mode = btn.dataset.mobileView;
      const next = ws.dataset.mobileView === mode ? "" : mode;
      if (next) ws.dataset.mobileView = next;
      else delete ws.dataset.mobileView;
      $$("[data-mobile-view]").forEach((el) =>
        el.setAttribute("aria-pressed", String(Boolean(next && el.dataset.mobileView === next))),
      );
      window.scrollTo(0, 0);
    }),
  );
  $$("[data-preview-width]").forEach((b) =>
    b.addEventListener("click", () => {
      const w = b.dataset.previewWidth || "100%";
      $("#design-preview").style.width = w;
      const stage = $(".preview-stage");
      if (stage) {
        stage.dataset.previewDevice = w === "390px" ? "mobile" : w === "768px" ? "tablet" : "desktop";
      }
      $$("[data-preview-width]").forEach((el) =>
        el.setAttribute("aria-pressed", String(el === b)),
      );
    }),
  );
  $("#preview-theme").addEventListener("click", () => {
    previewDark = !previewDark;
    $("#preview-theme").setAttribute("aria-pressed", String(previewDark));
    const doc = $("#design-preview").contentDocument;
    if (doc?.documentElement) {
      doc.documentElement.dataset.theme = previewDark ? "dark" : "light";
    }
  });
  $("#export-design")?.addEventListener("click", () => {
    if (!config) {
      showStudioToast("Dışa aktarılacak tasarım ayarları henüz yüklenemedi.", "error");
      return;
    }
    const a = document.createElement("a"),
      url = URL.createObjectURL(
        new Blob([JSON.stringify(config, null, 2)], {
          type: "application/json",
        }),
      );
    a.href = url;
    a.download = "msi-design.json";
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 500);
    showStudioToast("Tasarım JSON dosyası indirildi.", "info");
  });
  $("#import-design-btn")?.addEventListener("click", () => {
    if (!config) {
      showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    $("#import-design")?.click();
  });
  $("#import-design")?.addEventListener("change", async (e) => {
    const file = e.target.files?.[0];
    if (!file) return;
    try {
      if (!config) {
        showStudioToast("Tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
        return;
      }
      const importReturnFocus =
        !document.activeElement ||
        document.activeElement === document.body ||
        document.activeElement.closest(".studio-menu-panel") ||
        document.activeElement.closest("[hidden]")
          ? $("#studio-tools-btn")
          : document.activeElement;
      if (
        dirty &&
        !(await studioConfirm(
          "Kaydedilmemiş taslak değişiklikler içe aktarılan JSON ile değiştirilecek. Devam edilsin mi?",
          {
            title: "Tasarımı İçe Aktar",
            confirmLabel: "İçe Aktar",
            tone: "warning",
            returnFocusTo: importReturnFocus,
          },
        ))
      ) {
        return;
      }
      if (file.size > 500000) throw Error("Dosya en fazla 500 KB olabilir.");
      const imported = JSON.parse(await file.text());
      const response = await fetch(`${adminBase}/tasarim/onizleme`, {
        method: "POST",
        headers: jsonCsrfHeaders(),
        body: JSON.stringify(imported),
      });
      if (!response.ok) {
        const error = await response
          .json()
          .catch(() => ({ error: "Geçersiz tasarım dosyası." }));
        throw Error(
          error.message || error.error || "Geçersiz tasarım dosyası.",
        );
      }
      config = imported;
      pageIndex = 0;
      render();
      mark();
      feedback("Dosya taslağa alındı. Uygulamak için kaydedin.");
    } catch (error) {
      feedback(error.message, true);
    } finally {
      e.target.value = "";
    }
  });
  $("#restore-defaults")?.addEventListener("click", async () => {
    if (!defaults) {
      showStudioToast("Varsayılan tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    if (
      !(await studioConfirm(
        "Tüm tasarım ayarları varsayılan fabrika düzenine sıfırlanacaktır (taslağa alınır). Devam edilsin mi?",
        { title: "Fabrika Varsayılanlarına Sıfırla", confirmLabel: "Varsayılanlara Sıfırla", tone: "danger" },
      ))
    ) {
      return;
    }
    config = clone(defaults);
    pageIndex = 0;
    render();
    mark();
    feedback(
      "Varsayılan tasarım taslağa alındı. Kayıtlı tasarım henüz değiştirilmedi.",
      false,
      "info",
    );
  });
  $("#discard-draft")?.addEventListener("click", async () => {
    if (!saved) {
      showStudioToast("Kayıtlı tasarım ayarları henüz yüklenemedi. Lütfen sayfayı yenileyin.", "error");
      return;
    }
    if (
      dirty &&
      !(await studioConfirm(
        "Kaydedilmemiş taslak değişiklikler tamamen silinecek. Emin misiniz?",
        { title: "Taslağı Geri Al", confirmLabel: "Taslağı Bırak", tone: "danger" },
      ))
    ) {
      return;
    }
    const wasDirty = dirty;
    config = clone(saved);
    pageIndex = 0;
    dirty = false;
    try {
      localStorage.removeItem("msi_design_draft");
    } catch {}
    render();
    preview();
    setSaveState(canSave ? "Kayıtlı tasarım" : "Salt okunur mod", false);
    if (wasDirty) showStudioToast("Taslak değişiklikler geri alındı.", "info");
  });
  window.addEventListener("beforeunload", (e) => {
    if (dirty) {
      e.preventDefault();
      e.returnValue = "";
    }
  });
  (async () => {
    try {
      const response = await fetch("/api/v1/admin/design");
      if (!response.ok) throw Error("Tasarım ayarları yüklenemedi.");
      const rawData = await response.json();
      const data = rawData?.data || rawData;
      if (typeof data.can_save === "boolean") {
        canSave = data.can_save;
      }
      applySavePermissionUi();
      saved = clone(data.config);
      config = data.config;
      defaults = data.defaults;
      let draft;
      try {
        draft = JSON.parse(localStorage.getItem("msi_design_draft") || "null");
      } catch {}
      if (
        draft?.version === 1 &&
        Array.isArray(draft.pages) &&
        draft.pages.length &&
        draft.theme
      ) {
        config = draft;
        dirty = true;
      }
      render();
      setSaveState(
        dirty
          ? "Yerel taslak geri yüklendi"
          : canSave
            ? "Kayıtlı tasarım"
            : "Salt okunur mod",
        dirty,
      );
      await preview();
    } catch (error) {
      feedback(error.message, true);
      setSaveState("Yüklenemedi", false);
      $("#save-button").disabled = true;
    }
  })();
})();
