(function () {
  "use strict";

  let plansCache = [];
  let productsCache = [];
  let invoicesPage = 1;

  function ctx() {
    return window.MSIAdmin && window.MSIAdmin._ctx;
  }

  async function loadPlans() {
    const c = ctx();
    const resp = await c.apiRequest("/api/v1/admin/subscription-plans");
    if (!resp.ok) return;
    plansCache = c.unwrapList(resp.payload, "plans");
    renderPlans();
  }

  function renderPlans() {
    const c = ctx();
    const tbody = document.getElementById("tbody-sub-plans");
    if (!tbody) return;
    if (!plansCache.length) {
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center;color:var(--text-muted);">Abonelik planı bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = plansCache
      .map(
        (p) => `<tr>
          <td><code>${c.esc(p.code)}</code></td>
          <td><strong>${c.esc(p.name_tr)}</strong><br><span style="font-size:11px;color:var(--text-muted);">${c.esc(p.name_en)}</span></td>
          <td style="font-family:var(--font-mono);">₺${Number(p.price_try || 0).toLocaleString("tr-TR")}</td>
          <td>${c.esc(p.period_label_tr)}</td>
          <td>
            <div style="display:flex;gap:4px;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-com-action="edit-plan" data-code="${c.esc(p.code)}"><i class="fa fa-pencil"></i></button>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-com-action="delete-plan" data-code="${c.esc(p.code)}"><i class="fa fa-trash"></i></button>
            </div>
          </td>
        </tr>`,
      )
      .join("");
  }

  function editPlan(code) {
    const p = plansCache.find((x) => x.code === code);
    if (!p) return;
    document.getElementById("sp-code").value = p.code || "";
    document.getElementById("sp-name-tr").value = p.name_tr || "";
    document.getElementById("sp-name-en").value = p.name_en || "";
    document.getElementById("sp-price").value = p.price_try ?? 0;
    document.getElementById("sp-period-tr").value = p.period_label_tr || "";
    document.getElementById("sp-period-en").value = p.period_label_en || "";
    document.getElementById("sp-features-tr").value = (p.features_tr || []).join("\n");
    document.getElementById("sp-features-en").value = (p.features_en || []).join("\n");
    document.getElementById("sp-featured").checked = !!p.is_featured;
    const sortEl = document.getElementById("sp-sort");
    if (sortEl) sortEl.value = p.sort_order ?? 10;
    ctx().showToast(`Plan "${p.code}" düzenleme formuna yüklendi.`);
  }

  async function submitPlan(e) {
    e.preventDefault();
    const c = ctx();
    const lines = (id) =>
      (document.getElementById(id)?.value || "")
        .split("\n")
        .map((s) => s.trim())
        .filter(Boolean);
    const payload = {
      code: document.getElementById("sp-code").value.trim(),
      name_tr: document.getElementById("sp-name-tr").value.trim(),
      name_en: document.getElementById("sp-name-en").value.trim() || document.getElementById("sp-name-tr").value.trim(),
      price_try: parseFloat(document.getElementById("sp-price").value || "0"),
      period_label_tr: document.getElementById("sp-period-tr").value.trim() || "Yıllık",
      period_label_en: document.getElementById("sp-period-en").value.trim() || "Annual",
      features_tr: lines("sp-features-tr"),
      features_en: lines("sp-features-en"),
      is_featured: document.getElementById("sp-featured").checked,
      sort_order: parseInt(document.getElementById("sp-sort").value || "10", 10),
    };
    const resp = await c.apiRequest("/api/v1/admin/subscription-plans", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      c.showToast(`Plan "${payload.code}" kaydedildi!`);
      document.getElementById("form-sub-plan").reset();
      await loadPlans();
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  async function deletePlan(code) {
    const c = ctx();
    if (!confirm(`"${code}" planını silmek istediğinize emin misiniz?`)) return;
    const resp = await c.apiRequest(`/api/v1/admin/subscription-plans/${encodeURIComponent(code)}`, {
      method: "DELETE",
    });
    if (resp.ok) {
      c.showToast(`Plan "${code}" silindi.`);
      await loadPlans();
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  async function loadProducts() {
    const c = ctx();
    const resp = await c.apiRequest("/api/v1/admin/commerce-products");
    if (!resp.ok) return;
    productsCache = c.unwrapList(resp.payload, "products");
    renderProducts();
  }

  function renderProducts() {
    const c = ctx();
    const tbody = document.getElementById("tbody-commerce-products");
    if (!tbody) return;
    if (!productsCache.length) {
      tbody.innerHTML = '<tr><td colspan="5" style="text-align:center;color:var(--text-muted);">Ürün kaydı bulunamadı.</td></tr>';
      return;
    }
    tbody.innerHTML = productsCache
      .map(
        (pr) => `<tr>
          <td><code>${c.esc(pr.sku)}</code></td>
          <td><span class="badge-tag">${c.esc(pr.product_type)}</span></td>
          <td><strong>${c.esc(pr.title_tr)}</strong><br><span style="font-size:11px;color:var(--text-muted);">${c.esc(pr.author_or_source)}</span></td>
          <td style="font-family:var(--font-mono);">₺${Number(pr.price_try || 0).toLocaleString("tr-TR")}</td>
          <td>
            <div style="display:flex;gap:4px;">
              <button type="button" class="btn-cmd btn-cmd-sm" data-com-action="edit-product" data-id="${c.esc(pr.id)}"><i class="fa fa-pencil"></i></button>
              <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-com-action="delete-product" data-id="${c.esc(pr.id)}"><i class="fa fa-trash"></i></button>
            </div>
          </td>
        </tr>`,
      )
      .join("");
  }

  function editProduct(id) {
    const pr = productsCache.find((x) => Number(x.id) === Number(id));
    if (!pr) return;
    document.getElementById("cp-id").value = pr.id || 0;
    document.getElementById("cp-sku").value = pr.sku || "";
    document.getElementById("cp-type").value = pr.product_type || "book";
    document.getElementById("cp-title-tr").value = pr.title_tr || "";
    document.getElementById("cp-title-en").value = pr.title_en || "";
    document.getElementById("cp-author").value = pr.author_or_source || "";
    document.getElementById("cp-price").value = pr.price_try ?? 0;
    document.getElementById("cp-res").value = pr.file_resolution_or_pages || "";
    document.getElementById("cp-cover").value = pr.cover_image || "";
    document.getElementById("cp-desc-tr").value = pr.description_tr || "";
    document.getElementById("cp-desc-en").value = pr.description_en || "";
    document.getElementById("cp-available").checked = !!pr.is_available;
    ctx().showToast(`Ürün #${pr.id} formda açıldı.`);
  }

  async function submitProduct(e) {
    e.preventDefault();
    const c = ctx();
    const payload = {
      id: parseInt(document.getElementById("cp-id").value || "0", 10),
      sku: document.getElementById("cp-sku").value.trim(),
      product_type: document.getElementById("cp-type").value || "book",
      title_tr: document.getElementById("cp-title-tr").value.trim(),
      title_en: document.getElementById("cp-title-en").value.trim() || document.getElementById("cp-title-tr").value.trim(),
      author_or_source: document.getElementById("cp-author").value.trim() || "UMSA Yayıncılık",
      price_try: parseFloat(document.getElementById("cp-price").value || "0"),
      file_resolution_or_pages: document.getElementById("cp-res").value.trim(),
      cover_image: document.getElementById("cp-cover").value.trim() || "images/msi/dergi-286.webp",
      description_tr: document.getElementById("cp-desc-tr").value.trim(),
      description_en: document.getElementById("cp-desc-en").value.trim(),
      is_available: document.getElementById("cp-available").checked,
    };
    const resp = await c.apiRequest("/api/v1/admin/commerce-products", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (resp.ok) {
      c.showToast(`Ürün "${payload.sku}" kaydedildi!`);
      document.getElementById("form-commerce-product").reset();
      document.getElementById("cp-id").value = "0";
      await loadProducts();
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  async function deleteProduct(id) {
    const c = ctx();
    if (!confirm(`#${id} ürününü silmek istediğinize emin misiniz?`)) return;
    const resp = await c.apiRequest(`/api/v1/admin/commerce-products/${encodeURIComponent(id)}`, {
      method: "DELETE",
    });
    if (resp.ok) {
      c.showToast("Ürün silindi.");
      await loadProducts();
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  async function loadInvoices(page) {
    const c = ctx();
    invoicesPage = typeof page === "number" ? page : invoicesPage;
    const resp = await c.apiRequest(`/api/v1/admin/invoices?page=${invoicesPage}&per_page=20`);
    if (!resp.ok) return;
    const list = c.unwrapList(resp.payload, "invoices");
    const tbody = document.getElementById("tbody-invoices");
    if (tbody) {
      tbody.innerHTML = !list.length
        ? '<tr><td colspan="6" style="text-align:center;color:var(--text-muted);">Fatura bulunamadı.</td></tr>'
        : list
            .map((inv) => {
              const isPaid = (inv.payment_status || "").toLowerCase() === "paid";
              const statusBadge = isPaid
                ? '<span class="badge-tag badge-paywall-0">Ödendi</span>'
                : `<span class="badge-tag badge-paywall-2">${c.esc(inv.payment_status)}</span>`;
              const actionBtn = isPaid
                ? '<span style="font-size:11px;color:var(--text-muted);">Tamam</span>'
                : `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-primary" data-com-action="mark-paid" data-id="${c.esc(inv.id)}"><i class="fa fa-check"></i> Ödendi İşaretle</button>`;
              return `<tr>
                <td style="font-family:var(--font-mono);font-size:11.5px;">${c.esc(inv.invoice_no)}<br><span style="color:var(--text-muted);">${c.esc(inv.created_at)}</span></td>
                <td>${c.esc(inv.full_name)}<br><span style="font-size:11px;color:var(--text-muted);">${c.esc(inv.user_email)}</span></td>
                <td>${c.esc(inv.item_title)}<br><code style="font-size:10.5px;">${c.esc(inv.item_code)}</code></td>
                <td style="font-family:var(--font-mono);">₺${Number(inv.amount_try || 0).toLocaleString("tr-TR")}</td>
                <td>${statusBadge}</td>
                <td>${actionBtn}</td>
              </tr>`;
            })
            .join("");
    }
    c.renderPaginationControls("pagination-invoices", resp.meta, loadInvoices);
  }

  async function markInvoicePaid(id) {
    const c = ctx();
    const resp = await c.apiRequest(`/api/v1/admin/invoices/${encodeURIComponent(id)}/mark-paid`, {
      method: "POST",
    });
    if (resp.ok) {
      c.showToast(`Fatura #${id} ödendi olarak işaretlendi ve haklar tanımlandı.`);
      await Promise.all([loadInvoices(), c.loadRevenueSummary()]);
    } else {
      c.showToast(resp.errorMessage, true);
    }
  }

  document.addEventListener("submit", (ev) => {
    if (ev.target?.id === "form-sub-plan") submitPlan(ev);
    if (ev.target?.id === "form-commerce-product") submitProduct(ev);
  });

  document.addEventListener("click", (ev) => {
    const btn = ev.target.closest("[data-com-action]");
    if (!btn) return;
    const act = btn.getAttribute("data-com-action");
    if (act === "edit-plan") editPlan(btn.getAttribute("data-code"));
    if (act === "delete-plan") deletePlan(btn.getAttribute("data-code"));
    if (act === "edit-product") editProduct(btn.getAttribute("data-id"));
    if (act === "delete-product") deleteProduct(btn.getAttribute("data-id"));
    if (act === "mark-paid") markInvoicePaid(btn.getAttribute("data-id"));
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail && ev.detail.tabId === "tab-14") {
      loadPlans();
      loadProducts();
      loadInvoices(1);
    }
  });
})();
