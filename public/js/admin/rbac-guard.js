(function () {
  "use strict";

  const RESOURCE_ALIASES = {
    content: "contents",
    category: "categories",
    approval: "approvals",
    newsletter: "newsletters",
    notification: "notifications",
    subscription: "subscriptions",
    setting: "settings",
    user: "users",
    role: "roles",
    ad: "ads",
    translation: "translations",
    corporate_request: "corporate_requests",
    issue: "issues",
    invoice: "invoices",
    product: "commerce",
    plan: "subscriptions",
    redirect: "redirects",
    audit_log: "audit",
    audit_logs: "audit",
  };

  function canonRes(r) {
    const clean = String(r || "").trim().toLowerCase();
    return RESOURCE_ALIASES[clean] || clean;
  }

  function permMatches(grantedRaw, requiredRaw) {
    const g = String(grantedRaw || "").trim().toLowerCase().replace(/\./g, ":");
    const r = String(requiredRaw || "").trim().toLowerCase().replace(/\./g, ":");
    if (!g || !r) return false;
    if (g === "*" || g === r) return true;

    const [gResRaw, gAct = "*"] = g.includes(":") ? g.split(":", 2) : [g, "*"];
    const [rResRaw, rAct = "*"] = r.includes(":") ? r.split(":", 2) : [r, "*"];
    const gRes = canonRes(gResRaw);
    const rRes = canonRes(rResRaw);

    if (
      gRes === "users" &&
      (gAct === "*" || gAct === "manage" || gAct === "verify") &&
      rRes === "corporate_requests" &&
      ["read", "view", "decide", "write"].includes(rAct)
    ) {
      return true;
    }

    if (gRes !== rRes) return false;
    if (gAct === "*" || rAct === "*") return true;
    if (gAct === rAct) return true;

    const actEquiv = (act) => {
      if (act === "view") return "read";
      if (act === "publish") return "publish";
      return act;
    };
    const ga = actEquiv(gAct);
    const ra = actEquiv(rAct);
    if (ga === ra) return true;
    if (ga === "manage" && ["read", "write", "publish", "delete", "decide", "upload", "export"].includes(ra)) {
      return true;
    }
    if (ga === "publish" && ["write", "read"].includes(ra)) return true;
    if ((ga === "write" || ga === "upload" || ga === "decide" || ga === "export") && ra === "read") {
      return true;
    }
    return false;
  }

  function hasAnyPerm(grantedList, requiredPipeExpr) {
    if (!requiredPipeExpr) return true;
    const reqs = String(requiredPipeExpr)
      .split("|")
      .map((s) => s.trim())
      .filter(Boolean);
    if (!reqs.length) return true;
    return reqs.some((req) => (grantedList || []).some((g) => permMatches(g, req)));
  }

  const ROLE_LANDING_TABS = {
    ads_manager: ["tab-2", "tab-14", "tab-3", "tab-10"],
    business_dev: ["tab-2", "tab-14", "tab-3"],
    finance_viewer: ["tab-14", "tab-3", "tab-15"],
    support_agent: ["tab-4", "tab-11", "tab-7", "tab-8"],
    content_publisher: ["tab-1", "tab-16", "tab-18"],
    content_editor: ["tab-1", "tab-18", "tab-10"],
    editor: ["tab-1", "tab-18", "tab-10"],
    managing_editor: ["tab-1", "tab-16", "tab-18"],
    editor_in_chief: ["tab-1", "tab-16", "tab-3"],
  };

  async function applyRbacVisibility() {
    const ctx = window.MSIAdmin && window.MSIAdmin._ctx;
    if (!ctx) return;

    const res = await ctx.apiRequest("/api/v1/admin/me");
    if (!res.ok || !res.data) return;

    const me = res.data;
    const perms = Array.isArray(me.permissions) ? me.permissions : [];

    const emailBadge = document.getElementById("operator-email-display");
    const roleBadge = document.getElementById("operator-role-display");
    if (emailBadge && me.email) emailBadge.textContent = me.email;
    if (roleBadge && me.role) roleBadge.textContent = me.role;

    document.querySelectorAll("[data-required-perm]").forEach((el) => {
      const expr = el.getAttribute("data-required-perm");
      const allowed = hasAnyPerm(perms, expr);
      if (!allowed) {
        el.style.display = "none";
        el.setAttribute("data-rbac-hidden", "1");
        if (el.hasAttribute("data-tab")) {
          const paneId = el.getAttribute("data-tab");
          const pane = paneId ? document.getElementById(paneId) : null;
          if (pane) pane.style.display = "none";
        }
      } else {
        el.style.removeProperty("display");
        el.removeAttribute("data-rbac-hidden");
        if (el.hasAttribute("data-tab")) {
          const paneId = el.getAttribute("data-tab");
          const pane = paneId ? document.getElementById(paneId) : null;
          if (pane) pane.style.removeProperty("display");
        }
      }
    });

    document.querySelectorAll(".sidebar-group, .sidebar-footer").forEach((grp) => {
      const visibleItems = grp.querySelectorAll('.sidebar-link:not([data-rbac-hidden="1"])');
      grp.style.display = visibleItems.length === 0 ? "none" : "";
    });

    const activeBtn = document.querySelector(".sidebar-link.active[data-tab], .tab-btn.active[data-tab]");
    const preferredTabs = ROLE_LANDING_TABS[me.role] || [];
    const preferredTarget = preferredTabs
      .map((tid) => document.querySelector(`.sidebar-link[data-tab="${tid}"]:not([data-rbac-hidden="1"]), .tab-btn[data-tab="${tid}"]:not([data-rbac-hidden="1"])`))
      .find(Boolean);

    if (preferredTarget && (!activeBtn || activeBtn.getAttribute("data-rbac-hidden") === "1" || activeBtn.getAttribute("data-tab") !== preferredTarget.getAttribute("data-tab"))) {
      ctx.switchTab(preferredTarget.getAttribute("data-tab"));
    } else if (activeBtn && activeBtn.getAttribute("data-rbac-hidden") === "1") {
      const firstVisible = document.querySelector('.sidebar-link[data-tab]:not([data-rbac-hidden="1"]), .tab-btn[data-tab]:not([data-rbac-hidden="1"])');
      if (firstVisible) {
        ctx.switchTab(firstVisible.getAttribute("data-tab"));
      }
    }

    window.MSIAdmin._me = me;
  }

  document.addEventListener("DOMContentLoaded", () => {
    setTimeout(applyRbacVisibility, 0);
  });
})();
