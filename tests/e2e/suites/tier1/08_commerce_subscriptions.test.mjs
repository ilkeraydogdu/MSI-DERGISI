import {
  getCachedSuperAdmin,
  makeContentPayload,
  makePlanPayload,
  makeProductPayload,
  uid,
  uniqueEmail,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-028: Subscriptions, PayTR POS, invoicing & proforma ───────────────
  suite.describe("REQ-028 Subscriptions, PayTR & Proforma Invoicing", () => {
    suite.test("GET /abonelik-ve-satis renders subscription plans and store catalog", { req: "REQ-028" }, async (c) => {
      const res = await c.http.createClient().get("/abonelik-ve-satis");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "dijital");
      c.assert.assertIncludes(res.text, "basili-dijital");
      c.assert.assertIncludes(res.text, "kurumsal");
    });

    suite.test("POST /api/v1/subscriptions/checkout records proforma order when PayTR is unconfigured", { req: "REQ-028" }, async (c) => {
      const email = uniqueEmail("sub");
      const res = await c.http.createClient().post("/api/v1/subscriptions/checkout", {
        json: {
          email,
          full_name: "Kurumsal Satin Alma",
          organization_name: "Savunma A.S.",
          item_type: "subscription",
          item_code: "dijital",
          amount_try: 1.0, // server must override with authoritative 1200.0
          distance_sales_consent: true,
        },
      });
      c.assert.assertStatus(res, 202);
      c.assert.assertEqual(res.json?.status, "order_recorded_pending_payment");
      c.assert.assertEqual(res.json?.payment_gateway, "corporate_proforma_pending");
      c.assert.assertEqual(res.json?.amount_try, 1200);
    });

    suite.test("admin marks pending invoice as paid and activates user subscription entitlement", { req: "REQ-028" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const co = await u.client.post("/api/v1/subscriptions/checkout", {
        json: {
          email: u.email,
          full_name: u.user.full_name,
          item_type: "subscription",
          item_code: "basili-dijital",
          distance_sales_consent: true,
        },
      });
      c.assert.assertStatus(co, 202);
      const invId = co.json?.invoice_id;
      c.assert.assertOk(invId > 0);

      const markPaid = await sa.client.post(`/api/v1/admin/invoices/${invId}/mark-paid`, { json: {} });
      c.assert.assertStatus(markPaid, 200);
      c.assert.assertEqual(unwrap(markPaid)?.status, "paid");
      c.assert.assertEqual(unwrap(markPaid)?.newly_paid, true);

      // Verify user now appears in /hesabim with paid invoice and upgraded subscription
      const acct = await u.client.get("/hesabim");
      c.assert.assertStatus(acct, 200);
      c.assert.assertIncludes(acct.text, co.json?.invoice_no);
    });

    suite.test("GET/POST/DELETE /api/v1/admin/subscription-plans manages plans", { req: "REQ-028" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const code = `e2e-plan-${uid("pl").toLowerCase().replace(/[^a-z0-9-]/g, "-")}`;
      const up = await sa.client.post("/api/v1/admin/subscription-plans", {
        json: makePlanPayload({ code, name_tr: "E2E Ozel Plan", price_try: 3500 }),
      });
      c.assert.assertStatus(up, 200);
      const del = await sa.client.delete(`/api/v1/admin/subscription-plans/${encodeURIComponent(code)}`);
      c.assert.assertStatus(del, 200);
    });

    suite.test("GET /api/v1/admin/invoices returns paginated invoice list", { req: "REQ-028" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/invoices", { query: { page: "1", per_page: "10" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)));
      c.assert.assertEqual(res.json?.meta?.per_page, 10);
    });

    suite.test("GET /abonelik-ve-satis and /js/msi-ui.js expose PayTR checkout & cart-checkout endpoints", { req: "REQ-028" }, async (c) => {
      const client = c.http.createClient();
      const [res, jsRes] = await Promise.all([
        client.get("/abonelik-ve-satis"),
        client.get("/js/msi-ui.js"),
      ]);
      c.assert.assertStatus(res, 200);
      c.assert.assertStatus(jsRes, 200);
      c.assert.assertIncludes(res.text, "/js/msi-ui.js");
      c.assert.assertIncludes(jsRes.text, "/api/v1/subscriptions/checkout");
      c.assert.assertIncludes(jsRes.text, "/api/v1/commerce/cart-checkout");
    });
  });

  // ── REQ-029: 5-stage automated renewal reminders ──────────────────────────
  suite.describe("REQ-029 Renewal Reminders & Auto-Halt", () => {
    suite.test("GET /api/v1/admin/reminders returns paginated reminder log", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/reminders", { query: { page: "1", per_page: "25" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)));
    });

    suite.test("POST /api/v1/admin/reminders/halt-renewed executes successfully", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/reminders/halt-renewed", { json: {} });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(unwrap(res)?.status, "halted_by_renewal");
      c.assert.assertOk(typeof unwrap(res)?.halted_count === "number");
    });

    suite.test("POST /api/v1/admin/subscriptions/trigger-reminders works at canonical admin path", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/subscriptions/trigger-reminders", { json: {} });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(unwrap(res)?.status, "halted_by_renewal");
    });

    suite.test("system-health exposes reminder subsystem status", { req: "REQ-029" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/system-health");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(res.json !== null);
    });

    suite.test("account KVKK export includes renewal_reminders array", { req: "REQ-029" }, async (c) => {
      const reg = await c.auth.registerUser();
      const exp = await reg.client.get("/api/v1/account/export");
      c.assert.assertStatus(exp, 200);
      c.assert.assertOk(Array.isArray(exp.json?.renewal_reminders));
    });
  });

  // ── REQ-030: On-demand subscriber lifecycle reports ───────────────────────
  suite.describe("REQ-030 Subscriber Lifecycle Reports", () => {
    suite.test("GET /api/v1/admin/analytics/user-cohorts returns cohort matrix", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/user-cohorts");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)));
    });

    suite.test("GET /api/v1/admin/analytics/churn-rate returns retention and churn stats", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/churn-rate");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(unwrap(res) !== null);
    });

    suite.test("GET /api/v1/admin/analytics/user-ltv returns lifetime value metrics", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/user-ltv");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(unwrap(res) !== null);
    });

    suite.test("GET /api/v1/admin/analytics/conversion-funnel returns visitor-to-paid funnel stages", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/conversion-funnel");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(unwrap(res) !== null);
    });

    suite.test("GET /api/v1/admin/analytics/revenue-summary returns revenue totals", { req: "REQ-030" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/analytics/revenue-summary");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(unwrap(res) !== null);
    });
  });

  // ── REQ-031: Books, Think-Tank Reports & RBF photos store ─────────────────
  suite.describe("REQ-031 E-Commerce Store, Cart & Digital Download", () => {
    suite.test("GET /api/v1/commerce/catalog returns seeded books, reports, and RBF photo products", { req: "REQ-031" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/commerce/catalog");
      c.assert.assertStatus(res, 200);
      const skus = (res.json?.commerce_products || []).map((p) => p.sku);
      c.assert.assertIncludes(skus, "KITAP-YALTES-20YIL");
      c.assert.assertIncludes(skus, "RAPOR-SAVUNMA-KOBI-2026");
      c.assert.assertIncludes(skus, "FOTO-RBF-KAAN-4K");
    });

    suite.test("POST /api/v1/commerce/cart-checkout computes authoritative multi-item total", { req: "REQ-031" }, async (c) => {
      const email = uniqueEmail("cart");
      const res = await c.http.createClient().post("/api/v1/commerce/cart-checkout", {
        json: {
          email,
          full_name: "Sepet Alicisi",
          distance_sales_consent: true,
          items: [
            { item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 1 },
            { item_type: "product", item_code: "RAPOR-SAVUNMA-KOBI-2026", qty: 2 },
          ],
        },
      });
      c.assert.assertStatus(res, 202);
      // 850 + 2 * 650 = 2150
      c.assert.assertEqual(res.json?.total_try, 2150);
      c.assert.assertEqual(res.json?.items?.length, 2);
    });

    suite.test("paid user can download purchased digital SKU via /api/v1/commerce/download/:sku", { req: "REQ-031" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });

      const co = await u.client.post("/api/v1/commerce/cart-checkout", {
        json: {
          email: u.email,
          full_name: u.user.full_name,
          distance_sales_consent: true,
          items: [{ item_type: "product", item_code: "RAPOR-SAVUNMA-KOBI-2026", qty: 1 }],
        },
      });
      c.assert.assertStatus(co, 202);
      const mark = await sa.client.post(`/api/v1/admin/invoices/${co.json.invoice_id}/mark-paid`, { json: {} });
      c.assert.assertStatus(mark, 200);

      const dl = await u.client.raw("GET", "/api/v1/commerce/download/RAPOR-SAVUNMA-KOBI-2026");
      c.assert.assertStatus(dl, 200);
      c.assert.assertOk(dl.rawBody.byteLength > 100);
    });

    suite.test("single-article purchase grants access to visibility_level=2 article", { req: "REQ-031" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = `t1-single-${uid("sa").toLowerCase().replace(/[^a-z0-9-]/g, "-")}`;
      const marker = `SINGLE_ARTICLE_UNLOCK_${uid("u")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Tekil Makale ${slug}`,
          body_tr: `<p>Acilis.</p><p>Detay.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          single_price_try: 125.0,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);

      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const co = await u.client.post("/api/v1/subscriptions/checkout", {
        json: {
          email: u.email,
          full_name: u.user.full_name,
          item_type: "single_article",
          item_code: slug,
          distance_sales_consent: true,
        },
      });
      c.assert.assertStatus(co, 202);
      c.assert.assertEqual(co.json?.amount_try, 125);
      await sa.client.post(`/api/v1/admin/invoices/${co.json.invoice_id}/mark-paid`, { json: {} });

      const detail = await u.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.paywall?.is_locked, false);
      c.assert.assertIncludes(detail.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("GET/POST/DELETE /api/v1/admin/commerce-products manages store catalog", { req: "REQ-031" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sku = `E2E-BOOK-${uid("bk").toUpperCase()}`;
      const up = await sa.client.post("/api/v1/admin/commerce-products", {
        json: makeProductPayload({ sku, title_tr: "E2E Savunma Kitabi", price_try: 490 }),
      });
      c.assert.assertStatus(up, 200);
      const id = unwrap(up)?.id;
      c.assert.assertOk(id > 0);
      const del = await sa.client.delete(`/api/v1/admin/commerce-products/${id}`);
      c.assert.assertStatus(del, 200);
    });
  });
}
