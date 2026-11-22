import {
  assertModalGeometry,
  assertUiTokensPresent,
} from "../../fixtures/design_tokens_contract.mjs";
import {
  createVerifiedSubscriber,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 3 Cross-Surface UX & Design Token Alignment Flows", () => {
    suite.test(
      "T3-U1: Global header [data-cart-toggle] opens aside#cart-drawer on / and /abonelik-ve-satis add-to-cart syncs [data-cart-count] across navigation",
      { req: ["F3", "R2"] },
      async (c) => {
        const pageRes = await openDomainPage(c, "/", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200, "Home / must return 200");

          // Global header cart trigger must exist & open aside#cart-drawer on /
          const headerToggle = pageRes.page.locator(".site-header [data-cart-toggle], header [data-cart-toggle]").first();
          await headerToggle.waitFor({ state: "visible", timeout: 8000 });
          await headerToggle.click();
          await pageRes.page.locator("aside#cart-drawer").waitFor({ state: "visible", timeout: 8000 });
          await pageRes.page.locator("aside#cart-drawer [data-cart-close]").first().click();
          await pageRes.page.locator("aside#cart-drawer").waitFor({ state: "hidden", timeout: 8000 });

          // Navigate to /abonelik-ve-satis and add a product to cart
          await pageRes.page.goto(`${c.baseUrl}/abonelik-ve-satis`, { waitUntil: "networkidle" });
          const addBtn = pageRes.page.locator("button[data-add-to-cart]").first();
          await addBtn.waitFor({ state: "visible", timeout: 8000 });
          await addBtn.click();
          await pageRes.page.waitForTimeout(200);

          const countOnStore = await pageRes.page
            .locator(".site-header [data-cart-count], [data-cart-count]")
            .first()
            .innerText();
          assert.assertOk(
            Number.parseInt(countOnStore.trim(), 10) >= 1,
            `Expected [data-cart-count] >= 1 after add-to-cart, got "${countOnStore}"`
          );

          // Navigate to /e-dergi and confirm [data-cart-count] persists across pages
          await pageRes.page.goto(`${c.baseUrl}/e-dergi`, { waitUntil: "networkidle" });
          const countOnEdergi = await pageRes.page
            .locator(".site-header [data-cart-count], [data-cart-count]")
            .first()
            .innerText();
          assert.assertOk(
            Number.parseInt(countOnEdergi.trim(), 10) >= 1,
            `Expected [data-cart-count] >= 1 on /e-dergi after navigation, got "${countOnEdergi}"`
          );

          assert.assertNoBrowserErrors(pageRes, "T3-U1 cross-page cart flow errors");
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-U2: /hesabim language switch (/switch/en <-> /switch/tr) preserves active tab (security & notifications)",
      { req: ["F5", "F7", "R2"] },
      async (c) => {
        const reader = await createVerifiedSubscriber(c, "registered");
        const pageRes = await openDomainPage(c, "/hesabim", {
          client: reader.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);

          // Switch to security tab and then switch language to EN
          await pageRes.page.locator('[data-ap-tab="security"]').first().click();
          await pageRes.page.locator('#ap-panel-security').waitFor({ state: "visible", timeout: 8000 });

          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
            pageRes.page.locator('a.ap-util-btn[href*="/switch/en"]').first().click(),
          ]);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "en");
          const securityVisibleAfterEn = await pageRes.page.locator("#ap-panel-security").isVisible();
          assert.assertOk(
            securityVisibleAfterEn,
            "/hesabim security tab must remain active after switching language to EN"
          );

          // Switch to notifications tab and then switch language back to TR
          await pageRes.page.locator('[data-ap-tab="notifications"]').first().click();
          await pageRes.page.locator('#ap-panel-notifications').waitFor({ state: "visible", timeout: 8000 });

          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
            pageRes.page.locator('a.ap-util-btn[href*="/switch/tr"]').first().click(),
          ]);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "tr");
          const notifVisibleAfterTr = await pageRes.page.locator("#ap-panel-notifications").isVisible();
          assert.assertOk(
            notifVisibleAfterTr,
            "/hesabim notifications tab must remain active after switching language to TR"
          );

          assert.assertNoBrowserErrors(pageRes, "T3-U2 /hesabim tab preservation errors");
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-U3: /hesabim mark-all-read updates #ap-notif-badge & #ap-kpi-unread, and #account-delete-form opens #account-delete-confirm-dialog",
      { req: ["F8", "R2"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const reader = await createVerifiedSubscriber(c, "registered");

        const notifRes = await sa.client.post("/api/v1/admin/notifications", {
          json: {
            title_tr: `T3 Bildirim ${uid("n")}`,
            title_en: `T3 Notice ${uid("n")}`,
            message_tr: "Okunmamış bildirim doğrulaması.",
            message_en: "Unread notification verification.",
            target_audience: "all",
          },
        });
        assert.assertStatus(notifRes, 200, "Admin notification seed must return 200");

        const pageRes = await openDomainPage(c, "/hesabim", {
          client: reader.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);

          const initialUnread = Number.parseInt(
            (await pageRes.page.locator("#ap-kpi-unread").innerText()).trim(),
            10
          );
          assert.assertOk(initialUnread >= 1, `Expected initial #ap-kpi-unread >= 1, got ${initialUnread}`);

          // Mark all read inside Notifications tab
          await pageRes.page.locator('[data-ap-tab="notifications"]').first().click();
          await pageRes.page.locator("#ap-panel-notifications").waitFor({ state: "visible", timeout: 8000 });
          await pageRes.page.locator("[data-mark-all-read]").first().click();

          await pageRes.page.waitForFunction(
            () => {
              const kpi = document.querySelector("#ap-kpi-unread")?.textContent?.trim();
              const badge = document.querySelector("#ap-notif-badge");
              const badgeHidden = !badge || badge.hasAttribute("hidden") || badge.textContent?.trim() === "0";
              return kpi === "0" && badgeHidden;
            },
            { timeout: 10000 }
          );

          // Delete form must open #account-delete-confirm-dialog rather than deleting immediately
          await pageRes.page.locator('[data-ap-tab="security"]').first().click();
          await pageRes.page.locator("#ap-panel-security").waitFor({ state: "visible", timeout: 8000 });
          await pageRes.page
            .locator('#account-delete-form input[name="password"]')
            .fill(reader.password);
          const confirmErasure = pageRes.page.locator('#account-delete-form input[name="confirm_erasure"]');
          if (await confirmErasure.count()) {
            await confirmErasure.check();
          }
          await pageRes.page.locator('#account-delete-form button[type="submit"]').click();

          await assertModalGeometry(pageRes.page, "#account-delete-confirm-dialog", { minPadding: 16 });
          await pageRes.page.locator("#account-delete-cancel-btn").click();
          await pageRes.page.locator("#account-delete-confirm-dialog").waitFor({ state: "hidden", timeout: 5000 });

          // Confirm reader account is still alive after cancelling
          const exportCheck = await reader.client.get("/api/v1/account/export");
          assert.assertStatus(exportCheck, 200, "Account must remain intact after cancelling delete dialog");
          assert.assertNoBrowserErrors(pageRes, "T3-U3 /hesabim notifications & delete guard errors");
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-U4: Cross-surface Role A (Surface 1 <-> #auth-dialog <-> /hesabim) and Role B (/podmin <-> /podmin/editor <-> /podmin/tasarim) token alignment",
      { req: ["F1", "F2", "F6", "F9", "F12", "F13", "R1"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const reader = await createVerifiedSubscriber(c, "registered", { superAdminClient: sa.client });

        // Role A: Surface 1 (/), #auth-dialog (/hesabim guest), Surface 2 (/hesabim authed)
        const homeRes = await openDomainPage(c, "/", { viewport: c.browser.VIEWPORT_DESKTOP });
        try {
          await homeRes.setTheme("light");
          await assertUiTokensPresent(homeRes.page, { role: "brand", theme: "light" });
          await homeRes.setTheme("dark");
          await assertUiTokensPresent(homeRes.page, { role: "brand", theme: "dark" });
        } finally {
          await homeRes.close();
        }

        const guestHesabim = await openDomainPage(c, "/hesabim", { viewport: c.browser.VIEWPORT_DESKTOP });
        try {
          await guestHesabim.setTheme("light");
          await assertUiTokensPresent(guestHesabim.page, { role: "brand", theme: "light" });
          await guestHesabim.page.locator('a[href="#modal-login"]').first().click();
          await guestHesabim.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 6000 });
          const authSubmitBg = await guestHesabim.page
            .locator('#auth-dialog button[type="submit"]')
            .first()
            .evaluate((el) => window.getComputedStyle(el).backgroundColor);
          assert.assertNotIncludes(
            authSubmitBg,
            "rgb(59, 102, 196)",
            "#auth-dialog primary submit must adopt Role A crimson rather than detached #3b66c4 blue"
          );
        } finally {
          await guestHesabim.close();
        }

        const authedHesabim = await openDomainPage(c, "/hesabim", {
          client: reader.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await authedHesabim.setTheme("light");
          await assertUiTokensPresent(authedHesabim.page, { role: "brand", theme: "light" });
          const portalAccent = await authedHesabim.page.evaluate(() => {
            const btn = document.querySelector("#ap-panel-overview .ap-btn-primary, #ap-panel-profile button[type='submit']");
            return btn ? window.getComputedStyle(btn).backgroundColor : "";
          });
          assert.assertNotIncludes(
            portalAccent,
            "rgb(59, 102, 196)",
            "/hesabim primary button must adopt Role A editorial crimson rather than detached #3b66c4 blue"
          );
          await authedHesabim.setTheme("dark");
          await assertUiTokensPresent(authedHesabim.page, { role: "brand", theme: "dark" });
        } finally {
          await authedHesabim.close();
        }

        // Role B: Surface 3 (/podmin), Surface 4 (/podmin/editor), Surface 5 (/podmin/tasarim)
        const podminRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await podminRes.page.waitForFunction(
            () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
            { timeout: 15000 }
          );
          await podminRes.setTheme("light");
          await assertUiTokensPresent(podminRes.page, { role: "ops", theme: "light" });
          await podminRes.setTheme("dark");
          await assertUiTokensPresent(podminRes.page, { role: "ops", theme: "dark" });
        } finally {
          await podminRes.close();
        }

        const edRes = await openDomainPage(c, `/${c.adminPath}/editor`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await edRes.page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
          await edRes.setTheme("light");
          await assertUiTokensPresent(edRes.page, { role: "ops", theme: "light" });
          await edRes.setTheme("dark");
          await assertUiTokensPresent(edRes.page, { role: "ops", theme: "dark" });
        } finally {
          await edRes.close();
        }

        const studioRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await studioRes.page.waitForFunction(
            () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
            { timeout: 15000 }
          );
          await studioRes.setTheme("light");
          await assertUiTokensPresent(studioRes.page, { role: "ops", theme: "light" });
          await studioRes.setTheme("dark");
          await assertUiTokensPresent(studioRes.page, { role: "ops", theme: "dark" });
        } finally {
          await studioRes.close();
        }
      }
    );
  });
}
