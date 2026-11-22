(function () {
  "use strict";
  const ctx = window.MSIAdmin?._ctx;
  if (!ctx) return;
  const { state, paginationState, byId, val, setVal, esc, apiRequest, unwrapList, uploadFile, showToast, renderTableState, renderPaginationControls, renderChipSlice, locale, t, svgIcon, confirmDialog, loadCustomFieldValues, validateCustomFields, saveCustomFieldValues } = ctx;

  const STATUS_LABELS = {
    published: "Yayında", draft: "Taslak", pending_review: "Onay Bekliyor", rejected: "Reddedildi", archived: "Arşiv",
  };
  const STATUS_LABELS_EN = {
    published: "Published", draft: "Draft", pending_review: "Pending Review", rejected: "Rejected", archived: "Archived",
  };
  function statusLabel(st) { return t("admin.status." + st, STATUS_LABELS[st] || st, STATUS_LABELS_EN[st] || st); }
  let _pendingApprovalId = null;
  let _pendingApprovalAction = null;

  function applyEditorRoleConstraints() {
    if (ctx.hasPerm("contents:publish")) return;
    const stEl = byId("c-status");
    const pubOpt = stEl?.querySelector('option[value="published"]');
    if (pubOpt) { pubOpt.disabled = true; pubOpt.textContent = t("admin.status.published_no_perm", "Yayında (Yayın Yetkisi Gerekli)", "Published (Publish Permission Required)"); }
    if (stEl && stEl.value === "published") stEl.value = "draft";
    const saveBtn = byId("btn-content-save") || document.querySelector('button[form="form-content-ssot"][type="submit"]');
    if (saveBtn) saveBtn.innerHTML = svgIcon("save") + " " + esc(t("admin.action.save_draft", "Taslak Olarak Kaydet", "Save as Draft"));
  }

  function toggleSponsoredFields() {
    const chk = byId("c-is-sponsored"), row = byId("c-sponsor-extra-row");
    if (chk && row) row.classList.toggle("hidden", !chk.checked);
  }

  async function loadContentsPage(page) {
    const p = typeof page === "number" ? page : paginationState.contents || 1;
    paginationState.contents = p;
    const url = ctx.hasPerm("contents:read")
      ? `/api/v1/admin/contents?page=${p}&per_page=20`
      : `/api/v1/contents?page=${p}&per_page=20`;
    const res = await apiRequest(url);
    if (!res.ok || !res.payload) {
      if (ctx.hasPerm("contents:read")) renderTableState(byId("tbody-contents"), "error", { colCount: 6, errorMsg: res.errorMessage, retryAction: "retry-contents" });
      return;
    }
    state.contents = unwrapList(res.payload, "contents");
    if (res.meta?.total_count > 0) state.totalContentsCount = res.meta.total_count;
    renderContentsTable();
    renderBulkContentsTable();
    ctx.renderKpis?.();
    ctx.populateSeoPreviewSelect?.();
    renderPaginationControls("pagination-contents", res.meta, loadContentsPage);
  }

  function renderContentsTable() {
    const tbody = byId("tbody-contents");
    if (!tbody) return;
    const q = val("filter-content-search").toLowerCase();
    const stFilter = val("filter-content-status");
    const typeFilter = val("filter-content-type");
    const filtered = state.contents.filter((c) => {
      if (stFilter && (c.status || "published") !== stFilter) return false;
      if (typeFilter && (c.content_type || "haberler") !== typeFilter) return false;
      if (!q) return true;
      return [c.title_tr, c.title_en, c.content_type, c.status, c.issue_number, ...(c.placements || []), ...(c.ssb_tags || [])]
        .some((f) => String(f || "").toLowerCase().includes(q));
    });
    if (!filtered.length) { renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_match_filter", "Filtreyle eşleşen içerik bulunamadı.") }); return; }

    const canWrite = ctx.hasPerm("contents:write");
    const canDel = ctx.hasPerm("contents:delete");
    const canPub = ctx.hasPerm("contents:publish");
    const adminBase = ((document.body && document.body.dataset && document.body.dataset.adminPath) || "/podmin").trim();
    const editorBase = `${adminBase}/editor`;

    tbody.innerHTML = filtered.map((c) => {
      const st = c.status || "published";
      const vis = Number(c.visibility_level || 0);
      const pwLbl = vis === 2 ? `${esc(t("admin.paywall.subscriber", "Abone"))} / ₺${esc(c.single_price_try || 0)}` : vis === 1 ? esc(t("admin.paywall.registered_subscriber", "Kayıtlı & Abone")) : esc(t("admin.paywall.open", "Açık Erişim"));
      const spHtml = c.is_sponsored ? `<div class="mt-1"><span class="badge-tag badge-sponsored">${svgIcon("shield")} ${esc(c.sponsor_brand || "SPONSORLU")}</span></div>` : "";
      const chips = [renderChipSlice(c.placements, "badge-placement"), renderChipSlice(c.ssb_tags, "badge-ssb")].filter(Boolean).join('<div class="mt-1"></div>') || '<span class="text-muted">—</span>';
      const showReview = canWrite && st !== "pending_review" && (st !== "published" || !canPub);
      const ops = [
        canWrite ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-content" data-id="${esc(c.id)}">${svgIcon("pencil")} ${esc(t("admin.action.edit", "Düzenle"))}</button>` : "",
        canWrite ? `<a href="${esc(editorBase)}?content_id=${encodeURIComponent(c.id)}" class="btn-cmd btn-cmd-sm" title="${esc(t("admin.action.open_studio", "Editör Stüdyosunda Aç", "Open in Editorial Studio"))}">${svgIcon("external-link")} ${esc(t("admin.action.studio", "Stüdyo", "Studio"))}</a>` : "",
        showReview ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-warning" data-admin-action="submit-content-review" data-id="${esc(c.id)}" title="${esc(t("admin.action.submit_review", "Onaya Gönder"))}">${svgIcon("paper-plane")}</button>` : "",
        canDel ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-content" data-id="${esc(c.id)}" title="${esc(t("admin.action.delete", "Sil"))}">${svgIcon("trash")}</button>` : "",
      ].filter(Boolean).join("");
      return `<tr>
        <td class="text-mono text-xs"><strong>#${esc(c.id)}</strong><div class="text-muted">${c.issue_number ? esc(t("admin.label.issue_prefix", "Sayı ", "Issue ")) + esc(c.issue_number) : esc(t("admin.label.digital", "Dijital", "Digital"))}</div></td>
        <td>
          <div class="font-semibold">${c.is_featured ? `${svgIcon("star", "text-warning")} ` : ""}<a href="/icerik/${esc(c.slug)}" target="_blank">${esc(c.title_tr)}</a></div>
          <div class="flex gap-2 mt-1 flex-wrap"><span class="badge-tag badge-status-${esc(st)}">${esc(statusLabel(st))}</span><span class="badge-tag">${esc(c.content_type)}</span></div>
        </td>
        <td>${chips}</td>
        <td><span class="badge-tag badge-paywall-${vis}">${pwLbl}</span>${spHtml}</td>
        <td class="text-mono text-xs text-right"><strong>${Number(c.view_count || 0).toLocaleString(locale())}</strong> ${esc(t("admin.label.views_short", "gör.", "views"))}<div class="text-success">${Number(c.true_read_count || 0).toLocaleString(locale())} ${esc(t("admin.label.reads", "okuma", "reads"))}</div></td>
        <td><div class="flex gap-2">${ops || `<span class="text-muted text-xs">${esc(t("admin.label.read_only", "Salt Okunur", "Read Only"))}</span>`}</div></td>
      </tr>`;
    }).join("");
  }

  function resetContentForm() {
    byId("form-content-ssot")?.reset();
    setVal("c-id", "0");
    setVal("c-published-at", "");
    setVal("c-author-title", "");
    document.querySelectorAll('input[name="c_target_inst"]').forEach((el) => { el.checked = false; });
    const stEl = byId("c-status");
    if (stEl) stEl.value = ctx.hasPerm("contents:publish") ? "published" : "draft";
    if (byId("content-form-heading")) byId("content-form-heading").textContent = t("admin.form.new_content", "Yeni İçerik Oluştur");
    applyEditorRoleConstraints();
    toggleSponsoredFields();
    ctx.switchLangTab(byId("form-content-ssot"), "tr");
    return loadCustomFieldValues("content-custom-fields", "content", "0");
  }

  function loadContentIntoForm(id) {
    const c = state.contents.find((x) => Number(x.id) === Number(id));
    if (!c) return;
    setVal("c-id", c.id); setVal("c-slug", c.slug);
    setVal("c-title-tr", c.title_tr); setVal("c-title-en", c.title_en);
    setVal("c-summary-tr", c.summary_tr); setVal("c-summary-en", c.summary_en);
    setVal("c-body-tr", c.body_tr); setVal("c-body-en", c.body_en);
    setVal("c-content-type", c.content_type || "haberler");
    setVal("c-source-group", c.source_group || "gunluk-haber");
    const desiredStatus = c.status || "published";
    setVal("c-status", !ctx.hasPerm("contents:publish") && desiredStatus === "published" ? "draft" : desiredStatus);
    setVal("c-issue-number", c.issue_number || "");
    setVal("c-visibility-level", String(c.visibility_level || 0));
    setVal("c-single-price", c.single_price_try || 0);
    setVal("c-read-time", c.read_time_min || 5);
    setVal("c-author-name", c.author_name || t("admin.editorial.default_author", "MSI Editoryal Masası"));
    setVal("c-author-title", c.author_title || "");
    setVal("c-published-at", c.published_at || "");
    setVal("c-cover-image", c.cover_image || "images/msi/dergi-286.webp");
    setVal("c-video-url", c.video_url || "");
    if (byId("c-is-sponsored")) byId("c-is-sponsored").checked = !!c.is_sponsored;
    if (byId("c-is-featured")) byId("c-is-featured").checked = !!c.is_featured;
    setVal("c-sponsor-brand", c.sponsor_brand || "");
    setVal("c-sponsor-legal-label", c.sponsor_legal_label || t("admin.editorial.default_sponsor_legal", "SPONSORLU İÇERİK"));
    const pSet = new Set(c.placements || []), sSet = new Set(c.ssb_tags || []), instSet = new Set(c.target_institutions || []);
    document.querySelectorAll('input[name="c_placement"]').forEach((el) => { el.checked = pSet.has(el.value); });
    document.querySelectorAll('input[name="c_ssb"]').forEach((el) => { el.checked = sSet.has(el.value); });
    document.querySelectorAll('input[name="c_target_inst"]').forEach((el) => { el.checked = instSet.has(el.value); });
    if (byId("content-form-heading")) byId("content-form-heading").textContent = `${t("admin.form.edit_content", "İçerik Düzenle")} (#${c.id})`;
    applyEditorRoleConstraints();
    toggleSponsoredFields();
    return loadCustomFieldValues("content-custom-fields", "content", String(c.id));
  }

  function buildContentPayload(statusOverride) {
    const editId = parseInt(val("c-id") || "0", 10);
    const existing = editId > 0 ? state.contents.find((x) => Number(x.id) === editId) : null;
    const isSponsored = !!byId("c-is-sponsored")?.checked;
    const issueRaw = val("c-issue-number");
    const rawSt = statusOverride || val("c-status") || "draft";
    const status = !ctx.hasPerm("contents:publish") && rawSt === "published" ? "draft" : rawSt;
    return {
      id: editId, slug: val("c-slug"),
      title_tr: val("c-title-tr"), title_en: val("c-title-en"),
      summary_tr: val("c-summary-tr"), summary_en: val("c-summary-en"),
      body_tr: val("c-body-tr"), body_en: val("c-body-en"),
      content_type: val("c-content-type") || "haberler",
      source_group: val("c-source-group") || "gunluk-haber",
      cover_image: val("c-cover-image") || "images/msi/dergi-286.webp",
      author_name: val("c-author-name") || t("admin.editorial.default_author", "MSI Editoryal Masası"),
      author_title: val("c-author-title") || existing?.author_title || t("admin.editorial.default_author_title", "Savunma Sanayii Analisti"),
      issue_number: issueRaw ? parseInt(issueRaw, 10) : null,
      published_at: val("c-published-at") || existing?.published_at || new Date().toISOString().slice(0, 16).replace("T", " "),
      read_time_min: parseInt(val("c-read-time") || "5", 10),
      visibility_level: parseInt(val("c-visibility-level") || "0", 10),
      single_price_try: parseFloat(val("c-single-price") || "0"),
      is_sponsored: isSponsored,
      sponsor_brand: isSponsored ? val("c-sponsor-brand") || t("admin.editorial.default_sponsor", "Savunma Sanayii Paydaşı") : null,
      sponsor_legal_label: isSponsored ? val("c-sponsor-legal-label") : null,
      is_featured: !!byId("c-is-featured")?.checked,
      video_url: val("c-video-url") || null,
      view_count: existing?.view_count ?? 0,
      true_read_count: existing?.true_read_count ?? 0,
      placements: Array.from(document.querySelectorAll('input[name="c_placement"]:checked')).map((e) => e.value),
      ssb_tags: Array.from(document.querySelectorAll('input[name="c_ssb"]:checked')).map((e) => e.value),
      target_institutions: Array.from(document.querySelectorAll('input[name="c_target_inst"]:checked')).map((e) => e.value),
      status,
    };
  }

  async function saveContent(statusOverride, sendReview) {
    const form = byId("form-content-ssot");
    const payload = buildContentPayload(statusOverride);
    if (!payload.title_tr || !payload.summary_tr || !payload.body_tr) {
      ctx.switchLangTab(form, "tr");
      showToast(t("admin.toast.required_tr_fields", "Lütfen zorunlu Türkçe başlık, özet ve gövde alanlarını doldurun."), true);
      return;
    }
    if (validateCustomFields && !validateCustomFields("content-custom-fields")) return;
    const resp = await apiRequest("/api/v1/admin/contents", {
      method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload),
    });
    if (!resp.ok) { showToast(resp.errorMessage, true); return; }
    const savedId = resp.data?.id || payload.id;
    if (savedId) {
      const cfRes = await saveCustomFieldValues("content-custom-fields", "content", String(savedId));
      if (cfRes && !cfRes.ok) {
        setVal("c-id", String(savedId));
        showToast(cfRes.errorMessage || t("admin.toast.custom_fields_save_failed", "Özel alan değerleri kaydedilemedi.", "Failed to save custom field values."), true);
        await ctx.refreshAllData();
        return;
      }
    }
    if ((sendReview || payload.status === "pending_review") && savedId) {
      await apiRequest(`/api/v1/admin/contents/${encodeURIComponent(savedId)}/submit-review`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: "{}",
      });
    }
    showToast(sendReview || payload.status === "pending_review" ? t("admin.toast.content_saved_review", "İçerik kaydedildi ve onaya gönderildi.") : payload.status === "draft" ? t("admin.toast.content_saved_draft", "İçerik taslak olarak kaydedildi.") : t("admin.toast.content_saved_published", "İçerik kaydedildi ve yayına alındı."));
    resetContentForm();
    ctx.closeDrawer("drawer-content");
    await ctx.refreshAllData();
  }

  function updateBulkCount() {
    if (byId("bulk-selected-count")) byId("bulk-selected-count").textContent = String(state.bulkSelected.size);
  }

  function renderBulkContentsTable() {
    const tbody = byId("tbody-bulk-contents");
    if (!tbody) return;
    const q = val("bulk-search-input").toLowerCase();
    const list = state.contents.filter((c) => !q || String(c.title_tr || c.slug || "").toLowerCase().includes(q));
    if (byId("bulk-select-all-cb")) byId("bulk-select-all-cb").checked = false;
    const visLbl = { 0: t("admin.paywall.open", "Açık Erişim"), 1: t("admin.paywall.registered", "Kayıtlı Üye"), 2: t("admin.paywall.subscriber", "Abone") };
    if (!list.length) {
      renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_match_content", "Eşleşen içerik bulunamadı.") });
    } else {
      tbody.innerHTML = list.map((c) => `<tr>
        <td class="w-checkbox"><input type="checkbox" class="bulk-cb" data-id="${esc(c.id)}" ${state.bulkSelected.has(Number(c.id)) ? "checked" : ""}></td>
        <td class="text-mono text-xs">#${esc(c.id)}</td>
        <td class="font-semibold">${esc(c.title_tr || c.slug)}</td>
        <td><span class="badge-tag">${esc(c.content_type || "haberler")}</span></td>
        <td class="text-xs">${esc(visLbl[c.visibility_level] ?? c.visibility_level)}</td>
        <td class="text-mono text-xs">${Number(c.view_count || 0).toLocaleString(locale())}</td>
      </tr>`).join("");
    }
    const sel = byId("version-history-select");
    if (sel) {
      const prev = sel.value;
      sel.innerHTML = `<option value="">${esc(t("admin.form.select_content", "-- İçerik seçin --"))}</option>` +
        state.contents.map((c) => `<option value="${esc(c.id)}">${esc(c.title_tr || c.slug)}</option>`).join("");
      if (prev) sel.value = prev;
    }
  }

  async function loadVersionHistory(page) {
    const cid = parseInt(val("version-history-select") || "0", 10);
    const box = byId("version-history-list");
    if (!box || !cid) return;
    paginationState.versionHistory = typeof page === "number" ? page : 1;
    const resp = await apiRequest(`/api/v1/admin/contents/${cid}/versions?page=${paginationState.versionHistory}&per_page=25`);
    const versions = resp.ok ? unwrapList(resp.payload, "versions") : [];
    if (!versions.length) {
      box.innerHTML = `<div class="text-muted text-center p-4 text-sm">${esc(t("admin.editorial.no_version_history", "Bu içerik için sürüm geçmişi kaydı bulunamadı."))}</div>`;
      renderPaginationControls("pagination-version-history", null, loadVersionHistory);
      return;
    }
    const canRestore = ctx.hasPerm("contents:write");
    box.innerHTML = versions.map((v) => {
      const ts = v.created_at ? new Date(v.created_at).toLocaleString(locale()) : "—";
      const btn = canRestore ? `<div class="mt-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="restore-version" data-content-id="${esc(cid)}" data-version="${esc(v.version)}">${svgIcon("undo")} ${esc(t("admin.action.restore_version", "Bu Sürüme Dön"))}</button></div>` : "";
      return `<div class="p-3 border-bottom text-xs">
        <div class="flex justify-between mb-1"><strong class="text-info">v${esc(v.version)}</strong><span class="text-muted">${esc(ts)}</span></div>
        <div class="font-semibold mb-1">${esc(v.title_tr)}</div>
        <div class="text-muted">${svgIcon("edit")} ${esc(v.changed_by)} · ${esc(v.change_note || t("admin.label.no_note", "Not yok"))}</div>${btn}
      </div>`;
    }).join("");
    renderPaginationControls("pagination-version-history", resp.meta, loadVersionHistory);
  }

  async function loadPendingApprovals() {
    if (!ctx.hasPerm("approvals:read|approvals:manage|approvals:approve|contents:publish")) return;
    const tbody = byId("tbody-pending-approvals"), badge = byId("approval-badge");
    const resp = await apiRequest("/api/v1/admin/approvals/pending");
    if (!resp.ok) return;
    const items = unwrapList(resp.payload, "approvals");
    if (badge) { badge.textContent = String(items.length); badge.classList.toggle("hidden", items.length === 0); }
    if (!tbody) return;
    if (!items.length) { renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_pending_approvals", "Şu anda onay bekleyen yayın bulunmuyor.") }); return; }
    const canAct = ctx.hasPerm("approvals:approve|approvals:manage|contents:publish");
    tbody.innerHTML = items.map((item) => {
      const appId = item.id ?? item.approval_id;
      const dt = item.requested_at ? new Date(item.requested_at).toLocaleString(locale()) : "—";
      const btns = canAct
        ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-emerald" data-admin-action="open-approval-modal" data-id="${esc(appId)}" data-decision="approve">${svgIcon("check")} ${esc(t("admin.action.approve", "Onayla", "Approve"))}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="open-approval-modal" data-id="${esc(appId)}" data-decision="reject">${svgIcon("times")} ${esc(t("admin.action.reject", "Reddet", "Reject"))}</button></div>`
        : `<span class="text-muted text-xs">${esc(t("admin.label.in_review", "İncelemede", "In Review"))}</span>`;
      return `<tr>
        <td><strong>${esc(item.title_tr)}</strong><br><span class="text-muted text-xs text-mono">/${esc(item.slug || "")}</span></td>
        <td>${esc(item.author_name || "—")}</td>
        <td><span class="badge-tag">${esc(item.content_type || "—")}</span></td>
        <td class="text-xs">${esc(dt)}</td>
        <td><span class="badge-tag badge-status-pending_review">${esc(t("admin.status.pending", "Onay Bekliyor", "Pending Review"))}</span></td>
        <td>${btns}</td>
      </tr>`;
    }).join("");
  }

  ctx.registerActions({
    "retry-contents": () => loadContentsPage(),
    "open-content-drawer": () => { resetContentForm(); ctx.openDrawer("drawer-content"); },
    "hero-open-content-drawer": () => { resetContentForm(); ctx.openDrawer("drawer-content"); },
    "reset-content-form": resetContentForm,
    "edit-content": (el) => { loadContentIntoForm(el.getAttribute("data-id")); ctx.openDrawer("drawer-content"); },
    "trigger-cover-upload": () => byId("c-cover-file")?.click(),
    "submit-form-for-review": () => saveContent("draft", true),
    "submit-content-review": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.submit_review", "#${id} numaralı içerik editöryal onaya gönderilsin mi?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/contents/${encodeURIComponent(id)}/submit-review`, { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
      if (r.ok) { showToast(t("admin.toast.content_submitted_review", "İçerik #${id} onaya gönderildi.").replace("${id}", id)); await ctx.refreshAllData(); } else showToast(r.errorMessage, true);
    },
    "delete-content": async (el) => {
      const id = el.getAttribute("data-id");
      if (!(await confirmDialog(t("admin.confirm.delete_content", "#${id} numaralı içerik silinecek. Emin misiniz?").replace("${id}", id)))) return;
      const r = await apiRequest(`/api/v1/admin/contents/${encodeURIComponent(id)}`, { method: "DELETE" });
      if (r.ok) { showToast(t("admin.toast.content_deleted", "İçerik #${id} silindi.").replace("${id}", id)); await ctx.refreshAllData(); } else showToast(r.errorMessage, true);
    },
    "bulk-select-all": () => {
      document.querySelectorAll(".bulk-cb").forEach((cb) => { cb.checked = true; state.bulkSelected.add(Number(cb.dataset.id)); });
      updateBulkCount();
    },
    "bulk-deselect-all": () => {
      state.bulkSelected.clear();
      document.querySelectorAll(".bulk-cb").forEach((cb) => { cb.checked = false; });
      updateBulkCount();
    },
    "execute-bulk": async () => {
      if (!state.bulkSelected.size) return showToast(t("admin.toast.select_at_least_one", "Lütfen en az bir içerik seçin."), true);
      const raw = val("bulk-action-select");
      if (!raw) return showToast(t("admin.toast.select_action", "Lütfen uygulanacak işlemi seçin."), true);
      let action = raw, value = null;
      if (raw.startsWith("set_visibility_")) { action = "set_visibility"; value = raw.split("_")[2]; }
      else if (raw === "set_featured") value = "true";
      else if (raw === "unset_featured") { action = "set_featured"; value = "false"; }
      if (action === "delete" && !(await confirmDialog(t("admin.confirm.bulk_delete", "${count} içerik kalıcı olarak silinecek. Emin misiniz?").replace("${count}", state.bulkSelected.size)))) return;
      const r = await apiRequest("/api/v1/admin/contents/bulk", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ ids: [...state.bulkSelected], action, value }) });
      if (r.ok) { showToast(t("admin.toast.bulk_complete", "Toplu işlem tamamlandı: ${count} kayıt.").replace("${count}", r.data?.affected ?? state.bulkSelected.size)); await ctx.refreshAllData(); } else showToast(r.errorMessage, true);
    },
    "restore-version": async (el) => {
      const cid = el.getAttribute("data-content-id"), ver = el.getAttribute("data-version");
      if (!(await confirmDialog(t("admin.confirm.restore_version", "İçerik #${cid}, v${ver} sürümüne geri döndürülsün mü?").replace("${cid}", cid).replace("${ver}", ver)))) return;
      const r = await apiRequest(`/api/v1/admin/contents/${cid}/versions/${ver}/restore`, { method: "POST" });
      if (r.ok) { showToast(t("admin.toast.version_restored", "v${ver} sürümü geri yüklendi.").replace("${ver}", ver)); await ctx.refreshAllData(); } else showToast(r.errorMessage, true);
    },
    "load-pending-approvals": loadPendingApprovals,
    "open-approval-modal": (el) => {
      _pendingApprovalId = Number(el.getAttribute("data-id"));
      _pendingApprovalAction = el.getAttribute("data-decision");
      setVal("approval-note", "");
      const ok = _pendingApprovalAction === "approve";
      if (byId("approval-modal-title")) byId("approval-modal-title").textContent = ok ? t("admin.form.approve_and_publish", "İçeriği Onayla ve Yayınla") : t("admin.form.reject_content", "İçeriği Reddet");
      const btn = byId("approval-confirm-btn");
      if (btn) { btn.textContent = ok ? t("admin.action.approve_and_publish", "Onayla ve Yayınla") : t("admin.action.reject", "Reddet"); btn.className = ok ? "btn-cmd btn-cmd-emerald" : "btn-cmd btn-cmd-danger"; }
      ctx.openModal?.("approval-modal", byId("approval-note"));
    },
    "close-approval-modal": () => { ctx.closeModal?.("approval-modal"); _pendingApprovalId = null; },
    "confirm-approval": async () => {
      if (!_pendingApprovalId || !_pendingApprovalAction) return;
      const act = _pendingApprovalAction;
      const r = await apiRequest(`/api/v1/admin/approvals/${_pendingApprovalId}/${act === "approve" ? "approve" : "reject"}`, {
        method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ note: val("approval-note") }),
      });
      if (r.ok) {
        ctx.closeModal?.("approval-modal");
        _pendingApprovalId = null;
        showToast(act === "approve" ? t("admin.toast.content_approved", "İçerik onaylandı ve yayına alındı.") : t("admin.toast.content_rejected", "İçerik reddedildi."));
        await ctx.refreshAllData();
      } else showToast(r.errorMessage, true);
    },
  });

  document.addEventListener("submit", (ev) => {
    if (ev.target?.id === "form-content-ssot") { ev.preventDefault(); saveContent(null, false); }
  });
  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "filter-content-search") renderContentsTable();
    else if (ev.target?.id === "bulk-search-input") renderBulkContentsTable();
  });
  document.addEventListener("change", async (ev) => {
    const el = ev.target;
    if (!el) return;
    if (el.id === "filter-content-status" || el.id === "filter-content-type") renderContentsTable();
    else if (el.id === "c-is-sponsored") toggleSponsoredFields();
    else if (el.id === "c-cover-file") {
      const u = await uploadFile(el);
      if (u) { setVal("c-cover-image", u); showToast(t("admin.toast.cover_uploaded", "Kapak görseli yüklendi.", "Cover image uploaded.")); ctx.loadMediaLibrary?.(); }
    } else if (el.id === "bulk-select-all-cb") {
      document.querySelectorAll(".bulk-cb").forEach((cb) => { cb.checked = el.checked; const id = Number(cb.dataset.id); if (el.checked) state.bulkSelected.add(id); else state.bulkSelected.delete(id); });
      updateBulkCount();
    } else if (el.classList?.contains("bulk-cb")) {
      const id = Number(el.dataset.id);
      if (el.checked) state.bulkSelected.add(id); else state.bulkSelected.delete(id);
      updateBulkCount();
    } else if (el.id === "version-history-select") loadVersionHistory(1);
  });

  ctx.registerLoader(async () => {
    applyEditorRoleConstraints();
    await Promise.all([loadContentsPage(), loadPendingApprovals()]);
  });
})();
