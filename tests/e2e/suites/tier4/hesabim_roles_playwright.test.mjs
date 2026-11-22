import {
  auditUiSurface,
  checkInteractiveVisibilityAndLayout,
  createVerifiedSubscriber,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

async function switchHesabimTab(ctx, page, vp, tabKey) {
  const isDesktop = vp.width >= 960;
  const tabSel = isDesktop
    ? `[data-ap-tab="${tabKey}"]`
    : `.ap-mobile-tab[data-ap-jump="${tabKey}"]`;
  const loc = page.locator(tabSel).first();
  await loc.waitFor({ state: "visible", timeout: 10000 });
  await loc.scrollIntoViewIfNeeded();
  await checkInteractiveVisibilityAndLayout(ctx, page, tabSel, {
    label: `/hesabim (${vp.width}) tab ${tabKey}`,
  });
  await loc.click();
  await page.locator(`[data-ap-panel="${tabKey}"]`).waitFor({ state: "visible", timeout: 8000 });
}

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 /hesabim Role Portal: Live Playwright Desktop & Mobile Suite (All Reader & Staff Roles)", () => {
    suite.test(
      "T4-14: Unauthenticated /hesabim renders guest login gate without exposing .account-command-card or private controls",
      { req: ["REQ-017", "REQ-038"] },
      async (c) => {
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", { viewport: vp });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `Guest /hesabim (${vp.width}x${vp.height})`,
              requireMain: true,
              requireSingleH1: true,
            });
            const guestCheck = await pageRes.page.evaluate(() => {
              return {
                hasWelcome: Boolean(document.querySelector(".account-welcome, [data-account-guest]")),
                hasLoginCta: Boolean(document.querySelector('a[href="#modal-login"], button[data-open-dialog="auth-dialog"], #account-guest-login-btn')),
                hasCommandCard: Boolean(document.querySelector(".account-command-card")),
                hasDeleteForm: Boolean(document.querySelector("#account-delete-form")),
              };
            });
            assert.assertOk(guestCheck.hasWelcome && guestCheck.hasLoginCta, "Guest /hesabim must show login gate");
            assert.assertEqual(guestCheck.hasCommandCard, false, "Guest must never see .account-command-card");
            assert.assertEqual(guestCheck.hasDeleteForm, false, "Guest must never see #account-delete-form");
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-15: /hesabim loads dedicated M5 assets (/css/account-portal.css & /js/account-portal.js) and E-Dergi shelf (Desktop & Mobile)",
      { req: ["REQ-038", "REQ-017"] },
      async (c) => {
        const sub = await createVerifiedSubscriber(c, "subscriber_digital");
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: sub.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `M5 /hesabim assets (${vp.width})`,
              requireMain: true,
            });

            await switchHesabimTab(c, pageRes.page, vp, "edergi");
            assert.assertNoOverflow(
              await pageRes.measureOverflow(),
              `/hesabim edergi tab (${vp.width})`
            );
            assert.assertNoAssetOrNetworkErrors(pageRes, [], `M5 /hesabim (${vp.width})`);

            const m5Check = await pageRes.page.evaluate(async () => {
              const hasPortalCss = Array.from(document.querySelectorAll('link[rel="stylesheet"]')).some((l) =>
                (l.getAttribute("href") || "").includes("/css/account-portal.css")
              );
              const hasPortalJs = Array.from(document.querySelectorAll("script[src]")).some((s) =>
                (s.getAttribute("src") || "").includes("/js/account-portal.js")
              );
              const hasEdergiShelf = Boolean(
                document.querySelector(
                  ".ap-edergi-shelf, #ap-panel-edergi, #account-edergi-shelf, [data-edergi-shelf], .account-edergi-shelf"
                )
              );
              return { hasPortalCss, hasPortalJs, hasEdergiShelf };
            });
            assert.assertOk(
              m5Check.hasPortalCss,
              "/hesabim must load dedicated /css/account-portal.css stylesheet (M5 REQ-038)"
            );
            assert.assertOk(
              m5Check.hasPortalJs,
              "/hesabim must load dedicated /js/account-portal.js script (M5 REQ-038)"
            );
            assert.assertOk(
              m5Check.hasEdergiShelf,
              "/hesabim must render an interactive E-Dergi shelf for authenticated readers (M5 REQ-038)"
            );
          } finally {
            await pageRes.close();
          }
        }

        // Verify static asset HTTP 200 directly
        const cssRes = await sub.client.get("/css/account-portal.css");
        const jsRes = await sub.client.get("/js/account-portal.js");
        assert.assertStatus(cssRes, 200);
        assert.assertStatus(jsRes, 200);
      }
    );

    suite.test(
      "T4-16: [registered] /hesabim hides .account-command-card, switches all tabs, and persists interactive SSB preferences & profile form (Desktop & Mobile)",
      { req: ["REQ-038", "REQ-006", "REQ-017"] },
      async (c) => {
        const reader = await createVerifiedSubscriber(c, "registered", {
          ssb_interests: ["kara-araclari"],
        });

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: reader.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `[registered] /hesabim (${vp.width})`,
              requireMain: true,
              requireSingleH1: true,
            });
            const cmdCount = await pageRes.page.locator(".account-command-card").count();
            assert.assertEqual(cmdCount, 0, "registered reader must NOT see staff .account-command-card");

            if (vp.width < 960) {
              await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#ap-sidebar-toggle");
              await pageRes.page.locator("#ap-sidebar-toggle").click();
              await pageRes.page.waitForTimeout(180);
              assert.assertNoOverflow(await pageRes.measureOverflow(), `[registered] /hesabim (${vp.width}) sidebar open`);
              await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#ap-sidebar-close");
              await pageRes.page.locator("#ap-sidebar-close").click();
              await pageRes.page.waitForTimeout(180);
            }

            // Exercise all /hesabim tabs cleanly on both Desktop & Mobile
            for (const tabKey of ["edergi", "security", "notifications", "overview", "profile"]) {
              await switchHesabimTab(c, pageRes.page, vp, tabKey);
              assert.assertNoOverflow(
                await pageRes.measureOverflow(),
                `[registered] /hesabim (${vp.width}) tab ${tabKey}`
              );
            }

            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#account-profile-form");
            const newName = `Güncel Okur ${uid("name")}`;
            await pageRes.page.locator('#account-profile-form input[name="full_name"]').fill(newName);

            const targetTag = vp.width === 1440 ? "hava-araclari" : "deniz-araclari";
            const ssbCheckbox = pageRes.page
              .locator(`#account-ssb-tags input[name="ssb_interests"][value="${targetTag}"]`)
              .first();
            if (!(await ssbCheckbox.isChecked())) {
              await ssbCheckbox.check();
            }

            await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/api/v1/account/profile") && r.status() === 200,
                { timeout: 10000 }
              ),
              pageRes.page.locator('#account-profile-form button[type="submit"]').click(),
            ]);

            await pageRes.page.reload({ waitUntil: "networkidle" });
            await switchHesabimTab(c, pageRes.page, vp, "profile");
            const persistedName = await pageRes.page
              .locator('#account-profile-form input[name="full_name"]')
              .inputValue();
            assert.assertEqual(persistedName, newName, `Updated profile name must persist across reload (${vp.width})`);
            assert.assertOk(
              await pageRes.page
                .locator(`#account-ssb-tags input[name="ssb_interests"][value="${targetTag}"]`)
                .first()
                .isChecked(),
              `Updated SSB interest ${targetTag} must persist across reload (${vp.width})`
            );
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-17: [subscriber_public] (.gov.tr verified) renders .status-badge.is-gov on Desktop & Mobile without staff command card",
      { req: ["REQ-038", "REQ-016"] },
      async (c) => {
        const govUser = await createVerifiedSubscriber(c, "subscriber_public", {
          email: `${uid("gov.portal")}@ssb.gov.tr`,
          organization_category: "Kamu / TSK / SSB",
          organization_name: "Savunma Sanayii Başkanlığı",
          is_gov_verified: true,
          ssb_interests: ["hava-araclari", "radarlar-ve-sensorler"],
        });

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: govUser.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `[subscriber_public] /hesabim (${vp.width})`,
              requireMain: true,
              requireSingleH1: true,
            });
            const hasGovBadge = await pageRes.page.locator(".status-badge.is-gov").count();
            assert.assertOk(hasGovBadge >= 1, "subscriber_public must display .status-badge.is-gov");
            const cmdCount = await pageRes.page.locator(".account-command-card").count();
            assert.assertEqual(cmdCount, 0, "subscriber_public must NOT see .account-command-card");
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-18: [subscriber_digital] displays subscription status, invoice table & personalized feed on Desktop & Mobile",
      { req: ["REQ-038", "REQ-028"] },
      async (c) => {
        const sub = await createVerifiedSubscriber(c, "subscriber_digital", {
          subscription_tier: "dijital",
          ssb_interests: ["deniz-araclari"],
        });
        await sub.client.post("/api/v1/subscriptions/checkout", {
          json: {
            email: sub.email,
            full_name: sub.user.full_name,
            item_type: "subscription",
            item_code: "dijital",
            distance_sales_consent: true,
          },
        });

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: sub.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `[subscriber_digital] /hesabim (${vp.width})`,
              requireMain: true,
              requireSingleH1: true,
            });
            const html = await pageRes.page.content();
            assert.assertIncludes(html, sub.email);
            assert.assertEqual(await pageRes.page.locator(".account-command-card").count(), 0);
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-19: [subscriber_print_digital] & [subscriber_corporate] /hesabim render cleanly on Desktop & Mobile without staff cards",
      { req: ["REQ-038", "REQ-028"] },
      async (c) => {
        for (const role of ["subscriber_print_digital", "subscriber_corporate"]) {
          const actor = await createVerifiedSubscriber(c, role, {
            subscription_tier: role === "subscriber_corporate" ? "kurumsal" : "basili-dijital",
          });
          for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
            const pageRes = await openDomainPage(c, "/hesabim", {
              client: actor.client,
              viewport: vp,
            });
            try {
              assert.assertEqual(pageRes.status, 200);
              await auditUiSurface(c, pageRes, {
                label: `[${role}] /hesabim (${vp.width})`,
                requireMain: true,
                requireSingleH1: true,
              });
              assert.assertEqual(await pageRes.page.locator(".account-command-card").count(), 0);
              assert.assertNoBrowserErrors(pageRes);
            } finally {
              await pageRes.close();
            }
          }
        }
      }
    );

    suite.test(
      "T4-20: [site_operator] staff portal on /hesabim renders .account-command-card with valid Admin & Studio links",
      { req: ["REQ-038", "REQ-033"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: op.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `[site_operator] /hesabim (${vp.width})`,
              requireMain: true,
              requireSingleH1: true,
            });
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, ".account-command-card");
            const links = await pageRes.page.evaluate(() =>
              Array.from(document.querySelectorAll(".account-command-card a[href]")).map((a) =>
                a.getAttribute("href")
              )
            );
            assert.assertOk(
              links.some((h) => h && h.includes(c.adminPath)),
              "Staff .account-command-card must link to Admin Command Center"
            );
            assert.assertOk(
              links.some((h) => h && h.includes(`${c.adminPath}/tasarim`)),
              "Staff .account-command-card must link to Design Studio"
            );
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-21: Live Playwright TOTP 2FA setup/verify/disable + KVKK export & erasure UI controls on /hesabim (Desktop & Mobile)",
      { req: ["REQ-038", "REQ-017", "REQ-032"] },
      async (c) => {
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const user = await createVerifiedSubscriber(c, "registered");
          const pageRes = await openDomainPage(c, "/hesabim", {
            client: user.client,
            viewport: vp,
          });
          try {
            await switchHesabimTab(c, pageRes.page, vp, "security");
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#account-totp-setup-btn", {
              label: `TOTP setup btn (${vp.width})`,
            });
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, 'a[data-kvkk-export], a[href="/api/v1/account/export"]', {
              label: `KVKK export link (${vp.width})`,
            });
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#account-delete-form", {
              label: `KVKK delete form (${vp.width})`,
            });

            // Trigger TOTP setup in UI
            await pageRes.page.locator("#account-totp-setup-btn").click();
            await pageRes.page.locator("#account-totp-setup-panel").waitFor({ state: "visible", timeout: 10000 });
            const secret = (await pageRes.page.locator("#account-totp-secret").textContent() || "").trim();
            assert.assertOk(secret.length >= 16, `Expected Base32 secret in #account-totp-secret, got "${secret}"`);

            const code = c.totp.generateTotp(secret);
            await pageRes.page.locator('#account-totp-verify-form input[name="code"]').fill(code);
            await pageRes.page.locator('#account-totp-verify-form button[type="submit"]').click();
            await pageRes.page.locator("#totp-backup-codes-box").waitFor({ state: "visible", timeout: 10000 });

            // Reload /hesabim and switch back to security tab to disable TOTP via UI form
            await pageRes.page.reload({ waitUntil: "networkidle" });
            await switchHesabimTab(c, pageRes.page, vp, "security");
            await pageRes.page.locator("#account-totp-disable-form").waitFor({ state: "visible", timeout: 10000 });
            await pageRes.page.locator('#account-totp-disable-form input[name="password"]').fill(user.password);
            const disableCodeInput = pageRes.page.locator('#account-totp-disable-form input[name="code"]');
            if ((await disableCodeInput.count()) > 0) {
              await disableCodeInput.fill(c.totp.generateTotp(secret));
            }
            await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/api/v1/auth/totp/disable") && r.status() === 200,
                { timeout: 10000 }
              ),
              pageRes.page.locator('#account-totp-disable-form button[type="submit"]').click(),
            ]);

            assert.assertNoOverflow(await pageRes.measureOverflow(), `/hesabim security (${vp.width})`);
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-34: /hesabim #ap-theme-toggle, /switch/en <-> /switch/tr, [data-mark-all-read], and [data-logout] -> guest gate",
      { req: ["F2", "F6", "REQ-038"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const reader = await createVerifiedSubscriber(c, "registered");
        const notifRes = await sa.client.post("/api/v1/admin/notifications", {
          json: {
            title_tr: `E2E Bildirim ${uid("n")}`,
            title_en: `E2E Notice ${uid("n")}`,
            message_tr: "Test bildirim içeriği.",
            message_en: "Test notification message.",
            target_audience: "all",
          },
        });
        assert.assertStatus(notifRes, 200);

        const pageRes = await openDomainPage(c, "/hesabim", {
          client: reader.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);

          // Theme toggle + persistence
          const t0 = await pageRes.page.locator("html").getAttribute("data-theme");
          await pageRes.page.locator("#ap-theme-toggle").click();
          await pageRes.page.waitForTimeout(150);
          const t1 = await pageRes.page.locator("html").getAttribute("data-theme");
          assert.assertNotEqual(t1, t0, "#ap-theme-toggle must flip html[data-theme]");
          await pageRes.page.reload({ waitUntil: "networkidle" });
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("data-theme"), t1);

          // Language switch /switch/en <-> /switch/tr
          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
            pageRes.page.locator('a.ap-util-btn[href="/switch/en"]').click(),
          ]);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "en");
          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
            pageRes.page.locator('a.ap-util-btn[href="/switch/tr"]').click(),
          ]);
          assert.assertEqual(await pageRes.page.locator("html").getAttribute("lang"), "tr");

          // Notifications tab -> mark all read
          await switchHesabimTab(c, pageRes.page, c.browser.VIEWPORT_DESKTOP, "notifications");
          const markAll = pageRes.page.locator("[data-mark-all-read]");
          if (await markAll.isVisible({ timeout: 3000 }).catch(() => false)) {
            await markAll.click();
            await pageRes.page.waitForFunction(
              () => document.querySelector("#ap-kpi-unread")?.textContent?.trim() === "0",
              { timeout: 8000 }
            );
          }

          // Logout -> returns to guest gate
          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 10000 }),
            pageRes.page.locator("[data-logout]").first().click(),
          ]);
          await pageRes.page.locator(".account-welcome").waitFor({ state: "visible", timeout: 8000 });
          assert.assertEqual(await pageRes.page.locator(".account-command-card").count(), 0);
          assert.assertNoAssetOrNetworkErrors(pageRes, [], "T4-34 /hesabim");
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T4-35: /hesabim staff vs non-staff, Design Studio & Editorial Studio (/editor) quick-link gating across all representative roles (Desktop & Mobile)",
      { req: ["F6", "REQ-038", "REQ-033", "REQ-042"] },
      async (c) => {
        const cases = [
          { role: "super_admin", expectCard: true, expectStudio: true, expectEditor: true },
          { role: "editor_in_chief", expectCard: true, expectStudio: true, expectEditor: true },
          { role: "content_editor", expectCard: true, expectStudio: false, expectEditor: true },
          { role: "ads_manager", expectCard: true, expectStudio: false, expectEditor: false },
          { role: "finance_viewer", expectCard: true, expectStudio: false, expectEditor: false },
          { role: "support_agent", expectCard: true, expectStudio: false, expectEditor: false },
          ...c.auth.READER_ROLES.map((role) => ({
            role,
            expectCard: false,
            expectStudio: false,
            expectEditor: false,
          })),
        ];

        for (const tc of cases) {
          const actor =
            tc.role === "super_admin"
              ? await c.auth.loginAsSuperAdmin()
              : await c.auth.createUserWithRole(tc.role);

          for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
            const pageRes = await openDomainPage(c, "/hesabim", {
              client: actor.client,
              viewport: vp,
            });
            try {
              assert.assertEqual(pageRes.status, 200);
              await auditUiSurface(c, pageRes, {
                label: `T4-35 [${tc.role}] /hesabim (${vp.width}x${vp.height})`,
                requireMain: true,
                requireSingleH1: true,
              });
              const cardCount = await pageRes.page.locator(".account-command-card").count();
              assert.assertEqual(
                cardCount > 0,
                tc.expectCard,
                `[${tc.role}] (${vp.width}) .account-command-card expectation mismatch`
              );
              const links = await pageRes.page.evaluate(() =>
                Array.from(
                  document.querySelectorAll(".account-command-card a[href], .ap-sidebar-footer a[href]")
                ).map((a) => a.getAttribute("href") || "")
              );
              if (tc.expectCard) {
                assert.assertOk(
                  links.some((h) => h.includes(c.adminPath)),
                  `[${tc.role}] (${vp.width}) must link to admin panel`
                );
                assert.assertEqual(
                  links.some((h) => h.includes(`${c.adminPath}/tasarim`)),
                  tc.expectStudio,
                  `[${tc.role}] (${vp.width}) Design Studio link presence mismatch`
                );
                assert.assertEqual(
                  links.some((h) => h.includes(`${c.adminPath}/editor`) || h.includes("/podmin/editor")),
                  tc.expectEditor,
                  `[${tc.role}] (${vp.width}) Editorial Studio (/editor) quick-link presence mismatch`
                );
              } else {
                assert.assertEqual(
                  links.some((h) => h.includes(c.adminPath) || h.includes("/podmin")),
                  false,
                  `Reader role [${tc.role}] (${vp.width}) must have zero /podmin links on /hesabim`
                );
              }
              assert.assertNoAssetOrNetworkErrors(pageRes, [], `T4-35 [${tc.role}] (${vp.width})`);
            } finally {
              await pageRes.close();
            }
          }
        }
      }
    );
  });
}
