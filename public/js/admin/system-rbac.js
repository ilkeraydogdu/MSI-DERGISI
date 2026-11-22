(function () {
  "use strict";

  const C = () => window.MSIAdmin && window.MSIAdmin._ctx;
  const WAF_PER_PAGE = 15;
  const toggleHidden = (id, show) => {
    const el = C().byId(id);
    if (el) { el.classList.toggle("hidden", !show); el.style.removeProperty("display"); }
  };

  async function fetchPaginated(perm, url, listKey, stateKey, pagId, renderFn, selfFn, page) {
    const c = C();
    if (!c.hasPerm(perm)) return;
    c.paginationState[stateKey] = typeof page === "number" ? page : c.paginationState[stateKey] || 1;
    const resp = await c.apiRequest(`${url}${url.includes("?") ? "&" : "?"}page=${c.paginationState[stateKey]}&per_page=20`);
    if (!resp.ok || !resp.payload) return;
    c.state[stateKey] = c.unwrapList(resp.payload, listKey);
    renderFn(c.state[stateKey]);
    c.renderPaginationControls(pagId, resp.meta, selfFn);
  }

  async function delItem(url, msg, okMsg, reloadFn) {
    const c = C();
    if (!(await c.confirmDialog(msg))) return;
    const r = await c.apiRequest(url, { method: "DELETE" });
    if (r.ok) { c.showToast(okMsg); await reloadFn(); } else c.showToast(r.errorMessage, true);
  }

  async function submitJson(url, body, okMsg, afterFn, method = "POST") {
    const c = C();
    const r = await c.apiRequest(url, { method, headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
    if (r.ok) { if (okMsg) c.showToast(okMsg); if (afterFn) await afterFn(r); }
    else c.showToast(r.errorMessage, true);
    return r;
  }

  function renderUserRoles() {
    const c = C(), tbody = c.byId("tbody-user-roles");
    if (!tbody) return;
    if (!c.state.userRoles.length) { c.renderTableState(tbody, "empty", { colCount: 6 }); return; }
    const canManage = c.hasPerm("roles:manage");
    tbody.innerHTML = c.state.userRoles.map((r) => {
      let count = 0; try { count = JSON.parse(r.permissions_json || "[]").length; } catch (_) {}
      const isSys = r.is_system == 1 || r.is_system === true;
      const acts = isSys ? `<span class="badge-tag">${c.esc(c.t("admin.rbac.system_role", "Sistem Rolü", "System Role"))}</span>`
        : canManage ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-role" data-role="${c.esc(r.role_code)}">${c.svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-role" data-role="${c.esc(r.role_code)}">${c.svgIcon("trash")}</button></div>` : "—";
      return `<tr><td class="text-mono text-xs text-info">${c.esc(r.role_code)}</td><td class="font-semibold">${c.esc(r.name_tr)}</td><td class="text-muted">${c.esc(r.name_en)}</td><td><span class="badge-tag">${count} ${c.esc(c.t("admin.rbac.permission_unit", "yetki", "perms"))}</span></td><td>${c.svgIcon(isSys ? "lock" : "unlock", isSys ? "text-warning" : "text-muted")}</td><td>${acts}</td></tr>`;
    }).join("");
    const sel = c.byId("manual-assign-role-code");
    if (sel && c.state.userRoles.length) {
      const prev = sel.value;
      sel.innerHTML = c.state.userRoles.map((r) => `<option value="${c.esc(r.role_code)}">${c.esc(r.role_code)} — ${c.esc(r.name_tr)}</option>`).join("");
      if ([...sel.options].some((o) => o.value === prev)) sel.value = prev;
    }
  }

  const loadUserRoles = (p) => fetchPaginated("users:read|users:manage|roles:manage", "/api/v1/admin/user-roles", "roles", "userRoles", "pagination-user-roles", () => { renderUserRoles(); renderAdminUsers(); }, loadUserRoles, p);

  function openRoleForm(role) {
    const c = C();
    toggleHidden("role-form-container", true);
    c.setVal("role-edit-mode", role ? "edit" : "create");
    const titleEl = c.byId("role-form-title");
    if (titleEl) {
      titleEl.textContent = role
        ? `${c.t("admin.system.edit_role", "Rolü Düzenle", "Edit Role")} (${role.role_code})`
        : c.t("admin.system.new_role_btn", "Yeni Rol Tanımla", "Define New Role");
    }
    const codeEl = c.byId("role-code");
    if (codeEl) { codeEl.value = role ? role.role_code : ""; codeEl.readOnly = !!role; }
    c.setVal("role-name-tr", role ? role.name_tr : "");
    c.setVal("role-name-en", role ? role.name_en : "");
    let pSet = new Set(); try { if (role) pSet = new Set(JSON.parse(role.permissions_json || "[]")); } catch (_) {}
    document.querySelectorAll("#role-permissions-grid input[type='checkbox']").forEach((cb) => { cb.checked = pSet.has(cb.value); });
    c.byId("role-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
    (role ? c.byId("role-name-tr") : codeEl)?.focus();
    if (role) c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", role.name_tr || role.role_code));
  }

  async function saveRole() {
    const c = C(), mode = c.val("role-edit-mode") || "create", code = c.val("role-code"), nameTr = c.val("role-name-tr");
    if (!code || !nameTr) { c.showToast(c.t("admin.error.role_required_fields", "Rol kodu ve Türkçe rol adı zorunludur.", "Role code and Turkish role name are required."), true); return; }
    const perms = [...document.querySelectorAll("#role-permissions-grid input:checked")].map((cb) => cb.value);
    const resp = await c.apiRequest(mode === "edit" ? `/api/v1/admin/user-roles/${encodeURIComponent(code)}` : "/api/v1/admin/user-roles", {
      method: mode === "edit" ? "PUT" : "POST", headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ role_code: code, name_tr: nameTr, name_en: c.val("role-name-en") || nameTr, permissions_json: JSON.stringify(perms) }),
    });
    if (resp.ok) { c.showToast(mode === "edit" ? c.t("admin.toast.role_updated", "Rol \"${code}\" güncellendi.", "Role \"${code}\" updated.").replace("${code}", code) : c.t("admin.toast.role_created", "Yeni rol \"${code}\" oluşturuldu.", "New role \"${code}\" created.").replace("${code}", code)); toggleHidden("role-form-container", false); await loadUserRoles(); }
    else c.showToast(resp.errorMessage, true);
  }

  async function loadAdminUsers(page) {
    const c = C();
    if (!c.hasPerm("users:read|users:manage")) return;
    const p = typeof page === "number" ? page : c.paginationState.adminUsers || 1;
    c.paginationState.adminUsers = p;
    const resp = await c.apiRequest(`/api/v1/admin/users?page=${p}&per_page=20`);
    if (!resp.ok || !resp.payload) return;
    c.state.adminUsers = c.unwrapList(resp.payload, "users");
    renderAdminUsers();
    c.renderPaginationControls("pagination-admin-users", resp.meta, loadAdminUsers);
  }

  function renderAdminUsers() {
    const c = C(), tbody = c.byId("tbody-admin-users");
    if (!tbody) return;
    const q = c.val("user-role-search").toLowerCase();
    const list = c.state.adminUsers.filter((u) => !q || [u.full_name, u.email, u.role, u.id].some((x) => String(x || "").toLowerCase().includes(q)));
    if (!list.length) { c.renderTableState(tbody, "empty", { colCount: 3, emptyMsg: c.t("admin.state.no_match_user", "Eşleşen kullanıcı bulunamadı.", "No matching users found.") }); return; }
    const choices = c.state.userRoles.length ? c.state.userRoles.map((r) => r.role_code) : ["site_operator", "super_admin", "content_editor", "content_publisher", "ads_manager", "finance_viewer", "support_agent", "registered"];
    const canR = c.hasPerm("roles:manage"), canU = c.hasPerm("users:manage");
    const roleMap = c.getLang() === "en" ? c.ROLE_LABELS_EN : c.ROLE_LABELS_TR;
    tbody.innerHTML = list.map((u) => {
      const uid = u.id ?? u.user_id;
      const rCtrl = canR ? `<select class="form-select select-flex" id="assign-role-sel-${c.esc(uid)}">${choices.map((rc) => `<option value="${c.esc(rc)}" ${rc === (u.role || "") ? "selected" : ""}>${c.esc(rc)}</option>`).join("")}</select><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-primary" data-admin-action="assign-user-role" data-user-id="${c.esc(uid)}">${c.esc(c.t("admin.system.assign_role_btn", "Rol Ata", "Assign Role"))}</button>` : "";
      const uBtns = canU ? `<button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-user" data-user-id="${c.esc(uid)}" title="${c.esc(c.t("admin.action.edit", "Düzenle", "Edit"))}">${c.svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-user" data-user-id="${c.esc(uid)}" title="${c.esc(c.t("admin.action.delete", "Sil", "Delete"))}">${c.svgIcon("trash")}</button>` : "";
      return `<tr>
        <td><div class="font-semibold">#${c.esc(uid)} · ${c.esc(u.full_name)}</div><div class="text-xs text-muted">${c.esc(u.email)}</div></td>
        <td>${u.role ? `<span class="badge-tag badge-placement">${c.esc(roleMap[u.role] || u.role)}</span> ` : ""}${u.subscription_tier ? `<span class="badge-tag">${c.esc(u.subscription_tier)}</span>` : ""}</td>
        <td><div class="flex-wrap-gap">${rCtrl}${uBtns || (!rCtrl ? `<span class="text-xs text-muted">${c.esc(c.t("admin.label.read_only", "Salt Okunur", "Read Only"))}</span>` : "")}</div></td>
      </tr>`;
    }).join("");
  }

  function showUserForm(u) {
    const c = C(), sel = c.byId("u-role"), em = c.byId("u-email");
    toggleHidden("user-crud-form-container", true);
    const titleEl = c.byId("user-form-title");
    if (titleEl) {
      titleEl.textContent = u
        ? `${c.t("admin.system.edit_user", "Kullanıcıyı Düzenle", "Edit User")} (#${u.id ?? u.user_id})`
        : c.t("admin.system.new_user", "Yeni Kullanıcı", "New User");
    }
    if (sel && c.state.userRoles.length) sel.innerHTML = c.state.userRoles.map((r) => `<option value="${c.esc(r.role_code)}">${c.esc(r.role_code)} — ${c.esc(r.name_tr)}</option>`).join("");
    if (em) { em.value = u?.email || ""; em.readOnly = !!u; }
    [["u-edit-id", u ? String(u.id ?? u.user_id ?? 0) : "0"], ["u-full-name", u?.full_name || ""], ["u-password", ""],
      ["u-role", u?.role || "registered"], ["u-org-cat", u?.organization_category || "Sektör Kuruluşları"],
      ["u-org-name", u?.organization_name || ""], ["u-job-title", u?.job_title || ""], ["u-sub-tier", u?.subscription_tier || ""]]
      .forEach(([id, v]) => c.setVal(id, v));
    if (c.byId("u-gov-verified")) c.byId("u-gov-verified").checked = !!u?.is_gov_verified;
    c.byId("user-crud-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
    (u ? c.byId("u-full-name") : em)?.focus();
    if (u) c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", u.full_name || u.email));
    return c.loadCustomFieldValues("user-custom-fields", "user", String(u?.id ?? u?.user_id ?? 0));
  }

  async function saveUserForm() {
    const c = C(), uid = parseInt(c.val("u-edit-id") || "0", 10), pw = c.val("u-password");
    if (c.validateCustomFields && !c.validateCustomFields("user-custom-fields")) return;
    const body = { email: c.val("u-email"), full_name: c.val("u-full-name"), role: c.val("u-role"), organization_category: c.val("u-org-cat"), organization_name: c.val("u-org-name"), job_title: c.val("u-job-title"), subscription_tier: c.val("u-sub-tier"), is_gov_verified: !!c.byId("u-gov-verified")?.checked };
    if (uid > 0 && pw) body.new_password = pw;
    if (!uid) body.password = pw;
    const r = await c.apiRequest(uid > 0 ? `/api/v1/admin/users/${uid}` : "/api/v1/admin/users", { method: uid > 0 ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
    if (r.ok) {
      const savedUid = uid > 0 ? uid : Number(r.data?.id || r.data?.user_id || r.payload?.data?.id || 0);
      if (savedUid > 0) {
        const cfRes = await c.saveCustomFieldValues("user-custom-fields", "user", String(savedUid));
        if (cfRes && !cfRes.ok) {
          c.setVal("u-edit-id", String(savedUid));
          const em = c.byId("u-email");
          if (em) em.readOnly = true;
          c.showToast(cfRes.errorMessage || c.t("admin.toast.custom_fields_save_failed", "Özel alan değerleri kaydedilemedi.", "Failed to save custom field values."), true);
          await loadAdminUsers();
          return;
        }
      }
      c.showToast(uid > 0 ? c.t("admin.toast.user_updated", "Kullanıcı #${id} güncellendi.", "User #${id} updated.").replace("${id}", uid) : c.t("admin.toast.user_created", "Kullanıcı \"${email}\" oluşturuldu.", "User \"${email}\" created.").replace("${email}", body.email));
      toggleHidden("user-crud-form-container", false);
      await loadAdminUsers();
    } else c.showToast(r.errorMessage, true);
  }

  function renderAuditLogs(logs) {
    const c = C(), tbody = c.byId("tbody-audit-logs");
    if (!tbody) return;
    if (!logs.length) { c.renderTableState(tbody, "empty", { colCount: 6, emptyMsg: c.t("admin.state.no_audit_logs", "Denetim kaydı bulunamadı.", "No audit logs found.") }); return; }
    const cls = { create: "text-success", update: "text-info", delete: "text-danger", login: "text-warning", flush: "text-accent" };
    tbody.innerHTML = logs.map((l) => {
      let d = l.details_json || ""; try { d = Object.entries(JSON.parse(d || "{}")).map(([k, v]) => `${k}: ${v}`).join(", "); } catch (_) {}
      return `<tr><td class="text-xs text-mono">${c.esc(l.created_at ? new Date(l.created_at).toLocaleString(c.locale()) : "—")}</td><td>${c.esc(l.actor_email)}</td><td><strong class="${cls[l.action] || "text-secondary"}">${c.esc(l.action)}</strong></td><td>${c.esc(l.entity_type)}${l.entity_id ? " #" + c.esc(l.entity_id) : ""}</td><td class="text-xs truncate" title="${c.esc(d)}">${c.esc(d.substring(0, 72) || "—")}</td><td class="text-xs text-mono text-muted">${c.esc(l.ip_address || "-")}</td></tr>`;
    }).join("");
  }

  const loadAuditLogs = (p) => fetchPaginated("audit:read|users:manage|settings:read", "/api/v1/admin/audit-logs", "logs", "auditLogs", "pagination-audit-logs", renderAuditLogs, loadAuditLogs, p);

  function renderSecurityLogsPage(page) {
    const c = C(), tb = c.byId("tbody-security-logs");
    if (!tb) return;
    if (typeof page === "number") c.paginationState.securityLogs = page;
    const cur = c.paginationState.securityLogs || 1;
    const q = c.val("waf-search").toLowerCase();
    const list = (c.state.securityLogs || []).filter((l) => !q || [l.client_ip, l.event_type, l.threat_level, l.request_path, l.details].some((f) => String(f || "").toLowerCase().includes(q)));
    if (!list.length) {
      c.renderTableState(tb, "empty", { colCount: 5, emptyMsg: c.t("admin.state.no_waf_logs", "Eşleşen WAF güvenlik kaydı bulunmuyor.", "No matching WAF security logs.") });
      c.renderPaginationControls("pagination-security-logs", null, renderSecurityLogsPage);
      return;
    }
    const tp = Math.max(1, Math.ceil(list.length / WAF_PER_PAGE));
    const sp = Math.min(cur, tp);
    const slice = list.slice((sp - 1) * WAF_PER_PAGE, sp * WAF_PER_PAGE);
    tb.innerHTML = slice.map((l) => `<tr>
      <td class="text-mono text-xs">${c.esc(l.created_at)}</td>
      <td class="text-mono text-danger font-semibold">${c.esc(l.client_ip)}</td>
      <td><span class="badge-tag badge-sponsored">${c.esc(l.event_type)}</span></td>
      <td><span class="badge-tag badge-paywall-2">${c.esc(l.threat_level)}</span></td>
      <td class="text-xs"><code class="text-mono">${c.esc(l.request_path)}</code><div class="text-muted">${c.esc(l.details)}</div></td>
    </tr>`).join("");
    c.renderPaginationControls("pagination-security-logs", { page: sp, total_pages: tp, total_count: list.length }, renderSecurityLogsPage);
  }

  async function refreshCentralEnginesTab() {
    const c = C();
    const [setRes, secRes, notifRes] = await Promise.all([
      c.hasPerm("settings:read") ? c.safeFetchJson("/api/v1/admin/settings") : Promise.resolve(null),
      c.hasPerm("audit:read|settings:read") ? c.safeFetchJson("/api/v1/admin/security-logs") : Promise.resolve(null),
      c.hasPerm("notifications:read|settings:read") ? c.safeFetchJson("/api/v1/admin/notifications") : Promise.resolve(null),
    ]);
    if (secRes) {
      c.state.securityLogs = c.unwrapList(secRes, "logs");
      renderSecurityLogsPage(1);
    }
    const render = (id, res, key, cols, fn) => {
      const tb = c.byId(id);
      if (!tb || !res) return;
      const list = c.unwrapList(res, key);
      if (!list.length) c.renderTableState(tb, "empty", { colCount: cols });
      else tb.innerHTML = list.map(fn).join("");
    };
    render("tbody-site-settings", setRes, "settings", 4, (s) => {
      const trVal = String(s.value_tr || "");
      const enVal = String(s.value_en || "");
      return `<tr><td class="text-mono text-xs text-info">${c.esc(s.key)}</td><td class="text-xs">${c.esc(trVal.length > 80 ? trVal.substring(0, 80) + "…" : trVal)}</td><td class="text-xs text-muted">${c.esc(enVal.length > 80 ? enVal.substring(0, 80) + "…" : enVal)}</td><td><span class="badge-tag">${c.esc(s.setting_group)}</span></td></tr>`;
    });
    render("tbody-sys-notifications", notifRes, "notifications", 4, (n) => {
      const audMapTr = { all: "Tüm Ziyaretçiler", MSB: "MSB Personeli", SSB: "SSB Personeli" };
      const audMapEn = { all: "All Visitors", MSB: "MoD (MSB) Personnel", SSB: "SSB Personnel" };
      const sevMapTr = { info: "Bilgi", success: "Başarı", warning: "Uyarı", critical: "Kritik" };
      const sevMapEn = { info: "Info", success: "Success", warning: "Warning", critical: "Critical" };
      const audRaw = String(n.target_audience || "all");
      const sevRaw = String(n.severity || "info").toLowerCase();
      const audLbl = c.t("admin.system.aud_" + audRaw.toLowerCase(), audMapTr[audRaw] || audRaw, audMapEn[audRaw] || audRaw);
      const sevLbl = c.t("admin.system.sev_" + sevRaw, sevMapTr[sevRaw] || sevRaw, sevMapEn[sevRaw] || sevRaw);
      return `<tr><td><span class="badge-tag">${c.esc(audLbl)}</span></td><td><span class="badge-tag badge-paywall-1">${c.esc(sevLbl)}</span></td><td><strong>${c.esc(n.title_tr)}</strong><br><span class="text-muted text-xs">${c.esc(n.message_tr)}</span></td><td>${n.is_banner || n.is_active_banner ? `<span class="badge-tag badge-paywall-0">${c.esc(c.t("admin.system.banner_badge", "Üst Bant", "Top Banner"))}</span>` : `<span class="badge-tag">${c.esc(c.t("admin.system.notif_badge", "Bildirim", "Notification"))}</span>`}</td></tr>`;
    });
  }

  function renderTranslationsTable() {
    const c = C(), tbody = c.byId("tbody-translations"), q = c.val("tr-search").toLowerCase();
    if (!tbody) return;
    const list = c.state.translations.filter((item) => !q || [item.key, item.value_tr, item.value_en].some((v) => String(v || "").toLowerCase().includes(q)));
    if (!list.length) { c.renderTableState(tbody, "empty", { colCount: 4 }); return; }
    const canW = c.hasPerm("translations:write|settings:write");
    tbody.innerHTML = list.map((item) => `<tr><td><code class="text-mono text-xs text-info">${c.esc(item.key)}</code><br><span class="text-xs text-muted">${c.esc(item.group)}</span></td><td class="text-xs">${c.esc(item.value_tr)}</td><td class="text-xs text-secondary">${c.esc(item.value_en)}</td><td>${canW ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-ext-op="edit-tr" data-key="${c.esc(item.key)}">${c.svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-tr" data-key="${c.esc(item.key)}">${c.svgIcon("trash")}</button></div>` : "—"}</td></tr>`).join("");
  }

  const loadTranslations = (p) => fetchPaginated("translations:read|settings:read", "/api/v1/admin/translations?per_page=25", "items", "translations", "pagination-translations", renderTranslationsTable, loadTranslations, p);

  // ── Pending Translations & Auto-Translate (#tab8-i18n) ───────────────────
  async function loadPendingTranslations(page) {
    const c = C();
    if (!c.hasPerm("translations:read")) return;
    const tb = c.byId("tbody-pending-translations");
    const p = typeof page === "number" ? page : c.paginationState.pendingTranslations || 1;
    c.paginationState.pendingTranslations = p;
    const st = c.val("tr-pending-status-filter");
    const resp = await c.apiRequest(`/api/v1/admin/translations/pending?page=${p}&per_page=20&status=${encodeURIComponent(st)}`);
    if (!resp.ok || !resp.payload) {
      if (tb) c.renderTableState(tb, "error", { colCount: 5, errorMsg: resp.errorMessage, retryAction: "refresh-pending-translations" });
      return;
    }
    c.state.pendingTranslations = c.unwrapList(resp.payload, "items");
    if (tb) {
      if (!c.state.pendingTranslations.length) {
        c.renderTableState(tb, "empty", { colCount: 5, emptyMsg: c.t("admin.i18n.no_pending", "Bu filtrede çeviri kaydı bulunmuyor.", "No translation entries in this filter.") });
      } else {
        const canWrite = c.hasPerm("translations:write");
        tb.innerHTML = c.state.pendingTranslations.map((item) => {
          const itemSt = String(item.status || "draft");
          const stBadge = itemSt === "approved"
            ? `<span class="badge-tag badge-status-published">${c.esc(c.t("admin.i18n.status_approved", "Onaylı", "Approved"))}</span>`
            : itemSt === "auto_translated"
              ? `<span class="badge-tag badge-status-pending_review">${c.esc(c.t("admin.i18n.status_auto", "Otomatik Çeviri", "Auto-Translated"))}</span>`
              : `<span class="badge-tag badge-status-draft">${c.esc(c.t("admin.i18n.status_draft", "Taslak", "Draft"))}</span>`;
          const btns = canWrite ? `<div class="flex gap-2">
            ${itemSt !== "approved" ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-emerald" data-admin-action="approve-translation" data-key="${c.esc(item.key)}" title="${c.esc(c.t("admin.action.approve", "Onayla", "Approve"))}">${c.svgIcon("check")}</button>` : ""}
            ${itemSt !== "draft" ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-warning" data-admin-action="reject-translation" data-key="${c.esc(item.key)}" title="${c.esc(c.t("admin.action.reject", "Reddet", "Reject"))}">${c.svgIcon("undo")}</button>` : ""}
          </div>` : "—";
          return `<tr>
            <td><code class="text-mono text-xs text-info">${c.esc(item.key)}</code><br><span class="text-xs text-muted">${c.esc(item.group || "general")}</span></td>
            <td class="text-xs">${c.esc(item.value_tr || "")}</td>
            <td class="text-xs text-secondary">${c.esc(item.value_en || "")}</td>
            <td>${stBadge}</td>
            <td>${btns}</td>
          </tr>`;
        }).join("");
      }
    }
    c.renderPaginationControls("pagination-pending-translations", resp.meta, loadPendingTranslations);
  }

  async function runAutoTranslate() {
    const c = C();
    if (!c.hasPerm("translations:write|translations:read")) return;
    const text = c.val("tr-val-tr");
    if (!text) {
      c.byId("tr-val-tr")?.focus();
      c.showToast(c.t("admin.i18n.tr_text_required", "Otomatik çeviri için Türkçe metin girin.", "Enter Turkish text for auto-translation."), true);
      return;
    }
    const btn = c.byId("btn-tr-auto-translate");
    if (btn) btn.disabled = true;
    try {
      const resp = await c.apiRequest("/api/v1/admin/translations/auto-translate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: c.val("tr-key") || null, text, source_lang: "tr", target_lang: "en" }),
      });
      if (!resp.ok) {
        c.showToast(resp.errorMessage, true);
        return;
      }
      const d = resp.data || {};
      c.setVal("tr-val-en", d.translated_text || "");
      const warnBox = c.byId("tr-glossary-warnings");
      const violations = Array.isArray(d.glossary_violations) ? d.glossary_violations : [];
      if (warnBox) {
        if (violations.length) {
          warnBox.classList.remove("hidden");
          warnBox.innerHTML = `<div class="text-xs font-bold text-warning mb-1">${c.esc(c.t("admin.i18n.glossary_warnings_title", "Terim Sözlüğü Uyarıları", "Glossary Warnings"))} (${violations.length})</div>` +
            violations.map((v) => `<div class="text-xs mb-1"><strong>[${c.esc(v.domain)}] ${c.esc(v.term_tr)}:</strong> <span class="text-danger">${c.esc(v.forbidden)}</span> → <span class="text-success">${c.esc(v.preferred_en)}</span>${v.notes ? ` <span class="text-muted">(${c.esc(v.notes)})</span>` : ""}</div>`).join("");
        } else {
          warnBox.classList.add("hidden");
          warnBox.innerHTML = "";
        }
      }
      c.showToast(c.t("admin.toast.auto_translated", "Otomatik çeviri tamamlandı.", "Auto-translation completed."));
    } finally {
      if (btn) btn.disabled = false;
    }
  }

  // ── Glossary CRUD (#tab8-glossary) ──────────────────────────────────────
  function renderGlossaryTable() {
    const c = C(), tb = c.byId("tbody-glossary");
    if (!tb) return;
    const q = c.val("glossary-search").toLowerCase();
    const list = (c.state.glossary || []).filter((g) => !q || [
      g.term_tr, g.term_en, g.preferred_en, g.abbreviation_tr, g.abbreviation_en, g.notes,
      ...(Array.isArray(g.forbidden_variants) ? g.forbidden_variants : []),
    ].some((v) => String(v || "").toLowerCase().includes(q)));
    if (!list.length) {
      c.renderTableState(tb, "empty", { colCount: 6, emptyMsg: c.t("admin.glossary.empty", "Eşleşen terim sözlüğü kaydı bulunamadı.", "No matching glossary terms found.") });
      return;
    }
    const canWrite = c.hasPerm("glossary:write|glossary:manage");
    const canDelete = c.hasPerm("glossary:manage");
    tb.innerHTML = list.map((g) => {
      const forbid = Array.isArray(g.forbidden_variants) && g.forbidden_variants.length ? g.forbidden_variants.join(", ") : "—";
      const st = g.is_active !== false
        ? `<span class="badge-tag badge-status-published">${c.esc(c.t("admin.status.active", "Aktif", "Active"))}</span>`
        : `<span class="badge-tag badge-status-draft">${c.esc(c.t("admin.status.inactive", "Pasif", "Inactive"))}</span>`;
      const acts = (canWrite || canDelete) ? `<div class="flex gap-2">
        ${canWrite ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-glossary" data-id="${c.esc(g.id)}" title="${c.esc(c.t("admin.action.edit", "Düzenle", "Edit"))}">${c.svgIcon("pencil")}</button>` : ""}
        ${canDelete ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-glossary" data-id="${c.esc(g.id)}" title="${c.esc(c.t("admin.action.delete", "Sil", "Delete"))}">${c.svgIcon("trash")}</button>` : ""}
      </div>` : "—";
      return `<tr>
        <td><div class="font-semibold">${c.esc(g.term_tr)}${g.abbreviation_tr ? ` <span class="text-xs text-muted">(${c.esc(g.abbreviation_tr)})</span>` : ""}</div><div class="text-xs text-muted">${c.esc(g.term_en || "")}</div></td>
        <td><div class="font-semibold text-success">${c.esc(g.preferred_en)}</div>${g.abbreviation_en ? `<div class="text-xs text-muted">(${c.esc(g.abbreviation_en)})</div>` : ""}</td>
        <td><span class="badge-tag">${c.esc(g.domain)}</span> <span class="badge-tag badge-ssb">${c.esc(g.rule_type)}</span><div class="text-xs text-mono text-muted mt-1">P${c.esc(g.priority ?? 50)}</div></td>
        <td class="text-xs text-danger">${c.esc(forbid)}</td>
        <td>${st}</td>
        <td>${acts}</td>
      </tr>`;
    }).join("");
  }

  async function loadGlossary() {
    const c = C();
    if (!c.hasPerm("glossary:write|glossary:manage")) return;
    const tb = c.byId("tbody-glossary");
    const dom = c.val("glossary-domain-filter");
    const activeOnly = c.byId("glossary-active-only")?.checked ? "true" : "false";
    const resp = await c.apiRequest(`/api/v1/admin/glossary?domain=${encodeURIComponent(dom)}&active_only=${activeOnly}`);
    if (!resp.ok) {
      if (tb) c.renderTableState(tb, "error", { colCount: 6, errorMsg: resp.errorMessage, retryAction: "refresh-glossary" });
      return;
    }
    c.state.glossary = c.unwrapList(resp.payload);
    renderGlossaryTable();
  }

  // ── Locales CRUD (#tab8-locales) ────────────────────────────────────────
  function renderLocalesTable() {
    const c = C(), tb = c.byId("tbody-locales");
    if (!tb) return;
    const list = c.state.locales || [];
    if (!list.length) {
      c.renderTableState(tb, "empty", { colCount: 6, emptyMsg: c.t("admin.locales.empty", "Kayıtlı yerel ayar bulunamadı.", "No locales configured.") });
      return;
    }
    const canManage = c.hasPerm("locales:manage");
    tb.innerHTML = list.map((loc) => {
      const defBadge = loc.is_default ? ` <span class="badge-tag badge-placement">${c.esc(c.t("admin.locales.default_badge", "Varsayılan", "Default"))}</span>` : "";
      const st = (loc.is_enabled !== false
        ? `<span class="badge-tag badge-status-published">${c.esc(c.t("admin.status.active", "Aktif", "Active"))}</span>`
        : `<span class="badge-tag badge-status-draft">${c.esc(c.t("admin.status.inactive", "Pasif", "Inactive"))}</span>`) + defBadge;
      const acts = canManage ? `<div class="flex gap-2">
        <button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-locale" data-code="${c.esc(loc.code)}" title="${c.esc(c.t("admin.action.edit", "Düzenle", "Edit"))}">${c.svgIcon("pencil")}</button>
        ${!loc.is_default ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-locale" data-code="${c.esc(loc.code)}" title="${c.esc(c.t("admin.action.delete", "Sil", "Delete"))}">${c.svgIcon("trash")}</button>` : ""}
      </div>` : "—";
      return `<tr>
        <td class="text-mono font-bold text-info">${c.esc(loc.code)}</td>
        <td><div class="font-semibold">${c.esc(loc.name_native)}</div><div class="text-xs text-muted">${c.esc(loc.name_en)}</div></td>
        <td class="text-mono text-xs">${c.esc(loc.hreflang)}<div class="text-muted">${c.esc(loc.og_locale)}</div></td>
        <td class="text-mono text-xs">${c.esc(String(loc.dir || "ltr").toUpperCase())} / ${c.esc(loc.fallback_code || "tr")}</td>
        <td>${st}</td>
        <td>${acts}</td>
      </tr>`;
    }).join("");
  }

  async function loadLocales() {
    const c = C();
    if (!c.hasPerm("locales:manage")) return;
    const tb = c.byId("tbody-locales");
    const resp = await c.apiRequest("/api/v1/admin/locales");
    if (!resp.ok) {
      if (tb) c.renderTableState(tb, "error", { colCount: 6, errorMsg: resp.errorMessage });
      return;
    }
    c.state.locales = c.unwrapList(resp.payload);
    renderLocalesTable();
  }

  // ── Email Groups CRUD (#tab6-email-groups) ──────────────────────────────
  function renderEmailGroupsTable() {
    const c = C(), tb = c.byId("tbody-email-groups");
    if (!tb) return;
    const list = c.state.emailGroups || [];
    if (!list.length) {
      c.renderTableState(tb, "empty", { colCount: 5, emptyMsg: c.t("admin.email_groups.empty", "Kayıtlı e-posta grubu bulunamadı.", "No email groups found.") });
      return;
    }
    const canWrite = c.hasPerm("email_groups:write|email_groups:manage");
    const canDelete = c.hasPerm("email_groups:manage");
    tb.innerHTML = list.map((eg) => {
      let staticCount = 0;
      try { const arr = JSON.parse(eg.static_emails_json || "[]"); if (Array.isArray(arr)) staticCount = arr.length; } catch (_) {}
      const sysBadge = eg.is_system ? ` <span class="badge-tag">${c.esc(c.t("admin.rbac.system_role", "Sistem", "System"))}</span>` : "";
      const consentPill = eg.requires_marketing_consent !== false
        ? `<span class="badge-tag badge-placement">KVKK</span>`
        : `<span class="badge-tag">${c.esc(c.t("admin.email_groups.no_kvkk", "KVKK İzinsiz", "No KVKK"))}</span>`;
      const activePill = eg.is_active !== false
        ? `<span class="badge-tag badge-status-published">${c.esc(c.t("admin.status.active", "Aktif", "Active"))}</span>`
        : `<span class="badge-tag badge-status-draft">${c.esc(c.t("admin.status.inactive", "Pasif", "Inactive"))}</span>`;
      const acts = (canWrite || canDelete) ? `<div class="flex gap-2">
        ${canWrite ? `<button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-email-group" data-code="${c.esc(eg.code)}" title="${c.esc(c.t("admin.action.edit", "Düzenle", "Edit"))}">${c.svgIcon("pencil")}</button>` : ""}
        ${canDelete && !eg.is_system ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-email-group" data-code="${c.esc(eg.code)}" title="${c.esc(c.t("admin.action.delete", "Sil", "Delete"))}">${c.svgIcon("trash")}</button>` : ""}
      </div>` : "—";
      return `<tr>
        <td class="text-mono text-xs text-info">${c.esc(eg.code)}${sysBadge}</td>
        <td><div class="font-semibold">${c.esc(eg.name_tr)} / ${c.esc(eg.name_en)}</div><div class="text-xs text-muted">${c.esc(eg.description_tr || "")}</div></td>
        <td class="text-xs text-mono"><div>${c.esc(String(eg.filter_json || "{}").substring(0, 60))}</div><div class="text-muted">${staticCount} ${c.esc(c.t("admin.email_groups.static_count", "statik alıcı", "static recipients"))}</div></td>
        <td>${consentPill} ${activePill}</td>
        <td>${acts}</td>
      </tr>`;
    }).join("");
  }

  function populateNewsletterSelects() {
    const c = C();
    const egSel = c.byId("nl-email-group");
    if (egSel) {
      const cur = egSel.value || "";
      const groups = (c.state.emailGroups || []).filter((g) => g && g.is_active !== false);
      egSel.innerHTML = `<option value="">${c.esc(c.t("admin.workflow.nl_email_group_none", "-- Grup seçin (segment filtresi kullan) --", "-- Select group (use segment filter) --"))}</option>` +
        groups.map((g) => `<option value="${c.esc(g.code)}">${c.esc(c.getLang() === "en" ? (g.name_en || g.name_tr) : (g.name_tr || g.code))} (${c.esc(g.code)})</option>`).join("");
      if ([...egSel.options].some((o) => o.value === cur)) egSel.value = cur;
    }
    const adSel = c.byId("nl-sponsor-ad-id");
    if (adSel) {
      const cur = adSel.value || "";
      const sortedAds = [...(c.state.ads || [])]
        .filter((a) => a && a.is_active !== false)
        .sort((a, b) => Number(!!b.is_newsletter_ad) - Number(!!a.is_newsletter_ad) || Number(b.budget_score || 0) - Number(a.budget_score || 0));
      adSel.innerHTML = `<option value="">${c.esc(c.t("admin.workflow.nl_auto_sponsor", "Otomatik (Aktif bülten reklamı)", "Auto (Active newsletter ad)"))}</option>` +
        sortedAds.map((a) => `<option value="${c.esc(a.id)}">${a.is_newsletter_ad ? "★ " : ""}${c.esc(a.sponsor_name)} — ${c.esc(a.title_tr || a.slot_code)} (#${c.esc(a.id)})</option>`).join("");
      if ([...adSel.options].some((o) => o.value === cur)) adSel.value = cur;
    }
  }

  async function loadEmailGroups() {
    const c = C();
    if (!c.hasPerm("email_groups:write|email_groups:manage")) return;
    const tb = c.byId("tbody-email-groups");
    const resp = await c.apiRequest("/api/v1/admin/email-groups");
    if (!resp.ok) {
      if (tb) c.renderTableState(tb, "error", { colCount: 5, errorMsg: resp.errorMessage });
      return;
    }
    c.state.emailGroups = c.unwrapList(resp.payload);
    renderEmailGroupsTable();
    populateNewsletterSelects();
  }

  const loadNewsletters = (p) => fetchPaginated("newsletters:read|analytics:read", "/api/v1/admin/newsletters", "campaigns", "newsletters", "pagination-newsletters", (list) => {
    const c = C(), tb = c.byId("tbody-newsletters"); if (!tb) return;
    if (!list.length) { c.renderTableState(tb, "empty", { colCount: 6, emptyMsg: c.t("admin.state.no_newsletters", "Henüz gönderilmiş bülten kampanyası bulunmuyor.", "No newsletter campaigns sent yet.") }); return; }
    const canW = c.hasPerm("newsletters:write");
    const nlStatusLabel = (st) => {
      const k = String(st || "sent").toLowerCase();
      const trMap = { sent: "Gönderildi", scheduled: "Zamanlandı", draft: "Taslak", queued: "Kuyrukta", failed: "Başarısız" };
      const enMap = { sent: "Sent", scheduled: "Scheduled", draft: "Draft", queued: "Queued", failed: "Failed" };
      return c.t("admin.status." + k, trMap[k] || st, enMap[k] || st);
    };
    tb.innerHTML = list.map((n) => `<tr><td class="text-mono text-xs"><strong>#${c.esc(n.id)}</strong><br><span class="text-muted">${c.esc(n.scheduled_at || n.sent_at || "")}</span></td><td><div class="font-semibold">${c.esc(n.subject_tr)}</div><div class="text-xs text-muted">${c.esc(n.subject_en || "")}</div></td><td><span class="badge-tag">${c.esc(n.email_group_code || n.target_institution)}</span> <span class="badge-tag badge-ssb">${c.esc(n.target_ssb_tag)}</span></td><td><span class="badge-tag badge-sponsored">${c.esc(n.sponsor_name || c.t("admin.label.unsponsored", "Sponsorsuz", "Unsponsored"))}</span></td><td class="text-mono text-xs"><strong class="text-success">${Number(n.recipient_count || 0).toLocaleString(c.locale())} ${c.esc(c.t("admin.label.recipients", "alıcı", "recipients"))}</strong><div class="text-muted">${c.esc(nlStatusLabel(n.status))}</div></td><td>${canW ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-newsletter" data-id="${c.esc(n.id)}">${c.svgIcon("trash")}</button>` : "—"}</td></tr>`).join("");
  }, loadNewsletters, p);

  const loadReminders = (p) => fetchPaginated("subscriptions:read|analytics:read", "/api/v1/admin/reminders", "reminders", "reminders", "pagination-reminders", (list) => {
    const c = C(), tb = c.byId("tbody-reminders"); if (!tb) return;
    if (!list.length) { c.renderTableState(tb, "empty", { colCount: 5, emptyMsg: c.t("admin.state.no_reminders", "Şu anda bekleyen yenileme hatırlatması bulunmuyor.", "No pending renewal reminders.") }); return; }
    const stg = {
      "2_months": c.t("admin.system.reminder_2_months", "2 Ay Kala (60 Gün)", "2 Months Left (60 Days)"),
      "1_month": c.t("admin.system.reminder_1_month", "1 Ay Kala (30 Gün)", "1 Month Left (30 Days)"),
      "2_weeks": c.t("admin.system.reminder_2_weeks", "2 Hafta Kala (14 Gün)", "2 Weeks Left (14 Days)"),
      "1_week": c.t("admin.system.reminder_1_week", "1 Hafta Kala (7 Gün)", "1 Week Left (7 Days)"),
      "1_day": c.t("admin.system.reminder_last_day", "Son Gün Uyarısı", "Final Day Notice"),
    };
    tb.innerHTML = list.map((r) => `<tr><td><div class="font-semibold">${c.esc(r.full_name)}</div><div class="text-xs text-muted">${c.esc(r.user_email)} · ${c.esc(r.organization_name)}</div></td><td><span class="badge-tag badge-placement">${c.esc(stg[r.stage_label] || r.stage_label)}</span></td><td class="text-mono font-bold">${c.esc(r.days_remaining)} ${c.esc(c.t("admin.analytics.days_unit", "gün", "days"))}</td><td>${r.status === "halted_by_renewal" ? `<span class="badge-tag badge-paywall-0">${c.esc(c.t("admin.status.renewed_halted", "Yeniledi — Durduruldu", "Renewed — Halted"))}</span>` : r.status === "sent" ? `<span class="badge-tag">${c.esc(c.t("admin.status.sent", "Gönderildi", "Sent"))}</span>` : `<span class="badge-tag badge-paywall-2">${c.esc(c.t("admin.status.scheduled", "Planlandı", "Scheduled"))}</span>`}</td><td class="text-mono text-xs text-muted">${c.esc(r.sent_at)}</td></tr>`).join("");
  }, loadReminders, p);

  function renderRedirectsTable(list) {
    const c = C(), tb = c.byId("tbody-redirects"); if (!tb) return;
    const q = c.val("redirect-search").toLowerCase();
    const filtered = (list || c.state.redirects || []).filter((s) => !q || [s.old_path, s.new_path].some((f) => String(f || "").toLowerCase().includes(q)));
    if (!filtered.length) { c.renderTableState(tb, "empty", { colCount: 5, emptyMsg: c.t("admin.state.no_redirects", "Eşleşen 301 yönlendirme kuralı bulunamadı.", "No matching 301 redirect rules found.") }); return; }
    const canD = c.hasPerm("settings:write|contents:publish|redirects:write|redirects:manage");
    tb.innerHTML = filtered.map((s) => `<tr><td class="text-mono text-xs text-danger">${c.esc(s.old_path)}</td><td class="text-mono text-xs text-success">${c.esc(s.new_path)}</td><td><span class="badge-tag badge-paywall-0">${c.esc(s.status_code || 301)}</span></td><td class="text-mono font-bold">${Number(s.hit_count || 0).toLocaleString(c.locale())}</td><td>${canD ? `<button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-ext-op="delete-redirect" data-id="${c.esc(s.id)}">${c.svgIcon("trash")}</button>` : "—"}</td></tr>`).join("");
  }

  const loadRedirects = (p) => fetchPaginated("settings:read|contents:read|redirects:write|redirects:manage", "/api/v1/admin/redirects", "redirects", "redirects", "pagination-redirects", renderRedirectsTable, loadRedirects, p);

  const loadCustomFields = (p) => fetchPaginated("settings:read|contents:read", "/api/v1/admin/custom-fields?applies_to=all", "fields", "customFields", "pagination-custom-fields", (list) => {
    const c = C(), tb = c.byId("tbody-custom-fields"); if (!tb) return;
    if (!list.length) { c.renderTableState(tb, "empty", { colCount: 8, emptyMsg: c.t("admin.state.no_custom_fields", "Tanımlı dinamik özel alan bulunmuyor.", "No custom fields defined.") }); return; }
    const canW = c.hasPerm("settings:write");
    const cfTypeLabel = (ft) => {
      const k = String(ft || "text").toLowerCase();
      const trMap = { text: "Kısa Metin", textarea: "Uzun Metin", select: "Açılır Seçim Listesi", checkbox: "Onay Kutusu", number: "Sayısal Değer", date: "Tarih", url: "Bağlantı (URL)" };
      const enMap = { text: "Short Text", textarea: "Long Text", select: "Dropdown List", checkbox: "Checkbox", number: "Numeric Value", date: "Date", url: "URL" };
      return c.t("admin.system.dt_" + k, trMap[k] || ft, enMap[k] || ft);
    };
    const cfEntityLabel = (ent) => {
      const k = String(ent || "content").toLowerCase();
      const trMap = { content: "İçerik", ad: "Reklam Kampanyası", user: "Kullanıcı" };
      const enMap = { content: "Content", ad: "Ad Campaign", user: "User" };
      return c.t("admin.system.ent_" + k, trMap[k] || ent, enMap[k] || ent);
    };
    tb.innerHTML = list.map((f) => {
      const ops = canW
        ? `<div class="flex gap-2"><button type="button" class="btn-cmd btn-cmd-sm" data-admin-action="edit-custom-field" data-key="${c.esc(f.field_key)}" title="${c.esc(c.t("admin.action.edit", "Düzenle", "Edit"))}">${c.svgIcon("pencil")}</button><button type="button" class="btn-cmd btn-cmd-sm btn-cmd-danger" data-admin-action="delete-custom-field" data-key="${c.esc(f.field_key)}" title="${c.esc(c.t("admin.action.delete", "Sil", "Delete"))}">${c.svgIcon("trash")}</button></div>`
        : "—";
      return `<tr><td class="text-mono text-xs text-info">${c.esc(f.field_key)}</td><td class="font-semibold">${c.esc(f.label_tr)}</td><td class="text-muted">${c.esc(f.label_en)}</td><td><span class="badge-tag badge-placement">${c.esc(cfTypeLabel(f.field_type))}</span></td><td class="text-xs">${c.esc(cfEntityLabel(f.applies_to))}</td><td>${f.is_required ? c.svgIcon("check", "text-success") : c.svgIcon("minus", "text-muted")}</td><td class="text-mono text-xs">${c.esc(f.sort_order)}</td><td>${ops}</td></tr>`;
    }).join("");
  }, loadCustomFields, p);

  const loadCacheOperations = (p) => fetchPaginated("settings:read", "/api/v1/admin/cache", "operations", "cacheOperations", "pagination-cache-operations", (list) => {
    const c = C(), tb = c.byId("tbody-cache-operations"); if (!tb) return;
    if (!list.length) { c.renderTableState(tb, "empty", { colCount: 5 }); return; }
    tb.innerHTML = list.map((op) => `<tr><td class="text-xs text-mono">${c.esc(op.created_at ? new Date(op.created_at).toLocaleString(c.locale()) : "—")}</td><td><code class="text-xs text-mono">${c.esc(op.cache_key)}</code></td><td><strong class="text-warning">${c.esc(op.operation)}</strong></td><td>${c.esc(op.actor_email)}</td><td class="text-xs text-muted">${c.esc(op.details || "—")}</td></tr>`).join("");
  }, loadCacheOperations, p);

  const c = C();
  c.registerLoader(() => Promise.all([
    loadUserRoles(), loadAdminUsers(), loadAuditLogs(), refreshCentralEnginesTab(),
    loadTranslations(), loadPendingTranslations(), loadGlossary(), loadLocales(),
    loadNewsletters(), loadEmailGroups(), loadReminders(), loadRedirects(),
    loadCustomFields(), loadCacheOperations(),
  ]));

  c.registerActions({
    "show-role-form": () => openRoleForm(null),
    "hide-role-form": () => toggleHidden("role-form-container", false),
    "edit-role": (el) => openRoleForm(c.state.userRoles.find((r) => r.role_code === el.getAttribute("data-role"))),
    "save-role": saveRole,
    "delete-role": (el) => { const rc = el.getAttribute("data-role"); delItem(`/api/v1/admin/user-roles/${encodeURIComponent(rc)}`, c.t("admin.confirm.delete_role", "\"${code}\" rolünü silmek istediğinize emin misiniz?", "Are you sure you want to delete role \"${code}\"?").replace("${code}", rc), c.t("admin.toast.role_deleted", "Rol \"${code}\" silindi.", "Role \"${code}\" deleted.").replace("${code}", rc), loadUserRoles); },
    "assign-user-role": (el) => {
      const uid = parseInt(el.getAttribute("data-user-id") || "0", 10), rc = c.val("assign-role-sel-" + uid);
      if (uid && rc) submitJson(`/api/v1/admin/users/${uid}/assign-role`, { role_code: rc }, c.t("admin.toast.role_assigned", "Kullanıcı #${id} rolü \"${role}\" olarak atandı.", "User #${id} assigned role \"${role}\".").replace("${id}", uid).replace("${role}", rc), () => Promise.all([loadAdminUsers(), loadAuditLogs()]));
    },
    "assign-manual-user-role": () => {
      const uid = parseInt(c.val("manual-assign-user-id") || "0", 10), rc = c.val("manual-assign-role-code");
      if (!uid || !rc) { c.showToast(c.t("admin.error.invalid_user_role", "Geçerli kullanıcı ID ve rol seçilmelidir.", "Valid user ID and role are required."), true); return; }
      submitJson(`/api/v1/admin/users/${uid}/assign-role`, { role_code: rc }, c.t("admin.toast.role_assigned", "Kullanıcı #${id} rolü \"${role}\" olarak atandı.", "User #${id} assigned role \"${role}\".").replace("${id}", uid).replace("${role}", rc), () => Promise.all([loadAdminUsers(), loadAuditLogs()]));
    },
    "show-create-user": () => showUserForm(null),
    "hide-user-form": () => toggleHidden("user-crud-form-container", false),
    "save-user-form": saveUserForm,
    "edit-user": (el) => showUserForm(c.state.adminUsers.find((x) => Number(x.id ?? x.user_id) === Number(el.getAttribute("data-user-id")))),
    "delete-user": (el) => delItem(`/api/v1/admin/users/${el.getAttribute("data-user-id")}`, c.t("admin.confirm.delete_user", "Kullanıcı silinsin/anonimleştirilsin mi?", "Delete/anonymise this user?"), c.t("admin.toast.user_deleted", "Kullanıcı silindi.", "User deleted."), loadAdminUsers),
    "open-2fa-modal": () => { toggleHidden("totp-setup-stage-init", true); toggleHidden("totp-setup-stage-verify", false); c.openModal?.("totp-security-modal"); },
    "close-2fa-modal": () => c.closeModal?.("totp-security-modal"),
    "start-2fa-setup": async () => {
      let resp = await c.apiRequest("/api/v1/auth/totp/setup", { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
      if (resp.status === 405) resp = await c.apiRequest("/api/v1/auth/totp/setup");
      if (!resp.ok) { c.showToast(resp.errorMessage, true); return; }
      const d = resp.data || {}, img = c.byId("totp-qr-img"), codes = d.backup_codes || d.recovery_codes || [];
      if (img) img.src = d.qr_svg ? "data:image/svg+xml;charset=utf-8," + encodeURIComponent(d.qr_svg) : (d.qr_data_url || "");
      c.setVal("totp-secret-display", d.secret || "");
      if (codes.length && c.byId("totp-backup-codes-list")) c.byId("totp-backup-codes-list").textContent = codes.join("   ");
      toggleHidden("totp-backup-codes-wrap", codes.length > 0);
      toggleHidden("totp-setup-stage-init", false); toggleHidden("totp-setup-stage-verify", true);
    },
    "open-newsletter-drawer": () => { c.byId("form-newsletter")?.reset(); populateNewsletterSelects(); c.openDrawer("drawer-newsletter"); },
    "reset-newsletter-form": () => {
      c.byId("form-newsletter")?.reset();
      c.setVal("nl-scheduled-at", "");
      populateNewsletterSelects();
      c.switchLangTab?.(c.byId("form-newsletter"), "tr");
    },
    "delete-newsletter": (el) => delItem(`/api/v1/admin/newsletters/${el.getAttribute("data-id")}`, c.t("admin.confirm.delete_newsletter", "Bülten kaydı silinsin mi?", "Delete this newsletter record?"), c.t("admin.toast.newsletter_deleted", "Bülten silindi.", "Newsletter deleted."), loadNewsletters),
    "halt-renewed-reminders": () => submitJson("/api/v1/admin/reminders/halt-renewed", { action: "halt_renewed" }, c.t("admin.toast.reminders_halted", "Yenileyen abonelerin hatırlatmaları durduruldu.", "Reminders halted for renewed subscribers."), loadReminders),
    "delete-redirect": (el) => delItem(`/api/v1/admin/redirects/${el.getAttribute("data-id")}`, c.t("admin.confirm.delete_redirect", "301 yönlendirmesi silinsin mi?", "Delete this 301 redirect?"), c.t("admin.toast.redirect_deleted", "Yönlendirme silindi.", "Redirect deleted."), loadRedirects),
    "reset-translation-form": () => {
      c.byId("form-translation-upsert")?.reset();
      c.setVal("tr-group", "general");
      const warnBox = c.byId("tr-glossary-warnings");
      if (warnBox) { warnBox.classList.add("hidden"); warnBox.innerHTML = ""; }
    },
    "edit-tr": (el) => {
      const trItem = c.state.translations.find((x) => x.key === el.getAttribute("data-key"));
      if (trItem) {
        const det = c.byId("form-translation-upsert")?.closest("details");
        if (det) det.open = true;
        c.setVal("tr-key", trItem.key); c.setVal("tr-group", trItem.group || "general"); c.setVal("tr-val-tr", trItem.value_tr); c.setVal("tr-val-en", trItem.value_en);
        det?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
        c.byId("tr-val-tr")?.focus();
        c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", trItem.key));
      }
    },
    "delete-tr": (el) => { const k = el.getAttribute("data-key"); delItem(`/api/v1/admin/translations/${encodeURIComponent(k)}`, c.t("admin.confirm.delete_translation", "\"${key}\" çevirisi silinsin mi?", "Delete translation \"${key}\"?").replace("${key}", k), c.t("admin.toast.translation_deleted", "Çeviri silindi.", "Translation deleted."), loadTranslations); },
    "auto-translate-text": runAutoTranslate,
    "refresh-pending-translations": () => loadPendingTranslations(c.paginationState.pendingTranslations || 1),
    "approve-translation": (el) => {
      const k = el.getAttribute("data-key");
      if (k) submitJson(`/api/v1/admin/translations/${encodeURIComponent(k)}/approve`, {}, c.t("admin.toast.translation_approved", "Çeviri onaylandı.", "Translation approved."), () => Promise.all([loadPendingTranslations(), loadTranslations()]));
    },
    "reject-translation": (el) => {
      const k = el.getAttribute("data-key");
      if (k) submitJson(`/api/v1/admin/translations/${encodeURIComponent(k)}/reject`, {}, c.t("admin.toast.translation_rejected", "Çeviri taslağa döndürüldü.", "Translation reverted to draft."), () => Promise.all([loadPendingTranslations(), loadTranslations()]));
    },
    "refresh-glossary": loadGlossary,
    "show-glossary-form": () => {
      c.byId("form-glossary")?.reset();
      c.setVal("gl-id", "0");
      c.setVal("gl-priority", "50");
      if (c.byId("gl-case-sensitive")) c.byId("gl-case-sensitive").checked = false;
      if (c.byId("gl-is-active")) c.byId("gl-is-active").checked = true;
      const titleEl = c.byId("glossary-form-title");
      if (titleEl) titleEl.textContent = c.t("admin.system.gl_new_btn", "Yeni Terim", "New Term");
      toggleHidden("glossary-form-container", true);
      c.byId("glossary-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("gl-term-tr")?.focus();
    },
    "hide-glossary-form": () => toggleHidden("glossary-form-container", false),
    "edit-glossary": (el) => {
      const g = (c.state.glossary || []).find((x) => Number(x.id) === Number(el.getAttribute("data-id")));
      if (!g) return;
      c.setVal("gl-id", g.id);
      c.setVal("gl-term-tr", g.term_tr);
      c.setVal("gl-term-en", g.term_en || "");
      c.setVal("gl-preferred-en", g.preferred_en || "");
      c.setVal("gl-abbr-tr", g.abbreviation_tr || "");
      c.setVal("gl-abbr-en", g.abbreviation_en || "");
      c.setVal("gl-domain", g.domain || "editorial");
      c.setVal("gl-rule-type", g.rule_type || "fixed");
      c.setVal("gl-priority", String(g.priority ?? 50));
      c.setVal("gl-source-ref", g.source_ref || "");
      c.setVal("gl-forbidden-variants", Array.isArray(g.forbidden_variants) ? g.forbidden_variants.join(", ") : "");
      c.setVal("gl-notes", g.notes || "");
      if (c.byId("gl-case-sensitive")) c.byId("gl-case-sensitive").checked = !!g.case_sensitive;
      if (c.byId("gl-is-active")) c.byId("gl-is-active").checked = g.is_active !== false;
      const titleEl = c.byId("glossary-form-title");
      if (titleEl) titleEl.textContent = `${c.t("admin.system.gl_edit", "Terimi Düzenle", "Edit Term")} (#${g.id})`;
      toggleHidden("glossary-form-container", true);
      c.byId("glossary-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("gl-term-tr")?.focus();
      c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", g.term_tr));
    },
    "delete-glossary": (el) => {
      const id = el.getAttribute("data-id");
      delItem(`/api/v1/admin/glossary/${encodeURIComponent(id)}`, c.t("admin.confirm.delete_glossary", "#${id} sözlük terimi silinsin mi?", "Delete glossary term #${id}?").replace("${id}", id), c.t("admin.toast.glossary_deleted", "Sözlük terimi silindi.", "Glossary term deleted."), loadGlossary);
    },
    "show-locale-form": () => {
      c.byId("form-locale")?.reset();
      c.setVal("loc-edit-mode", "create");
      const codeEl = c.byId("loc-code");
      if (codeEl) { codeEl.value = ""; codeEl.readOnly = false; }
      c.setVal("loc-dir", "ltr");
      c.setVal("loc-fallback-code", "tr");
      c.setVal("loc-sort-order", "0");
      if (c.byId("loc-is-enabled")) c.byId("loc-is-enabled").checked = true;
      if (c.byId("loc-is-default")) c.byId("loc-is-default").checked = false;
      const titleEl = c.byId("locale-form-title");
      if (titleEl) titleEl.textContent = c.t("admin.system.loc_new", "Yeni Dil Ekle", "Add Locale");
      toggleHidden("locale-form-container", true);
      c.byId("locale-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      codeEl?.focus();
    },
    "hide-locale-form": () => toggleHidden("locale-form-container", false),
    "edit-locale": (el) => {
      const loc = (c.state.locales || []).find((x) => x.code === el.getAttribute("data-code"));
      if (!loc) return;
      c.setVal("loc-edit-mode", "edit");
      const codeEl = c.byId("loc-code");
      if (codeEl) { codeEl.value = loc.code; codeEl.readOnly = true; }
      c.setVal("loc-name-native", loc.name_native || "");
      c.setVal("loc-name-en", loc.name_en || "");
      c.setVal("loc-hreflang", loc.hreflang || "");
      c.setVal("loc-og-locale", loc.og_locale || "");
      c.setVal("loc-dir", loc.dir || "ltr");
      c.setVal("loc-fallback-code", loc.fallback_code || "tr");
      c.setVal("loc-sort-order", String(loc.sort_order ?? 0));
      if (c.byId("loc-is-enabled")) c.byId("loc-is-enabled").checked = loc.is_enabled !== false;
      if (c.byId("loc-is-default")) c.byId("loc-is-default").checked = !!loc.is_default;
      const titleEl = c.byId("locale-form-title");
      if (titleEl) titleEl.textContent = `${c.t("admin.system.loc_edit", "Dili Düzenle", "Edit Locale")} (${loc.code})`;
      toggleHidden("locale-form-container", true);
      c.byId("locale-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("loc-name-native")?.focus();
      c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", loc.name_native || loc.code));
    },
    "delete-locale": (el) => {
      const code = el.getAttribute("data-code");
      delItem(`/api/v1/admin/locales/${encodeURIComponent(code)}`, c.t("admin.confirm.delete_locale", "\"${code}\" yerel ayarı silinsin mi?", "Delete locale \"${code}\"?").replace("${code}", code), c.t("admin.toast.locale_deleted", "Yerel ayar silindi.", "Locale deleted."), loadLocales);
    },
    "show-email-group-form": () => {
      c.byId("form-email-group")?.reset();
      c.setVal("eg-id", "0");
      const codeEl = c.byId("eg-code");
      if (codeEl) { codeEl.value = ""; codeEl.readOnly = false; }
      c.setVal("eg-filter-json", "{}");
      c.setVal("eg-static-emails-json", "[]");
      c.setVal("eg-sort-order", "0");
      if (c.byId("eg-requires-consent")) c.byId("eg-requires-consent").checked = true;
      if (c.byId("eg-is-active")) c.byId("eg-is-active").checked = true;
      const titleEl = c.byId("email-group-form-title");
      if (titleEl) titleEl.textContent = c.t("admin.workflow.eg_new", "Yeni E-Posta Grubu", "New Email Group");
      toggleHidden("email-group-form-container", true);
      c.byId("email-group-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      codeEl?.focus();
    },
    "hide-email-group-form": () => toggleHidden("email-group-form-container", false),
    "edit-email-group": (el) => {
      const eg = (c.state.emailGroups || []).find((x) => x.code === el.getAttribute("data-code"));
      if (!eg) return;
      c.setVal("eg-id", String(eg.id || 0));
      const codeEl = c.byId("eg-code");
      if (codeEl) { codeEl.value = eg.code; codeEl.readOnly = true; }
      c.setVal("eg-name-tr", eg.name_tr || "");
      c.setVal("eg-name-en", eg.name_en || "");
      c.setVal("eg-desc-tr", eg.description_tr || "");
      c.setVal("eg-desc-en", eg.description_en || "");
      c.setVal("eg-sort-order", String(eg.sort_order ?? 0));
      c.setVal("eg-filter-json", eg.filter_json || "{}");
      c.setVal("eg-static-emails-json", eg.static_emails_json || "[]");
      if (c.byId("eg-requires-consent")) c.byId("eg-requires-consent").checked = eg.requires_marketing_consent !== false;
      if (c.byId("eg-is-active")) c.byId("eg-is-active").checked = eg.is_active !== false;
      const titleEl = c.byId("email-group-form-title");
      if (titleEl) titleEl.textContent = `${c.t("admin.workflow.eg_edit", "E-Posta Grubunu Düzenle", "Edit Email Group")} (${eg.code})`;
      toggleHidden("email-group-form-container", true);
      c.byId("email-group-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("eg-name-tr")?.focus();
      c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", eg.name_tr || eg.code));
    },
    "delete-email-group": (el) => {
      const code = el.getAttribute("data-code");
      delItem(`/api/v1/admin/email-groups/${encodeURIComponent(code)}`, c.t("admin.confirm.delete_email_group", "\"${code}\" e-posta grubu silinsin mi?", "Delete email group \"${code}\"?").replace("${code}", code), c.t("admin.toast.email_group_deleted", "E-posta grubu silindi.", "Email group deleted."), loadEmailGroups);
    },
    "trigger-backup": async () => {
      if (!c.hasPerm("backup:manage")) return;
      const btn = c.byId("btn-trigger-backup");
      const resEl = c.byId("backup-last-result");
      const fnEl = c.byId("backup-last-filename");
      const szEl = c.byId("backup-last-size");
      if (btn) btn.disabled = true;
      if (resEl) {
        resEl.className = "text-sm text-muted";
        resEl.textContent = c.t("admin.backup.creating", "Yedek oluşturuluyor...", "Creating backup...");
      }
      try {
        const resp = await c.apiRequest("/api/v1/admin/backup", { method: "POST", headers: { "Content-Type": "application/json" }, body: "{}" });
        if (resp.ok) {
          const d = resp.data || {};
          if (resEl) {
            resEl.className = "text-sm text-success font-semibold";
            resEl.textContent = c.t("admin.backup.completed", "Veritabanı yedeği başarıyla alındı.", "Database backup completed successfully.");
          }
          if (fnEl) fnEl.textContent = d.filename || d.path || "";
          if (szEl) szEl.textContent = d.size_bytes != null ? `(${(Number(d.size_bytes) / 1048576).toFixed(2)} MB)` : "";
          c.showToast(c.t("admin.backup.completed", "Veritabanı yedeği başarıyla alındı.", "Database backup completed successfully."));
        } else {
          if (resEl) {
            resEl.className = "text-sm text-danger";
            resEl.textContent = resp.errorMessage;
          }
          if (fnEl) fnEl.textContent = "";
          if (szEl) szEl.textContent = "";
          c.showToast(resp.errorMessage, true);
        }
      } finally {
        if (btn) btn.disabled = false;
      }
    },
    "show-custom-field-form": () => {
      c.setVal("cf-key", "");
      c.setVal("cf-label-tr", "");
      c.setVal("cf-label-en", "");
      c.setVal("cf-type", "text");
      c.setVal("cf-applies-to", "content");
      c.setVal("cf-sort-order", "0");
      c.setVal("cf-options", "");
      if (c.byId("cf-required")) c.byId("cf-required").checked = false;
      const titleEl = c.byId("custom-field-form-title");
      if (titleEl) titleEl.textContent = c.t("admin.system.cf_new", "Yeni Özel Alan", "New Custom Field");
      toggleHidden("custom-field-form-container", true);
      c.byId("custom-field-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("cf-key")?.focus();
    },
    "hide-custom-field-form": () => toggleHidden("custom-field-form-container", false),
    "edit-custom-field": (el) => {
      const k = el.getAttribute("data-key");
      const f = (c.state.customFields || []).find((x) => x.field_key === k);
      if (!f) return;
      c.setVal("cf-key", f.field_key || "");
      c.setVal("cf-label-tr", f.label_tr || "");
      c.setVal("cf-label-en", f.label_en || "");
      c.setVal("cf-type", f.field_type || "text");
      c.setVal("cf-applies-to", f.applies_to || "content");
      c.setVal("cf-sort-order", f.sort_order ?? 0);
      c.setVal("cf-options", f.options_json && f.options_json !== "[]" ? f.options_json : "");
      if (c.byId("cf-required")) c.byId("cf-required").checked = !!f.is_required;
      const titleEl = c.byId("custom-field-form-title");
      if (titleEl) titleEl.textContent = `${c.t("admin.system.cf_edit", "Özel Alanı Düzenle", "Edit Custom Field")} (${f.field_key})`;
      toggleHidden("custom-field-form-container", true);
      c.byId("custom-field-form-container")?.scrollIntoView?.({ behavior: "smooth", block: "nearest" });
      c.byId("cf-label-tr")?.focus();
      c.showToast(c.t("admin.toast.editing_item", "\"${name}\" düzenlemeye alındı.", "\"${name}\" loaded for editing.").replace("${name}", f.label_tr || f.field_key));
    },
    "save-custom-field": () => saveCustomField(),
    "delete-custom-field": (el) => { const k = el.getAttribute("data-key"); delItem(`/api/v1/admin/custom-fields/${encodeURIComponent(k)}`, c.t("admin.confirm.delete_custom_field", "\"${key}\" alanı silinsin mi?", "Delete field \"${key}\"?").replace("${key}", k), c.t("admin.toast.custom_field_deleted", "Özel alan silindi.", "Custom field deleted."), loadCustomFields); },
    "flush-cache": async () => { if (await c.confirmDialog(c.t("admin.confirm.flush_cache", "Tüm önbellek katmanları temizlenecek. Devam etmek istiyor musunuz?", "Flush all cache layers?"))) await submitJson("/api/v1/admin/cache", { cache_key: "all" }, c.t("admin.toast.cache_flushed", "Tüm önbellek katmanları temizlendi.", "All cache layers flushed."), loadCacheOperations); },
  });

  async function saveCustomField() {
    const fKey = c.val("cf-key"), lblTr = c.val("cf-label-tr"), fType = c.val("cf-type") || "text";
    if (!fKey || !lblTr) { c.showToast(c.t("admin.error.custom_field_required", "Alan anahtarı ve Türkçe etiketi zorunludur.", "Field key and Turkish label are required."), true); return; }
    let optsJson = (c.val("cf-options") || "").trim();
    if (fType === "select") {
      if (!optsJson) {
        c.byId("cf-options")?.focus();
        c.showToast(c.t("admin.error.custom_field_select_options", "Seçim listesi türü için en az bir seçenek girin (JSON dizi veya virgülle ayrılmış).", "Enter at least one option for select type (JSON array or comma-separated)."), true);
        return;
      }
      if (optsJson.startsWith("[")) {
        try {
          const parsed = JSON.parse(optsJson);
          if (!Array.isArray(parsed) || !parsed.length) throw new Error("empty");
        } catch (_) {
          c.byId("cf-options")?.focus();
          c.showToast(c.t("admin.error.custom_field_options_json", "Seçenekler geçerli bir JSON dizisi (ör. [\"A\",\"B\"]) veya virgülle ayrılmış liste olmalıdır.", "Options must be a valid JSON array (e.g. [\"A\",\"B\"]) or comma-separated list."), true);
          return;
        }
      } else {
        const items = optsJson.split(",").map((s) => s.trim()).filter(Boolean);
        if (!items.length) {
          c.byId("cf-options")?.focus();
          c.showToast(c.t("admin.error.custom_field_select_options", "Seçim listesi türü için en az bir seçenek girin (JSON dizi veya virgülle ayrılmış).", "Enter at least one option for select type (JSON array or comma-separated)."), true);
          return;
        }
        optsJson = JSON.stringify(items);
      }
    } else if (!optsJson) {
      optsJson = "[]";
    }
    await submitJson(
      "/api/v1/admin/custom-fields",
      { field_key: fKey, label_tr: lblTr, label_en: c.val("cf-label-en") || lblTr, field_type: fType, options_json: optsJson, is_required: !!c.byId("cf-required")?.checked, sort_order: parseInt(c.val("cf-sort-order") || "0", 10), applies_to: c.val("cf-applies-to") || "content" },
      c.t("admin.toast.custom_field_saved", "Özel alan kaydedildi.", "Custom field saved."),
      () => {
        c.setVal("cf-key", "");
        c.setVal("cf-label-tr", "");
        c.setVal("cf-label-en", "");
        c.setVal("cf-type", "text");
        c.setVal("cf-applies-to", "content");
        c.setVal("cf-sort-order", "0");
        c.setVal("cf-options", "");
        if (c.byId("cf-required")) c.byId("cf-required").checked = false;
        toggleHidden("custom-field-form-container", false);
        return loadCustomFields();
      },
    );
  }

  document.addEventListener("submit", async (ev) => {
    const id = ev.target?.id;
    if (!id) return;
    if (id === "form-admin-user") {
      ev.preventDefault();
      await saveUserForm();
    } else if (id === "form-admin-role") {
      ev.preventDefault();
      await saveRole();
    } else if (id === "form-custom-field") {
      ev.preventDefault();
      await saveCustomField();
    } else if (id === "form-site-setting") {
      ev.preventDefault();
      await submitJson("/api/v1/admin/settings", { key: c.val("set-key"), value_tr: c.val("set-val-tr"), value_en: c.val("set-val-en") || c.val("set-val-tr"), setting_group: c.val("set-group") || "general" }, c.t("admin.toast.setting_updated", "Site ayarı güncellendi.", "Site setting updated."), refreshCentralEnginesTab);
    } else if (id === "form-sys-notif") {
      ev.preventDefault();
      const aud = c.val("notif-audience") || "all";
      await submitJson("/api/v1/admin/notifications", { target_audience: aud.toUpperCase() === "ALL" ? "all" : aud, severity: c.val("notif-severity"), is_banner: c.val("notif-banner") === "true", category: "editorial", title_tr: c.val("notif-title-tr"), message_tr: c.val("notif-msg-tr") }, c.t("admin.toast.notification_published", "Duyuru yayınlandı.", "Notification published."), () => { ev.target.reset(); return refreshCentralEnginesTab(); });
    } else if (id === "form-translation-upsert") {
      ev.preventDefault();
      await submitJson("/api/v1/admin/translations", { key: c.val("tr-key"), group: c.val("tr-group") || "general", value_tr: c.val("tr-val-tr"), value_en: c.val("tr-val-en") }, c.t("admin.toast.translation_saved", "Çeviri \"${key}\" kaydedildi.", "Translation \"${key}\" saved.").replace("${key}", c.val("tr-key")), () => { ev.target.reset(); return Promise.all([loadTranslations(), loadPendingTranslations()]); });
    } else if (id === "form-glossary") {
      ev.preventDefault();
      if (!c.hasPerm("glossary:write|glossary:manage")) return;
      const termTr = c.val("gl-term-tr"), prefEn = c.val("gl-preferred-en");
      if (!termTr || !prefEn) {
        c.showToast(c.t("admin.glossary.required_fields", "Türkçe terim ve tercih edilen İngilizce karşılık zorunludur.", "Turkish term and preferred English equivalent are required."), true);
        return;
      }
      const btn = c.byId("btn-glossary-save");
      if (btn) btn.disabled = true;
      try {
        const body = {
          id: parseInt(c.val("gl-id") || "0", 10),
          term_tr: termTr,
          term_en: c.val("gl-term-en") || prefEn,
          abbreviation_tr: c.val("gl-abbr-tr"),
          abbreviation_en: c.val("gl-abbr-en"),
          domain: c.val("gl-domain") || "editorial",
          rule_type: c.val("gl-rule-type") || "fixed",
          preferred_en: prefEn,
          forbidden_variants: c.val("gl-forbidden-variants").split(",").map((s) => s.trim()).filter(Boolean),
          case_sensitive: !!c.byId("gl-case-sensitive")?.checked,
          priority: parseInt(c.val("gl-priority") || "50", 10),
          source_ref: c.val("gl-source-ref"),
          notes: c.val("gl-notes"),
          is_active: !!c.byId("gl-is-active")?.checked,
        };
        await submitJson("/api/v1/admin/glossary", body, c.t("admin.toast.glossary_saved", "Sözlük terimi kaydedildi.", "Glossary term saved."), () => {
          toggleHidden("glossary-form-container", false);
          ev.target.reset();
          return loadGlossary();
        });
      } finally {
        if (btn) btn.disabled = false;
      }
    } else if (id === "form-locale") {
      ev.preventDefault();
      if (!c.hasPerm("locales:manage")) return;
      const code = c.val("loc-code"), nativeName = c.val("loc-name-native"), enName = c.val("loc-name-en"), hreflang = c.val("loc-hreflang"), ogLoc = c.val("loc-og-locale");
      if (!code || !nativeName || !enName || !hreflang || !ogLoc) {
        c.showToast(c.t("admin.locales.required_fields", "Kod, yerel ad, İngilizce ad, hreflang ve og:locale alanları zorunludur.", "Code, native name, English name, hreflang and og:locale are required."), true);
        return;
      }
      const mode = c.val("loc-edit-mode") || "create";
      const btn = c.byId("btn-locale-save");
      if (btn) btn.disabled = true;
      try {
        const body = {
          code,
          name_native: nativeName,
          name_en: enName,
          hreflang,
          og_locale: ogLoc,
          dir: c.val("loc-dir") || "ltr",
          is_default: !!c.byId("loc-is-default")?.checked,
          is_enabled: !!c.byId("loc-is-enabled")?.checked,
          fallback_code: c.val("loc-fallback-code") || "tr",
          sort_order: parseInt(c.val("loc-sort-order") || "0", 10),
        };
        const url = mode === "edit" ? `/api/v1/admin/locales/${encodeURIComponent(code)}` : "/api/v1/admin/locales";
        await submitJson(url, body, c.t("admin.toast.locale_saved", "Yerel ayar kaydedildi.", "Locale saved."), () => {
          toggleHidden("locale-form-container", false);
          return loadLocales();
        }, mode === "edit" ? "PUT" : "POST");
      } finally {
        if (btn) btn.disabled = false;
      }
    } else if (id === "form-email-group") {
      ev.preventDefault();
      if (!c.hasPerm("email_groups:write|email_groups:manage")) return;
      const code = c.val("eg-code"), nameTr = c.val("eg-name-tr");
      if (!code || !nameTr) {
        c.showToast(c.t("admin.email_groups.required_fields", "Grup kodu ve Türkçe grup adı zorunludur.", "Group code and Turkish name are required."), true);
        return;
      }
      const filterStr = c.val("eg-filter-json") || "{}";
      const staticStr = c.val("eg-static-emails-json") || "[]";
      try {
        const parsedFilter = JSON.parse(filterStr);
        if (!parsedFilter || typeof parsedFilter !== "object" || Array.isArray(parsedFilter)) throw new Error("filter");
        const parsedStatic = JSON.parse(staticStr);
        if (!Array.isArray(parsedStatic)) throw new Error("static");
      } catch (_) {
        c.showToast(c.t("admin.email_groups.invalid_json", "Filtre (JSON nesnesi) ve Statik E-Posta Listesi (JSON dizisi) geçerli JSON olmalıdır.", "Filter (JSON object) and Static Emails (JSON array) must be valid JSON."), true);
        return;
      }
      const btn = c.byId("btn-email-group-save");
      if (btn) btn.disabled = true;
      try {
        const body = {
          id: parseInt(c.val("eg-id") || "0", 10),
          code,
          name_tr: nameTr,
          name_en: c.val("eg-name-en") || nameTr,
          description_tr: c.val("eg-desc-tr"),
          description_en: c.val("eg-desc-en"),
          filter_json: filterStr,
          static_emails_json: staticStr,
          requires_marketing_consent: !!c.byId("eg-requires-consent")?.checked,
          is_active: !!c.byId("eg-is-active")?.checked,
          sort_order: parseInt(c.val("eg-sort-order") || "0", 10),
        };
        await submitJson("/api/v1/admin/email-groups", body, c.t("admin.toast.email_group_saved", "E-posta grubu kaydedildi.", "Email group saved."), () => {
          toggleHidden("email-group-form-container", false);
          return loadEmailGroups();
        });
      } finally {
        if (btn) btn.disabled = false;
      }
    } else if (id === "form-newsletter") {
      ev.preventDefault();
      if (!c.requireField(ev.target, "nl-subject-tr", c.t("admin.error.newsletter_subject_required", "Bülten konusu (TR) zorunludur.", "Newsletter subject (TR) is required."), "tr")) return;
      const adId = c.val("nl-sponsor-ad-id");
      const schedVal = c.val("nl-scheduled-at");
      const statusVal = c.val("nl-status") || (schedVal ? "scheduled" : "sent");
      const egCode = c.val("nl-email-group");
      const isScheduled = statusVal === "scheduled";
      const isDraft = statusVal === "draft";
      if (isScheduled) {
        const schedMs = schedVal ? new Date(schedVal).getTime() : NaN;
        if (!schedVal || Number.isNaN(schedMs) || schedMs <= Date.now()) {
          c.byId("nl-scheduled-at")?.focus();
          c.showToast(c.t("admin.error.newsletter_schedule_future", "Zamanlanmış gönderim için ileri bir tarih ve saat seçin.", "Please select a future date and time for scheduled dispatch."), true);
          return;
        }
      }
      const okToast = isScheduled
        ? c.t("admin.toast.newsletter_scheduled", "E-Bülten zamanlandı.", "Newsletter scheduled.")
        : isDraft
          ? c.t("admin.toast.newsletter_drafted", "E-Bülten taslağı kaydedildi.", "Newsletter draft saved.")
          : c.t("admin.toast.newsletter_sent", "E-Bülten gönderildi.", "Newsletter sent.");
      await submitJson("/api/v1/admin/newsletters", {
        id: 0,
        subject_tr: c.val("nl-subject-tr"),
        subject_en: c.val("nl-subject-en") || c.val("nl-subject-tr"),
        target_group_filter: c.val("nl-group-filter"),
        email_group_code: egCode || null,
        target_institution: c.val("nl-institution"),
        target_ssb_tag: c.val("nl-ssb-tag"),
        sponsor_ad_id: adId ? parseInt(adId, 10) : null,
        sponsor_name: c.val("nl-sponsor-name") || null,
        body_html: c.val("nl-body-html") || null,
        recipient_count: 0,
        status: statusVal,
        scheduled_at: isDraft ? null : (schedVal || null),
        sent_at: isScheduled || isDraft ? "" : new Date().toISOString().slice(0, 16).replace("T", " "),
      }, okToast, () => { c.closeDrawer("drawer-newsletter"); ev.target.reset(); return loadNewsletters(1); });
    } else if (id === "form-seo-redirect") {
      ev.preventDefault();
      await submitJson("/api/v1/admin/redirects", { id: 0, old_path: c.val("seo-old-path"), new_path: c.val("seo-new-path"), status_code: 301, hit_count: 0 }, c.t("admin.toast.redirect_added", "301 yönlendirme kuralı eklendi.", "301 redirect rule added."), () => { ev.target.reset(); return loadRedirects(1); });
    } else if (id === "form-totp-verify-setup") {
      ev.preventDefault();
      const code = c.val("totp-setup-code");
      if (!code) { c.showToast(c.t("admin.error.totp_code_required", "Lütfen doğrulama kodunu girin.", "Please enter the verification code."), true); return; }
      let r = await c.apiRequest("/api/v1/auth/totp/verify-setup", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code }) });
      if (r.status === 404) r = await c.apiRequest("/api/v1/auth/totp/verify", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ code }) });
      if (r.ok) {
        const codes = r.data?.backup_codes || r.data?.recovery_codes || [];
        if (codes.length && c.byId("totp-backup-codes-list")) { c.byId("totp-backup-codes-list").textContent = codes.join("   "); toggleHidden("totp-backup-codes-wrap", true); c.showToast(c.t("admin.toast.totp_enabled_backup", "2FA etkinleştirildi. Yedek kodlarınızı kaydedin.", "2FA enabled. Save your backup codes.")); }
        else { c.showToast(c.t("admin.toast.totp_enabled", "2FA başarıyla etkinleştirildi.", "2FA successfully enabled.")); c.closeModal?.("totp-security-modal"); }
      } else c.showToast(r.errorMessage, true);
    } else if (id === "form-totp-disable") {
      ev.preventDefault();
      const password = c.byId("totp-disable-password")?.value || "";
      if (!password) { c.showToast(c.t("admin.error.password_required", "Lütfen mevcut parolanızı girin.", "Please enter your current password."), true); return; }
      await submitJson("/api/v1/auth/totp/disable", { password }, c.t("admin.toast.totp_disabled", "2FA devre dışı bırakıldı.", "2FA disabled."), () => { ev.target.reset(); c.closeModal?.("totp-security-modal"); });
    }
  });

  const filterAudit = () => {
    const act = c.val("audit-filter-action"), actor = c.val("audit-filter-actor").toLowerCase();
    renderAuditLogs(c.state.auditLogs.filter((l) => (!act || l.action === act) && (!actor || (l.actor_email || "").toLowerCase().includes(actor))));
  };
  document.addEventListener("change", (ev) => {
    const id = ev.target?.id;
    if (id === "audit-filter-action") filterAudit();
    else if (id === "tr-pending-status-filter") loadPendingTranslations(1);
    else if (id === "glossary-domain-filter" || id === "glossary-active-only") loadGlossary();
    else if (id === "nl-scheduled-at" && c.val("nl-scheduled-at") && c.val("nl-status") === "sent") c.setVal("nl-status", "scheduled");
  });
  document.addEventListener("input", (ev) => {
    const id = ev.target?.id;
    if (id === "audit-filter-actor") filterAudit();
    else if (id === "user-role-search") renderAdminUsers();
    else if (id === "tr-search") renderTranslationsTable();
    else if (id === "glossary-search") renderGlossaryTable();
    else if (id === "waf-search") renderSecurityLogsPage(1);
    else if (id === "redirect-search") renderRedirectsTable();
  });
  window.addEventListener("msi:admin:refreshed", populateNewsletterSelects);
  window.addEventListener("msi:admin:tab", (ev) => {
    if (ev.detail?.initial) return;
    const tabId = ev.detail?.tabId;
    if (tabId === "tab-8") Promise.all([loadTranslations(1), loadPendingTranslations(1), loadGlossary(), loadLocales()]);
    else if (tabId === "tab-6") Promise.all([loadNewsletters(1), loadEmailGroups()]).then(populateNewsletterSelects);
  });
})();
