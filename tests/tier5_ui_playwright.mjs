#!/usr/bin/env node
import path from "node:path";
import { fileURLToPath } from "node:url";
import { startIsolatedServer } from "./e2e/lib/server.mjs";
import { createHttpFactory } from "./e2e/lib/http.mjs";
import { createAuthHelpers } from "./e2e/lib/auth.mjs";
import { createBrowserManager, VIEWPORT_DESKTOP, VIEWPORT_MOBILE, measureOverflow } from "./e2e/lib/browser.mjs";
import * as assert from "./e2e/lib/assertions.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const REPO_ROOT = path.resolve(__dirname, "..");

async function main() {
  const srv = await startIsolatedServer({ repoRoot: REPO_ROOT, build: false });
  const http = createHttpFactory({ baseUrl: srv.baseUrl });
  const browserMgr = createBrowserManager({ baseUrl: srv.baseUrl, repoRoot: REPO_ROOT });
  const ctx = {
    baseUrl: srv.baseUrl,
    adminPath: srv.adminPath,
    credentials: srv.credentials,
    http,
    browser: { VIEWPORT_DESKTOP, VIEWPORT_MOBILE },
    assert,
  };
  ctx.auth = createAuthHelpers(ctx);

  const results = [];
  let failed = 0;

  async function runCheck(name, fn) {
    process.stdout.write(`  [TIER5-UI] ${name} ... `);
    try {
      await fn();
      console.log("PASS");
      results.push({ name, ok: true });
    } catch (err) {
      failed += 1;
      console.log("FAIL");
      console.error(`    -> ${err?.stack || err}`);
      results.push({ name, ok: false, error: String(err?.message || err) });
    }
  }

  try {
    const sa = await ctx.auth.loginAsSuperAdmin();
    const staffRoles = [
      "super_admin",
      "site_operator",
      "editor_in_chief",
      "content_editor",
      "ads_manager",
      "finance_viewer",
      "support_agent",
      "business_dev",
    ];
    const actors = {};
    for (const role of staffRoles) {
      if (role === "super_admin") actors[role] = sa;
      else actors[role] = await ctx.auth.createUserWithRole(role, { superAdminClient: sa.client });
    }

    // Check 1: /hesabim .account-command-card & sidebar links never 403 across all staff roles
    await runCheck("1. /hesabim command-card & sidebar links return 200 (never 403) across all staff roles", async () => {
      const studioDeniedRoles = new Set([
        "content_editor",
        "ads_manager",
        "finance_viewer",
        "support_agent",
        "business_dev",
      ]);

      for (const role of staffRoles) {
        const actor = actors[role];
        for (const vp of [VIEWPORT_DESKTOP, VIEWPORT_MOBILE]) {
          const pageRes = await browserMgr.openPage("/hesabim", {
            client: actor.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200, `[${role}] /hesabim must return 200`);
            const cardLinks = await pageRes.page.evaluate(() =>
              Array.from(document.querySelectorAll(".account-command-card a[href]")).map((a) =>
                a.getAttribute("href")
              )
            );
            assert.assertOk(cardLinks.length >= 1, `[${role}] Expected >=1 link in .account-command-card`);
            for (const href of cardLinks) {
              if (studioDeniedRoles.has(role)) {
                assert.assertOk(
                  !href.includes("/tasarim"),
                  `[${role}] must NOT see /tasarim link in .account-command-card (got ${href})`
                );
              }
              const linkRes = await actor.client.get(href, { redirect: "manual" });
              assert.assertEqual(
                linkRes.status,
                200,
                `[${role}] /hesabim command-card link ${href} returned ${linkRes.status}`
              );
            }
            const ov = await measureOverflow(pageRes.page);
            assert.assertOk(!ov.hasHorizontalOverflow, `[${role}] /hesabim (${vp.width}) overflow: ${ov.scrollWidth}>${ov.innerWidth}`);
            assert.assertEqual(pageRes.serverErrors.length, 0, `[${role}] /hesabim serverErrors: ${JSON.stringify(pageRes.serverErrors)}`);
            assert.assertEqual(pageRes.pageErrors.length, 0, `[${role}] /hesabim pageErrors: ${JSON.stringify(pageRes.pageErrors)}`);
          } finally {
            await pageRes.close();
          }
        }
      }
    });

    // Check 2: Every visible Admin sidebar tab across all 8 staff roles triggers ZERO 4xx/5xx errors,
    // editor_in_chief has NO #tab-17, and #tab-17 renders clean disabled banner when MONITORING_ENABLED=false
    await runCheck("2. Admin visible tabs yield 0 4xx/5xx errors across all staff roles & #tab-17 disabled state is clean", async () => {
      for (const role of staffRoles) {
        const actor = actors[role];
        const pageRes = await browserMgr.openPage(`/${ctx.adminPath}`, {
          client: actor.client,
          viewport: VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200, `[${role}] admin shell must return 200`);
          await pageRes.page.waitForFunction(
            () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
            { timeout: 15000 }
          );
          await pageRes.page.waitForTimeout(150);

          const visibleTabs = await pageRes.page.evaluate(() => {
            return Array.from(document.querySelectorAll('#admin-sidebar [data-tab^="tab-"]'))
              .filter((b) => {
                const st = window.getComputedStyle(b);
                return (
                  !b.classList.contains("hidden") &&
                  b.getAttribute("data-rbac-hidden") !== "1" &&
                  st.display !== "none"
                );
              })
              .map((b) => b.getAttribute("data-tab"));
          });

          if (role === "editor_in_chief") {
            assert.assertOk(
              !visibleTabs.includes("tab-17"),
              "editor_in_chief must NOT see #tab-17 (Sunucu İzleme)"
            );
          }

          for (const tabId of visibleTabs) {
            await pageRes.page.locator(`#admin-sidebar [data-tab="${tabId}"]`).first().click();
            await pageRes.page.waitForTimeout(120);
            const ov = await measureOverflow(pageRes.page);
            assert.assertOk(!ov.hasHorizontalOverflow, `[${role}] Admin ${tabId} horizontal overflow`);

            if (tabId === "tab-17") {
              const monState = await pageRes.page.evaluate(() => {
                const banner = document.getElementById("monitoring-status-banner");
                const cpuSub = document.getElementById("mon-cpu-sub")?.textContent || "";
                return {
                  bannerVisible: Boolean(banner && !banner.classList.contains("hidden") && banner.textContent.trim().length > 0),
                  bannerText: banner?.textContent?.trim() || "",
                  cpuSub,
                };
              });
              assert.assertOk(
                monState.bannerVisible,
                `[${role}] #tab-17 must show #monitoring-status-banner when monitoring is disabled`
              );
              assert.assertOk(
                !monState.cpuSub.includes("ölçülüyor"),
                `[${role}] #tab-17 #mon-cpu-sub must not stay stuck in 'ölçülüyor' (got: ${monState.cpuSub})`
              );
            }
          }

          assert.assertEqual(
            pageRes.serverErrors.length,
            0,
            `[${role}] Expected 0 4xx/5xx server errors across all visible Admin tabs, got: ${JSON.stringify(pageRes.serverErrors)}`
          );
          assert.assertEqual(
            pageRes.pageErrors.length,
            0,
            `[${role}] Expected 0 uncaught pageErrors, got: ${JSON.stringify(pageRes.pageErrors)}`
          );
        } finally {
          await pageRes.close();
        }
      }
    });

    // Check 3: Design Studio mobile (390x844) immediate preview accessibility + quick-toggle + desktop parity
    await runCheck("3. Design Studio mobile (390x844) immediate preview & Controls/Preview toggle + Desktop (1440x900) parity", async () => {
      const mobRes = await browserMgr.openPage(`/${ctx.adminPath}/tasarim`, {
        client: sa.client,
        viewport: VIEWPORT_MOBILE,
      });
      try {
        assert.assertEqual(mobRes.status, 200);
        await mobRes.page.locator("#design-preview").waitFor({ state: "visible", timeout: 15000 });
        await mobRes.page.waitForFunction(
          () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
          { timeout: 15000 }
        );

        const initialLayout = await mobRes.page.evaluate(() => {
          const iframe = document.querySelector("#design-preview");
          const switcher = document.querySelector(".mobile-studio-switcher");
          return {
            iframeTop: Math.round(iframe.getBoundingClientRect().top),
            switcherVisible: Boolean(switcher && window.getComputedStyle(switcher).display !== "none"),
          };
        });
        assert.assertOk(initialLayout.switcherVisible, "Mobile quick-toggle .mobile-studio-switcher must be visible at 390x844");
        assert.assertOk(
          initialLayout.iframeTop < 700,
          `Mobile #design-preview must be visible near top without scrolling >2500px (got iframeTop=${initialLayout.iframeTop})`
        );

        // Toggle to Controls -> preview hides, sidebar visible near top
        await mobRes.page.locator('[data-mobile-view="controls"]').click();
        await mobRes.page.waitForTimeout(150);
        const controlsState = await mobRes.page.evaluate(() => {
          const preview = document.querySelector(".editor-preview");
          const sidebar = document.querySelector(".editor-sidebar");
          return {
            previewHidden: window.getComputedStyle(preview).display === "none",
            sidebarTop: Math.round(sidebar.getBoundingClientRect().top),
          };
        });
        assert.assertOk(controlsState.previewHidden, "Clicking Controls must hide .editor-preview on mobile");
        assert.assertOk(controlsState.sidebarTop < 350, `Controls view must bring .editor-sidebar to top (got ${controlsState.sidebarTop})`);
        assert.assertOk(!(await measureOverflow(mobRes.page)).hasHorizontalOverflow, "Mobile controls view must have 0 overflow");

        // Toggle to Live Preview -> sidebar hides, iframe visible near top
        await mobRes.page.locator('[data-mobile-view="preview"]').click();
        await mobRes.page.waitForTimeout(150);
        const previewState = await mobRes.page.evaluate(() => {
          const sidebar = document.querySelector(".editor-sidebar");
          const iframe = document.querySelector("#design-preview");
          return {
            sidebarHidden: window.getComputedStyle(sidebar).display === "none",
            iframeTop: Math.round(iframe.getBoundingClientRect().top),
          };
        });
        assert.assertOk(previewState.sidebarHidden, "Clicking Live Preview must hide .editor-sidebar on mobile");
        assert.assertOk(previewState.iframeTop < 350, `Live Preview view must bring #design-preview to top (got ${previewState.iframeTop})`);
        assert.assertOk(!(await measureOverflow(mobRes.page)).hasHorizontalOverflow, "Mobile preview view must have 0 overflow");
        assert.assertEqual(mobRes.serverErrors.length, 0);
        assert.assertEqual(mobRes.pageErrors.length, 0);
      } finally {
        await mobRes.close();
      }

      // Desktop (1440x900) keeps switcher hidden and 2-column workspace intact
      const deskRes = await browserMgr.openPage(`/${ctx.adminPath}/tasarim`, {
        client: sa.client,
        viewport: VIEWPORT_DESKTOP,
      });
      try {
        assert.assertEqual(deskRes.status, 200);
        await deskRes.page.locator("#design-preview").waitFor({ state: "visible", timeout: 15000 });
        const deskLayout = await deskRes.page.evaluate(() => {
          const switcher = document.querySelector(".mobile-studio-switcher");
          const sidebar = document.querySelector(".editor-sidebar");
          const preview = document.querySelector(".editor-preview");
          return {
            switcherHidden: window.getComputedStyle(switcher).display === "none",
            sidebarLeft: Math.round(sidebar.getBoundingClientRect().left),
            previewLeft: Math.round(preview.getBoundingClientRect().left),
          };
        });
        assert.assertOk(deskLayout.switcherHidden, "Mobile studio switcher must be hidden on Desktop 1440x900");
        assert.assertOk(deskLayout.previewLeft > deskLayout.sidebarLeft, "Desktop must retain side-by-side 2-column workspace");
        assert.assertOk(!(await measureOverflow(deskRes.page)).hasHorizontalOverflow, "Desktop Design Studio must have 0 overflow");
        assert.assertEqual(deskRes.serverErrors.length, 0);
        assert.assertEqual(deskRes.pageErrors.length, 0);
      } finally {
        await deskRes.close();
      }
    });

    console.log(`\nTier 5 Playwright UI/UX Summary: ${results.length - failed}/${results.length} passed`);
    if (failed > 0) {
      process.exitCode = 1;
    }
  } finally {
    await browserMgr.close();
    await srv.stop();
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
