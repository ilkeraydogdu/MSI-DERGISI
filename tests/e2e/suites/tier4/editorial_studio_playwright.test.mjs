import {
  auditUiSurface,
  checkInteractiveVisibilityAndLayout,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

async function waitForEditorialStudioReady(page) {
  await page.locator("#hf-header-slot").waitFor({ state: "visible", timeout: 20000 });
  await page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
  await page.locator("#btn-new-article").waitFor({ state: "visible", timeout: 15000 });
  await page.waitForTimeout(200);
}

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 Editorial Studio: Live Playwright WASM UI (Desktop & Mobile)", () => {
    suite.test(
      "T4-42: Editorial Studio boots WASM workspace cleanly on Desktop (1440x900) & Mobile (390x844) with 0 >=400 errors & 0 overflow",
      { req: ["REQ-042", "REQ-004"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, `/${c.adminPath}/editor`, {
            client: chief.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200, `Editorial Studio (${vp.width}) must return 200`);
            await waitForEditorialStudioReady(pageRes.page);
            await auditUiSurface(c, pageRes, {
              label: `Editorial Studio (${vp.width}x${vp.height})`,
              requireMain: false,
              requireHeading: false,
            });

            // Verify theme toggle works if visible on desktop
            if (vp.width >= 1024) {
              await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#btn-msi-theme-toggle");
              const t0 = await pageRes.page.locator("html").getAttribute("data-theme");
              await pageRes.page.locator("#btn-msi-theme-toggle").click();
              await pageRes.page.waitForTimeout(150);
              const t1 = await pageRes.page.locator("html").getAttribute("data-theme");
              assert.assertNotEqual(t1, t0, "#btn-msi-theme-toggle must flip html[data-theme]");
              assert.assertNoOverflow(
                await pageRes.measureOverflow(),
                `Editorial Studio overflow after theme toggle (${vp.width})`
              );
            }

            assert.assertNoAssetOrNetworkErrors(
              pageRes,
              [],
              `Editorial Studio (${vp.width}x${vp.height}) network/console errors`
            );
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-43: Interactive WASM article creation via #btn-new-article, wizard steps & X-CSRF-Token persistence across reload",
      { req: ["REQ-042"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const title = `WASM E2E Makale ${uid("wasm")}`;
        let createdArticleId = null;

        const pageRes = await openDomainPage(c, `/${c.adminPath}/editor`, {
          client: chief.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await waitForEditorialStudioReady(pageRes.page);

          await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#btn-new-article");
          await pageRes.page.locator("#btn-new-article").click();
          await pageRes.page.locator("#editor-title-textarea").waitFor({ state: "visible", timeout: 8000 });

          await pageRes.page.locator("#editor-title-textarea").fill(title);
          await pageRes.page.locator("#editor-title-textarea").dispatchEvent("input");
          await pageRes.page.locator("#editor-spot-textarea").fill("WASM stüdyo spot özeti.");
          await pageRes.page.locator("#editor-spot-textarea").dispatchEvent("input");

          // Advance wizard steps 1 -> 2 -> 3 -> 4
          for (const targetStep of [2, 3, 4]) {
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#btn-wizard-next");
            await pageRes.page.locator("#btn-wizard-next").click();
            await pageRes.page.waitForTimeout(120);
            if (targetStep === 4) {
              await pageRes.page
                .locator('[data-action="save-draft"], [data-action="publish-now"]')
                .first()
                .waitFor({ state: "visible", timeout: 8000 });
            }
          }

          // Save draft via WASM fetch (verifies X-CSRF-Token cookie header)
          const [saveResp] = await Promise.all([
            pageRes.page.waitForResponse(
              (r) =>
                r.url().includes("/api/v1/admin/editorial/articles") &&
                r.request().method() === "POST",
              { timeout: 15000 }
            ),
            pageRes.page.locator('[data-action="save-draft"]').first().click(),
          ]);
          assert.assertEqual(saveResp.status(), 200, "WASM draft save must return HTTP 200");
          const savedJson = await saveResp.json();
          createdArticleId = savedJson?.id || null;
          assert.assertOk(Boolean(createdArticleId), "WASM draft save must return article id");

          // Let post-save lock acquisition & list refresh settle before reload
          await pageRes.page.waitForTimeout(400);

          // Reload page, switch to Drafts tab, verify article persists in left panel
          await pageRes.page.reload({ waitUntil: "networkidle" });
          await waitForEditorialStudioReady(pageRes.page);
          await pageRes.page.locator("#btn-left-tab-drafts").click();
          await pageRes.page.waitForTimeout(250);
          const leftText = await pageRes.page.locator("#left-scroll").innerText();
          assert.assertIncludes(
            leftText,
            title,
            "Saved draft must appear under #btn-left-tab-drafts after page reload"
          );

          assert.assertNoAssetOrNetworkErrors(pageRes, [], "T4-43 WASM save & reload");
        } finally {
          await pageRes.close();
          if (createdArticleId) {
            await chief.client.delete(`/api/v1/admin/editorial/articles/${createdArticleId}`).catch(() => {});
          }
        }
      }
    );

    suite.test(
      "T4-44: Editorial Studio Research tab, Settings popover (Kategoriler/Kaynaklar), Media modal & Revisions modal render cleanly",
      { req: ["REQ-042"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const slug = `t4-ed-tabs-${uid("t")}`;
        const cr = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Modal Test Makale ${slug}`,
            content: "<p>Revizyon testi içeriği.</p>",
            status: "draft",
            slug,
          },
        });
        assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        const pageRes = await openDomainPage(c, `/${c.adminPath}/editor?content_id=${artId}`, {
          client: chief.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await waitForEditorialStudioReady(pageRes.page);

          // 1. Open Revisions modal
          const revBtn = pageRes.page.locator('[data-action="open-revisions-modal"]').first();
          await revBtn.waitFor({ state: "visible", timeout: 10000 });
          await revBtn.click();
          await pageRes.page.waitForTimeout(250);
          const modalText = await pageRes.page.locator("#hf-modals-slot").innerText();
          assert.assertMatch(
            modalText.toLocaleLowerCase("tr-TR"),
            /revizyon/,
            "Revisions modal must open"
          );
          await pageRes.page.keyboard.press("Escape");
          await pageRes.page.waitForTimeout(150);

          // 2. Open Settings popover & inspect Sources subtab
          await pageRes.page.locator("#btn-settings-trigger").click();
          await pageRes.page.locator("#hf-settings-popover").waitFor({ state: "visible", timeout: 8000 });
          await pageRes.page.locator("#btn-settings-tab-sources").click();
          await pageRes.page.waitForTimeout(150);
          assert.assertOk(
            await pageRes.page.locator("#hf-new-source-url").isVisible(),
            "Settings Sources tab must expose #hf-new-source-url"
          );
          await pageRes.page.keyboard.press("Escape");
          await pageRes.page.waitForTimeout(150);

          // 3. Switch to Research tab
          await pageRes.page.locator("#btn-left-tab-research").click();
          await pageRes.page.waitForTimeout(200);
          assert.assertNoOverflow(await pageRes.measureOverflow(), "T4-44 Research tab overflow");

          // 4. Open Step 2 Media modal
          await pageRes.page.locator('[data-action="set-wizard-step"][data-step="2"]').click();
          await pageRes.page.locator('[data-action="open-media-library"]').first().click();
          await pageRes.page.waitForTimeout(250);
          assert.assertNoOverflow(await pageRes.measureOverflow(), "T4-44 Media modal overflow");

          const allowedExternalResearchImgs = !c.isExternalTarget
            ? [
                (val, url) =>
                  Boolean(url && /^https?:\/\//i.test(url) && !url.startsWith(c.baseUrl)),
                "ERR_CONNECTION_REFUSED",
              ]
            : [];
          assert.assertNoAssetOrNetworkErrors(
            pageRes,
            allowedExternalResearchImgs,
            "T4-44 Editorial Studio interactive tabs/modals"
          );
        } finally {
          await pageRes.close();
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "T4-45: Concurrent lock conflict banner (.hf-conflict-banner) appears in second operator's Editorial Studio browser",
      { req: ["REQ-042"] },
      async (c) => {
        const userA = await c.auth.createUserWithRole("content_editor");
        const userB = await c.auth.createUserWithRole("editor_in_chief");
        const slug = `t4-ed-lock-${uid("l")}`;

        const cr = await userA.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Kilit Banner Testi ${slug}`,
            content: "<p>Eşzamanlı düzenleme testi.</p>",
            status: "draft",
            slug,
          },
        });
        assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        // User A opens deep-linked article -> acquires lock
        const pageA = await openDomainPage(c, `/${c.adminPath}/editor?content_id=${artId}`, {
          client: userA.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        let pageB = null;
        try {
          await waitForEditorialStudioReady(pageA.page);
          await pageA.page.waitForTimeout(400);

          // User B opens same deep-linked article -> must display .hf-conflict-banner
          pageB = await openDomainPage(c, `/${c.adminPath}/editor?content_id=${artId}`, {
            client: userB.client,
            viewport: c.browser.VIEWPORT_DESKTOP,
          });
          await waitForEditorialStudioReady(pageB.page);
          await pageB.page.locator(".hf-conflict-banner").waitFor({ state: "visible", timeout: 12000 });
          const bannerText = await pageB.page.locator(".hf-conflict-banner").innerText();
          assert.assertIncludes(bannerText, "düzenleniyor");

          assert.assertNoAssetOrNetworkErrors(pageA, [], "T4-45 User A");
          assert.assertNoAssetOrNetworkErrors(pageB, [], "T4-45 User B");
        } finally {
          if (pageB) await pageB.close();
          await pageA.close();
          await userB.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`).catch(() => {});
          await userB.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "T4-46: Command Center sidebar links to /{adminPath}/editor and ads_manager sees styled 403 page on /{adminPath}/editor",
      { req: ["REQ-042", "REQ-037", "REQ-033"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const adsMgr = await c.auth.createUserWithRole("ads_manager");

        // 1. Command Center sidebar link -> Editorial Studio
        const cmdRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: chief.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await cmdRes.page.waitForFunction(
            () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
            { timeout: 15000 }
          );
          const editorLink = cmdRes.page.locator(`#admin-sidebar a[href$="/editor"]`).first();
          await editorLink.waitFor({ state: "visible", timeout: 8000 });
          const href = await editorLink.getAttribute("href");
          assert.assertIncludes(href || "", `/${c.adminPath}/editor`);

          await Promise.all([
            cmdRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            editorLink.click(),
          ]);
          await waitForEditorialStudioReady(cmdRes.page);
          assert.assertIncludes(cmdRes.page.url(), `/${c.adminPath}/editor`);
          assert.assertNoAssetOrNetworkErrors(cmdRes, ["/api/v1/admin/monitoring/bundle"], "T4-46 Command Center -> Studio");
        } finally {
          await cmdRes.close();
        }

        // 2. ads_manager sees styled 403 forbidden page on Desktop & Mobile without JS errors or overflow
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const adsRes = await openDomainPage(c, `/${c.adminPath}/editor`, {
            client: adsMgr.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(adsRes.status, 403, `ads_manager GET /editor (${vp.width}) must return 403`);
            await adsRes.page.locator("section.msi-studio-forbidden").waitFor({ state: "visible", timeout: 8000 });
            const badgeText = await adsRes.page.locator(".msi-studio-badge-err").innerText();
            assert.assertIncludes(badgeText, "403");
            assert.assertNoOverflow(await adsRes.measureOverflow(), `ads_manager 403 overflow (${vp.width})`);
            assert.assertNoAssetOrNetworkErrors(
              adsRes,
              [`/${c.adminPath}/editor`, "status of 403"],
              `ads_manager 403 (${vp.width})`
            );
          } finally {
            await adsRes.close();
          }
        }
      }
    );
  });
}
