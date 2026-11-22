import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.test(
    "Isolated server /api/v1/health returns 200 ok on ephemeral port != 8090",
    { req: "E2E-INFRA", tier: "smoke" },
    async (c) => {
      assert.assertNotEqual(c.port, 8090, "E2E port must not collide with production 8090");
      const client = c.http.createClient();
      const res = await client.get("/api/v1/health");
      assert.assertStatus(res, 200, "Health endpoint should return 200");
      assert.assertJson(res, { status: "ok" }, "Health status must be ok");
      assert.assertOk(
        res.json?.counts?.contents > 0,
        `Expected seeded contents in isolated DB, got ${JSON.stringify(res.json?.counts)}`
      );
      assert.assertEqual(
        res.json?.public_url,
        c.baseUrl,
        "Server public_url should match isolated ephemeral baseUrl"
      );
    }
  );

  suite.test(
    "DB & filesystem isolation leaves production storage untouched",
    { req: "E2E-INFRA", tier: "smoke" },
    async (c) => {
      assert.assertOk(
        c.tempDir.startsWith("/tmp/msi_e2e_"),
        `Expected tempDir under /tmp/msi_e2e_*, got ${c.tempDir}`
      );
      assert.assertOk(fs.existsSync(c.dbPath), "Isolated SQLite DB should exist");
      assert.assertOk(fs.existsSync(c.edergiDir), "Isolated edergiDir should exist");
      assert.assertOk(fs.existsSync(c.uploadsDir), "Isolated uploadsDir should exist");
      assert.assertOk(
        fs.lstatSync(path.join(c.tempDir, "public", "css")).isSymbolicLink(),
        "public/css should be symlinked for zero-copy static serving"
      );

      const prodDbPath = path.join(c.repoRoot, "storage", "db", "msi_platform.db");
      const prodDbStatBefore = fs.existsSync(prodDbPath) ? fs.statSync(prodDbPath) : null;

      // Mutate isolated DB via super_admin user creation
      const created = await c.auth.createUserWithRole("registered");
      assert.assertOk(created.id > 0, "Created isolated test user should have positive id");

      if (prodDbStatBefore) {
        const prodDbStatAfter = fs.statSync(prodDbPath);
        assert.assertEqual(
          prodDbStatAfter.mtimeMs,
          prodDbStatBefore.mtimeMs,
          "Production DB mtime must remain untouched"
        );
        assert.assertEqual(
          prodDbStatAfter.size,
          prodDbStatBefore.size,
          "Production DB size must remain untouched"
        );
      } else {
        assert.assertOk(
          !fs.existsSync(prodDbPath),
          "Production DB should not be created if previously absent"
        );
      }
    }
  );

  suite.test(
    "Seeded site_operator & super_admin logins, /api/v1/admin/me, and CSRF enforcement",
    { req: ["E2E-INFRA", "REQ-033"], tier: "smoke" },
    async (c) => {
      const op = await c.auth.loginAsAdmin();
      assert.assertEqual(op.role, "site_operator", "Bootstrapped operator must have role site_operator");
      const opMe = await op.client.get("/api/v1/admin/me");
      assert.assertStatus(opMe, 200, "site_operator /api/v1/admin/me should succeed");
      assert.assertEqual(opMe.json?.data?.role, "site_operator");
      assert.assertEqual(opMe.json?.data?.is_wildcard, false);

      const sa = await c.auth.loginAsSuperAdmin();
      assert.assertEqual(sa.role, "super_admin", "Bootstrapped super_admin must have role super_admin");
      const saMe = await sa.client.get("/api/v1/admin/me");
      assert.assertStatus(saMe, 200, "super_admin /api/v1/admin/me should succeed");
      assert.assertEqual(saMe.json?.data?.role, "super_admin");
      assert.assertEqual(saMe.json?.data?.is_wildcard, true);

      // CSRF enforcement check: omitting X-CSRF-Token on mutating admin API must fail with 403
      const noCsrfRes = await sa.client.post("/api/v1/admin/users", {
        csrf: false,
        json: {
          email: `e2e.csrf.${crypto.randomBytes(3).toString("hex")}@example.com`,
          password: "E2ePassword!2026",
          full_name: "CSRF Check User",
          role: "registered",
        },
      });
      assert.assertStatus(noCsrfRes, 403, "Mutating admin request without X-CSRF-Token must return 403");
    }
  );

  suite.test(
    "User registration (.gov.tr auto-verify), role provisioning & TOTP 2FA cycle",
    { req: ["E2E-INFRA", "REQ-016", "REQ-017"], tier: "smoke" },
    async (c) => {
      const suffix = crypto.randomBytes(4).toString("hex");
      const govReg = await c.auth.registerUser({
        email: `e2e.gov.${suffix}@ssb.gov.tr`,
        full_name: "E2E Kamu Görevlisi",
        organization_category: "Kamu / TSK / SSB",
      });
      assert.assertEqual(govReg.govAutoSubscribed, true, ".gov.tr email should auto-subscribe");
      assert.assertEqual(
        govReg.user?.role,
        "subscriber_public",
        ".gov.tr registration should assign subscriber_public"
      );
      assert.assertEqual(govReg.user?.is_gov_verified, true, ".gov.tr should set is_gov_verified=true");

      // Provision representative staff, reader, and unauthenticated clients
      const sa = await c.auth.loginAsSuperAdmin();
      for (const staffRole of ["editor_in_chief", "content_editor", "ads_manager"]) {
        const actor = await c.auth.createUserWithRole(staffRole, { superAdminClient: sa.client });
        const me = await actor.client.get("/api/v1/admin/me");
        assert.assertStatus(me, 200, `Staff role ${staffRole} must access /api/v1/admin/me`);
        assert.assertEqual(me.json?.data?.role, staffRole);
      }

      for (const readerRole of ["subscriber_digital", "registered"]) {
        const reader = await c.auth.createUserWithRole(readerRole, { superAdminClient: sa.client });
        const me = await reader.client.get("/api/v1/admin/me");
        assert.assertStatus(me, 403, `Reader role ${readerRole} must be denied /api/v1/admin/me`);
      }

      const anon = await c.auth.createUserWithRole(c.auth.UNAUTHENTICATED);
      const anonMe = await anon.client.get("/api/v1/admin/me");
      assert.assertStatus(anonMe, 401, "Unauthenticated client must get 401 on /api/v1/admin/me");

      // Full TOTP 2FA setup -> challenge -> verify -> disable cycle
      const totpUser = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const { secret, backupCodes } = await c.auth.enableTotp(totpUser.client);
      assert.assertOk(secret.length >= 16, "TOTP secret should be valid Base32");
      assert.assertOk(backupCodes.length >= 1, "TOTP enrollment should return backup codes");

      const logoutRes = await totpUser.client.post("/api/v1/auth/logout");
      assert.assertStatus(logoutRes, 200, "Logout should succeed");

      const freshClient = c.http.createClient();
      const challenge = await c.auth.login(freshClient, {
        email: totpUser.email,
        password: totpUser.password,
      });
      assert.assertEqual(challenge.status, "totp_required", "2FA-enrolled user login must require TOTP");
      assert.assertOk(challenge.tempToken, "Expected tempToken on TOTP challenge");

      const validCode = c.totp.generateTotp(secret);
      const badCode = validCode === "000000" ? "111111" : "000000";
      const badLogin = await freshClient.post("/api/v1/auth/totp/login", {
        json: { temp_token: challenge.tempToken, code: badCode },
      });
      assert.assertStatus(badLogin, 401, "Invalid TOTP code must be rejected with 401");

      const verified = await c.auth.verifyTotpLogin(freshClient, challenge.tempToken, secret);
      assert.assertEqual(verified.status, "authenticated", "Valid TOTP code must authenticate");

      await c.auth.disableTotp(freshClient, { password: totpUser.password });

      const postDisableClient = c.http.createClient();
      const directLogin = await c.auth.login(postDisableClient, {
        email: totpUser.email,
        password: totpUser.password,
      });
      assert.assertEqual(
        directLogin.status,
        "authenticated",
        "After disabling TOTP, login should authenticate directly"
      );
    }
  );

  suite.test(
    "Playwright Chromium renders / and admin gateway/panel on desktop & mobile without errors or overflow",
    { req: ["E2E-INFRA", "REQ-004", "REQ-037"], tier: "smoke" },
    async (c) => {
      const sa = await c.auth.loginAsSuperAdmin();
      const targets = [
        { label: "public home (desktop)", path: "/", viewport: c.browser.VIEWPORT_DESKTOP },
        { label: "public home (mobile)", path: "/", viewport: c.browser.VIEWPORT_MOBILE },
        {
          label: "admin login gateway (desktop)",
          path: `/${c.adminPath}`,
          viewport: c.browser.VIEWPORT_DESKTOP,
        },
        {
          label: "admin login gateway (mobile)",
          path: `/${c.adminPath}`,
          viewport: c.browser.VIEWPORT_MOBILE,
        },
        {
          label: "authenticated admin panel (desktop)",
          path: `/${c.adminPath}`,
          viewport: c.browser.VIEWPORT_DESKTOP,
          client: sa.client,
          expectAdminShell: true,
        },
        {
          label: "authenticated admin panel (mobile)",
          path: `/${c.adminPath}`,
          viewport: c.browser.VIEWPORT_MOBILE,
          client: sa.client,
          expectAdminShell: true,
        },
      ];

      for (const t of targets) {
        const pageRes = await c.browser.openPage(t.path, {
          viewport: t.viewport,
          client: t.client,
        });
        try {
          assert.assertEqual(pageRes.status, 200, `${t.label} must return HTTP 200`);
          assert.assertNoBrowserErrors(pageRes, `${t.label} had browser/5xx errors`);
          assert.assertNoOverflow(pageRes, `${t.label} had horizontal overflow`);
          if (t.expectAdminShell) {
            const html = await pageRes.page.content();
            assert.assertOk(
              !html.includes('id="admin-login-form"') || html.includes("msi-admin") || html.includes("tab-"),
              `${t.label} should render authenticated admin panel rather than guest login only`
            );
          }
        } finally {
          await pageRes.close();
        }
      }
    }
  );

  suite.test(
    "Playwright multi-domain Host routing renders distinct msidergisi.com vs savunmahaber.com pages cleanly",
    { req: ["E2E-INFRA", "REQ-001"], tier: "smoke" },
    async (c) => {
      const msiPage = await c.browser.openPage("/", { host: "msidergisi.com" });
      let msiTitle = "";
      try {
        assert.assertEqual(msiPage.status, 200, "msidergisi.com / must return HTTP 200");
        assert.assertNoBrowserErrors(msiPage, "msidergisi.com / had browser/5xx errors");
        assert.assertNoOverflow(msiPage, "msidergisi.com / had horizontal overflow");
        msiTitle = await msiPage.page.title();
      } finally {
        await msiPage.close();
      }

      const shPage = await c.browser.openPage("/", { host: "savunmahaber.com" });
      let shTitle = "";
      try {
        assert.assertEqual(shPage.status, 200, "savunmahaber.com / must return HTTP 200");
        assert.assertNoBrowserErrors(shPage, "savunmahaber.com / had browser/5xx errors");
        assert.assertNoOverflow(shPage, "savunmahaber.com / had horizontal overflow");
        shTitle = await shPage.page.title();
      } finally {
        await shPage.close();
      }

      assert.assertNotEqual(
        shTitle,
        msiTitle,
        "savunmahaber.com and msidergisi.com should render distinct domain titles"
      );

      const reader = await c.auth.createUserWithRole("subscriber_digital");
      const portalPage = await c.browser.openPage("/hesabim", {
        host: "savunmahaber.com",
        client: reader.client,
      });
      try {
        assert.assertEqual(portalPage.status, 200, "Authenticated /hesabim on savunmahaber.com must return 200");
        assert.assertNoBrowserErrors(portalPage, "Authenticated /hesabim on savunmahaber.com had browser/5xx errors");
        const portalHtml = await portalPage.page.content();
        assert.assertIncludes(
          portalHtml,
          reader.email,
          "Authenticated /hesabim on savunmahaber.com should render the logged-in user's account"
        );
      } finally {
        await portalPage.close();
      }
    }
  );
}
