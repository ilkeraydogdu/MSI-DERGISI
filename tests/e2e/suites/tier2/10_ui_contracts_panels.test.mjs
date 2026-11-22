import {
  getCachedRoleSession,
  getCachedSuperAdmin,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-037 Boundaries: Admin Panel & Design Studio Access & Contracts ────
  suite.describe("REQ-037 Admin Panel & Design Studio Boundaries", () => {
    suite.test("authenticated reader role on GET /{adminPath} sees login gateway (not admin dashboard)", { req: "REQ-037" }, async (c) => {
      const reader = await getCachedRoleSession(c, "registered");
      const res = await reader.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, 'id="login-form"');
      c.assert.assertNotIncludes(res.text, "admin-sidebar");
    });

    suite.test("unauthenticated GET /{adminPath}/tasarim redirects to /{adminPath}", { req: "REQ-037" }, async (c) => {
      const res = await c.http.createClient().get(`/${c.adminPath}/tasarim`, { redirect: "manual" });
      c.assert.assertStatus(res, [302, 303, 307]);
      c.assert.assertIncludes(res.headers.get("location") || "", `/${c.adminPath}`);
    });

    suite.test("unauthenticated POST /{adminPath}/tasarim/onizleme returns 401", { req: "REQ-037" }, async (c) => {
      const res = await c.http.createClient().post(`/${c.adminPath}/tasarim/onizleme`, { json: {} });
      c.assert.assertStatus(res, 401);
    });

    suite.test("authenticated admin gateway sets X-Frame-Options and noindex headers", { req: "REQ-037" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.headers.get("x-robots-tag") || "", "noindex");
      c.assert.assertEqual(res.headers.get("x-frame-options"), "SAMEORIGIN");
    });

    suite.test("M4 role-adaptive Admin SSR+JS RBAC gates commercial/system tabs for finance_viewer and ads_manager", { req: "REQ-037" }, async (c) => {
      const fin = await getCachedRoleSession(c, "finance_viewer");
      const ads = await getCachedRoleSession(c, "ads_manager");
      const [finHtml, adsHtml, finMe, adsMe, coreJs] = await Promise.all([
        fin.client.get(`/${c.adminPath}`),
        ads.client.get(`/${c.adminPath}`),
        fin.client.get("/api/v1/admin/me"),
        ads.client.get("/api/v1/admin/me"),
        fin.client.get("/js/admin/core.js"),
      ]);
      c.assert.assertStatus(finHtml, 200);
      c.assert.assertStatus(adsHtml, 200);
      c.assert.assertStatus(finMe, 200);
      c.assert.assertStatus(adsMe, 200);
      c.assert.assertStatus(coreJs, 200);

      c.assert.assertIncludes(finHtml.text, 'data-operator-role="finance_viewer"');
      c.assert.assertIncludes(adsHtml.text, 'data-operator-role="ads_manager"');
      c.assert.assertMatch(
        finHtml.text,
        /id="tab-btn-1"[^>]*data-required-perm="contents:read"/,
        "M4 tab-btn-1 must carry data-required-perm=\"contents:read\""
      );
      c.assert.assertMatch(
        finHtml.text,
        /id="tab-btn-14"[^>]*data-required-perm="revenue:read\|commerce:read\|subscriptions:read\|invoices:read"/,
        "M4 tab-btn-14 must carry revenue/commerce/subscriptions/invoices permissions"
      );
      c.assert.assertMatch(
        adsHtml.text,
        /id="tab-btn-11"[^>]*data-required-perm="users:read\|roles:manage\|audit:read"/,
        "M4 tab-btn-11 must carry data-required-perm=\"users:read|roles:manage|audit:read\""
      );
      c.assert.assertMatch(
        adsHtml.text,
        /id="tab-btn-2"[^>]*data-required-perm="ads:read"/,
        "M4 tab-btn-2 must carry data-required-perm=\"ads:read\""
      );

      const finData = unwrap(finMe);
      const finPerms = Array.isArray(finData?.permissions) ? finData.permissions : [];
      c.assert.assertEqual(finData?.role, "finance_viewer");
      c.assert.assertEqual(finData?.is_wildcard, false);
      for (const p of ["contents:read", "contents:write", "contents:publish", "contents:manage", "contents:*", "*"]) {
        c.assert.assertNotIncludes(finPerms, p, `finance_viewer must lack ${p} so tab-btn-1 is RBAC-hidden`);
      }
      c.assert.assertOk(
        ["revenue:read", "commerce:read", "subscriptions:read", "invoices:read"].some((p) => finPerms.includes(p)),
        "finance_viewer must hold revenue/commerce/subscriptions/invoices read permission"
      );

      const adsData = unwrap(adsMe);
      const adsPerms = Array.isArray(adsData?.permissions) ? adsData.permissions : [];
      c.assert.assertEqual(adsData?.role, "ads_manager");
      c.assert.assertEqual(adsData?.is_wildcard, false);
      for (const forbiddenPerm of ["users:read", "users:manage", "roles:manage", "audit:read", "audit:manage", "*"]) {
        c.assert.assertNotIncludes(
          adsPerms,
          forbiddenPerm,
          `ads_manager must lack ${forbiddenPerm} so tab-btn-11 is RBAC-hidden`
        );
      }
      c.assert.assertOk(
        ["ads:read", "ads:manage", "ads:*"].some((p) => adsPerms.includes(p)),
        "ads_manager must hold ads:read or ads:manage"
      );

      c.assert.assertIncludes(coreJs.text, "applyRbacVisibility");
      c.assert.assertIncludes(coreJs.text, "data-rbac-hidden");
      c.assert.assertIncludes(coreJs.text, "ROLE_LANDING_TABS");
    });
  });

  // ── REQ-038 Boundaries: /hesabim Role Portal Security & Asset Separation ──
  suite.describe("REQ-038 /hesabim Role Portal Boundaries", () => {
    suite.test("GET /hesabim sets Cache-Control: no-store and X-Robots-Tag: noindex", { req: "REQ-038" }, async (c) => {
      const res = await c.http.createClient().get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.headers.get("cache-control") || "", "no-store");
      c.assert.assertIncludes(res.headers.get("x-robots-tag") || "", "noindex");
    });

    suite.test("GET /auth/sign-in and /profile/settings redirect cleanly to /hesabim", { req: "REQ-038" }, async (c) => {
      const client = c.http.createClient();
      const r1 = await client.get("/auth/sign-in", { redirect: "manual" });
      const r2 = await client.get("/profile/settings", { redirect: "manual" });
      c.assert.assertStatus(r1, [301, 302, 303, 307, 308]);
      c.assert.assertStatus(r2, [301, 302, 303, 307, 308]);
      c.assert.assertIncludes(r1.headers.get("location") || "", "/hesabim");
      c.assert.assertIncludes(r2.headers.get("location") || "", "/hesabim");
    });

    suite.test("guest /hesabim never leaks authenticated staff shortcuts or KVKK delete form", { req: "REQ-038" }, async (c) => {
      const res = await c.http.createClient().get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, "account-delete-form");
      c.assert.assertNotIncludes(res.text, `/${c.adminPath}`);
    });

    suite.test("subscriber_public on /hesabim does NOT receive staff admin panel link", { req: "REQ-038" }, async (c) => {
      const gov = await getCachedRoleSession(c, "subscriber_public");
      const res = await gov.client.get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, `/${c.adminPath}`);
    });

    suite.test("public homepage does NOT bundle account-portal.css/js (M4/M5 asset separation contract)", { req: "REQ-038" }, async (c) => {
      const home = await c.http.createClient().get("/");
      c.assert.assertStatus(home, 200);
      c.assert.assertNotIncludes(home.text, "account-portal.css");
      c.assert.assertNotIncludes(home.text, "account-portal.js");

      // And /css/account-portal.css + /js/account-portal.js exist for /hesabim
      const css = await c.http.createClient().get("/css/account-portal.css");
      c.assert.assertStatus(css, 200, "M5 /css/account-portal.css must be served for /hesabim");
    });
  });

  // ── REQ-041 Boundaries: Admin Users Pagination & RBAC ─────────────────────
  suite.describe("REQ-041 Admin Users Pagination Boundaries", () => {
    suite.test("GET /api/v1/admin/users?page=0&per_page=999 clamps to page=1 and per_page=100", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/users", { query: { page: "0", per_page: "999" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.meta?.page, 1);
      c.assert.assertEqual(res.json?.meta?.per_page, 100);
    });

    suite.test("GET /api/v1/admin/users?page=9999 returns empty data array without error", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/users", { query: { page: "9999", per_page: "25" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(unwrap(res).length, 0);
    });

    suite.test("ads_manager is denied GET /api/v1/admin/users (403)", { req: "REQ-041" }, async (c) => {
      const ads = await getCachedRoleSession(c, "ads_manager");
      const res = await ads.client.get("/api/v1/admin/users");
      c.assert.assertStatus(res, 403);
    });

    suite.test("GET /api/v1/admin/users excludes soft-deleted users from paginated results", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const disp = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      await sa.client.delete(`/api/v1/admin/users/${disp.id}`);
      const res = await sa.client.get("/api/v1/admin/users", { query: { page: "1", per_page: "100" } });
      c.assert.assertStatus(res, 200);
      const ids = unwrap(res).map((u) => u.id);
      c.assert.assertNotIncludes(ids, disp.id, "Deleted user must not appear in /api/v1/admin/users");
    });

    suite.test("Admin #tab-11 pagination UI handles prev/next page navigation state (M4 contract)", { req: "REQ-041" }, async (c) => {
      const js = await c.http.createClient().get("/js/admin/system-rbac.js");
      c.assert.assertStatus(js, 200);
      c.assert.assertMatch(
        js.text,
        /adminUsersPage|usersPage|pagination-(admin-)?users/i,
        "M4 system-rbac.js must implement paginated navigation state for Admin Users tab"
      );
    });
  });
}
