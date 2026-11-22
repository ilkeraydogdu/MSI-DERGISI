import {
  assertNoUnexpectedBrowserErrors,
  auditUiSurface,
  checkInteractiveVisibilityAndLayout,
  createAdminContent,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

async function waitForAdminReady(page) {
  await page.waitForFunction(
    () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
    { timeout: 15000 }
  );
  // Give applyRbacVisibility a tick to settle DOM attributes
  await page.waitForTimeout(150);
}

async function getVisibleAdminTabs(page) {
  return await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('#admin-sidebar [data-tab^="tab-"]'));
    const visible = [];
    const hidden = [];
    for (const b of btns) {
      const tab = b.getAttribute("data-tab");
      const st = window.getComputedStyle(b);
      const isHidden =
        b.classList.contains("hidden") ||
        b.getAttribute("data-rbac-hidden") === "1" ||
        st.display === "none" ||
        st.visibility === "hidden";
      if (isHidden) hidden.push(tab);
      else visible.push(tab);
    }
    const studioLink = document.querySelector('a[href$="/tasarim"]');
    const studioVisible = studioLink
      ? !studioLink.classList.contains("hidden") &&
        studioLink.getAttribute("data-rbac-hidden") !== "1" &&
        window.getComputedStyle(studioLink).display !== "none"
      : false;
    return { visible, hidden, studioVisible };
  });
}

async function assertForbiddenPanesHidden(ctx, page, role, expectedForbiddenTabs) {
  const { assert } = ctx;
  const paneChecks = await page.evaluate((forbidden) => {
    return forbidden.map((tabId) => {
      const pane = document.getElementById(tabId);
      if (!pane) return { tabId, exists: false, rbacHidden: false, isHidden: true };
      const st = window.getComputedStyle(pane);
      const isHidden =
        pane.classList.contains("hidden") ||
        pane.getAttribute("data-rbac-hidden") === "1" ||
        st.display === "none" ||
        st.visibility === "hidden";
      return {
        tabId,
        exists: true,
        rbacHidden: pane.getAttribute("data-rbac-hidden") === "1",
        isHidden,
      };
    });
  }, expectedForbiddenTabs);

  for (const check of paneChecks) {
    assert.assertOk(
      check.isHidden && (!check.exists || check.rbacHidden),
      `[${role}] Restricted pane #${check.tabId} must have data-rbac-hidden="1" and remain hidden (got ${JSON.stringify(check)})`
    );
  }
}

