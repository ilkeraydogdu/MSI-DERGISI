import {
  getCachedSuperAdmin,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-037: Admin Panel & Design Studio UI/UX Overhaul ───────────────────
  suite.describe("REQ-037 Admin Panel & Design Studio UI Contracts", () => {
    suite.test("unauthenticated GET /{adminPath} and /podmin serve admin login gateway", { req: "REQ-037" }, async (c) => {
      const client = c.http.createClient();
      const r1 = await client.get(`/${c.adminPath}`);
      const r2 = await client.get("/podmin");
      c.assert.assertStatus(r1, 200);
      c.assert.assertStatus(r2, 200);
      c.assert.assertIncludes(r1.text, 'id="login-form"');
    });

    suite.test("authenticated staff GET /{adminPath} renders Admin Command Center and CSS/JS assets load 200", { req: "REQ-037" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "admin-sidebar");
      c.assert.assertIncludes(res.text, "data-operator-role=\"super_admin\"");

      for (const asset of [
        "/css/admin/tokens.css",
        "/css/admin/layout.css",
        "/css/admin/components.css",
        "/js/admin/core.js",
        "/js/admin/editorial.js",
      ]) {
        const aRes = await sa.client.get(asset);
        c.assert.assertStatus(aRes, 200, `Admin asset ${asset} must return 200`);
      }
    });

    suite.test("GET /{adminPath}/tasarim serves Design Studio for authorized staff", { req: "REQ-037" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get(`/${c.adminPath}/tasarim`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "<html");
    });

    suite.test("GET /api/v1/admin/overview returns live operational KPIs", { req: "REQ-037" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/overview");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(typeof res.json?.total_contents === "number");
      c.assert.assertOk(typeof res.json?.registered_users === "number");
    });

    suite.test("M4 Admin UI preserves all 18 tabs in 5 domain nav groups with data-required-perm and wires client RBAC visibility", { req: "REQ-037" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const editor = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const [saHtml, editorHtml, coreJs, editorMe] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        editor.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/core.js"),
        editor.client.get("/api/v1/admin/me"),
      ]);
      c.assert.assertStatus(saHtml, 200);
      c.assert.assertStatus(editorHtml, 200);
      c.assert.assertStatus(coreJs, 200);
      c.assert.assertStatus(editorMe, 200);
      for (let i = 1; i <= 18; i++) {
        c.assert.assertIncludes(
          saHtml.text,
          `id="tab-btn-${i}"`,
          `M4 Admin UI must preserve tab-btn-${i} (tab-1..tab-18) for super_admin`
        );
      }
      for (const groupKey of [
        "admin.nav.group_editorial",
        "admin.nav.group_commercial",
        "admin.nav.group_analytics",
        "admin.nav.group_comms",
        "admin.nav.group_system",
      ]) {
        c.assert.assertIncludes(
          saHtml.text,
          `data-group-name="${groupKey}"`,
          `M4 Admin sidebar must organize tabs into 5 domain navigation groups (missing ${groupKey})`
        );
      }
      c.assert.assertIncludes(
        saHtml.text,
        "data-required-perm=",
        "M4 Admin sidebar buttons must carry data-required-perm attributes for role-scoped visibility"
      );
      c.assert.assertIncludes(
        editorHtml.text,
        'data-operator-role="content_editor"',
        "M4 Admin SSR must bind data-operator-role=\"content_editor\" on <body>"
      );
      c.assert.assertMatch(
        editorHtml.text,
        /id="tab-btn-1"[^>]*data-required-perm="contents:read"/,
        "M4 tab-btn-1 must carry data-required-perm=\"contents:read\""
      );
      c.assert.assertMatch(
        editorHtml.text,
        /id="tab-btn-11"[^>]*data-required-perm="users:read\|roles:manage\|audit:read"/,
        "M4 tab-btn-11 must carry data-required-perm=\"users:read|roles:manage|audit:read\""
      );
      const meData = unwrap(editorMe);
      const perms = Array.isArray(meData?.permissions) ? meData.permissions : [];
      c.assert.assertEqual(meData?.role, "content_editor");
      c.assert.assertEqual(meData?.is_wildcard, false);
      c.assert.assertOk(
        perms.includes("contents:read") || perms.includes("contents:*"),
        "content_editor must possess contents:read for tab-1"
      );
      for (const forbiddenPerm of ["users:read", "users:manage", "roles:manage", "audit:read", "audit:manage", "*"]) {
        c.assert.assertNotIncludes(
          perms,
          forbiddenPerm,
          `content_editor must NOT hold ${forbiddenPerm} so applyRbacVisibility hides tab-11`
        );
      }
      c.assert.assertIncludes(coreJs.text, "applyRbacVisibility", "M4 core.js must implement applyRbacVisibility()");
      c.assert.assertIncludes(coreJs.text, "data-rbac-hidden", "M4 core.js must mark restricted controls with data-rbac-hidden");
      c.assert.assertIncludes(coreJs.text, "ROLE_LANDING_TABS", "M4 core.js must define ROLE_LANDING_TABS");
    });
  });

  // ── REQ-038: Authenticated /hesabim Role Portal UI/UX Overhaul ────────────
  suite.describe("REQ-038 Authenticated /hesabim Role Portal UI Contracts", () => {
    suite.test("GET /hesabim serves dedicated account-portal.css & account-portal.js assets (M5 contract)", { req: "REQ-038" }, async (c) => {
      const reg = await c.auth.registerUser();
      const [page, css, js] = await Promise.all([
        reg.client.get("/hesabim"),
        reg.client.get("/css/account-portal.css"),
        reg.client.get("/js/account-portal.js"),
      ]);
      c.assert.assertStatus(page, 200);
      c.assert.assertStatus(css, 200, "M5 must serve /css/account-portal.css");
      c.assert.assertStatus(js, 200, "M5 must serve /js/account-portal.js");
      c.assert.assertIncludes(page.text, "account-portal.css", "M5 /hesabim must link account-portal.css");
      c.assert.assertIncludes(page.text, "account-portal.js", "M5 /hesabim must load account-portal.js");
    });

    suite.test("/hesabim renders role-aware sections (profile, 14 SSB interests, TOTP 2FA, KVKK)", { req: "REQ-038" }, async (c) => {
      const reg = await c.auth.registerUser();
      const res = await reg.client.get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "account-kvkk-card");
      c.assert.assertIncludes(res.text, "personalized-feed-band");
      c.assert.assertIncludes(res.text, "hava-araclari");
    });

    suite.test("/hesabim displays corporate/paid subscription badge & invoices for subscriber_digital", { req: "REQ-038" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const res = await sub.client.get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, sub.email);
      c.assert.assertIncludes(res.text, "/e-dergi");
    });

    suite.test("/hesabim renders staff admin shortcut link for authenticated staff roles", { req: "REQ-038" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, `/${c.adminPath}`);
    });

    suite.test("/hesabim?lang=en renders English reader portal cleanly", { req: "REQ-038" }, async (c) => {
      const reg = await c.auth.registerUser();
      const res = await reg.client.get("/hesabim", { query: { lang: "en" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
    });
  });

  // ── REQ-041: Admin Users (tab-11) pagination controls ─────────────────────
  suite.describe("REQ-041 Admin Users Pagination API & UI", () => {
    suite.test("GET /api/v1/admin/users?page=1&per_page=5 returns paginated slice and meta", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/users", { query: { page: "1", per_page: "5" } });
      c.assert.assertStatus(res, 200);
      const rows = unwrap(res);
      c.assert.assertOk(Array.isArray(rows) && rows.length <= 5);
      c.assert.assertEqual(res.json?.meta?.page, 1);
      c.assert.assertEqual(res.json?.meta?.per_page, 5);
      c.assert.assertOk(res.json?.meta?.total_count >= rows.length);
    });

    suite.test("GET /api/v1/admin/users page 2 returns non-overlapping users when total > per_page", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      // Ensure at least 4 users exist
      for (let i = 0; i < 3; i++) {
        await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      }
      const p1 = await sa.client.get("/api/v1/admin/users", { query: { page: "1", per_page: "2" } });
      const p2 = await sa.client.get("/api/v1/admin/users", { query: { page: "2", per_page: "2" } });
      c.assert.assertStatus(p1, 200);
      c.assert.assertStatus(p2, 200);
      const ids1 = unwrap(p1).map((u) => u.id);
      const ids2 = unwrap(p2).map((u) => u.id);
      c.assert.assertOk(ids2.length >= 1, "Expected page 2 users");
      for (const id of ids2) {
        c.assert.assertNotIncludes(ids1, id, "Page 1 and Page 2 user IDs must not overlap");
      }
    });

    suite.test("GET /api/v1/admin/user-roles returns paginated roles metadata", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/user-roles", { query: { page: "1", per_page: "10" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(res.json?.meta?.total_count >= 17, "Expected >=17 canonical seeded roles");
    });

    suite.test("Admin #tab-11 HTML renders dedicated users pagination container (M4 contract)", { req: "REQ-041" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(
        res.text,
        /id="pagination-(admin-)?users"/i,
        "M4 #tab-11 must include pagination container for #tbody-admin-users"
      );
    });

    suite.test("Admin system-rbac.js fetches paginated /api/v1/admin/users with page param (M4 contract)", { req: "REQ-041" }, async (c) => {
      const res = await c.http.createClient().get("/js/admin/system-rbac.js");
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(
        res.text,
        '/api/v1/admin/users?per_page=100"',
        "M4 system-rbac.js must replace hardcoded per_page=100 with paginated page/per_page navigation"
      );
      c.assert.assertMatch(
        res.text,
        /pagination-(admin-)?users|loadAdminUsers\s*\(\s*page/i,
        "M4 system-rbac.js must wire users pagination UI controls"
      );
    });
  });
}
