(function () {
  "use strict";
  const ctx = window.MSIAdmin?._ctx;
  if (!ctx) return;
  const { state, paginationState, byId, val, setVal, esc, apiRequest, unwrapList, showToast, renderTableState, renderPaginationControls, getLang, t, svgIcon, confirmDialog } = ctx;

  let _mediaSearchTimer = null;

  function syncDynamicTaxonomyControls() {
    const isEn = getLang() === "en";
    const tagMap = new Map();
    (state.ssbTags || []).forEach((tag) => {
      if (tag?.code) {
        const lbl = isEn ? (tag.name_en || tag.name_tr || tag.code) : (tag.name_tr || tag.code);
        tagMap.set(tag.code, lbl);
      }
    });
    const mergedTags = Array.from(tagMap.entries()).map(([code, label]) => ({ code, label }));

    if (mergedTags.length) {
      const syncMatrix = (matrixId, inputName) => {
        const m = byId(matrixId);
        if (!m) return;
        const checked = new Set(Array.from(m.querySelectorAll(`input[name="${inputName}"]:checked`)).map((e) => e.value));
        m.innerHTML = mergedTags.map((tag) => `<label class="check-pill"><input type="checkbox" name="${esc(inputName)}" value="${esc(tag.code)}" ${checked.has(tag.code) ? "checked" : ""}> <span>${esc(tag.label)}</span></label>`).join("");
      };
      syncMatrix("c-ssb-matrix", "c_ssb");
      syncMatrix("ad-ssb-matrix", "ad_ssb");

      const nlSel = byId("nl-ssb-tag");
      if (nlSel) {
        const cur = nlSel.value || "ALL";
        nlSel.innerHTML = `<option value="ALL">${esc(t("admin.taxonomy.all_ssb_tags", "Tüm SSB İlgi Alanları", "All SSB Interest Areas"))}</option>` +
          mergedTags.map((tag) => `<option value="${esc(tag.code)}">${esc(tag.label)}</option>`).join("");
        if ([...nlSel.options].some((o) => o.value === cur)) nlSel.value = cur;
      }
    }

    if (Array.isArray(state.categories) && state.categories.length) {
      const pm = byId("c-placements-matrix");
      if (pm) {
        const existing = new Set(Array.from(pm.querySelectorAll('input[name="c_placement"]')).map((e) => e.value));
        state.categories
          .filter((c) => c?.slug && !existing.has(c.slug) && (c.is_dynamic || c.group_type === "gruplandirilmis" || c.group_type === "icerikler"))
          .forEach((c) => pm.insertAdjacentHTML("beforeend", `<label class="check-pill"><input type="checkbox" name="c_placement" value="${esc(c.slug)}"> <span>${esc(c.name_tr || c.slug)}</span></label>`));
      }
      const ctSel = byId("c-content-type");
      if (ctSel) {
        const existing = new Set(Array.from(ctSel.options).map((o) => o.value));
        state.categories
          .filter((c) => c?.slug && c.is_dynamic && c.group_type === "icerikler" && !existing.has(c.slug))
          .forEach((c) => {
            const opt = document.createElement("option");
            opt.value = c.slug;
            opt.textContent = `${c.name_tr || c.slug} (${c.slug})`;
            ctSel.appendChild(opt);
          });
      }
    }
  }

  function renderTaxonomyTables() {
    const canWrite = ctx.hasPerm("categories:write|settings:write");
    const tbodyCats = byId("tbody-categories");
    if (tbodyCats) {
      if (!state.categories.length) renderTableState(tbodyCats, "empty", { colCount: 5 });
      else {
        tbodyCats.innerHTML = state.categories.map((c) => {
          const ops = canWrite
            ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-category" data-id="${esc(c.id)}" title="${esc(t("admin.action.edit", "Düzenle"))}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-category" data-id="${esc(c.id)}" title="${esc(t("admin.action.delete", "Sil"))}">${svgIcon("trash")}</button></div>`
            : '<span class="text-muted text-xs">—</span>';
          return `<tr>
            <td class="text-mono text-xs text-info">/${esc(c.slug)}</td>
            <td><div class="font-semibold">${esc(c.name_tr)}</div><div class="text-xs text-muted">${esc(c.name_en)}</div></td>
            <td><span class="badge-tag">${esc(c.group_type)}</span></td>
            <td>${c.is_dynamic ? `<span class="badge-tag badge-paywall-0">${esc(t("admin.taxonomy.dynamic", "Dinamik"))}</span>` : `<span class="badge-tag">${esc(t("admin.taxonomy.core", "Çekirdek"))}</span>`}</td>
            <td>${ops}</td>
          </tr>`;
        }).join("");
      }
    }

    const tbodySsb = byId("tbody-ssb-tags");
    if (tbodySsb) {
      if (!state.ssbTags.length) renderTableState(tbodySsb, "empty", { colCount: 5 });
      else {
        tbodySsb.innerHTML = state.ssbTags.map((tag) => {
          const ops = canWrite
            ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-ssb" data-code="${esc(tag.code || "")}" title="${esc(t("admin.action.edit", "Düzenle", "Edit"))}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-ssb" data-code="${esc(tag.code || "")}" title="${esc(t("admin.action.delete", "Sil", "Delete"))}">${svgIcon("trash")}</button></div>`
            : '<span class="text-muted text-xs">—</span>';
          return `<tr>
            <td class="text-mono text-xs text-success">${esc(tag.code || "")}</td>
            <td><div class="font-semibold">${esc(tag.name_tr || "")}</div><div class="text-xs text-muted">${esc(tag.name_en || "")}</div></td>
            <td><span class="badge-tag badge-ssb">${esc(tag.category_group || "—")}</span></td>
            <td class="text-mono text-xs">${esc(tag.sort_order ?? 0)}</td>
            <td>${ops}</td>
          </tr>`;
        }).join("");
      }
    }
  }

  async function loadTaxonomy() {
    const catUrl = ctx.hasPerm("categories:read|contents:read") ? "/api/v1/admin/categories" : "/api/v1/categories";
    const ssbUrl = ctx.hasPerm("categories:read|contents:read|ads:read") ? "/api/v1/admin/ssb-tags" : "/api/v1/ssb-tags";
    const [catRes, ssbRes] = await Promise.all([apiRequest(catUrl), apiRequest(ssbUrl)]);
    if (catRes.ok) state.categories = unwrapList(catRes.payload, "categories");
    if (ssbRes.ok) state.ssbTags = unwrapList(ssbRes.payload, "ssb_tags");
    syncDynamicTaxonomyControls();
    renderTaxonomyTables();
  }

  async function loadMediaLibrary(page) {
    if (!ctx.hasPerm("media:read|media:upload|contents:read")) return;
    const p = typeof page === "number" ? page : paginationState.mediaLibrary || 1;
    paginationState.mediaLibrary = p;
    const q = encodeURIComponent(val("media-search"));
    const resp = await apiRequest(`/api/v1/admin/media-library?page=${p}&per_page=18&q=${q}`);
    if (!resp.ok) return;
    state.mediaLibrary = unwrapList(resp.payload, "items");
    renderMediaLibrary();
    renderPaginationControls("pagination-media-library", resp.meta, loadMediaLibrary);
  }

  function renderMediaLibrary() {
    const grid = byId("media-library-grid");
    if (!grid) return;
    if (!state.mediaLibrary.length) {
      grid.innerHTML = `<div class="col-span-full text-center p-4 text-muted">${esc(t("admin.state.no_match_media", "Eşleşen medya dosyası bulunamadı."))}</div>`;
      return;
    }
    const canEdit = ctx.hasPerm("media:upload|contents:write");
    const canDel = ctx.hasPerm("media:delete|contents:delete");
    grid.innerHTML = state.mediaLibrary.map((m) => {
      const path = m.path || m.url || "";
      const src = path.startsWith("/") || path.startsWith("http") ? path : "/" + path;
      const mime = String(m.mime_type || "");
      const isImg = mime.startsWith("image/");
      const extLabel = (mime.split("/")[1] || path.split(".").pop() || "dosya").toUpperCase();
      const sizeKB = ((Number(m.file_size || 0)) / 1024).toFixed(1);
      const displayName = (m.original_name || path.split("/").pop() || path);
      const thumb = isImg
        ? `<img src="${esc(src)}" alt="${esc(m.alt_text_tr || displayName)}" loading="lazy">`
        : `<div class="img-placeholder text-muted">${svgIcon("file-o")}</div>`;
      const ops = (canEdit || canDel) && m.id ? `<div class="flex gap-2 mt-2">
        ${canEdit ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-media" data-id="${esc(m.id)}" title="${esc(t("admin.action.edit_alt_tags", "Alt Metin & Etiket"))}">${svgIcon("pencil")}</button>` : ""}
        ${canDel ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-media" data-id="${esc(m.id)}" title="${esc(t("admin.action.delete", "Sil"))}">${svgIcon("trash")}</button>` : ""}
      </div>` : "";
      return `<div class="media-card" data-admin-action="copy-media-url" data-url="${esc(path)}" title="${esc(displayName)} — Adresi kopyalamak için tıklayın">
        ${thumb}
        <div class="media-meta">
          <span class="media-filename">${esc(displayName)}</span>
          <div class="media-badge-row"><span>${esc(extLabel)}</span><span>${esc(sizeKB)} KB</span></div>
        </div>
        ${ops}
      </div>`;
    }).join("");
  }

  function populateRecRuleDatalists() {
    const srcList = byId("rec-source-tag-list");
    if (srcList) {
      const seen = new Set(["*"]);
      const opts = [`<option value="*">${esc(t("admin.taxonomy.all_sources", "Tüm etiket ve kategoriler", "All tags and categories"))}</option>`];
      (state.ssbTags || []).forEach((tag) => {
        if (tag?.code && !seen.has(tag.code)) {
          seen.add(tag.code);
          opts.push(`<option value="${esc(tag.code)}">${esc(tag.name_tr || tag.code)} (SSB)</option>`);
        }
      });
      (state.categories || []).forEach((cat) => {
        if (cat?.slug && !seen.has(cat.slug)) {
          seen.add(cat.slug);
          opts.push(`<option value="${esc(cat.slug)}">${esc(cat.name_tr || cat.slug)}</option>`);
        }
      });
      srcList.innerHTML = opts.join("");
    }
    const slugList = byId("rec-content-slug-list");
    if (slugList) {
      slugList.innerHTML = (state.contents || [])
        .filter((c) => c?.slug)
        .slice(0, 120)
        .map((c) => `<option value="${esc(c.slug)}">${esc(c.title_tr || c.slug)}</option>`)
        .join("");
    }
  }

  function populateSeoPreviewSelect() {
    const sel = byId("seo-preview-select");
    if (!sel) { populateRecRuleDatalists(); return; }
    const prev = sel.value;
    sel.innerHTML = `<option value="">${esc(t("admin.form.select_content", "-- İçerik seçin --"))}</option>` +
      state.contents.map((c) => `<option value="${esc(c.id)}">${esc(c.title_tr || c.slug)} (${esc(c.content_type || "haber")})</option>`).join("");
    if (prev && [...sel.options].some((o) => o.value === prev)) {
      sel.value = prev;
    } else if (state.contents.length) {
      sel.value = String(state.contents[0].id);
      updateSeoPreview();
    }
    populateRecRuleDatalists();
  }

  function updateSeoPreview() {
    const c = state.contents.find((x) => Number(x.id) === parseInt(val("seo-preview-select") || "0", 10));
    if (!c) return;
    const title = c.title_tr || c.slug || "Başlıksız";
    const desc = String(c.summary_tr || c.body_tr || "").replace(/<[^>]*>/g, "").substring(0, 160);
    const url = `msidergisi.com/icerik/${c.slug || c.id}`;
    if (byId("seo-google-title")) byId("seo-google-title").textContent = title;
    if (byId("seo-google-url")) byId("seo-google-url").textContent = url;
    if (byId("seo-google-desc")) byId("seo-google-desc").textContent = desc || t("admin.seo.no_description", "Açıklama metni bulunmuyor.");
    if (byId("seo-og-title")) byId("seo-og-title").textContent = title;
    if (byId("seo-og-desc")) byId("seo-og-desc").textContent = desc || t("admin.seo.no_description", "Açıklama metni bulunmuyor.");
    const imgWrap = byId("seo-og-img-wrap");
    if (imgWrap) {
      if (c.cover_image) {
        const src = String(c.cover_image).startsWith("/") || String(c.cover_image).startsWith("http") ? c.cover_image : "/" + c.cover_image;
        imgWrap.innerHTML = `<img src="${esc(src)}" alt="${esc(title)}" loading="lazy">`;
      } else {
        imgWrap.textContent = t("admin.seo.cover_preview", "Kapak görseli önizlemesi");
      }
    }
    if (byId("seo-jsonld")) {
      byId("seo-jsonld").textContent = JSON.stringify({
        "@context": "https://schema.org", "@type": "NewsArticle", headline: title, description: desc,
        url: `https://${url}`, datePublished: c.published_at || c.created_at || new Date().toISOString(),
        author: { "@type": "Organization", name: c.author_name || "MSI Dergisi" },
      }, null, 2);
    }
  }

  async function loadRecommendationRules() {
    populateRecRuleDatalists();
    if (!ctx.hasPerm("contents:read|settings:read|ads:read")) return;
    const resp = await apiRequest("/api/v1/admin/recommendation-rules");
    if (!resp.ok) return;
    state.recommendationRules = unwrapList(resp.payload, "rules");
    renderRecommendationRules();
  }

  function renderRecommendationRules() {
    const tbody = byId("tbody-recommendation-rules");
    if (!tbody) return;
    const list = state.recommendationRules || [];
    if (!list.length) {
      renderTableState(tbody, "empty", { colCount: 7, emptyMsg: t("admin.state.no_recommendation_rules", "Kayıtlı öneri kuralı bulunamadı.", "No recommendation rules configured yet.") });
      return;
    }
    const canWrite = ctx.hasPerm("contents:write|contents:manage|settings:write");
    tbody.innerHTML = list.map((r) => {
      const stBadge = r.is_active !== false
        ? `<span class="badge-tag badge-status-published">${esc(t("admin.status.active", "Aktif", "Active"))}</span>`
        : `<span class="badge-tag badge-status-draft">${esc(t("admin.status.inactive", "Pasif", "Inactive"))}</span>`;
      const ops = canWrite
        ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-rec-rule" data-id="${esc(r.id)}" title="${esc(t("admin.action.edit", "Düzenle", "Edit"))}">${svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-rec-rule" data-id="${esc(r.id)}" title="${esc(t("admin.action.delete", "Sil", "Delete"))}">${svgIcon("trash")}</button></div>`
        : '<span class="text-muted text-xs">—</span>';
      const rawInst = String(r.target_institution || "").trim();
      const instDisp = !rawInst || rawInst.toUpperCase() === "ALL" ? "*" : rawInst;
      return `<tr>
        <td><div class="font-semibold">${esc(r.rule_name)}</div><div class="text-mono text-xs text-muted">#${esc(r.id)}</div></td>
        <td><span class="badge-tag badge-ssb">${esc(r.source_tag_or_cat || "*")}</span></td>
        <td>${esc(instDisp)}</td>
        <td class="text-mono text-xs text-info">${esc(r.recommended_content_slug)}</td>
        <td class="text-mono text-right font-bold">${esc(r.priority ?? 10)}</td>
        <td>${stBadge}</td>
        <td>${ops}</td>
      </tr>`;
    }).join("");
  }

  function resetRecRuleForm() {
    byId("form-recommendation-rule")?.reset();
    setVal("rec-id", "0");
    setVal("rec-source-tag", "*");
    setVal("rec-target-inst", "*");
    setVal("rec-priority", "10");
    if (byId("rec-is-active")) byId("rec-is-active").checked = true;
    const modeBadge = byId("rec-form-mode-badge");
    if (modeBadge) {
      modeBadge.textContent = "";
      modeBadge.classList.add("hidden");
    }
  }

  function loadRecRuleIntoForm(id) {
    const r = (state.recommendationRules || []).find((x) => Number(x.id) === Number(id));
    if (!r) return;
    const det = byId("form-recommendation-rule")?.closest("details");
    if (det) det.open = true;
    const rawInst = String(r.target_institution || "").trim();
    setVal("rec-id", r.id);
    setVal("rec-rule-name", r.rule_name || "");
    setVal("rec-source-tag", r.source_tag_or_cat || "*");
    setVal("rec-target-inst", !rawInst || rawInst.toUpperCase() === "ALL" ? "*" : rawInst);
    setVal("rec-content-slug", r.recommended_content_slug || "");
    setVal("rec-priority", r.priority ?? 10);
    if (byId("rec-is-active")) byId("rec-is-active").checked = r.is_active !== false;
    const modeBadge = byId("rec-form-mode-badge");
    if (modeBadge) {
      modeBadge.textContent = t("admin.editorial.rec_editing_badge", "Düzenleme modu: #${id} — ${name}", "Editing rule: #${id} — ${name}")
        .replace("${id}", String(r.id))
        .replace("${name}", r.rule_name || "");
      modeBadge.classList.remove("hidden");
    }
    byId("rec-rule-name")?.focus();
    showToast(t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", r.rule_name));
  }

  ctx.loadMediaLibrary = loadMediaLibrary;
  ctx.populateSeoPreviewSelect = populateSeoPreviewSelect;
  ctx.loadRecommendationRules = loadRecommendationRules;

  ctx.registerActions({
    "edit-category": (el) => {
      const cat = state.categories.find((x) => Number(x.id) === Number(el.getAttribute("data-id")));
      if (!cat) return;
      const det = byId("form-new-category")?.closest("details");
      if (det) det.open = true;
      setVal("cat-edit-id", cat.id); setVal("cat-name-tr", cat.name_tr); setVal("cat-name-en", cat.name_en);
      setVal("cat-slug", cat.slug); setVal("cat-group-type", cat.group_type || "icerikler");
      setVal("cat-sort-order", cat.sort_order ?? 25); setVal("cat-desc-tr", cat.description_tr || "");
      byId("cat-name-tr")?.focus();
      showToast(t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.").replace("${name}", cat.name_tr));
    },
    "delete-category": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_category", "#${id} kategorisini silmek istediğinize emin misiniz?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/categories/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.category_deleted", "Kategori silindi.")); await loadTaxonomy(); } else showToast(r.errorMessage, true);
    },
    "edit-ssb": (el) => {
      const tag = state.ssbTags.find((x) => x.code === el.getAttribute("data-code"));
      if (!tag) return;
      const det = byId("form-new-ssb")?.closest("details");
      if (det) det.open = true;
      setVal("ssb-edit-old-code", tag.code); setVal("ssb-code", tag.code);
      setVal("ssb-name-tr", tag.name_tr); setVal("ssb-name-en", tag.name_en);
      setVal("ssb-group", tag.category_group || ""); setVal("ssb-sort", tag.sort_order ?? 15);
      byId("ssb-name-tr")?.focus();
      showToast(t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.").replace("${name}", tag.name_tr));
    },
    "delete-ssb": async (el) => {
      const code = el.getAttribute("data-code");
      if (!(await confirmDialog(t("admin.confirm.delete_ssb", "\"${code}\" SSB etiketini silmek istediğinize emin misiniz?").replace("${code}", code)))) return;
      const r = await apiRequest(`/api/v1/admin/ssb-tags/${encodeURIComponent(code)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.ssb_deleted", "SSB etiketi silindi.")); await loadTaxonomy(); } else showToast(r.errorMessage, true);
    },
    "reset-rec-rule-form": resetRecRuleForm,
    "edit-rec-rule": (el) => loadRecRuleIntoForm(el.getAttribute("data-id")),
    "delete-rec-rule": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_rec_rule", "#${id} öneri kuralını silmek istediğinize emin misiniz?", "Are you sure you want to delete recommendation rule #${id}?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/recommendation-rules/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.rec_rule_deleted", "Öneri kuralı silindi.", "Recommendation rule deleted.")); await loadRecommendationRules(); } else showToast(r.errorMessage, true);
    },
    "copy-media-url": (el) => {
      const url = el.getAttribute("data-url") || "";
      if (navigator.clipboard && url) navigator.clipboard.writeText(url).then(() => showToast(t("admin.toast.media_url_copied", "Medya adresi kopyalandı: ") + url));
    },
    "edit-media": (el, ev) => {
      ev?.stopPropagation();
      const m = state.mediaLibrary.find((x) => Number(x.id) === Number(el.getAttribute("data-id")));
      if (!m) return;
      setVal("media-edit-id", m.id); setVal("media-edit-alt-tr", m.alt_text_tr || "");
      setVal("media-edit-alt-en", m.alt_text_en || ""); setVal("media-edit-tags", m.tags || "");
      byId("media-edit-panel")?.classList.remove("hidden");
    },
    "save-media-meta": async () => {
      const id = val("media-edit-id");
      const r = await apiRequest(`/api/v1/admin/media-library/${encodeURIComponent(id)}`, {
        method: "PUT", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ alt_text_tr: val("media-edit-alt-tr"), alt_text_en: val("media-edit-alt-en"), tags: val("media-edit-tags") }),
      });
      if (r.ok) { showToast(t("admin.toast.media_meta_updated", "Medya bilgileri güncellendi.")); byId("media-edit-panel")?.classList.add("hidden"); await loadMediaLibrary(); }
      else showToast(r.errorMessage, true);
    },
    "cancel-media-meta": () => byId("media-edit-panel")?.classList.add("hidden"),
    "delete-media": async (el, ev) => {
      ev?.stopPropagation();
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_media", "#${id} medya kaydını silmek istediğinize emin misiniz?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/media-library/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.media_deleted", "Medya silindi.")); await loadMediaLibrary(); } else showToast(r.errorMessage, true);
    },
  });

  document.addEventListener("submit", async (ev) => {
    const id = ev.target?.id;
    if (id === "form-new-category") {
      ev.preventDefault();
      const editId = parseInt(val("cat-edit-id") || "0", 10);
      const payload = {
        id: editId, slug: val("cat-slug"), parent_slug: null, name_tr: val("cat-name-tr"),
        name_en: val("cat-name-en") || val("cat-name-tr"), group_type: val("cat-group-type") || "icerikler",
        description_tr: val("cat-desc-tr") || "MSI Dergisi özel kategori bölümü",
        description_en: val("cat-name-en") || val("cat-name-tr"), icon_class: "cubes",
        sort_order: parseInt(val("cat-sort-order") || "25", 10), is_dynamic: true,
      };
      const r = await apiRequest(editId > 0 ? `/api/v1/admin/categories/${editId}` : "/api/v1/admin/categories", {
        method: editId > 0 ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload),
      });
      if (r.ok) { showToast(editId > 0 ? t("admin.toast.category_updated", "Kategori güncellendi.") : t("admin.toast.category_created", "Yeni kategori eklendi.")); ev.target.reset(); setVal("cat-edit-id", "0"); await loadTaxonomy(); }
      else showToast(r.errorMessage, true);
    } else if (id === "form-new-ssb") {
      ev.preventDefault();
      const oldCode = val("ssb-edit-old-code");
      const payload = {
        id: 0, code: val("ssb-code"), name_tr: val("ssb-name-tr"), name_en: val("ssb-name-en") || val("ssb-name-tr"),
        category_group: val("ssb-group") || "Savunma Sanayii", sort_order: parseInt(val("ssb-sort") || "15", 10),
      };
      const r = await apiRequest(oldCode ? `/api/v1/admin/ssb-tags/${encodeURIComponent(oldCode)}` : "/api/v1/admin/ssb-tags", {
        method: oldCode ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload),
      });
      if (r.ok) { showToast(oldCode ? t("admin.toast.ssb_updated", "SSB etiketi güncellendi.") : t("admin.toast.ssb_created", "SSB etiketi eklendi.")); ev.target.reset(); setVal("ssb-edit-old-code", ""); await loadTaxonomy(); }
      else showToast(r.errorMessage, true);
    } else if (id === "form-recommendation-rule") {
      ev.preventDefault();
      const ruleName = val("rec-rule-name");
      const recSlug = val("rec-content-slug");
      if (!ruleName || !recSlug) {
        showToast(t("admin.error.rec_rule_required", "Kural adı ve önerilecek içerik slug alanı zorunludur.", "Rule name and recommended content slug are required."), true);
        return;
      }
      const editId = parseInt(val("rec-id") || "0", 10);
      const payload = {
        rule_name: ruleName,
        source_tag_or_cat: val("rec-source-tag") || "*",
        target_institution: val("rec-target-inst") || "*",
        recommended_content_slug: recSlug,
        priority: parseInt(val("rec-priority") || "10", 10),
        is_active: !!byId("rec-is-active")?.checked,
      };
      const r = await apiRequest(editId > 0 ? `/api/v1/admin/recommendation-rules/${editId}` : "/api/v1/admin/recommendation-rules", {
        method: editId > 0 ? "PUT" : "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (r.ok) {
        showToast(editId > 0 ? t("admin.toast.rec_rule_updated", "Öneri kuralı güncellendi.", "Recommendation rule updated.") : t("admin.toast.rec_rule_created", "Öneri kuralı oluşturuldu.", "Recommendation rule created."));
        resetRecRuleForm();
        await loadRecommendationRules();
      } else showToast(r.errorMessage, true);
    }
  });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "media-search") {
      clearTimeout(_mediaSearchTimer);
      _mediaSearchTimer = setTimeout(() => loadMediaLibrary(1), 220);
    }
  });

  document.addEventListener("change", async (ev) => {
    const el = ev.target;
    if (el?.id === "seo-preview-select") updateSeoPreview();
    else if (el?.id === "media-bulk-upload" && el.files?.length) {
      let count = 0;
      for (const f of el.files) {
        const fd = new FormData(); fd.append("file", f);
        const r = await apiRequest("/api/v1/admin/upload", { method: "POST", body: fd });
        if (r.ok) count++; else showToast(r.errorMessage, true);
      }
      el.value = "";
      if (count > 0) { showToast(t("admin.toast.media_bulk_uploaded", "${count} medya dosyası yüklendi.", "${count} media file(s) uploaded.").replace("${count}", String(count))); await loadMediaLibrary(1); }
    }
  });

  document.addEventListener("keydown", (ev) => {
    if (ev.key !== "Enter" && ev.key !== " ") return;
    const lbl = ev.target?.closest?.('label[for="media-bulk-upload"]');
    if (lbl) {
      ev.preventDefault();
      byId("media-bulk-upload")?.click();
    }
  });

  window.addEventListener("msi:admin:refreshed", populateRecRuleDatalists);
  ctx.registerLoader(() => Promise.all([loadTaxonomy(), loadMediaLibrary(), loadRecommendationRules()]).then(populateRecRuleDatalists));
})();
