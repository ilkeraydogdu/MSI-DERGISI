import http from "node:http";
import {
  computePaytrCallbackHash,
  createVerifiedSubscriber,
  uid,
} from "../../fixtures/tier34_helpers.mjs";
import { startIsolatedServer } from "../../lib/server.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 7: Commerce Cart, PayTR, Invoices, /hesabim & KVKK Erasure", () => {
    suite.test(
      "T3-42: Multi-item cart checkout (book + think-tank report + RBF photo) enforces distance_sales_consent & authoritative totals",
      { req: ["REQ-031", "REQ-028"] },
      async (c) => {
        const buyer = await createVerifiedSubscriber(c, "registered");

        // Missing consent -> 400
        const noConsent = await buyer.client.post("/api/v1/commerce/cart-checkout", {
          json: {
            email: buyer.email,
            full_name: buyer.user.full_name,
            distance_sales_consent: false,
            items: [{ item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 1 }],
          },
        });
        assert.assertStatus(
          noConsent,
          400,
          "Cart checkout without distance_sales_consent=true must fail with 400"
        );

        // Valid multi-item cart: 850 + 650 + 2200 = 3700 TRY
        const cartRes = await buyer.client.post("/api/v1/commerce/cart-checkout", {
          json: {
            email: buyer.email,
            full_name: buyer.user.full_name,
            organization_name: "E2E Savunma Tedariği",
            distance_sales_consent: true,
            items: [
              { item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 1 },
              { item_type: "product", item_code: "RAPOR-SAVUNMA-KOBI-2026", qty: 1 },
              { item_type: "product", item_code: "FOTO-RBF-KAAN-4K", qty: 1 },
            ],
          },
        });
        assert.assertStatus(cartRes, 202, "Proforma cart checkout must return 202");
        assert.assertEqual(cartRes.json?.status, "order_recorded_pending_payment");
        assert.assertEqual(cartRes.json?.amount_try, 3700, "Authoritative server total must equal 3700 TRY");
        assert.assertEqual(cartRes.json?.items?.length, 3);
      }
    );

    suite.test(
      "T3-43: Admin invoice mark-paid activates digital SKU download entitlement and blocks non-buyers with 403",
      { req: ["REQ-031", "REQ-017"] },
      async (c) => {
        const buyer = await createVerifiedSubscriber(c, "registered");
        const bystander = await createVerifiedSubscriber(c, "registered");
        const op = await c.auth.loginAsAdmin();

        const sku = "RAPOR-SAVUNMA-KOBI-2026";
        const cartRes = await buyer.client.post("/api/v1/commerce/cart-checkout", {
          json: {
            email: buyer.email,
            full_name: buyer.user.full_name,
            distance_sales_consent: true,
            items: [{ item_type: "product", item_code: sku, qty: 1 }],
          },
        });
        assert.assertStatus(cartRes, 202);
        const invId = cartRes.json?.invoice_id;

        // Before mark-paid, buyer cannot download asset
        const prePayDl = await buyer.client.raw("GET", `/api/v1/commerce/download/${sku}`);
        assert.assertStatus(prePayDl, 403, "Unpaid commerce SKU download must return 403");

        // Mark paid as site_operator
        const paidRes = await op.client.post(`/api/v1/admin/invoices/${invId}/mark-paid`, {
          json: {},
        });
        assert.assertStatus(paidRes, 200);
        assert.assertEqual(paidRes.json?.data?.newly_paid, true);

        // Buyer now downloads PDF entitlement
        const buyerDl = await buyer.client.raw("GET", `/api/v1/commerce/download/${sku}`);
        assert.assertStatus(buyerDl, 200, "Paid SKU download must return 200");
        assert.assertOk(
          buyerDl.rawBody.subarray(0, 5).toString("ascii") === "%PDF-",
          "Digital delivery certificate/asset must be valid PDF"
        );

        // Bystander still gets 403; anonymous gets 401
        const bystanderDl = await bystander.client.raw("GET", `/api/v1/commerce/download/${sku}`);
        assert.assertStatus(bystanderDl, 403, "Non-buyer must be denied commerce SKU download");
        const anonDl = await c.http.createClient().raw("GET", `/api/v1/commerce/download/${sku}`);
        assert.assertStatus(anonDl, 401);
      }
    );

    suite.test(
      "T3-44: Subscription checkout -> mark-paid upgrades user role, shows invoice on /hesabim, and updates lifecycle analytics",
      { req: ["REQ-028", "REQ-029", "REQ-030"] },
      async (c) => {
        const reader = await createVerifiedSubscriber(c, "registered");
        const op = await c.auth.loginAsAdmin();

        const subCheckout = await reader.client.post("/api/v1/subscriptions/checkout", {
          json: {
            email: reader.email,
            full_name: reader.user.full_name,
            item_type: "subscription",
            item_code: "dijital",
            distance_sales_consent: true,
          },
        });
        assert.assertStatus(subCheckout, 202);
        const invId = subCheckout.json?.invoice_id;
        const invNo = subCheckout.json?.invoice_no;

        const paidRes = await op.client.post(`/api/v1/admin/invoices/${invId}/mark-paid`, {
          json: {},
        });
        assert.assertStatus(paidRes, 200);

        // Assert reader's role upgraded to subscriber_digital after mark-paid
        const expAfterPaid = await reader.client.get("/api/v1/account/export");
        assert.assertStatus(expAfterPaid, 200);
        assert.assertEqual(
          expAfterPaid.json?.profile?.role,
          "subscriber_digital",
          "Subscription mark-paid must upgrade reader role to subscriber_digital"
        );
        const reLoginRes = await c.auth.login(c.http.createClient(), {
          email: reader.email,
          password: reader.password,
        });
        assert.assertEqual(
          reLoginRes.user?.role,
          "subscriber_digital",
          "Re-login after subscription mark-paid must reflect upgraded subscriber_digital role"
        );

        const hesabimRes = await reader.client.get("/hesabim");
        assert.assertStatus(hesabimRes, 200);
        assert.assertIncludes(
          hesabimRes.text,
          invNo,
          "/hesabim must display user's invoice number after subscription checkout"
        );

        const triggerRem = await op.client.post("/api/v1/admin/subscriptions/trigger-reminders", {
          json: {},
        });
        assert.assertStatus(triggerRem, 200);

        for (const ep of [
          "/api/v1/admin/analytics/user-cohorts",
          "/api/v1/admin/analytics/churn-rate",
          "/api/v1/admin/analytics/user-ltv",
          "/api/v1/admin/analytics/conversion-funnel",
        ]) {
          const rep = await op.client.get(ep);
          assert.assertStatus(rep, 200, `Lifecycle report ${ep} must return 200`);
        }
      }
    );

    suite.test(
      "T3-45: PayTR gateway HMAC-SHA256 checkout token + callback verification & idempotent entitlement activation",
      { req: ["REQ-028"] },
      async (c) => {
        // Default isolated server leaves PayTR unconfigured -> 503 on callback
        const unconfCb = await c.http.createClient().post("/api/v1/payments/paytr-callback", {
          form: {
            merchant_oid: "MSIMSI2026000001",
            status: "success",
            total_amount: "120000",
            hash: "invalid",
          },
        });
        assert.assertStatus(unconfCb, 503, "Unconfigured PayTR callback must return 503");

        const merchantId = "999888";
        const merchantKey = "e2e_paytr_secret_key_123";
        const merchantSalt = "e2e_paytr_salt_456";

        const paytrStub = http.createServer((req, res) => {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ status: "success", token: "e2e_iframe_tok_999" }));
        });
        await new Promise((resolve) => paytrStub.listen(0, "127.0.0.1", resolve));
        const stubPort = paytrStub.address().port;

        let paytrServer = null;
        try {
          paytrServer = await startIsolatedServer({
            repoRoot: c.repoRoot,
            envOverrides: {
              PAYTR_TEST_MODE: "true",
              PAYTR_MERCHANT_ID: merchantId,
              PAYTR_MERCHANT_KEY: merchantKey,
              PAYTR_MERCHANT_SALT: merchantSalt,
              PAYTR_API_URL: `http://127.0.0.1:${stubPort}/odeme/api/get-token`,
            },
          });

          const cl = new c.http.Client({ baseUrl: paytrServer.baseUrl });
          const buyerEmail = `${uid("paytr.buyer")}@example.com`;
          await cl.post("/api/v1/auth/register", {
            json: {
              email: buyerEmail,
              full_name: "PayTR Buyer",
              password: "E2ePassword!2026",
              kvkk_consent: true,
              kvkk_acknowledged: true,
              tos_accepted: true,
              website_url: "",
            },
          });

          const coRes = await cl.post("/api/v1/subscriptions/checkout", {
            json: {
              email: buyerEmail,
              full_name: "PayTR Buyer",
              item_type: "subscription",
              item_code: "dijital",
              distance_sales_consent: true,
            },
          });
          assert.assertStatus(coRes, 200, "Configured PayTR checkout must return 200 with iframe_token");
          assert.assertEqual(coRes.json?.status, "paytr_checkout_ready");
          assert.assertEqual(coRes.json?.iframe_token, "e2e_iframe_tok_999");

          const merchantOid = coRes.json?.merchant_oid;
          const totalAmount = String(coRes.json?.amount_kurus);

          // Bad hash -> 403
          const badHashCb = await cl.post("/api/v1/payments/paytr-callback", {
            form: {
              merchant_oid: merchantOid,
              status: "success",
              total_amount: totalAmount,
              hash: "tampered_hash_value",
            },
          });
          assert.assertStatus(badHashCb, 403, "Tampered PayTR callback hash must be rejected with 403");

          // Valid hash -> 200 OK
          const validHash = computePaytrCallbackHash({
            merchantKey,
            merchantSalt,
            merchantOid,
            status: "success",
            totalAmount,
          });
          const okCb = await cl.post("/api/v1/payments/paytr-callback", {
            form: {
              merchant_oid: merchantOid,
              status: "success",
              total_amount: totalAmount,
              hash: validHash,
            },
          });
          assert.assertStatus(okCb, 200, "Valid PayTR callback HMAC must succeed with 200 OK");
          assert.assertEqual(okCb.text.trim(), "OK");

          // Idempotency check: second valid PayTR callback with same merchant_oid must still return 200 OK cleanly
          const okCb2 = await cl.post("/api/v1/payments/paytr-callback", {
            form: {
              merchant_oid: merchantOid,
              status: "success",
              total_amount: totalAmount,
              hash: validHash,
            },
          });
          assert.assertStatus(okCb2, 200, "Duplicate PayTR callback must be idempotent (200 OK)");
          assert.assertEqual(okCb2.text.trim(), "OK");

          // Verify invoice marked paid & subscription entitlement/role activated
          const exportAfterPaytr = await cl.get("/api/v1/account/export");
          assert.assertStatus(exportAfterPaytr, 200);
          assert.assertEqual(
            exportAfterPaytr.json?.profile?.role,
            "subscriber_digital",
            "Valid PayTR callback must upgrade buyer role to subscriber_digital"
          );
          assert.assertOk(
            Boolean(exportAfterPaytr.json?.profile?.subscription_expires_at),
            "Valid PayTR callback must set subscription_expires_at"
          );
          const paidOrders = (exportAfterPaytr.json?.orders || []).filter(
            (o) => o.invoice_no === coRes.json?.invoice_no
          );
          assert.assertEqual(paidOrders.length, 1, "Idempotent PayTR callback must keep exactly 1 invoice record");
          assert.assertEqual(
            paidOrders[0]?.payment_status,
            "paid",
            "PayTR callback must finalize invoice payment_status === 'paid'"
          );
          assert.assertIncludes(
            exportAfterPaytr.json?.purchased_item_codes || [],
            "dijital",
            "PayTR callback must record purchased_item_codes entitlement"
          );

          // Verify PayTR status="failed" does NOT mark invoice paid or upgrade user role
          const failCl = new c.http.Client({ baseUrl: paytrServer.baseUrl });
          const failEmail = `${uid("paytr.fail")}@example.com`;
          await failCl.post("/api/v1/auth/register", {
            json: {
              email: failEmail,
              full_name: "PayTR Fail Buyer",
              password: "E2ePassword!2026",
              kvkk_consent: true,
              kvkk_acknowledged: true,
              tos_accepted: true,
              website_url: "",
            },
          });
          const failCo = await failCl.post("/api/v1/subscriptions/checkout", {
            json: {
              email: failEmail,
              full_name: "PayTR Fail Buyer",
              item_type: "subscription",
              item_code: "dijital",
              distance_sales_consent: true,
            },
          });
          assert.assertStatus(failCo, 200);
          const failOid = failCo.json?.merchant_oid;
          const failAmount = String(failCo.json?.amount_kurus);
          const failHash = computePaytrCallbackHash({
            merchantKey,
            merchantSalt,
            merchantOid: failOid,
            status: "failed",
            totalAmount: failAmount,
          });
          const failCb = await failCl.post("/api/v1/payments/paytr-callback", {
            form: {
              merchant_oid: failOid,
              status: "failed",
              total_amount: failAmount,
              hash: failHash,
            },
          });
          assert.assertStatus(failCb, 200, "Failed PayTR notification must still acknowledge 200 OK");
          assert.assertEqual(failCb.text.trim(), "OK");

          const exportAfterFail = await failCl.get("/api/v1/account/export");
          assert.assertStatus(exportAfterFail, 200);
          assert.assertEqual(
            exportAfterFail.json?.profile?.role,
            "registered",
            "Failed PayTR callback must NOT upgrade user role"
          );
          const failedInv = (exportAfterFail.json?.orders || []).find(
            (o) => o.invoice_no === failCo.json?.invoice_no
          );
          assert.assertNotEqual(
            failedInv?.payment_status,
            "paid",
            "Failed PayTR callback must NOT mark invoice paid"
          );

          // Verify PayTR multi-item cart-checkout callback activates digital SKU download entitlement
          const cartCl = new c.http.Client({ baseUrl: paytrServer.baseUrl });
          const cartEmail = `${uid("paytr.cart")}@example.com`;
          await cartCl.post("/api/v1/auth/register", {
            json: {
              email: cartEmail,
              full_name: "PayTR Cart Buyer",
              password: "E2ePassword!2026",
              kvkk_consent: true,
              kvkk_acknowledged: true,
              tos_accepted: true,
              website_url: "",
            },
          });
          const cartCo = await cartCl.post("/api/v1/commerce/cart-checkout", {
            json: {
              email: cartEmail,
              full_name: "PayTR Cart Buyer",
              distance_sales_consent: true,
              items: [
                { item_type: "product", item_code: "KITAP-YALTES-20YIL", qty: 1 },
                { item_type: "product", item_code: "RAPOR-SAVUNMA-KOBI-2026", qty: 1 },
              ],
            },
          });
          assert.assertStatus(cartCo, 200, "Configured PayTR cart-checkout must return 200");
          const cartOid = cartCo.json?.merchant_oid;
          const cartAmount = String(cartCo.json?.amount_kurus);
          const cartHash = computePaytrCallbackHash({
            merchantKey,
            merchantSalt,
            merchantOid: cartOid,
            status: "success",
            totalAmount: cartAmount,
          });
          const cartCb = await cartCl.post("/api/v1/payments/paytr-callback", {
            form: {
              merchant_oid: cartOid,
              status: "success",
              total_amount: cartAmount,
              hash: cartHash,
            },
          });
          assert.assertStatus(cartCb, 200);
          assert.assertEqual(cartCb.text.trim(), "OK");

          const cartDl = await cartCl.raw("GET", "/api/v1/commerce/download/RAPOR-SAVUNMA-KOBI-2026");
          assert.assertStatus(cartDl, 200, "PayTR cart-checkout callback must unlock digital SKU download");
          assert.assertEqual(cartDl.rawBody.subarray(0, 5).toString("ascii"), "%PDF-");

          // Also verify /api/v1/payments/paytr/callback alias handles idempotent callback cleanly
          const aliasCb = await cl.post("/api/v1/payments/paytr/callback", {
            form: {
              merchant_oid: merchantOid,
              status: "success",
              total_amount: totalAmount,
              hash: validHash,
            },
          });
          assert.assertStatus(aliasCb, 200, "Alias /api/v1/payments/paytr/callback must return 200 OK");
          assert.assertEqual(aliasCb.text.trim(), "OK");
        } finally {
          if (paytrServer) {
            await paytrServer.stop();
          }
          await new Promise((resolve) => paytrStub.close(resolve));
        }
      }
    );

    suite.test(
      "T3-46: KVKK consent audit + profile ETK toggle + GDPR/KVKK JSON export omits credential secrets",
      { req: ["REQ-032", "REQ-015"] },
      async (c) => {
        const reg = await c.auth.registerUser({
          email: `${uid("kvkk.exp")}@example.com`,
          full_name: "KVKK Veri Sahibi",
          organization_name: "E2E KVKK Kurumu",
          ssb_interests: ["kara-araclari"],
          email_consent: false,
        });

        const updRes = await reg.client.post("/api/v1/account/profile", {
          json: {
            full_name: "KVKK Güncel İsim",
            ssb_interests: ["kara-araclari", "hava-araclari"],
            email_consent: true,
          },
        });
        assert.assertStatus(updRes, 200);
        assert.assertEqual(updRes.json?.data?.user?.email_consent, true);

        const expRes = await reg.client.get("/api/v1/account/export");
        assert.assertStatus(expRes, 200, "KVKK data portability export must return 200 JSON");
        assert.assertEqual(expRes.json?.profile?.email, reg.email);
        assert.assertNotIncludes(
          expRes.text,
          "password_hash",
          "KVKK JSON export must never leak password_hash"
        );
        assert.assertNotIncludes(
          expRes.text,
          "totp_secret",
          "KVKK JSON export must never leak totp_secret"
        );
      }
    );

    suite.test(
      "T3-47: KVKK account erasure (/api/v1/account/delete) anonymizes PII, revokes session token, and blocks re-use",
      { req: ["REQ-032", "REQ-033"] },
      async (c) => {
        const reg = await c.auth.registerUser({
          email: `${uid("kvkk.del")}@example.com`,
          full_name: "Silinecek Kullanıcı",
        });

        // Capture session cookie before deletion to verify persistent server-side revocation
        const capturedSession = reg.client.getRawCookie("msi_session");
        assert.assertOk(Boolean(capturedSession), "Expected msi_session cookie after registration");

        // Wrong password must fail 401
        const badDel = await reg.client.post("/api/v1/account/delete", {
          json: { password: "WrongPassword!999" },
        });
        assert.assertStatus(badDel, 401, "Account deletion with wrong password must return 401");

        // Correct password deletes account
        const okDel = await reg.client.post("/api/v1/account/delete", {
          json: { password: reg.password },
        });
        assert.assertStatus(okDel, 200, "Valid KVKK account erasure must return 200");
        assert.assertEqual(okDel.json?.data?.deleted, true);

        // Replay captured session token on fresh client -> must fail 401
        const replayClient = c.http.createClient();
        replayClient.setCookie("msi_session", capturedSession);
        const replayExport = await replayClient.get("/api/v1/account/export");
        assert.assertStatus(
          replayExport,
          401,
          "Revoked session token after account deletion must be rejected with 401"
        );

        // Login with deleted account credentials must fail 401
        const relogin = await c.http.createClient().post("/api/v1/auth/login", {
          json: { email: reg.email, password: reg.password },
        });
        assert.assertStatus(relogin, 401, "Erased account must not be able to log in");
      }
    );

    suite.test(
      "T3-48: Logout persistently revokes msi_session token so replaying old cookie fails with 401",
      { req: ["REQ-017", "REQ-033"] },
      async (c) => {
        const staff = await c.auth.createUserWithRole("content_editor");
        const capturedToken = staff.client.getRawCookie("msi_session");
        assert.assertOk(Boolean(capturedToken), "Expected msi_session cookie after login");

        const logoutRes = await staff.client.post("/api/v1/auth/logout");
        assert.assertStatus(logoutRes, 200);

        const replayClient = c.http.createClient();
        replayClient.setCookie("msi_session", capturedToken);

        const adminMeReplay = await replayClient.get("/api/v1/admin/me");
        assert.assertStatus(
          adminMeReplay,
          401,
          "Replaying revoked session token on /api/v1/admin/me after logout must return 401"
        );

        const exportReplay = await replayClient.get("/api/v1/account/export");
        assert.assertStatus(
          exportReplay,
          401,
          "Replaying revoked session token on /api/v1/account/export after logout must return 401"
        );
      }
    );
  });
}
