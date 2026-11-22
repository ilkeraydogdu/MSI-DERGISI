import {
  getCachedSuperAdmin,
  getSeededContentSlug,
  makeContentPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-024: Reading depth, video, ad & ssb_tag telemetry ─────────────────
  suite.describe("REQ-024 Reading Depth, Video, Ad & ssb_tag Telemetry", () => {
    suite.test("POST /api/v1/analytics/track marks is_true_read=true when duration>=25s and scroll>=70%", { req: "REQ-024" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("tr")}`,
          user_id: null,
          organization_category: "MSB",
          organization_name: "MSB",
          event_type: "article_read",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 30,
          scroll_depth_pct: 75,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.status, "recorded");
      c.assert.assertEqual(res.json?.is_true_read, true);
    });

    suite.test("POST /api/v1/analytics/track marks video_completed when video_watch_pct >= 95", { req: "REQ-024" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("vid")}`,
          user_id: null,
          organization_category: "SSB",
          organization_name: "SSB",
          event_type: "video_progress",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 60,
          scroll_depth_pct: 50,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 96,
        },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.video_watch_pct, 96);
    });

    suite.test("POST /api/v1/analytics/track records ad_impression and ad_click events", { req: "REQ-024" }, async (c) => {
      const client = c.http.createClient();
      const imp = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("imp")}`,
          user_id: null,
          organization_category: "Sektör Kuruluşları",
          organization_name: "Test Co",
          event_type: "ad_impression",
          content_slug: null,
          ad_id: 1,
          duration_sec: 0,
          scroll_depth_pct: 0,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      const clk = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("clk")}`,
          user_id: null,
          organization_category: "Sektör Kuruluşları",
          organization_name: "Test Co",
          event_type: "ad_click",
          content_slug: null,
          ad_id: 1,
          duration_sec: 0,
          scroll_depth_pct: 0,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      c.assert.assertStatus(imp, 200);
      c.assert.assertStatus(clk, 200);
    });

    suite.test("keyword-performance reflects tracked ssb_tag analytics events", { req: "REQ-024" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      await c.http.createClient().post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("kw")}`,
          user_id: null,
          organization_category: "MSB",
          organization_name: "MSB",
          event_type: "article_read",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 40,
          scroll_depth_pct: 85,
          video_completed: false,
          is_true_read: true,
          ssb_tag: "radarlar-ve-sensorler",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      const res = await sa.client.get("/api/v1/admin/analytics/keyword-performance");
      c.assert.assertStatus(res, 200);
      const rows = unwrap(res);
      c.assert.assertOk(Array.isArray(rows) && rows.some((r) => JSON.stringify(r).includes("radarlar-ve-sensorler")));
    });

    suite.test("article SSR sets data-ssb-tag and msi-ui.js populates ssb_tag (M3 contract)", { req: "REQ-024" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ssb-attr");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSB Tag Attr ${slug}`,
          ssb_tags: ["elektronik-harp-sistemleri"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const [articleRes, jsRes] = await Promise.all([
        c.http.createClient().get(`/icerik/${slug}`),
        c.http.createClient().get("/js/msi-ui.js"),
      ]);
      c.assert.assertStatus(articleRes, 200);
      c.assert.assertStatus(jsRes, 200);
      c.assert.assertIncludes(
        articleRes.text,
        "data-ssb-tag=",
        "M3 reader.rs must emit data-ssb-tag attribute on article/page root"
      );
      c.assert.assertNotIncludes(
        jsRes.text,
        "ssb_tag: null,",
        "M3 msi-ui.js buildAnalyticsPayload must not hardcode ssb_tag: null"
      );
    });
  });

  // ── REQ-025: Per-reader report + k>=3 anonymization ───────────────────────
  suite.describe("REQ-025 Reader Reports & k>=3 Anonymization", () => {
    suite.test("GET /api/v1/admin/analytics/readers returns per-reader read and ad interaction metrics", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/readers");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json));
    });

    suite.test("GET /api/v1/admin/analytics/readers?anonymize=1 masks user_id, email, and full_name", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = await getSeededContentSlug(c, true);
      const u = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        organization_category: "MSB",
      });
      await u.client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("k3")}`,
          user_id: u.id,
          organization_category: "MSB",
          organization_name: "MSB",
          event_type: "article_read",
          content_slug: slug,
          ad_id: null,
          duration_sec: 35,
          scroll_depth_pct: 80,
          video_completed: false,
          is_true_read: true,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });

      const res = await sa.client.get("/api/v1/admin/analytics/readers", {
        query: { anonymize: "1" },
      });
      c.assert.assertStatus(res, 200);
      for (const r of res.json) {
        c.assert.assertEqual(r.user_id, 0, "Anonymized reader report must mask user_id=0");
        c.assert.assertNotEqual(r.email, u.email, "Anonymized reader report must not expose raw email");
        c.assert.assertMatch(r.email || "", /^anon_[0-9a-f]+$/, "Anonymized reader email must be hashed anon_* token");
        c.assert.assertNotIncludes(r.full_name || "", u.email);
      }
    });

    suite.test("GET /api/v1/admin/analytics/bundle?anonymize=true sets anonymized=true across bundle", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/bundle", {
        query: { anonymize: "true" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.anonymized, true);
      c.assert.assertOk(Array.isArray(res.json?.contents));
      c.assert.assertOk(Array.isArray(res.json?.ads));
      c.assert.assertOk(Array.isArray(res.json?.readers));
    });

    suite.test("k-anonymity collapses small (<3 user) institutional cohorts into Diğer Kurumlar (k<3)", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const rareOrg = `RareInst_${uid("ro")}`;
      const u = await c.auth.createUserWithRole("registered", {
        superAdminClient: sa.client,
        organization_category: rareOrg,
      });
      await u.client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("rk")}`,
          user_id: u.id,
          organization_category: rareOrg,
          organization_name: rareOrg,
          event_type: "article_read",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 30,
          scroll_depth_pct: 75,
          video_completed: false,
          is_true_read: true,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      const res = await sa.client.get("/api/v1/admin/analytics/readers", {
        query: { anonymize: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(JSON.stringify(res.json), rareOrg, "Single-user rareOrg must not leak in k>=3 anonymized output");
    });

    suite.test("GET /api/v1/admin/analytics/daily?anonymize=1 suppresses low-k rows", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [rawRes, anonRes] = await Promise.all([
        sa.client.get("/api/v1/admin/analytics/daily"),
        sa.client.get("/api/v1/admin/analytics/daily", { query: { anonymize: "1" } }),
      ]);
      c.assert.assertStatus(rawRes, 200);
      c.assert.assertStatus(anonRes, 200);
      const rawRows = unwrap(rawRes);
      const rows = unwrap(anonRes);
      c.assert.assertOk(Array.isArray(rawRows) && Array.isArray(rows));
      for (const r of rows) {
        const sess = Number(r.unique_sessions ?? 0);
        const cnt = Number(r.event_count ?? 0);
        c.assert.assertOk(
          sess >= 3 || (sess === 0 && cnt >= 3),
          `Anonymized daily analytics must suppress low-k rows (got unique_sessions=${sess}, event_count=${cnt})`
        );
        c.assert.assertOk(!("email" in r) && !("user_id" in r), "Daily analytics rows must not contain PII");
      }
      const lowKRawCount = rawRows.filter((r) => {
        const sess = Number(r.unique_sessions ?? 0);
        const cnt = Number(r.event_count ?? 0);
        return !(sess >= 3 || (sess === 0 && cnt >= 3));
      }).length;
      c.assert.assertEqual(
        rows.length,
        rawRows.length - lowKRawCount,
        "Anonymized /daily row count must exclude exactly the low-k (<3) rows"
      );
    });
  });

  // ── REQ-026: Per-content/ad institutional & B2B month-end reports ─────────
  suite.describe("REQ-026 Institutional B2B Content & Ad Reports", () => {
    suite.test("GET /api/v1/admin/analytics/contents returns per-content institutional breakdown", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/contents");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 1);
    });

    suite.test("GET /api/v1/admin/analytics/ads returns per-ad impressions, clicks, and institution shares", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/ads");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 1);
    });

    suite.test("GET /api/v1/admin/analytics/sponsor-report works at canonical admin path", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/sponsor-report");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk("contents" in res.json && "ads" in res.json && "readers" in res.json);
    });

    suite.test("GET /api/v1/admin/analytics/advertiser-report returns advertiser ROI breakdown", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/advertiser-report");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)));
    });

    suite.test("GET /api/v1/admin/reports/subscribers returns consolidated monthly institution breakdown", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/reports/subscribers");
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertOk(typeof data?.total_users === "number");
      c.assert.assertOk("by_institution" in data || "institution_breakdown" in data);
    });
  });
}
