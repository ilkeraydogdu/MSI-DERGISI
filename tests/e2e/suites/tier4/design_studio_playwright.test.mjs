import {
  auditUiSurface,
  checkInteractiveVisibilityAndLayout,
  openDomainPage,
  withDesignConfigBackup,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 Visual Design Studio: Live Playwright Desktop & Mobile Suite", () => {
    suite.test(
      "T4-11: Design Studio (/${adminPath}/tasarim) renders cleanly on Desktop & Mobile, switches tabs & loads #design-preview iframe",
      { req: ["REQ-037", "REQ-005"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
            client: sa.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(
              pageRes.status,
              200,
              `Design Studio (${vp.width}x${vp.height}) must return 200`
            );
            await pageRes.page.locator("#design-preview").waitFor({ state: "visible", timeout: 15000 });
            await pageRes.page.waitForFunction(
              () => {
                const frame = document.querySelector("#design-preview");
                return Boolean(frame && frame.contentDocument && frame.contentDocument.body && frame.contentDocument.body.innerText.trim().length > 0);
              },
              { timeout: 15000 }
            );

            await auditUiSurface(c, pageRes, {
              label: `Design Studio (${vp.width}x${vp.height})`,
              requireMain: false,
              requireHeading: false,
            });

            // Switch across pages -> theme -> navigation tabs
            for (const tabKey of ["theme", "navigation", "pages"]) {
              const tabBtn = `[data-editor-tab="${tabKey}"]`;
              await checkInteractiveVisibilityAndLayout(c, pageRes.page, tabBtn, {
                label: `Design Studio tab ${tabKey} (${vp.width})`,
              });
              await pageRes.page.locator(tabBtn).click();
              await pageRes.page.waitForTimeout(100);
              await pageRes.page.locator(`#${tabKey}-panel`).waitFor({ state: "visible", timeout: 5000 });
              assert.assertNoOverflow(
                await pageRes.measureOverflow(),
                `Design Studio overflow on tab ${tabKey} (${vp.width})`
              );
            }

            assert.assertNoBrowserErrors(pageRes, `Design Studio (${vp.width}) browser errors`);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-12: Design Studio interactive preview viewport (390px/100%), theme edit, discard draft & save/restore round-trip",
      { req: ["REQ-037", "REQ-005"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();

        await withDesignConfigBackup(sa.client, async () => {
          const pageRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
            client: sa.client,
            viewport: c.browser.VIEWPORT_DESKTOP,
          });
          try {
            await pageRes.page.waitForFunction(
              () => {
                const frame = document.querySelector("#design-preview");
                return Boolean(frame && frame.contentDocument && frame.contentDocument.body?.innerText?.trim());
              },
              null,
              { timeout: 15000 }
            );

            // Toggle preview viewport 390px -> 100% (wait for 220ms CSS width transition)
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-preview-width="390px"]');
            await pageRes.page.locator('[data-preview-width="390px"]').click();
            await pageRes.page.waitForFunction(
              () => {
                const el = document.querySelector("#design-preview");
                return Boolean(el && el.getBoundingClientRect().width <= 410);
              },
              null,
              { timeout: 5000 }
            );
            const mobileFrameWidth = await pageRes.page
              .locator("#design-preview")
              .evaluate((el) => Math.round(el.getBoundingClientRect().width));
            assert.assertOk(
              mobileFrameWidth <= 410,
              `Expected #design-preview width ~390px after clicking 390px toggle, got ${mobileFrameWidth}px`
            );
            await pageRes.page.locator('[data-preview-width="100%"]').click();

            // Edit theme radius -> verify dirty state -> wait for 700ms debounced previewTimer + explicit #preview-button -> save -> assert exact persisted config.theme.radius
            await pageRes.page.locator('[data-editor-tab="theme"]').click();
            await pageRes.page.locator("#theme-radius").waitFor({ state: "visible" });
            const origRadius = await pageRes.page.locator("#theme-radius").inputValue();
            const testRadius = origRadius === "14" ? "10" : "14";
            await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/tasarim/onizleme") && r.request().method() === "POST" && r.status() === 200,
                { timeout: 10000 }
              ),
              (async () => {
                await pageRes.page.locator("#theme-radius").fill(testRadius);
                await pageRes.page.locator("#theme-radius").dispatchEvent("input");
              })(),
            ]);

            const dirtyAfterEdit = await pageRes.page.locator("#save-state").getAttribute("data-dirty");
            assert.assertEqual(dirtyAfterEdit, "true", "Editing #theme-radius must mark #save-state dirty");

            await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/tasarim/onizleme") && r.request().method() === "POST" && r.status() === 200,
                { timeout: 10000 }
              ),
              pageRes.page.locator("#preview-button").click(),
            ]);

            await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/api/v1/admin/design") && r.request().method() === "POST" && r.status() === 200,
                { timeout: 10000 }
              ),
              pageRes.page.locator("#save-button").click(),
            ]);
            await pageRes.page.waitForFunction(
              () => {
                const fb = document.querySelector("#editor-feedback");
                const pill = document.querySelector("#save-state");
                return (
                  fb?.dataset?.state === "success" &&
                  pill?.dataset?.dirty === "false"
                );
              },
              null,
              { timeout: 10000 }
            );

            const afterSave = await sa.client.get("/api/v1/admin/design");
            assert.assertStatus(afterSave, 200);
            assert.assertEqual(
              afterSave.json?.config?.theme?.radius,
              Number(testRadius),
              `Saved Design Studio config.theme.radius must equal ${testRadius} (got ${afterSave.json?.config?.theme?.radius})`
            );

            // Edit radius again -> click #discard-draft -> verify UI & saved state revert cleanly
            const discardTestRadius = testRadius === "14" ? "6" : "14";
            await pageRes.page.locator("#theme-radius").fill(discardTestRadius);
            await pageRes.page.locator("#theme-radius").dispatchEvent("input");
            assert.assertEqual(
              await pageRes.page.locator("#save-state").getAttribute("data-dirty"),
              "true",
              "Second edit before discard must mark #save-state dirty"
            );

            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#discard-draft");
            await pageRes.page.locator("#discard-draft").click();
            await pageRes.page.locator("#studio-confirm-accept").waitFor({ state: "visible", timeout: 5000 });
            await pageRes.page.locator("#studio-confirm-accept").click();
            await pageRes.page.waitForFunction(
              (expected) => {
                const pill = document.querySelector("#save-state");
                const inp = document.querySelector("#theme-radius");
                return pill?.dataset?.dirty === "false" && inp?.value === String(expected);
              },
              testRadius,
              { timeout: 10000 }
            );

            const afterDiscard = await sa.client.get("/api/v1/admin/design");
            assert.assertStatus(afterDiscard, 200);
            assert.assertEqual(
              afterDiscard.json?.config?.theme?.radius,
              Number(testRadius),
              "#discard-draft must leave persisted server config.theme.radius unchanged"
            );
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        });
      }
    );

    suite.test(
      "T4-13: Design Studio RBAC allows editor_in_chief (settings:read) and denies content_editor / ads_manager (403)",
      { req: ["REQ-037", "REQ-033"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const editor = await c.auth.createUserWithRole("content_editor");
        const adsMgr = await c.auth.createUserWithRole("ads_manager");

        const eicStudio = await eic.client.get(`/${c.adminPath}/tasarim`);
        assert.assertStatus(eicStudio, 200, "editor_in_chief with settings:read must access Design Studio");

        const editorStudio = await editor.client.get(`/${c.adminPath}/tasarim`);
        assert.assertStatus(editorStudio, 403, "content_editor must be denied Design Studio (403)");

        const adsStudio = await adsMgr.client.get(`/${c.adminPath}/tasarim`);
        assert.assertStatus(adsStudio, 403, "ads_manager must be denied Design Studio (403)");

        // Verify #editor-admin-link navigates back to Command Center
        const pageRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: eic.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#editor-admin-link");
          const href = await pageRes.page.locator("#editor-admin-link").getAttribute("href");
          assert.assertIncludes(href || "", c.adminPath, "#editor-admin-link must point to Admin panel");
          assert.assertNoBrowserErrors(pageRes);
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T4-33: Design Studio #studio-theme-toggle, 768px tablet preview, #preview-theme, #open-page & mobile .mobile-studio-switcher",
      { req: ["F2", "F5", "REQ-037"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();

        // Desktop (1440x900)
        const deskRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await deskRes.page.waitForFunction(
            () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
            { timeout: 15000 }
          );

          const beforeTheme = await deskRes.page.locator("html").getAttribute("data-theme");
          await deskRes.page.locator("#studio-theme-toggle").click();
          await deskRes.page.waitForTimeout(150);
          const afterTheme = await deskRes.page.locator("html").getAttribute("data-theme");
          assert.assertNotEqual(afterTheme, beforeTheme, "#studio-theme-toggle must flip html[data-theme]");

          // Tablet 768px preview width
          await deskRes.page.locator('[data-preview-width="768px"]').click();
          await deskRes.page.waitForFunction(
            () => {
              const el = document.querySelector("#design-preview");
              const w = el ? el.getBoundingClientRect().width : 0;
              return w >= 720 && w <= 790;
            },
            { timeout: 5000 }
          );

          // #preview-theme toggles preview iframe theme
          await deskRes.page.locator("#preview-theme").click();
          await deskRes.page.waitForTimeout(200);
          const iframeTheme = await deskRes.page
            .locator("#design-preview")
            .evaluate((f) => f.contentDocument?.documentElement?.getAttribute("data-theme"));
          assert.assertEqual(iframeTheme, "dark", "#preview-theme must switch iframe preview to dark");

          // #open-page points to valid href
          const openHref = await deskRes.page.locator("#open-page").getAttribute("href");
          assert.assertOk(openHref && openHref.startsWith("/"), `#open-page must have valid href, got ${openHref}`);
          assert.assertNoAssetOrNetworkErrors(deskRes, [], "T4-33 Desktop Design Studio");
        } finally {
          await deskRes.close();
        }

        // Mobile (390x844) .mobile-studio-switcher
        const mobRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_MOBILE,
        });
        try {
          await mobRes.page.locator(".mobile-studio-switcher").waitFor({ state: "visible", timeout: 8000 });
          await mobRes.page.locator("#mobile-view-preview").click();
          await mobRes.page.waitForTimeout(120);
          assert.assertEqual(await mobRes.page.locator("#mobile-view-preview").getAttribute("aria-pressed"), "true");
          assert.assertNoOverflow(await mobRes.measureOverflow(), "T4-33 Mobile preview view overflow");

          await mobRes.page.locator("#mobile-view-controls").click();
          await mobRes.page.waitForTimeout(120);
          assert.assertEqual(await mobRes.page.locator("#mobile-view-controls").getAttribute("aria-pressed"), "true");
          assert.assertNoOverflow(await mobRes.measureOverflow(), "T4-33 Mobile controls view overflow");
          assert.assertNoAssetOrNetworkErrors(mobRes, [], "T4-33 Mobile Design Studio");
        } finally {
          await mobRes.close();
        }
      }
    );

    suite.test(
      "T4-40: Design Studio read-only UX for editor_in_chief (settings:read) and #studio-confirm-modal Tab focus-trap",
      { req: ["F5", "REQ-037"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const eicRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: eic.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await eicRes.page.waitForFunction(
            () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
            { timeout: 15000 }
          );
          const uiState = await eicRes.page.evaluate(() => {
            const btn = document.getElementById("save-button");
            const banner = document.getElementById("studio-readonly-banner");
            return {
              canSaveAttr: document.body.dataset.canSave,
              saveDisabled: Boolean(btn?.disabled),
              saveAriaDisabled: btn?.getAttribute("aria-disabled"),
              bannerHidden: Boolean(banner?.hidden),
            };
          });
          assert.assertEqual(uiState.canSaveAttr, "false");
          assert.assertEqual(uiState.saveDisabled, true, "editor_in_chief #save-button must be disabled");
          assert.assertEqual(uiState.saveAriaDisabled, "true");
          assert.assertEqual(uiState.bannerHidden, false, "editor_in_chief must see #studio-readonly-banner");

          await Promise.all([
            eicRes.page.waitForResponse(
              (r) => r.url().includes("/tasarim/onizleme") && r.request().method() === "POST" && r.status() === 200,
              { timeout: 10000 }
            ),
            eicRes.page.locator("#preview-button").click(),
          ]);
          assert.assertNoOverflow(await eicRes.measureOverflow(), "T4-40 editor_in_chief overflow");
          assert.assertNoBrowserErrors(eicRes, "T4-40 editor_in_chief runtime/HTTP errors");
        } finally {
          await eicRes.close();
        }

        // #studio-confirm-modal Tab / Shift+Tab focus trap
        const sa = await c.auth.loginAsSuperAdmin();
        const saRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await saRes.page.waitForFunction(
            () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
            { timeout: 15000 }
          );
          await saRes.page.locator("#studio-tools-btn").click();
          await saRes.page.locator("#studio-tools-menu").waitFor({ state: "visible", timeout: 5000 });
          await saRes.page.locator("#restore-defaults").click();
          await saRes.page.locator("#studio-confirm-modal").waitFor({ state: "visible", timeout: 5000 });

          // Focus last button (#studio-confirm-accept), press Tab -> wraps inside #studio-confirm-modal
          await saRes.page.locator("#studio-confirm-accept").focus();
          await saRes.page.keyboard.press("Tab");
          const insideForward = await saRes.page.evaluate(() =>
            Boolean(document.getElementById("studio-confirm-modal")?.contains(document.activeElement))
          );
          assert.assertOk(insideForward, "Tab from #studio-confirm-accept must stay inside #studio-confirm-modal");

          await saRes.page.keyboard.press("Shift+Tab");
          const insideBackward = await saRes.page.evaluate(() =>
            Boolean(document.getElementById("studio-confirm-modal")?.contains(document.activeElement))
          );
          assert.assertOk(insideBackward, "Shift+Tab inside #studio-confirm-modal must stay inside modal");

          await saRes.page.keyboard.press("Escape");
          await saRes.page.locator("#studio-confirm-modal").waitFor({ state: "hidden", timeout: 5000 });
          assert.assertNoBrowserErrors(saRes, "T4-40 super_admin modal trap errors");
        } finally {
          await saRes.close();
        }
      }
    );
  });
}
