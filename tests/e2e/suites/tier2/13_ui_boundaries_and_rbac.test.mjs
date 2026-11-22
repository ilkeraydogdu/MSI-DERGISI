import {
  assertModalGeometry,
} from "../../fixtures/design_tokens_contract.mjs";
import { openDomainPage } from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 2 UI Boundaries, Route Guards, RBAC & Viewport Containment", () => {
    suite.test(
      "T2-U1: Negative route & unauthenticated auth boundaries (/en/* 404s, studio 302 redirects, admin API 401)",
      { req: ["F4", "F14", "R2", "R3"], readOnly: true },
      async (c) => {
        const anon = c.http.createClient();

        // Unmounted /en/* aliases must 404 (validating why ?lang=en must be used)
        for (const deadRoute of ["/en/abonelik-ve-satis", "/en/e-magazine"]) {
          const r = await anon.get(deadRoute, { redirect: "manual" });
          assert.assertStatus(r, 404, `Unmounted ${deadRoute} must return 404`);
        }

        // Valid ?lang=en equivalents must return 200
        for (const validEnRoute of ["/abonelik-ve-satis?lang=en", "/e-dergi?lang=en"]) {
          const r = await anon.get(validEnRoute, { redirect: "manual" });
          assert.assertStatus(r, 200, `Mounted ${validEnRoute} must return 200`);
        }

        // Unauthenticated studio routes must redirect (302/303) to /{adminPath}
        for (const studioRoute of [`/${c.adminPath}/editor`, `/${c.adminPath}/tasarim`]) {
          const r = await anon.get(studioRoute, { redirect: "manual" });
          assert.assertStatus(r, [302, 303], `Unauthenticated GET ${studioRoute} must redirect 302/303`);
          const loc = r.headers.get("location") || "";
          assert.assertIncludes(
            loc,
            `/${c.adminPath}`,
            `Unauthenticated ${studioRoute} redirect location must target /${c.adminPath}`
          );
        }

        // Unauthenticated /api/v1/admin/me must return 401
        const meRes = await anon.get("/api/v1/admin/me");
        assert.assertStatus(meRes, 401, "Unauthenticated GET /api/v1/admin/me must return 401");
      }
    );

    suite.test(
      "T2-U2: Public Surface 1 & guest gateway boundary geometry at 1440x900 & 390x844 (zero overflow, #auth-dialog & mega menu containment)",
      { req: ["F2", "F5", "R2"], readOnly: true },
      async (c) => {
        for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
          for (const route of ["/", "/abonelik-ve-satis", "/e-dergi", "/hesabim", `/${c.adminPath}`]) {
            const pageRes = await openDomainPage(c, route, { viewport: vp });
            try {
              assert.assertEqual(pageRes.status, 200, `${route} (${vp.width}) must return 200`);
              assert.assertNoOverflow(
                await pageRes.measureOverflow(),
                `${route} (${vp.width}x${vp.height}) horizontal overflow`
              );
              assert.assertNoBrowserErrors(pageRes, `${route} (${vp.width}) browser errors`);

              if (route === "/" && vp.width >= 1024) {
                const megaSummary = pageRes.page.locator("nav#site-navigation details.nav-more summary").first();
                if (await megaSummary.isVisible().catch(() => false)) {
                  await megaSummary.click();
                  await pageRes.page.waitForTimeout(120);
                  const megaBox = await pageRes.page
                    .locator("nav#site-navigation details.nav-more .mega-menu")
                    .first()
                    .evaluate((el) => {
                      const r = el.getBoundingClientRect();
                      return {
                        width: r.width,
                        height: r.height,
                        left: r.left,
                        right: r.right,
                        innerWidth: window.innerWidth,
                      };
                    });
                  assert.assertOk(
                    megaBox.width > 0 && megaBox.left >= 0 && megaBox.right <= megaBox.innerWidth + 1,
                    `Desktop mega menu clipped outside viewport: ${JSON.stringify(megaBox)}`
                  );
                  await pageRes.page.keyboard.press("Escape");
                }
              }

              if (route === "/hesabim") {
                const loginTrigger = pageRes.page.locator('a[href="#modal-login"]').first();
                if (await loginTrigger.isVisible().catch(() => false)) {
                  await loginTrigger.click();
                  await assertModalGeometry(pageRes.page, "#auth-dialog", { minPadding: 16 });
                  await pageRes.page.keyboard.press("Escape");
                }
              }
            } finally {
              await pageRes.close();
            }
          }
        }
      }
    );

    suite.test(
      "T2-U3: RBAC matrix across registered, subscriber_public, finance_viewer, editor, site_operator & super_admin",
      { req: ["F14", "R3"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const op = await c.auth.loginAsAdmin();

        // Non-staff readers
        for (const readerRole of ["registered", "subscriber_public"]) {
          const reader = await c.auth.createUserWithRole(readerRole, { superAdminClient: sa.client });

          const editorRes = await reader.client.get(`/${c.adminPath}/editor`, { redirect: "manual" });
          assert.assertStatus(editorRes, [303, 403], `${readerRole} must be denied /${c.adminPath}/editor (303/403)`);
          if (editorRes.status === 303) {
            assert.assertIncludes(editorRes.headers.get("location") || "", `/${c.adminPath}`);
          }

          const studioRes = await reader.client.get(`/${c.adminPath}/tasarim`, { redirect: "manual" });
          assert.assertStatus(studioRes, [303, 403], `${readerRole} must be denied /${c.adminPath}/tasarim (303/403)`);
          if (studioRes.status === 303) {
            assert.assertIncludes(studioRes.headers.get("location") || "", `/${c.adminPath}`);
          }

          const adminMeRes = await reader.client.get("/api/v1/admin/me");
          assert.assertStatus(adminMeRes, 403, `${readerRole} must get 403 on /api/v1/admin/me`);

          const adminDesignRes = await reader.client.get("/api/v1/admin/design");
          assert.assertStatus(adminDesignRes, 403, `${readerRole} must get 403 on /api/v1/admin/design`);

          const gatewayRes = await reader.client.get(`/${c.adminPath}`);
          assert.assertStatus(gatewayRes, 200, `${readerRole} GET /${c.adminPath} renders login gateway`);
          assert.assertIncludes(
            gatewayRes.text,
            "login-form",
            `${readerRole} on /${c.adminPath} must see login-form rather than admin command shell`
          );
          assert.assertNotIncludes(
            gatewayRes.text,
            'id="admin-sidebar"',
            `${readerRole} on /${c.adminPath} must NOT receive #admin-sidebar command shell`
          );
        }

        // finance_viewer: staff role lacking contents:write -> 403 on /podmin/editor
        const fv = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
        const fvMe = await fv.client.get("/api/v1/admin/me");
        assert.assertStatus(fvMe, 200, "finance_viewer must authenticate on /api/v1/admin/me");
        const fvEditor = await fv.client.get(`/${c.adminPath}/editor`, { redirect: "manual" });
        assert.assertStatus(fvEditor, 403, "finance_viewer (no contents:write) must get 403 on /editor");

        // editor, site_operator, super_admin: permitted on /editor and link /css/design-tokens.css
        const ed = await c.auth.createUserWithRole("editor", { superAdminClient: sa.client });
        for (const actor of [
          { label: "editor", client: ed.client },
          { label: "site_operator", client: op.client },
          { label: "super_admin", client: sa.client },
        ]) {
          const edRes = await actor.client.get(`/${c.adminPath}/editor`);
          assert.assertStatus(edRes, 200, `${actor.label} must access /${c.adminPath}/editor (200)`);
          assert.assertIncludes(
            edRes.text,
            "/css/design-tokens.css",
            `${actor.label} /${c.adminPath}/editor HTML must link /css/design-tokens.css`
          );
        }
      }
    );

    suite.test(
      "T2-U4: Double-submit CSRF returns 403 when X-CSRF-Token is omitted on /api/v1/admin/design & /api/v1/account/*",
      { req: ["F14", "R3"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const reader = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });

        const designNoCsrf = await sa.client.post("/api/v1/admin/design", {
          csrf: false,
          json: {},
        });
        assert.assertStatus(
          designNoCsrf,
          403,
          "POST /api/v1/admin/design without X-CSRF-Token must fail with 403"
        );

        const accountNoCsrf = await reader.client.post("/api/v1/account/profile", {
          csrf: false,
          json: { full_name: "No CSRF Attempt" },
        });
        assert.assertStatus(
          accountNoCsrf,
          403,
          "POST /api/v1/account/profile without X-CSRF-Token must fail with 403"
        );
      }
    );

    suite.test(
      "T2-U5: Authenticated Surfaces 2-5 (/hesabim, /podmin, /podmin/editor, /podmin/tasarim) zero horizontal overflow & clipped nav/modals at 1440x900 & 390x844",
      { req: ["F6", "F10", "F12", "F13", "R2"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const reader = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });

        const targets = [
          { label: "Surface 2 /hesabim", route: "/hesabim", client: reader.client },
          { label: "Surface 3 /podmin", route: `/${c.adminPath}`, client: sa.client, waitAdmin: true },
          { label: "Surface 4 /podmin/editor", route: `/${c.adminPath}/editor`, client: sa.client, waitWasm: true },
          { label: "Surface 5 /podmin/tasarim", route: `/${c.adminPath}/tasarim`, client: sa.client, waitPreview: true },
        ];

        for (const t of targets) {
          for (const vp of [c.browser.VIEWPORT_DESKTOP, c.browser.VIEWPORT_MOBILE]) {
            const pageRes = await openDomainPage(c, t.route, {
              client: t.client,
              viewport: vp,
            });
            try {
              assert.assertEqual(pageRes.status, 200, `${t.label} (${vp.width}) must return 200`);
              if (t.waitAdmin) {
                await pageRes.page.waitForFunction(
                  () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
                  { timeout: 15000 }
                );
              }
              if (t.waitWasm) {
                await pageRes.page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
              }
              if (t.waitPreview) {
                await pageRes.page.waitForFunction(
                  () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
                  { timeout: 15000 }
                );
              }

              assert.assertNoOverflow(
                await pageRes.measureOverflow(),
                `${t.label} (${vp.width}x${vp.height}) horizontal overflow`
              );

              // Check no visible nav/tab/button text clips horizontally
              const clipped = await pageRes.page.evaluate(() => {
                const hits = [];
                for (const el of document.querySelectorAll("button, [role=tab], .sidebar-link, .ap-nav-btn")) {
                  const st = window.getComputedStyle(el);
                  const r = el.getBoundingClientRect();
                  if (st.display === "none" || st.visibility === "hidden" || r.width <= 0 || r.height <= 0) continue;
                  if (st.textOverflow === "ellipsis" || el.classList.contains("text-ellipsis")) continue;
                  if (el.clientWidth > 10 && el.scrollWidth > el.clientWidth + 4) {
                    hits.push((el.innerText || "").trim().slice(0, 40));
                  }
                }
                return hits;
              });
              assert.assertEqual(
                clipped.length,
                0,
                `${t.label} (${vp.width}) has clipped button/nav labels: ${clipped.join(", ")}`
              );
            } finally {
              await pageRes.close();
            }
          }
        }
      }
    );
  });
}
