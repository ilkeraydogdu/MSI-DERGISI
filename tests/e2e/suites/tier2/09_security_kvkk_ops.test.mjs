import {
  antiBotFields,
  getCachedRoleSession,
  getCachedSuperAdmin,
  makeContentPayload,
  makeRedirectPayload,
  uid,
  uniqueEmail,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-032 Boundaries: KVKK Export, Consent & Erasure ────────────────────
  suite.describe("REQ-032 KVKK Boundaries", () => {
    suite.test("unauthenticated GET /api/v1/account/export returns 401", { req: "REQ-032" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/account/export");
      c.assert.assertStatus(res, 401);
    });

    suite.test("POST /api/v1/account/delete rejects wrong password with 401 and empty password with 400", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const rBlank = await reg.client.post("/api/v1/account/delete", { json: { password: "" } });
      const rWrong = await reg.client.post("/api/v1/account/delete", { json: { password: "WrongPassword!1" } });
      c.assert.assertStatus(rBlank, 400);
      c.assert.assertStatus(rWrong, 401);
    });

    suite.test("POST /api/v1/account/delete rejects cross-origin Origin header with 403 CSRF_ORIGIN", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const res = await reg.client.post("/api/v1/account/delete", {
        headers: { Origin: "https://evil-attacker.example" },
        json: { password: reg.password },
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("POST /api/v1/account/profile rejects empty or >120-char full_name with 400", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const rEmpty = await reg.client.post("/api/v1/account/profile", { json: { full_name: "   " } });
      const rLong = await reg.client.post("/api/v1/account/profile", { json: { full_name: "A".repeat(121) } });
      c.assert.assertStatus(rEmpty, 400);
      c.assert.assertStatus(rLong, 400);
    });

    suite.test("POST /api/v1/account/profile rejects cross-origin Referer with 403 CSRF_ORIGIN", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const res = await reg.client.post("/api/v1/account/profile", {
        headers: { Referer: "https://evil-site.example/phish" },
        json: { full_name: "Valid Name" },
      });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-033 Boundaries: Full 17-Role RBAC Matrix, WAF, CSRF, Lockout ──────
  suite.describe("REQ-033 Security, WAF, CSRF, Lockout & 17-Role RBAC Matrix", () => {
    suite.test("all 17 canonical roles + unauthenticated enforce exact admin gateway & /api/v1/admin/me matrix", { req: "REQ-033" }, async (c) => {
      // Unauthenticated -> 401
      const anon = await getCachedRoleSession(c, c.auth.UNAUTHENTICATED);
      c.assert.assertStatus(await anon.client.get("/api/v1/admin/me"), 401);

      // 5 non-staff reader roles -> 403 on /api/v1/admin/me and POST Design Studio (or 303 on GET subpage)
      for (const role of c.auth.READER_ROLES) {
        const reader = await getCachedRoleSession(c, role);
        const me = await reader.client.get("/api/v1/admin/me");
        c.assert.assertStatus(me, 403, `Reader role ${role} must be denied /api/v1/admin/me`);
        const studio = await reader.client.get(`/${c.adminPath}/tasarim`, { redirect: "manual" });
        c.assert.assertStatus(studio, [303, 403], `Reader role ${role} must be denied Design Studio`);
        const studioPreview = await reader.client.post(`/${c.adminPath}/tasarim/onizleme`, { json: {} });
        c.assert.assertStatus(studioPreview, 403, `Reader role ${role} must get 403 on Design Studio POST`);
      }

      // All 12 staff roles -> 200 on /api/v1/admin/me and /api/v1/admin/overview
      for (const role of c.auth.STAFF_ROLES) {
        const staff = await getCachedRoleSession(c, role);
        const me = await staff.client.get("/api/v1/admin/me");
        c.assert.assertStatus(me, 200, `Staff role ${role} must access /api/v1/admin/me`);
        c.assert.assertEqual(unwrap(me)?.role, role);
        const ov = await staff.client.get("/api/v1/admin/overview");
        c.assert.assertStatus(ov, 200, `Staff role ${role} must access /api/v1/admin/overview`);
      }
    });

    suite.test("privilege escalation & system-role protection across editable and privileged staff roles", { req: "REQ-033" }, async (c) => {
      // System-locked role cannot be deleted even by super_admin
      const sa = await getCachedSuperAdmin(c);
      for (const sysRole of c.auth.SYSTEM_STAFF_ROLES) {
        const delSys = await sa.client.delete(`/api/v1/admin/user-roles/${sysRole}`);
        c.assert.assertStatus(delSys, 403, `System role ${sysRole} must not be deletable`);
      }

      // Non-super_admin (admin, site_operator, support_agent) cannot create super_admin or grant wildcard '*'
      for (const nonSa of ["admin", "site_operator", "support_agent"]) {
        const actor = await getCachedRoleSession(c, nonSa);
        const escUser = await actor.client.post("/api/v1/admin/users", {
          json: {
            email: uniqueEmail("esc"),
            password: "ValidPassword!2026",
            full_name: "Escalate Test",
            role: "super_admin",
          },
        });
        c.assert.assertStatus(escUser, 403, `${nonSa} must not create super_admin`);
      }

      // Admin cannot create wildcard '*' role or mutate/delete existing is_system=1 roles
      const adminActor = await getCachedRoleSession(c, "admin");
      const escRole = await adminActor.client.post("/api/v1/admin/user-roles", {
        json: {
          role_code: `wild_${uid("w").toLowerCase()}`,
          name_tr: "Wildcard",
          name_en: "Wildcard",
          permissions_json: JSON.stringify(["*"]),
        },
      });
      c.assert.assertStatus(escRole, 403, "admin must not create wildcard '*' role");
      const adminMutateSa = await adminActor.client.put("/api/v1/admin/user-roles/super_admin", {
        json: {
          name_tr: "Tampered SA",
          name_en: "Tampered SA",
          permissions_json: JSON.stringify(["contents:read"]),
        },
      });
      c.assert.assertStatus(adminMutateSa, 403, "admin must not mutate is_system=1 super_admin role");
      const adminDelSys = await adminActor.client.delete("/api/v1/admin/user-roles/admin");
      c.assert.assertStatus(adminDelSys, 403, "admin must not delete is_system=1 admin role");

      // Least-privilege checks across remaining editable staff roles
      const roleDeniedPairs = [
        ["editor_in_chief", "GET", "/api/v1/admin/ads"],
        ["managing_editor", "GET", "/api/v1/admin/invoices"],
        ["content_publisher", "GET", "/api/v1/admin/ads"],
        ["content_editor", "DELETE", "/api/v1/admin/contents/1"],
        ["editor", "DELETE", "/api/v1/admin/contents/1"],
        ["business_dev", "GET", "/api/v1/admin/audit-logs"],
        ["ads_manager", "GET", "/api/v1/admin/users"],
        ["ads_manager", "GET", "/api/v1/admin/user-roles"],
        ["finance_viewer", "GET", "/api/v1/admin/users"],
        ["support_agent", "GET", "/api/v1/admin/ads"],
        ["support_agent", "GET", "/api/v1/admin/contents"],
      ];
      for (const [role, method, path] of roleDeniedPairs) {
        const actor = await getCachedRoleSession(c, role);
        const res = await actor.client.request(method, path);
        c.assert.assertStatus(res, 403, `${role} should be denied ${method} ${path}`);
      }

      const supActor = await getCachedRoleSession(c, "support_agent");
      const supPostContent = await supActor.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug: uniqueSlug("sup-deny"), title_tr: "Support Denied" }),
      });
      c.assert.assertStatus(supPostContent, 403, "support_agent should be denied POST /api/v1/admin/contents");
    });

    suite.test("WAF blocks SQLi, XSS, path traversal, and sensitive file paths with 403", { req: "REQ-033" }, async (c) => {
      const client = c.http.createClient();
      const sqli = await client.get("/api/v1/search?q=%27%20UNION%20SELECT%20password_hash%20FROM%20users%20--");
      const xss = await client.get("/api/v1/search?q=%3Cscript%3Ealert(document.cookie)%3C/script%3E");
      const trav = await client.get("/icerik/%2e%2e%2f%2e%2e%2fetc%2fpasswd");
      const env = await client.get("/.env");
      const git = await client.get("/.git/config");
      const cargo = await client.get("/Cargo.toml");
      c.assert.assertStatus(sqli, 403);
      c.assert.assertStatus(xss, 403);
      c.assert.assertStatus(trav, 403);
      c.assert.assertStatus(env, 403);
      c.assert.assertStatus(git, 403);
      c.assert.assertStatus(cargo, 403);
    });

    suite.test("CSRF enforcement rejects missing token, forged token, and cross-origin on /api/v1/admin/*", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const rMissing = await sa.client.post("/api/v1/admin/cache", {
        csrf: false,
        json: { cache_key: "all" },
      });
      const rBad = await sa.client.post("/api/v1/admin/cache", {
        headers: { "x-csrf-token": "forged.deadbeef" },
        json: { cache_key: "all" },
      });
      const rOrigin = await sa.client.post("/api/v1/admin/cache", {
        headers: { Origin: "https://evil.example" },
        json: { cache_key: "all" },
      });
      c.assert.assertStatus(rMissing, 403);
      c.assert.assertStatus(rBad, 403);
      c.assert.assertStatus(rOrigin, 403);
    });

    suite.test("brute-force lockout triggers 429 ACCOUNT_LOCKED after 5 failed logins on disposable user", { req: "REQ-033" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const attacker = c.http.createClient();
      for (let i = 0; i < 5; i++) {
        const fail = await attacker.post("/api/v1/auth/login", {
          json: { email: reg.email, password: `WrongPass_${i}!`, ...antiBotFields() },
        });
        c.assert.assertStatus(fail, 401);
      }
      const locked = await attacker.post("/api/v1/auth/login", {
        json: { email: reg.email, password: reg.password, ...antiBotFields() },
      });
      c.assert.assertStatus(locked, 429);
      c.assert.assertEqual(locked.json?.error_code, "ACCOUNT_LOCKED");
    });

    suite.test("honeypot website_url triggers silent rejection on login and fake 200 without user creation on register", { req: "REQ-033" }, async (c) => {
      const botEmail = uniqueEmail("honeypot");
      const regRes = await c.auth.registerUser({
        email: botEmail,
        ...antiBotFields(),
        website_url: "https://spam-bot.example",
      });
      c.assert.assertStatus(regRes.res, 200);

      // Verify user was NOT actually created in DB by attempting legitimate login -> 401
      const loginAttempt = await c.http.createClient().post("/api/v1/auth/login", {
        json: { email: botEmail, password: "E2eReader!2026", ...antiBotFields() },
      });
      c.assert.assertStatus(loginAttempt, 401);

      // Login with honeypot filled also rejects with 401
      const validUser = await c.auth.registerUser({ ...antiBotFields() });
      const hpLogin = await c.http.createClient().post("/api/v1/auth/login", {
        json: {
          email: validUser.email,
          password: validUser.password,
          website_url: "https://spam.example",
          _form_ts: antiBotFields()._form_ts,
        },
      });
      c.assert.assertStatus(hpLogin, 401);
    });
  });

  // ── REQ-034 Boundaries: Backup, Open Data & Monitoring RBAC ───────────────
  suite.describe("REQ-034 Backup & Open Data Boundaries", () => {
    suite.test("site_operator is denied POST /api/v1/admin/backup (403, requires backup:manage)", { req: "REQ-034" }, async (c) => {
      const op = await c.auth.loginAsAdmin();
      const res = await op.client.post("/api/v1/admin/backup", { json: {} });
      c.assert.assertStatus(res, 403);
    });

    suite.test("finance_viewer is denied GET /api/v1/admin/export/open-data (403)", { req: "REQ-034" }, async (c) => {
      const fin = await getCachedRoleSession(c, "finance_viewer");
      const res = await fin.client.get("/api/v1/admin/export/open-data");
      c.assert.assertStatus(res, 403);
    });

    suite.test("content_editor is denied POST /api/v1/admin/cache flush (403)", { req: "REQ-034" }, async (c) => {
      const ed = await getCachedRoleSession(c, "content_editor");
      const res = await ed.client.post("/api/v1/admin/cache", { json: { cache_key: "all" } });
      c.assert.assertStatus(res, 403);
    });

    suite.test("support_agent is denied GET /api/v1/admin/monitoring/bundle (403)", { req: "REQ-034" }, async (c) => {
      const sup = await getCachedRoleSession(c, "support_agent");
      const res = await sup.client.get("/api/v1/admin/monitoring/bundle");
      c.assert.assertStatus(res, 403);
    });

    suite.test("unauthenticated GET /api/v1/admin/export/open-data returns 401", { req: "REQ-034" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/admin/export/open-data");
      c.assert.assertStatus(res, 401);
    });
  });

  // ── REQ-035 Boundaries: 301 Redirects & Stealth 404 ───────────────────────
  suite.describe("REQ-035 Redirect & SEO Boundaries", () => {
    suite.test("stealth 404 on /admin and /wp-admin hides admin gateway path", { req: "REQ-035" }, async (c) => {
      const client = c.http.createClient();
      const r1 = await client.get("/admin", { redirect: "manual" });
      const r2 = await client.get("/wp-admin", { redirect: "manual" });
      c.assert.assertStatus(r1, 404);
      c.assert.assertStatus(r2, 404);
    });

    suite.test("DELETE /api/v1/admin/redirects/999999 returns 404 for nonexistent rule", { req: "REQ-035" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.delete("/api/v1/admin/redirects/999999");
      c.assert.assertStatus(res, 404);
    });

    suite.test("ads_manager is denied POST /api/v1/admin/redirects (403)", { req: "REQ-035" }, async (c) => {
      const ads = await getCachedRoleSession(c, "ads_manager");
      const res = await ads.client.post("/api/v1/admin/redirects", {
        json: makeRedirectPayload({ old_path: "/x", new_path: "/y" }),
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("POST /api/v1/admin/redirects rejects empty old_path or new_path with 400", { req: "REQ-035" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/redirects", {
        json: makeRedirectPayload({ old_path: "", new_path: "" }),
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("unauthenticated POST /sitemap.xml returns 405 Method Not Allowed", { req: "REQ-035" }, async (c) => {
      const res = await c.http.createClient().post("/sitemap.xml", { json: {} });
      c.assert.assertStatus(res, 405);
    });
  });

  // ── REQ-036 Boundaries: Future-Phase Hubs ─────────────────────────────────
  suite.describe("REQ-036 Future-Phase Hub Boundaries", () => {
    suite.test("GET /yazar/nonexistent-author-xyz renders 404 page cleanly without 500", { req: "REQ-036" }, async (c) => {
      const res = await c.http.createClient().get(`/yazar/missing-author-${uid("au").toLowerCase()}`);
      c.assert.assertStatus(res, 404);
      c.assert.assertIncludes(res.text, "not-found-card");
    });

    suite.test("GET /en/savunma-sanayii-tarihi and /en/defense-industry-history resolve cleanly", { req: "REQ-036" }, async (c) => {
      const client = c.http.createClient();
      const r1 = await client.get("/en/defense-industry-history");
      const r2 = await client.get("/en/defence-industry-history");
      c.assert.assertStatus(r1, 200);
      c.assert.assertStatus(r2, 200);
    });

    suite.test("GET /yazarlar?lang=en renders English Authors directory", { req: "REQ-036" }, async (c) => {
      const res = await c.http.createClient().get("/yazarlar", { query: { lang: "en" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
    });

    suite.test("GET /ogrenci-kulupleri?lang=en renders English Student Clubs page", { req: "REQ-036" }, async (c) => {
      const res = await c.http.createClient().get("/ogrenci-kulupleri", { query: { lang: "en" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
    });

    suite.test("GET /etkinlikler?page=99 handles out-of-range pagination cleanly without 500", { req: "REQ-036" }, async (c) => {
      const res = await c.http.createClient().get("/etkinlikler", { query: { page: "99" } });
      c.assert.assertStatus(res, 200);
    });
  });
}
