import {
  createAdminAd,
  createAdminContent,
  createVerifiedSubscriber,
  makeMinimalPdfBuffer,
  makePdfPart,
  uid,
  uniqueIssueNumber,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 2: Gov Auto-Verify, E-Dergi DRM, Analytics ssb_tag & k>=3 B2B", () => {
    suite.test(
      "T3-08: .gov.tr / .tsk.tr auto-verify assigns subscriber_public, enforces email_verified on PDF, and grants verified DRM PDF",
      { req: ["REQ-016", "REQ-020"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const issueNo = uniqueIssueNumber();
        const pdfPart = makePdfPart(`gov-issue-${issueNo}.pdf`, makeMinimalPdfBuffer(4, "Gov Test"));
        const upRes = await op.client.post("/api/v1/admin/upload", {
          multipart: { file: pdfPart },
        });
        assert.assertStatus(upRes, 200);
        const uploadedName = upRes.json.url.split("/").pop();

        try {
          const issRes = await op.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `gov-test-issue-${issueNo}`,
              title_tr: `E2E Sayı ${issueNo}`,
              title_en: `E2E Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 4,
              dossier_count: 2,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: `/api/v1/edergi/${issueNo}/download-pdf`,
              publication_type: "msi",
              uploaded_pdf_file: uploadedName,
              spreads: [],
            },
          });
          assert.assertStatus(issRes, 200);

          // .tsk.tr signup auto-assigns subscriber_public, but email_verified=false blocks PDF until verified
          const tskReg = await c.auth.registerUser({
            email: `${uid("tsk")}@kkk.tsk.tr`,
            full_name: "Bnb. E2E Test",
            organization_category: "Kamu / TSK / SSB",
            organization_name: "Kara Kuvvetleri Komutanlığı",
          });
          assert.assertEqual(tskReg.govAutoSubscribed, true, ".tsk.tr must auto-subscribe");
          assert.assertEqual(tskReg.user?.role, "subscriber_public");
          assert.assertEqual(tskReg.user?.is_gov_verified, true);
          assert.assertEqual(tskReg.user?.email_verified, false);

          const unverifiedDl = await tskReg.client.raw(
            "GET",
            `/api/v1/edergi/${issueNo}/download-pdf`,
            { query: { pages: "1-2" } }
          );
          assert.assertStatus(
            unverifiedDl,
            403,
            "Unverified .tsk.tr user must be blocked from E-Dergi PDF download with 403"
          );

          // Verified subscriber_public downloads watermarked PDF
          const verifiedGov = await createVerifiedSubscriber(c, "subscriber_public", {
            email: `${uid("verified.gov")}@msb.gov.tr`,
            organization_category: "Kamu / TSK / SSB",
            organization_name: "Millî Savunma Bakanlığı",
            is_gov_verified: true,
          });
          const dlRes = await verifiedGov.client.raw(
            "GET",
            `/api/v1/edergi/${issueNo}/download-pdf`,
            { query: { pages: "1-2" } }
          );
          assert.assertStatus(dlRes, 200, "Verified subscriber_public must download E-Dergi PDF");
          assert.assertIncludes(
            dlRes.headers.get("content-type") || "",
            "application/pdf",
            "E-Dergi download must return application/pdf"
          );
          const watermark = dlRes.headers.get("x-msi-drm-watermark");
          assert.assertOk(
            watermark && watermark.length >= 8,
            `Expected non-empty X-MSI-DRM-Watermark header, got ${watermark}`
          );
          assert.assertOk(
            dlRes.rawBody.subarray(0, 5).toString("ascii") === "%PDF-",
            "Downloaded E-Dergi slice must start with %PDF-"
          );
        } finally {
          await op.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-09: External corporate domain queues Tab 4 request -> support_agent approves -> upgrades to subscriber_corporate",
      { req: ["REQ-016", "REQ-017"] },
      async (c) => {
        const corpEmail = `${uid("corp.req")}@savunma-vendor.com`;
        const corpReg = await c.auth.registerUser({
          email: corpEmail,
          full_name: "E2E Kurumsal Başvuru",
          organization_category: "Sektör Kuruluşları",
          organization_name: "Test Savunma Sanayii A.Ş.",
          request_message: "Kurumsal dijital abonelik onayı talep ediyorum.",
        });
        assert.assertEqual(corpReg.govAutoSubscribed, false);
        assert.assertEqual(
          corpReg.corporateVerificationQueued,
          true,
          "Corporate domain with request_message must queue Tab 4 verification request"
        );

        const support = await c.auth.createUserWithRole("support_agent");
        const reqsRes = await support.client.get("/api/v1/admin/corporate-requests");
        assert.assertStatus(reqsRes, 200);
        const pendingReq = (Array.isArray(reqsRes.json) ? reqsRes.json : []).find(
          (r) => r.email === corpEmail && r.status === "pending"
        );
        assert.assertOk(pendingReq, "Queued corporate verification request must appear in Tab 4 API");

        await c.auth.approveCorporateRequest(pendingReq.id, "approved", support.client);

        // Verify user role upgraded via /hesabim or re-login
        const reLoginClient = c.http.createClient();
        const loginRes = await c.auth.login(reLoginClient, {
          email: corpEmail,
          password: corpReg.password,
        });
        assert.assertEqual(
          loginRes.user?.role,
          "subscriber_corporate",
          "Approved corporate request must upgrade user to subscriber_corporate"
        );
      }
    );

    suite.test(
      "T3-10: Partial E-Dergi page-range download (?pages=1-2 vs ?pages=all) slices PDF cleanly with unique DRM watermark",
      { req: ["REQ-020"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const issueNo = uniqueIssueNumber();
        const pdfBuf = makeMinimalPdfBuffer(6, `Slice Issue ${issueNo}`);
        const upRes = await op.client.post("/api/v1/admin/upload", {
          multipart: { file: makePdfPart(`slice-${issueNo}.pdf`, pdfBuf) },
        });
        assert.assertStatus(upRes, 200);
        const filename = upRes.json.url.split("/").pop();

        try {
          const issRes = await op.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `slice-issue-${issueNo}`,
              title_tr: `Kesit Sayı ${issueNo}`,
              title_en: `Slice Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 6,
              dossier_count: 1,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: `/api/v1/edergi/${issueNo}/download-pdf`,
              publication_type: "msi",
              uploaded_pdf_file: filename,
              spreads: [],
            },
          });
          assert.assertStatus(issRes, 200);

          const sub = await createVerifiedSubscriber(c, "subscriber_digital");
          const fullRes = await sub.client.raw("GET", `/api/v1/edergi/${issueNo}/download-pdf`, {
            query: { pages: "all" },
          });
          const partialRes = await sub.client.raw("GET", `/api/v1/edergi/${issueNo}/download-pdf`, {
            query: { pages: "1-2" },
          });

          assert.assertStatus(fullRes, 200);
          assert.assertStatus(partialRes, 200);
          assert.assertOk(
            partialRes.rawBody.subarray(0, 5).toString("ascii") === "%PDF-",
            "Partial PDF must be valid %PDF-"
          );
          assert.assertOk(
            partialRes.rawBody.length < fullRes.rawBody.length,
            `Partial 2-page PDF (${partialRes.rawBody.length}B) should be smaller than 6-page full PDF (${fullRes.rawBody.length}B)`
          );
          assert.assertOk(
            Boolean(partialRes.headers.get("x-msi-drm-watermark")),
            "Partial PDF must carry X-MSI-DRM-Watermark header"
          );
        } finally {
          await op.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-11: Analytics telemetry records ssb_tag, true_read, video_completed and populates B2B sponsor report",
      { req: ["REQ-024", "REQ-026"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const article = await createAdminContent(c, op.client, {
          slug: uid("t3-telemetry-article"),
          ssb_tags: ["radarlar-ve-sensorler"],
        });
        const ad = await createAdminAd(c, op.client, {
          sponsor_name: uid("TelemetrySponsor"),
          target_ssb_tags: ["radarlar-ve-sensorler"],
          force_override: true,
        });

        const reader = await createVerifiedSubscriber(c, "subscriber_digital", {
          organization_category: "Sektör Kuruluşları",
          organization_name: "Havelsan E2E Test",
        });

        const readTrack = await reader.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: uid("sess"),
            event_type: "article_read",
            content_slug: article.slug,
            ssb_tag: "radarlar-ve-sensorler",
            duration_sec: 32,
            scroll_depth_pct: 85,
            video_watch_pct: 100,
            is_true_read: false,
            video_completed: false,
            organization_category: "",
            organization_name: "",
            created_at: "",
          },
        });
        assert.assertStatus(readTrack, 200);
        assert.assertEqual(
          readTrack.json?.is_true_read,
          true,
          "duration>=25 & scroll>=70 must mark is_true_read=true"
        );

        const adClickTrack = await reader.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: uid("sess"),
            event_type: "ad_click",
            ad_id: ad.id,
            ssb_tag: "radarlar-ve-sensorler",
            duration_sec: 5,
            scroll_depth_pct: 50,
            video_watch_pct: 0,
            is_true_read: false,
            video_completed: false,
            organization_category: "",
            organization_name: "",
            created_at: "",
          },
        });
        assert.assertStatus(adClickTrack, 200);

        const reportRes = await op.client.get("/api/v1/admin/analytics/sponsor-report");
        assert.assertStatus(reportRes, 200, "B2B sponsor-report must return 200 for staff");
        const contentRep = (reportRes.json?.contents || []).find(
          (r) => (r.content_slug || r.slug) === article.slug
        );
        assert.assertOk(contentRep, "Created article must appear in B2B sponsor report");
        assert.assertOk(
          contentRep.true_reads >= 1 && contentRep.total_views >= 1,
          `Article report must reflect true_reads >= 1 and total_views >= 1 (got true_reads=${contentRep.true_reads}, total_views=${contentRep.total_views})`
        );
        assert.assertEqual(
          readTrack.json?.video_watch_pct,
          100,
          "Analytics track response must confirm clamped video_watch_pct === 100"
        );

        // Verify persisted analytics_events record via /api/v1/admin/export/open-data has video_completed === true
        const openDataRes = await op.client.get("/api/v1/admin/export/open-data");
        assert.assertStatus(openDataRes, 200, "Open-data export must succeed for site_operator");
        const recordedEv = (openDataRes.json?.analytics_events || []).find(
          (ev) => ev.id === readTrack.json?.event_id || ev.content_slug === article.slug
        );
        assert.assertOk(recordedEv, "Recorded telemetry event must appear in analytics_events export");
        assert.assertEqual(
          recordedEv.video_completed,
          true,
          "video_watch_pct=100 (>=95) must persist video_completed=true"
        );
        assert.assertEqual(
          recordedEv.is_true_read,
          true,
          "duration>=25 & scroll>=70 must persist is_true_read=true"
        );
        assert.assertEqual(
          recordedEv.ssb_tag,
          "radarlar-ve-sensorler",
          "Persisted analytics event must retain ssb_tag"
        );

        const adRep = (reportRes.json?.ads || []).find((a) => a.ad_id === ad.id || a.id === ad.id);
        assert.assertOk(adRep, "Created ad must appear in B2B sponsor report");
        assert.assertOk(
          (adRep.clicks ?? adRep.total_clicks ?? 0) >= 1,
          "Ad report must reflect recorded ad_click telemetry"
        );
      }
    );

    suite.test(
      "T3-12: k>=3 anonymization (?anonymize=true) suppresses PII & small cohorts (<3 actors) across reader & B2B reports",
      { req: ["REQ-025", "REQ-026"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const rareOrgCat = uid("NadirKurumKat");
        const rareReader = await createVerifiedSubscriber(c, "subscriber_digital", {
          organization_category: rareOrgCat,
          organization_name: "Tek Kişilik Gizli Birim",
        });

        await rareReader.client.post("/api/v1/analytics/track", {
          json: {
            id: 0,
            session_id: uid("rare-sess"),
            event_type: "article_read",
            content_slug: "nonexistent-or-any-slug",
            ssb_tag: "hava-araclari",
            duration_sec: 40,
            scroll_depth_pct: 90,
            video_watch_pct: 0,
            is_true_read: false,
            video_completed: false,
            organization_category: "",
            organization_name: "",
            created_at: "",
          },
        });

        const anonReaders = await op.client.get("/api/v1/admin/analytics/readers", {
          query: { anonymize: "true" },
        });
        assert.assertStatus(anonReaders, 200);
        const serializedReaders = JSON.stringify(anonReaders.json || []);
        assert.assertNotIncludes(
          serializedReaders,
          rareReader.email,
          "Anonymized reader analytics must not expose raw user email"
        );
        assert.assertNotIncludes(
          serializedReaders,
          "Tek Kişilik Gizli Birim",
          "Anonymized reader analytics must not expose organization_name PII"
        );

        const subReport = await op.client.get("/api/v1/admin/reports/subscribers", {
          query: { anonymize: "true" },
        });
        assert.assertStatus(subReport, 200);
        const instBreakdown = subReport.json?.data?.institution_breakdown || [];
        assert.assertOk(
          !instBreakdown.some((b) => b.organization_category === rareOrgCat),
          `Institution with <3 users (${rareOrgCat}) must be suppressed under k>=3 anonymization`
        );
      }
    );

    suite.test(
      "T3-13: Moved admin endpoints live exclusively under /api/v1/admin/* with strict RBAC and legacy paths return 404",
      { req: ["REQ-026", "REQ-033"] },
      async (c) => {
        const reader = await createVerifiedSubscriber(c, "registered");
        const anon = c.http.createClient();

        // Legacy non-admin paths must be removed (404)
        for (const legacyCall of [
          () => anon.post("/api/v1/ads/check-conflict", { json: {} }),
          () => anon.get("/api/v1/analytics/sponsor-report"),
          () => anon.post("/api/v1/subscriptions/trigger-reminders", { json: {} }),
        ]) {
          const r = await legacyCall();
          assert.assertStatus(r, 404, "Legacy unprefixed admin endpoint must return 404");
        }

        // Canonical /api/v1/admin/* endpoints must enforce 401 anon & 403 reader
        for (const call of [
          (cl) => cl.post("/api/v1/admin/ads/check-conflict", { json: { slot_code: "sidebar_300x250", sponsor_name: "X" } }),
          (cl) => cl.get("/api/v1/admin/analytics/sponsor-report"),
          (cl) => cl.post("/api/v1/admin/subscriptions/trigger-reminders", { json: {} }),
        ]) {
          const anonRes = await call(anon);
          assert.assertStatus(anonRes, 401, "Anonymous request to moved admin endpoint must return 401");
          const readerRes = await call(reader.client);
          assert.assertStatus(readerRes, 403, "Reader request to moved admin endpoint must return 403");
        }
      }
    );

    suite.test(
      "T3-14: Article SSR (reader.rs) exposes non-empty data-ssb-tag attribute matching content ssb_tags",
      { req: ["REQ-024", "REQ-012"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const tag = "elektronik-harp-sistemleri";
        const art = await createAdminContent(c, op.client, {
          slug: uid("t3-ssr-ssbtag"),
          ssb_tags: [tag],
        });

        const anon = c.http.createClient();
        const htmlRes = await anon.get(`/icerik/${art.slug}`);
        assert.assertStatus(htmlRes, 200);
        assert.assertMatch(
          htmlRes.text,
          new RegExp(`data-ssb-tag=["']${tag}["']`),
          `Article SSR must emit data-ssb-tag="${tag}" so msi-ui.js populates ssb_tag telemetry (M3 REQ-024)`
        );
      }
    );
  });
}
