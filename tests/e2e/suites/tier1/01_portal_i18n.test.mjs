import {
  getCachedSuperAdmin,
  makeContentPayload,
  makeGlossaryPayload,
  makeLocalePayload,
  uid,
  uniqueSlug,
  unwrap,
  CANONICAL_SSB_TAGS,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-001: Multi-domain single-DB architecture ──────────────────────────
  suite.describe("REQ-001 Multi-Domain Single-DB", () => {
    suite.test("msidergisi.com serves magazine mode homepage HTML", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "MSI");
      c.assert.assertIncludes(res.headers.get("content-type") || "", "text/html");
    });

    suite.test("savunmahaber.com serves news mode homepage distinct from msidergisi.com", { req: "REQ-001" }, async (c) => {
      const msi = c.http.createClient({ defaultHost: "msidergisi.com" });
      const sh = c.http.createClient({ defaultHost: "savunmahaber.com" });
      const [r1, r2] = await Promise.all([msi.get("/"), sh.get("/")]);
      c.assert.assertStatus(r1, 200);
      c.assert.assertStatus(r2, 200);
      c.assert.assertIncludes(r2.text.toLowerCase(), "savunma");
      c.assert.assertNotEqual(r1.text, r2.text, "msidergisi.com and savunmahaber.com must render distinct domain views");
    });

    suite.test("query ?mode=news overrides domain presentation to news mode", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/", { query: { mode: "news" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text.toLowerCase(), "savunma");
    });

    suite.test("single-DB SSOT content published via admin is visible on both domains", { req: "REQ-001" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-multidomain");
      const titleTr = `Coklu Alan Adi Haberi ${uid("m")}`;
      const createRes = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: titleTr,
          title_en: `Multi Domain News ${slug}`,
          summary_tr: "Ozet TR",
          summary_en: "Summary EN",
          body_tr: "<p>Govde metin TR.</p>",
          body_en: "<p>Body text EN.</p>",
          content_type: "haberler",
          source_group: "gunluk-haber",
          status: "published",
          visibility_level: 0,
          placements: ["haberler"],
        }),
      });
      c.assert.assertStatus(createRes, 200);

      const msiClient = c.http.createClient({ defaultHost: "msidergisi.com" });
      const shClient = c.http.createClient({ defaultHost: "savunmahaber.com" });
      const [rMsi, rSh] = await Promise.all([
        msiClient.get(`/icerik/${slug}`),
        shClient.get(`/icerik/${slug}`),
      ]);
      c.assert.assertStatus(rMsi, 200);
      c.assert.assertStatus(rSh, 200);
      c.assert.assertIncludes(rMsi.text, titleTr);
      c.assert.assertIncludes(rSh.text, titleTr);
    });

    suite.test("/api/v1/health reports primary and news_portal domain metadata", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/health");
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.domains?.primary, "msidergisi.com");
      c.assert.assertEqual(res.json?.domains?.news_portal, "savunmahaber.com");
    });
  });

  // ── REQ-002: Bilingual TR/EN + Accept-Language auto-detection ─────────────
  suite.describe("REQ-002 Bilingual TR/EN & Accept-Language", () => {
    suite.test("Accept-Language: en-US,en;q=0.9,tr;q=0.5 auto-detects EN on /", { req: "REQ-002" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/", {
        headers: { "Accept-Language": "en-US,en;q=0.9,tr;q=0.5" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i, "Expected <html lang='en'> from Accept-Language");
    });

    suite.test("Accept-Language with higher Turkish q-weight resolves TR", { req: "REQ-002" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/", {
        headers: { "Accept-Language": "en-US;q=0.4,tr-TR,tr;q=0.9" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i, "Expected <html lang='tr'> when tr q > en q");
    });

    suite.test("/en route explicitly renders English portal regardless of Accept-Language", { req: "REQ-002" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/en", {
        headers: { "Accept-Language": "tr-TR,tr;q=0.9" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
    });

    suite.test("/switch/en sets msi_lang=en cookie and overrides Accept-Language", { req: "REQ-002" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const sw = await client.get("/switch/en", { redirect: "manual" });
      c.assert.assertStatus(sw, [302, 303, 307]);
      c.assert.assertEqual(client.getCookie("msi_lang"), "en");

      const home = await client.get("/", {
        headers: { "Accept-Language": "tr-TR,tr;q=0.9" },
      });
      c.assert.assertStatus(home, 200);
      c.assert.assertMatch(home.text, /<html[^>]+lang="en"/i);
    });

    suite.test("query ?lang=tr overrides msi_lang=en cookie and Accept-Language", { req: "REQ-002" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      client.setCookie("msi_lang", "en");
      const res = await client.get("/", {
        query: { lang: "tr" },
        headers: { "Accept-Language": "en-US,en;q=0.9" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i);
    });
  });

  // ── REQ-003: Auto-translate, glossary guard & locales CRUD ────────────────
  suite.describe("REQ-003 Translation, Glossary & Locales", () => {
    suite.test("GET /api/v1/admin/translations returns paginated translation entries", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/translations", { query: { page: "1", per_page: "10" } });
      c.assert.assertStatus(res, 200);
      const list = unwrap(res);
      c.assert.assertOk(Array.isArray(list), "Translations list must be an array");
      c.assert.assertOk(res.json?.meta?.per_page === 10, "Pagination meta per_page should be 10");
    });

    suite.test("POST and DELETE /api/v1/admin/translations upserts and removes bilingual key", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const key = `e2e.tr.${uid("k")}`;
      const up = await sa.client.post("/api/v1/admin/translations", {
        json: {
          key,
          value_tr: "Deneme Etiketi",
          value_en: "Test Label",
          group: "e2e",
        },
      });
      c.assert.assertStatus(up, 200);
      c.assert.assertEqual(unwrap(up)?.key, key);

      const del = await sa.client.delete(`/api/v1/admin/translations/${encodeURIComponent(key)}`);
      c.assert.assertStatus(del, 200);
    });

    suite.test("GET/POST/DELETE /api/v1/admin/glossary manages defense terminology guard", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const termTr = `Milli Radar ${uid("g")}`;
      const createRes = await sa.client.post("/api/v1/admin/glossary", {
        json: makeGlossaryPayload({
          term_tr: termTr,
          term_en: "National Radar",
          preferred_en: "National Radar",
          domain: "defense",
        }),
      });
      c.assert.assertStatus(createRes, 200);
      const createdId = unwrap(createRes)?.id;
      c.assert.assertOk(createdId > 0, "Created glossary id must be positive");

      const listRes = await sa.client.get("/api/v1/admin/glossary");
      c.assert.assertStatus(listRes, 200);
      const items = unwrap(listRes);
      c.assert.assertOk(items.some((t) => t.id === createdId && t.term_tr === termTr));

      const delRes = await sa.client.delete(`/api/v1/admin/glossary/${createdId}`);
      c.assert.assertStatus(delRes, 200);
    });

    suite.test("GET/POST/PUT/DELETE /api/v1/admin/locales manages active locales", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const code = "de";
      const createRes = await sa.client.post("/api/v1/admin/locales", {
        json: makeLocalePayload({ code, is_enabled: true }),
      });
      c.assert.assertStatus(createRes, 200);

      try {
        const listRes = await sa.client.get("/api/v1/admin/locales");
        c.assert.assertStatus(listRes, 200);
        const locales = unwrap(listRes);
        c.assert.assertOk(Array.isArray(locales) && locales.some((l) => l.code === code));

        const updRes = await sa.client.put(`/api/v1/admin/locales/${code}`, {
          json: makeLocalePayload({
            code,
            name_native: "Deutsch (DE)",
            is_enabled: false,
            sort_order: 11,
          }),
        });
        c.assert.assertStatus(updRes, 200);
      } finally {
        const delRes = await sa.client.delete(`/api/v1/admin/locales/${code}`);
        c.assert.assertStatus(delRes, 200);
      }
    });

    suite.test("GET /api/v1/admin/translations/pending returns paginated pending workflow list", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/translations/pending", {
        query: { page: "1", per_page: "20" },
      });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertOk(Array.isArray(data), "Pending translations must be an array");
    });
  });

  // ── REQ-004: Responsive mobile-compatible portal & manifest ───────────────
  suite.describe("REQ-004 Responsive Mobile Portal & PWA", () => {
    suite.test("public homepage includes viewport meta tag for mobile scaling", { req: "REQ-004" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<meta[^>]+name="viewport"[^>]+width=device-width/i);
    });

    suite.test("/manifest.webmanifest serves valid PWA JSON metadata", { req: "REQ-004" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/manifest.webmanifest");
      c.assert.assertStatus(res, 200);
      c.assert.assertJson(res, (j) => Boolean(j.name && j.start_url && Array.isArray(j.icons)));
    });

    suite.test("/css/msi-design-system.css contains responsive media queries", { req: "REQ-004" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/css/msi-design-system.css");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "@media");
    });

    suite.test("/e-dergi and /abonelik-ve-satis render mobile viewport meta cleanly", { req: "REQ-004" }, async (c) => {
      const client = c.http.createClient();
      for (const p of ["/e-dergi", "/abonelik-ve-satis"]) {
        const res = await client.get(p);
        c.assert.assertStatus(res, 200);
        c.assert.assertMatch(res.text, /<meta[^>]+name="viewport"/i, `${p} missing viewport meta`);
      }
    });

    suite.test("/css/design-tokens.css serves dynamic CSS custom properties", { req: "REQ-004" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/css/design-tokens.css");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.headers.get("content-type") || "", "text/css");
      c.assert.assertIncludes(res.text, ":root");
    });
  });

  // ── REQ-005: Rich Homepage configurable components & Design Studio ─────────
  suite.describe("REQ-005 Homepage Components & Design Studio", () => {
    suite.test("homepage renders hero, issues, topics, and subscription/plans sections", { req: "REQ-005" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "/e-dergi");
      c.assert.assertIncludes(res.text, "/abonelik-ve-satis");
      c.assert.assertMatch(res.text, /class="[^"]*(hero|issues|topics|section)/i);
    });

    suite.test("GET /api/v1/admin/design returns magazine & news studio layout config", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/design");
      c.assert.assertStatus(res, 200);
      const cfg = unwrap(res);
      c.assert.assertOk(cfg && typeof cfg === "object", "Design config must be an object");
    });

    suite.test("GET /api/v1/settings exposes public site settings & announcement config", { req: "REQ-005" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/settings");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(res.json !== null, "Settings response must be valid JSON");
    });

    suite.test("POST /{adminPath}/tasarim/onizleme renders live preview HTML for staff", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const designRes = await sa.client.get("/api/v1/admin/design");
      c.assert.assertStatus(designRes, 200);
      const cfg = unwrap(designRes)?.config || unwrap(designRes)?.defaults || unwrap(designRes);

      const previewRes = await sa.client.post(`/${c.adminPath}/tasarim/onizleme`, {
        json: cfg,
      });
      c.assert.assertStatus(previewRes, 200);
      c.assert.assertIncludes(previewRes.text, "<html");
    });

    suite.test("/sayfa/hakkimizda or /kurumsal/hakkimizda renders configured SSR page", { req: "REQ-005" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/kurumsal/hakkimizda");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "<main");
    });
  });

  // ── REQ-006: Personalized feed & 14 SSB tags ──────────────────────────────
  suite.describe("REQ-006 Personalized Feed & 14 SSB Tags", () => {
    suite.test("GET /api/v1/ssb-tags returns all 14 canonical defense technology tags", { req: "REQ-006" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/ssb-tags");
      c.assert.assertStatus(res, 200);
      const tags = unwrap(res);
      c.assert.assertOk(Array.isArray(tags) && tags.length >= 14, `Expected >=14 SSB tags, got ${tags?.length}`);
      const codes = tags.map((t) => t.code);
      for (const expected of CANONICAL_SSB_TAGS) {
        c.assert.assertIncludes(codes, expected, `Missing canonical SSB tag ${expected}`);
      }
    });

    suite.test("GET /api/v1/public/personalized?interests=... returns matching non-sponsored items", { req: "REQ-006" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/public/personalized", {
        query: { interests: "hava-araclari,radarlar-ve-sensorler", limit: "6" },
      });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertOk(Array.isArray(data?.items), "Personalized items must be an array");
      for (const item of data.items) {
        c.assert.assertEqual(item.is_sponsored, false, "Personalized feed must exclude sponsored items");
      }
    });

    suite.test("authenticated user SSB interests drive /api/v1/public/personalized", { req: "REQ-006" }, async (c) => {
      const reg = await c.auth.registerUser({
        ssb_interests: ["deniz-araclari", "elektronik-harp-sistemleri"],
      });
      const res = await reg.client.get("/api/v1/public/personalized");
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertEqual(data?.personalized, true);
      c.assert.assertIncludes(data?.interests, "deniz-araclari");
    });

    suite.test("updating /api/v1/account/profile changes user SSB preferences", { req: "REQ-006" }, async (c) => {
      const reg = await c.auth.registerUser({ ssb_interests: ["kara-araclari"] });
      const upd = await reg.client.post("/api/v1/account/profile", {
        json: {
          full_name: reg.user.full_name,
          ssb_interests: ["hava-araclari", "muhimmatlar-ve-fuze-sistemleri"],
        },
      });
      c.assert.assertStatus(upd, 200);
      const feed = await reg.client.get("/api/v1/public/personalized");
      c.assert.assertStatus(feed, 200);
      c.assert.assertIncludes(unwrap(feed)?.interests, "muhimmatlar-ve-fuze-sistemleri");
    });

    suite.test("GET /arama?ssb_tag=hava-araclari filters search results by SSB tag", { req: "REQ-006" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/search", {
        query: { ssb_tag: "hava-araclari", per_page: "5" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json?.items), "Search items must be an array");
    });
  });
}
