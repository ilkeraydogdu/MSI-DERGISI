import {
  assertButtonHierarchyAndTypeFloor,
  assertModalGeometry,
  assertNoExternalCdnsOrMockdata,
  assertToastContract,
  assertUiTokensPresent,
  captureSurfaceScreenshot,
} from "../../fixtures/design_tokens_contract.mjs";
import {
  auditUiSurface,
  createVerifiedSubscriber,
  openDomainPage,
  seedCookieConsent,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

async function resolveReaderClient(c, role = "registered") {
  if (c.isExternalTarget && c.credentials?.readerEmail && c.credentials?.readerPassword) {
    const client = c.http.createClient();
    await c.auth.login(client, {
      email: c.credentials.readerEmail,
      password: c.credentials.readerPassword,
    });
    await client.ensureCsrf();
    return { client, email: c.credentials.readerEmail, password: c.credentials.readerPassword, ephemeral: false };
  }
  const sub = await createVerifiedSubscriber(c, role);
  return { ...sub, ephemeral: true };
}

async function resolveAdminClient(c) {
  if (c.isExternalTarget && c.credentials?.superAdmin?.email && c.credentials?.superAdmin?.password) {
    return await c.auth.loginAsSuperAdmin();
  }
  if (c.isExternalTarget && c.credentials?.admin?.email && c.credentials?.admin?.password) {
    return await c.auth.loginAsAdmin();
  }
  return await c.auth.loginAsSuperAdmin();
}

async function runCheck(failures, label, fn) {
  try {
    await fn();
  } catch (err) {
    failures.push(`[${label}] ${err?.message || String(err)}`);
  }
}

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 UI/UX Unification Playwright Matrix (All 5 Surfaces)", () => {
    suite.test(
      "T4-U1: Surface 1 Public Storefront & Reader (/, /abonelik-ve-satis, /e-dergi, /arama, /sifre-sifirla) Role A tokens, light/dark, desktop/mobile & screenshots",
      { req: ["F1", "F2", "F3", "F5", "R1", "R2"], readOnly: true },
      async (c) => {
        const failures = [];
        const routes = [
          { slug: "home", path: "/", requireMain: true },
          { slug: "store", path: "/abonelik-ve-satis", requireMain: true },
          { slug: "edergi", path: "/e-dergi", requireMain: true },
          { slug: "search", path: "/arama", requireMain: true },
          { slug: "reset", path: "/sifre-sifirla?token=preview", requireMain: true },
        ];

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const vpName = vp.width >= 1024 ? "desktop" : "mobile";
          for (const theme of ["light", "dark"]) {
            const context = await c.browser.newContext({ viewport: vp, locale: "tr-TR" });
            try {
              await seedCookieConsent(context, { necessary: true, analytics: false, marketing: false });
              await context.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_theme", t);
                  window.localStorage.setItem("msi_admin_theme", t);
                } catch {
                  // ignore
                }
              }, theme);

              for (const r of routes) {
                const pageRes = await openDomainPage(c, r.path, { context, viewport: vp });
                try {
                  assert.assertEqual(pageRes.status, 200, `Surface 1 ${r.path} (${theme}/${vpName}) must return 200`);
                  await pageRes.setTheme(theme);

                  await auditUiSurface(c, pageRes, {
                    label: `Surface 1 ${r.path} (${theme}/${vpName})`,
                    requireMain: r.requireMain,
                    requireHeading: true,
                  });
                  assertNoExternalCdnsOrMockdata(await pageRes.page.content(), `Surface 1 ${r.path}`);

                  await runCheck(failures, `${r.slug}/${theme}/${vpName} tokens`, () =>
                    assertUiTokensPresent(pageRes.page, { role: "brand", theme })
                  );

                  if (r.slug === "home") {
                    await runCheck(failures, `${r.slug}/${theme}/${vpName} header buttons`, () =>
                      assertButtonHierarchyAndTypeFloor(
                        pageRes.page,
                        [".site-header [data-cart-toggle]", ".site-header [data-open-dialog='notifications-dialog']"],
                        { role: "brand", minHeight: 36 }
                      )
                    );
                  }
                  if (r.slug === "store") {
                    await runCheck(failures, `${r.slug}/${theme}/${vpName} store buttons`, () =>
                      assertButtonHierarchyAndTypeFloor(
                        pageRes.page,
                        ["[data-select-plan], [data-add-to-cart]"],
                        { role: "brand", minHeight: 40 }
                      )
                    );
                  }

                  await captureSurfaceScreenshot(
                    pageRes,
                    `surface1_${r.slug}_${theme}_${vpName}.png`,
                    { ctx: c, fullPage: true }
                  );
                } finally {
                  await pageRes.close();
                }
              }
            } finally {
              await context.close();
            }
          }
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U1 Surface 1 unification failures (${failures.length}): ${failures.slice(0, 6).join(" | ")}`
        );
      }
    );

    suite.test(
      "T4-U2: Surface 1 Interactive Overlays (#auth-dialog, #issue-dialog, #offer-dialog, #cart-drawer, #newsletter-dialog, #cookie-dialog, mega menu, mobile drawer, bottom-right queued .toast-region)",
      { req: ["F2", "F3", "R1", "R2"], readOnly: true },
      async (c) => {
        const failures = [];
        const deskCtx = await c.browser.newContext({ viewport: c.browser.VIEWPORT_DESKTOP, locale: "tr-TR" });
        try {
          await seedCookieConsent(deskCtx, { necessary: true, analytics: false, marketing: false });

          const homeRes = await openDomainPage(c, "/", { context: deskCtx, viewport: c.browser.VIEWPORT_DESKTOP });
          try {
            // 1. Desktop mega menu
            const megaDetails = homeRes.page.locator("nav#site-navigation details.nav-more").first();
            const megaSummary = megaDetails.locator("summary").first();
            if (await megaSummary.isVisible().catch(() => false)) {
              await megaSummary.click();
              await megaDetails.locator(".mega-menu").first().waitFor({ state: "visible", timeout: 5000 });
              await captureSurfaceScreenshot(homeRes, "surface1_mega_menu_desktop.png", { ctx: c, fullPage: false });
              await homeRes.page.keyboard.press("Escape");
              if (await megaDetails.evaluate((el) => el.hasAttribute("open")).catch(() => false)) {
                await megaSummary.click();
              }
            }

            // Dismiss floating ad popup if present so it does not intercept subsequent page clicks
            const floatClose = homeRes.page.locator("#ad-floating-popup [data-dismiss-floating-ad]").first();
            if (await floatClose.isVisible().catch(() => false)) {
              await floatClose.click().catch(() => {});
            }

            // 2. #cart-drawer via global header [data-cart-toggle]
            await runCheck(failures, "home global [data-cart-toggle]", async () => {
              const headerToggle = homeRes.page.locator(".site-header [data-cart-toggle], header [data-cart-toggle]").first();
              await headerToggle.waitFor({ state: "visible", timeout: 4000 });
              await headerToggle.click();
              await homeRes.page.locator("aside#cart-drawer").waitFor({ state: "visible", timeout: 5000 });
              await captureSurfaceScreenshot(homeRes, "surface1_cart_drawer_desktop.png", { ctx: c, fullPage: false });
              await homeRes.page.locator("aside#cart-drawer [data-cart-close]").first().click();
              await homeRes.page.locator("aside#cart-drawer").waitFor({ state: "hidden", timeout: 5000 });
            });

            // 3. #newsletter-dialog
            const nlBtn = homeRes.page.locator('[data-open-dialog="newsletter-dialog"]').first();
            if (await nlBtn.isVisible().catch(() => false)) {
              await nlBtn.click();
              await runCheck(failures, "#newsletter-dialog geometry", () =>
                assertModalGeometry(homeRes.page, "#newsletter-dialog", { minPadding: 16 })
              );
              await homeRes.page.locator("#newsletter-dialog [data-close-dialog]").first().click();
              await homeRes.page.locator("#newsletter-dialog").waitFor({ state: "hidden", timeout: 5000 });
            }

            // 4. #cookie-dialog
            const cookieBtn = homeRes.page.locator('[data-open-dialog="cookie-dialog"]').first();
            if (await cookieBtn.isVisible().catch(() => false)) {
              await cookieBtn.click();
              await runCheck(failures, "#cookie-dialog geometry", () =>
                assertModalGeometry(homeRes.page, "#cookie-dialog", { minPadding: 16 })
              );
              await homeRes.page.locator("#cookie-dialog [data-close-dialog]").first().click();
              await homeRes.page.locator("#cookie-dialog").waitFor({ state: "hidden", timeout: 5000 });
            }

            // 5. #notifications-dialog & bottom-right queued .toast-region
            await homeRes.page.locator('[data-open-dialog="notifications-dialog"]').first().click();
            await homeRes.page.locator("#notifications-dialog").waitFor({ state: "visible", timeout: 5000 });
            await homeRes.page.locator("#notifications-dialog [data-read-notifications]").first().click();
            await homeRes.page.locator("#notifications-dialog [data-close-dialog]").first().click();

            await runCheck(failures, "Surface 1 .toast-region contract", () =>
              assertToastContract(
                homeRes.page,
                ".toast-region",
                ".toast-region > *, .toast-region .toast, .toast-region .ui-toast",
                { expectedRole: "status" }
              )
            );
            assert.assertNoBrowserErrors(homeRes, "Surface 1 desktop overlays errors");
          } finally {
            await homeRes.close();
          }

          // 6. #auth-dialog on /hesabim guest (tabs + forgot password + Role A)
          const guestRes = await openDomainPage(c, "/hesabim", { context: deskCtx, viewport: c.browser.VIEWPORT_DESKTOP });
          try {
            await guestRes.page.locator('a[href="#modal-login"]').first().click();
            await runCheck(failures, "#auth-dialog geometry", () =>
              assertModalGeometry(guestRes.page, "#auth-dialog", { minPadding: 20 })
            );
            await guestRes.page.locator("#register-tab").click();
            await guestRes.page.locator("#register-form").waitFor({ state: "visible", timeout: 5000 });
            await guestRes.page.locator("#login-tab").click();
            await guestRes.page.locator('[data-auth-view="forgot"]').first().click();
            await guestRes.page.locator("#forgot-password-form").waitFor({ state: "visible", timeout: 5000 });
            await captureSurfaceScreenshot(guestRes, "surface1_auth_dialog_desktop.png", { ctx: c, fullPage: false });
            await guestRes.page.keyboard.press("Escape");
          } finally {
            await guestRes.close();
          }

          // 7. #issue-dialog on /e-dergi & #offer-dialog on /abonelik-ve-satis
          const edergiRes = await openDomainPage(c, "/e-dergi", { context: deskCtx, viewport: c.browser.VIEWPORT_DESKTOP });
          try {
            const openIssueBtn = edergiRes.page.locator("[data-open-issue]").first();
            await openIssueBtn.waitFor({ state: "visible", timeout: 8000 });
            await openIssueBtn.click();
            await runCheck(failures, "#issue-dialog geometry", () =>
              assertModalGeometry(edergiRes.page, "#issue-dialog", { minPadding: 16 })
            );
            await captureSurfaceScreenshot(edergiRes, "surface1_issue_dialog_desktop.png", { ctx: c, fullPage: false });
            await edergiRes.page.keyboard.press("Escape");
          } finally {
            await edergiRes.close();
          }

          const storeRes = await openDomainPage(c, "/abonelik-ve-satis", { context: deskCtx, viewport: c.browser.VIEWPORT_DESKTOP });
          try {
            const offerTrigger = storeRes.page.locator("[data-select-plan], [data-select-product]").first();
            await offerTrigger.waitFor({ state: "visible", timeout: 8000 });
            await offerTrigger.click();
            await runCheck(failures, "#offer-dialog geometry", () =>
              assertModalGeometry(storeRes.page, "#offer-dialog", { minPadding: 16 })
            );
            await captureSurfaceScreenshot(storeRes, "surface1_offer_dialog_desktop.png", { ctx: c, fullPage: false });
            await storeRes.page.keyboard.press("Escape");
          } finally {
            await storeRes.close();
          }
        } finally {
          await deskCtx.close();
        }

        // Mobile drawer
        const mobCtx = await c.browser.newContext({ viewport: c.browser.VIEWPORT_MOBILE, locale: "tr-TR" });
        try {
          await seedCookieConsent(mobCtx, { necessary: true, analytics: false, marketing: false });
          const mobHome = await openDomainPage(c, "/", { context: mobCtx, viewport: c.browser.VIEWPORT_MOBILE });
          try {
            await mobHome.page.locator("button.mobile-toggle").first().click();
            await mobHome.page.locator("nav#site-navigation.is-open").waitFor({ state: "visible", timeout: 5000 });
            assert.assertNoOverflow(await mobHome.measureOverflow(), "Surface 1 mobile drawer open overflow");
            await captureSurfaceScreenshot(mobHome, "surface1_mobile_drawer.png", { ctx: c, fullPage: false });
            await mobHome.page.locator("button.mobile-toggle").first().click();
          } finally {
            await mobHome.close();
          }
        } finally {
          await mobCtx.close();
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U2 Surface 1 overlay failures (${failures.length}): ${failures.join(" | ")}`
        );
      }
    );

    suite.test(
      "T4-U3: Surface 2 (/hesabim) Role A tokens, zero dark-mode FOUC flash, desktop quick-card no-scroll, #account-delete-confirm-dialog, bottom-right .ap-toast-region & screenshots",
      { req: ["F1", "F6", "F7", "F8", "R1", "R2"] },
      async (c) => {
        const failures = [];
        const reader = await resolveReaderClient(c, "registered");

        const ssrRes = await reader.client.get("/hesabim");
        assert.assertStatus(ssrRes, 200);
        await runCheck(failures, "/hesabim SSR dark FOUC check", () => {
          assert.assertNotIncludes(
            ssrRes.text.slice(0, 400),
            'data-theme="light"',
            '/hesabim SSR <html> must not hardcode data-theme="light" causing dark-mode FOUC'
          );
        });

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const vpName = vp.width >= 1024 ? "desktop" : "mobile";
          for (const theme of ["light", "dark"]) {
            const context = await c.browser.newContext({
              client: reader.client,
              viewport: vp,
              locale: "tr-TR",
            });
            try {
              await context.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_theme", t);
                  window.localStorage.setItem("msi_admin_theme", t);
                } catch {
                  // ignore
                }
              }, theme);

              const pageRes = await openDomainPage(c, "/hesabim", { context, viewport: vp });
              try {
                assert.assertEqual(pageRes.status, 200);
                const appliedTheme = await pageRes.page.locator("html").getAttribute("data-theme");
                assert.assertEqual(
                  appliedTheme,
                  theme,
                  `/hesabim (${vpName}) must hydrate directly into ${theme} theme without FOUC`
                );

                await auditUiSurface(c, pageRes, {
                  label: `Surface 2 /hesabim (${theme}/${vpName})`,
                  requireMain: true,
                  requireSingleH1: true,
                });

                await runCheck(failures, `hesabim/${theme}/${vpName} Role A tokens`, () =>
                  assertUiTokensPresent(pageRes.page, { role: "brand", theme })
                );
                await runCheck(failures, `hesabim/${theme}/${vpName} buttons`, () =>
                  assertButtonHierarchyAndTypeFloor(
                    pageRes.page,
                    ["#ap-theme-toggle", ".ap-logout-btn"],
                    { role: "brand", minHeight: 36 }
                  )
                );

                await captureSurfaceScreenshot(
                  pageRes,
                  `surface2_hesabim_${theme}_${vpName}.png`,
                  { ctx: c, fullPage: true }
                );

                if (vp.width >= 1024 && theme === "light") {
                  const scrollBefore = await pageRes.page.evaluate(() => window.scrollY);
                  await pageRes.page.locator('.ap-quick-card[data-ap-jump="edergi"]').first().click();
                  await pageRes.page.locator("#ap-panel-edergi").waitFor({ state: "visible", timeout: 5000 });
                  const scrollAfter = await pageRes.page.evaluate(() => window.scrollY);
                  await runCheck(failures, "desktop quick-card no-scroll", () => {
                    assert.assertOk(
                      Math.abs(scrollAfter - scrollBefore) <= 24,
                      `Desktop .ap-quick-card click must not trigger unwanted scroll jump (${scrollBefore} -> ${scrollAfter})`
                    );
                  });

                  for (const tabKey of ["profile", "notifications", "security"]) {
                    await pageRes.page.locator(`[data-ap-tab="${tabKey}"]`).first().click();
                    await pageRes.page.locator(`#ap-panel-${tabKey}`).waitFor({ state: "visible", timeout: 5000 });
                  }

                  await pageRes.page
                    .locator('#account-delete-form input[name="password"]')
                    .fill(reader.password);
                  const confirmErasure = pageRes.page.locator('#account-delete-form input[name="confirm_erasure"]');
                  if (await confirmErasure.count()) {
                    await confirmErasure.check();
                  }
                  await pageRes.page.locator('#account-delete-form button[type="submit"]').click();
                  await runCheck(failures, "#account-delete-confirm-dialog geometry", () =>
                    assertModalGeometry(pageRes.page, "#account-delete-confirm-dialog", { minPadding: 16 })
                  );
                  await captureSurfaceScreenshot(pageRes, "surface2_delete_modal_desktop.png", { ctx: c, fullPage: false });
                  await pageRes.page.locator("#account-delete-cancel-btn").click();
                  await pageRes.page.locator("#account-delete-confirm-dialog").waitFor({ state: "hidden", timeout: 5000 });

                  await pageRes.page.locator('[data-ap-tab="profile"]').first().click();
                  await pageRes.page.locator("#ap-panel-profile").waitFor({ state: "visible", timeout: 5000 });
                  await Promise.all([
                    pageRes.page.waitForResponse(
                      (r) => r.url().includes("/api/v1/account/profile") && r.status() === 200,
                      { timeout: 10000 }
                    ),
                    pageRes.page.locator('#account-profile-form button[type="submit"]').click(),
                  ]);
                  await runCheck(failures, "Surface 2 .ap-toast-region contract", () =>
                    assertToastContract(
                      pageRes.page,
                      ".ap-toast-region",
                      ".ap-toast-region .ap-toast",
                      { expectedRole: "status" }
                    )
                  );
                }
              } finally {
                await pageRes.close();
              }
            } finally {
              await context.close();
            }
          }
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U3 Surface 2 failures (${failures.length}): ${failures.slice(0, 6).join(" | ")}`
        );
      }
    );

    suite.test(
      "T4-U4: Surface 3 (/podmin) Role B tokens, #hero-quick-actions overflow accessibility, sticky wide .ops-table actions, #ops-confirm-modal, #totp-security-modal, .ops-toast-region & screenshots",
      { req: ["F1", "F9", "F10", "R1", "R2"] },
      async (c) => {
        const failures = [];
        const sa = await resolveAdminClient(c);

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const vpName = vp.width >= 1024 ? "desktop" : "mobile";
          for (const theme of ["light", "dark"]) {
            const pageRes = await openDomainPage(c, `/${c.adminPath}`, {
              client: sa.client,
              viewport: vp,
            });
            try {
              assert.assertEqual(pageRes.status, 200);
              await pageRes.page.waitForFunction(
                () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
                { timeout: 15000 }
              );
              await pageRes.setTheme(theme);

              await auditUiSurface(c, pageRes, {
                label: `Surface 3 /podmin (${theme}/${vpName})`,
                requireMain: false,
              });
              await runCheck(failures, `podmin/${theme}/${vpName} Role B tokens`, () =>
                assertUiTokensPresent(pageRes.page, { role: "ops", theme })
              );
              await runCheck(failures, `podmin/${theme}/${vpName} buttons`, () =>
                assertButtonHierarchyAndTypeFloor(
                  pageRes.page,
                  ["#topbar-refresh-btn", "#topbar-2fa-btn"],
                  { role: "ops", minHeight: 36 }
                )
              );

              await captureSurfaceScreenshot(
                pageRes,
                `surface3_podmin_${theme}_${vpName}.png`,
                { ctx: c, fullPage: true }
              );

              if (vp.width >= 1024 && theme === "light") {
                await runCheck(failures, "#hero-quick-actions overflow", async () => {
                  const heroAudit = await pageRes.page.evaluate(() => {
                    const strip = document.querySelector("#hero-quick-actions");
                    if (!strip) return { found: false };
                    const permitted = Array.from(strip.querySelectorAll("a, button")).filter(
                      (el) => el.getAttribute("data-rbac-hidden") !== "1"
                    );
                    const hiddenPermitted = permitted.filter((el) => {
                      const st = window.getComputedStyle(el);
                      return el.classList.contains("hidden") || el.hidden || st.display === "none";
                    });
                    const overflowTrigger = document.querySelector(
                      "#hero-quick-actions-more, [data-hero-overflow], .hero-more-actions, #hero-quick-actions details summary, #command-hero-strip [aria-haspopup]"
                    );
                    return {
                      found: true,
                      permittedCount: permitted.length,
                      hiddenPermittedCount: hiddenPermitted.length,
                      hasOverflowMenu: Boolean(overflowTrigger),
                    };
                  });
                  assert.assertOk(heroAudit.found, "#hero-quick-actions must exist on /podmin");
                  assert.assertOk(
                    heroAudit.hiddenPermittedCount === 0 || heroAudit.hasOverflowMenu,
                    `#hero-quick-actions silently hides ${heroAudit.hiddenPermittedCount}/${heroAudit.permittedCount} permitted actions without an accessible overflow menu`
                  );
                });

                await runCheck(failures, "wide .ops-table sticky actions", async () => {
                  const tableAudit = await pageRes.page.evaluate(() => {
                    const wrap = document.querySelector(".tab-pane.active .table-wrap, .table-wrap");
                    if (!wrap) return { hasTable: false };
                    const table = wrap.querySelector("table.ops-table");
                    if (!table) return { hasTable: false };
                    const lastActionCell = table.querySelector("tbody tr td:last-child, thead tr th:last-child");
                    if (!lastActionCell) return { hasTable: false };
                    const wrapRect = wrap.getBoundingClientRect();
                    const cellRect = lastActionCell.getBoundingClientRect();
                    const cellPos = window.getComputedStyle(lastActionCell).position;
                    return {
                      hasTable: true,
                      fitsWithoutScroll: table.scrollWidth <= wrap.clientWidth + 2,
                      isSticky: cellPos === "sticky",
                      reachableInWrap: cellRect.right <= wrapRect.right + 4,
                      tableScrollW: table.scrollWidth,
                      wrapClientW: wrap.clientWidth,
                    };
                  });
                  if (tableAudit.hasTable && !tableAudit.fitsWithoutScroll) {
                    assert.assertOk(
                      tableAudit.isSticky || tableAudit.reachableInWrap,
                      `Wide .ops-table action column must be sticky or reachable without clipping: ${JSON.stringify(tableAudit)}`
                    );
                  }
                });

                await pageRes.page.locator("#topbar-2fa-btn").click();
                await runCheck(failures, "#totp-security-modal geometry", () =>
                  assertModalGeometry(pageRes.page, "#totp-security-modal", { minPadding: 16 })
                );
                await captureSurfaceScreenshot(pageRes, "surface3_totp_modal_desktop.png", { ctx: c, fullPage: false });
                await pageRes.page.locator("#btn-close-2fa-modal").click();
                await pageRes.page.locator("#totp-security-modal").waitFor({ state: "hidden", timeout: 5000 });

                await pageRes.page.evaluate(() => {
                  if (window.MSIAdmin && typeof window.MSIAdmin.confirmDialog === "function") {
                    window.MSIAdmin.confirmDialog({
                      title: "E2E Onay Kontrolü",
                      message: "İşlem onay penceresi yerleşim testi.",
                    });
                  }
                  if (window.MSIAdmin && typeof window.MSIAdmin.showToast === "function") {
                    window.MSIAdmin.showToast("E2E bilgi bildirimi", "info");
                    window.MSIAdmin.showToast("E2E hata bildirimi", "error");
                  }
                });
                if (await pageRes.page.locator("#ops-confirm-modal").isVisible().catch(() => false)) {
                  await runCheck(failures, "#ops-confirm-modal geometry", () =>
                    assertModalGeometry(pageRes.page, "#ops-confirm-modal", { minPadding: 16 })
                  );
                  await pageRes.page.locator("#ops-confirm-cancel").click();
                }
                const toastCount = await pageRes.page.locator(".ops-toast-region .ops-toast").count();
                if (toastCount === 0) {
                  await pageRes.page.locator("#topbar-refresh-btn").click();
                }
                await runCheck(failures, ".ops-toast-region contract", () =>
                  assertToastContract(pageRes.page, ".ops-toast-region", ".ops-toast-region .ops-toast")
                );
              }
            } finally {
              await pageRes.close();
            }
          }
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U4 Surface 3 failures (${failures.length}): ${failures.slice(0, 6).join(" | ")}`
        );
      }
    );

    suite.test(
      "T4-U5: Surface 4 (/podmin/editor) WASM boot, nativeDialogs === 0 (#hf-editor-confirm-modal on delete/discard), bottom-right queued #hf-toast-slot, Role B tokens, button/type floor & screenshots",
      { req: ["F1", "F11", "F12", "R1", "R2"] },
      async (c) => {
        const failures = [];
        const sa = await resolveAdminClient(c);
        const slug = `t4-u5-${uid("ed")}`;
        const createRes = await sa.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Unification WASM Taslak ${slug}`,
            content: "<p>Editör stüdyo birleştirme testi.</p>",
            status: "draft",
            slug,
          },
        });
        assert.assertStatus(createRes, 200, "Ephemeral draft creation for Surface 4 check must return 200");
        const artId = String(createRes.json.id);

        try {
          for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
            const vpName = vp.width >= 1024 ? "desktop" : "mobile";
            for (const theme of ["light", "dark"]) {
              const pageRes = await openDomainPage(c, `/${c.adminPath}/editor?content_id=${artId}`, {
                client: sa.client,
                viewport: vp,
                autoAcceptDialogs: false,
              });
              try {
                assert.assertEqual(pageRes.status, 200);
                await pageRes.page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
                await pageRes.page.locator("#btn-new-article").waitFor({ state: "visible", timeout: 15000 });
                await pageRes.setTheme(theme);

                await auditUiSurface(c, pageRes, {
                  label: `Surface 4 /podmin/editor (${theme}/${vpName})`,
                  requireMain: false,
                  requireHeading: false,
                });
                await runCheck(failures, `editor/${theme}/${vpName} Role B tokens`, () =>
                  assertUiTokensPresent(pageRes.page, { role: "ops", theme })
                );
                await runCheck(failures, `editor/${theme}/${vpName} buttons & type floor`, async () => {
                  const btnSelectors =
                    vp.width >= 1024 ? ["#btn-new-article", "#btn-wizard-next"] : ["#btn-new-article"];
                  await assertButtonHierarchyAndTypeFloor(
                    pageRes.page,
                    btnSelectors,
                    { role: "ops", minHeight: 36, minFontSize: 11 }
                  );
                  const tinyLabels = await pageRes.page.evaluate(() => {
                    const hits = [];
                    for (const el of document.querySelectorAll("button, [role='tab'], .hf-wizard-step-btn")) {
                      const st = window.getComputedStyle(el);
                      const r = el.getBoundingClientRect();
                      if (st.display === "none" || st.visibility === "hidden" || r.width <= 0 || r.height <= 0) continue;
                      const fs = Number.parseFloat(st.fontSize) || 0;
                      if (fs > 0 && fs < 11) {
                        hits.push({ text: (el.innerText || "").trim().slice(0, 30), fontSize: fs });
                      }
                    }
                    return hits;
                  });
                  assert.assertEqual(
                    tinyLabels.length,
                    0,
                    `Surface 4 (${theme}/${vpName}) has sub-11px button/tab labels: ${JSON.stringify(tinyLabels)}`
                  );
                });

                await captureSurfaceScreenshot(
                  pageRes,
                  `surface4_editor_${theme}_${vpName}.png`,
                  { ctx: c, fullPage: true }
                );

                if (vp.width >= 1024 && theme === "light") {
                  await pageRes.page.locator("#btn-left-tab-drafts").click();
                  await pageRes.page.waitForTimeout(250);
                  const delBtn = pageRes.page.locator(`[data-action="delete-article"][data-article-id="${artId}"]`).first();
                  if (await delBtn.count()) {
                    await delBtn.click({ force: true });
                    await pageRes.page.waitForTimeout(300);
                    await runCheck(failures, "delete-article zero nativeConfirm", () =>
                      assert.assertEqual(
                        pageRes.nativeDialogs.length,
                        0,
                        `Surface 4 delete-article must NOT call window.confirm() (got ${JSON.stringify(pageRes.nativeDialogs)})`
                      )
                    );
                    if (await pageRes.page.locator("#hf-editor-confirm-modal").isVisible().catch(() => false)) {
                      await runCheck(failures, "delete-article #hf-editor-confirm-modal", () =>
                        assertModalGeometry(pageRes.page, "#hf-editor-confirm-modal", { minPadding: 16 })
                      );
                      await pageRes.page.locator('#hf-editor-confirm-modal [data-action="cancel-editor-confirm"]').first().click();
                      await pageRes.page.waitForTimeout(150);
                    } else {
                      failures.push("[delete-article #hf-editor-confirm-modal] modal did not open on delete-article");
                    }
                  }

                  await pageRes.page.locator("#editor-title-textarea").fill(`Dirty Unification ${slug}`);
                  await pageRes.page.locator("#editor-title-textarea").dispatchEvent("input");
                  await pageRes.page.locator("#btn-new-article").click();
                  await runCheck(failures, "discard #hf-editor-confirm-modal", () =>
                    assertModalGeometry(pageRes.page, "#hf-editor-confirm-modal", { minPadding: 16 })
                  );
                  await captureSurfaceScreenshot(pageRes, "surface4_confirm_modal_desktop.png", { ctx: c, fullPage: false });
                  await pageRes.page.locator('#hf-editor-confirm-modal [data-action="cancel-editor-confirm"]').first().click();

                  await pageRes.page.locator("#btn-settings-trigger").click();
                  await pageRes.page.locator("#hf-settings-popover").waitFor({ state: "visible", timeout: 8000 });
                  await pageRes.page.keyboard.press("Escape");

                  await pageRes.page.locator('[data-action="set-wizard-step"][data-step="2"]').click();
                  await pageRes.page.locator('[data-action="open-media-library"]').first().click();
                  await runCheck(failures, "#hf-media-modal geometry", () =>
                    assertModalGeometry(pageRes.page, "#hf-media-modal", { minPadding: 16 })
                  );
                  await pageRes.page.keyboard.press("Escape");

                  await pageRes.page.locator('[data-action="set-wizard-step"][data-step="4"]').click();
                  await pageRes.page.locator('[data-action="save-draft"]').first().waitFor({ state: "visible", timeout: 8000 });
                  await Promise.all([
                    pageRes.page.waitForResponse(
                      (r) => r.url().includes("/api/v1/admin/editorial/articles") && r.request().method() === "POST",
                      { timeout: 15000 }
                    ),
                    pageRes.page.locator('[data-action="save-draft"]').first().click(),
                  ]);
                  await runCheck(failures, "#hf-toast-slot contract", () =>
                    assertToastContract(
                      pageRes.page,
                      "#hf-toast-slot .hf-toast-region, #hf-toast-slot",
                      "#hf-toast-slot .hf-toast, #hf-toast-slot [role='status'], #hf-toast-slot [role='alert']"
                    )
                  );
                }

                await runCheck(failures, `editor/${theme}/${vpName} zero nativeDialogs`, () =>
                  assert.assertEqual(
                    pageRes.nativeDialogs.length,
                    0,
                    `Surface 4 must emit 0 native alert/confirm/prompt dialogs (got ${JSON.stringify(pageRes.nativeDialogs)})`
                  )
                );
              } finally {
                await pageRes.close();
              }
            }
          }
        } finally {
          await sa.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U5 Surface 4 failures (${failures.length}): ${failures.slice(0, 6).join(" | ")}`
        );
      }
    );

    suite.test(
      "T4-U6: Surface 5 (/podmin/tasarim) Role B tokens, studioConfirm() on section 'Sil' & top-nav 'Kaldır', floating bottom-right toast with zero #design-preview shift & screenshots",
      { req: ["F1", "F13", "R1", "R2"] },
      async (c) => {
        const failures = [];
        const sa = await resolveAdminClient(c);

        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const vpName = vp.width >= 1024 ? "desktop" : "mobile";
          for (const theme of ["light", "dark"]) {
            const pageRes = await openDomainPage(c, `/${c.adminPath}/tasarim`, {
              client: sa.client,
              viewport: vp,
              autoAcceptDialogs: false,
            });
            try {
              assert.assertEqual(pageRes.status, 200);
              await pageRes.page.waitForFunction(
                () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
                { timeout: 15000 }
              );
              await pageRes.setTheme(theme);

              await auditUiSurface(c, pageRes, {
                label: `Surface 5 /podmin/tasarim (${theme}/${vpName})`,
                requireMain: false,
                requireHeading: false,
              });
              await runCheck(failures, `tasarim/${theme}/${vpName} Role B tokens`, () =>
                assertUiTokensPresent(pageRes.page, { role: "ops", theme })
              );
              await runCheck(failures, `tasarim/${theme}/${vpName} buttons`, () =>
                assertButtonHierarchyAndTypeFloor(
                  pageRes.page,
                  ["#preview-button", "#save-button"],
                  { role: "ops", minHeight: 36 }
                )
              );

              await captureSurfaceScreenshot(
                pageRes,
                `surface5_tasarim_${theme}_${vpName}.png`,
                { ctx: c, fullPage: true }
              );

              if (vp.width >= 1024 && theme === "light") {
                // Verify #studio-tools-btn opens #studio-tools-menu with Export/Import/Reset tools cleanly
                await pageRes.page.locator("#studio-tools-btn").click();
                await pageRes.page.locator("#studio-tools-menu").waitFor({ state: "visible", timeout: 5000 });
                for (const toolId of ["#export-design", "#import-design-btn", "#restore-defaults"]) {
                  const vis = await pageRes.page.locator(toolId).isVisible();
                  assert.assertOk(vis, `Design Studio tools menu must expose ${toolId}`);
                }
                await pageRes.page.keyboard.press("Escape");
                await pageRes.page.locator("#studio-tools-menu").waitFor({ state: "hidden", timeout: 5000 });

                const previewTopBefore = await pageRes.page
                  .locator("#design-preview")
                  .evaluate((el) => el.getBoundingClientRect().top);

                // Duplicate first section so deleting the duplicate leaves original sections intact & triggers toast
                const firstSec = pageRes.page.locator("#section-list details.section-editor").first();
                await firstSec.locator("summary").click();
                await firstSec.locator('button:has-text("Kopyala")').click();
                await pageRes.page.locator("#section-list details.section-editor").nth(1).locator("summary").click();
                await pageRes.page.locator("#section-list details.section-editor").nth(1).locator("button.remove").click();

                await runCheck(failures, "section Sil #studio-confirm-modal", () =>
                  assertModalGeometry(pageRes.page, "#studio-confirm-modal", { minPadding: 16 })
                );
                await captureSurfaceScreenshot(pageRes, "surface5_confirm_modal_desktop.png", { ctx: c, fullPage: false });
                await pageRes.page.locator("#studio-confirm-accept").click();
                await pageRes.page.locator("#studio-confirm-modal").waitFor({ state: "hidden", timeout: 5000 });

                const previewTopAfter = await pageRes.page
                  .locator("#design-preview")
                  .evaluate((el) => el.getBoundingClientRect().top);

                await runCheck(failures, "#design-preview zero shift on feedback", () =>
                  assert.assertOk(
                    Math.abs(previewTopAfter - previewTopBefore) <= 1,
                    `#design-preview iframe shifted vertically on feedback (${previewTopBefore} -> ${previewTopAfter})`
                  )
                );

                await runCheck(failures, "#studio-toast-region contract", () =>
                  assertToastContract(
                    pageRes.page,
                    "#studio-toast-region, .ops-toast-region",
                    "#studio-toast-region .ops-toast, .ops-toast-region .ops-toast"
                  )
                );

                // Top-nav "Kaldır" must also open #studio-confirm-modal
                await pageRes.page.locator('[data-editor-tab="navigation"]').click();
                await pageRes.page.locator("#navigation-panel").waitFor({ state: "visible", timeout: 5000 });
                const navItems = pageRes.page.locator("#navigation-list details");
                if ((await navItems.count()) > 0) {
                  await navItems.first().locator("summary").click();
                  await navItems.first().locator('button:has-text("Kaldır"), button.remove').first().click();
                  await runCheck(failures, "top-nav Kaldır #studio-confirm-modal", () =>
                    assertModalGeometry(pageRes.page, "#studio-confirm-modal", { minPadding: 16 })
                  );
                  await pageRes.page.locator("#studio-confirm-cancel").click();
                  await pageRes.page.locator("#studio-confirm-modal").waitFor({ state: "hidden", timeout: 5000 });
                }
              }

              await runCheck(failures, `tasarim/${theme}/${vpName} zero nativeDialogs`, () =>
                assert.assertEqual(
                  pageRes.nativeDialogs.length,
                  0,
                  `Surface 5 must emit 0 native dialogs (got ${JSON.stringify(pageRes.nativeDialogs)})`
                )
              );
            } finally {
              await pageRes.close();
            }
          }
        }

        assert.assertEqual(
          failures.length,
          0,
          `T4-U6 Surface 5 failures (${failures.length}): ${failures.slice(0, 6).join(" | ")}`
        );
      }
    );
  });
}
