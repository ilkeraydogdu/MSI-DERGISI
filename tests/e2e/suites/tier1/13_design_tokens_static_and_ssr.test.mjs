import crypto from "node:crypto";
import { DatabaseSync } from "node:sqlite";
import {
  assertNoExternalCdnsOrMockdata,
  ROLE_A_TOKENS,
  ROLE_B_TOKENS,
} from "../../fixtures/design_tokens_contract.mjs";

const REQUIRED_DESIGN_TOKEN_CSS_VARS = [
  "--ui-font-sans",
  "--ui-font-serif",
  "--ui-font-mono",
  "--ui-space-1",
  "--ui-space-2",
  "--ui-space-3",
  "--ui-space-4",
  "--ui-space-5",
  "--ui-space-6",
  "--ui-space-8",
  "--ui-space-10",
  "--ui-radius-xs",
  "--ui-radius-sm",
  "--ui-radius-md",
  "--ui-radius-lg",
  "--ui-radius-xl",
  "--ui-radius-pill",
  "--ui-z-sticky",
  "--ui-z-header",
  "--ui-z-drawer",
  "--ui-z-modal",
  "--ui-z-confirm",
  "--ui-z-toast",
  "--ui-success",
  "--ui-warning",
  "--ui-danger",
  "--ui-info",
  "--ui-brand-bg",
  "--ui-brand-surface",
  "--ui-brand-accent",
  "--ui-ops-bg",
  "--ui-ops-surface",
  "--ui-ops-accent",
];

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Tier 1 UI Unification: Design Tokens, SSR & Asset Integrity", () => {
    suite.test(
      "T1-U1: GET /css/design-tokens.css returns 200 and defines all --ui-* typography, scale, Role A & Role B tokens",
      { req: ["F1", "R1", "R3"], readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const res = await client.get("/css/design-tokens.css");
        assert.assertStatus(res, 200, "/css/design-tokens.css must return 200");
        assert.assertIncludes(
          (res.headers.get("content-type") || "").toLowerCase(),
          "text/css",
          "/css/design-tokens.css must serve text/css"
        );
        assertNoExternalCdnsOrMockdata(res.text, "/css/design-tokens.css");

        const missing = REQUIRED_DESIGN_TOKEN_CSS_VARS.filter((v) => !res.text.includes(v));
        assert.assertEqual(
          missing.length,
          0,
          `/css/design-tokens.css is missing required tokens: ${missing.join(", ")}`
        );

        const cssLower = res.text.toLowerCase();
        assert.assertIncludes(cssLower, "ibm plex sans", "/css/design-tokens.css must specify IBM Plex Sans");
        assert.assertIncludes(cssLower, ROLE_A_TOKENS.light["--ui-brand-bg"], "Must define Role A light bg");
        assert.assertIncludes(cssLower, ROLE_A_TOKENS.light["--ui-brand-accent"], "Must define Role A light accent");
        assert.assertIncludes(cssLower, ROLE_B_TOKENS.light["--ui-ops-bg"], "Must define Role B light bg");
        assert.assertIncludes(cssLower, ROLE_B_TOKENS.light["--ui-ops-accent"], "Must define Role B light accent");
      }
    );

    suite.test(
      "T1-U2: Surface 1 public routes, /hesabim (guest) & /{adminPath} (guest) return 200, link /css/design-tokens.css & pass CDN/mockdata scan",
      { req: ["F1", "F2", "F6", "F9", "R3"], readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const routes = [
          "/",
          "/?mode=news",
          "/en",
          "/abonelik-ve-satis",
          "/abonelik-ve-satis?lang=en",
          "/e-dergi",
          "/e-dergi?lang=en",
          "/arama",
          "/en/search",
          "/sifre-sifirla",
          "/sifre-sifirla?lang=en",
          "/hesabim",
          `/${c.adminPath}`,
        ];

        for (const route of routes) {
          const res = await client.get(route);
          assert.assertStatus(res, 200, `GET ${route} must return HTTP 200`);
          assert.assertIncludes(
            res.text,
            "/css/design-tokens.css",
            `GET ${route} SSR HTML must link /css/design-tokens.css`
          );
          assertNoExternalCdnsOrMockdata(res.text, `SSR ${route}`);
        }
      }
    );

    suite.test(
      "T1-U3: Transactional routes /hesap-dogrula?lang=en & /abonelik-iptal?lang=en localize EN copy without hardcoded TR strings",
      { req: ["F4", "R2"], readOnly: true },
      async (c) => {
        const client = c.http.createClient();

        const verifyEn = await client.get("/hesap-dogrula?lang=en");
        assert.assertStatus(verifyEn, [200, 400], "/hesap-dogrula?lang=en must return 200 or 400");
        assert.assertMatch(
          verifyEn.text,
          /<html[^>]+lang="en"/i,
          "/hesap-dogrula?lang=en must render <html lang='en'>"
        );
        assert.assertIncludes(verifyEn.text, "/css/design-tokens.css", "/hesap-dogrula must link design-tokens.css");
        assertNoExternalCdnsOrMockdata(verifyEn.text, "/hesap-dogrula?lang=en");
        for (const trLeak of [
          "E-POSTA DOĞRULAMA",
          "Doğrulama Başarısız",
          "Doğrulama bağlantısı eksik",
          "Hesabıma Dön",
        ]) {
          assert.assertNotIncludes(
            verifyEn.text,
            trLeak,
            `/hesap-dogrula?lang=en leaked hardcoded Turkish string "${trLeak}"`
          );
        }

        const unsubEn = await client.get("/abonelik-iptal?lang=en");
        assert.assertStatus(unsubEn, [200, 400], "/abonelik-iptal?lang=en must return 200 or 400");
        assert.assertMatch(
          unsubEn.text,
          /<html[^>]+lang="en"/i,
          "/abonelik-iptal?lang=en must render <html lang='en'>"
        );
        assert.assertIncludes(unsubEn.text, "/css/design-tokens.css", "/abonelik-iptal must link design-tokens.css");
        assertNoExternalCdnsOrMockdata(unsubEn.text, "/abonelik-iptal?lang=en");
        for (const trLeak of [
          "ABONELİK YÖNETİMİ",
          "Geçersiz Bağlantı",
          "Bu abonelik iptal bağlantısı",
        ]) {
          assert.assertNotIncludes(
            unsubEn.text,
            trLeak,
            `/abonelik-iptal?lang=en leaked hardcoded Turkish string "${trLeak}"`
          );
        }
      }
    );

    suite.test(
      "T1-U4: Global header [data-cart-toggle] & [data-cart-count] exist on / and /e-dergi, and msi-ui.js has zero dead /en/* storefront links",
      { req: ["F3", "F4", "R2"], readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        for (const route of ["/", "/e-dergi", "/abonelik-ve-satis"]) {
          const res = await client.get(route);
          assert.assertStatus(res, 200);
          assert.assertIncludes(
            res.text,
            "data-cart-toggle",
            `${route} global header must include [data-cart-toggle]`
          );
          assert.assertIncludes(
            res.text,
            "data-cart-count",
            `${route} global header must include [data-cart-count]`
          );
        }

        const jsRes = await client.get("/js/msi-ui.js");
        assert.assertStatus(jsRes, 200, "/js/msi-ui.js must return 200");
        assertNoExternalCdnsOrMockdata(jsRes.text, "/js/msi-ui.js");
        assert.assertNotIncludes(
          jsRes.text,
          "/en/abonelik-ve-satis",
          "msi-ui.js must not link to unmounted /en/abonelik-ve-satis"
        );
        assert.assertNotIncludes(
          jsRes.text,
          "/en/e-magazine",
          "msi-ui.js must not reference unmounted /en/e-magazine"
        );
      }
    );

    suite.test(
      "T1-U5: All 5-surface CSS & JS static assets return 200 and pass zero-CDN / zero-mockdata scan",
      { req: ["F1", "R3"], readOnly: true },
      async (c) => {
        const client = c.http.createClient();
        const assets = [
          "/css/design-tokens.css",
          "/css/msi-design-system.css",
          "/css/account-portal.css",
          "/css/admin/tokens.css",
          "/css/admin/layout.css",
          "/css/admin/components.css",
          "/css/admin/login.css",
          "/css/admin/editorial-studio.css",
          "/css/msi-design-editor.css",
          "/js/msi-ui.js",
          "/js/account-portal.js",
          "/js/admin/core.js",
          "/js/msi-design-editor.js",
        ];

        for (const assetPath of assets) {
          const res = await client.get(assetPath);
          assert.assertStatus(res, 200, `Static asset ${assetPath} must return 200`);
          assertNoExternalCdnsOrMockdata(res.text, assetPath);
        }
      }
    );

    suite.test(
      "T1-U6: Authenticated SSR shells (/hesabim, /{adminPath}, /{adminPath}/editor, /{adminPath}/tasarim) link /css/design-tokens.css & pass CDN scan",
      { req: ["F1", "F6", "F9", "F12", "F13", "R3"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const routes = [
          "/hesabim",
          `/${c.adminPath}`,
          `/${c.adminPath}/editor`,
          `/${c.adminPath}/tasarim`,
        ];

        for (const route of routes) {
          const res = await sa.client.get(route);
          assert.assertStatus(res, 200, `Authenticated GET ${route} must return 200`);
          assert.assertIncludes(
            res.text,
            "/css/design-tokens.css",
            `Authenticated ${route} must link /css/design-tokens.css`
          );
          assertNoExternalCdnsOrMockdata(res.text, `Authenticated SSR ${route}`);
        }
      }
    );

    suite.test(
      "T1-U7: Transactional /abonelik-iptal & POST /api/v1/email-preferences localize frequency labels (no raw 'weekly') and respect ?lang=en",
      { req: ["F4", "R2"] },
      async (c) => {
        if (c.isExternalTarget || !c.dbPath) {
          // On external targets we cannot seed an unsubscribe token directly in SQLite.
          return;
        }
        const reader = await c.auth.createUserWithRole("registered");
        const rawToken = `e2e-unsub-${crypto.randomBytes(12).toString("hex")}`;
        const tokenHash = crypto.createHash("sha256").update(rawToken).digest("hex");

        const db = new DatabaseSync(c.dbPath);
        try {
          db.prepare("INSERT INTO unsubscribe_tokens (user_id, token) VALUES (?, ?)").run(
            reader.id,
            tokenHash
          );
        } finally {
          db.close();
        }

        const client = c.http.createClient();
        const pageEn = await client.get("/abonelik-iptal", {
          query: { token: rawToken, lang: "en" },
        });
        assert.assertStatus(pageEn, 200, "Valid unsubscribe token GET ?lang=en must return 200");
        assert.assertMatch(pageEn.text, /<html[^>]+lang="en"/i);
        for (const trLeak of [
          "E-Posta Aboneliğinden Çıkmak Üzeresiniz",
          "Tercihlerinizi Yönetin",
          "Tercihleri Kaydet",
        ]) {
          assert.assertNotIncludes(
            pageEn.text,
            trLeak,
            `Valid /abonelik-iptal?lang=en leaked Turkish string "${trLeak}"`
          );
        }

        const prefsTr = await client.post("/api/v1/email-preferences", {
          headers: { Accept: "text/html" },
          form: {
            token: rawToken,
            newsletter_frequency: "weekly",
            categories: "Havacılık",
          },
        });
        assert.assertStatus(prefsTr, 200, "POST /api/v1/email-preferences HTML response must return 200");
        assert.assertNotIncludes(
          prefsTr.text,
          "<strong>weekly</strong>",
          "Turkish /api/v1/email-preferences confirmation must localize 'weekly' rather than interpolating raw enum"
        );
        assert.assertIncludes(
          prefsTr.text,
          "Haftalık",
          "Turkish /api/v1/email-preferences confirmation must display localized 'Haftalık'"
        );
      }
    );
  });
}