async function auditRoleAdminDesktopAndMobile(ctx, role, {
  expectedVisibleTabs,
  expectedForbiddenTabs,
  expectStudioVisible,
  expectedLandingTab = null,
  interactiveDesktopCheck = null,
  interactiveMobileCheck = null,
}) {
  const { assert } = ctx;
  const actor = await ctx.auth.createUserWithRole(role);

  // 1. Desktop (1440x900)
  const deskRes = await openDomainPage(ctx, `/${ctx.adminPath}`, {
    client: actor.client,
    viewport: ctx.browser.VIEWPORT_DESKTOP,
  });
  try {
    assert.assertEqual(deskRes.status, 200, `[${role}] Desktop admin shell must return 200`);
    await waitForAdminReady(deskRes.page);
    await auditUiSurface(ctx, deskRes, {
      label: `Admin [${role}] Desktop`,
      requireMain: false,
      requireHeading: false,
    });

    const tabs = await getVisibleAdminTabs(deskRes.page);
    for (const t of expectedVisibleTabs) {
      assert.assertIncludes(
        tabs.visible,
        t,
        `[${role}] Permitted tab ${t} must be visible in Admin sidebar`
      );
    }
    for (const t of expectedForbiddenTabs) {
      assert.assertIncludes(
        tabs.hidden,
        t,
        `[${role}] Restricted tab ${t} must be hidden from Admin sidebar`
      );
    }
    await assertForbiddenPanesHidden(ctx, deskRes.page, role, expectedForbiddenTabs);

    assert.assertEqual(
      tabs.studioVisible,
      expectStudioVisible,
      `[${role}] Design Studio link visibility mismatch`
    );

    if (expectedLandingTab) {
      const activePaneVisible = await deskRes.page
        .locator(`#${expectedLandingTab}`)
        .evaluate((el) => !el.classList.contains("hidden") && window.getComputedStyle(el).display !== "none");
      assert.assertOk(
        activePaneVisible,
        `[${role}] Expected landing tab #${expectedLandingTab} to be active`
      );
    }

    // Switch across all permitted tabs & verify pane activation + zero overflow
    for (const tabId of expectedVisibleTabs) {
      const btn = deskRes.page.locator(`#admin-sidebar [data-tab="${tabId}"]`).first();
      await checkInteractiveVisibilityAndLayout(ctx, deskRes.page, `#admin-sidebar [data-tab="${tabId}"]`, {
        label: `[${role}] Desktop sidebar ${tabId}`,
      });
      await btn.click();
      await deskRes.page.waitForTimeout(80);
      const paneActive = await deskRes.page
        .locator(`#${tabId}`)
        .evaluate((el) => el.classList.contains("active") && !el.classList.contains("hidden"));
      assert.assertOk(paneActive, `[${role}] Desktop pane #${tabId} must activate on click`);
      assert.assertNoOverflow(
        await deskRes.measureOverflow(),
        `[${role}] Desktop overflow after switching to ${tabId}`
      );
    }

    if (interactiveDesktopCheck) {
      await interactiveDesktopCheck(deskRes);
    }
    assert.assertNoAssetOrNetworkErrors(
      deskRes,
      !ctx.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [],
      `[${role}] Desktop runtime/asset/API errors after interaction`
    );
  } finally {
    await deskRes.close();
  }

  // 2. Mobile (390x844)
  const mobRes = await openDomainPage(ctx, `/${ctx.adminPath}`, {
    client: actor.client,
    viewport: ctx.browser.VIEWPORT_MOBILE,
  });
  try {
    assert.assertEqual(mobRes.status, 200, `[${role}] Mobile admin shell must return 200`);
    await waitForAdminReady(mobRes.page);
    await auditUiSurface(ctx, mobRes, {
      label: `Admin [${role}] Mobile`,
      requireMain: false,
      requireHeading: false,
    });
    await assertForbiddenPanesHidden(ctx, mobRes.page, role, expectedForbiddenTabs);

    // Verify mobile sidebar drawer toggle, tab switching, and zero overflow
    const toggleSel = "#sidebar-toggle";
    for (const tabId of expectedVisibleTabs.slice(0, 3)) {
      await checkInteractiveVisibilityAndLayout(ctx, mobRes.page, toggleSel, {
        label: `[${role}] Mobile #sidebar-toggle (${tabId})`,
      });
      await mobRes.page.locator(toggleSel).click();
      await mobRes.page.waitForTimeout(180);
      assert.assertNoOverflow(
        await mobRes.measureOverflow(),
        `[${role}] Mobile overflow with sidebar drawer open`
      );
      const mobTabBtn = mobRes.page.locator(`#admin-sidebar [data-tab="${tabId}"]`).first();
      await mobTabBtn.waitFor({ state: "visible", timeout: 5000 });
      await mobTabBtn.click();
      await mobRes.page.waitForTimeout(150);
      const mobPaneActive = await mobRes.page
        .locator(`#${tabId}`)
        .evaluate((el) => el.classList.contains("active") && !el.classList.contains("hidden"));
      assert.assertOk(mobPaneActive, `[${role}] Mobile pane #${tabId} must activate when selected from drawer`);
      assert.assertNoOverflow(
        await mobRes.measureOverflow(),
        `[${role}] Mobile overflow after switching to ${tabId}`
      );
    }

    // Open mobile drawer once more and verify #sidebar-close-btn closes it cleanly
    await mobRes.page.locator(toggleSel).click();
    await mobRes.page.waitForTimeout(180);
    await checkInteractiveVisibilityAndLayout(ctx, mobRes.page, "#sidebar-close-btn", {
      label: `[${role}] Mobile #sidebar-close-btn`,
    });
    await mobRes.page.locator("#sidebar-close-btn").first().click();
    await mobRes.page.waitForTimeout(150);
    const sidebarStillOpen = await mobRes.page
      .locator("#admin-sidebar")
      .evaluate((el) => el.classList.contains("open"));
    assert.assertEqual(sidebarStillOpen, false, `[${role}] Mobile #sidebar-close-btn must close sidebar`);

    if (interactiveMobileCheck) {
      await interactiveMobileCheck(mobRes);
    }
    assert.assertNoAssetOrNetworkErrors(
      mobRes,
      !ctx.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [],
      `[${role}] Mobile runtime/asset/API errors after drawer & tab interactions`
    );
  } finally {
    await mobRes.close();
  }
}

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 Admin Command Center: Live Playwright Role UI/UX (Desktop & Mobile)", () => {
    suite.test(
      "T4-01: Admin login gateway (/${adminPath} & /podmin) interactive controls, negative login error, and live browser sign-in",
      { req: ["REQ-037", "REQ-033"] },
      async (c) => {
        // Unauthenticated /podmin renders gateway on Desktop & Mobile
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const gwRes = await openDomainPage(c, "/podmin", { viewport: vp });
          try {
            assert.assertEqual(gwRes.status, 200);
            await auditUiSurface(c, gwRes, {
              label: `/podmin gateway (${vp.width}x${vp.height})`,
              requireMain: false,
            });
            await checkInteractiveVisibilityAndLayout(c, gwRes.page, "#login-email");
            await checkInteractiveVisibilityAndLayout(c, gwRes.page, "#login-password");
            await checkInteractiveVisibilityAndLayout(c, gwRes.page, "#login-submit-btn");

            // Password visibility toggle
            await gwRes.page.locator("#btn-toggle-pw").click();
            const pwType = await gwRes.page.locator("#login-password").getAttribute("type");
            assert.assertEqual(pwType, "text", "#btn-toggle-pw must reveal password input");
          } finally {
            await gwRes.close();
          }
        }

        // Negative invalid login -> clean UI error & zero uncaught JS/5xx errors
        const loginRes = await openDomainPage(c, `/${c.adminPath}`, {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await loginRes.page.fill("#login-email", c.credentials.admin.email);
          await loginRes.page.fill("#login-password", "WrongAdminPass!999");
          await loginRes.page.click("#login-submit-btn");
          await loginRes.page.locator("#login-error").waitFor({ state: "visible", timeout: 10000 });
          assertNoUnexpectedBrowserErrors(c, loginRes, ["401", "status of 401"], "Invalid admin login UI");

          // Valid login via UI form -> redirects to Admin shell
          await loginRes.page.fill("#login-password", c.credentials.admin.password);
          await Promise.all([
            loginRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            loginRes.page.click("#login-submit-btn"),
          ]);
          await waitForAdminReady(loginRes.page);
          assert.assertNoOverflow(await loginRes.measureOverflow());
        } finally {
          await loginRes.close();
        }
      }
    );

    suite.test(
      "T4-02: [super_admin] Admin Command Center all 18 tabs + #tab-11 Users pagination UI (REQ-041) on Desktop & Mobile",
      { req: ["REQ-037", "REQ-041"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();

        // Provision 25 users so pagination (>20 rows) is triggered cleanly
        for (let i = 0; i < 22; i++) {
          await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
        }

        const deskRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(deskRes.status, 200);
          await waitForAdminReady(deskRes.page);
          await auditUiSurface(c, deskRes, {
            label: "Admin [super_admin] Desktop",
            requireMain: false,
            requireHeading: false,
          });

          const tabs = await getVisibleAdminTabs(deskRes.page);
          assert.assertEqual(tabs.visible.length, 18, "super_admin must see all 18 admin tabs");
          assert.assertOk(tabs.studioVisible, "super_admin must see Design Studio link");

          // Iterate through all 18 tabs without horizontal overflow (note: tab-17 fetches /monitoring/bundle which returns 503 when MONITORING_ENABLED=false in isolated test env)
          for (let n = 1; n <= 18; n++) {
            await deskRes.page.locator(`#admin-sidebar [data-tab="tab-${n}"]`).first().click();
            await deskRes.page.waitForTimeout(80);
            assert.assertNoOverflow(
              await deskRes.measureOverflow(),
              `super_admin Desktop overflow on tab-${n}`
            );
          }

          // REQ-041: #tab-11 Users pagination UI must exist strictly inside the #tbody-admin-users card and navigate pages
          await deskRes.page.locator('#admin-sidebar [data-tab="tab-11"]').first().click();
          await deskRes.page.waitForTimeout(250);
          const usersPagInfo = await deskRes.page.evaluate(() => {
            const tbody = document.querySelector("#tbody-admin-users");
            if (!tbody) return { hasUsersTbody: false, hasPagination: false, hasNextBtn: false };
            const usersCard = tbody.closest(".admin-card, .ops-card, #tab11-users");
            if (!usersCard) return { hasUsersTbody: true, hasPagination: false, hasNextBtn: false };
            const pagEl = usersCard.querySelector(
              "#pagination-admin-users, #users-pagination, [data-users-pagination], .pagination-bar, .pagination-controls"
            );
            const nextBtn = pagEl
              ? Array.from(pagEl.querySelectorAll("button:not([disabled])")).find(
                  (b) => b.dataset.page === "2" || /sonraki|next/i.test(b.textContent || "")
                )
              : null;
            const firstRowText = tbody.querySelector("tr")?.textContent?.trim() || "";
            return {
              hasUsersTbody: true,
              hasPagination: Boolean(pagEl && pagEl.querySelectorAll("button").length > 0),
              hasNextBtn: Boolean(nextBtn),
              firstRowText,
            };
          });
          assert.assertOk(
            usersPagInfo.hasPagination,
            "#tab-11 Users card (#tbody-admin-users) must render dedicated pagination controls when >20 users exist (M4 REQ-041)"
          );
          assert.assertOk(
            usersPagInfo.hasNextBtn,
            "#tab-11 Users pagination must expose an enabled Next/page-2 button when >20 users exist (M4 REQ-041)"
          );
          await deskRes.page
            .locator("#tbody-admin-users")
            .locator('xpath=ancestor::*[contains(@class,"admin-card") or @id="tab11-users"][1]')
            .locator('button:not([disabled])[data-page="2"], .pagination-controls button:not([disabled])')
            .last()
            .click();
          await deskRes.page.waitForTimeout(250);
          const page2FirstRow = await deskRes.page
            .locator("#tbody-admin-users tr")
            .first()
            .textContent()
            .catch(() => "");
          assert.assertNotEqual(
            (page2FirstRow || "").trim(),
            usersPagInfo.firstRowText,
            "#tab-11 Users pagination Next click must load distinct page-2 users (M4 REQ-041)"
          );

          const allowedNet = !c.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [];
          assert.assertNoAssetOrNetworkErrors(
            deskRes,
            allowedNet,
            "super_admin Desktop unexpected browser/asset/API errors across tab-1..18"
          );
        } finally {
          await deskRes.close();
        }

        const mobRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_MOBILE,
        });
        try {
          await waitForAdminReady(mobRes.page);
          await auditUiSurface(c, mobRes, {
            label: "Admin [super_admin] Mobile",
            requireMain: false,
            requireHeading: false,
          });
          await mobRes.page.locator("#sidebar-toggle").click();
          await mobRes.page.waitForTimeout(150);
          assert.assertNoOverflow(await mobRes.measureOverflow());
          await mobRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
          await mobRes.page.waitForTimeout(150);
          assert.assertNoOverflow(await mobRes.measureOverflow());
          assert.assertNoAssetOrNetworkErrors(
            mobRes,
            !c.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [],
            "super_admin Mobile unexpected browser/asset/API errors"
          );
        } finally {
          await mobRes.close();
        }
      }
    );

    suite.test(
      "T4-03: [site_operator] Admin Command Center Desktop & Mobile + #drawer-content interactive layout",
      { req: ["REQ-037"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const res = await openDomainPage(c, `/${c.adminPath}`, {
            client: op.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(res.status, 200);
            await waitForAdminReady(res.page);
            await auditUiSurface(c, res, {
              label: `Admin [site_operator] ${vp.width}x${vp.height}`,
              requireMain: false,
              requireHeading: false,
            });
            const tabs = await getVisibleAdminTabs(res.page);
            assert.assertEqual(tabs.visible.length, 18);
            assert.assertOk(tabs.studioVisible);

            const openDrawerSel = '[data-admin-action="open-content-drawer"], [data-admin-action="hero-open-content-drawer"]';
            await checkInteractiveVisibilityAndLayout(c, res.page, openDrawerSel, {
              label: `site_operator open-content-drawer (${vp.width})`,
            });
            await res.page.locator(openDrawerSel).first().click();
            await res.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 8000 });
            assert.assertNoOverflow(
              await res.measureOverflow(),
              `site_operator #drawer-content overflow (${vp.width})`
            );
            await res.page.locator('#drawer-content [data-admin-action="close-drawer"]').first().click();
            await res.page.locator("#drawer-content").waitFor({ state: "hidden", timeout: 5000 });
            assert.assertNoAssetOrNetworkErrors(
              res,
              !c.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [],
              `site_operator (${vp.width}) asset/network errors`
            );
          } finally {
            await res.close();
          }
        }
      }
    );

    suite.test(
      "T4-04: [editor_in_chief] Role-scoped tabs, hidden commercial/user tabs & #drawer-content check (Desktop & Mobile)",
      { req: ["REQ-037", "REQ-009"] },
      async (c) => {
        const checkContentDrawer = async (pageRes, vpLabel) => {
          await pageRes.page.locator("#tab-1").waitFor({ state: "visible", timeout: 5000 });
          const openBtn = '[data-admin-action="open-content-drawer"]';
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, openBtn, {
            label: `editor_in_chief ${vpLabel} open-content-drawer`,
          });
          await pageRes.page.locator(openBtn).first().click();
          await pageRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 8000 });
          assert.assertNoOverflow(
            await pageRes.measureOverflow(),
            `editor_in_chief #drawer-content overflow (${vpLabel})`
          );
          await pageRes.page.locator('#drawer-content [data-admin-action="close-drawer"]').first().click();
          await pageRes.page.locator("#drawer-content").waitFor({ state: "hidden", timeout: 5000 });
        };

        await auditRoleAdminDesktopAndMobile(c, "editor_in_chief", {
          expectedVisibleTabs: ["tab-1", "tab-18", "tab-10", "tab-5", "tab-16", "tab-12", "tab-6", "tab-3", "tab-8", "tab-13"],
          expectedForbiddenTabs: ["tab-2", "tab-14", "tab-4", "tab-11"],
          expectStudioVisible: true,
          expectedLandingTab: "tab-1",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
            await checkContentDrawer(pageRes, "Desktop");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
            await checkContentDrawer(pageRes, "Mobile");
          },
        });
      }
    );

    suite.test(
      "T4-05: [content_editor] Role-scoped editorial tabs only (approvals/ads/users/studio hidden) on Desktop & Mobile",
      { req: ["REQ-037", "REQ-009"] },
      async (c) => {
        await auditRoleAdminDesktopAndMobile(c, "content_editor", {
          expectedVisibleTabs: ["tab-1", "tab-18", "tab-10", "tab-5", "tab-12"],
          expectedForbiddenTabs: ["tab-2", "tab-14", "tab-4", "tab-16", "tab-6", "tab-11", "tab-8", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-1",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-admin-action="open-content-drawer"]');
            await pageRes.page.locator('[data-admin-action="open-content-drawer"]').first().click();
            await pageRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 8000 });
            assert.assertNoOverflow(await pageRes.measureOverflow(), "content_editor #drawer-content Desktop overflow");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-admin-action="open-content-drawer"]');
            await pageRes.page.locator('[data-admin-action="open-content-drawer"]').first().click();
            await pageRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 8000 });
            assert.assertNoOverflow(await pageRes.measureOverflow(), "content_editor #drawer-content Mobile overflow");
          },
        });
      }
    );

    suite.test(
      "T4-06: [content_publisher] Role-scoped publish & approval tabs without Studio/Ads/Users on Desktop & Mobile",
      { req: ["REQ-037", "REQ-009"] },
      async (c) => {
        await auditRoleAdminDesktopAndMobile(c, "content_publisher", {
          expectedVisibleTabs: ["tab-1", "tab-18", "tab-10", "tab-5", "tab-16", "tab-12", "tab-7"],
          expectedForbiddenTabs: ["tab-2", "tab-14", "tab-4", "tab-6", "tab-8", "tab-11", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-1",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-16"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-admin-action="load-pending-approvals"]');
            await pageRes.page.locator('[data-admin-action="load-pending-approvals"]').first().click();
            await pageRes.page.waitForTimeout(150);
            assert.assertNoOverflow(await pageRes.measureOverflow(), "content_publisher #tab-16 overflow");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-16"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-admin-action="load-pending-approvals"]');
            assert.assertNoOverflow(await pageRes.measureOverflow(), "content_publisher #tab-16 Mobile overflow");
          },
        });
      }
    );

    suite.test(
      "T4-07: [ads_manager] Auto-lands on tab-2, hides editorial/revenue/users, and opens #drawer-ad cleanly",
      { req: ["REQ-037", "REQ-021"] },
      async (c) => {
        const openAndVerifyAdDrawer = async (pageRes, vpLabel) => {
          const newAdSel = '[data-admin-action="open-ad-drawer"]';
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, newAdSel, {
            label: `ads_manager ${vpLabel} open-ad-drawer`,
          });
          await pageRes.page.locator(newAdSel).first().click();
          await pageRes.page.locator("#drawer-ad").waitFor({ state: "visible", timeout: 8000 });
          assert.assertNoOverflow(
            await pageRes.measureOverflow(),
            `ads_manager #drawer-ad overflow (${vpLabel})`
          );
          await pageRes.page.locator('#drawer-ad [data-admin-action="close-drawer"]').first().click();
          await pageRes.page.locator("#drawer-ad").waitFor({ state: "hidden", timeout: 5000 });
        };

        await auditRoleAdminDesktopAndMobile(c, "ads_manager", {
          expectedVisibleTabs: ["tab-2", "tab-10", "tab-3", "tab-9", "tab-15"],
          expectedForbiddenTabs: ["tab-1", "tab-14", "tab-4", "tab-16", "tab-6", "tab-11", "tab-8", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-2",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
            await openAndVerifyAdDrawer(pageRes, "Desktop");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
            await openAndVerifyAdDrawer(pageRes, "Mobile");
          },
        });
      }
    );

    suite.test(
      "T4-08: [finance_viewer] Auto-lands on tab-14 (Revenue), hides editorial/ads/users on Desktop & Mobile",
      { req: ["REQ-037", "REQ-028"] },
      async (c) => {
        const switchInvoicesSubtab = async (pageRes, vpLabel) => {
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, '#tab-14 [data-subtab="tab14-invoices"]', {
            label: `finance_viewer ${vpLabel} tab14-invoices`,
          });
          await pageRes.page.locator('#tab-14 [data-subtab="tab14-invoices"]').first().click();
          await pageRes.page.locator("#tab14-invoices").waitFor({ state: "visible", timeout: 5000 });
          assert.assertNoOverflow(await pageRes.measureOverflow(), `finance_viewer tab14-invoices (${vpLabel}) overflow`);
        };

        await auditRoleAdminDesktopAndMobile(c, "finance_viewer", {
          expectedVisibleTabs: ["tab-14", "tab-3", "tab-9", "tab-15", "tab-7"],
          expectedForbiddenTabs: ["tab-1", "tab-2", "tab-4", "tab-16", "tab-6", "tab-11", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-14",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-14"]').first().click();
            await switchInvoicesSubtab(pageRes, "Desktop");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-14"]').first().click();
            await switchInvoicesSubtab(pageRes, "Mobile");
          },
        });
      }
    );

    suite.test(
      "T4-09: [support_agent] & [business_dev] Role-scoped landing tabs & hidden restricted modules (Desktop & Mobile)",
      { req: ["REQ-037", "REQ-016"] },
      async (c) => {
        await auditRoleAdminDesktopAndMobile(c, "support_agent", {
          expectedVisibleTabs: ["tab-4", "tab-14", "tab-8", "tab-11", "tab-7"],
          expectedForbiddenTabs: ["tab-1", "tab-2", "tab-16", "tab-6", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-4",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-4"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#filter-corp-status");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-4"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#filter-corp-status");
          },
        });

        await auditRoleAdminDesktopAndMobile(c, "business_dev", {
          expectedVisibleTabs: ["tab-2", "tab-14", "tab-3", "tab-9", "tab-15"],
          expectedForbiddenTabs: ["tab-1", "tab-4", "tab-16", "tab-11", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-2",
          interactiveDesktopCheck: async (pageRes) => {
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#filter-ad-status");
          },
          interactiveMobileCheck: async (pageRes) => {
            await pageRes.page.locator("#sidebar-toggle").click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#filter-ad-status");
          },
        });
      }
    );

    suite.test(
      "T4-10: M4 Admin UI wiring audit: convert-pdf drawer (#tab-18), target_institutions, custom-field-values, recommendation-rules & ad schedule",
      { req: ["REQ-008", "REQ-012", "REQ-019", "REQ-022", "REQ-039", "REQ-040"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const pageRes = await openDomainPage(c, `/${ctx.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await waitForAdminReady(pageRes.page);

          // 1. Navigate to #tab-18 and open #drawer-issue to verify M2/M4 convert-pdf controls interactively
          await pageRes.page.locator('#admin-sidebar [data-tab="tab-18"]').first().click();
          await pageRes.page.waitForTimeout(150);
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-admin-action="open-issue-drawer"]');
          await pageRes.page.locator('[data-admin-action="open-issue-drawer"]').first().click();
          await pageRes.page.locator("#drawer-issue").waitFor({ state: "visible", timeout: 8000 });
          await checkInteractiveVisibilityAndLayout(
            c,
            pageRes.page,
            '#drawer-issue [data-admin-action="convert-drawer-pdf"], #btn-iss-convert-now',
            { label: "#drawer-issue convert-drawer-pdf button" }
          );
          const hasConvertPdfUi = await pageRes.page.evaluate(() => {
            const scope = document.querySelector("#tab-18");
            return Boolean(
              scope?.querySelector(
                '[data-admin-action="convert-issue-pdf"], [data-admin-action="convert-drawer-pdf"], #btn-iss-convert-now'
              )
            );
          });
          assert.assertOk(
            hasConvertPdfUi,
            "Issues panel/drawer (#tab-18 / #drawer-issue) must expose convert-issue-pdf / convert-drawer-pdf / #btn-iss-convert-now trigger (M2/M4 REQ-019)"
          );
          await pageRes.page.locator('#drawer-issue [data-admin-action="close-drawer"]').first().click();
          await pageRes.page.locator("#drawer-issue").waitFor({ state: "hidden", timeout: 5000 });

          // 2. Navigate to #tab-2 and open #drawer-ad to inspect M4 ad schedule controls
          await pageRes.page.locator('#admin-sidebar [data-tab="tab-2"]').first().click();
          await pageRes.page.waitForTimeout(150);
          await pageRes.page.locator('[data-admin-action="open-ad-drawer"]').first().click();
          await pageRes.page.locator("#drawer-ad").waitFor({ state: "visible", timeout: 8000 });
          const adChecks = await pageRes.page.evaluate(() => {
            const adForm = document.querySelector("#form-ad-campaign");
            return {
              hasAdScheduleUi: Boolean(
                adForm?.querySelector('[name="start_date"], #ad-start-date') &&
                  adForm?.querySelector('[name="end_date"], #ad-end-date') &&
                  adForm?.querySelector('[name="rotation_mode"], #ad-rotation-mode')
              ),
              hasAdCustomFields: Boolean(
                document.querySelector("#ad-custom-fields, #drawer-ad [data-custom-field-values]")
              ),
            };
          });
          await pageRes.page.locator('#drawer-ad [data-admin-action="close-drawer"]').first().click();
          await pageRes.page.locator("#drawer-ad").waitFor({ state: "hidden", timeout: 5000 });

          // 3. Navigate to #tab-1 and open #drawer-content to inspect target_institutions & custom-field-values
          await pageRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
          await pageRes.page.waitForTimeout(150);
          await pageRes.page.locator('[data-admin-action="open-content-drawer"]').first().click();
          await pageRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 8000 });
          const contentChecks = await pageRes.page.evaluate(() => {
            const contentForm = document.querySelector("#form-content-ssot");
            const hasTargetInstitutions = Boolean(
              contentForm?.querySelector(
                '[name="c_target_inst"], [name="target_institutions"], #c-target-institutions, #c-target-institutions-matrix, [data-target-institutions]'
              )
            );
            const hasCustomFieldValuesUi = Boolean(
              document.querySelector(
                "#content-custom-fields, #ad-custom-fields, #user-custom-fields, [data-custom-field-values]"
              )
            );
            const hasRecommendationRulesUi = Boolean(
              document.querySelector(
                "#tbody-recommendation-rules, #form-recommendation-rule, [data-recommendation-rules]"
              )
            );
            return {
              hasTargetInstitutions,
              hasCustomFieldValuesUi,
              hasRecommendationRulesUi,
            };
          });

          assert.assertOk(
            contentChecks.hasTargetInstitutions,
            "Content editor #form-content-ssot must wire target_institutions control (M4 REQ-039)"
          );
          assert.assertOk(
            contentChecks.hasCustomFieldValuesUi || adChecks.hasAdCustomFields,
            "Admin drawers must wire custom-field-values inputs (M4 REQ-008)"
          );
          assert.assertOk(
            contentChecks.hasRecommendationRulesUi,
            "Admin panel must wire recommendation-rules management UI (M4 REQ-012)"
          );
          assert.assertOk(
            adChecks.hasAdScheduleUi,
            "Ad form #form-ad-campaign must wire start_date, end_date & rotation_mode inputs (M4 REQ-022)"
          );
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T4-29: Login Gateway full interactive matrix (/podmin: lang, theme, showcase tabs, forgot password, TOTP 2FA -> Command Center)",
      { req: ["F2", "F3", "REQ-037"] },
      async (c) => {
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const staff = await c.auth.createUserWithRole("site_operator");
          const { secret } = await c.auth.enableTotp(staff.client);
          const gwRes = await openDomainPage(c, "/podmin", { viewport: vp });
          try {
            assert.assertEqual(gwRes.status, 200);
            const dataAdminPath = await gwRes.page.locator("body").getAttribute("data-admin-path");
            assert.assertOk(Boolean(dataAdminPath), "body[data-admin-path] must be present");
            assert.assertNotIncludes((dataAdminPath || "").toLowerCase(), "msi-komuta");

            // Query localization ?lang=en <-> ?lang=tr (no TR/EN clutter buttons on /podmin)
            await gwRes.page.goto(`${c.baseUrl}/podmin?lang=en`, { waitUntil: "networkidle" });
            assert.assertEqual(await gwRes.page.locator("html").getAttribute("lang"), "en");
            await gwRes.page.goto(`${c.baseUrl}/podmin?lang=tr`, { waitUntil: "networkidle" });
            assert.assertEqual(await gwRes.page.locator("html").getAttribute("lang"), "tr");

            // Autonomous OS color-scheme sync (dark <-> light) without manual toggle button
            await gwRes.page.emulateMedia({ colorScheme: "dark" });
            await gwRes.page.waitForTimeout(120);
            assert.assertEqual(await gwRes.page.locator("html").getAttribute("data-theme"), "dark");
            await gwRes.page.emulateMedia({ colorScheme: "light" });
            await gwRes.page.waitForTimeout(120);
            assert.assertEqual(await gwRes.page.locator("html").getAttribute("data-theme"), "light");
            await gwRes.page.reload({ waitUntil: "networkidle" });
            assert.assertEqual(await gwRes.page.locator("html").getAttribute("data-theme"), "light");

            // Full-bleed interactive WebGL / fallback defence gallery stage integrity
            await gwRes.page.locator("#login-stage").waitFor({ state: "visible", timeout: 5000 });
            const heroLoaded = await gwRes.page.locator("#stage-fallback-img").evaluate((img) => Boolean(img && img.complete && img.naturalWidth > 0));
            assert.assertOk(heroLoaded, "#stage-fallback-img must load cleanly on /podmin");
            const activeIdx = await gwRes.page.locator("#login-stage").getAttribute("data-active-index");
            assert.assertEqual(activeIdx, "0", "#login-stage initial active index should be 0");

            // Forgot password subview
            await gwRes.page.locator("#btn-show-forgot").click();
            await gwRes.page.locator("#forgot-password-form").waitFor({ state: "visible", timeout: 5000 });
            await gwRes.page.locator("#forgot-email").fill(staff.email);
            await gwRes.page.locator("#forgot-submit-btn").click();
            await gwRes.page.locator("#login-success.visible").waitFor({ state: "visible", timeout: 10000 });
            await gwRes.page.locator("#btn-back-from-forgot").click();
            await gwRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 5000 });

            // TOTP 2FA flow -> back -> re-submit -> Command Center
            await gwRes.page.locator("#login-email").fill(staff.email);
            await gwRes.page.locator("#login-password").fill(staff.password);
            await gwRes.page.locator("#login-submit-btn").click();
            await gwRes.page.locator("#totp-login-form").waitFor({ state: "visible", timeout: 10000 });

            await gwRes.page.locator("#btn-back-from-totp").click();
            await gwRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 5000 });

            await gwRes.page.locator("#login-email").fill(staff.email);
            await gwRes.page.locator("#login-password").fill(staff.password);
            await gwRes.page.locator("#login-submit-btn").click();
            await gwRes.page.locator("#totp-login-form").waitFor({ state: "visible", timeout: 10000 });
            await gwRes.page.locator("#totp-code-input").fill(c.totp.generateTotp(secret));
            await Promise.all([
              gwRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
              gwRes.page.locator("#totp-submit-btn").click(),
            ]);
            await waitForAdminReady(gwRes.page);
            assert.assertNoOverflow(await gwRes.measureOverflow(), `T4-29 (${vp.width}) overflow`);
            assert.assertNoAssetOrNetworkErrors(
              gwRes,
              ["/api/v1/admin/monitoring/bundle"],
              `T4-29 (${vp.width}) asset/network errors`
            );
          } finally {
            await gwRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-30: Missing 3 staff roles (admin, managing_editor, editor) Command Center Desktop & Mobile",
      { req: ["F4", "REQ-037", "REQ-009"] },
      async (c) => {
        const all18 = Array.from({ length: 18 }, (_, i) => `tab-${i + 1}`);
        await auditRoleAdminDesktopAndMobile(c, "admin", {
          expectedVisibleTabs: all18,
          expectedForbiddenTabs: [],
          expectStudioVisible: true,
          expectedLandingTab: "tab-1",
        });

        await auditRoleAdminDesktopAndMobile(c, "managing_editor", {
          expectedVisibleTabs: ["tab-1", "tab-18", "tab-16", "tab-12", "tab-10", "tab-5", "tab-6", "tab-8", "tab-9", "tab-7", "tab-13"],
          expectedForbiddenTabs: ["tab-2", "tab-14", "tab-4", "tab-3", "tab-15", "tab-11", "tab-17"],
          expectStudioVisible: true,
          expectedLandingTab: "tab-1",
        });

        await auditRoleAdminDesktopAndMobile(c, "editor", {
          expectedVisibleTabs: ["tab-1", "tab-18", "tab-12", "tab-10", "tab-5"],
          expectedForbiddenTabs: ["tab-2", "tab-14", "tab-4", "tab-16", "tab-6", "tab-3", "tab-9", "tab-15", "tab-8", "tab-11", "tab-17", "tab-7", "tab-13"],
          expectStudioVisible: false,
          expectedLandingTab: "tab-1",
        });
      }
    );

    suite.test(
      "T4-31: Admin #topbar-theme-toggle, #sidebar-quick-filter, #drawer-newsletter, [data-lang-tab], #totp-security-modal & #approval-modal",
      { req: ["F2", "F4", "REQ-037"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const editor = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
        const draft = await createAdminContent(c, editor.client, { status: "draft" });
        const subRes = await editor.client.post(`/api/v1/admin/contents/${draft.id}/submit-review`, { json: {} });
        assert.assertStatus(subRes, 200);

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, `/${c.adminPath}`, {
            client: sa.client,
            viewport: vp,
          });
          try {
            await waitForAdminReady(pageRes.page);

            // Theme toggle + persistence
            const t0 = await pageRes.page.locator("html").getAttribute("data-theme");
            await pageRes.page.locator("#topbar-theme-toggle").click();
            await pageRes.page.waitForTimeout(120);
            const t1 = await pageRes.page.locator("html").getAttribute("data-theme");
            assert.assertNotEqual(t1, t0, "#topbar-theme-toggle must flip html[data-theme]");
            await pageRes.page.reload({ waitUntil: "networkidle" });
            await waitForAdminReady(pageRes.page);
            assert.assertEqual(await pageRes.page.locator("html").getAttribute("data-theme"), t1);

            if (vp.width >= 1024) {
              // Sidebar quick filter
              await pageRes.page.locator("#sidebar-quick-filter").fill("Bülten");
              await pageRes.page.waitForTimeout(100);
              const tab6Vis = await pageRes.page.locator("#tab-btn-6").isVisible();
              const tab1Vis = await pageRes.page.locator("#tab-btn-1").isVisible();
              assert.assertOk(tab6Vis, "#tab-btn-6 must match sidebar filter 'Bülten'");
              assert.assertEqual(tab1Vis, false, "#tab-btn-1 must hide when filtered for 'Bülten'");
              await pageRes.page.locator("#sidebar-quick-filter").fill("");
              await pageRes.page.waitForTimeout(100);
            } else {
              await pageRes.page.locator("#sidebar-toggle").click();
              await pageRes.page.waitForTimeout(150);
            }

            // Newsletter drawer + lang tabs
            await pageRes.page.locator('#admin-sidebar [data-tab="tab-6"]').first().click();
            await pageRes.page.waitForTimeout(150);
            await pageRes.page.locator('[data-admin-action="open-newsletter-drawer"]').first().click();
            await pageRes.page.locator("#drawer-newsletter").waitFor({ state: "visible", timeout: 8000 });
            await pageRes.page.locator('#form-newsletter [data-lang-tab="en"]').first().click();
            await pageRes.page.locator('#form-newsletter [data-lang-pane="en"]').first().waitFor({ state: "visible", timeout: 5000 });
            await pageRes.page.locator('#form-newsletter [data-lang-tab="tr"]').first().click();
            await pageRes.page.locator('#drawer-newsletter [data-admin-action="close-drawer"]').first().click();
            await pageRes.page.locator("#drawer-newsletter").waitFor({ state: "hidden", timeout: 5000 });

            // TOTP security modal open/close on both desktop & mobile
            await pageRes.page.locator("#topbar-2fa-btn").click();
            await pageRes.page.locator("#totp-security-modal").waitFor({ state: "visible", timeout: 5000 });
            await pageRes.page.locator("#btn-close-2fa-modal").click();
            await pageRes.page.locator("#totp-security-modal").waitFor({ state: "hidden", timeout: 5000 });

            // Approval modal open/close on Desktop (sidebar requires mobile toggle below 1024px)
            if (vp.width >= 1024) {
              await pageRes.page.locator('#admin-sidebar [data-tab="tab-16"]').first().click();
              await pageRes.page.locator('[data-admin-action="open-approval-modal"]').first().waitFor({ state: "visible", timeout: 8000 });
              await pageRes.page.locator('[data-admin-action="open-approval-modal"]').first().click();
              await pageRes.page.locator("#approval-modal").waitFor({ state: "visible", timeout: 5000 });
              await pageRes.page.locator('#approval-modal [data-admin-action="close-approval-modal"]').first().click();
              await pageRes.page.locator("#approval-modal").waitFor({ state: "hidden", timeout: 5000 });
            }

            assert.assertNoOverflow(await pageRes.measureOverflow(), `T4-31 (${vp.width}) overflow`);
            assert.assertNoAssetOrNetworkErrors(
              pageRes,
              ["/api/v1/admin/monitoring/bundle"],
              `T4-31 (${vp.width}) browser errors`
            );
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-32: Admin logout via #btn-logout returns cleanly to #login-form gateway",
      { req: ["F3", "F4", "REQ-037"] },
      async (c) => {
        const op = await c.auth.createUserWithRole("site_operator");
        const pageRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: op.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await waitForAdminReady(pageRes.page);
          await Promise.all([
            pageRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            pageRes.page.locator("#btn-logout").click(),
          ]);
          await pageRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 8000 });
          assert.assertEqual(await pageRes.page.locator("#admin-sidebar").count(), 0);
          assert.assertNoAssetOrNetworkErrors(
            pageRes,
            ["/api/v1/admin/monitoring/bundle"],
            "T4-32 logout errors"
          );
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T4-39: Admin modals (#totp-security-modal, #drawer-content) trap Tab/Shift+Tab focus and mobile #admin-sidebar closes on Escape",
      { req: ["F4", "REQ-037"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();

        // 1. Desktop: #totp-security-modal & #drawer-content Tab/Shift+Tab focus-trapping
        const deskRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await waitForAdminReady(deskRes.page);

          // Open #totp-security-modal
          await deskRes.page.locator("#topbar-2fa-btn").click();
          await deskRes.page.locator("#totp-security-modal").waitFor({ state: "visible", timeout: 5000 });
          await deskRes.page.waitForTimeout(80);

          for (let i = 0; i < 8; i++) {
            await deskRes.page.keyboard.press("Tab");
            const insideModal = await deskRes.page.evaluate(() => {
              const modal = document.getElementById("totp-security-modal");
              return Boolean(modal && modal.contains(document.activeElement));
            });
            assert.assertOk(insideModal, `Tab step ${i + 1} escaped #totp-security-modal`);
          }
          await deskRes.page.keyboard.press("Shift+Tab");
          const insideOnShiftTab = await deskRes.page.evaluate(() => {
            const modal = document.getElementById("totp-security-modal");
            return Boolean(modal && modal.contains(document.activeElement));
          });
          assert.assertOk(insideOnShiftTab, "Shift+Tab escaped #totp-security-modal");

          await deskRes.page.keyboard.press("Escape");
          await deskRes.page.locator("#totp-security-modal").waitFor({ state: "hidden", timeout: 5000 });

          // Open #drawer-content and test wrapping from last visible focusable -> first
          await deskRes.page.locator('#admin-sidebar [data-tab="tab-1"]').first().click();
          await deskRes.page.locator('[data-admin-action="open-content-drawer"]').first().click();
          await deskRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 5000 });
          await deskRes.page.waitForTimeout(80);

          const drawerFocusInfo = await deskRes.page.evaluate(() => {
            const drawer = document.getElementById("drawer-content");
            const sel = 'a[href], button:not([disabled]), input:not([type="hidden"]):not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
            const vis = Array.from(drawer.querySelectorAll(sel)).filter((el) => {
              if (el.closest("[hidden], .hidden")) return false;
              const st = getComputedStyle(el);
              return st.display !== "none" && st.visibility !== "hidden" && el.getClientRects().length > 0;
            });
            if (vis.length) vis[vis.length - 1].focus();
            return { count: vis.length, firstId: vis[0]?.id || vis[0]?.tagName, lastId: vis[vis.length - 1]?.id || vis[vis.length - 1]?.tagName };
          });
          assert.assertOk(drawerFocusInfo.count >= 2, "Expected >= 2 visible focusables in #drawer-content");
          await deskRes.page.keyboard.press("Tab");
          const wrappedInsideDrawer = await deskRes.page.evaluate(() => {
            const drawer = document.getElementById("drawer-content");
            return Boolean(drawer && drawer.contains(document.activeElement));
          });
          assert.assertOk(wrappedInsideDrawer, "Tab from last element in #drawer-content must stay inside drawer");
          await deskRes.page.keyboard.press("Escape");
          await deskRes.page.locator("#drawer-content").waitFor({ state: "hidden", timeout: 5000 });
        } finally {
          await deskRes.close();
        }

        // 2. Mobile (390x844): #admin-sidebar closes on Escape and restores focus to #sidebar-toggle
        const mobRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_MOBILE,
        });
        try {
          await waitForAdminReady(mobRes.page);
          await mobRes.page.locator("#sidebar-toggle").click();
          await mobRes.page.waitForTimeout(150);
          const opened = await mobRes.page.locator("#admin-sidebar").evaluate((el) => el.classList.contains("open"));
          assert.assertEqual(opened, true, "Mobile #admin-sidebar must open on #sidebar-toggle click");

          await mobRes.page.keyboard.press("Escape");
          await mobRes.page.waitForTimeout(120);
          const closedState = await mobRes.page.evaluate(() => ({
            open: document.getElementById("admin-sidebar")?.classList.contains("open"),
            activeId: document.activeElement?.id || "",
          }));
          assert.assertEqual(closedState.open, false, "Mobile #admin-sidebar must close on Escape");
          assert.assertEqual(closedState.activeId, "sidebar-toggle", "Escape on mobile #admin-sidebar must restore focus to #sidebar-toggle");
        } finally {
          await mobRes.close();
        }
      }
    );
  });
}
