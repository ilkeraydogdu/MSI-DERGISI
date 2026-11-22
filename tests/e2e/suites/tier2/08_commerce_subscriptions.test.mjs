import {
  computePaytrHash,
  getCachedSuperAdmin,
  toMerchantOid,
  uid,
  uniqueEmail,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-028 Boundaries: Checkout, PayTR Callback & Invoices ───────────────
  suite.describe("REQ-028 Subscriptions, PayTR & Invoice Boundaries", () => {
    suite.test("POST /api/v1/subscriptions/checkout rejects distance_sales_consent=false with 400", { req: "REQ-028" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/subscriptions/checkout", {
        json: {
          email: uniqueEmail("noconsent"),
          full_name: "Test User",
          item_type: "subscription",
          item_code: "dijital",
          distance_sales_consent: false,
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/subscriptions/checkout rejects unknown item_code with 400", { req: "REQ-028" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/subscriptions/checkout", {
        json: {
          email: uniqueEmail("badplan"),
          full_name: "Test User",
          item_type: "subscription",
          item_code: "nonexistent-plan-999",
          distance_sales_consent: true,
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/subscriptions/checkout rejects mismatched user_email vs email (400)", { req: "REQ-028" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/subscriptions/checkout", {
        json: {
          user_email: uniqueEmail("a"),
          email: uniqueEmail("b"),
          full_name: "Test User",
          item_type: "subscription",
          item_code: "dijital",
          distance_sales_consent: true,
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/payments/paytr-callback and /api/v1/payments/paytr/callback alias return 503 when PayTR keys are unset in isolated env", { req: "REQ-028" }, async (c) => {
      const oid = toMerchantOid("INV20260001");
      const hash = computePaytrHash(oid, "dummy-key", "dummy-salt", "success", "120000");
      for (const cbPath of ["/api/v1/payments/paytr-callback", "/api/v1/payments/paytr/callback"]) {
        const res = await c.http.createClient().post(cbPath, {
          form: {
            merchant_oid: oid,
            status: "success",
            total_amount: "120000",
            hash,
          },
        });
        c.assert.assertStatus(res, 503, `Unconfigured PayTR callback ${cbPath} must return 503`);
      }
    });

    suite.test("POST /api/v1/admin/invoices/999999/mark-paid returns 404 and finance_viewer gets 403", { req: "REQ-028" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const r404 = await sa.client.post("/api/v1/admin/invoices/999999/mark-paid", { json: {} });
      c.assert.assertStatus(r404, 404);

      const fin = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
      const r403 = await fin.client.post("/api/v1/admin/invoices/1/mark-paid", { json: {} });
      c.assert.assertStatus(r403, 403);
    });
  });

  // ── REQ-029 Boundaries: Renewal Reminders & Moved Route ───────────────────
  suite.describe("REQ-029 Renewal Reminder Boundaries", () => {
    suite.test("removed legacy POST /api/v1/subscriptions/trigger-reminders returns 404", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/subscriptions/trigger-reminders", { json: {} });
      c.assert.assertStatus(res, 404);
    });

    suite.test("unauthenticated POST /api/v1/admin/subscriptions/trigger-reminders returns 401", { req: "REQ-029" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/admin/subscriptions/trigger-reminders", { json: {} });
      c.assert.assertStatus(res, 401);
    });

    suite.test("finance_viewer (read-only) is denied POST /api/v1/admin/reminders/halt-renewed (403)", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fin = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
      const res = await fin.client.post("/api/v1/admin/reminders/halt-renewed", { json: {} });
      c.assert.assertStatus(res, 403);
    });

    suite.test("content_editor is denied GET /api/v1/admin/reminders and ads_manager is denied halt-renewed (403)", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const r1 = await ed.client.get("/api/v1/admin/reminders");
      const r2 = await ads.client.post("/api/v1/admin/reminders/halt-renewed", { json: {} });
      c.assert.assertStatus(r1, 403);
      c.assert.assertStatus(r2, 403);
    });

    suite.test("GET /api/v1/admin/reminders clamps out-of-range page/per_page parameters", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/reminders", {
        query: { page: "0", per_page: "9999" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.meta?.page, 1);
      c.assert.assertEqual(res.json?.meta?.per_page, 100);
    });
  });

  // ── REQ-030 Boundaries: Lifecycle Reports RBAC & Bounds ───────────────────
  suite.describe("REQ-030 Subscriber Lifecycle Report Boundaries", () => {
    suite.test("content_editor is denied GET /api/v1/admin/analytics/user-cohorts (403)", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.get("/api/v1/admin/analytics/user-cohorts");
      c.assert.assertStatus(res, 403);
    });

    suite.test("ads_manager is denied GET /api/v1/admin/analytics/revenue-summary (403)", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const res = await ads.client.get("/api/v1/admin/analytics/revenue-summary");
      c.assert.assertStatus(res, 403);
    });

    suite.test("support_agent is denied GET /api/v1/admin/analytics/revenue-summary (403)", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sup = await c.auth.createUserWithRole("support_agent", { superAdminClient: sa.client });
      const res = await sup.client.get("/api/v1/admin/analytics/revenue-summary");
      c.assert.assertStatus(res, 403);
    });

    suite.test("subscriber_digital reader is denied /api/v1/admin/analytics/conversion-funnel (403)", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get("/api/v1/admin/analytics/conversion-funnel");
      c.assert.assertStatus(res, 403);
    });

    suite.test("GET /api/v1/admin/analytics/user-cohorts?anonymize=1 enforces k>=3 suppression", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [rawRes, anonRes] = await Promise.all([
        sa.client.get("/api/v1/admin/analytics/user-cohorts"),
        sa.client.get("/api/v1/admin/analytics/user-cohorts", { query: { anonymize: "1" } }),
      ]);
      c.assert.assertStatus(rawRes, 200);
      c.assert.assertStatus(anonRes, 200);
      const raw = rawRes.json?.data ?? rawRes.json;
      const anon = anonRes.json?.data ?? anonRes.json;
      c.assert.assertOk(Array.isArray(raw) && Array.isArray(anon), "User cohorts must return arrays");

      for (const row of anon) {
        c.assert.assertOk(
          row.registered >= 3,
          `Anonymized cohort ${row.cohort_month} must have registered >= 3 (got ${row.registered})`
        );
        for (const m of [1, 2, 3, 4, 5]) {
          const users = row[`m${m}_users`];
          const active = row[`active_m${m}`];
          c.assert.assertOk(
            users === null || users === 0 || users >= 3,
            `Cohort ${row.cohort_month} m${m}_users must be null, 0, or >=3 (got ${users})`
          );
          c.assert.assertOk(
            active === null || active === 0 || active >= 3,
            `Cohort ${row.cohort_month} active_m${m} must be null, 0, or >=3 (got ${active})`
          );
          if (users === null) {
            c.assert.assertEqual(row[`m${m}`], null, `Cohort ${row.cohort_month} m${m} pct must be null when suppressed`);
          }
        }
        c.assert.assertOk(
          row.active_m6 === null || row.active_m6 === 0 || row.active_m6 >= 3,
          `Cohort ${row.cohort_month} active_m6 must be null, 0, or >=3 (got ${row.active_m6})`
        );
      }

      for (const rawRow of raw) {
        const match = anon.find((r) => r.cohort_month === rawRow.cohort_month);
        if (rawRow.registered < 3) {
          c.assert.assertEqual(match, undefined, `Low-k cohort ${rawRow.cohort_month} (registered=${rawRow.registered}) must be omitted`);
        } else {
          c.assert.assertOk(match, `Cohort ${rawRow.cohort_month} with registered>=3 must remain in anonymized output`);
          for (const m of [1, 2, 3, 4, 5]) {
            const rawAct = rawRow[`active_m${m}`];
            if (rawAct > 0 && rawAct < 3) {
              c.assert.assertEqual(match[`active_m${m}`], null, `Low-k cell active_m${m}=${rawAct} must be suppressed to null`);
              c.assert.assertEqual(match[`m${m}_users`], null, `Low-k cell m${m}_users=${rawAct} must be suppressed to null`);
            }
          }
        }
      }
    });
  });

  // ── REQ-031 Boundaries: Cart Checkout & Digital Downloads ─────────────────
  suite.describe("REQ-031 E-Commerce Cart & Download Boundaries", () => {
    suite.test("POST /api/v1/commerce/cart-checkout rejects empty items array with 400", { req: "REQ-031" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/commerce/cart-checkout", {
        json: {
          email: uniqueEmail("emptycart"),
          full_name: "Test",
          distance_sales_consent: true,
          items: [],
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/commerce/cart-checkout rejects qty=0 and qty=51 (400)", { req: "REQ-031" }, async (c) => {
      const client = c.http.createClient();
      const r0 = await client.post("/api/v1/commerce/cart-checkout", {
        json: {
          email: uniqueEmail("q0"),
          full_name: "Test",
          distance_sales_consent: true,
          items: [{ item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 0 }],
        },
      });
      const r51 = await client.post("/api/v1/commerce/cart-checkout", {
        json: {
          email: uniqueEmail("q51"),
          full_name: "Test",
          distance_sales_consent: true,
          items: [{ item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 51 }],
        },
      });
      c.assert.assertStatus(r0, 400);
      c.assert.assertStatus(r51, 400);
    });

    suite.test("GET /api/v1/commerce/download/:sku returns 401 for unauthenticated user", { req: "REQ-031" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/commerce/download/RAPOR-SAVUNMA-KOBI-2026");
      c.assert.assertStatus(res, 401);
    });

    suite.test("GET /api/v1/commerce/download/:sku returns 403 for unpaid registered user", { req: "REQ-031" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const res = await u.client.get("/api/v1/commerce/download/RAPOR-SAVUNMA-KOBI-2026");
      c.assert.assertStatus(res, 403);
    });

    suite.test("GET /api/v1/commerce/download/..%2F..%2Fetc%2Fpasswd or unknown SKU is rejected (400/403/404)", { req: "REQ-031" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const rTrav = await u.client.get("/api/v1/commerce/download/..%2F..%2F.env");
      const rMiss = await u.client.get(`/api/v1/commerce/download/MISSING-${uid("sku")}`);
      c.assert.assertStatus(rTrav, [400, 403]);
      c.assert.assertStatus(rMiss, 404);
    });
  });
}
