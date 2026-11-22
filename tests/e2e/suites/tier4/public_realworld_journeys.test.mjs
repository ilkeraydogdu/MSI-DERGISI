import {
  assertNoUnexpectedBrowserErrors,
  auditUiSurface,
  checkInteractiveVisibilityAndLayout,
  createAdminAd,
  createAdminContent,
  createVerifiedSubscriber,
  openDomainPage,
  seedCookieConsent,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 4 Real-World End-to-End & Multi-Domain Playwright Journeys", () => {
    suite.test(
      "T4-22: Public /e-dergi #issue-dialog interactive flipbook + page-range download selector on Desktop & Mobile",
      { req: ["REQ-018", "REQ-020", "REQ-004"] },
      async (c) => {
        const sub = await createVerifiedSubscriber(c, "subscriber_digital");
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const pageRes = await openDomainPage(c, "/e-dergi", {
            client: sub.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `/e-dergi (${vp.width})`,
              requireMain: true,
              requireSingleH1: true,
            });

            await pageRes.page.locator("[data-open-issue]").first().click();
            await pageRes.page.locator("#issue-dialog").waitFor({ state: "visible", timeout: 10000 });
            assert.assertNoOverflow(await pageRes.measureOverflow(), `/e-dergi #issue-dialog (${vp.width})`);

            // Assert #issue-tab-flipbook is visible and click it directly
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#issue-tab-flipbook", {
              label: `/e-dergi #issue-tab-flipbook (${vp.width})`,
            });
            await pageRes.page.locator("#issue-tab-flipbook").click();
            await pageRes.page.waitForTimeout(200);

            const dialogAudit = await pageRes.page.evaluate(() => {
              const dlg = document.querySelector("#issue-dialog");
              return {
                hasFlipbook: Boolean(
                  dlg?.querySelector(
                    "#issue-flipbook-panel, .edergi-flipbook-panel, .edergi-flipbook, [data-edergi-flipbook], #issue-flipbook-stage"
                  )
                ),
                hasPageRangeUi: Boolean(
                  dlg?.querySelector(
                    '#edergi-pages-input, input[name="edergi_export_mode"], input[name="pages"], select[name="pages"], [data-edergi-pages]'
                  )
                ),
              };
            });
            assert.assertOk(dialogAudit.hasFlipbook, "#issue-dialog must include interactive flipbook viewer (REQ-018)");
            assert.assertOk(dialogAudit.hasPageRangeUi, "#issue-dialog must include page-range selector (REQ-020)");
            assert.assertNoBrowserErrors(pageRes);
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-23: Article page mid-body RELATED (.msi-inline-related), bottom strip (>=3) & live ssb_tag telemetry beacon",
      { req: ["REQ-012", "REQ-024"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const tag = "radarlar-ve-sensorler";
        const slug = uid("t4-article-journey");
        await createAdminContent(c, op.client, {
          slug,
          title_tr: "AESA Radar Menzil ve Elektronik Karıştırma Analizi",
          ssb_tags: [tag],
          is_sponsored: false,
          body_tr:
            "<p>Birinci paragraf: Modern AESA radarlar çok hedefli takip kabiliyeti sunar.</p>" +
            "<p>İkinci paragraf: Galyum nitrür modüller yüksek çıkış gücü sağlar.</p>" +
            "<p>Üçüncü paragraf: Düşük algılanabilirlik modları elektronik harbe karşı direnci artırır.</p>" +
            "<p>Dördüncü paragraf: Platform entegrasyonu hava ve deniz unsurlarında hızla yaygınlaşmaktadır.</p>",
        });

        const context = await c.browser.newContext({
          viewport: c.browser.VIEWPORT_DESKTOP,
          locale: "tr-TR",
        });
        let pageRes = null;
        try {
          await seedCookieConsent(context, {
            necessary: true,
            analytics: true,
            marketing: true,
          });
          // Read-only navigator.sendBeacon spy so we can inspect Blob JSON payloads
          // (Playwright's Request.postData() returns null for sendBeacon Blob bodies in Chromium).
          await context.addInitScript(() => {
            const origSendBeacon = navigator.sendBeacon.bind(navigator);
            window.__e2eBeacons = [];
            navigator.sendBeacon = function (url, data) {
              const entry = { url: String(url), payload: null, _done: Promise.resolve() };
              window.__e2eBeacons.push(entry);
              if (typeof data === "string") {
                try {
                  entry.payload = JSON.parse(data);
                } catch {
                  entry.payload = null;
                }
              } else if (data && typeof data.text === "function") {
                entry._done = data
                  .text()
                  .then((txt) => {
                    try {
                      entry.payload = JSON.parse(txt);
                    } catch {
                      entry.payload = null;
                    }
                  })
                  .catch(() => {});
              }
              return origSendBeacon(url, data);
            };
          });

          pageRes = await openDomainPage(c, `/icerik/${slug}`, {
            context,
            viewport: c.browser.VIEWPORT_DESKTOP,
          });
          assert.assertEqual(pageRes.status, 200);
          await auditUiSurface(c, pageRes, {
            label: "Article detail Desktop",
            requireMain: true,
            requireSingleH1: true,
          });

          // Mid-body inline related + bottom recommendations >= 3 distinct
          const relatedAudit = await pageRes.page.evaluate(() => {
            const inlineEl = document.querySelector(".msi-article-body .msi-inline-related[data-related-inline], .msi-inline-related[data-related-inline]");
            const bottomLinks = Array.from(document.querySelectorAll(".msi-related-strip a[href], .related-strip a[href], section .stories a[href]"))
              .map((a) => a.getAttribute("href"))
              .filter(Boolean);
            const inlineLinks = inlineEl
              ? Array.from(inlineEl.querySelectorAll("a[href]")).map((a) => a.getAttribute("href")).filter(Boolean)
              : [];
            const distinct = new Set([...inlineLinks, ...bottomLinks]);
            const ssbAttr = document.querySelector(`[data-ssb-tag]`)?.getAttribute("data-ssb-tag") || null;
            return {
              hasInlineRelated: Boolean(inlineEl),
              distinctCount: distinct.size,
              ssbAttr,
            };
          });
          assert.assertOk(
            relatedAudit.hasInlineRelated,
            "Article page must render mid-body .msi-inline-related[data-related-inline] (M3 REQ-012)"
          );
          assert.assertOk(
            relatedAudit.distinctCount >= 3,
            `Expected >= 3 distinct related recommendations, got ${relatedAudit.distinctCount}`
          );
          assert.assertEqual(
            relatedAudit.ssbAttr,
            tag,
            "Article page must expose data-ssb-tag matching content ssb_tags[0] (M3 REQ-024)"
          );

          // Capture live /api/v1/analytics/track beacon response triggered by scrolling & pagehide
          const [trackRes] = await Promise.all([
            pageRes.page.waitForResponse(
              (res) =>
                res.url().includes("/api/v1/analytics/track") &&
                res.request().method() === "POST" &&
                res.status() === 200,
              { timeout: 10000 }
            ),
            pageRes.page.evaluate(() => {
              window.scrollTo(0, document.body.scrollHeight);
              window.dispatchEvent(new Event("scroll"));
              window.dispatchEvent(new Event("pagehide"));
            }),
          ]);
          assert.assertEqual(
            trackRes.status(),
            200,
            "POST /api/v1/analytics/track must return HTTP 200"
          );

          // Verify outgoing navigator.sendBeacon article_read payload ssb_tag === tag
          const beacons = await pageRes.page.evaluate(async () => {
            const list = Array.isArray(window.__e2eBeacons) ? window.__e2eBeacons : [];
            await Promise.all(list.map((item) => item._done).filter(Boolean));
            return list.map((item) => ({ url: item.url, payload: item.payload }));
          });
          const articleReadBeacon = beacons.find(
            (b) =>
              b.url.includes("/api/v1/analytics/track") &&
              b.payload?.event_type === "article_read" &&
              b.payload?.content_slug === slug
          );
          assert.assertOk(
            articleReadBeacon,
            `Live browser telemetry must send article_read beacon for slug "${slug}" (captured ${beacons.length} beacons)`
          );
          assert.assertEqual(
            articleReadBeacon.payload?.ssb_tag,
            tag,
            `Live browser telemetry beacon must send ssb_tag="${tag}" (not null) (M3 REQ-024)`
          );

          // Also verify backend persisted the article_read event with matching content_slug and ssb_tag
          let persistedEv = null;
          for (let attempt = 0; attempt < 15; attempt += 1) {
            const openDataRes = await op.client.get("/api/v1/admin/export/open-data");
            assert.assertStatus(openDataRes, 200, "Open-data export must succeed for admin");
            persistedEv = (openDataRes.json?.analytics_events || []).find(
              (ev) => ev.content_slug === slug && ev.event_type === "article_read"
            );
            if (persistedEv) break;
            await pageRes.page.waitForTimeout(150);
          }
          assert.assertOk(
            persistedEv,
            `Persisted analytics_events must contain article_read for slug "${slug}" (M3 REQ-024)`
          );
          assert.assertEqual(
            persistedEv.ssb_tag,
            tag,
            `Persisted analytics_events article_read.ssb_tag must equal "${tag}" (M3 REQ-024)`
          );
          assert.assertNoBrowserErrors(pageRes);
        } finally {
          if (pageRes) await pageRes.close();
          await context.close().catch(() => {});
        }
      }
    );

    suite.test(
      "T4-24: /abonelik-ve-satis multi-item cart drawer (#cart-drawer) checkout -> /hesabim invoice display (Desktop & Mobile)",
      { req: ["REQ-031", "REQ-028", "REQ-038"] },
      async (c) => {
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          const buyer = await createVerifiedSubscriber(c, "registered");
          const pageRes = await openDomainPage(c, "/abonelik-ve-satis", {
            client: buyer.client,
            viewport: vp,
          });
          try {
            assert.assertEqual(pageRes.status, 200);
            await auditUiSurface(c, pageRes, {
              label: `/abonelik-ve-satis (${vp.width}x${vp.height})`,
              requireMain: true,
              requireSingleH1: true,
            });

            const addBtns = pageRes.page.locator("button[data-add-to-cart]");
            const count = await addBtns.count();
            assert.assertOk(count >= 2, "Store must expose >= 2 [data-add-to-cart] products");
            await addBtns.nth(0).click();
            await pageRes.page.locator("#cart-drawer").waitFor({ state: "visible", timeout: 10000 });
            assert.assertNoOverflow(
              await pageRes.measureOverflow(),
              `/abonelik-ve-satis #cart-drawer open (${vp.width}) horizontal overflow`
            );

            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "[data-cart-close]", {
              label: `#cart-drawer close button (${vp.width})`,
            });
            await pageRes.page.locator("[data-cart-close]").first().click();
            await pageRes.page.locator("#cart-drawer").waitFor({ state: "hidden", timeout: 5000 });
            await addBtns.nth(1).click();

            await pageRes.page.locator("#cart-drawer").waitFor({ state: "visible", timeout: 10000 });
            await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#cart-checkout-form", {
              label: `#cart-checkout-form (${vp.width})`,
            });
            assert.assertNoOverflow(
              await pageRes.measureOverflow(),
              `/abonelik-ve-satis #cart-drawer multi-item (${vp.width}) horizontal overflow`
            );

            await pageRes.page.locator('#cart-checkout-form input[name="full_name"]').fill(buyer.user.full_name);
            await pageRes.page.locator('#cart-checkout-form input[name="email"]').fill(buyer.email);
            const consentBox = pageRes.page.locator('#cart-checkout-form input[name="distance_sales_consent"]');
            if (!(await consentBox.isChecked())) {
              await consentBox.check();
            }

            const [checkoutResp] = await Promise.all([
              pageRes.page.waitForResponse(
                (r) => r.url().includes("/api/v1/commerce/cart-checkout"),
                { timeout: 10000 }
              ),
              pageRes.page.locator('#cart-checkout-form button[type="submit"]').click(),
            ]);
            assert.assertOk(
              checkoutResp.status() === 200 || checkoutResp.status() === 202,
              `Cart checkout (${vp.width}) expected 200/202, got ${checkoutResp.status()}`
            );
            const coJson = await checkoutResp.json();
            const invoiceNo = coJson.invoice_no;
            assert.assertOk(Boolean(invoiceNo), "Cart checkout response must include invoice_no");
            assert.assertNoBrowserErrors(pageRes, `/abonelik-ve-satis (${vp.width}) browser errors`);

            // Navigate to /hesabim on same domain context and verify invoice appears
            await pageRes.page.goto(`${c.baseUrl}/hesabim`, { waitUntil: "networkidle" });
            const hesabimHtml = await pageRes.page.content();
            assert.assertIncludes(
              hesabimHtml,
              invoiceNo,
              `/hesabim (${vp.width}) must display newly created cart invoice`
            );
            assert.assertNoOverflow(
              await pageRes.measureOverflow(),
              `/hesabim after checkout (${vp.width}) horizontal overflow`
            );
          } finally {
            await pageRes.close();
          }
        }
      }
    );

    suite.test(
      "T4-25: Multi-domain live browser journeys (msidergisi.com vs savunmahaber.com) + view switch, dark mode & mobile toggle",
      { req: ["REQ-001", "REQ-004", "REQ-005"] },
      async (c) => {
        // 1. msidergisi.com Desktop: view switch & theme toggle
        const msiRes = await openDomainPage(c, "/", {
          host: "msidergisi.com",
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(msiRes.status, 200);
          await auditUiSurface(c, msiRes, {
            label: "msidergisi.com Desktop",
            requireMain: true,
            requireSingleH1: true,
          });

          await checkInteractiveVisibilityAndLayout(c, msiRes.page, "#btnSwitchHaber", {
            label: "msidergisi.com Desktop #btnSwitchHaber",
          });
          await msiRes.page.locator("#btnSwitchHaber").click();
          await msiRes.page.waitForTimeout(150);
          assert.assertNoOverflow(await msiRes.measureOverflow());

          await checkInteractiveVisibilityAndLayout(c, msiRes.page, "#btnToggleThemeMode");
          await msiRes.page.locator("#btnToggleThemeMode").click();
          const themeAttr = await msiRes.page.locator("html").getAttribute("data-theme");
          assert.assertEqual(themeAttr, "dark", "#btnToggleThemeMode must switch html[data-theme='dark']");
          assert.assertNoBrowserErrors(msiRes);
        } finally {
          await msiRes.close();
        }

        // 2. savunmahaber.com Mobile: mobile nav toggle & zero overflow
        const shMobRes = await openDomainPage(c, "/", {
          host: "savunmahaber.com",
          viewport: c.browser.VIEWPORT_MOBILE,
        });
        try {
          assert.assertEqual(shMobRes.status, 200);
          await auditUiSurface(c, shMobRes, {
            label: "savunmahaber.com Mobile",
            requireMain: true,
            requireSingleH1: true,
          });
          await checkInteractiveVisibilityAndLayout(c, shMobRes.page, ".mobile-toggle");
          await shMobRes.page.locator(".mobile-toggle").click();
          await shMobRes.page.waitForTimeout(150);
          assert.assertNoOverflow(await shMobRes.measureOverflow());
          assert.assertNoBrowserErrors(shMobRes);
        } finally {
          await shMobRes.close();
        }
      }
    );

    suite.test(
      "T4-26: English portal (/en, /en/search & Accept-Language: en-US) renders cleanly on Desktop & Mobile",
      { req: ["REQ-002", "REQ-001", "REQ-004"] },
      async (c) => {
        for (const path of ["/en", "/en/search"]) {
          for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
            const pageRes = await openDomainPage(c, path, {
              viewport: vp,
              locale: "en-US",
              extraHTTPHeaders: { "Accept-Language": "en-US,en;q=0.9" },
            });
            try {
              assert.assertEqual(pageRes.status, 200);
              const htmlLang = await pageRes.page.locator("html").getAttribute("lang");
              assert.assertEqual(htmlLang, "en", `${path} (${vp.width}) must render html[lang="en"]`);
              await auditUiSurface(c, pageRes, {
                label: `EN ${path} (${vp.width})`,
                requireMain: true,
                requireSingleH1: true,
              });
            } finally {
              await pageRes.close();
            }
          }
        }
      }
    );

    suite.test(
      "T4-27: End-to-end cross-role campaign: content_editor -> editor_in_chief -> ads_manager -> reader telemetry -> finance_viewer B2B",
      { req: ["REQ-009", "REQ-010", "REQ-022", "REQ-023", "REQ-025", "REQ-026"] },
      async (c) => {
        const editor = await c.auth.createUserWithRole("content_editor");
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const adsMgr = await c.auth.createUserWithRole("ads_manager");
        const finViewer = await c.auth.createUserWithRole("finance_viewer");

        // 1. content_editor drafts sponsored dossier
        const slug = uid("t4-cross-campaign");
        const draft = await createAdminContent(c, editor.client, {
          slug,
          content_type: "advertorial",
          placements: ["advertorial"],
          is_sponsored: true,
          sponsor_brand: "CrossRole Defense Co.",
          sponsor_legal_label: "SPONSORLU İÇERİK",
          ssb_tags: ["hava-araclari"],
          status: "draft",
        });

        // 2. Submit & editor_in_chief approves
        const subRev = await editor.client.post(`/api/v1/admin/contents/${draft.id}/submit-review`, { json: {} });
        assert.assertStatus(subRev, 200);
        const approvalId = subRev.json?.data?.approval_id;
        const appRes = await eic.client.post(`/api/v1/admin/approvals/${approvalId}/approve`, {
          json: { note: "Cross-role E2E onay" },
        });
        assert.assertStatus(appRes, 200);

        // 3. ads_manager provisions auxiliary fallback ad (with schedule)
        const auxAd = await createAdminAd(c, adsMgr.client, {
          sponsor_name: uid("CrossRoleAux"),
          sponsor_type: "auxiliary_service",
          is_default_fallback: true,
          start_date: "2026-01-01",
          end_date: "2026-12-31",
        });

        // 4. Verified subscriber reads article in browser & generates telemetry
        const reader = await createVerifiedSubscriber(c, "subscriber_digital");
        const pageRes = await openDomainPage(c, `/icerik/${slug}`, {
          client: reader.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
          seedConsent: { necessary: true, analytics: true, marketing: true },
        });
        try {
          assert.assertEqual(pageRes.status, 200);
          const html = await pageRes.page.content();
          assert.assertIncludes(html, "SPONSORLU İÇERİK");
          assert.assertNoBrowserErrors(pageRes);
        } finally {
          await pageRes.close();
        }

        await reader.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: uid("cross-sess"),
            event_type: "article_read",
            content_slug: slug,
            ad_id: auxAd.id,
            ssb_tag: "hava-araclari",
            duration_sec: 45,
            scroll_depth_pct: 92,
            video_watch_pct: 0,
            is_true_read: false,
            video_completed: false,
            organization_category: "",
            organization_name: "",
            created_at: "",
          },
        });

        // 5. finance_viewer inspects B2B sponsor report & anonymized reader report
        const b2bRes = await finViewer.client.get("/api/v1/admin/analytics/sponsor-report", {
          query: { anonymize: "true" },
        });
        assert.assertStatus(b2bRes, 200);
        assert.assertEqual(b2bRes.json?.anonymized, true);
        assert.assertOk(
          (b2bRes.json?.contents || []).some((r) => (r.content_slug || r.slug) === slug),
          "Campaign article must appear in finance_viewer B2B sponsor report"
        );
      }
    );

    suite.test(
      "T4-28: Full reader lifecycle: signup -> /hesabim profile & 2FA -> KVKK export -> browser KVKK erasure & token revocation",
      { req: ["REQ-015", "REQ-017", "REQ-032", "REQ-033"] },
      async (c) => {
        const reg = await c.auth.registerUser({
          email: `${uid("lifecycle")}@example.com`,
          full_name: "Yaşam Döngüsü Okuru",
          ssb_interests: ["hava-araclari"],
        });

        // Enroll TOTP, then re-login via Playwright credentials helper (exercising 2FA)
        const { secret } = await c.auth.enableTotp(reg.client);
        const capturedSession = reg.client.getRawCookie("msi_session");

        const pageRes = await openDomainPage(c, "/hesabim", {
          credentials: {
            email: reg.email,
            password: reg.password,
            totpSecret: secret,
          },
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);
          await auditUiSurface(c, pageRes, {
            label: "Lifecycle /hesabim",
            requireMain: true,
            requireSingleH1: true,
          });

          // Switch to M5 security tab and submit KVKK erasure in #account-delete-form
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, '[data-ap-tab="security"]', {
            label: "Lifecycle /hesabim security tab",
          });
          await pageRes.page.locator('[data-ap-tab="security"]').first().click();
          await pageRes.page.locator("#account-delete-form").waitFor({ state: "visible", timeout: 8000 });
          await pageRes.page.locator('#account-delete-form input[name="password"]').fill(reg.password);
          const confirmCheck = pageRes.page.locator('#account-delete-form input[name="confirm_erasure"]');
          if ((await confirmCheck.count()) > 0 && !(await confirmCheck.isChecked())) {
            await confirmCheck.check();
          }
          const [delResp] = await Promise.all([
            pageRes.page.waitForResponse(
              (r) => r.url().includes("/api/v1/account/delete") && r.status() === 200,
              { timeout: 10000 }
            ),
            (async () => {
              await pageRes.page.locator('#account-delete-form button[type="submit"]').click();
              const modalConfirm = pageRes.page.locator("#account-delete-confirm-btn");
              if (await modalConfirm.isVisible({ timeout: 3000 }).catch(() => false)) {
                await modalConfirm.click();
              }
            })(),
          ]);
          assert.assertEqual(delResp.status(), 200);

          assert.assertNoBrowserErrors(pageRes);
        } finally {
          await pageRes.close();
        }

        // Verify old session cookie cannot access /api/v1/account/export
        const staleClient = c.http.createClient();
        staleClient.setCookie("msi_session", capturedSession);
        const staleExport = await staleClient.get("/api/v1/account/export");
        assert.assertStatus(staleExport, 401, "Erased user's old session cookie must be rejected with 401");
      }
    );

    suite.test(
      "T4-36: Public #auth-dialog (login, register + .gov.tr #gov-domain-hint, forgot, totp, ?next=/e-dergi) & /sifre-sifirla mismatch/submit error UI",
      { req: ["F7", "REQ-015", "REQ-016", "REQ-017"] },
      async (c) => {
        // 1. Public #auth-dialog: open via /hesabim guest gate, .gov.tr hint, register, forgot, and TOTP login
        const homeRes = await openDomainPage(c, "/hesabim", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        const regEmail = `${uid("t4reg")}@example.com`;
        const regPass = "E2eDialog!2026";
        try {
          await homeRes.page.locator('a[href="#modal-login"]').first().click();
          await homeRes.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 5000 });
          await homeRes.page.locator("#register-tab").click();
          await homeRes.page.locator("#register-form").waitFor({ state: "visible", timeout: 5000 });

          // .gov.tr hint check
          await homeRes.page.locator('#register-form input[name="email"]').fill("uzman@ssb.gov.tr");
          await homeRes.page.locator('#register-form input[name="email"]').dispatchEvent("input");
          await homeRes.page.locator("#gov-domain-hint").waitFor({ state: "visible", timeout: 5000 });

          // Fill registration form & wait >= 2.2s for anti-spam timing token
          await homeRes.page.locator('#register-form input[name="full_name"]').fill("T4 Dialog Okuru");
          await homeRes.page.locator('#register-form input[name="email"]').fill(regEmail);
          await homeRes.page.locator('#register-form input[name="email"]').dispatchEvent("input");
          await homeRes.page.locator('#register-form input[name="password"]').fill(regPass);
          await homeRes.page.locator('#register-form input[name="job_title"]').fill("Mühendis");
          await homeRes.page.locator('#register-form input[name="organization_name"]').fill("E2E Savunma");
          await homeRes.page.locator('#register-form input[name="kvkk_consent"]').check();
          await homeRes.page.locator("#register-tos-accepted").check();
          await homeRes.page.waitForTimeout(2300);

          await Promise.all([
            homeRes.page.waitForResponse(
              (r) => r.url().includes("/api/v1/auth/register") && r.status() === 200,
              { timeout: 10000 }
            ),
            homeRes.page.locator('#register-form button[type="submit"]').click(),
          ]);
          await homeRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 5000 });

          // Forgot password view inside #auth-dialog
          await homeRes.page.locator('.auth-forgot-trigger[data-auth-view="forgot"]').click();
          await homeRes.page.locator("#forgot-password-form").waitFor({ state: "visible", timeout: 5000 });
          await homeRes.page.locator('#forgot-password-form input[name="email"]').fill(regEmail);
          await homeRes.page.waitForTimeout(2100);
          await Promise.all([
            homeRes.page.waitForResponse(
              (r) => r.url().includes("/api/v1/auth/forgot-password") && r.status() === 200,
              { timeout: 10000 }
            ),
            homeRes.page.locator('#forgot-password-form button[type="submit"]').click(),
          ]);
          await homeRes.page.locator('#forgot-password-form [data-auth-view="login"]').click();
          await homeRes.page.locator("#login-form").waitFor({ state: "visible", timeout: 5000 });

          // Enroll TOTP on a fresh user and exercise TOTP view inside #auth-dialog
          const totpUser = await createVerifiedSubscriber(c, "registered");
          const { secret } = await c.auth.enableTotp(totpUser.client);
          await homeRes.page.locator('#auth-dialog #login-form input[name="email"]').fill(totpUser.email);
          await homeRes.page.locator('#auth-dialog #login-form input[name="password"]').fill(totpUser.password);
          await homeRes.page.locator('#auth-dialog #login-form button[type="submit"]').click();
          await homeRes.page.locator("#totp-login-form").waitFor({ state: "visible", timeout: 8000 });
          await homeRes.page.locator('#totp-login-form input[name="code"]').fill(c.totp.generateTotp(secret));
          await Promise.all([
            homeRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            homeRes.page.locator('#totp-login-form button[type="submit"]').click(),
          ]);
          await homeRes.page.locator(".ap-topbar").waitFor({ state: "visible", timeout: 8000 });
          assert.assertNoAssetOrNetworkErrors(homeRes, [], "T4-36 #auth-dialog");
        } finally {
          await homeRes.close();
        }

        // 2. ?next=/e-dergi auto-open and post-login redirect
        const redirectUser = await createVerifiedSubscriber(c, "registered");
        const nextRes = await openDomainPage(c, "/hesabim?next=/e-dergi", {
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          await nextRes.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 8000 });
          await nextRes.page.locator('#auth-dialog #login-form input[name="email"]').fill(redirectUser.email);
          await nextRes.page.locator('#auth-dialog #login-form input[name="password"]').fill(redirectUser.password);
          await nextRes.page.waitForTimeout(2100);
          await Promise.all([
            nextRes.page.waitForNavigation({ waitUntil: "networkidle", timeout: 15000 }),
            nextRes.page.locator('#auth-dialog #login-form button[type="submit"]').click(),
          ]);
          assert.assertIncludes(nextRes.page.url(), "/e-dergi", "?next=/e-dergi must redirect to /e-dergi");
          assert.assertNoAssetOrNetworkErrors(nextRes, [], "T4-36 ?next=/e-dergi");
        } finally {
          await nextRes.close();
        }

        // 3. /sifre-sifirla password mismatch + submit error UI
        const resetRes = await openDomainPage(c, "/sifre-sifirla?token=invalid-reset-tok", {
          viewport: c.browser.VIEWPORT_MOBILE,
        });
        try {
          await auditUiSurface(c, resetRes, { label: "T4-36 /sifre-sifirla", requireMain: true });
          await resetRes.page.locator('#reset-password-form input[name="new_password"]').fill("ValidPass!2026");
          await resetRes.page.locator('#reset-password-form input[name="confirm_password"]').fill("Mismatch!2026");
          await resetRes.page.locator('#reset-password-form button[type="submit"]').click();
          await resetRes.page.locator("#reset-password-form .form-feedback.error").waitFor({ state: "visible", timeout: 5000 });

          await resetRes.page.locator('#reset-password-form input[name="confirm_password"]').fill("ValidPass!2026");
          await resetRes.page.locator('#reset-password-form button[type="submit"]').click();
          await resetRes.page.waitForResponse(
            (r) => r.url().includes("/api/v1/auth/reset-password") && (r.status() === 400 || r.status() === 404),
            { timeout: 10000 }
          );
          await resetRes.page.locator("#reset-password-form .form-feedback.error").waitFor({ state: "visible", timeout: 5000 });
          assertNoUnexpectedBrowserErrors(c, resetRes, ["400", "404"], "T4-36 /sifre-sifirla");
        } finally {
          await resetRes.close();
        }
      }
    );
  });
}
