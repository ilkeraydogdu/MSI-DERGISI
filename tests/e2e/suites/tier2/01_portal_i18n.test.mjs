import {
  getCachedSuperAdmin,
  makeGlossaryPayload,
  makeLocalePayload,
  uid,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-001 Boundary / Negative: Multi-Domain ─────────────────────────────
  suite.describe("REQ-001 Multi-Domain Boundaries", () => {
    suite.test("unknown Host header safely falls back without 500 error", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/", { host: "unknown-domain.example" });
      c.assert.assertStatus(res, 200);
    });

    suite.test("invalid ?mode=xyz query parameter falls back safely to domain default", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "msidergisi.com" });
      const res = await client.get("/", { query: { mode: "invalid_mode_99" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "MSI");
    });

    suite.test("savunmahaber.com /en renders English news domain mode cleanly", { req: "REQ-001" }, async (c) => {
      const client = c.http.createClient({ defaultHost: "savunmahaber.com" });
      const res = await client.get("/en");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="en"/i);
    });

    suite.test("GET /api/v1/contents/:slug returns 404 for unknown slug across both domains", { req: "REQ-001" }, async (c) => {
      for (const host of ["msidergisi.com", "savunmahaber.com"]) {
        const res = await c.http.createClient({ defaultHost: host }).get("/api/v1/contents/nonexistent-slug-e2e");
        c.assert.assertStatus(res, 404);
      }
    });

    suite.test("savunmahaber.com /rss.xml renders distinct news domain syndication title", { req: "REQ-001" }, async (c) => {
      const msi = await c.http.createClient({ defaultHost: "msidergisi.com" }).get("/rss.xml");
      const sh = await c.http.createClient({ defaultHost: "savunmahaber.com" }).get("/rss.xml");
      c.assert.assertStatus(msi, 200);
      c.assert.assertStatus(sh, 200);
      c.assert.assertIncludes(sh.text, "<rss");
      c.assert.assertIncludes(sh.text.toLowerCase(), "savunma");
    });
  });

  // ── REQ-002 Boundary / Negative: Accept-Language & Locale Switching ───────
  suite.describe("REQ-002 Bilingual & Accept-Language Boundaries", () => {
    suite.test("unsupported Accept-Language (fr-FR,de;q=0.8) falls back to default TR", { req: "REQ-002" }, async (c) => {
      const res = await c.http.createClient().get("/", {
        headers: { "Accept-Language": "fr-FR,fr;q=0.9,de;q=0.8" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i);
    });

    suite.test("Accept-Language with en;q=0 explicitly excludes English", { req: "REQ-002" }, async (c) => {
      const res = await c.http.createClient().get("/", {
        headers: { "Accept-Language": "en;q=0,tr;q=0.5" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i);
    });

    suite.test("GET /switch/invalid rejects unsupported locale with 400", { req: "REQ-002" }, async (c) => {
      const res = await c.http.createClient().get("/switch/xx", { redirect: "manual" });
      c.assert.assertStatus(res, 400);
    });

    suite.test("invalid ?lang=zz falls back cleanly without breaking SSR", { req: "REQ-002" }, async (c) => {
      const res = await c.http.createClient().get("/", { query: { lang: "zz" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i);
    });

    suite.test("Accept-Language wildcard * resolves default TR deterministically", { req: "REQ-002" }, async (c) => {
      const res = await c.http.createClient().get("/", {
        headers: { "Accept-Language": "*" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /<html[^>]+lang="tr"/i);
    });
  });

  // ── REQ-003 Boundary / Negative: i18n, Glossary & Locales ─────────────────
  suite.describe("REQ-003 Translation, Glossary & Locales Boundaries", () => {
    suite.test("POST /api/v1/admin/translations/auto-translate returns 400 when MT provider unconfigured", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/translations/auto-translate", {
        json: { text: "Savunma sanayii raporu", source_lang: "tr", target_lang: "en" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("DELETE /api/v1/admin/glossary/999999 returns 404", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.delete("/api/v1/admin/glossary/999999");
      c.assert.assertStatus(res, 404);
    });

    suite.test("POST /api/v1/admin/glossary rejects empty term_tr/term_en with 400", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/glossary", {
        json: makeGlossaryPayload({ term_tr: "   ", term_en: "", preferred_en: "" }),
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/admin/locales rejects empty code with 400", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/locales", {
        json: makeLocalePayload({ code: "   ", name_native: "Invalid", name_en: "Invalid" }),
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("content_editor is denied locales:manage and glossary:manage with 403", { req: "REQ-003" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const r1 = await ed.client.get("/api/v1/admin/locales");
      const r2 = await ed.client.get("/api/v1/admin/glossary");
      c.assert.assertStatus(r1, 403);
      c.assert.assertStatus(r2, 403);
    });
  });

  // ── REQ-004 Boundary / Negative: Responsive Assets & Static Security ──────
  suite.describe("REQ-004 Responsive & Static Asset Boundaries", () => {
    suite.test("GET /css/nonexistent-file.css returns 404", { req: "REQ-004" }, async (c) => {
      const res = await c.http.createClient().get(`/css/missing-${uid("c")}.css`);
      c.assert.assertStatus(res, 404);
    });

    suite.test("static path traversal /css/../.env is blocked with 403", { req: "REQ-004" }, async (c) => {
      const res = await c.http.createClient().get("/css/../../.env");
      c.assert.assertStatus(res, 403);
    });

    suite.test("POST /manifest.webmanifest is rejected with 405", { req: "REQ-004" }, async (c) => {
      const res = await c.http.createClient().post("/manifest.webmanifest", { json: {} });
      c.assert.assertStatus(res, 405);
    });

    suite.test("404 fallback page renders clean mobile viewport HTML without 500", { req: "REQ-004" }, async (c) => {
      const res = await c.http.createClient().get(`/nonexistent-page-${uid("p")}`);
      c.assert.assertStatus(res, 404);
      c.assert.assertMatch(res.text, /<meta[^>]+name="viewport"/i);
    });

    suite.test("GET /css/design-tokens.css returns non-empty CSS without HTML tags", { req: "REQ-004" }, async (c) => {
      const res = await c.http.createClient().get("/css/design-tokens.css");
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, "<script");
    });
  });

  // ── REQ-005 Boundary / Negative: Design Studio & Settings ─────────────────
  suite.describe("REQ-005 Design Studio & Settings Boundaries", () => {
    suite.test("POST /api/v1/admin/design rejects malformed/invalid DesignConfig", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/design", {
        json: { invalid_key_only: true },
      });
      c.assert.assertStatus(res, [400, 422]);
    });

    suite.test("POST /api/v1/admin/settings rejects invalid design_config JSON with 400", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/settings", {
        json: {
          key: "design_config",
          value_tr: "{not valid json",
          setting_group: "design",
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("GET /sayfa/nonexistent-page-slug returns 404", { req: "REQ-005" }, async (c) => {
      const res = await c.http.createClient().get(`/sayfa/missing-${uid("pg")}`);
      c.assert.assertStatus(res, 404);
    });

    suite.test("content_editor is denied GET/POST /api/v1/admin/design with 403", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const designRes = await sa.client.get("/api/v1/admin/design");
      const cfg = unwrap(designRes)?.config || unwrap(designRes)?.defaults || {};
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const g = await ed.client.get("/api/v1/admin/design");
      const p = await ed.client.post("/api/v1/admin/design", { json: cfg });
      c.assert.assertStatus(g, 403);
      c.assert.assertStatus(p, 403);
    });

    suite.test("ads_manager is denied GET /{adminPath}/tasarim with 403", { req: "REQ-005" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const res = await ads.client.get(`/${c.adminPath}/tasarim`, { redirect: "manual" });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-006 Boundary / Negative: Personalized Feed & SSB Tags ─────────────
  suite.describe("REQ-006 Personalized Feed & SSB Tag Boundaries", () => {
    suite.test("GET /api/v1/public/personalized clamps out-of-range limit=0 and limit=999", { req: "REQ-006" }, async (c) => {
      const client = c.http.createClient();
      const rLow = await client.get("/api/v1/public/personalized", { query: { limit: "0" } });
      const rHigh = await client.get("/api/v1/public/personalized", { query: { limit: "999" } });
      c.assert.assertStatus(rLow, 200);
      c.assert.assertStatus(rHigh, 200);
      c.assert.assertOk((unwrap(rLow)?.items || []).length >= 1);
      c.assert.assertOk((unwrap(rHigh)?.items || []).length <= 24);
    });

    suite.test("GET /api/v1/public/personalized with unknown interest tag does not error", { req: "REQ-006" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/public/personalized", {
        query: { interests: "nonexistent-ssb-tag-xyz" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)?.items));
    });

    suite.test("authenticated user ignores arbitrary ?interests= query override", { req: "REQ-006" }, async (c) => {
      const reg = await c.auth.registerUser({ ssb_interests: ["deniz-araclari"] });
      const res = await reg.client.get("/api/v1/public/personalized", {
        query: { interests: "hizmetler-kurumsal-catering" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(unwrap(res)?.interests || [], "deniz-araclari");
    });

    suite.test("GET /api/v1/public/personalized never exposes draft or sponsored items", { req: "REQ-006" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const draftSlug = `t2-draft-${uid("d").toLowerCase().replace(/[^a-z0-9-]/g, "-")}`;
      await sa.client.post("/api/v1/admin/contents", {
        json: {
          id: 0,
          slug: draftSlug,
          title_tr: "Gizli Taslak Feed",
          title_en: "Secret Draft",
          summary_tr: "",
          summary_en: "",
          body_tr: "<p>Draft</p>",
          body_en: "",
          content_type: "haberler",
          source_group: "gunluk-haber",
          cover_image: "",
          author_name: "",
          author_title: "",
          issue_number: null,
          published_at: "",
          read_time_min: 3,
          visibility_level: 0,
          single_price_try: 0,
          is_sponsored: false,
          sponsor_brand: null,
          sponsor_legal_label: null,
          is_featured: false,
          video_url: null,
          view_count: 0,
          true_read_count: 0,
          placements: ["haberler"],
          ssb_tags: ["hava-araclari"],
          status: "draft",
        },
      });
      const res = await c.http.createClient().get("/api/v1/public/personalized", {
        query: { interests: "hava-araclari", limit: "24" },
      });
      c.assert.assertStatus(res, 200);
      const slugs = (unwrap(res)?.items || []).map((i) => i.slug);
      c.assert.assertNotIncludes(slugs, draftSlug);
    });

    suite.test("content_editor is denied POST /api/v1/admin/ssb-tags with 403", { req: "REQ-006" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.post("/api/v1/admin/ssb-tags", {
        json: {
          id: 0,
          code: "unauth-tag",
          name_tr: "Yetkisiz",
          name_en: "Unauth",
          icon_name: "shield",
          sort_order: 99,
          is_active: true,
        },
      });
      c.assert.assertStatus(res, 403);
    });
  });
}
