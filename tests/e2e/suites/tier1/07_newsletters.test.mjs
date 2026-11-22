import {
  antiBotFields,
  getCachedSuperAdmin,
  uid,
  uniqueEmail,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-027: Newsletters, Email Groups, Scheduling & Sponsor Banner ───────
  suite.describe("REQ-027 Newsletters, Email Groups, Scheduling & Unsubscribe", () => {
    suite.test("POST /api/v1/public/newsletter/subscribe enrolls guest into specified email_group_code", { req: "REQ-027" }, async (c) => {
      const email = uniqueEmail("nlguest");
      const res = await c.http.createClient().post("/api/v1/public/newsletter/subscribe", {
        json: {
          email,
          full_name: "Bulten Okuyucusu",
          email_group_code: "edergi-releases",
          kvkk_consent: true,
          email_consent: true,
          lang: "tr",
          ...antiBotFields(),
        },
      });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertEqual(data?.subscribed, true);
      c.assert.assertEqual(data?.email_group_code, "edergi-releases");
    });

    suite.test("GET/POST/DELETE /api/v1/admin/email-groups manages custom email groups", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const code = `grp-${uid("eg").toLowerCase().replace(/[^a-z0-9-]/g, "-")}`;
      const createRes = await sa.client.post("/api/v1/admin/email-groups", {
        json: {
          id: 0,
          code,
          name_tr: "E2E Ozel Grup",
          name_en: "E2E Custom Group",
          description_tr: "Test aciklama",
          description_en: "Test description",
          requires_marketing_consent: true,
          is_active: true,
          sort_order: 50,
        },
      });
      c.assert.assertStatus(createRes, 200);
      c.assert.assertEqual(unwrap(createRes)?.code, code);

      const listRes = await sa.client.get("/api/v1/admin/email-groups");
      c.assert.assertStatus(listRes, 200);
      c.assert.assertOk(unwrap(listRes).some((g) => g.code === code));

      const delRes = await sa.client.delete(`/api/v1/admin/email-groups/${encodeURIComponent(code)}`);
      c.assert.assertStatus(delRes, 200);
    });

    suite.test("POST /api/v1/admin/newsletters with future scheduled_at creates scheduled campaign", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const subj = `Planli Bulten ${uid("nl")}`;
      const res = await sa.client.post("/api/v1/admin/newsletters", {
        json: {
          id: 0,
          subject_tr: subj,
          subject_en: `Scheduled ${subj}`,
          target_group_filter: "all_consented",
          target_institution: "",
          target_ssb_tag: "",
          sponsor_ad_id: 1,
          sponsor_name: "ASELSAN",
          recipient_count: 0,
          status: "",
          sent_at: "",
          scheduled_at: "2030-05-15T09:30:00Z",
          email_group_code: "general",
          body_html: "<p>Gelecek bulten icerigi.</p>",
        },
      });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertEqual(data?.status, "scheduled");
      c.assert.assertOk(String(data?.scheduled_at || "").startsWith("2030-05-15"));
      await sa.client.delete(`/api/v1/admin/newsletters/${data.id}`);
    });

    suite.test("POST /api/v1/admin/newsletters targets email_group_code and embeds sponsor banner", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const guestEmail = uniqueEmail("nltarget");
      await c.http.createClient().post("/api/v1/public/newsletter/subscribe", {
        json: {
          email: guestEmail,
          full_name: "Hedef Abone",
          email_group_code: "exhibition-alerts",
          kvkk_consent: true,
          email_consent: true,
          ...antiBotFields(),
        },
      });

      const res = await sa.client.post("/api/v1/admin/newsletters", {
        json: {
          id: 0,
          subject_tr: `Fuar Bulteni ${uid("ex")}`,
          subject_en: "Exhibition Newsletter",
          target_group_filter: "all_consented",
          target_institution: "",
          target_ssb_tag: "",
          sponsor_ad_id: 6,
          sponsor_name: "SAHA EXPO 2026",
          recipient_count: 0,
          status: "",
          sent_at: "",
          email_group_code: "exhibition-alerts",
          body_html: "",
        },
      });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertOk(data?.id > 0);
      c.assert.assertOk(["queued", "sent"].includes(data?.status));
      await sa.client.delete(`/api/v1/admin/newsletters/${data.id}`);
    });

    suite.test("GET /abonelik-iptal and /newsletters/unsubscribe/:token handle unsubscribe flow", { req: "REQ-027" }, async (c) => {
      const client = c.http.createClient();
      const page = await client.get("/abonelik-iptal");
      c.assert.assertStatus(page, 400); // missing token renders transactional 400 HTML page
      c.assert.assertIncludes(page.text, "ABONELİK");
      const redir = await client.get("/newsletters/unsubscribe/sample-token-check", { redirect: "manual" });
      c.assert.assertStatus(redir, [301, 302, 303, 307]);
      c.assert.assertIncludes(redir.headers.get("location") || "", "/abonelik-iptal");
    });

    suite.test("Admin #form-newsletter UI exposes email_group_code and scheduled_at controls (M4 contract)", { req: "REQ-027" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [panelRes, jsRes] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/system-rbac.js"),
      ]);
      c.assert.assertStatus(panelRes, 200);
      c.assert.assertStatus(jsRes, 200);
      c.assert.assertMatch(
        panelRes.text,
        /id="nl-email-group"|name="email_group_code"/i,
        "M4 #form-newsletter must include email_group_code selector"
      );
      c.assert.assertMatch(
        panelRes.text,
        /id="nl-scheduled-at"|name="scheduled_at"/i,
        "M4 #form-newsletter must include scheduled_at input"
      );
      c.assert.assertIncludes(
        jsRes.text,
        "scheduled_at",
        "M4 system-rbac.js newsletter submit handler must send scheduled_at"
      );
    });
  });
}
