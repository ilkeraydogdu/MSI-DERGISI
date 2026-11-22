import {
  checkInteractiveVisibilityAndLayout,
  createVerifiedSubscriber,
  makeMinimalPdfBuffer,
  makePdfPart,
  makeTinyPngPart,
  openDomainPage,
  uid,
  uniqueIssueNumber,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 3: E-Dergi PDF Pipeline, Composite PK, Converter & Flipbook", () => {
    suite.test(
      "T3-15: Admin PDF upload -> issue creation auto-detects page_count and serves /api/v1/admin/edergi-files/:name",
      { req: ["REQ-011", "REQ-019"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const issueNo = uniqueIssueNumber();
        const pdfPart = makePdfPart(`pipeline-${issueNo}.pdf`, makeMinimalPdfBuffer(5, `Pipeline ${issueNo}`));

        const uploadRes = await eic.client.post("/api/v1/admin/upload", {
          multipart: { file: pdfPart },
        });
        assert.assertStatus(uploadRes, 200, "Admin PDF upload must succeed");
        const filename = uploadRes.json.url.split("/").pop();

        try {
          const saveRes = await eic.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `pipeline-issue-${issueNo}`,
              title_tr: `E2E Sayı ${issueNo}`,
              title_en: `E2E Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 99,
              dossier_count: 3,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: "",
              publication_type: "msi",
              uploaded_pdf_file: filename,
              spreads: [],
            },
          });
          assert.assertStatus(saveRes, 200);
          assert.assertEqual(
            saveRes.json?.data?.issue?.page_count,
            5,
            "Issue creation must auto-detect 5 pages from uploaded PDF"
          );

          const previewRes = await eic.client.raw("GET", `/api/v1/admin/edergi-files/${filename}`);
          assert.assertStatus(previewRes, 200, "Admin PDF preview must return 200");
          assert.assertIncludes(previewRes.headers.get("content-type") || "", "application/pdf");
        } finally {
          await eic.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-16: Composite PK (publication_type, issue_number) supports msi & msi_tdr sharing issue_number and composite REST routes",
      { req: ["REQ-040"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const sharedIssueNo = uniqueIssueNumber();

        try {
          for (const pubType of ["msi", "msi_tdr"]) {
            const res = await eic.client.post("/api/v1/admin/issues", {
              json: {
                issue_number: sharedIssueNo,
                slug: `${pubType}-shared-${sharedIssueNo}`,
                title_tr: `${pubType.toUpperCase()} Sayı ${sharedIssueNo}`,
                title_en: `${pubType.toUpperCase()} Issue ${sharedIssueNo}`,
                publish_month_tr: "Kasım",
                publish_month_en: "November",
                year: 2026,
                cover_image: "images/msi/dergi-286.webp",
                is_special_issue: false,
                page_count: 4,
                dossier_count: 1,
                summary_tr: "Özet",
                summary_en: "Summary",
                pdf_path: `/api/v1/edergi/${sharedIssueNo}/download-pdf`,
                publication_type: pubType,
                spreads: [],
              },
            });
            assert.assertStatus(res, 200, `Creating ${pubType} issue #${sharedIssueNo} must succeed`);
          }

          // Composite GET & PUT & DELETE (M2 REQ-040)
          const getTdr = await eic.client.get(`/api/v1/admin/issues/msi_tdr/${sharedIssueNo}`);
          assert.assertStatus(
            getTdr,
            200,
            "GET /api/v1/admin/issues/:publication_type/:issue_number must return 200 (M2 REQ-040)"
          );

          const putTdr = await eic.client.put(`/api/v1/admin/issues/msi_tdr/${sharedIssueNo}`, {
            json: {
              issue_number: sharedIssueNo,
              slug: `msi_tdr-shared-${sharedIssueNo}`,
              title_tr: `Güncel MSI TDR ${sharedIssueNo}`,
              title_en: `Updated MSI TDR ${sharedIssueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 4,
              dossier_count: 2,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: `/api/v1/edergi/${sharedIssueNo}/download-pdf`,
              publication_type: "msi_tdr",
              spreads: [],
            },
          });
          assert.assertStatus(
            putTdr,
            200,
            "PUT /api/v1/admin/issues/:publication_type/:issue_number must return 200 (M2 REQ-040)"
          );

          const delTdr = await eic.client.delete(`/api/v1/admin/issues/msi_tdr/${sharedIssueNo}`);
          assert.assertStatus(
            delTdr,
            200,
            "DELETE /api/v1/admin/issues/:publication_type/:issue_number must delete only msi_tdr row (M2 REQ-040)"
          );

          const getMsiAfter = await eic.client.get(`/api/v1/admin/issues/msi/${sharedIssueNo}`);
          assert.assertStatus(
            getMsiAfter,
            200,
            "Deleting msi_tdr issue must leave msi issue with same issue_number intact"
          );
        } finally {
          await eic.client.delete(`/api/v1/admin/issues/msi/${sharedIssueNo}?force=true`).catch(() => {});
          await eic.client.delete(`/api/v1/admin/issues/msi_tdr/${sharedIssueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-17: One-click PDF->Web converter (POST /api/v1/admin/issues/:pub/:num/convert-pdf) extracts spreads & page_count",
      { req: ["REQ-019"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const issueNo = uniqueIssueNumber();
        const pdfPart = makePdfPart(`convert-${issueNo}.pdf`, makeMinimalPdfBuffer(4, `Convert ${issueNo}`));
        const upRes = await eic.client.post("/api/v1/admin/upload", {
          multipart: { file: pdfPart },
        });
        assert.assertStatus(upRes, 200);
        const filename = upRes.json.url.split("/").pop();

        try {
          const issRes = await eic.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `convert-issue-${issueNo}`,
              title_tr: `Dönüştürme Sayı ${issueNo}`,
              title_en: `Convert Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 1,
              dossier_count: 1,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: "",
              publication_type: "msi",
              uploaded_pdf_file: filename,
              spreads: [],
            },
          });
          assert.assertStatus(issRes, 200);

          const convertRes = await eic.client.post(
            `/api/v1/admin/issues/msi/${issueNo}/convert-pdf`,
            { json: {} }
          );
          assert.assertStatus(
            convertRes,
            200,
            "One-click convert-pdf endpoint must return 200 (M2 REQ-019)"
          );
          const convData = convertRes.json?.data || convertRes.json;
          assert.assertEqual(convData?.converted, true, "Response must include converted: true");
          assert.assertEqual(convData?.page_count, 4, "Converter must extract 4 pages");
          assert.assertOk(
            Array.isArray(convData?.spreads) && convData.spreads.length >= 1,
            "Converter must populate spreads array"
          );
        } finally {
          await eic.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-18: Reader flipbook page endpoint (GET /api/v1/edergi/:issue/pages/:page_num) enforces entitlement and streams watermarked page",
      { req: ["REQ-018", "REQ-013"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const issueNo = uniqueIssueNumber();
        const pdfPart = makePdfPart(`flipbook-${issueNo}.pdf`, makeMinimalPdfBuffer(4, `Flipbook ${issueNo}`));
        const upRes = await op.client.post("/api/v1/admin/upload", {
          multipart: { file: pdfPart },
        });
        assert.assertStatus(upRes, 200);
        const filename = upRes.json.url.split("/").pop();

        try {
          await op.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `flipbook-issue-${issueNo}`,
              title_tr: `Flipbook Sayı ${issueNo}`,
              title_en: `Flipbook Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 4,
              dossier_count: 1,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: `/api/v1/edergi/${issueNo}/download-pdf`,
              publication_type: "msi",
              uploaded_pdf_file: filename,
              spreads: [],
            },
          });

          const anon = c.http.createClient();
          const anonPage = await anon.raw("GET", `/api/v1/edergi/${issueNo}/pages/1`);
          assert.assertStatus(
            anonPage,
            401,
            "Anonymous flipbook page request must return 401 (M2 REQ-018)"
          );

          const regUser = await createVerifiedSubscriber(c, "registered");
          const regPage = await regUser.client.raw("GET", `/api/v1/edergi/${issueNo}/pages/1`);
          assert.assertStatus(
            regPage,
            403,
            "Non-subscriber registered user flipbook page request must return 403 (M2 REQ-018)"
          );

          const sub = await createVerifiedSubscriber(c, "subscriber_digital");
          const subPage = await sub.client.raw("GET", `/api/v1/edergi/${issueNo}/pages/1`);
          assert.assertStatus(
            subPage,
            200,
            "Verified subscriber must receive 200 on flipbook page API (M2 REQ-018)"
          );
          const ct = subPage.headers.get("content-type") || "";
          assert.assertOk(
            ct.includes("image/") || ct.includes("application/pdf"),
            `Flipbook page response must be image/* or application/pdf, got ${ct}`
          );
        } finally {
          await op.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-19: #issue-dialog on /e-dergi wires interactive flipbook spread controls & page-range download selector UI",
      { req: ["REQ-018", "REQ-020"] },
      async (c) => {
        const sub = await createVerifiedSubscriber(c, "subscriber_digital");
        const pageRes = await openDomainPage(c, "/e-dergi", {
          client: sub.client,
          viewport: c.browser.VIEWPORT_DESKTOP,
        });
        try {
          assert.assertEqual(pageRes.status, 200);
          await pageRes.page.locator("[data-open-issue]").first().click();
          await pageRes.page.locator("#issue-dialog").waitFor({ state: "visible", timeout: 10000 });

          // Assert #issue-tab-flipbook is visible and switch directly to Flipbook view
          await checkInteractiveVisibilityAndLayout(c, pageRes.page, "#issue-tab-flipbook", {
            label: "T3-19 #issue-tab-flipbook",
          });
          await pageRes.page.locator("#issue-tab-flipbook").click();
          await pageRes.page.waitForTimeout(200);

          const uiState = await pageRes.page.evaluate(() => {
            const dlg = document.querySelector("#issue-dialog");
            if (!dlg) return { exists: false };
            const pageRangeInput = dlg.querySelector(
              '#edergi-pages-input, input[name="edergi_export_mode"], input[name="pages"], select[name="pages"], [data-edergi-pages]'
            );
            const flipbookEl = dlg.querySelector(
              "#issue-flipbook-panel, .edergi-flipbook-panel, .edergi-flipbook, [data-edergi-flipbook], #issue-flipbook-stage"
            );
            return {
              exists: true,
              hasPageRangeUi: Boolean(pageRangeInput),
              hasFlipbookUi: Boolean(flipbookEl),
            };
          });
          assert.assertOk(uiState.exists, "#issue-dialog must open");
          assert.assertOk(
            uiState.hasPageRangeUi,
            "#issue-dialog must provide a page-range selector UI wired to ?pages= (M2 REQ-020)"
          );
          assert.assertOk(
            uiState.hasFlipbookUi,
            "#issue-dialog must provide an interactive flipbook page-spread viewer (M2 REQ-018)"
          );
          assert.assertNoBrowserErrors(pageRes);
        } finally {
          await pageRes.close();
        }
      }
    );

    suite.test(
      "T3-20: Media library rejects spoofed signatures & unsafe SVG, accepts valid PNG, and updates metadata",
      { req: ["REQ-011"] },
      async (c) => {
        const editor = await c.auth.createUserWithRole("content_editor");

        // Spoofed PNG header
        const badPng = await editor.client.post("/api/v1/admin/upload", {
          multipart: {
            file: {
              content: Buffer.from("NOT_A_PNG_FILE_PAYLOAD"),
              filename: "spoof.png",
              contentType: "image/png",
            },
          },
        });
        assert.assertStatus(badPng, 400, "Spoofed PNG magic bytes must be rejected with 400");

        // Unsafe SVG with onload
        const unsafeSvg = await editor.client.post("/api/v1/admin/upload", {
          multipart: {
            file: {
              content: Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"></svg>'),
              filename: "xss.svg",
              contentType: "image/svg+xml",
            },
          },
        });
        assert.assertStatus(unsafeSvg, 400, "SVG with inline event handler must be rejected with 400");

        // Valid PNG upload
        const pngName = `${uid("valid-img")}.png`;
        const okUp = await editor.client.post("/api/v1/admin/upload", {
          multipart: { file: makeTinyPngPart(pngName) },
        });
        assert.assertStatus(okUp, 200, "Valid PNG upload must return 200");

        const mediaList = await editor.client.get("/api/v1/admin/media-library", {
          query: { q: pngName.replace(".png", "") },
        });
        assert.assertStatus(mediaList, 200);
        const item = (mediaList.json?.data || []).find((m) =>
          (m.original_name || m.filename || "").includes(pngName.replace(".png", ""))
        );
        assert.assertOk(item, "Uploaded PNG must be registered in media_library");

        const updRes = await editor.client.put(`/api/v1/admin/media-library/${item.id}`, {
          json: {
            alt_text_tr: "E2E Test Görseli",
            alt_text_en: "E2E Test Image",
            tags: "kaan,havacilik",
          },
        });
        assert.assertStatus(updRes, 200, "Updating media metadata must return 200");
      }
    );

    suite.test(
      "T3-21: Issue delete returns 409 when referenced by contents and detaches cleanly with ?force=true",
      { req: ["REQ-040", "REQ-007"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const issueNo = uniqueIssueNumber();

        let contentId = null;
        try {
          await eic.client.post("/api/v1/admin/issues", {
            json: {
              issue_number: issueNo,
              slug: `ref-issue-${issueNo}`,
              title_tr: `Referanslı Sayı ${issueNo}`,
              title_en: `Referenced Issue ${issueNo}`,
              publish_month_tr: "Kasım",
              publish_month_en: "November",
              year: 2026,
              cover_image: "images/msi/dergi-286.webp",
              is_special_issue: false,
              page_count: 4,
              dossier_count: 1,
              summary_tr: "Özet",
              summary_en: "Summary",
              pdf_path: `/api/v1/edergi/${issueNo}/download-pdf`,
              publication_type: "msi",
              spreads: [],
            },
          });

          const contentRes = await eic.client.post("/api/v1/admin/contents", {
            json: {
              id: 0,
              slug: uid("t3-ref-content"),
              title_tr: "Sayıya Bağlı Makale",
              title_en: "Linked Issue Article",
              summary_tr: "Özet",
              summary_en: "Summary",
              body_tr: "<p>Gövde metni.</p>",
              body_en: "<p>Body text.</p>",
              content_type: "dergi-icerikleri",
              source_group: "dergi-icerigi",
              cover_image: "images/msi/kaan.webp",
              author_name: "Editör",
              author_title: "Yazar",
              issue_number: issueNo,
              published_at: "2026-11-20 10:00:00",
              read_time_min: 4,
              visibility_level: 0,
              single_price_try: 0,
              is_sponsored: false,
              is_featured: false,
              view_count: 0,
              true_read_count: 0,
              placements: ["dergi-icerikleri"],
              ssb_tags: ["hava-araclari"],
              status: "published",
              en_status: "approved",
            },
          });
          assert.assertStatus(contentRes, 200);
          contentId = contentRes.json?.data?.id ?? null;

          const conflictDel = await eic.client.delete(`/api/v1/admin/issues/${issueNo}`);
          assert.assertStatus(
            conflictDel,
            409,
            "Deleting issue referenced by content without ?force=true must return 409"
          );

          const forceDel = await eic.client.delete(`/api/v1/admin/issues/${issueNo}`, {
            query: { force: "true" },
          });
          assert.assertStatus(forceDel, 200, "Deleting issue with ?force=true must succeed");
          assert.assertEqual(forceDel.json?.data?.detached_contents, 1);
        } finally {
          await eic.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=true`).catch(() => {});
          if (contentId) {
            await eic.client.delete(`/api/v1/admin/contents/${contentId}`).catch(() => {});
          }
        }
      }
    );

    suite.test(
      "T3-54: Exhaustive catalog-to-storage sweep verifies every DB issue PDF & page stream across msi and msi_tdr with zero 404s",
      { req: ["REQ-018", "REQ-020", "REQ-040"] },
      async (c) => {
        const sub = await createVerifiedSubscriber(c, "subscriber_corporate", {
          organization_category: "MSB",
          subscription_tier: "kurumsal",
        });
        const op = await c.auth.loginAsAdmin();

        const [pubIssuesRes, adminIssuesRes] = await Promise.all([
          c.http.createClient().get("/api/v1/issues"),
          op.client.get("/api/v1/admin/issues", { query: { publication_type: "all" } }),
        ]);
        assert.assertStatus(pubIssuesRes, 200);
        assert.assertStatus(adminIssuesRes, 200);

        const pubIssues = Array.isArray(pubIssuesRes.json) ? pubIssuesRes.json : [];
        const adminIssues = Array.isArray(adminIssuesRes.json?.data) ? adminIssuesRes.json.data : [];
        assert.assertOk(pubIssues.length >= 1, "Public /api/v1/issues must not be empty");
        assert.assertEqual(
          pubIssues.length,
          adminIssues.length,
          "Public and admin issue counts must align"
        );

        for (const iss of adminIssues) {
          const pubType = iss.publication_type || "msi";
          const num = iss.issue_number;

          if (iss.uploaded_pdf_file) {
            const rawAdmin = await op.client.raw(
              "GET",
              `/api/v1/admin/edergi-files/${encodeURIComponent(iss.uploaded_pdf_file)}`
            );
            assert.assertStatus(
              rawAdmin,
              200,
              `Admin edergi file ${iss.uploaded_pdf_file} for ${pubType}#${num} must exist (200)`
            );
          }

          const p1 = await sub.client.raw("GET", `/api/v1/edergi/${num}/pages/1`, {
            query: { publication_type: pubType },
          });
          assert.assertStatus(p1, 200, `Flipbook page 1 for ${pubType}#${num} must return 200`);

          const dl = await sub.client.raw("GET", `/api/v1/edergi/${num}/download-pdf`, {
            query: { publication_type: pubType, pages: "1" },
          });
          assert.assertStatus(dl, 200, `Watermarked single-page PDF for ${pubType}#${num} must return 200`);
          assert.assertEqual(dl.rawBody.subarray(0, 5).toString("ascii"), "%PDF-");
          assert.assertOk(Boolean(dl.headers.get("x-msi-drm-watermark")));
        }
      }
    );
  });
}
