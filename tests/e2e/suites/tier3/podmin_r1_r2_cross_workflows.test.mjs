import {
  assertNoUnexpectedBrowserErrors,
  auditUiSurface,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";
import { createBrowserManager } from "../../lib/browser.mjs";
import { Client } from "../../lib/http.mjs";
import { startIsolatedServer } from "../../lib/server.mjs";

const STUDIO_PERMITTED_ROLES = new Set([
  "super_admin",
  "admin",
  "site_operator",
  "editor_in_chief",
  "managing_editor",
]);

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 3 Pairwise R1/R2 Cross-Feature Workflows (T3-49..T3-54)", () => {
    suite.test(
      "T3-49: Explicit podmin + empty ADMIN_PATH_SECRET='' route & link parity across /podmin, /podmin/tasarim, /hesabim & /msi-komuta 404",
      { req: ["F1", "F5", "F6"] },
      async (c) => {
        for (const cfg of [{ adminPath: "podmin" }, { adminPath: "" }]) {
          const srv = await startIsolatedServer(cfg);
          const bm = createBrowserManager({ baseUrl: srv.baseUrl, repoRoot: c.repoRoot });
          try {
            const client = new Client({ baseUrl: srv.baseUrl });
            await c.auth.login(client, srv.credentials.superAdmin);
            await client.ensureCsrf();

            // Retired routes return 404
            for (const dead of ["/msi-komuta", "/msi-komuta/", "/msi-komuta/tasarim"]) {
              const r = await client.get(dead, { redirect: "manual" });
              assert.assertStatus(r, 404, `Retired ${dead} must be 404 (adminPath=${JSON.stringify(cfg.adminPath)})`);
            }
            assert.assertStatus(await client.post("/msi-komuta/tasarim/onizleme", { json: {} }), 404);

            // Canonical /podmin, /podmin/tasarim, /hesabim parity
            const [adm, studio, hesabim, home] = await Promise.all([
              client.get("/podmin"),
              client.get("/podmin/tasarim"),
              client.get("/hesabim"),
              client.get("/"),
            ]);
            for (const res of [adm, studio, hesabim, home]) {
              assert.assertStatus(res, 200);
              assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
            }
            assert.assertIncludes(studio.text, 'href="/podmin"');
            assert.assertIncludes(hesabim.text, 'href="/podmin"');
            assert.assertIncludes(hesabim.text, 'href="/podmin/tasarim"');

            // Browser render of /podmin & /podmin/tasarim without 4xx/5xx/console errors
            const browserCtx = await bm.newContext({ client, viewport: bm.VIEWPORT_DESKTOP });
            try {
              const p1 = await bm.openPage("/podmin", { context: browserCtx });
              try {
                await p1.page.waitForFunction(() => Boolean(window.MSIAdmin?._me?.role), { timeout: 10000 });
                assert.assertNoAssetOrNetworkErrors(
                  p1,
                  ["/api/v1/admin/monitoring/bundle"],
                  "T3-49 /podmin asset check"
                );
              } finally {
                await p1.close();
              }

              const p2 = await bm.openPage("/podmin/tasarim", { context: browserCtx });
              try {
                await p2.page.locator("#design-preview").waitFor({ state: "visible", timeout: 10000 });
                assert.assertNoAssetOrNetworkErrors(p2, [], "T3-49 /podmin/tasarim asset check");
              } finally {
                await p2.close();
              }
            } finally {
              await browserCtx.close();
            }
          } finally {
            await bm.close();
            await srv.stop();
          }
        }
      }
    );

    suite.test(
      "T3-50: Login gateway Playwright pairwise (forgot password, TOTP 2FA, lang switch, theme toggle -> Command Center)",
      { req: ["F2", "F3", "F4"] },
      async (c) => {
        const staff = await c.auth.createUserWithRole("site_operator");
        const { secret } = await c.auth.enableTotp(staff.client);

        const pageRes = await openDomainPage(c, "/podmin", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);

          // 1. Query localization ?lang=en <-> ?lang=tr (no manual TR/EN buttons in UI)
          await pageRes.page.goto(`${c.baseUrl}/podmin?lang=en`, { waitUntil: "networkidle" });
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "en");
          await pageRes.page.goto(`${c.baseUrl}/podmin?lang=tr`, { waitUntil: "networkidle" });
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "tr");

          // 2. Autonomous OS color-scheme synchronization (dark <-> light)
          await pageRes.page.emulateMedia({ colorScheme: "dark" });
          await pageRes.page.waitForTimeout(120);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("data-theme"), "dark", "OS dark scheme must auto-apply dark theme");
          await pageRes.page.emulateMedia({ colorScheme: "light" });
          await pageRes.page.waitForTimeout(120);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("data-theme"), "light", "OS light scheme must auto-apply light theme");

          // 3. Forgot password subview & back
          await pageRes.page.locator("#btn-show-forgot").click();
          await pageRes.page.locator("#forgot-password-form").waitFor({ state: "visible", timeout: 5000 });
          await pageRes.page.locator("#forgot-email").fill(staff.email);
          await pageRes.page.locator("#forgot-submit-btn").click();
          await pageRes.page.locator("#login-success.visible").waitFor({ state: "visible", timeout: 10000 });
          await pageRes.page.locator("#btn-back-from-forgot").click();
          await pageRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 5000 });

          // 4. TOTP 2FA login -> redirects to Command Center
          await pageRes.page.locator("#login-email").fill(staff.email);
          await pageRes.page.locator("#login-password").fill(staff.password);
          await pageRes.page.locator("#login-submit-btn").click();
          await pageRes.page.locator("#totp-login-form").waitFor({ state: "visible", timeout: 10000 });

          await pageRes.page.locator("#totp-code-input").fill(c.totp.generateTotp(secret));
          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            pageRes.page.locator("#totp-submit-btn").click(),
          ]);
          await pageRes.page.waitForFunction(() => Boolean(window.MSIAdmin?._me?.role), { timeout: 10000 });
          assert.assertNoOverflow(await pageRes.measureOverflow());
          assert.assertNoAssetOrNetworkErrors(
            pageRes,
            ["/api/v1/admin/monitoring/bundle"],
            "T3-50 gateway->command center errors"
          );
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-51: All 12 staff roles Command Center + /hesabim .account-command-card pairwise boundary check",
      { req: ["F4", "F6"] },
      async (c) => {
        for (const role of c.auth.STAFF_ROLES) {
          const actor = await c.auth.createUserWithRole(role);
          const [admRes, meRes, hesabimRes] = await Promise.all([
            actor.client.get("/podmin"),
            actor.client.get("/api/v1/admin/me"),
            actor.client.get("/hesabim"),
          ]);
          assert.assertStatus(admRes, 200, `${role} /podmin`);
          assert.assertStatus(meRes, 200, `${role} /api/v1/admin/me`);
          assert.assertStatus(hesabimRes, 200, `${role} /hesabim`);

          assert.assertIncludes(admRes.text, `data-operator-role="${role}"`);
          assert.assertIncludes(hesabimRes.text, "account-command-card");
          assert.assertIncludes(hesabimRes.text, `href="/${c.adminPath}"`);

          if (STUDIO_PERMITTED_ROLES.has(role)) {
            assert.assertIncludes(hesabimRes.text, `href="/${c.adminPath}/tasarim"`);
          } else {
            assert.assertNotIncludes(hesabimRes.text, `/${c.adminPath}/tasarim`);
          }
        }

        // Playwright verification of previously unexercised staff roles (admin, managing_editor, editor)
        for (const role of ["admin", "managing_editor", "editor"]) {
          const actor = await c.auth.createUserWithRole(role);
          const pageRes = await openDomainPage(c, "/podmin", {
            client: actor.client,
            viewport: c.browser.VIEWPORT_DESKTOP,
          });
          try {
            await pageRes.page.waitForFunction(
              (r) => window.MSIAdmin?._me?.role === r,
              role,
              { timeout: 10000 }
            );
            assert.assertNoOverflow(await pageRes.measureOverflow(), `${role} /podmin overflow`);
            assert.assertNoAssetOrNetworkErrors(
              pageRes,
              ["/api/v1/admin/monitoring/bundle"],
              `T3-51 [${role}] browser errors`
            );
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T3-52: All 5 reader roles + guest /hesabim theme toggle + tab switching + logout pairwise check",
      { req: ["F2", "F6"] },
      async (c) => {
        // Guest check
        const guestPage = await openDomainPage(c, "/hesabim", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(guestPage.status, 200);
          assert.assertEqual(await guestPage.page.locator(".account-command-card").count(), 0);
          assert.assertOk((await guestPage.page.locator(".account-welcome").count()) >= 1);
          assert.assertNoAssetOrNetworkErrors(guestPage, [], "T3-52 guest /hesabim");
        } finally {
          await guestPage.close();
        }

        // All 5 reader roles verified over HTTP + registered interactive Playwright flow
        for (const role of c.auth.READER_ROLES) {
          const actor = await c.auth.createUserWithRole(role);
          const res = await actor.client.get("/hesabim");
          assert.assertStatus(res, 200);
          assert.assertNotIncludes(res.text, "account-command-card");
          assert.assertIncludes(res.text, 'id="ap-theme-toggle"');

          if (role === "registered" || role === "subscriber_public") {
            const pRes = await openDomainPage(c, "/hesabim", {
              client: actor.client,
              viewport: c.browser.VIEWPORT_DESKTOP,
            });
            try {
              const beforeTheme = await pRes.page.locator("html").getAttribute("data-theme");
              await pRes.page.locator("#ap-theme-toggle").click();
              await pRes.page.waitForTimeout(120);
              const afterTheme = await pRes.page.locator("html").getAttribute("data-theme");
              assert.assertNotEqual(afterTheme, beforeTheme, "#ap-theme-toggle must toggle theme");

              for (const tabKey of ["edergi", "profile", "security", "notifications", "overview"]) {
                await pRes.page.locator(`[data-ap-tab="${tabKey}"]`).first().click();
                await pRes.page.locator(`[data-ap-panel="${tabKey}"]`).waitFor({ state: "visible", timeout: 5000 });
                assert.assertNoOverflow(await pRes.measureOverflow(), `${role} tab ${tabKey}`);
              }

              // Logout returns to guest gate
              await Promise.all([
                pRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
                pRes.page.locator("[data-logout]").first().click(),
              ]);
              await pRes.page.locator(".account-welcome").waitFor({ state: "visible", timeout: 8000 });
              assert.assertNoAssetOrNetworkErrors(pRes, [], `T3-52 [${role}]`);
            } finally {
              await pRes.close();
            }
          }
        }
      }
    );

    suite.test(
      "T3-53: Public #auth-dialog + .gov.tr hint + ?next= redirect + /sifre-sifirla pairwise flow",
      { req: ["F7"] },
      async (c) => {
        const reg = await c.auth.registerUser({ email: `${uid("t3next")}@example.com` });

        // 1. #auth-dialog & .gov.tr domain hint via /hesabim guest login gate
        const homeRes = await openDomainPage(c, "/hesabim", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await homeRes.page.locator('a[href="#modal-login"]').first().click();
          await homeRes.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 5000 });
          await homeRes.page.locator("#register-tab").click();
          await homeRes.page.locator('#register-form input[name="email"]').fill("analyst@ssb.gov.tr");
          await homeRes.page.locator('#register-form input[name="email"]').dispatchEvent("input");
          await homeRes.page.locator("#gov-domain-hint").waitFor({ state: "visible", timeout: 5000 });
          assert.assertNoAssetOrNetworkErrors(homeRes, [], "T3-53 #auth-dialog");
        } finally {
          await homeRes.close();
        }

        // 2. ?next=/e-dergi auto-opens #auth-dialog and redirects after login
        const nextRes = await openDomainPage(c, "/hesabim?next=/e-dergi", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await nextRes.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 8000 });
          await nextRes.page.locator('#auth-dialog #login-form input[name="email"]').fill(reg.email);
          await nextRes.page.locator('#auth-dialog #login-form input[name="password"]').fill(reg.password);
          await nextRes.page.waitForTimeout(2200);
          await Promise.all([
            nextRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            nextRes.page.locator('#auth-dialog #login-form button[type="submit"]').click(),
          ]);
          assert.assertIncludes(nextRes.page.url(), "/e-dergi", "?next=/e-dergi must redirect after login");
          assert.assertNoAssetOrNetworkErrors(nextRes, [], "T3-53 ?next=/e-dergi");
        } finally {
          await nextRes.close();
        }

        // 3. /sifre-sifirla UI form validation & error handling
        const resetRes = await openDomainPage(c, "/sifre-sifirla?token=invalid-t3-token", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await auditUiSurface(c, resetRes, { label: "T3-53 /sifre-sifirla", requireMain: true });
          await resetRes.page.locator('#reset-password-form input[name="new_password"]').fill("ValidPass!2026");
          await resetRes.page.locator('#reset-password-form input[name="confirm_password"]').fill("DifferentPass!2026");
          await resetRes.page.locator('#reset-password-form button[type="submit"]').click();
          await resetRes.page.locator("#reset-password-form [data-feedback]").waitFor({ state: "visible", timeout: 5000 });
          assertNoUnexpectedBrowserErrors(c, resetRes, ["400", "404"], "T3-53 reset password");
        } finally {
          await resetRes.close();
        }
      }
    );

    suite.test(
      "T3-54: Design Studio /podmin/tasarim theme toggle + preview theme + prefers-reduced-motion pairwise check",
      { req: ["F2", "F5"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const pageRes = await openDomainPage(c, "/podmin/tasarim", {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await pageRes.page.waitForFunction(
            () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
            { timeout: 15000 }
          );

          const initTheme = await pageRes.page.locator("html").getAttribute("data-theme");
          await pageRes.page.locator("#studio-theme-toggle").click();
          await pageRes.page.waitForTimeout(150);
          const nextTheme = await pageRes.page.locator("html").getAttribute("data-theme");
          assert.assertNotEqual(nextTheme, initTheme, "#studio-theme-toggle must toggle studio theme");

          await pageRes.page.locator("#preview-theme").click();
          await pageRes.page.waitForTimeout(200);
          const previewTheme = await pageRes.page
            .locator("#design-preview")
            .evaluate((f) => f.contentDocument?.documentElement?.getAttribute("data-theme"));
          assert.assertEqual(previewTheme, "dark", "#preview-theme click must switch preview iframe to dark");

          await pageRes.emulateReducedMotion(true);
          await pageRes.page.reload({ waitUntil: "networkidle" });
          await pageRes.page.locator("#design-preview").waitFor({ state: "visible", timeout: 10000 });
          assert.assertNoOverflow(await pageRes.measureOverflow());
          assert.assertNoAssetOrNetworkErrors(pageRes, [], "T3-54 Design Studio");
        } finally {
          await pageRes.close();
        }
      }
    );
  });
}
