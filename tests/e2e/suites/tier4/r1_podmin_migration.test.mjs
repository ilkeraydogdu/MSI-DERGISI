import fs from "node:fs";
import path from "node:path";

import { createBrowserManager } from "../../lib/browser.mjs";
import { Client } from "../../lib/http.mjs";
import { startIsolatedServer } from "../../lib/server.mjs";
import { runUxRubricAudit } from "./ux_rubric_audit.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 R1 /podmin Migration & UX Rubric Audit Harness", () => {
    suite.test(
      "T4-37: Isolated server with adminPath='podmin' AND adminPath='' — full /podmin & /podmin/tasarim workflows, /msi-komuta* 404, zero msi-komuta leaks",
      { req: ["F1", "F3", "F4", "F5", "F6"] },
      async (c) => {
        for (const cfg of [{ adminPath: "podmin" }, { adminPath: "" }]) {
          const srv = await startIsolatedServer(cfg);
          const bm = createBrowserManager({ baseUrl: srv.baseUrl, repoRoot: c.repoRoot });
          try {
            const client = new Client({ baseUrl: srv.baseUrl });
            await c.auth.login(client, srv.credentials.superAdmin);
            await client.ensureCsrf();

            // 1. All /msi-komuta* endpoints return 404
            for (const dead of [
              "/msi-komuta",
              "/msi-komuta/",
              "/msi-komuta/tasarim",
              "/msi-komuta/anything",
            ]) {
              const r = await client.get(dead, { redirect: "manual" });
              assert.assertStatus(r, 404, `${dead} must return 404`);
            }
            assert.assertStatus(await client.post("/msi-komuta/tasarim/onizleme", { json: {} }), 404);

            // 2. Zero 'msi-komuta' leaks across all HTML & core JS responses
            for (const p of [
              "/",
              "/en",
              "/hesabim",
              "/podmin",
              "/podmin/tasarim",
              "/sifre-sifirla",
              "/js/admin/core.js",
              "/js/admin/login.js",
              "/js/msi-design-editor.js",
              "/js/account-portal.js",
              "/js/msi-ui.js",
            ]) {
              const r = await client.get(p);
              assert.assertStatus(r, 200, `${p} must return 200`);
              assert.assertNotIncludes(r.text.toLowerCase(), "msi-komuta", `Leak found in ${p}`);
            }

            // 3. Unauthenticated login gateway -> login -> /podmin Command Center -> /podmin/tasarim -> /hesabim
            const unauthPage = await bm.openPage("/podmin", { viewport: bm.VIEWPORT_DESKTOP });
            try {
              assert.assertEqual(unauthPage.status, 200);
              const dataAdminPath = await unauthPage.page.locator("body").getAttribute("data-admin-path");
              assert.assertEqual(dataAdminPath, "/podmin", "Gateway body[data-admin-path] must equal /podmin");

              await unauthPage.page.locator("#login-email").fill(srv.credentials.superAdmin.email);
              await unauthPage.page.locator("#login-password").fill(srv.credentials.superAdmin.password);
              await Promise.all([
                unauthPage.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
                unauthPage.page.locator("#login-submit-btn").click(),
              ]);
              await unauthPage.page.waitForFunction(
                () => Boolean(window.MSIAdmin?._me?.role === "super_admin"),
                { timeout: 15000 }
              );
              assert.assertIncludes(unauthPage.page.url(), "/podmin");
              assert.assertNoAssetOrNetworkErrors(
                unauthPage,
                ["/api/v1/admin/monitoring/bundle"],
                "T4-37 /podmin browser login->dashboard"
              );

              // Navigate to Design Studio via sidebar link
              const studioHref = await unauthPage.page
                .locator('#admin-sidebar a[href$="/tasarim"]')
                .first()
                .getAttribute("href");
              assert.assertEqual(studioHref, "/podmin/tasarim");
              await unauthPage.page.goto(`${srv.baseUrl}/podmin/tasarim`, { waitUntil: "networkidle" });
              await unauthPage.page.waitForFunction(
                () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
                { timeout: 15000 }
              );
              assert.assertEqual(
                await unauthPage.page.locator("#editor-admin-link").getAttribute("href"),
                "/podmin"
              );

              // Navigate to /hesabim and confirm .account-command-card points to /podmin
              await unauthPage.page.goto(`${srv.baseUrl}/hesabim`, { waitUntil: "networkidle" });
              const cardLinks = await unauthPage.page.evaluate(() =>
                Array.from(document.querySelectorAll(".account-command-card a[href]")).map((a) =>
                  a.getAttribute("href")
                )
              );
              assert.assertIncludes(cardLinks, "/podmin");
              assert.assertIncludes(cardLinks, "/podmin/tasarim");
              assert.assertNoAssetOrNetworkErrors(
                unauthPage,
                ["/api/v1/admin/monitoring/bundle"],
                "T4-37 full workflow"
              );
            } finally {
              await unauthPage.close();
            }
          } finally {
            await bm.close();
            await srv.stop();
          }
        }
      }
    );

    suite.test(
      "T4-38: UX Rubric Audit Harness writes screenshots + ux_rubric_report.json with 0 runtime errors, 0 overflow, 0 broken media, and 0 overlaps",
      { req: ["F9", "F2", "F3", "F4", "F5", "F6", "F7"] },
      async (c) => {
        const outDir = c.tempDir
          ? path.join(c.tempDir, `ux_rubric_t4_38_${Date.now()}`)
          : path.join(c.repoRoot, ".tmp_ux_rubric_t4_38");

        try {
          const { report, reportPath } = await runUxRubricAudit({
            outDir,
            theme: "both",
            viewports: ["desktop", "mobile"],
            adminPath: "podmin",
            quick: true,
            strict: false,
          });

          assert.assertOk(fs.existsSync(reportPath), `Expected report JSON at ${reportPath}`);
          assert.assertOk(report.summary.totalChecks >= 10, `Expected >= 10 audit checks, got ${report.summary.totalChecks}`);
          assert.assertOk(report.summary.reducedMotionOk, "Reduced motion check must settle cleanly");

          for (const r of report.results) {
            assert.assertOk(fs.existsSync(r.screenshotFullPath), `Missing full screenshot: ${r.screenshotFullPath}`);
            assert.assertOk(fs.existsSync(r.screenshotViewportPath), `Missing viewport screenshot: ${r.screenshotViewportPath}`);
            assert.assertEqual(
              r.metrics.hasHorizontalOverflow,
              false,
              `[${r.id}] Horizontal overflow detected: ${r.metrics.scrollWidth} > ${r.metrics.innerWidth}`
            );
            assert.assertEqual(
              r.metrics.brokenImages.length,
              0,
              `[${r.id}] Broken images: ${JSON.stringify(r.metrics.brokenImages)}`
            );
            assert.assertEqual(
              r.metrics.zeroSizeSvgs.length,
              0,
              `[${r.id}] Zero-size SVGs: ${JSON.stringify(r.metrics.zeroSizeSvgs)}`
            );
            assert.assertEqual(
              r.metrics.overlapViolationsTotal,
              0,
              `[${r.id}] Overlapping interactive controls: ${JSON.stringify(r.metrics.overlapViolations)}`
            );
            assert.assertEqual(
              r.metrics.consoleErrors.length,
              0,
              `[${r.id}] Console errors: ${JSON.stringify(r.metrics.consoleErrors)}`
            );
            assert.assertEqual(
              r.metrics.pageErrors.length,
              0,
              `[${r.id}] Page errors: ${JSON.stringify(r.metrics.pageErrors)}`
            );
            assert.assertEqual(
              r.metrics.httpErrors.length,
              0,
              `[${r.id}] HTTP errors: ${JSON.stringify(r.metrics.httpErrors)}`
            );
          }
          assert.assertEqual(report.summary.failed, 0, "UX Rubric core checks must report 0 failures");
        } finally {
          fs.rmSync(outDir, { recursive: true, force: true });
        }
      }
    );

    suite.test(
      "T4-41: Trailing-slash /podmin/ & /podmin/tasarim/ GET/HEAD redirects + 404 /msi-komuta SEO headers",
      { req: ["F1", "F3", "F5"] },
      async (c) => {
        const anon = c.http.createClient();
        for (const [p, expectedUnauthLoc] of [
          ["/podmin/", "/podmin"],
          ["/podmin/tasarim/", "/podmin"],
          [`/${c.adminPath}/`, `/${c.adminPath}`],
          [`/${c.adminPath}/tasarim/`, `/${c.adminPath}`],
        ]) {
          for (const method of ["GET", "HEAD"]) {
            const r = await anon.request(method, p, { redirect: "manual" });
            assert.assertStatus(r, 303, `Unauth ${method} ${p} must return 303 redirect (not 401 JSON)`);
            assert.assertEqual(r.headers.get("location"), expectedUnauthLoc);
          }
        }

        const sa = await c.auth.loginAsSuperAdmin();
        for (const [p, expectedLoc] of [
          [`/${c.adminPath}/`, `/${c.adminPath}`],
          [`/${c.adminPath}/tasarim/`, `/${c.adminPath}/tasarim`],
        ]) {
          for (const method of ["GET", "HEAD"]) {
            const r = await sa.client.request(method, p, { redirect: "manual" });
            assert.assertStatus(r, 308, `Auth ${method} ${p} must return 308 permanent redirect (not 404)`);
            assert.assertEqual(r.headers.get("location"), expectedLoc);
          }
          const followed = await sa.client.get(p);
          assert.assertStatus(followed, 200, `Following auth ${p} must return 200`);
        }

        const dead = await anon.get("/msi-komuta", { redirect: "manual" });
        assert.assertStatus(dead, 404);
        assert.assertEqual((dead.headers.get("link") || "").includes('rel="canonical"'), false, "404 /msi-komuta must not emit rel=canonical");
        assert.assertIncludes(dead.headers.get("x-robots-tag") || "", "noindex");
        assert.assertIncludes(dead.headers.get("x-robots-tag") || "", "nofollow");
      }
    );
  });
}
