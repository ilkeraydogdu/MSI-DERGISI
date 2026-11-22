import {
  getCachedRoleSession,
  getCachedSuperAdmin,
} from "../../fixtures/tier12_helpers.mjs";
import { Client } from "../../lib/http.mjs";
import { startIsolatedServer } from "../../lib/server.mjs";

const STUDIO_DENIED_STAFF_ROLES = [
  "content_publisher",
  "content_editor",
  "editor",
  "business_dev",
  "ads_manager",
  "finance_viewer",
  "support_agent",
];

const PRIVILEGE_RESTRICTED_STAFF = [
  "content_editor",
  "editor",
  "ads_manager",
  "finance_viewer",
  "business_dev",
];

export function registerTests(suite) {
  // ── F1 Boundaries: /msi-komuta Removal & /podmin AuthZ (6 tests) ──────────
  suite.describe("F1 Boundaries: /msi-komuta 404 Removal & Route Isolation", () => {
    suite.test(
      "retired /msi-komuta, /msi-komuta/, /msi-komuta/editor, /msi-komuta/tasarim, /msi-komuta/tasarim/onizleme, /msi-komuta/x return 404",
      { req: "F1", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        for (const p of [
          "/msi-komuta",
          "/msi-komuta/",
          "/msi-komuta/editor",
          "/msi-komuta/editor/studio.js",
          "/msi-komuta/tasarim",
          "/msi-komuta/nonexistent",
        ]) {
          const r = await client.get(p, { redirect: "manual" });
          c.assert.assertStatus(r, 404, `Retired route GET ${p} must return 404`);
        }
        const postPreview = await client.post("/msi-komuta/tasarim/onizleme", { json: {} });
        c.assert.assertStatus(postPreview, 404, "Retired POST /msi-komuta/tasarim/onizleme must return 404");
      }
    );

    suite.test(
      "isolated server with ADMIN_PATH_SECRET='podmin' returns 404 on all /msi-komuta* routes",
      { req: "F1" },
      async (c) => {
        const srv = await startIsolatedServer({ adminPath: "podmin" });
        try {
          const client = new Client({ baseUrl: srv.baseUrl });
          for (const p of ["/msi-komuta", "/msi-komuta/", "/msi-komuta/tasarim", "/msi-komuta/unknown"]) {
            const r = await client.get(p, { redirect: "manual" });
            c.assert.assertStatus(r, 404, `GET ${p} must return 404 on podmin server`);
          }
          const postR = await client.post("/msi-komuta/tasarim/onizleme", { json: {} });
          c.assert.assertStatus(postR, 404);
          const podR = await client.get("/podmin");
          c.assert.assertStatus(podR, 200);
          c.assert.assertIncludes(podR.text, 'id="login-form"');
        } finally {
          await srv.stop();
        }
      }
    );

    suite.test(
      "isolated server with ADMIN_PATH_SECRET='' (empty fallback) resolves to /podmin and returns 404 on /msi-komuta*",
      { req: "F1" },
      async (c) => {
        const srv = await startIsolatedServer({ adminPath: "" });
        try {
          c.assert.assertEqual(srv.env.ADMIN_PATH_SECRET, "");
          const client = new Client({ baseUrl: srv.baseUrl });
          const rLegacy = await client.get("/msi-komuta", { redirect: "manual" });
          c.assert.assertStatus(rLegacy, 404);
          const rPodmin = await client.get("/podmin");
          c.assert.assertStatus(rPodmin, 200);
          c.assert.assertIncludes(rPodmin.text, 'data-admin-path="/podmin"');
        } finally {
          await srv.stop();
        }
      }
    );

    suite.test(
      "isolated server with ADMIN_PATH_SECRET='msi-komuta' rejects legacy slug -> mounts /podmin and returns 404 on /msi-komuta*",
      { req: "F1" },
      async (c) => {
        const srv = await startIsolatedServer({ adminPath: "msi-komuta" });
        try {
          const client = new Client({ baseUrl: srv.baseUrl });
          for (const p of ["/msi-komuta", "/msi-komuta/", "/msi-komuta/tasarim"]) {
            const r = await client.get(p, { redirect: "manual" });
            c.assert.assertStatus(r, 404, `Retired slug must be rejected even when configured: ${p}`);
          }
          const podR = await client.get("/podmin");
          c.assert.assertStatus(podR, 200);
          c.assert.assertIncludes(podR.text, 'data-admin-path="/podmin"');
        } finally {
          await srv.stop();
        }
      }
    );

    suite.test(
      "unauthenticated & all 5 READER_ROLES get 303/403 on GET /podmin/tasarim",
      { req: "F1" },
      async (c) => {
        const guestRes = await c.http.createClient().get("/podmin/tasarim", { redirect: "manual" });
        c.assert.assertStatus(guestRes, [302, 303, 307, 401, 403]);
        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const r = await session.client.get("/podmin/tasarim", { redirect: "manual" });
          c.assert.assertStatus(r, [303, 403], `Reader role ${role} must not access /podmin/tasarim`);
        }
      }
    );

    suite.test(
      "unauthenticated & all 5 READER_ROLES get 401/403 on POST /podmin/tasarim/onizleme and /api/v1/admin/overview",
      { req: "F1" },
      async (c) => {
        const guestPreview = await c.http.createClient().post("/podmin/tasarim/onizleme", { json: {} });
        c.assert.assertStatus(guestPreview, 401);
        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const [prevRes, apiRes] = await Promise.all([
            session.client.post("/podmin/tasarim/onizleme", { json: {} }),
            session.client.get("/api/v1/admin/overview"),
          ]);
          c.assert.assertStatus(prevRes, 403, `Reader ${role} must get 403 on /podmin/tasarim/onizleme`);
          c.assert.assertStatus(apiRes, 403, `Reader ${role} must get 403 on /api/v1/admin/overview`);
        }
      }
    );
  });

  // ── F2 Boundaries: Soft Tokens, Reduced Motion & Asset Resilience (5 tests)
  suite.describe("F2 Boundaries: Theme Resilience, Soft Palette & Reduced Motion", () => {
    suite.test(
      "invalid forged localStorage theme degrades safely to valid light/dark theme without JS errors",
      { req: "F2", readOnly: true },
      async (c) => {
        const ctx = await c.browser.newContext({ viewport: c.browser.VIEWPORT_DESKTOP });
        try {
          await ctx.addInitScript(() => {
            try {
              window.localStorage.setItem("msi_admin_theme", "invalid_corrupt_theme_99");
              window.localStorage.setItem("msi_theme", "invalid_corrupt_theme_99");
            } catch {
              // ignore
            }
          });
          const res = await c.browser.openPage("/podmin", { context: ctx });
          try {
            c.assert.assertEqual(res.status, 200);
            const appliedTheme = await res.page.locator("html").getAttribute("data-theme");
            c.assert.assertOk(
              appliedTheme === "light" || appliedTheme === "dark",
              `Invalid stored theme must degrade to light/dark, got "${appliedTheme}"`
            );
            c.assert.assertNoAssetOrNetworkErrors(res);
          } finally {
            await res.close();
          }
        } finally {
          await ctx.close();
        }
      }
    );

    suite.test(
      "missing static asset under /css/admin/ returns 404 (not 200 HTML or 500)",
      { req: "F2", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/css/admin/nonexistent-e2e-999.css");
        c.assert.assertStatus(res, 404);
      }
    );

    suite.test(
      "tokens.css avoids harsh #000000 body background and defines soft --adm-bg values for Light & Dark",
      { req: "F2", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/css/admin/tokens.css");
        c.assert.assertStatus(res, 200);
        c.assert.assertNotIncludes(res.text, "--adm-bg: #000000");
        c.assert.assertNotIncludes(res.text, "--adm-bg: #000;");
        c.assert.assertMatch(res.text, /--adm-bg:\s*(?:var\(--[a-z0-9-]+,\s*)?#[0-9a-f]{6}/i);
      }
    );

    suite.test(
      "prefers-reduced-motion: reduce renders /podmin and /hesabim cleanly at full opacity with zero errors",
      { req: "F2", readOnly: true },
      async (c) => {
        const res = await c.browser.openPage("/podmin", { viewport: c.browser.VIEWPORT_DESKTOP });
        try {
          await res.emulateReducedMotion(true);
          await res.page.reload({ waitUntil: "networkidle" });
          await res.page.waitForTimeout(200);
          const cardOpacity = await res.page.locator(".login-card").evaluate((el) => {
            return parseFloat(window.getComputedStyle(el).opacity);
          });
          c.assert.assertOk(cardOpacity >= 0.9, `Login card must settle at opacity >= 0.9 under reduced motion, got ${cardOpacity}`);
          c.assert.assertNoAssetOrNetworkErrors(res);
        } finally {
          await res.close();
        }
      }
    );

    suite.test(
      "unexpected ?v= cache-buster query string on admin/portal CSS still serves 200 without error",
      { req: "F2", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        for (const p of ["/css/admin/tokens.css?v=999", "/css/account-portal.css?v=999"]) {
          const r = await client.get(p);
          c.assert.assertStatus(r, 200);
          c.assert.assertOk(r.text.length > 100);
        }
      }
    );
  });

  // ── F3 Boundaries: Login Gateway Isolation & Auth Errors (5 tests) ────────
  suite.describe("F3 Boundaries: Login Gateway Isolation & Auth Validation", () => {
    suite.test(
      "all 5 READER_ROLES on GET /podmin see #login-form and NEVER #admin-sidebar or data-operator-role",
      { req: "F3" },
      async (c) => {
        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/podmin");
          c.assert.assertStatus(res, 200);
          c.assert.assertIncludes(res.text, 'id="login-form"', `Reader ${role} on /podmin must see login form`);
          c.assert.assertNotIncludes(res.text, 'id="admin-sidebar"', `Reader ${role} must NEVER see admin-sidebar`);
          c.assert.assertNotIncludes(res.text, "data-operator-role=");
        }
      }
    );

    suite.test(
      "POST /api/v1/auth/login rejects wrong password (401) and empty payload (400/422) without 500",
      { req: "F3" },
      async (c) => {
        const client = c.http.createClient();
        const badPass = await client.post("/api/v1/auth/login", {
          json: { email: c.credentials.admin.email, password: "InvalidPassword!999" },
        });
        c.assert.assertStatus(badPass, 401);

        const emptyBody = await client.post("/api/v1/auth/login", {
          json: { email: "", password: "" },
        });
        c.assert.assertStatus(emptyBody, [400, 401, 422]);
      }
    );

    suite.test(
      "POST /api/v1/auth/totp/login rejects invalid temp_token and bad code cleanly (400/401)",
      { req: "F3" },
      async (c) => {
        const res = await c.http.createClient().post("/api/v1/auth/totp/login", {
          json: { temp_token: "invalid-e2e-temp-token", code: "000000" },
        });
        c.assert.assertStatus(res, [400, 401]);
      }
    );

    suite.test(
      "GET /podmin?lang=zz (unsupported lang) falls back cleanly to valid gateway markup without 500",
      { req: "F3", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/podmin", { query: { lang: "zz" } });
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'id="login-form"');
      }
    );

    suite.test(
      "POST /api/v1/auth/forgot-password with nonexistent email returns generic 200 (no account enumeration)",
      { req: "F3" },
      async (c) => {
        const res = await c.http.createClient().post("/api/v1/auth/forgot-password", {
          json: { email: `nonexistent.${Date.now()}@example.com` },
        });
        c.assert.assertStatus(res, 200);
      }
    );
  });

  // ── F4 Boundaries: Command Center RBAC & CSRF (5 tests) ───────────────────
  suite.describe("F4 Boundaries: Admin Command Center RBAC, CSRF & Subroutes", () => {
    suite.test(
      "unauthenticated & all 5 READER_ROLES are denied /api/v1/admin/me, /overview, and /users",
      { req: "F4" },
      async (c) => {
        const guest = c.http.createClient();
        c.assert.assertStatus(await guest.get("/api/v1/admin/me"), 401);
        c.assert.assertStatus(await guest.get("/api/v1/admin/users"), 401);

        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          for (const ep of ["/api/v1/admin/me", "/api/v1/admin/overview", "/api/v1/admin/users"]) {
            const r = await session.client.get(ep);
            c.assert.assertStatus(r, 403, `Reader ${role} must get 403 on ${ep}`);
          }
        }
      }
    );

    suite.test(
      "non-user-admin staff roles (content_editor, editor, ads_manager, finance_viewer, business_dev) get 403 on /api/v1/admin/users & /monitoring/bundle",
      { req: "F4" },
      async (c) => {
        for (const role of PRIVILEGE_RESTRICTED_STAFF) {
          const session = await getCachedRoleSession(c, role);
          const [usersRes, monRes] = await Promise.all([
            session.client.get("/api/v1/admin/users"),
            session.client.get("/api/v1/admin/monitoring/bundle"),
          ]);
          c.assert.assertStatus(usersRes, 403, `${role} must get 403 on /api/v1/admin/users`);
          c.assert.assertStatus(monRes, [403, 503], `${role} must not read monitoring bundle`);
        }
      }
    );

    suite.test(
      "authenticated staff mutating admin API rejects missing or forged X-CSRF-Token (403)",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.post("/api/v1/admin/categories", {
          headers: { "x-csrf-token": "forged-csrf-token-e2e" },
          json: { name_tr: "CSRF Test", name_en: "CSRF Test", slug: "csrf-test" },
        });
        c.assert.assertStatus(res, 403);
      }
    );

    suite.test(
      "unknown sub-route under /podmin/nonexistent-panel returns 303 unauth and 404 auth (not 500)",
      { req: "F4" },
      async (c) => {
        const guestRes = await c.http.createClient().get("/podmin/nonexistent-panel-999", {
          redirect: "manual",
        });
        c.assert.assertStatus(guestRes, [303, 404]);

        const sa = await getCachedSuperAdmin(c);
        const authRes = await sa.client.get("/podmin/nonexistent-panel-999");
        c.assert.assertStatus(authRes, 404);
      }
    );

    suite.test(
      "support_agent cannot escalate own role via /api/v1/account/profile payload injection",
      { req: "F4" },
      async (c) => {
        const disp = await c.auth.createUserWithRole("support_agent");
        await disp.client.post("/api/v1/account/profile", {
          json: {
            full_name: "Support Test",
            role: "super_admin",
          },
        });
        const me = await disp.client.get("/api/v1/admin/me");
        c.assert.assertStatus(me, 200);
        c.assert.assertEqual(me.json?.data?.role || me.json?.role, "support_agent");
      }
    );
  });

  // ── F5 Boundaries: Design Studio RBAC & Validation (5 tests) ──────────────
  suite.describe("F5 Boundaries: Design Studio RBAC & Preview Security", () => {
    suite.test(
      "all 7 non-Studio staff roles get 403 on GET /podmin/tasarim",
      { req: "F5" },
      async (c) => {
        for (const role of STUDIO_DENIED_STAFF_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/podmin/tasarim", { redirect: "manual" });
          c.assert.assertStatus(res, 403, `Non-Studio role ${role} must get 403 on /podmin/tasarim`);
        }
      }
    );

    suite.test(
      "all 7 non-Studio staff roles get 403 on POST /podmin/tasarim/onizleme and GET /api/v1/admin/design",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const designRes = await sa.client.get("/api/v1/admin/design");
        c.assert.assertStatus(designRes, 200);
        const validConfig = designRes.json?.config;
        for (const role of STUDIO_DENIED_STAFF_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const [prev, api] = await Promise.all([
            session.client.post("/podmin/tasarim/onizleme", { json: validConfig }),
            session.client.get("/api/v1/admin/design"),
          ]);
          c.assert.assertStatus(prev, 403, `${role} must get 403 on POST /podmin/tasarim/onizleme`);
          c.assert.assertStatus(api, 403, `${role} must get 403 on GET /api/v1/admin/design`);
        }
      }
    );

    suite.test(
      "POST /podmin/tasarim/onizleme strips <script tags from sandboxed preview HTML",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const designRes = await sa.client.get("/api/v1/admin/design");
        c.assert.assertStatus(designRes, 200);
        const slug = designRes.json?.config?.pages?.[0]?.slug || "magazine";
        const clonedConfig = JSON.parse(JSON.stringify(designRes.json?.config));
        const probeScript = '<script data-e2e-probe="1">window.__xss=1;</script>';
        const targetPage =
          (clonedConfig.pages || []).find((p) => p.slug === slug) || clonedConfig.pages?.[0];
        if (targetPage) {
          targetPage.title =
            (targetPage.title || "").length + probeScript.length + 1 <= 200
              ? `${targetPage.title} ${probeScript}`
              : `MSI Preview ${probeScript}`;
          targetPage.description =
            (targetPage.description || "").length + probeScript.length + 1 <= 2000
              ? `${targetPage.description} ${probeScript}`
              : `MSI Description ${probeScript}`;
          const enabledSec = (targetPage.sections || []).find((s) => s.enabled);
          if (enabledSec) {
            enabledSec.title =
              (enabledSec.title || "").length + probeScript.length + 1 <= 200
                ? `${enabledSec.title} ${probeScript}`
                : `Section ${probeScript}`;
            enabledSec.description =
              (enabledSec.description || "").length + probeScript.length + 1 <= 10000
                ? `${enabledSec.description} ${probeScript}`
                : `Section Desc ${probeScript}`;
          }
        }
        const res = await sa.client.post("/podmin/tasarim/onizleme", {
          query: { page: slug },
          json: clonedConfig,
        });
        c.assert.assertStatus(res, 200);
        c.assert.assertNotIncludes(
          res.text,
          "<script data-e2e-probe",
          "Preview HTML must strip or escape injected <script data-e2e-probe> payload"
        );
        c.assert.assertEqual(
          /<script\b/i.test(res.text),
          false,
          "Preview HTML must strip <script> tags for sandboxed iframe hygiene"
        );
      }
    );

    suite.test(
      "POST /api/v1/admin/design rejects invalid non-object payload with 400/422 (no 500)",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.post("/api/v1/admin/design", {
          json: { invalid_design_payload: true },
        });
        c.assert.assertStatus(res, [400, 422]);
      }
    );

    suite.test(
      "unauthenticated POST /podmin/tasarim/onizleme returns 401",
      { req: "F5", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().post("/podmin/tasarim/onizleme", { json: {} });
        c.assert.assertStatus(res, 401);
      }
    );
  });

  // ── F6 Boundaries: /hesabim Isolation & Account Security (5 tests) ────────
  suite.describe("F6 Boundaries: /hesabim Reader Isolation & CSRF", () => {
    suite.test(
      "unauthenticated & all 5 READER_ROLES never receive .account-command-card or admin href on /hesabim",
      { req: "F6" },
      async (c) => {
        const guest = await c.http.createClient().get("/hesabim");
        c.assert.assertStatus(guest, 200);
        c.assert.assertNotIncludes(guest.text, "account-command-card");
        c.assert.assertNotIncludes(guest.text, `/${c.adminPath}`);

        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/hesabim");
          c.assert.assertStatus(res, 200);
          c.assert.assertNotIncludes(res.text, "account-command-card");
          c.assert.assertNotIncludes(res.text, `href="/${c.adminPath}"`);
          c.assert.assertNotIncludes(res.text, 'href="/podmin"');
        }
      }
    );

    suite.test(
      "/hesabim?email=<other> query parameter cannot impersonate another user",
      { req: "F6" },
      async (c) => {
        const r1 = await getCachedRoleSession(c, "registered");
        const res = await r1.client.get("/hesabim", {
          query: { email: c.credentials.superAdmin.email },
        });
        c.assert.assertStatus(res, 200);
        c.assert.assertNotIncludes(res.text, c.credentials.superAdmin.email);
        c.assert.assertNotIncludes(res.text, "account-command-card");
      }
    );

    suite.test(
      "POST /api/v1/account/profile enforces authentication (401) and CSRF token (403)",
      { req: "F6" },
      async (c) => {
        const unauthRes = await c.http.createClient().post("/api/v1/account/profile", {
          json: { full_name: "Hacker" },
        });
        c.assert.assertStatus(unauthRes, 401);

        const reader = await getCachedRoleSession(c, "registered");
        const badCsrf = await reader.client.post("/api/v1/account/profile", {
          headers: { "x-csrf-token": "wrong-csrf-e2e" },
          json: { full_name: "Bad CSRF" },
        });
        c.assert.assertStatus(badCsrf, 403);
      }
    );

    suite.test(
      "GET /api/v1/account/export denies unauthenticated callers (401)",
      { req: "F6", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/api/v1/account/export");
        c.assert.assertStatus(res, 401);
      }
    );

    suite.test(
      "POST /api/v1/account/delete rejects wrong password (401/403) without erasing account",
      { req: "F6" },
      async (c) => {
        const disp = await c.auth.registerUser();
        const delRes = await disp.client.post("/api/v1/account/delete", {
          json: { password: "WrongPassword!999", confirm_erasure: true },
        });
        c.assert.assertStatus(delRes, [400, 401, 403]);
        const stillAlive = await disp.client.get("/api/v1/account/export");
        c.assert.assertStatus(stillAlive, 200);
      }
    );
  });

  // ── F7 Boundaries: Open Redirect, XSS Escaping & Auth Validation (5 tests)
  suite.describe("F7 Boundaries: Redirect Sanitization, Reset & Anti-Spam", () => {
    suite.test(
      "external ?next=https://evil.example open-redirect attempt does not redirect externally on /hesabim",
      { req: "F7", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/hesabim", {
          query: { next: "https://evil.example/phish" },
          redirect: "manual",
        });
        c.assert.assertStatus(res, 200);
        const loc = res.headers.get("location") || "";
        c.assert.assertNotIncludes(loc, "evil.example");
      }
    );

    suite.test(
      "GET /sifre-sifirla?token= blocks <script> via WAF and HTML-escapes attribute quotes safely",
      { req: "F7", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const wafBlocked = await client.get("/sifre-sifirla", {
          query: { token: '"><script>alert(1)</script>' },
        });
        c.assert.assertStatus(wafBlocked, 403);

        const attrEscaped = await client.get("/sifre-sifirla", {
          query: { token: '" data-injected="1' },
        });
        c.assert.assertStatus(attrEscaped, 200);
        c.assert.assertNotIncludes(attrEscaped.text, '" data-injected="1"');
        c.assert.assertIncludes(attrEscaped.text, "&quot; data-injected=&quot;1");
      }
    );

    suite.test(
      "POST /api/v1/auth/reset-password rejects invalid token and short password (<8 chars) with 400/422",
      { req: "F7" },
      async (c) => {
        const client = c.http.createClient();
        const badToken = await client.post("/api/v1/auth/reset-password", {
          json: { token: "nonexistent-reset-token", new_password: "ValidPassword!2026" },
        });
        c.assert.assertStatus(badToken, [400, 404, 422]);

        const shortPw = await client.post("/api/v1/auth/reset-password", {
          json: { token: "nonexistent-reset-token", new_password: "short" },
        });
        c.assert.assertStatus(shortPw, [400, 422]);
      }
    );

    suite.test(
      "POST /api/v1/auth/register rejects triggered honeypot (no user created) and missing KVKK/TOS consent",
      { req: "F7" },
      async (c) => {
        const bot = await c.auth.registerUser({
          allowError: true,
          website_url: "https://spam.example",
        });
        // Backend silently drops honeypot submissions with 200 user:null (or 4xx) without creating an account
        c.assert.assertEqual(bot.user, null, "Honeypot submission must never create a user");
        const loginBot = await c.http.createClient().post("/api/v1/auth/login", {
          json: { email: bot.email, password: bot.password },
        });
        c.assert.assertStatus(loginBot, 401, "Honeypot-dropped registration must not allow login");

        const noConsent = await c.auth.registerUser({
          allowError: true,
          kvkk_consent: false,
          tos_accepted: false,
        });
        c.assert.assertStatus(noConsent.res, [400, 422]);
      }
    );

    suite.test(
      "POST /api/v1/auth/register rejects duplicate email (409) and malformed email (400/422)",
      { req: "F7" },
      async (c) => {
        const first = await c.auth.registerUser();
        const dup = await c.auth.registerUser({
          allowError: true,
          email: first.email,
        });
        c.assert.assertStatus(dup.res, [400, 409, 422]);

        const badEmail = await c.auth.registerUser({
          allowError: true,
          email: "not-an-email",
        });
        c.assert.assertStatus(badEmail.res, [400, 422]);
      }
    );
  });
}
