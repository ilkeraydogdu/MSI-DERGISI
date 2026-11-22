import {
  antiBotFields,
  formTs,
  getCachedSuperAdmin,
  uid,
  uniqueEmail,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-027 Boundaries: Newsletters, Email Groups & Unsubscribe ───────────
  suite.describe("REQ-027 Newsletter & Email Group Boundaries", () => {
    suite.test("POST /api/v1/public/newsletter/subscribe rejects kvkk_consent=false with 400", { req: "REQ-027" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/public/newsletter/subscribe", {
        json: {
          email: uniqueEmail("nlbad"),
          kvkk_consent: false,
          email_consent: true,
          ...antiBotFields(),
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/public/newsletter/subscribe rejects invalid email format with 400", { req: "REQ-027" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/public/newsletter/subscribe", {
        json: {
          email: "not-an-email",
          kvkk_consent: true,
          email_consent: true,
          ...antiBotFields(),
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/public/newsletter/subscribe blocks fast _form_ts timing with 429", { req: "REQ-027" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/public/newsletter/subscribe", {
        json: {
          email: uniqueEmail("nlbot"),
          kvkk_consent: true,
          email_consent: true,
          website_url: "",
          _form_ts: formTs(0),
        },
      });
      c.assert.assertStatus(res, 429);
    });

    suite.test("POST /api/v1/email-preferences rejects missing or invalid token with 400", { req: "REQ-027" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/email-preferences", {
        json: {
          token: "invalid-token-999",
          newsletter_frequency: "weekly",
          categories: ["haberler"],
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("DELETE /api/v1/admin/newsletters/999999 returns 404", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.delete("/api/v1/admin/newsletters/999999");
      c.assert.assertStatus(res, 404);
    });

    suite.test("ads_manager is denied POST /api/v1/admin/newsletters and /api/v1/admin/email-groups (403)", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const r1 = await ads.client.post("/api/v1/admin/newsletters", {
        json: {
          id: 0,
          subject_tr: "Denied",
          subject_en: "Denied",
          target_group_filter: "all_consented",
          target_institution: "",
          target_ssb_tag: "",
          sponsor_ad_id: null,
          sponsor_name: "",
          recipient_count: 0,
          status: "",
          sent_at: "",
          body_html: "",
        },
      });
      const r2 = await ads.client.post("/api/v1/admin/email-groups", {
        json: {
          id: 0,
          code: `denied-${uid("g").toLowerCase()}`,
          name_tr: "Denied",
          name_en: "Denied",
          description_tr: "",
          description_en: "",
          requires_marketing_consent: true,
          is_active: true,
          sort_order: 1,
        },
      });
      c.assert.assertStatus(r1, 403);
      c.assert.assertStatus(r2, 403);
    });
  });
}
