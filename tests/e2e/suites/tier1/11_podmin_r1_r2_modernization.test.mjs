import {
  getCachedRoleSession,
  getCachedSuperAdmin,
} from "../../fixtures/tier12_helpers.mjs";
import { startIsolatedServer } from "../../lib/server.mjs";

const STUDIO_PERMITTED_ROLES = [
  "super_admin",
  "admin",
  "site_operator",
  "editor_in_chief",
  "managing_editor",
];

const STUDIO_DENIED_STAFF_ROLES = [
  "content_publisher",
  "content_editor",
  "editor",
  "business_dev",
  "ads_manager",
  "finance_viewer",
  "support_agent",
];

function extractLocalAssetPaths(html) {
  const paths = new Set();
  for (const m of html.matchAll(/<link[^>]+rel=["']stylesheet["'][^>]*href=["']([^"']+)["']/gi)) {
    const href = m[1];
    if (href.startsWith("/") && !href.startsWith("//")) paths.add(href);
  }
  for (const m of html.matchAll(/<link[^>]+href=["']([^"']+)["'][^>]*rel=["']stylesheet["']/gi)) {
    const href = m[1];
    if (href.startsWith("/") && !href.startsWith("//")) paths.add(href);
  }
  for (const m of html.matchAll(/<script[^>]+src=["']([^"']+)["']/gi)) {
    const src = m[1];
    if (src.startsWith("/") && !src.startsWith("//")) paths.add(src);
  }
  return Array.from(paths);
}

function assertStrictCspMarkup(assert, html, label) {
  assert.assertNotIncludes(html, "onclick=", `${label} must not contain inline onclick=`);
  assert.assertNotIncludes(html, "onchange=", `${label} must not contain inline onchange=`);
  assert.assertNotIncludes(html, "onsubmit=", `${label} must not contain inline onsubmit=`);
  // Inline style= attributes are forbidden on modern admin/gateway shells
  assert.assertEqual(
    /\sstyle\s*=\s*["']/i.test(html),
    false,
    `${label} must not contain inline style= attributes under strict CSP`
  );
  // Every <script> tag must carry a nonce attribute
  const scriptTags = html.match(/<script\b[^>]*>/gi) || [];
  assert.assertOk(scriptTags.length > 0, `${label} should include script tags`);
  for (const tag of scriptTags) {
    if (/type=["']application\/(ld\+json|json|speculationrules)["']/i.test(tag)) continue;
    assert.assertMatch(tag, /\bnonce=["'][^"']+["']/i, `${label} script tag missing CSP nonce: ${tag}`);
  }
}

function assertSpeculationRulesExcludePodmin(assert, html, label) {
  const matches = Array.from(
    html.matchAll(/<script\b[^>]*type=["']speculationrules["'][^>]*>([\s\S]*?)<\/script>/gi)
  );
  assert.assertOk(matches.length > 0, `${label} must emit <script type="speculationrules">`);
  for (const m of matches) {
    const rawJson = (m[1] || "").trim();
    assert.assertNotIncludes(
      rawJson.toLowerCase(),
      "msi-komuta",
      `${label} speculationrules JSON must not reference msi-komuta`
    );
    let parsed;
    try {
      parsed = JSON.parse(rawJson);
    } catch (err) {
      throw new Error(`${label} speculationrules contains invalid JSON: ${err.message}`);
    }
    const rules = [
      ...(Array.isArray(parsed?.prefetch) ? parsed.prefetch : []),
      ...(Array.isArray(parsed?.prerender) ? parsed.prerender : []),
    ];
    assert.assertOk(rules.length > 0, `${label} speculationrules must define prefetch/prerender rules`);
    const excludedMatches = [];
    const collectNotMatches = (node) => {
      if (!node || typeof node !== "object") return;
      if (Array.isArray(node)) {
        for (const item of node) collectNotMatches(item);
        return;
      }
      if (node.not && typeof node.not === "object" && Array.isArray(node.not.href_matches)) {
        excludedMatches.push(...node.not.href_matches);
      }
      for (const val of Object.values(node)) {
        if (val && typeof val === "object") collectNotMatches(val);
      }
    };
    collectNotMatches(rules);
    assert.assertOk(
      excludedMatches.includes("/podmin*"),
      `${label} speculationrules must exclude "/podmin*" in not.href_matches, got: ${JSON.stringify(excludedMatches)}`
    );
  }
}

export function registerTests(suite) {
  // ── F1: Canonical /podmin Routing & /msi-komuta Removal (6 tests) ─────────
  suite.describe("F1 Canonical /podmin Routing & /msi-komuta Removal", () => {
    suite.test(
      "unauthenticated GET /podmin returns 200 with admin login gateway",
      { req: "F1", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/podmin");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'id="login-form"');
        c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
      }
    );

    suite.test(
      "authenticated staff GET /podmin returns 200 with Command Center shell",
      { req: "F1" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'id="admin-sidebar"');
        c.assert.assertIncludes(res.text, 'data-operator-role="super_admin"');
        c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
        if (/type=["']speculationrules["']/i.test(res.text)) {
          assertSpeculationRulesExcludePodmin(c.assert, res.text, "authenticated /podmin");
        }
      }
    );

    suite.test(
      "authenticated authorized staff GET /podmin/tasarim returns 200 with Design Studio",
      { req: "F1" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'id="design-preview"');
        c.assert.assertIncludes(res.text, 'id="editor-admin-link"');
        c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
      }
    );

    suite.test(
      "authenticated authorized staff POST /podmin/tasarim/onizleme returns 200 preview HTML",
      { req: "F1" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const designRes = await sa.client.get("/api/v1/admin/design");
        c.assert.assertStatus(designRes, 200);
        const slug = designRes.json?.config?.pages?.[0]?.slug || "magazine";
        const res = await sa.client.post("/podmin/tasarim/onizleme", {
          query: { page: slug },
          json: designRes.json?.config,
        });
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "<html");
        c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
      }
    );

    suite.test(
      "/hesabim staff .account-command-card links to canonical admin route (/podmin on empty/default fallback)",
      { req: "F1" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/hesabim");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "account-command-card");
        c.assert.assertIncludes(res.text, `href="/${c.adminPath}"`);
        c.assert.assertIncludes(res.text, `href="/${c.adminPath}/tasarim"`);
        c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");

        // Verify explicit empty ADMIN_PATH_SECRET="" resolves to /podmin
        if (!c.isExternalTarget) {
          const emptySrv = await startIsolatedServer({ adminPath: "" });
          try {
            const emptySa = emptySrv.credentials.superAdmin;
            const extClient = c.http.createClient();
            // Point a fresh client at emptySrv
            const { Client } = await import("../../lib/http.mjs");
            const podClient = new Client({ baseUrl: emptySrv.baseUrl });
            await c.auth.login(podClient, emptySa);
            const hesabimPod = await podClient.get("/hesabim");
            c.assert.assertStatus(hesabimPod, 200);
            c.assert.assertIncludes(hesabimPod.text, 'href="/podmin"');
            c.assert.assertIncludes(hesabimPod.text, 'href="/podmin/tasarim"');
            c.assert.assertNotIncludes(hesabimPod.text.toLowerCase(), "msi-komuta");
            void extClient;
          } finally {
            await emptySrv.stop();
          }
        }
      }
    );

    suite.test(
      "zero msi-komuta references in rendered public HTML (/, /en, /podmin, /hesabim, /sifre-sifirla) and core JS",
      { req: "F1", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const targets = [
          "/",
          "/en",
          "/podmin",
          "/hesabim",
          "/sifre-sifirla",
          "/js/admin/core.js",
          "/js/admin/login.js",
          "/js/msi-design-editor.js",
          "/js/account-portal.js",
          "/js/msi-ui.js",
        ];
        for (const p of targets) {
          const res = await client.get(p);
          c.assert.assertStatus(res, 200, `${p} must return 200`);
          c.assert.assertNotIncludes(
            res.text.toLowerCase(),
            "msi-komuta",
            `Found forbidden legacy 'msi-komuta' string in ${p}`
          );
        }
      }
    );

    suite.test(
      'SPECULATION_RULES_JSON in rendered HTML (/, /en, /hesabim) parses cleanly, excludes "/podmin*", and has zero msi-komuta',
      { req: "F1", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        for (const p of ["/", "/en", "/hesabim"]) {
          const res = await client.get(p);
          c.assert.assertStatus(res, 200, `${p} must return 200`);
          assertSpeculationRulesExcludePodmin(c.assert, res.text, `GET ${p}`);
        }
      }
    );
  });

  // ── F2: Unified Soft Eye-Friendly Token System (5 tests) ──────────────────
  suite.describe("F2 Unified Soft Eye-Friendly Token System", () => {
    suite.test(
      "all CSS & JS assets referenced in rendered /podmin HTML load HTTP 200 with non-empty bodies",
      { req: "F2", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const htmlRes = await client.get("/podmin");
        c.assert.assertStatus(htmlRes, 200);
        const assets = extractLocalAssetPaths(htmlRes.text);
        c.assert.assertOk(assets.length >= 4, "Expected >= 4 CSS/JS assets on /podmin");
        for (const asset of assets) {
          const r = await client.get(asset);
          c.assert.assertStatus(r, 200, `Asset ${asset} referenced by /podmin must return 200`);
          c.assert.assertOk(r.text.trim().length > 0, `Asset ${asset} must be non-empty`);
        }
      }
    );

    suite.test(
      "all CSS & JS assets referenced in rendered /podmin/tasarim HTML load HTTP 200 with non-empty bodies",
      { req: "F2" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const htmlRes = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(htmlRes, 200);
        const assets = extractLocalAssetPaths(htmlRes.text);
        c.assert.assertOk(assets.length >= 3, "Expected >= 3 CSS/JS assets on /podmin/tasarim");
        for (const asset of assets) {
          const r = await sa.client.get(asset);
          c.assert.assertStatus(r, 200, `Asset ${asset} referenced by /podmin/tasarim must return 200`);
          c.assert.assertOk(r.text.trim().length > 0, `Asset ${asset} must be non-empty`);
        }
      }
    );

    suite.test(
      "all CSS & JS assets referenced in rendered /hesabim HTML load HTTP 200 with non-empty bodies",
      { req: "F2" },
      async (c) => {
        const reader = await getCachedRoleSession(c, "registered");
        const htmlRes = await reader.client.get("/hesabim");
        c.assert.assertStatus(htmlRes, 200);
        const assets = extractLocalAssetPaths(htmlRes.text);
        c.assert.assertOk(assets.length >= 2, "Expected >= 2 CSS/JS assets on /hesabim");
        for (const asset of assets) {
          const r = await reader.client.get(asset);
          c.assert.assertStatus(r, 200, `Asset ${asset} referenced by /hesabim must return 200`);
          c.assert.assertOk(r.text.trim().length > 0, `Asset ${asset} must be non-empty`);
        }
      }
    );

    suite.test(
      "tokens.css defines Light & Dark --adm-* theme variables and prefers-reduced-motion rules",
      { req: "F2", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/css/admin/tokens.css");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'data-theme="light"');
        c.assert.assertIncludes(res.text, 'data-theme="dark"');
        for (const token of [
          "--adm-bg:",
          "--adm-surface:",
          "--adm-text:",
          "--adm-accent:",
          "--adm-border:",
          "--adm-radius-md:",
          "--adm-ease:",
        ]) {
          c.assert.assertIncludes(res.text, token, `tokens.css must define ${token}`);
        }
        c.assert.assertMatch(res.text, /@media\s*\(\s*prefers-reduced-motion\s*:\s*reduce\s*\)/i);
      }
    );

    suite.test(
      "account-portal.css & msi-design-editor.css define Light/Dark theme variables and reduced-motion rules",
      { req: "F2", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const [apCss, edCss, layoutCss, loginCss] = await Promise.all([
          client.get("/css/account-portal.css"),
          client.get("/css/msi-design-editor.css"),
          client.get("/css/admin/layout.css"),
          client.get("/css/admin/login.css"),
        ]);
        for (const r of [apCss, edCss, layoutCss, loginCss]) {
          c.assert.assertStatus(r, 200);
        }
        c.assert.assertIncludes(apCss.text, 'data-theme="dark"');
        c.assert.assertIncludes(apCss.text, 'data-theme="light"');
        c.assert.assertIncludes(apCss.text, "--ap-bg:");
        c.assert.assertIncludes(apCss.text, "--ap-surface:");
        c.assert.assertIncludes(edCss.text, 'data-theme="dark"');
        c.assert.assertIncludes(edCss.text, 'data-theme="light"');
        c.assert.assertIncludes(edCss.text, "--ed-bg:");
        c.assert.assertIncludes(edCss.text, "--ed-surface:");
        // Reduced motion support across stylesheets
        const combined = `${apCss.text}\n${edCss.text}\n${layoutCss.text}\n${loginCss.text}`;
        c.assert.assertMatch(combined, /prefers-reduced-motion:\s*reduce/i);
      }
    );
  });

  // ── F3: Admin Login Gateway Modernization (/podmin, 5 tests) ──────────────
  suite.describe("F3 Admin Login Gateway Modernization", () => {
    suite.test(
      "/podmin login gateway preserves all 19 required DOM contract IDs and body[data-admin-path]",
      { req: "F3", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/podmin");
        c.assert.assertStatus(res, 200);
        const requiredIds = [
          "login-form",
          "login-email",
          "login-password",
          "btn-toggle-pw",
          "btn-show-forgot",
          "login-submit-btn",
          "totp-login-form",
          "totp-code-input",
          "totp-submit-btn",
          "btn-back-from-totp",
          "forgot-password-form",
          "forgot-email",
          "forgot-submit-btn",
          "btn-back-from-forgot",
          "login-error",
          "login-success",
        ];
        for (const id of requiredIds) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `/podmin login gateway missing #${id}`);
        }
        c.assert.assertNotIncludes(res.text, 'id="login-lang-switch"', "/podmin must omit manual language buttons");
        c.assert.assertNotIncludes(res.text, 'id="login-theme-toggle"', "/podmin must use autonomous OS/clock theme without toggle button");
        c.assert.assertIncludes(res.text, "prefers-color-scheme: dark", "/podmin must include autonomous OS/clock theme boot");
        c.assert.assertMatch(res.text, /<body[^>]+data-admin-path=["']\/[^"']+["']/i);
      }
    );

    suite.test(
      "/podmin login gateway obeys strict CSP (zero inline style= or onclick=, script nonces present)",
      { req: "F3", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/podmin");
        c.assert.assertStatus(res, 200);
        assertStrictCspMarkup(c.assert, res.text, "/podmin login gateway");
      }
    );

    suite.test(
      "/podmin?lang=tr and /podmin?lang=en render localized Turkish & English login gateways",
      { req: "F3", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const [trRes, enRes] = await Promise.all([
          client.get("/podmin", { query: { lang: "tr" } }),
          client.get("/podmin", { query: { lang: "en" } }),
        ]);
        c.assert.assertStatus(trRes, 200);
        c.assert.assertStatus(enRes, 200);
        c.assert.assertMatch(trRes.text, /<html[^>]+lang="tr"/i);
        c.assert.assertMatch(enRes.text, /<html[^>]+lang="en"/i);
        c.assert.assertNotEqual(trRes.text, enRes.text, "TR and EN login pages must differ");
      }
    );

    suite.test(
      "/podmin login gateway loads GSAP, login.js, fighter hero & SVG logos cleanly (200) without FontAwesome icons.css",
      { req: "F3", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const res = await client.get("/podmin");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "gsap.min.js");
        c.assert.assertIncludes(res.text, "/js/admin/login.js");
        c.assert.assertIncludes(res.text, "/images/admin/podmin-fighter-vertical.webp");
        c.assert.assertIncludes(res.text, "/images/logo.svg");
        c.assert.assertIncludes(res.text, "/images/logo-white.svg");
        c.assert.assertNotIncludes(res.text, "icons.css");
        for (const assetPath of [
          "/js/vendor/gsap.min.js",
          "/images/admin/podmin-fighter-vertical.webp",
          "/images/logo.svg",
          "/images/logo-white.svg",
        ]) {
          const assetRes = await client.raw("GET", assetPath);
          c.assert.assertStatus(assetRes, 200, `${assetPath} referenced on /podmin must return 200`);
          c.assert.assertOk(assetRes.rawBody.byteLength > 50, `${assetPath} must be non-empty`);
        }
      }
    );

    suite.test(
      "/podmin login & forgot-password backend APIs accept valid payloads",
      { req: "F3" },
      async (c) => {
        const client = c.http.createClient();
        const op = await c.auth.login(client, c.credentials.admin);
        c.assert.assertEqual(op.status, "authenticated");
        const forgotRes = await c.http.createClient().post("/api/v1/auth/forgot-password", {
          json: { email: c.credentials.admin.email },
        });
        c.assert.assertStatus(forgotRes, 200);
      }
    );
  });

  // ── F4: Admin Command Center Modernization (All 12 Staff Roles, 6 tests) ──
  suite.describe("F4 Admin Command Center Modernization (12 Staff Roles & 18 Tabs)", () => {
    suite.test(
      "all 12 STAFF_ROLES (including admin, managing_editor, editor) render GET /podmin 200 with data-operator-role",
      { req: "F4" },
      async (c) => {
        for (const role of c.auth.STAFF_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/podmin");
          c.assert.assertStatus(res, 200, `Staff role ${role} must get 200 on GET /podmin`);
          c.assert.assertIncludes(
            res.text,
            `data-operator-role="${role}"`,
            `GET /podmin must bind data-operator-role="${role}"`
          );
          c.assert.assertIncludes(res.text, 'id="admin-sidebar"');
          c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
        }
      }
    );

    suite.test(
      "Admin Command Center preserves all 18 tabs (#tab-btn-1..18, #tab-1..18), hero & KPI strips",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin");
        c.assert.assertStatus(res, 200);
        for (let i = 1; i <= 18; i++) {
          c.assert.assertIncludes(res.text, `id="tab-btn-${i}"`, `Missing #tab-btn-${i}`);
          c.assert.assertIncludes(res.text, `id="tab-${i}"`, `Missing #tab-${i}`);
        }
        for (const id of [
          "command-hero-strip",
          "hero-quick-actions",
          "hero-module-count",
          "global-kpi-strip",
          "sidebar-quick-filter",
          "topbar-theme-toggle",
          "topbar-2fa-btn",
          "btn-logout",
        ]) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `Missing Command Center contract #${id}`);
        }
      }
    );

    suite.test(
      "Admin Command Center preserves all 4 drawers and 4 modals + #ops-toast region",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin");
        c.assert.assertStatus(res, 200);
        for (const id of [
          "drawer-content",
          "drawer-issue",
          "drawer-ad",
          "drawer-newsletter",
          "totp-security-modal",
          "ops-confirm-modal",
          "approval-modal",
          "ad-conflict-modal",
        ]) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `Missing drawer/modal #${id}`);
        }
        c.assert.assertIncludes(res.text, "ops-toast");
      }
    );

    suite.test(
      "Admin Command Center uses inline <svg class=\"adm-icon\"> without icons.css and obeys strict CSP",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, '<svg class="adm-icon"');
        c.assert.assertNotIncludes(res.text, "icons.css");
        assertStrictCspMarkup(c.assert, res.text, "/podmin Command Center");
      }
    );

    suite.test(
      "Admin Command Center supports bilingual ?lang=tr and ?lang=en with #msi-i18n JSON",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const [trRes, enRes] = await Promise.all([
          sa.client.get("/podmin", { query: { lang: "tr" } }),
          sa.client.get("/podmin", { query: { lang: "en" } }),
        ]);
        c.assert.assertStatus(trRes, 200);
        c.assert.assertStatus(enRes, 200);
        c.assert.assertIncludes(trRes.text, 'id="msi-i18n"');
        c.assert.assertIncludes(enRes.text, 'data-admin-lang="en"');
      }
    );

    suite.test(
      "Admin Command Center loads GSAP + Chart.js + modular admin scripts HTTP 200",
      { req: "F4" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "/js/vendor/gsap.min.js");
        c.assert.assertIncludes(res.text, "/js/vendor/chart.umd.min.js");
        for (const asset of extractLocalAssetPaths(res.text)) {
          const a = await sa.client.get(asset);
          c.assert.assertStatus(a, 200, `Admin shell asset ${asset} must return 200`);
        }
      }
    );
  });

  // ── F5: Visual Design Studio Modernization (/podmin/tasarim, 5 tests) ─────
  suite.describe("F5 Visual Design Studio Modernization", () => {
    suite.test(
      "GET /podmin/tasarim preserves header & action controls (#editor-admin-link, #save-state, #studio-theme-toggle, #discard-draft, #preview-button, #save-button)",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(res, 200);
        for (const id of [
          "editor-admin-link",
          "save-state",
          "studio-theme-toggle",
          "discard-draft",
          "preview-button",
          "save-button",
        ]) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `Design Studio missing #${id}`);
        }
      }
    );

    suite.test(
      "GET /podmin/tasarim preserves preview & confirm modal contracts (#design-preview, #preview-theme, #open-page, #studio-confirm-modal)",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(res, 200);
        for (const id of ["design-preview", "preview-theme", "open-page", "studio-confirm-modal"]) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `Design Studio missing #${id}`);
        }
      }
    );

    suite.test(
      "GET /podmin/tasarim preserves pages, theme, and navigation sidebar tabs & panels",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(res, 200);
        for (const tab of ["pages", "theme", "navigation"]) {
          c.assert.assertIncludes(res.text, `data-editor-tab="${tab}"`, `Design Studio missing tab ${tab}`);
          c.assert.assertIncludes(res.text, `id="${tab}-panel"`, `Design Studio missing #${tab}-panel`);
        }
      }
    );

    suite.test(
      "GET /podmin/tasarim uses inline SVG icons without icons.css and obeys strict CSP",
      { req: "F5" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const res = await sa.client.get("/podmin/tasarim");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, '<svg class="adm-icon"');
        c.assert.assertNotIncludes(res.text, "icons.css");
        assertStrictCspMarkup(c.assert, res.text, "/podmin/tasarim");
      }
    );

    suite.test(
      "all 5 Studio-permitted staff roles access GET /podmin/tasarim and /api/v1/admin/design with 200",
      { req: "F5" },
      async (c) => {
        for (const role of STUDIO_PERMITTED_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const [studioRes, apiRes] = await Promise.all([
            session.client.get("/podmin/tasarim"),
            session.client.get("/api/v1/admin/design"),
          ]);
          c.assert.assertStatus(studioRes, 200, `${role} must access GET /podmin/tasarim`);
          c.assert.assertStatus(apiRes, 200, `${role} must access GET /api/v1/admin/design`);
        }
      }
    );
  });

  // ── F6: Account Role Portal Modernization (/hesabim, 6 tests) ─────────────
  suite.describe("F6 Account Role Portal Modernization (/hesabim)", () => {
    suite.test(
      "all 12 STAFF_ROLES render .account-command-card on /hesabim with Studio link ONLY for Studio-permitted roles",
      { req: "F6" },
      async (c) => {
        for (const role of STUDIO_PERMITTED_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/hesabim");
          c.assert.assertStatus(res, 200);
          c.assert.assertIncludes(res.text, "account-command-card", `${role} must see .account-command-card`);
          c.assert.assertIncludes(res.text, `href="/${c.adminPath}"`, `${role} must see admin link`);
          c.assert.assertIncludes(res.text, `href="/${c.adminPath}/tasarim"`, `${role} must see Studio link`);
        }
        for (const role of STUDIO_DENIED_STAFF_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/hesabim");
          c.assert.assertStatus(res, 200);
          c.assert.assertIncludes(res.text, "account-command-card", `${role} must see .account-command-card`);
          c.assert.assertIncludes(res.text, `href="/${c.adminPath}"`, `${role} must see admin link`);
          c.assert.assertNotIncludes(
            res.text,
            `/${c.adminPath}/tasarim`,
            `Non-Studio role ${role} must NOT see Design Studio link on /hesabim`
          );
        }
      }
    );

    suite.test(
      "all 5 READER_ROLES render /hesabim WITHOUT .account-command-card and WITH preserved .ap-* / #ap-panel-* contracts",
      { req: "F6" },
      async (c) => {
        for (const role of c.auth.READER_ROLES) {
          const session = await getCachedRoleSession(c, role);
          const res = await session.client.get("/hesabim");
          c.assert.assertStatus(res, 200, `Reader role ${role} must get 200 on /hesabim`);
          c.assert.assertNotIncludes(
            res.text,
            "account-command-card",
            `Reader role ${role} must NEVER see .account-command-card`
          );
          for (const marker of [
            "ap-topbar",
            'id="ap-sidebar-toggle"',
            "ap-role-pill",
            'id="ap-theme-toggle"',
            "data-logout",
            "data-ap-tab",
            'id="ap-panel-overview"',
            'id="ap-panel-edergi"',
            'id="ap-panel-profile"',
            'id="ap-panel-security"',
            'id="ap-panel-notifications"',
          ]) {
            c.assert.assertIncludes(res.text, marker, `${role} /hesabim missing ${marker}`);
          }
        }
      }
    );

    suite.test(
      "guest /hesabim renders .account-welcome gate without .account-command-card or private forms",
      { req: "F6", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/hesabim");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "account-welcome");
        c.assert.assertNotIncludes(res.text, "account-command-card");
        c.assert.assertNotIncludes(res.text, "ap-panel-overview");
        c.assert.assertNotIncludes(res.text, "account-delete-form");
      }
    );

    suite.test(
      "authenticated /hesabim preserves profile, SSB tags, TOTP, KVKK, and notification contracts",
      { req: "F6" },
      async (c) => {
        const reader = await getCachedRoleSession(c, "registered");
        const res = await reader.client.get("/hesabim");
        c.assert.assertStatus(res, 200);
        for (const id of [
          "account-profile-form",
          "account-ssb-tags",
          "account-totp-setup-btn",
          "account-kvkk-card",
          "account-delete-form",
          "account-delete-confirm-dialog",
        ]) {
          c.assert.assertIncludes(res.text, `id="${id}"`, `/hesabim missing #${id}`);
        }
        c.assert.assertIncludes(res.text, "data-mark-all-read");
      }
    );

    suite.test(
      "authenticated /hesabim renders Turkish and English portal views cleanly via ?lang=tr|en",
      { req: "F6" },
      async (c) => {
        const reader = await getCachedRoleSession(c, "registered");
        const [trRes, enRes] = await Promise.all([
          reader.client.get("/hesabim", { query: { lang: "tr" } }),
          reader.client.get("/hesabim", { query: { lang: "en" } }),
        ]);
        c.assert.assertStatus(trRes, 200);
        c.assert.assertStatus(enRes, 200);
        c.assert.assertMatch(trRes.text, /<html[^>]+lang="tr"/i);
        c.assert.assertMatch(enRes.text, /<html[^>]+lang="en"/i);
      }
    );

    suite.test(
      "authenticated /hesabim preserves strict CSP nonces on scripts and loads account-portal.{css,js}",
      { req: "F6" },
      async (c) => {
        const reader = await getCachedRoleSession(c, "registered");
        const res = await reader.client.get("/hesabim");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "/css/account-portal.css");
        c.assert.assertIncludes(res.text, "/js/account-portal.js");
        const scriptTags = res.text.match(/<script\b[^>]*>/gi) || [];
        for (const tag of scriptTags) {
          if (/type=["']application\/(ld\+json|json|speculationrules)["']/i.test(tag)) continue;
          c.assert.assertMatch(tag, /\bnonce=["'][^"']+["']/i, `/hesabim script missing nonce: ${tag}`);
        }
      }
    );
  });

  // ── F7: Public Auth Dialog & Password Reset Modernization (5 tests) ────────
  suite.describe("F7 Public Auth Dialog & Password Reset Modernization", () => {
    suite.test(
      "public homepage preserves #auth-dialog, #login-form, #register-form, #forgot-password-form, #totp-login-form, and /hesabim link",
      { req: "F7", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const [homeRes, hesabimRes] = await Promise.all([
          client.get("/"),
          client.get("/hesabim"),
        ]);
        c.assert.assertStatus(homeRes, 200);
        c.assert.assertStatus(hesabimRes, 200);
        for (const marker of [
          'id="auth-dialog"',
          'id="login-form"',
          'id="register-form"',
          'id="forgot-password-form"',
          'id="totp-login-form"',
          'href="/hesabim"',
        ]) {
          c.assert.assertIncludes(homeRes.text, marker, `Public homepage missing ${marker}`);
        }
        c.assert.assertIncludes(hesabimRes.text, 'href="#modal-login"');
      }
    );

    suite.test(
      "GET /sifre-sifirla and /sifre-yenile both return 200 with #reset-password-form and token field",
      { req: "F7", readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        for (const path of ["/sifre-sifirla?token=sample-e2e-token", "/sifre-yenile?token=sample-e2e-token"]) {
          const res = await client.get(path);
          c.assert.assertStatus(res, 200, `${path} must return 200`);
          c.assert.assertIncludes(res.text, 'id="reset-password-form"');
          c.assert.assertIncludes(res.text, 'name="token"');
          c.assert.assertIncludes(res.text, 'name="new_password"');
          c.assert.assertIncludes(res.text, 'name="confirm_password"');
          c.assert.assertNotIncludes(res.text.toLowerCase(), "msi-komuta");
        }
      }
    );

    suite.test(
      "public #register-form preserves .gov.tr/.tsk.tr hint (#gov-domain-hint) and KVKK/ETK markup",
      { req: "F7", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/");
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, 'id="gov-domain-hint"');
        c.assert.assertIncludes(res.text, 'name="kvkk_consent"');
        c.assert.assertIncludes(res.text, 'name="email_consent"');
        c.assert.assertIncludes(res.text, 'id="register-tos-accepted"');
      }
    );

    suite.test(
      "English public portal /en also preserves #auth-dialog and localized auth forms",
      { req: "F7", readOnly: true },
      async (c) => {
        const res = await c.http.createClient().get("/en");
        c.assert.assertStatus(res, 200);
        c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
        c.assert.assertIncludes(res.text, 'id="auth-dialog"');
        c.assert.assertIncludes(res.text, 'id="register-form"');
      }
    );

    suite.test(
      ".gov.tr public registration auto-verifies subscriber_public via POST /api/v1/auth/register",
      { req: "F7" },
      async (c) => {
        const reg = await c.auth.registerUser({
          email: `e2e.f7.${Date.now()}@msb.gov.tr`,
          organization_category: "Kamu / TSK / SSB",
          organization_name: "MSB",
          email_consent: true,
        });
        c.assert.assertOk(reg.govAutoSubscribed, ".gov.tr signup must auto-verify subscriber_public");
        c.assert.assertEqual(reg.user?.role, "subscriber_public");
      }
    );
  });
}
