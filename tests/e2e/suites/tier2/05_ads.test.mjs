import {
  getCachedSuperAdmin,
  makeAdPayload,
  makeContentPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-021 Boundaries: Ad Visibility & RBAC ──────────────────────────────
  suite.describe("REQ-021 Ad Format & Targeting Boundaries", () => {
    suite.test("expired ad campaign (end_date in past) is excluded from GET /api/v1/ads", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `ExpiredCo_${uid("exp")}`;
      const r = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          sponsor_name: sponsor,
          budget_score: 100,
          start_date: "2020-01-01",
          end_date: "2020-12-31",
          force_override: true,
        }),
      });
      c.assert.assertStatus(r, 200);
      const id = unwrap(r)?.id;
      try {
        const pub = await c.http.createClient().get("/api/v1/ads");
        c.assert.assertOk(!pub.json.some((a) => a.sponsor_name === sponsor), "Expired ad must not serve publicly");
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("future ad campaign (start_date in 2035) is excluded from GET /api/v1/ads", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `FutureCo_${uid("fut")}`;
      const r = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          sponsor_name: sponsor,
          budget_score: 100,
          start_date: "2035-01-01",
          end_date: "2035-12-31",
          force_override: true,
        }),
      });
      c.assert.assertStatus(r, 200);
      const id = unwrap(r)?.id;
      try {
        const pub = await c.http.createClient().get("/api/v1/ads");
        c.assert.assertOk(!pub.json.some((a) => a.sponsor_name === sponsor), "Future ad must not serve publicly");
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("inactive ad (is_active=false) is excluded from public GET /api/v1/ads", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `InactiveCo_${uid("ina")}`;
      const r = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({ sponsor_name: sponsor, is_active: false, force_override: true }),
      });
      c.assert.assertStatus(r, 200);
      const id = unwrap(r)?.id;
      try {
        const pub = await c.http.createClient().get("/api/v1/ads");
        c.assert.assertOk(!pub.json.some((a) => a.sponsor_name === sponsor));
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("content_editor is denied GET /api/v1/admin/ads with 403", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.get("/api/v1/admin/ads");
      c.assert.assertStatus(res, 403);
    });

    suite.test("finance_viewer is denied POST /api/v1/admin/ads with 403", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fin = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
      const res = await fin.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({ slot_code: "ad-news-top", sponsor_name: "Denied", sponsor_type: "in_house" }),
      });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-022 Boundaries: Quota Conflict & Route Auth ───────────────────────
  suite.describe("REQ-022 Ad Quota Conflict & Schedule Boundaries", () => {
    suite.test("POST /api/v1/admin/ads rejects start_date > end_date with 400", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-news-top",
          sponsor_name: "BadDates",
          start_date: "2026-12-31",
          end_date: "2026-01-01",
        }),
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("non-overlapping date windows on same slot/tag do NOT report quota conflict", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const tag = `nonoverlap-${uid("no")}`;
      const ids = [];
      try {
        for (let i = 0; i < 3; i++) {
          const r = await sa.client.post("/api/v1/admin/ads", {
            json: makeAdPayload({
              sponsor_name: `Win1_${i}`,
              sponsor_type: "defense_oem",
              target_ssb_tags: [tag],
              start_date: "2027-01-01",
              end_date: "2027-01-31",
              force_override: true,
            }),
          });
          c.assert.assertStatus(r, 200);
          ids.push(unwrap(r)?.id);
        }
        const chk = await sa.client.post("/api/v1/admin/ads/check-conflict", {
          json: {
            slot_code: "ad-news-infeed",
            sponsor_name: "Win2",
            target_ssb_tags: [tag],
            start_date: "2027-03-01",
            end_date: "2027-03-31",
          },
        });
        c.assert.assertStatus(chk, 200);
        c.assert.assertEqual(chk.json?.has_conflict, false);
      } finally {
        for (const id of ids) await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("is_default_fallback=true is exempt from 409 quota conflict block", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-news-top",
          sponsor_name: `FallbackEx_${uid("fb")}`,
          sponsor_type: "in_house",
          is_default_fallback: true,
          target_ssb_tags: ["hava-araclari"],
        }),
      });
      c.assert.assertStatus(res, 200);
      await sa.client.delete(`/api/v1/admin/ads/${unwrap(res)?.id}`);
    });

    suite.test("removed legacy POST /api/v1/ads/check-conflict returns 404 (moved under /admin/)", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/ads/check-conflict", {
        json: { slot_code: "ad-news-top", sponsor_name: "Legacy" },
      });
      c.assert.assertStatus(res, 404);
    });

    suite.test("content_editor is denied POST /api/v1/admin/ads/check-conflict with 403", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.post("/api/v1/admin/ads/check-conflict", {
        json: { slot_code: "ad-news-top", sponsor_name: "Denied" },
      });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-023 Boundaries: Competitor Protection ─────────────────────────────
  suite.describe("REQ-023 Competitor Protection Boundaries", () => {
    suite.test("high-budget defense_oem (score=100) is still blocked on sponsored content", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-comp-hi");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsor ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "MKE",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const oemSponsor = `MaxOEM_${uid("m")}`;
      const adRes = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-mag-top",
          dimensions: "970x90",
          sponsor_name: oemSponsor,
          sponsor_type: "defense_oem",
          pill_style: "gold",
          budget_score: 100,
          target_categories: ["advertorial"],
          force_override: true,
        }),
      });
      c.assert.assertStatus(adRes, 200);
      const adId = unwrap(adRes)?.id;
      try {
        const ads = await c.http.createClient().get("/api/v1/ads", { query: { content_slug: slug } });
        c.assert.assertOk(!ads.json.some((a) => a.sponsor_name === oemSponsor));
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${adId}`);
      }
    });

    suite.test("defense_oem with is_default_fallback=true is still blocked on sponsored content", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-comp-fb");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsor FB ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "KALE",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      for (const ad of detail.json?.context_ads || []) {
        c.assert.assertNotEqual(ad.sponsor_type, "defense_oem");
      }
    });

    suite.test("auxiliary_service ad IS permitted on sponsored content", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-comp-aux");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Aux Ok ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "OTOKAR",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const ads = await c.http.createClient().get("/api/v1/ads", { query: { content_slug: slug } });
      c.assert.assertStatus(ads, 200);
      c.assert.assertOk(ads.json.some((a) => a.sponsor_type === "auxiliary_service" || a.sponsor_type === "in_house"));
    });

    suite.test("sponsored content with empty sponsor_brand still enforces competitor shield", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-comp-nobrand");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `NoBrand Sponsor ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(detail.json?.paywall?.competitor_protection_active, true);
    });

    suite.test("SSR /icerik/:slug for sponsored article omits defense_oem banners", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-comp-ssr");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSR Sponsor Shield ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "ROKETSAN",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, "ad-mag-top"); // seeded TUSAŞ defense_oem
    });
  });
}
