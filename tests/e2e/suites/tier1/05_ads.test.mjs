import {
  getCachedSuperAdmin,
  makeAdPayload,
  makeContentPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-021: Ad formats & multi-dimensional targeting ─────────────────────
  suite.describe("REQ-021 Ad Formats & Multi-Dimensional Targeting", () => {
    suite.test("GET /api/v1/ads returns active banners and floating popup campaign", { req: "REQ-021" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/ads");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 1);
      c.assert.assertOk(res.json.some((a) => a.is_floating_popup), "Expected floating popup ad in public ads");
    });

    suite.test("admin creates targeted ad campaign with category, SSB tag, and institution targeting", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `TargetCo_${uid("ad")}`;
      const res = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-news-infeed",
          sponsor_name: sponsor,
          sponsor_type: "auxiliary_service",
          budget_score: 95,
          target_categories: ["haberler"],
          target_ssb_tags: ["radarlar-ve-sensorler"],
          target_institutions: ["MSB"],
          force_override: true,
        }),
      });
      c.assert.assertStatus(res, 200);
      const id = unwrap(res)?.id;
      c.assert.assertOk(id > 0);
      await sa.client.delete(`/api/v1/admin/ads/${id}`);
    });

    suite.test("targeted ad matches authenticated reader institution and article SSB tag", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `InstMatch_${uid("im")}`;
      const slug = uniqueSlug("t1-ad-match");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Ad Match ${slug}`,
          ssb_tags: ["radarlar-ve-sensorler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const adRes = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-mag-top",
          dimensions: "970x90",
          sponsor_name: sponsor,
          sponsor_type: "defense_oem",
          pill_style: "gold",
          budget_score: 99,
          target_categories: ["haberler"],
          target_ssb_tags: ["radarlar-ve-sensorler"],
          target_institutions: ["MSB"],
          force_override: true,
        }),
      });
      c.assert.assertStatus(adRes, 200);
      const adId = unwrap(adRes)?.id;
      try {
        const msbUser = await c.auth.createUserWithRole("subscriber_public", {
          superAdminClient: sa.client,
          organization_category: "MSB",
        });
        const ads = await msbUser.client.get("/api/v1/ads", {
          query: { category: "haberler", content_slug: slug },
        });
        c.assert.assertStatus(ads, 200);
        c.assert.assertOk(ads.json.some((a) => a.sponsor_name === sponsor));
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${adId}`);
      }
    });

    suite.test("GET /api/v1/admin/ads lists all campaigns including rotation_mode and schedule fields", { req: "REQ-021" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/ads");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 5);
      c.assert.assertOk("rotation_mode" in res.json[0], "Expected rotation_mode field on AdCampaign");
    });

    suite.test("homepage SSR renders ad strip and floating popup container", { req: "REQ-021" }, async (c) => {
      const res = await c.http.createClient().get("/");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /ad-floating-popup|msi-ad|sponsor/i);
    });
  });

  // ── REQ-022: Ad quota conflict + schedule date persistence ────────────────
  suite.describe("REQ-022 Ad Quota Conflict & Schedule Dates", () => {
    suite.test("POST /api/v1/admin/ads persists start_date, end_date, and rotation_mode", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sponsor = `Sched_${uid("sc")}`;
      const res = await sa.client.post("/api/v1/admin/ads", {
        json: makeAdPayload({
          slot_code: "ad-news-billboard",
          dimensions: "970x250",
          sponsor_name: sponsor,
          sponsor_type: "auxiliary_service",
          pill_style: "emerald",
          budget_score: 70,
          target_categories: ["haberler"],
          target_institutions: ["MSB"],
          start_date: "2026-01-01",
          end_date: "2026-12-31",
          rotation_mode: "equal",
          force_override: true,
        }),
      });
      c.assert.assertStatus(res, 200);
      const id = unwrap(res)?.id;
      try {
        const list = await sa.client.get("/api/v1/admin/ads");
        const saved = list.json.find((a) => a.id === id);
        c.assert.assertEqual(saved?.start_date, "2026-01-01");
        c.assert.assertEqual(saved?.end_date, "2026-12-31");
        c.assert.assertEqual(saved?.rotation_mode, "equal");
      } finally {
        await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("POST /api/v1/admin/ads/check-conflict detects quota exhaustion on overlapping tag/slot", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const tag = `quota-tag-${uid("qt")}`;
      const createdIds = [];
      try {
        for (let i = 0; i < 3; i++) {
          const r = await sa.client.post("/api/v1/admin/ads", {
            json: makeAdPayload({
              slot_code: "ad-news-top",
              dimensions: "970x90",
              sponsor_name: `QuotaOEM_${i}_${uid("q")}`,
              sponsor_type: "defense_oem",
              target_ssb_tags: [tag],
              start_date: "2026-06-01",
              end_date: "2026-06-30",
              force_override: true,
            }),
          });
          c.assert.assertStatus(r, 200);
          createdIds.push(unwrap(r)?.id);
        }

        const chk = await sa.client.post("/api/v1/admin/ads/check-conflict", {
          json: {
            slot_code: "ad-news-top",
            sponsor_name: "FourthOEM",
            target_ssb_tags: [tag],
            start_date: "2026-06-10",
            end_date: "2026-06-20",
          },
        });
        c.assert.assertStatus(chk, 200);
        c.assert.assertEqual(chk.json?.has_conflict, true);
      } finally {
        for (const id of createdIds) await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("POST /api/v1/admin/ads returns 409 Conflict when quota exceeded without force_override", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const tag = `conflict-tag-${uid("ct")}`;
      const createdIds = [];
      try {
        for (let i = 0; i < 3; i++) {
          const r = await sa.client.post("/api/v1/admin/ads", {
            json: makeAdPayload({
              slot_code: "ad-mag-mid",
              sponsor_name: `ConflictOEM_${i}_${uid("c")}`,
              sponsor_type: "defense_oem",
              target_ssb_tags: [tag],
              force_override: true,
            }),
          });
          c.assert.assertStatus(r, 200);
          createdIds.push(unwrap(r)?.id);
        }

        const conflictRes = await sa.client.post("/api/v1/admin/ads", {
          json: makeAdPayload({
            slot_code: "ad-mag-mid",
            sponsor_name: "Blocked4thOEM",
            sponsor_type: "defense_oem",
            target_ssb_tags: [tag],
          }),
        });
        c.assert.assertStatus(conflictRes, 409);
        c.assert.assertEqual(conflictRes.json?.conflict_check?.has_conflict, true);
      } finally {
        for (const id of createdIds) await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("POST /api/v1/admin/ads succeeds when force_override=true is explicitly passed", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const tag = `override-tag-${uid("ot")}`;
      const createdIds = [];
      try {
        for (let i = 0; i < 4; i++) {
          const r = await sa.client.post("/api/v1/admin/ads", {
            json: makeAdPayload({
              slot_code: "ad-mag-pricing",
              dimensions: "300x250",
              sponsor_name: `OverrideOEM_${i}_${uid("o")}`,
              sponsor_type: "defense_oem",
              target_ssb_tags: [tag],
              force_override: true,
            }),
          });
          c.assert.assertStatus(r, 200);
          createdIds.push(unwrap(r)?.id);
        }
        c.assert.assertEqual(createdIds.length, 4);
      } finally {
        for (const id of createdIds) await sa.client.delete(`/api/v1/admin/ads/${id}`);
      }
    });

    suite.test("Admin Ad Campaign Drawer exposes start_date/end_date schedule inputs (M4 contract)", { req: "REQ-022" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const panelRes = await sa.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(panelRes, 200);
      c.assert.assertMatch(
        panelRes.text,
        /id="ad-start-date"|name="start_date"/i,
        "M4 #form-ad-campaign must include start_date input"
      );
      c.assert.assertMatch(
        panelRes.text,
        /id="ad-end-date"|name="end_date"/i,
        "M4 #form-ad-campaign must include end_date input"
      );
    });
  });

  // ── REQ-023: Competitor protection on sponsored content ───────────────────
  suite.describe("REQ-023 Competitor Protection & Fallback", () => {
    suite.test("sponsored content detail excludes defense_oem ads from context_ads", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-comp1");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsorlu Rakip Korumasi ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "OTOKAR",
          sponsor_legal_label: "SPONSORLU İÇERİK",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.paywall?.competitor_protection_active, true);
      for (const ad of detail.json?.context_ads || []) {
        c.assert.assertNotEqual(
          ad.sponsor_type,
          "defense_oem",
          "Competing defense_oem ad must never appear on sponsored content"
        );
      }
    });

    suite.test("GET /api/v1/ads?content_slug=... excludes defense_oem when content is sponsored", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-comp2");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Ad Filter ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "FNSS",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const ads = await c.http.createClient().get("/api/v1/ads", { query: { content_slug: slug } });
      c.assert.assertStatus(ads, 200);
      for (const ad of ads.json) {
        c.assert.assertOk(
          ad.sponsor_type === "auxiliary_service" || ad.sponsor_type === "in_house",
          `Unexpected sponsor_type ${ad.sponsor_type} on sponsored content`
        );
      }
    });

    suite.test("sponsored content renders auxiliary_service or in_house fallback ads", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-comp3");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Fallback Check ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "BMC",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const ads = await c.http.createClient().get("/api/v1/ads", { query: { content_slug: slug } });
      c.assert.assertStatus(ads, 200);
      c.assert.assertOk(ads.json.length >= 1, "Fallback auxiliary/in_house ads should still be returned");
    });

    suite.test("non-sponsored content allows defense_oem ads in context_ads", { req: "REQ-023" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-comp4");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Normal Haber ${slug}`,
          is_sponsored: false,
        }),
      });
      c.assert.assertStatus(cr, 200);
      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.paywall?.competitor_protection_active, false);
      c.assert.assertOk((detail.json?.context_ads || []).length >= 1);
    });

    suite.test("unmatched category/tag query falls back to default/highest-budget ads", { req: "REQ-023" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/ads", {
        query: { category: "nonexistent-category-xyz" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 1, "Expected fallback ads when unmatched");
    });
  });
}
