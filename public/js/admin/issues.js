(function () {
  "use strict";
  const ctx = window.MSIAdmin?._ctx;
  if (!ctx) return;
  const { state, paginationState, byId, val, setVal, esc, apiRequest, unwrapList, uploadFile, showToast, renderTableState, renderPaginationControls, t, svgIcon, confirmDialog } = ctx;

  let pubFilter = "all";
  let editingTarget = null;
  const PER_PAGE = 12;

  function normPubType(raw) {
    return String(raw || "msi").trim().toLowerCase() === "msi_tdr" ? "msi_tdr" : "msi";
  }

  function findIssueByKey(num, pubType) {
    const n = Number(num);
    const pt = normPubType(pubType);
    return state.issues.find((i) => Number(i.issue_number) === n && normPubType(i.publication_type) === pt);
  }

  function ensureConverterControls() {
    const form = byId("form-issue");
    if (!form || byId("iss-auto-convert")) return;
    const box = document.createElement("div");
    box.id = "iss-converter-box";
    box.className = "form-group mt-2";
    box.innerHTML = `
      <div class="flex flex-wrap items-center justify-between gap-2 p-3 border rounded-md">
        <label class="flex items-center gap-2 text-xs font-semibold cursor-pointer">
          <input type="checkbox" id="iss-auto-convert" checked>
          <span>${esc(t("admin.issues.auto_convert", "PDF yüklenince / kaydedilince otomatik Web E-Dergi'ye dönüştür", "Auto-convert to Web E-Dergi on save"))}</span>
        </label>
        <button type="button" id="btn-iss-convert-now" class="btn-cmd btn-cmd-sm" data-admin-action="convert-drawer-pdf" data-convert-pdf="1">
          ${svgIcon("magic")} ${esc(t("admin.issues.convert_now", "PDF → Web E-Dergi Dönüştür", "Convert PDF → Web E-Dergi"))}
        </button>
      </div>
      <div id="iss-convert-status" class="text-xs text-muted mt-1"></div>
    `;
    form.appendChild(box);
  }

  function updateDrawerConvertStatus(iss) {
    const st = byId("iss-convert-status");
    if (!st) return;
    if (!iss) {
      st.textContent = t("admin.issues.convert_hint_new", "PDF yüklendikten sonra tek tıkla sayfa önbelleği ve içindekiler üretilir.", "After uploading a PDF, generate page cache and TOC in one click.");
      return;
    }
    const spreadsLen = Array.isArray(iss.spreads) ? iss.spreads.length : 0;
    st.textContent = spreadsLen > 0
      ? `${t("admin.issues.converted_state", "Web E-Dergi hazır", "Web E-Dergi ready")}: ${iss.page_count || 0} ${t("admin.issues.pages_unit", "sayfa")} · ${spreadsLen} ${t("admin.issues.sections_unit", "bölüm")}`
      : t("admin.issues.not_converted_state", "Henüz Web E-Dergi'ye dönüştürülmedi.", "Not yet converted to Web E-Dergi.");
  }

  function populateIssueDatalist() {
    const dl = byId("c-issue-datalist");
    if (!dl) return;
    dl.innerHTML = state.issues
      .map((iss) => `<option value="${esc(iss.issue_number)}">${normPubType(iss.publication_type) === "msi_tdr" ? "MSI TDR" : "MSI"} #${esc(iss.issue_number)} — ${esc(iss.title_tr)}</option>`)
      .join("");
  }

  function renderIssuesTable(page) {
    const tbody = byId("tbody-admin-issues");
    if (!tbody) return;
    if (typeof page === "number") paginationState.issues = page;
    const curPage = paginationState.issues || 1;
    const q = val("iss-search").toLowerCase();
    const filtered = state.issues.filter((i) => {
      if (pubFilter !== "all" && normPubType(i.publication_type) !== pubFilter) return false;
      if (!q) return true;
      return [i.issue_number, i.title_tr, i.title_en, i.publish_month_tr, i.year, i.event_name]
        .some((f) => String(f || "").toLowerCase().includes(q));
    });
    if (!filtered.length) {
      renderTableState(tbody, "empty", { colCount: 6, emptyMsg: t("admin.state.no_match_issue", "Eşleşen dergi sayısı bulunamadı.") });
      renderPaginationControls("pagination-issues", null, renderIssuesTable);
      return;
    }
    const totalPages = Math.max(1, Math.ceil(filtered.length / PER_PAGE));
    const safePage = Math.min(curPage, totalPages);
    const slice = filtered.slice((safePage - 1) * PER_PAGE, safePage * PER_PAGE);
    const canWrite = ctx.hasPerm("issues:write|issues:manage");
    const canDelete = ctx.hasPerm("issues:delete|issues:manage");

    tbody.innerHTML = slice.map((iss) => {
      const pubType = normPubType(iss.publication_type);
      const pubBadge = pubType === "msi_tdr"
        ? '<span class="badge-tag badge-placement">MSI TDR</span>'
        : '<span class="badge-tag badge-paywall-0">MSI Dergisi</span>';
      const coverSrc = iss.cover_image
        ? (String(iss.cover_image).startsWith("/") || String(iss.cover_image).startsWith("http") ? iss.cover_image : "/" + iss.cover_image)
        : "/images/msi/dergi-286.webp";
      const spreadsCount = Array.isArray(iss.spreads) ? iss.spreads.length : 0;
      const webBadge = spreadsCount > 0
        ? `<div class="mt-1"><span class="badge-tag badge-paywall-0">Web E-Dergi (${esc(spreadsCount)})</span></div>`
        : `<div class="mt-1"><span class="badge-tag">${esc(t("admin.issues.not_converted", "Dönüştürülmedi", "Not converted"))}</span></div>`;
      const pdfCell = iss.uploaded_pdf_file
        ? `<a href="/api/v1/admin/edergi-files/${encodeURIComponent(iss.uploaded_pdf_file)}" target="_blank" class="pdf-pill" title="${esc(iss.uploaded_pdf_file)}">${svgIcon("file-pdf-o")} ${esc(t("admin.issues.pdf_preview", "PDF Önizle", "Preview PDF"))}</a>${webBadge}`
        : `<span class="badge-tag">${esc(t("admin.issues.dynamic_drm", "Dinamik DRM", "Dynamic DRM"))}</span>${webBadge}`;
      const editBtns = [
        canWrite ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-issue" data-issue-number="${esc(iss.issue_number)}" data-publication-type="${esc(pubType)}" title="${esc(t("admin.action.edit", "Düzenle", "Edit"))}">${svgIcon("pencil")}</button>` : "",
        canWrite && iss.uploaded_pdf_file ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="convert-issue-pdf" data-convert-pdf="1" data-issue-number="${esc(iss.issue_number)}" data-publication-type="${esc(pubType)}" title="${esc(t("admin.issues.convert_btn", "PDF → Web E-Dergi", "PDF → Web E-Dergi"))}">${svgIcon("magic")}</button>` : "",
        canDelete ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-issue" data-issue-number="${esc(iss.issue_number)}" data-publication-type="${esc(pubType)}" title="${esc(t("admin.action.delete", "Sil", "Delete"))}">${svgIcon("trash")}</button>` : "",
      ].filter(Boolean).join("");
      return `<tr>
        <td class="text-mono font-bold">#${esc(iss.issue_number)}</td>
        <td>${pubBadge}<div class="text-xs text-muted mt-1">${esc(iss.publish_month_tr)} ${esc(iss.year)}</div></td>
        <td>
          <div class="issue-row-title">
            <img src="${esc(coverSrc)}" alt="Sayı ${esc(iss.issue_number)}" class="issue-cover-thumb" loading="lazy">
            <div>
              <div class="font-semibold">${esc(iss.title_tr)}</div>
              <div class="text-xs text-muted">${esc(iss.title_en)}</div>
            </div>
          </div>
        </td>
        <td class="text-xs font-semibold">${esc(iss.page_count)} ${esc(t("admin.issues.pages_unit", "sayfa"))} · ${esc(iss.dossier_count)} ${esc(t("admin.issues.dossiers_unit", "dosya"))}</td>
        <td>${pdfCell}</td>
        <td>
          <div class="flex gap-2">
            ${editBtns}
            <a href="/e-dergi?issue=${esc(iss.issue_number)}&publication_type=${encodeURIComponent(pubType)}" target="_blank" class="btn-cmd btn-cmd-sm" title="${esc(t("admin.issues.open_kiosk", "Dijital Kiosk'ta Aç", "Open in Digital Kiosk"))}">${svgIcon("external-link")}</a>
          </div>
        </td>
      </tr>`;
    }).join("");

    renderPaginationControls(
      "pagination-issues",
      { page: safePage, total_pages: totalPages, total_count: filtered.length },
      renderIssuesTable,
    );
  }

  async function loadIssues() {
    if (!ctx.hasPerm("issues:read|issues:delete|contents:read")) return;
    const resp = await apiRequest("/api/v1/admin/issues");
    if (!resp.ok) return;
    state.issues = unwrapList(resp.payload, "issues");
    populateIssueDatalist();
    renderIssuesTable(1);
  }

  function resetIssueForm() {
    editingTarget = null;
    byId("form-issue")?.reset();
    if (byId("iss-number")) byId("iss-number").disabled = false;
    if (byId("iss-pub-type")) byId("iss-pub-type").disabled = false;
    const lockHint = byId("iss-key-lock-hint");
    if (lockHint) { lockHint.textContent = ""; lockHint.classList.add("hidden"); }
    setVal("iss-pdf-file", "");
    ensureConverterControls();
    updateDrawerConvertStatus(null);
    if (byId("issue-form-heading")) byId("issue-form-heading").textContent = t("admin.form.new_issue", "Yeni Dergi Sayısı Tanımla");
    ctx.switchLangTab(byId("form-issue"), "tr");
  }

  function loadIssueIntoForm(num, pubType) {
    ensureConverterControls();
    const iss = findIssueByKey(num, pubType) || state.issues.find((i) => Number(i.issue_number) === Number(num));
    if (!iss) return;
    const normPub = normPubType(iss.publication_type);
    editingTarget = { publication_type: normPub, issue_number: Number(iss.issue_number) };
    setVal("iss-number", iss.issue_number);
    setVal("iss-pub-type", normPub);
    if (byId("iss-number")) byId("iss-number").disabled = true;
    if (byId("iss-pub-type")) byId("iss-pub-type").disabled = true;
    const lockHint = byId("iss-key-lock-hint");
    if (lockHint) {
      lockHint.textContent = t(
        "admin.issues.key_locked_hint",
        "Düzenleme modunda yayın türü ve sayı numarası kilitlidir (yeni sayı açmak için Temizle'yi kullanın).",
        "Publication type and issue number are locked while editing (click Clear to create a new issue).",
      );
      lockHint.classList.remove("hidden");
    }
    setVal("iss-year", iss.year || 2026);
    setVal("iss-month-tr", iss.publish_month_tr || "");
    setVal("iss-month-en", iss.publish_month_en || "");
    setVal("iss-title-tr", iss.title_tr || "");
    setVal("iss-title-en", iss.title_en || "");
    setVal("iss-slug", iss.slug || "");
    setVal("iss-cover-image", iss.cover_image || "");
    setVal("iss-pdf-file", iss.uploaded_pdf_file || "");
    setVal("iss-pages", iss.page_count || 112);
    setVal("iss-dossiers", iss.dossier_count || 6);
    setVal("iss-event", iss.event_name || "");
    if (byId("iss-special")) byId("iss-special").checked = !!iss.is_special_issue;
    setVal("iss-summary-tr", iss.summary_tr || "");
    setVal("iss-summary-en", iss.summary_en || "");
    updateDrawerConvertStatus(iss);
    if (byId("issue-form-heading")) byId("issue-form-heading").textContent = `${t("admin.form.edit_issue", "Dergi Sayısı Düzenle")} (${normPub === "msi_tdr" ? "MSI TDR" : "MSI"} #${iss.issue_number})`;
  }

  async function convertIssuePdf(pubType, issueNum, btnEl) {
    const pt = normPubType(pubType);
    const num = Number(issueNum);
    if (!num) return false;
    if (btnEl) btnEl.disabled = true;
    try {
      const resp = await apiRequest(`/api/v1/admin/issues/${encodeURIComponent(pt)}/${encodeURIComponent(num)}/convert-pdf`, {
        method: "POST",
      });
      if (resp.ok) {
        const data = resp.payload?.data || resp.payload || {};
        const spreadsCnt = Array.isArray(data.spreads) ? data.spreads.length : 0;
        const pageCnt = data.page_count || 0;
        setVal("iss-pages", pageCnt);
        if (spreadsCnt > 0) setVal("iss-dossiers", spreadsCnt);
        if (data.issue) updateDrawerConvertStatus(data.issue);
        showToast(
          t("admin.toast.issue_converted", "${pub} #${num} Web E-Dergi'ye dönüştürüldü (${pages} sayfa, ${spreads} bölüm).")
            .replace("${pub}", pt === "msi_tdr" ? "MSI TDR" : "MSI")
            .replace("${num}", num)
            .replace("${pages}", pageCnt)
            .replace("${spreads}", spreadsCnt),
        );
        await loadIssues();
        return true;
      }
      showToast(resp.errorMessage, true);
      return false;
    } finally {
      if (btnEl) btnEl.disabled = false;
    }
  }

  async function submitIssueForm(ev) {
    ev.preventDefault();
    const issueNum = parseInt(val("iss-number") || "0", 10);
    const pubType = normPubType(val("iss-pub-type") || "msi");
    const titleTr = val("iss-title-tr");
    if (issueNum <= 0 || !titleTr) {
      ctx.switchLangTab(byId("form-issue"), "tr");
      showToast(t("admin.error.issue_required_fields", "Sayı numarası ve Türkçe başlık zorunludur."), true);
      return;
    }
    const existing = findIssueByKey(issueNum, pubType);
    const rawPdf = val("iss-pdf-file");
    const pdfBase = rawPdf ? rawPdf.split("/").pop() : null;
    const payload = {
      issue_number: issueNum,
      publication_type: pubType,
      year: parseInt(val("iss-year") || "2026", 10),
      publish_month_tr: val("iss-month-tr") || "Kasım",
      publish_month_en: val("iss-month-en"),
      title_tr: titleTr,
      title_en: val("iss-title-en"),
      slug: val("iss-slug"),
      cover_image: val("iss-cover-image") || "images/msi/dergi-286.webp",
      uploaded_pdf_file: pdfBase,
      pdf_path: `/api/v1/edergi/${issueNum}/download-pdf?publication_type=${encodeURIComponent(pubType)}`,
      page_count: parseInt(val("iss-pages") || "112", 10),
      dossier_count: parseInt(val("iss-dossiers") || "6", 10),
      event_name: val("iss-event") || null,
      is_special_issue: !!byId("iss-special")?.checked,
      summary_tr: val("iss-summary-tr"),
      summary_en: val("iss-summary-en"),
      spreads: existing?.spreads || [],
    };
    const isKeyUpdate = editingTarget && editingTarget.publication_type === pubType && editingTarget.issue_number === issueNum;
    const url = isKeyUpdate
      ? `/api/v1/admin/issues/${encodeURIComponent(pubType)}/${encodeURIComponent(issueNum)}`
      : "/api/v1/admin/issues";
    const resp = await apiRequest(url, {
      method: isKeyUpdate ? "PUT" : "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      showToast(t("admin.toast.issue_saved", "Sayı #${num} kaydedildi.").replace("${num}", issueNum));
      const autoConvert = !!byId("iss-auto-convert")?.checked;
      if (autoConvert && pdfBase) {
        const convOk = await convertIssuePdf(pubType, issueNum);
        if (!convOk) {
          editingTarget = { publication_type: pubType, issue_number: issueNum };
          if (byId("iss-number")) byId("iss-number").disabled = true;
          if (byId("iss-pub-type")) byId("iss-pub-type").disabled = true;
          await loadIssues();
          return;
        }
      }
      resetIssueForm();
      ctx.closeDrawer("drawer-issue");
      await loadIssues();
    } else showToast(resp.errorMessage, true);
  }

  ctx.registerActions({
    "open-issue-drawer": () => { resetIssueForm(); ctx.openDrawer("drawer-issue"); },
    "reset-issue-form": resetIssueForm,
    "edit-issue": (el) => {
      loadIssueIntoForm(el.getAttribute("data-issue-number"), el.getAttribute("data-publication-type"));
      ctx.openDrawer("drawer-issue");
    },
    "convert-issue-pdf": async (el) => {
      await convertIssuePdf(el.getAttribute("data-publication-type"), el.getAttribute("data-issue-number"), el);
    },
    "convert-drawer-pdf": async (el) => {
      const issueNum = parseInt(val("iss-number") || "0", 10);
      const pubType = normPubType(val("iss-pub-type") || "msi");
      if (!issueNum || !val("iss-pdf-file")) {
        showToast(t("admin.error.convert_requires_pdf", "Önce sayı numarası ve PDF dosyası seçin."), true);
        return;
      }
      await convertIssuePdf(pubType, issueNum, el);
    },
    "delete-issue": async (el) => {
      const num = el.getAttribute("data-issue-number");
      const pubType = normPubType(el.getAttribute("data-publication-type"));
      if (!(await confirmDialog(t("admin.confirm.delete_issue", "${pub} #${num} numaralı dergi sayısını silmek istediğinize emin misiniz?", "Are you sure you want to delete ${pub} issue #${num}?").replace("${pub}", pubType === "msi_tdr" ? "MSI TDR" : "MSI").replace("${num}", num)))) return;
      const keyPath = `/api/v1/admin/issues/${encodeURIComponent(pubType)}/${encodeURIComponent(num)}`;
      let r = await apiRequest(keyPath, { method: "DELETE" });
      if (r.status === 409) {
        if (!(await confirmDialog(r.errorMessage || t("admin.confirm.force_delete_issue", "Bu sayıya bağlı içerikler var. Yine de kalıcı olarak silinsin mi?", "This issue has linked content. Force delete anyway?")))) return;
        r = await apiRequest(`${keyPath}?force=true`, { method: "DELETE" });
      }
      if (r.ok) { showToast(t("admin.toast.issue_deleted", "Sayı #${num} silindi.", "Issue #${num} deleted.").replace("${num}", num)); await loadIssues(); } else showToast(r.errorMessage, true);
    },
    "trigger-iss-cover": () => byId("iss-cover-file")?.click(),
    "trigger-iss-pdf": () => byId("iss-pdf-upload")?.click(),
  });

  document.addEventListener("submit", (ev) => {
    if (ev.target?.id === "form-issue") submitIssueForm(ev);
  });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "iss-search") renderIssuesTable(1);
  });

  document.addEventListener("change", async (ev) => {
    const el = ev.target;
    if (!el) return;
    if (el.id === "iss-pub-filter") { pubFilter = el.value || "all"; renderIssuesTable(1); }
    else if (el.id === "iss-cover-file") {
      const u = await uploadFile(el);
      if (u) { setVal("iss-cover-image", u); showToast(t("admin.toast.cover_uploaded", "Kapak görseli yüklendi.", "Cover image uploaded.")); }
    } else if (el.id === "iss-pdf-upload") {
      const u = await uploadFile(el);
      if (u) {
        setVal("iss-pdf-file", u.split("/").pop());
        ensureConverterControls();
        const st = byId("iss-convert-status");
        if (st) st.textContent = t("admin.issues.pdf_ready_convert", "PDF yüklendi — kaydedildiğinde Web E-Dergi'ye dönüştürülecek.", "PDF uploaded — will convert to Web E-Dergi when saved.");
        showToast(t("admin.toast.pdf_uploaded", "E-Dergi PDF dosyası yüklendi.", "E-Magazine PDF uploaded."));
      }
    }
  });

  ensureConverterControls();
  ctx.registerLoader(loadIssues);
})();
