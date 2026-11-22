import {
  checkInteractiveVisibilityAndLayout,
  createAdminAd,
  createVerifiedSubscriber,
  openDomainPage,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 4: Newsletters, Email Groups, Scheduling & Sponsor Banners", () => {
    suite.test(
      "T3-22: Email group CRUD + public newsletter subscription populates group while honoring ETK opt-out",
      { req: ["REQ-027", "REQ-032"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const groupCode = uid("t3-group").toLowerCase();

        const grpRes = await eic.client.post("/api/v1/admin/email-groups", {
          json: {
            id: 0,
            code: groupCode,
            name_tr: "E2E Deniz Platformları Grubu",
            name_en: "E2E Naval Platforms Group",
            description_tr: "Test grubu",
            description_en: "Test group",
            filter_json: "{}",
            static_emails_json: "[]",
            requires_marketing_consent: true,
            is_system: false,
            is_active: true,
            sort_order: 20,
          },
        });
        assert.assertStatus(grpRes, 200, "Creating custom email group must return 200");

        const anon = c.http.createClient();
        const optInEmail = `${uid("nl.optin")}@example.com`;
        const optInRes = await anon.post("/api/v1/public/newsletter/subscribe", {
          json: {
            email: optInEmail,
            full_name: "Bülten Okuru",
            email_group_code: groupCode,
            kvkk_consent: true,
            email_consent: true,
            website_url: "",
          },
        });
        assert.assertStatus(optInRes, 200);
        assert.assertEqual(optInRes.json?.data?.subscribed, true);
        assert.assertEqual(optInRes.json?.data?.email_group_code, groupCode);

        // KVKK consent=true with ETK email_consent=false must NOT subscribe
        const optOutRes = await anon.post("/api/v1/public/newsletter/subscribe", {
          json: {
            email: `${uid("nl.optout")}@example.com`,
            full_name: "KVKK Only Okur",
            email_group_code: groupCode,
            kvkk_consent: true,
            email_consent: false,
            website_url: "",
          },
        });
        assert.assertStatus(optOutRes, 200);
        assert.assertEqual(
          optOutRes.json?.data?.subscribed,
          false,
          "email_consent=false must keep subscribed=false"
        );

        const listRes = await eic.client.get("/api/v1/admin/email-groups");
        assert.assertStatus(listRes, 200);
        const found = (listRes.json?.data || []).find((g) => g.code === groupCode);
        assert.assertOk(found, "Created email group must be listed in /api/v1/admin/email-groups");
        assert.assertIncludes(
          found.static_emails_json || "",
          optInEmail,
          "Opted-in public subscriber email must be stored in email group"
        );
      }
    );

    suite.test(
      "T3-23: Newsletter campaign with email_group_code + future scheduled_at stores status=scheduled and recipient_count",
      { req: ["REQ-027"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const groupCode = uid("t3-sched-grp").toLowerCase();
        const memberEmail = `${uid("sched.member")}@example.com`;

        await eic.client.post("/api/v1/admin/email-groups", {
          json: {
            id: 0,
            code: groupCode,
            name_tr: "Zamanlanmış Bülten Grubu",
            name_en: "Scheduled Group",
            description_tr: "",
            description_en: "",
            filter_json: "{}",
            static_emails_json: JSON.stringify([memberEmail]),
            requires_marketing_consent: true,
            is_system: false,
            is_active: true,
            sort_order: 25,
          },
        });

        const futureDate = new Date(Date.now() + 72 * 3600 * 1000).toISOString();
        const subj = `Zamanlı Bülten ${uid("nl")}`;
        const nlRes = await eic.client.post("/api/v1/admin/newsletters", {
          json: {
            id: 0,
            subject_tr: subj,
            subject_en: "Scheduled Briefing",
            target_group_filter: "all",
            target_institution: "",
            target_ssb_tag: "",
            sponsor_ad_id: null,
            sponsor_name: null,
            recipient_count: 0,
            status: "draft",
            sent_at: "",
            scheduled_at: futureDate,
            email_group_code: groupCode,
            body_html: "<p>Gelecek tarihli bülten içeriği.</p>",
          },
        });
        assert.assertStatus(nlRes, 200);
        assert.assertEqual(
          nlRes.json?.data?.status,
          "scheduled",
          "Future scheduled_at must record campaign with status=scheduled"
        );
        assert.assertOk(
          nlRes.json?.data?.recipient_count >= 1,
          "Scheduled campaign targeting email group must resolve recipient_count >= 1"
        );

        const listNl = await eic.client.get("/api/v1/admin/newsletters", {
          query: { per_page: "50" },
        });
        assert.assertStatus(listNl, 200);
        const saved = (listNl.json?.data || []).find((n) => n.subject_tr === subj);
        assert.assertOk(saved, "Scheduled newsletter must appear in /api/v1/admin/newsletters");
        assert.assertEqual(saved.email_group_code, groupCode);
        assert.assertOk(Boolean(saved.scheduled_at), "scheduled_at must persist");
      }
    );

    suite.test(
      "T3-24: Immediate newsletter campaign resolves sponsor_ad_id + opted-in SSB recipients and queues dispatch",
      { req: ["REQ-027", "REQ-021"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const ssbTag = "simulatorler-ve-egitim-sistemleri";

        const sponsorAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("NewsletterSponsorAd"),
          is_newsletter_ad: true,
          target_ssb_tags: [ssbTag],
          force_override: true,
        });

        await createVerifiedSubscriber(c, "subscriber_digital", {
          ssb_interests: [ssbTag],
          email_consent: true,
        });

        // Verify status="draft" with scheduled_at=null saves as a draft without dispatching
        const draftRes = await op.client.post("/api/v1/admin/newsletters", {
          json: {
            id: 0,
            subject_tr: `Taslak Bülten ${uid("nl")}`,
            subject_en: "Draft Newsletter",
            target_group_filter: "all",
            target_institution: "",
            target_ssb_tag: ssbTag,
            sponsor_ad_id: sponsorAd.id,
            sponsor_name: sponsorAd.sponsorName,
            recipient_count: 0,
            status: "draft",
            sent_at: "",
            scheduled_at: null,
            email_group_code: null,
            body_html: "<p>Taslak bülten içeriği.</p>",
          },
        });
        assert.assertStatus(draftRes, 200);
        assert.assertEqual(
          draftRes.json?.data?.status,
          "draft",
          "status='draft' with scheduled_at=null must save as draft without dispatching"
        );
        assert.assertEqual(
          draftRes.json?.data?.recipient_count,
          0,
          "Draft newsletter must keep recipient_count === 0"
        );
        assert.assertEqual(
          draftRes.json?.data?.dispatch,
          undefined,
          "Draft newsletter must not trigger email dispatch"
        );

        const nlRes = await op.client.post("/api/v1/admin/newsletters", {
          json: {
            id: 0,
            subject_tr: `Anlık Sponsorlu Bülten ${uid("nl")}`,
            subject_en: "Immediate Sponsored Newsletter",
            target_group_filter: "all",
            target_institution: "",
            target_ssb_tag: ssbTag,
            sponsor_ad_id: sponsorAd.id,
            sponsor_name: sponsorAd.sponsorName,
            recipient_count: 0,
            status: "sent",
            sent_at: "",
            scheduled_at: null,
            email_group_code: null,
            body_html: "<p>Simülatör teknolojileri özel bülteni.</p>",
          },
        });
        assert.assertStatus(nlRes, 200);
        assert.assertEqual(
          nlRes.json?.data?.status,
          "queued",
          "With MAIL_DELIVERY_ENABLED=false, immediate newsletter dispatch must report status=queued"
        );
        assert.assertOk(
          nlRes.json?.data?.recipient_count >= 1,
          "Campaign targeting opted-in SSB reader must have recipient_count >= 1"
        );
        assert.assertEqual(
          nlRes.json?.data?.dispatch?.delivery_mode,
          "validated_queue_ready",
          "Dispatch summary must confirm validated_queue_ready mode"
        );
      }
    );

    suite.test(
      "T3-25: RFC 8058 One-Click unsubscribe & /api/v1/email-preferences validate tokens cleanly and ETK opt-out persists without PII leakage",
      { req: ["REQ-027", "REQ-032"] },
      async (c) => {
        const anon = c.http.createClient();

        // Note: In email_service.rs (`dispatch_newsletter_emails`), valid RFC 8058 unsubscribe_tokens are
        // UUIDv4 tokens generated strictly inside the outgoing MIME email body/header (`email-only` — never
        // exposed via HTTP APIs or logs when MAIL_DELIVERY_ENABLED=false). We therefore test:
        // (1) Missing/invalid token rejection across GET/POST /abonelik-iptal & /api/v1/email-preferences without PII leakage;
        // (2) Opaque-box ETK opt-out via /api/v1/account/profile persisting across /api/v1/account/export and newsletter targeting.
        const missingTokenGet = await anon.get("/abonelik-iptal");
        assert.assertStatus(missingTokenGet, 400, "GET /abonelik-iptal without token must return 400");

        const invalidTokenGet = await anon.get("/abonelik-iptal", {
          query: { token: "nonexistent-rfc8058-token" },
        });
        assert.assertStatus(invalidTokenGet, 400, "GET /abonelik-iptal with invalid token must return 400");

        const invalidOneClick = await anon.post("/abonelik-iptal", {
          form: {
            "List-Unsubscribe": "One-Click",
            token: "nonexistent-rfc8058-token",
          },
        });
        assert.assertStatus(
          invalidOneClick,
          400,
          "POST /abonelik-iptal with invalid One-Click token must return 400"
        );
        const cardMatch = invalidOneClick.text.match(/<div class=["'][^"']*unsubscribe-card[^"']*["'][^>]*>([\s\S]*?)<\/div><\/section>/i);
        assert.assertOk(Boolean(cardMatch), "Expected .unsubscribe-card container in invalid /abonelik-iptal HTML");
        assert.assertNotIncludes(cardMatch[1], "@", "Invalid unsubscribe card must not leak any email address");

        const invalidPrefs = await anon.post("/api/v1/email-preferences", {
          json: {
            token: "nonexistent-prefs-token",
            newsletter_frequency: "monthly",
            categories: ["haberler"],
          },
        });
        assert.assertStatus(
          invalidPrefs,
          400,
          "POST /api/v1/email-preferences with invalid token must return 400"
        );

        // Verify ETK opt-out flow excludes opted-out reader from newsletter targeting
        const op = await c.auth.loginAsAdmin();
        const uniqueTag = uid("t3-unsub-tag");
        const reader = await createVerifiedSubscriber(c, "subscriber_digital", {
          ssb_interests: [uniqueTag],
          email_consent: true,
        });
        const optOutRes = await reader.client.post("/api/v1/account/profile", {
          json: {
            full_name: reader.user.full_name,
            email_consent: false,
            marketing_emails_opted_in: false,
          },
        });
        assert.assertStatus(optOutRes, 200);
        assert.assertEqual(optOutRes.json?.data?.user?.marketing_emails_opted_in, false);

        const expAfter = await reader.client.get("/api/v1/account/export");
        assert.assertStatus(expAfter, 200);
        assert.assertEqual(expAfter.json?.consent_records?.email_consent, false);
        assert.assertEqual(expAfter.json?.consent_records?.marketing_emails_opted_in, false);

        const nlAfterOptOut = await op.client.post("/api/v1/admin/newsletters", {
          json: {
            id: 0,
            subject_tr: `Opt-Out Kontrol ${uid("nl")}`,
            subject_en: "Opt-Out Verification",
            target_group_filter: "all",
            target_institution: "",
            target_ssb_tag: uniqueTag,
            recipient_count: 0,
            status: "sent",
            sent_at: "",
            scheduled_at: null,
            body_html: "<p>Test</p>",
          },
        });
        assert.assertStatus(nlAfterOptOut, 200);
        assert.assertOk(
          ["queued", "sent"].includes(nlAfterOptOut.json?.data?.status) &&
            Boolean(nlAfterOptOut.json?.data?.dispatch),
          "Opt-out verification campaign must go through immediate dispatch path (not draft)"
        );
        assert.assertEqual(
          nlAfterOptOut.json?.data?.recipient_count,
          0,
          "Opted-out subscriber must have recipient_count === 0 for their unique SSB tag"
        );
      }
    );

    suite.test(
      "T3-26: Admin #tab-6 #form-newsletter wires email_group_code & scheduled_at UI inputs (M4)",
      { req: ["REQ-027", "REQ-037"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const pageRes = await openDomainPage(c, `/${c.adminPath}`, {
          client: sa.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);
          assert.assertNoBrowserErrors(pageRes);
          await pageRes.page.waitForFunction(
            () => Boolean(window.MSIAdmin && window.MSIAdmin._me),
            { timeout: 15000 }
          );
          await pageRes.page.locator('#tab-btn-6, [data-tab="tab-6"]').first().click();
          await pageRes.page.waitForTimeout(150);
          await checkInteractiveVisibilityAndLayout(
            c,
            pageRes.page,
            '#tab-6 [data-subtab="tab6-campaigns"]',
            { label: "#tab-6 tab6-campaigns subtab" }
          );
          await pageRes.page.locator('#tab-6 [data-subtab="tab6-campaigns"]').first().click();
          await pageRes.page.locator("#form-newsletter").waitFor({ state: "attached", timeout: 10000 });

          const fields = await pageRes.page.evaluate(() => {
            const form = document.querySelector("#form-newsletter");
            if (!form) return { exists: false };
            const emailGroupInput = form.querySelector(
              '[name="email_group_code"], #nl-email-group, #nl-email-group-code'
            );
            const scheduledInput = form.querySelector(
              '[name="scheduled_at"], #nl-scheduled-at, input[type="datetime-local"]'
            );
            return {
              exists: true,
              hasEmailGroupInput: Boolean(emailGroupInput),
              hasScheduledInput: Boolean(scheduledInput),
            };
          });
          assert.assertOk(fields.exists, "#form-newsletter must exist in #tab-6");
          assert.assertOk(
            fields.hasEmailGroupInput,
            "#form-newsletter must provide email_group_code input (M4 REQ-027)"
          );
          assert.assertOk(
            fields.hasScheduledInput,
            "#form-newsletter must provide scheduled_at input (M4 REQ-027)"
          );
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-27: Newsletter & email-group RBAC allows editor_in_chief and denies ads_manager / finance_viewer with 403",
      { req: ["REQ-027", "REQ-033"] },
      async (c) => {
        const adsMgr = await c.auth.createUserWithRole("ads_manager");
        const finViewer = await c.auth.createUserWithRole("finance_viewer");

        for (const actor of [adsMgr, finViewer]) {
          const nlDeny = await actor.client.post("/api/v1/admin/newsletters", {
            json: {
              id: 0,
              subject_tr: "Yetkisiz Bülten",
              subject_en: "",
              target_group_filter: "all",
              target_institution: "",
              target_ssb_tag: "",
              recipient_count: 0,
              status: "draft",
              sent_at: "",
              body_html: "",
            },
          });
          assert.assertStatus(
            nlDeny,
            403,
            `Role ${actor.role} must be denied POST /api/v1/admin/newsletters`
          );
          const grpDeny = await actor.client.get("/api/v1/admin/email-groups");
          assert.assertStatus(
            grpDeny,
            403,
            `Role ${actor.role} must be denied GET /api/v1/admin/email-groups`
          );
        }
      }
    );
  });
}
