(function () {
  "use strict";

  let translationsPage = 1;
  let mediaPage = 1;
  let translationsCache = [];

  function ctx() {
    return window.MSIAdmin && window.MSIAdmin._ctx;
  }

  function enhanceTaxonomyTables() {
    const c = ctx();
    if (!c) return;
    document.querySelectorAll("#tbody-categories tr").forEach((tr) => {
      if (tr.querySelector("[data-ext-tax]")) return;
      const slugCell = tr.querySelector("td");
      const slug = slugCell ? slugCell.textContent.replace(/^\//, "").trim() : "";
      const item = (c.state.categories || []).find((x) => x.slug === slug);
      if (!item) return;
      const td = document.createElement("td");
      td.setAttribute("data-ext-tax", "1");
      td.innerHTML = `<div style="display:flex;gap:4px;">
        <button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-category" data-id="${c.esc(item.id)}" title="Düzenle"><i class="fa fa-pencil"></i></button>
        <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-category" data-id="${c.esc(item.id)}" title="Sil"><i class="fa fa-trash"></i></button>
      </div>`;
      tr.appendChild(td);
    });

    document.querySelectorAll("#tbody-ssb-tags tr").forEach((tr) => {
      if (tr.querySelector("[data-ext-ssb]")) return;
      const codeCell = tr.querySelector("td");
      const code = codeCell ? codeCell.textContent.trim() : "";
      const item = (c.state.ssbTags || []).find((x) => x.code === code);
      if (!item) return;
      const td = document.createElement("td");
      td.setAttribute("data-ext-ssb", "1");
      td.innerHTML = `<div style="display:flex;gap:4px;">
        <button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-ssb" data-code="${c.esc(item.code)}" title="Düzenle"><i class="fa fa-pencil"></i></button>
        <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-ssb" data-code="${c.esc(item.code)}" title="Sil"><i class="fa fa-trash"></i></button>
      </div>`;
      tr.appendChild(td);
    });

    document.querySelectorAll("#tbody-newsletters tr").forEach((tr) => {
      if (tr.querySelector("[data-ext-nl]") || tr.querySelector('td[colspan]')) return;
      const strongId = tr.querySelector("td strong");
      const id = strongId ? strongId.textContent.replace("#", "").trim() : "";
      if (!id) return;
      const td = document.createElement("td");
      td.setAttribute("data-ext-nl", "1");
      td.innerHTML = `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-newsletter" data-id="${c.esc(id)}" title="Kampanyayı Sil"><i class="fa fa-trash"></i></button>`;
      tr.appendChild(td);
    });

    document.querySelectorAll("#tbody-redirects tr").forEach((tr) => {
      if (tr.querySelector("[data-ext-rd]") || tr.querySelector('td[colspan]')) return;
      const cells = tr.querySelectorAll("td");
      const oldPath = cells[0] ? cells[0].textContent.trim() : "";
      const item = (c.state.redirects || []).find((x) => x.old_path === oldPath);
      if (!item) return;
      const td = document.createElement("td");
      td.setAttribute("data-ext-rd", "1");
      td.innerHTML = `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-redirect" data-id="${c.esc(item.id)}" title="Yönlendirmeyi Sil"><i class="fa fa-trash"></i></button>`;
      tr.appendChild(td);
    });
  }

  function enhanceUsersAndVersions() {
    const c = ctx();
    if (!c) return;
    document.querySelectorAll("#tbody-admin-users tr").forEach((tr) => {
      const btnAssign = tr.querySelector('[data-admin-action="assign-user-role"]');
      if (!btnAssign || tr.querySelector("[data-ext-user-op]")) return;
      const uid = btnAssign.getAttribute("data-user-id");
      const wrap = btnAssign.parentElement;
      if (!wrap) return;
      wrap.insertAdjacentHTML(
        "beforeend",
        `<button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-user" data-user-id="${c.esc(uid)}" data-ext-user-op="1" title="Kullanıcıyı Düzenle"><i class="fa fa-pencil"></i></button>
         <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-user" data-user-id="${c.esc(uid)}" title="Sil"><i class="fa fa-trash"></i></button>`,
      );
    });

    const contentId = document.getElementById("version-history-select")?.value;
    if (contentId) {
      document.querySelectorAll("#version-history-list > div").forEach((card) => {
        if (card.querySelector("[data-ext-restore]")) return;
        const vText = card.querySelector("strong")?.textContent || "";
        const version = vText.replace(/^v/i, "").trim();
        if (!version) return;
        card.insertAdjacentHTML(
          "beforeend",
          `<div style="margin-top:6px;"><button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="restore-version" data-ext-restore="1" data-content-id="${c.esc(contentId)}" data-version="${c.esc(version)}"><i class="fa fa-undo"></i> Bu Versiyona Geri Dön</button></div>`,
        );
      });
    }
  }

  function enhanceMediaCards() {
    const c = ctx();
    if (!c) return;
    document.querySelectorAll("#media-library-grid [data-admin-action='copy-media-url']").forEach((card) => {
      if (card.querySelector("[data-ext-media-ops]")) return;
      const url = card.getAttribute("data-url") || "";
      const m = (c.state.mediaLibrary || []).find((x) => (x.path || x.url || "") === url);
      if (!m || !m.id) return;
      card.insertAdjacentHTML(
        "beforeend",
        `<div data-ext-media-ops="1" style="display:flex;gap:4px;margin-top:6px;">
          <button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-media" data-id="${c.esc(m.id)}" title="Alt Text & Etiket"><i class="fa fa-pencil"></i></button>
          <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-media" data-id="${c.esc(m.id)}" title="Sil"><i class="fa fa-trash"></i></button>
        </div>`,
      );
    });
  }

  async function loadTranslations(page) {
    const c = ctx();
    if (!c) return;
    translationsPage = typeof page === "number" ? page : translationsPage;
    const resp = await c.apiRequest(`/api/v1/admin/translations?page=${translationsPage}&per_page=30`);
    if (!resp.ok) return;
    translationsCache = c.unwrapList(resp.payload, "items");
    renderTranslationsTable();
    c.renderPaginationControls("pagination-translations", resp.meta, loadTranslations);
  }

  function renderTranslationsTable() {
    const c = ctx();
    const tbody = document.getElementById("tbody-translations");
    if (!tbody) return;
    const q = (document.getElementById("tr-search")?.value || "").toLowerCase().trim();
    const filtered = translationsCache.filter((t) =>
      !q ||
      (t.key || "").toLowerCase().includes(q) ||
      (t.value_tr || "").toLowerCase().includes(q) ||
      (t.value_en || "").toLowerCase().includes(q),
    );
    tbody.innerHTML = !filtered.length
      ? '<tr><td colspan="4" style="text-align:center;color:var(--text-muted);">Çeviri bulunamadı.</td></tr>'
      : filtered
          .map(
            (t) => `<tr>
              <td><code>${c.esc(t.key)}</code><br><span style="font-size:10px;color:var(--text-muted);">${c.esc(t.group)}</span></td>
              <td style="font-size:11.5px;">${c.esc(t.value_tr)}</td>
              <td style="font-size:11.5px;">${c.esc(t.value_en)}</td>
              <td>
                <div style="display:flex;gap:4px;">
                  <button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-tr" data-key="${c.esc(t.key)}"><i class="fa fa-pencil"></i></button>
                  <button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-tr" data-key="${c.esc(t.key)}"><i class="fa fa-trash"></i></button>
                </div>
              </td>
            </tr>`,
          )
          .join("");
  }

  async function loadMediaSearchPage(page) {
    const c = ctx();
    if (!c) return;
    mediaPage = typeof page === "number" ? page : 1;
    const rawQ = (document.getElementById("media-search")?.value || "").trim();
    const q = encodeURIComponent(rawQ);
    const resp = await c.apiRequest(`/api/v1/admin/media-library?page=${mediaPage}&per_page=36&q=${q}`);
    if (!resp.ok) return;
    c.state.mediaLibrary = c.unwrapList(resp.payload, "items");
    const grid = document.getElementById("media-library-grid");
    if (grid) {
      if (!c.state.mediaLibrary.length) {
        grid.innerHTML = '<div style="grid-column:1/-1; text-align:center; padding:24px; color:var(--text-muted);">Aramanızla eşleşen görsel bulunamadı.</div>';
      } else {
        grid.innerHTML = c.state.mediaLibrary
          .map((m) => `<div class="media-card" data-media-id="${c.esc(m.id)}"><img src="${c.esc(m.path)}" alt="${c.esc(m.alt_text_tr || m.original_name || "")}" loading="lazy"><div class="media-meta"><strong>${c.esc(m.original_name || m.path)}</strong><span>${c.esc(m.mime_type || "")}</span></div></div>`)
          .join("");
      }
    }
    enhanceMediaCards();
    c.renderPaginationControls("pagination-media-library", resp.meta, loadMediaSearchPage);
  }

  function showUserForm(u) {
    const box = document.getElementById("user-crud-form-container");
    if (!box) return;
    const roleSel = document.getElementById("u-role");
    if (roleSel && ctx().state.userRoles.length) {
      roleSel.innerHTML = ctx().state.userRoles
        .map((r) => `<option value="${ctx().esc(r.role_code)}">${ctx().esc(r.role_code)}</option>`)
        .join("");
    }
    document.getElementById("u-edit-id").value = u ? String(u.id || 0) : "0";
    const em = document.getElementById("u-email");
    if (em) {
      em.value = u ? u.email || "" : "";
      em.readOnly = !!u;
    }
    document.getElementById("u-full-name").value = u ? u.full_name || "" : "";
    document.getElementById("u-password").value = "";
    document.getElementById("u-role").value = u ? u.role || "registered" : "registered";
    document.getElementById("u-org-cat").value = u ? u.organization_category || "Sektör Kuruluşları" : "Sektör Kuruluşları";
    document.getElementById("u-org-name").value = u ? u.organization_name || "" : "";
    document.getElementById("u-job-title").value = u ? u.job_title || "" : "";
    document.getElementById("u-sub-tier").value = u ? u.subscription_tier || "" : "";
    document.getElementById("u-gov-verified").checked = u ? !!u.is_gov_verified : false;
    box.style.display = "block";
  }

  async function saveUserForm() {
    const c = ctx();
    const uid = parseInt(document.getElementById("u-edit-id").value || "0", 10);
    const pw = document.getElementById("u-password").value.trim();
    const body = {
      email: document.getElementById("u-email").value.trim(),
      full_name: document.getElementById("u-full-name").value.trim(),
      role: document.getElementById("u-role").value,
      organization_category: document.getElementById("u-org-cat").value.trim(),
      organization_name: document.getElementById("u-org-name").value.trim(),
      job_title: document.getElementById("u-job-title").value.trim(),
      subscription_tier: document.getElementById("u-sub-tier").value.trim(),
      is_gov_verified: document.getElementById("u-gov-verified").checked,
    };
    if (uid > 0) {
      if (pw) body.new_password = pw;
      const resp = await c.apiRequest(`/api/v1/admin/users/${uid}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (resp.ok) {
        c.showToast(`Kullanıcı #${uid} güncellendi!`);
        document.getElementById("user-crud-form-container").style.display = "none";
        await c.loadAdminUsers();
        enhanceUsersAndVersions();
      } else {
        c.showToast(resp.errorMessage, true);
      }
    } else {
      body.password = pw;
      const resp = await c.apiRequest("/api/v1/admin/users", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (resp.ok) {
        c.showToast(`Yeni kullanıcı "${body.email}" oluşturuldu!`);
        document.getElementById("user-crud-form-container").style.display = "none";
        await c.loadAdminUsers();
        enhanceUsersAndVersions();
      } else {
        c.showToast(resp.errorMessage, true);
      }
    }
  }

  document.addEventListener("submit", async (ev) => {
    const c = ctx();
    if (!c) return;
    if (ev.target?.id === "form-new-category") {
      const editId = parseInt(document.getElementById("cat-edit-id")?.value || "0", 10);
      if (editId > 0) {
        ev.preventDefault();
        ev.stopPropagation();
        const payload = {
          id: editId,
          slug: document.getElementById("cat-slug").value.trim(),
          parent_slug: null,
          name_tr: document.getElementById("cat-name-tr").value.trim(),
          name_en: document.getElementById("cat-name-en").value.trim(),
          group_type: document.getElementById("cat-group-type").value,
          description_tr: document.getElementById("cat-desc-tr").value.trim(),
          description_en: document.getElementById("cat-name-en").value.trim(),
          icon_class: "fa fa-cubes",
          sort_order: parseInt(document.getElementById("cat-sort-order").value || "25", 10),
          is_dynamic: true,
        };
        const resp = await c.apiRequest(`/api/v1/admin/categories/${editId}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        if (resp.ok) {
          c.showToast("Kategori güncellendi!");
          document.getElementById("cat-edit-id").value = "0";
          document.getElementById("form-new-category").reset();
          await c.refreshAllData();
        } else {
          c.showToast(resp.errorMessage, true);
        }
      }
    } else if (ev.target?.id === "form-new-ssb") {
      const oldCode = (document.getElementById("ssb-edit-old-code")?.value || "").trim();
      if (oldCode) {
        ev.preventDefault();
        ev.stopPropagation();
        const payload = {
          id: 0,
          code: document.getElementById("ssb-code").value.trim(),
          name_tr: document.getElementById("ssb-name-tr").value.trim(),
          name_en: document.getElementById("ssb-name-en").value.trim(),
          category_group: document.getElementById("ssb-group").value.trim() || "Savunma Sanayii",
          sort_order: parseInt(document.getElementById("ssb-sort").value || "15", 10),
        };
        const resp = await c.apiRequest(`/api/v1/admin/ssb-tags/${encodeURIComponent(oldCode)}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });
        if (resp.ok) {
          c.showToast("SSB etiketi güncellendi!");
          document.getElementById("ssb-edit-old-code").value = "";
          document.getElementById("form-new-ssb").reset();
          await c.refreshAllData();
        } else {
          c.showToast(resp.errorMessage, true);
        }
      }
    } else if (ev.target?.id === "form-translation-upsert") {
      ev.preventDefault();
      const payload = {
        key: document.getElementById("tr-key").value.trim(),
        group: document.getElementById("tr-group").value.trim() || "general",
        value_tr: document.getElementById("tr-val-tr").value.trim(),
        value_en: document.getElementById("tr-val-en").value.trim(),
      };
      const resp = await c.apiRequest("/api/v1/admin/translations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload),
      });
      if (resp.ok) {
        c.showToast(`Çeviri "${payload.key}" kaydedildi!`);
        ev.target.reset();
        await loadTranslations();
      } else {
        c.showToast(resp.errorMessage, true);
      }
    }
  }, true);

  document.addEventListener("click", async (ev) => {
    const btn = ev.target.closest("[data-ext-op]");
    if (!btn) return;
    const c = ctx();
    const op = btn.getAttribute("data-ext-op");
    if (op === "edit-category") {
      const cat = (c.state.categories || []).find((x) => Number(x.id) === Number(btn.getAttribute("data-id")));
      if (!cat) return;
      document.getElementById("cat-edit-id").value = cat.id;
      document.getElementById("cat-name-tr").value = cat.name_tr || "";
      document.getElementById("cat-name-en").value = cat.name_en || "";
      document.getElementById("cat-slug").value = cat.slug || "";
      document.getElementById("cat-group-type").value = cat.group_type || "icerikler";
      document.getElementById("cat-sort-order").value = cat.sort_order || 25;
      document.getElementById("cat-desc-tr").value = cat.description_tr || "";
      c.showToast(`Kategori #${cat.id} düzenlemeye alındı.`);
    } else if (op === "delete-category") {
      const id = btn.getAttribute("data-id");
      if (!confirm(`#${id} kategorisini silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/categories/${id}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("Kategori silindi."); await c.refreshAllData(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "edit-ssb") {
      const tag = (c.state.ssbTags || []).find((x) => x.code === btn.getAttribute("data-code"));
      if (!tag) return;
      document.getElementById("ssb-edit-old-code").value = tag.code;
      document.getElementById("ssb-code").value = tag.code;
      document.getElementById("ssb-name-tr").value = tag.name_tr || "";
      document.getElementById("ssb-name-en").value = tag.name_en || "";
      document.getElementById("ssb-group").value = tag.category_group || "";
      document.getElementById("ssb-sort").value = tag.sort_order || 15;
      c.showToast(`SSB etiketi "${tag.code}" düzenlemeye alındı.`);
    } else if (op === "delete-ssb") {
      const code = btn.getAttribute("data-code");
      if (!confirm(`"${code}" SSB etiketini silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/ssb-tags/${encodeURIComponent(code)}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("SSB etiketi silindi."); await c.refreshAllData(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "delete-newsletter") {
      const id = btn.getAttribute("data-id");
      if (!confirm(`#${id} bültenini silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/newsletters/${id}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("Bülten silindi."); await c.refreshAllData(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "delete-redirect") {
      const id = btn.getAttribute("data-id");
      if (!confirm(`#${id} yönlendirme kuralını silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/redirects/${id}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("301 yönlendirmesi silindi."); await c.refreshAllData(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "edit-tr") {
      const t = translationsCache.find((x) => x.key === btn.getAttribute("data-key"));
      if (!t) return;
      document.getElementById("tr-key").value = t.key;
      document.getElementById("tr-group").value = t.group || "general";
      document.getElementById("tr-val-tr").value = t.value_tr || "";
      document.getElementById("tr-val-en").value = t.value_en || "";
    } else if (op === "delete-tr") {
      const k = btn.getAttribute("data-key");
      if (!confirm(`"${k}" çeviri anahtarını silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/translations/${encodeURIComponent(k)}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("Çeviri silindi."); await loadTranslations(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "edit-media") {
      ev.stopPropagation();
      const id = btn.getAttribute("data-id");
      const m = (c.state.mediaLibrary || []).find((x) => Number(x.id) === Number(id));
      if (!m) return;
      document.getElementById("media-edit-panel").style.display = "block";
      document.getElementById("media-edit-id").value = m.id;
      document.getElementById("media-edit-alt-tr").value = m.alt_text_tr || "";
      document.getElementById("media-edit-alt-en").value = m.alt_text_en || "";
      document.getElementById("media-edit-tags").value = m.tags || "";
    } else if (op === "save-media-meta") {
      const id = document.getElementById("media-edit-id").value;
      const resp = await c.apiRequest(`/api/v1/admin/media-library/${id}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          alt_text_tr: document.getElementById("media-edit-alt-tr").value.trim(),
          alt_text_en: document.getElementById("media-edit-alt-en").value.trim(),
          tags: document.getElementById("media-edit-tags").value.trim(),
        }),
      });
      if (resp.ok) {
        c.showToast("Medya metadata güncellendi.");
        document.getElementById("media-edit-panel").style.display = "none";
        await loadMediaSearchPage(mediaPage);
      } else c.showToast(resp.errorMessage, true);
    } else if (op === "cancel-media-meta") {
      document.getElementById("media-edit-panel").style.display = "none";
    } else if (op === "delete-media") {
      ev.stopPropagation();
      const id = btn.getAttribute("data-id");
      if (!confirm(`#${id} medya kaydını silmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/media-library/${id}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("Medya silindi."); await loadMediaSearchPage(mediaPage); } else c.showToast(resp.errorMessage, true);
    } else if (op === "show-create-user") {
      showUserForm(null);
    } else if (op === "hide-user-form") {
      document.getElementById("user-crud-form-container").style.display = "none";
    } else if (op === "save-user-form") {
      await saveUserForm();
    } else if (op === "edit-user") {
      const uid = Number(btn.getAttribute("data-user-id"));
      const u = (c.state.adminUsers || []).find((x) => Number(x.id ?? x.user_id) === uid);
      showUserForm(u || null);
    } else if (op === "delete-user") {
      const uid = btn.getAttribute("data-user-id");
      if (!confirm(`#${uid} kullanıcısını silmek / anonimleştirmek istediğinize emin misiniz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/users/${uid}`, { method: "DELETE" });
      if (resp.ok) { c.showToast("Kullanıcı silindi."); await c.loadAdminUsers(); enhanceUsersAndVersions(); } else c.showToast(resp.errorMessage, true);
    } else if (op === "restore-version") {
      const cid = btn.getAttribute("data-content-id");
      const ver = btn.getAttribute("data-version");
      if (!confirm(`İçerik #${cid}, v${ver} sürümüne geri döndürülecek. Onaylıyor musunuz?`)) return;
      const resp = await c.apiRequest(`/api/v1/admin/contents/${cid}/versions/${ver}/restore`, { method: "POST" });
      if (resp.ok) {
        c.showToast(`İçerik v${ver} sürümüne geri yüklendi!`);
        await c.refreshAllData();
      } else c.showToast(resp.errorMessage, true);
    }
  });

  document.addEventListener("input", (ev) => {
    if (ev.target?.id === "tr-search") renderTranslationsTable();
    if (ev.target?.id === "media-search") loadMediaSearchPage(1);
  });

  window.addEventListener("msi:admin:refreshed", () => {
    setTimeout(() => {
      enhanceTaxonomyTables();
      enhanceUsersAndVersions();
      enhanceMediaCards();
    }, 30);
  });

  window.addEventListener("msi:admin:tab", (ev) => {
    const id = ev.detail && ev.detail.tabId;
    if (id === "tab-8") loadTranslations(1);
    if (id === "tab-10") loadMediaSearchPage(1);
    if (id === "tab-11" || id === "tab-12") setTimeout(enhanceUsersAndVersions, 60);
    if (id === "tab-5" || id === "tab-6" || id === "tab-7") setTimeout(enhanceTaxonomyTables, 30);
  });

  document.addEventListener("change", (ev) => {
    if (ev.target?.id === "version-history-select") {
      setTimeout(enhanceUsersAndVersions, 120);
    }
  });

  document.addEventListener("DOMContentLoaded", () => {
    let scheduled = false;
    const obs = new MutationObserver(() => {
      if (scheduled) return;
      scheduled = true;
      requestAnimationFrame(() => {
        scheduled = false;
        obs.disconnect();
        try {
          enhanceTaxonomyTables();
          enhanceUsersAndVersions();
          enhanceMediaCards();
        } finally {
          attachObservers();
        }
      });
    });
    function attachObservers() {
      [
        "tbody-categories",
        "tbody-ssb-tags",
        "tbody-newsletters",
        "tbody-redirects",
        "tbody-admin-users",
        "version-history-list",
        "media-library-grid",
      ].forEach((id) => {
        const el = document.getElementById(id);
        if (el) obs.observe(el, { childList: true });
      });
    }
    attachObservers();
  });
})();
