(function () {
  "use strict";

  let issuesCache = [];
  let pubFilter = "all";

  function ctx() {
    return window.MSIAdmin && window.MSIAdmin._ctx;
  }

  function populateIssueDatalist() {
    const dl = document.getElementById("c-issue-datalist");
    if (!dl) return;
    dl.innerHTML = issuesCache
      .map(
        (iss) =>
          `<option value="${iss.issue_number}">${iss.publication_type === "msi_tdr" ? "MSI TDR" : "MSI"} #${iss.issue_number} — ${ctx().esc(iss.title_tr)}</option>`,
      )
      .join("");
  }

  function renderIssuesTable() {
    const tbody = document.getElementById("tbody-admin-issues");
    if (!tbody) return;
    const c = ctx();
    const list = issuesCache.filter((iss) =>
      pubFilter === "all" ? true : (iss.publication_type || "msi") === pubFilter,
    );
    if (!list.length) {
      tbody.innerHTML =
        '<tr><td colspan="6" style="text-align:center; padding:20px; color:var(--text-muted);">Sayı kaydı bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = list
      .map((iss) => {
        const pubBadge =
          iss.publication_type === "msi_tdr"
            ? '<span class="badge-tag" style="background:rgba(14,165,233,0.18);color:#38bdf8;">MSI TDR</span>'
            : '<span class="badge-tag badge-paywall-0">MSI Dergisi</span>';
        const pdfLink = iss.uploaded_pdf_file
          ? `<a href="/api/v1/admin/edergi-files/${encodeURIComponent(iss.uploaded_pdf_file)}" target="_blank" class="badge-tag badge-paywall-0" title="Yüklü PDF Önizle"><i class="fa fa-file-pdf-o"></i> ${c.esc(iss.uploaded_pdf_file)}</a>`
          : `<span style="color:var(--text-muted); font-size:11px;">Dinamik DRM</span>`;
        return `<tr>
          <td style="font-family:var(--font-mono); font-weight:700;">#${c.esc(iss.issue_number)}</td>
          <td>${pubBadge}<br><span style="font-size:11px; color:var(--text-muted);">${c.esc(iss.publish_month_tr)} ${c.esc(iss.year)}</span></td>
          <td>
            <div style="font-weight:700;">${c.esc(iss.title_tr)}</div>
            <div style="font-size:11px; color:var(--text-muted);">${c.esc(iss.title_en)}</div>
          </td>
          <td style="font-size:11.5px;">${c.esc(iss.page_count)} sayfa · ${c.esc(iss.dossier_count)} dosya</td>
          <td>${pdfLink}</td>
          <td>
            <div style="display:flex; gap:4px; flex-wrap:wrap;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-ext-action="edit-issue" data-issue-number="${c.esc(iss.issue_number)}" title="Düzenle"><i class="fa fa-pencil"></i></button>
              <a href="/e-dergi?issue=${c.esc(iss.issue_number)}" target="_blank" class="btn-cmd btn-cmd-sm" title="Kiosk"><i class="fa fa-external-link"></i></a>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-action="delete-issue" data-issue-number="${c.esc(iss.issue_number)}" data-required-perm="issues:write" title="Sil"><i class="fa fa-trash"></i></button>
            </div>
          </td>
        </tr>`;
      })
      .join("");
  }

  async function loadIssues() {
    const c = ctx();
    if (!c) return;
    const resp = await c.apiRequest("/api/v1/admin/issues");
    if (!resp.ok) return;
    issuesCache = c.unwrapList(resp.payload, "issues");
    if (c.state) c.state.issues = issuesCache;
    populateIssueDatalist();
    renderIssuesTable();
  }

  function loadIssueIntoForm(num) {
    const iss = issuesCache.find((i) => Number(i.issue_number) === Number(num));
    if (!iss) return;
    document.getElementById("iss-number").value = iss.issue_number;
    document.getElementById("iss-pub-type").value = iss.publication_type || "msi";
    document.getElementById("iss-year").value = iss.year || 2026;
    document.getElementById("iss-month-tr").value = iss.publish_month_tr || "";
    document.getElementById("iss-month-en").value = iss.publish_month_en || "";
    document.getElementById("iss-title-tr").value = iss.title_tr || "";
    document.getElementById("iss-title-en").value = iss.title_en || "";
    document.getElementById("iss-slug").value = iss.slug || "";
    document.getElementById("iss-cover-image").value = iss.cover_image || "";
    document.getElementById("iss-pdf-file").value = iss.uploaded_pdf_file || "";
    document.getElementById("iss-pages").value = iss.page_count || 100;
    document.getElementById("iss-dossiers").value = iss.dossier_count || 6;
    document.getElementById("iss-event").value = iss.event_name || "";
    document.getElementById("iss-special").checked = !!iss.is_special_issue;
    document.getElementById("iss-summary-tr").value = iss.summary_tr || "";
    document.getElementById("iss-summary-en").value = iss.summary_en || "";
    ctx().showToast(`Sayı #${iss.issue_number} düzenleme formuna yüklendi.`);
  }

  function resetIssueForm() {
    const form = document.getElementById("form-issue");
    if (form) form.reset();
    document.getElementById("iss-pdf-file").value = "";
  }

  async function submitIssueForm(e) {
    if (e) e.preventDefault();
    const c = ctx();
    const issueNum = parseInt(document.getElementById("iss-number").value || "0", 10);
    const titleTr = document.getElementById("iss-title-tr").value.trim();
    if (issueNum <= 0 || !titleTr) {
      c.showToast("Sayı No ve Türkçe başlık zorunludur.", true);
      return false;
    }
    const existing = issuesCache.find((i) => Number(i.issue_number) === issueNum);
    const rawPdf = document.getElementById("iss-pdf-file").value.trim();
    const pdfBase = rawPdf ? rawPdf.split("/").pop() : null;

    const payload = {
      issue_number: issueNum,
      publication_type: document.getElementById("iss-pub-type").value || "msi",
      year: parseInt(document.getElementById("iss-year").value || "2026", 10),
      publish_month_tr: document.getElementById("iss-month-tr").value.trim() || "Kasım",
      publish_month_en: document.getElementById("iss-month-en").value.trim() || "November",
      title_tr: titleTr,
      title_en: document.getElementById("iss-title-en").value.trim() || titleTr,
      slug: document.getElementById("iss-slug").value.trim(),
      cover_image: document.getElementById("iss-cover-image").value.trim() || "images/msi/dergi-286.webp",
      uploaded_pdf_file: pdfBase || null,
      pdf_path: `/api/v1/edergi/${issueNum}/download-pdf`,
      page_count: parseInt(document.getElementById("iss-pages").value || "100", 10),
      dossier_count: parseInt(document.getElementById("iss-dossiers").value || "6", 10),
      event_name: document.getElementById("iss-event").value.trim() || null,
      is_special_issue: document.getElementById("iss-special").checked,
      summary_tr: document.getElementById("iss-summary-tr").value.trim(),
      summary_en: document.getElementById("iss-summary-en").value.trim(),
      spreads: existing ? existing.spreads || [] : [],
    };

    const resp = await c.apiRequest("/api/v1/admin/issues", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      c.showToast(`Sayı #${issueNum} kaydedildi!`);
      await loadIssues();
    } else {
      c.showToast(resp.errorMessage, true);
    }
    return false;
  }

  async function deleteIssue(num) {
    const c = ctx();
    if (!confirm(`#${num} numaralı dergi sayısını silmek istediğinize emin misiniz?`)) return;
    const resp = await c.apiRequest(`/api/v1/admin/issues/${encodeURIComponent(num)}`, {
      method: "DELETE",
    });
    if (resp.ok) {
      c.showToast(`Sayı #${num} silindi.`);
      await loadIssues();
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  async function uploadIssueAsset(inputEl, targetId, isPdf) {
    const c = ctx();
    const file = inputEl?.files?.[0];
    if (!file) return;
    const fd = new FormData();
    fd.append("file", file);
    const resp = await c.apiRequest("/api/v1/admin/upload", { method: "POST", body: fd });
    inputEl.value = "";
    if (!resp.ok) {
      c.showToast(resp.errorMessage || "Yükleme hatası.", true);
      return;
    }
    const url = (resp.data && (resp.data.url || resp.data.path)) || "";
    const target = document.getElementById(targetId);
    if (target && url) {
      target.value = isPdf ? url.split("/").pop() : url;
    }
    c.showToast(isPdf ? "E-Dergi PDF dosyası yüklendi!" : "Kapak görseli yüklendi!");
  }

  document.addEventListener("submit", (ev) => {
    if (ev.target && ev.target.id === "form-issue") {
      ev.preventDefault();
      submitIssueForm(ev);
    }
  });

  document.addEventListener("change", (ev) => {
    const t = ev.target;
    if (!t) return;
    if (t.id === "iss-cover-file") uploadIssueAsset(t, "iss-cover-image", false);
    if (t.id === "iss-pdf-upload") uploadIssueAsset(t, "iss-pdf-file", true);
    if (t.id === "iss-pub-filter") {
      pubFilter = t.value || "all";
      renderIssuesTable();
    }
  });

  document.addEventListener("click", (ev) => {
    const btn = ev.target.closest("[data-ext-action]");
    if (!btn) return;
    const act = btn.getAttribute("data-ext-action");
    if (act === "edit-issue") loadIssueIntoForm(btn.getAttribute("data-issue-number"));
    if (act === "delete-issue") deleteIssue(btn.getAttribute("data-issue-number"));
    if (act === "reset-issue-form") resetIssueForm();
    if (act === "trigger-iss-cover") document.getElementById("iss-cover-file")?.click();
    if (act === "trigger-iss-pdf") document.getElementById("iss-pdf-upload")?.click();
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail && ev.detail.tabId === "tab-18") loadIssues();
  });

  document.addEventListener("DOMContentLoaded", () => {
    setTimeout(loadIssues, 50);
  });
})();
