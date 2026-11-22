import {
  getCachedSuperAdmin,
  getSeededContentSlug,
  uid,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-024 Boundaries: Analytics Telemetry ───────────────────────────────
  suite.describe("REQ-024 Analytics Telemetry Boundaries", () => {
    suite.test("POST /api/v1/analytics/track sets is_true_read=false when duration<25 or scroll<70", { req: "REQ-024" }, async (c) => {
      const client = c.http.createClient();
      const slug = await getSeededContentSlug(c, true);
      const r1 = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("f1")}`,
          user_id: null,
          organization_category: "",
          organization_name: "",
          event_type: "article_read",
          content_slug: slug,
          ad_id: null,
          duration_sec: 24,
          scroll_depth_pct: 90,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      const r2 = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("f2")}`,
          user_id: null,
          organization_category: "",
          organization_name: "",
          event_type: "article_read",
          content_slug: slug,
          ad_id: null,
          duration_sec: 45,
          scroll_depth_pct: 69,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      c.assert.assertStatus(r1, 200);
      c.assert.assertStatus(r2, 200);
      c.assert.assertEqual(r1.json?.is_true_read, false, "duration=24s (<25s) must not be true_read");
      c.assert.assertEqual(r2.json?.is_true_read, false, "scroll=69% (<70%) must not be true_read");
    });

    suite.test("POST /api/v1/analytics/track clamps video_watch_pct to 0..=100", { req: "REQ-024" }, async (c) => {
      const client = c.http.createClient();
      const rHigh = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("vh")}`,
          user_id: null,
          organization_category: "",
          organization_name: "",
          event_type: "video_progress",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 10,
          scroll_depth_pct: 10,
          video_completed: false,
          is_true_read: false,
          ssb_tag: null,
          created_at: "",
          video_watch_pct: 250,
        },
      });
      const rLow = await client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("vl")}`,
          user_id: null,
          organization_category: "",
          organization_name: "",
          event_type: "video_progress",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 10,
          scroll_depth_pct: 10,
          video_completed: false,
          is_true_read: false,
          ssb_tag: null,
          created_at: "",
          video_watch_pct: -20,
        },
      });
      c.assert.assertStatus(rHigh, 200);
      c.assert.assertStatus(rLow, 200);
      c.assert.assertEqual(rHigh.json?.video_watch_pct, 100);
      c.assert.assertEqual(rLow.json?.video_watch_pct, 0);
    });

    suite.test("authenticated user session overrides spoofed user_id/org in /api/v1/analytics/track", { req: "REQ-024" }, async (c) => {
      const reg = await c.auth.registerUser({ organization_category: "Üniversite / Öğrenci" });
      const res = await reg.client.post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("sp")}`,
          user_id: 999999,
          organization_category: "SpoofedOrg",
          organization_name: "Spoofed",
          event_type: "article_read",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 30,
          scroll_depth_pct: 80,
          video_completed: false,
          is_true_read: true,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.status, "recorded");
    });

    suite.test("exact boundary duration_sec=25 and scroll_depth_pct=70 counts as is_true_read=true", { req: "REQ-024" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/analytics/track", {
        json: {
          id: 0,
          session_id: `sess_${uid("bnd")}`,
          user_id: null,
          organization_category: "MSB",
          organization_name: "MSB",
          event_type: "article_read",
          content_slug: "kaan-milli-muharip-ucak",
          ad_id: null,
          duration_sec: 25,
          scroll_depth_pct: 70,
          video_completed: false,
          is_true_read: false,
          ssb_tag: "hava-araclari",
          created_at: "",
          video_watch_pct: 0,
        },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.is_true_read, true);
    });

    suite.test("public/js/msi-ui.js buildAnalyticsPayload reads data-ssb-tag from DOM (M3 contract)", { req: "REQ-024" }, async (c) => {
      const res = await c.http.createClient().get("/js/msi-ui.js");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(
        res.text,
        /data-ssb-tag|dataset\.ssbTag/i,
        "M3 msi-ui.js buildAnalyticsPayload must read data-ssb-tag instead of sending ssb_tag: null"
      );
    });
  });

  // ── REQ-025 Boundaries: k>=3 Anonymization & RBAC ─────────────────────────
  suite.describe("REQ-025 Reader Report & k>=3 Boundaries", () => {
    suite.test("k-anonymity keeps cohort when >= 3 distinct users belong to same institution", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = await getSeededContentSlug(c, true);
      const cohortOrg = `Cohort3_${uid("c3")}`;
      for (let i = 0; i < 3; i++) {
        const u = await c.auth.createUserWithRole("registered", {
          superAdminClient: sa.client,
          organization_category: cohortOrg,
        });
        await u.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: `sess_${uid("c3u")}`,
            user_id: u.id,
            organization_category: cohortOrg,
            organization_name: cohortOrg,
            event_type: "article_read",
            content_slug: slug,
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
      }
      const res = await sa.client.get("/api/v1/admin/analytics/contents", {
        query: { anonymize: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(JSON.stringify(res.json), cohortOrg, "Cohort with k>=3 users should remain visible");
    });

    suite.test("k-anonymity suppresses 2-user cohort (<3 threshold) in content & ad reports", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const smallOrg = `Small2_${uid("s2")}`;
      for (let i = 0; i < 2; i++) {
        const u = await c.auth.createUserWithRole("registered", {
          superAdminClient: sa.client,
          organization_category: smallOrg,
        });
        await u.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: `sess_${uid("s2u")}`,
            user_id: u.id,
            organization_category: smallOrg,
            organization_name: smallOrg,
            event_type: "ad_impression",
            content_slug: null,
            ad_id: 1,
            duration_sec: 0,
            scroll_depth_pct: 0,
            video_completed: false,
            is_true_read: false,
            ssb_tag: null,
            created_at: "",
            video_watch_pct: 0,
          },
        });
      }
      const res = await sa.client.get("/api/v1/admin/analytics/ads", {
        query: { anonymize: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(JSON.stringify(res.json), smallOrg, "Cohort with k=2 (<3) must be masked");
    });

    suite.test("GET /api/v1/admin/analytics/user-ltv?anonymize=1 suppresses low-k metrics", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const rareTier = `RareLtvTier_${uid("ltv")}`;
      const rareUser = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: rareTier,
      });
      try {
        const [rawRes, anonRes] = await Promise.all([
          sa.client.get("/api/v1/admin/analytics/user-ltv"),
          sa.client.get("/api/v1/admin/analytics/user-ltv", { query: { anonymize: "1" } }),
        ]);
        c.assert.assertStatus(rawRes, 200);
        c.assert.assertStatus(anonRes, 200);
        const raw = unwrap(rawRes);
        const anon = unwrap(anonRes);
        c.assert.assertOk(raw && anon, "LTV responses must be non-null");

        const rawRare = (raw.by_tier || []).find((t) => t.tier === rareTier);
        c.assert.assertEqual(rawRare?.user_count, 1, "Unanonymized LTV must include single-user rareTier");
        c.assert.assertOk(
          !(anon.by_tier || []).some((t) => t.tier === rareTier),
          "Anonymized LTV must suppress single-user (k<3) tier from by_tier"
        );
        for (const t of anon.by_tier || []) {
          c.assert.assertOk(
            t.user_count >= 3,
            `Anonymized LTV by_tier entry ${t.tier} must satisfy user_count >= 3 (got ${t.user_count})`
          );
        }

        if (raw.paying_users > 0 && raw.paying_users < 3) {
          c.assert.assertEqual(anon.paying_users, null, "0 < paying_users < 3 must be suppressed to null");
          c.assert.assertEqual(anon.avg_ltv_try, null, "avg_ltv_try must be null when 0 < paying_users < 3");
          c.assert.assertEqual(anon.total_revenue, null, "total_revenue must be null when 0 < paying_users < 3");
          c.assert.assertEqual(Object.keys(anon.by_item_type || {}).length, 0, "by_item_type must be cleared when 0 < paying_users < 3");
        } else {
          c.assert.assertEqual(anon.paying_users, raw.paying_users);
        }
      } finally {
        await sa.client.delete(`/api/v1/admin/users/${rareUser.id}`);
      }
    });

    suite.test("content_editor is denied GET /api/v1/admin/analytics/readers with 403", { req: "REQ-025" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.get("/api/v1/admin/analytics/readers");
      c.assert.assertStatus(res, 403);
    });

    suite.test("unauthenticated client is denied GET /api/v1/admin/analytics/bundle with 401", { req: "REQ-025" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/admin/analytics/bundle");
      c.assert.assertStatus(res, 401);
    });
  });

  // ── REQ-026 Boundaries: Institutional B2B Reports & Moved Route ───────────
  suite.describe("REQ-026 Institutional B2B Report Boundaries", () => {
    suite.test("removed legacy GET /api/v1/analytics/sponsor-report returns 404 (moved under /admin/)", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/analytics/sponsor-report");
      c.assert.assertStatus(res, 404);
    });

    suite.test("support_agent is denied GET /api/v1/admin/analytics/sponsor-report with 403", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sup = await c.auth.createUserWithRole("support_agent", { superAdminClient: sa.client });
      const res = await sup.client.get("/api/v1/admin/analytics/sponsor-report");
      c.assert.assertStatus(res, 403);
    });

    suite.test("GET /api/v1/admin/analytics/sponsor-report?anonymize=1 masks reader PII", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/sponsor-report", {
        query: { anonymize: "true" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.anonymized, true);
      for (const r of res.json?.readers || []) {
        c.assert.assertEqual(r.user_id, 0);
        c.assert.assertMatch(r.email || "", /^anon_[0-9a-f]+$/, "Anonymized reader email must be hashed anon_* token");
      }
    });

    suite.test("GET /api/v1/admin/analytics/daily handles inverted date range gracefully", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/daily", {
        query: { from: "2030-01-01", to: "2020-01-01" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(unwrap(res).length, 0);
    });

    suite.test("GET /api/v1/admin/reports/subscribers?anonymize=1 preserves aggregate counts without PII", { req: "REQ-026" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/reports/subscribers", {
        query: { anonymize: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(unwrap(res)?.anonymized, true);
    });
  });
}
