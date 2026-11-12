(() => {
  "use strict";
  const $ = (s) => document.querySelector(s),
    $$ = (s) => [...document.querySelectorAll(s)];
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
    previewTimer,
    revision = 0,
    requestId = 0,
    previewDark = false;
  const clone = (o) => JSON.parse(JSON.stringify(o)),
    page = () => config.pages[pageIndex];
  function feedback(message, error = false) {
    $("#editor-feedback").textContent = message;
    $("#editor-feedback").setAttribute("role", error ? "alert" : "status");
  }
  function mark() {
    dirty = true;
    revision++;
    $("#save-state").textContent = "Kaydedilmemiş değişiklikler";
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
          () => {
            page().sections.splice(index, 1);
            renderSections();
            mark();
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
          () => {
            config.navigation.splice(i, 1);
            renderNavigation();
            mark();
          },
          "button secondary",
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
  async function preview() {
    if (!config) return;
    const request = ++requestId;
    const selected = page().slug;
    try {
      const response = await fetch(
        "/admin/tasarim/onizleme?page=" + encodeURIComponent(selected),
        {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(config),
        },
      );
      if (request !== requestId) return;
      if (!response.ok) {
        const data = await response
          .json()
          .catch(() => ({ error: "Önizleme oluşturulamadı." }));
        throw Error(data.error || "Önizleme oluşturulamadı.");
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
    const b = $("#save-button");
    b.disabled = true;
    const atRevision = revision;
    const submitted = clone(config);
    feedback("Kaydediliyor…");
    try {
      const response = await fetch("/api/v1/admin/design", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(submitted),
      });
      const data = await response.json();
      if (!response.ok) throw Error(data.error || "Kaydetme başarısız.");
      saved = clone(submitted);
      if (revision === atRevision) {
        dirty = false;
        try {
          localStorage.removeItem("msi_design_draft");
        } catch {}
        $("#save-state").textContent = "Tüm değişiklikler kaydedildi";
      }
      feedback("Tasarım kaydedildi. Sayfalar yeni ayarlarla üretiliyor.");
    } catch (error) {
      feedback(error.message, true);
    } finally {
      b.disabled = false;
    }
  }
  $("#save-button").addEventListener("click", save);
  $("#preview-button").addEventListener("click", preview);
  $("#page-select").addEventListener("change", (e) => {
    pageIndex = Number(e.target.value);
    pageFields();
    renderSections();
    preview();
  });
  ["title", "slug", "description", "mode"].forEach((key) =>
    $("#page-" + key).addEventListener(
      key === "mode" ? "change" : "input",
      (e) => {
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
    const copy = clone(page());
    copy.slug = uniqueSlug(page().slug);
    copy.title += " · Kopya";
    config.pages.push(copy);
    pageIndex = config.pages.length - 1;
    render();
    mark();
  });
  $("#delete-page").addEventListener("click", () => {
    config.pages.splice(pageIndex, 1);
    pageIndex = 0;
    render();
    mark();
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
    page().sections.push(newSection($("#new-section-kind").value));
    renderSections();
    $("#section-list").lastElementChild.open = true;
    mark();
  });
  $("#add-navigation").addEventListener("click", () => {
    config.navigation.push({ label: "Yeni bağlantı", href: "/" });
    renderNavigation();
    mark();
  });
  $$("[data-editor-tab]").forEach((tab) => {
    function activate() {
      const name = tab.dataset.editorTab;
      $$("[data-editor-tab]").forEach((t) => {
        const active = t === tab;
        t.setAttribute("aria-selected", String(active));
        t.tabIndex = active ? 0 : -1;
        $("#" + t.dataset.editorTab + "-panel").hidden = !active;
      });
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
  $$("[data-preview-width]").forEach((b) =>
    b.addEventListener("click", () => {
      $("#design-preview").style.width = b.dataset.previewWidth;
      $$("[data-preview-width]").forEach((el) =>
        el.setAttribute("aria-pressed", String(el === b)),
      );
    }),
  );
  $("#preview-theme").addEventListener("click", () => {
    previewDark = !previewDark;
    $("#preview-theme").setAttribute("aria-pressed", String(previewDark));
    $("#design-preview").contentDocument.documentElement.dataset.theme =
      previewDark ? "dark" : "light";
  });
  $("#export-design").addEventListener("click", () => {
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
  });
  $("#import-design").addEventListener("change", async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    try {
      if (file.size > 500000) throw Error("Dosya en fazla 500 KB olabilir.");
      const imported = JSON.parse(await file.text());
      const response = await fetch("/admin/tasarim/onizleme", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(imported),
      });
      if (!response.ok) {
        const error = await response
          .json()
          .catch(() => ({ error: "Geçersiz tasarım dosyası." }));
        throw Error(error.error);
      }
      config = imported;
      pageIndex = 0;
      render();
      mark();
      feedback("Dosya taslağa alındı. Uygulamak için kaydedin.");
    } catch (error) {
      feedback(error.message, true);
    }
    e.target.value = "";
  });
  $("#restore-defaults").addEventListener("click", () => {
    config = clone(defaults);
    pageIndex = 0;
    render();
    mark();
    feedback(
      "Varsayılan tasarım taslağa alındı. Kayıtlı tasarım henüz değiştirilmedi.",
    );
  });
  $("#discard-draft").addEventListener("click", () => {
    config = clone(saved);
    pageIndex = 0;
    dirty = false;
    try {
      localStorage.removeItem("msi_design_draft");
    } catch {}
    render();
    preview();
    $("#save-state").textContent = "Kayıtlı tasarım";
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
      const data = await response.json();
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
      $("#save-state").textContent = dirty
        ? "Yerel taslak geri yüklendi"
        : "Kayıtlı tasarım";
      await preview();
    } catch (error) {
      feedback(error.message, true);
      $("#save-state").textContent = "Yüklenemedi";
      $("#save-button").disabled = true;
    }
  })();
})();
