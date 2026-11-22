import {
  buildMinimalPdf,
  getCachedSuperAdmin,
  makeIssuePayload,
  uid,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-018: Browser-based interactive E-Dergi flipbook ───────────────────
  suite.describe("REQ-018 Interactive E-Dergi Flipbook", () => {
    suite.test("GET /e-dergi renders issue kiosk with seeded issues", { req: "REQ-018" }, async (c) => {
      const res = await c.http.createClient().get("/e-dergi");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "286");
      c.assert.assertIncludes(res.text, "issue-dialog");
    });

    suite.test("GET /api/v1/issues returns structured MagazineIssue catalog", { req: "REQ-018" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/issues");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json) && res.json.length >= 6);
      c.assert.assertOk(res.json.some((i) => i.issue_number === 286));
    });

    suite.test("GET /api/v1/edergi/:issue/pages/:page_num streams single-page render for flipbook (M2 contract)", { req: "REQ-018" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const res = await sub.client.raw("GET", "/api/v1/edergi/286/pages/1", {
        query: { publication_type: "msi" },
      });
      c.assert.assertStatus(res, 200, "M2 flipbook single-page endpoint /api/v1/edergi/:issue/pages/:page_num must return 200");
      const ct = (res.headers.get("content-type") || "").toLowerCase();
      c.assert.assertMatch(ct, /(image\/|application\/pdf|image\/svg\+xml)/, "Page render must be image, SVG, or PDF page");
      c.assert.assertOk(res.rawBody.byteLength > 50, "Rendered page payload must be non-empty");
    });

    suite.test("#issue-dialog includes interactive flipbook spread container beyond raw PDF iframe (M2 contract)", { req: "REQ-018" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const res = await sub.client.get("/e-dergi");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(
        res.text,
        /data-flipbook|issue-flipbook|edergi-flipbook/i,
        "M2 requires dedicated interactive flipbook reader markup in #issue-dialog"
      );
    });

    suite.test("flipbook assets/JS implement spread page navigation via /pages/:page_num (M2 contract)", { req: "REQ-018" }, async (c) => {
      const jsRes = await c.http.createClient().get("/js/msi-ui.js");
      c.assert.assertStatus(jsRes, 200);
      c.assert.assertIncludes(
        jsRes.text,
        "/pages/",
        "M2 msi-ui.js flipbook must fetch /api/v1/edergi/:issue/pages/:page_num"
      );
    });
  });

  // ── REQ-019: One-click Print-PDF -> Web E-Dergi conversion pipeline ───────
  suite.describe("REQ-019 Print-PDF to Web E-Dergi Converter", () => {
    suite.test("uploading multi-page PDF and saving issue auto-detects exact page_count", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const pdfBuf = buildMinimalPdf(5, "MSI Sayi 901");
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: pdfBuf, filename: `msi-901-${uid("p")}.pdf`, contentType: "application/pdf" },
        },
      });
      c.assert.assertStatus(up, 200);
      const uploadedFilename = String(up.json?.url || "").split("/").pop();
      c.assert.assertOk(uploadedFilename?.endsWith(".pdf"));

      const issueNo = 901;
      const save = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({
          issue_number: issueNo,
          publication_type: "msi",
          title_tr: "MSI E2E Sayi 901",
          title_en: "MSI E2E Issue 901",
          uploaded_pdf_file: uploadedFilename,
          page_count: 1,
        }),
      });
      c.assert.assertStatus(save, 200);
      c.assert.assertEqual(unwrap(save)?.issue?.page_count, 5, "Server must extract real PDF page_count=5");
      await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
    });

    suite.test("POST /api/v1/admin/issues/:publication_type/:issue_number/convert-pdf extracts spreads (M2 contract)", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const pdfBuf = buildMinimalPdf(4, "MSI Converter 902");
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: pdfBuf, filename: `msi-902-${uid("c")}.pdf`, contentType: "application/pdf" },
        },
      });
      c.assert.assertStatus(up, 200);
      const uploadedFilename = String(up.json?.url || "").split("/").pop();

      const issueNo = 902;
      const sv = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({
          issue_number: issueNo,
          publication_type: "msi",
          title_tr: "Converter Test 902",
          uploaded_pdf_file: uploadedFilename,
          page_count: 1,
        }),
      });
      c.assert.assertStatus(sv, 200);

      const conv = await sa.client.post(`/api/v1/admin/issues/msi/${issueNo}/convert-pdf`, { json: {} });
      try {
        c.assert.assertStatus(conv, 200, "M2 convert-pdf endpoint must return 200");
        const data = unwrap(conv);
        c.assert.assertEqual(data?.converted, true);
        c.assert.assertEqual(data?.page_count, 4);
        c.assert.assertOk(Array.isArray(data?.spreads) && data.spreads.length >= 1, "Expected extracted spreads array");
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
      }
    });

    suite.test("GET /api/v1/admin/edergi-files/:name serves uploaded issue PDF to staff", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const pdfBuf = buildMinimalPdf(2, "Admin Preview");
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: pdfBuf, filename: `preview-${uid("p")}.pdf`, contentType: "application/pdf" },
        },
      });
      c.assert.assertStatus(up, 200);
      const fname = String(up.json?.url || "").split("/").pop();
      const preview = await sa.client.raw("GET", `/api/v1/admin/edergi-files/${fname}`);
      c.assert.assertStatus(preview, 200);
      c.assert.assertEqual(preview.rawBody.subarray(0, 5).toString("ascii"), "%PDF-");
    });

    suite.test("Admin Tab 18 UI exposes one-click PDF->Web converter action (M2/M4 contract)", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [panelRes, issuesJs] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/issues.js"),
      ]);
      c.assert.assertStatus(panelRes, 200);
      c.assert.assertStatus(issuesJs, 200);
      c.assert.assertIncludes(
        issuesJs.text,
        "convert-pdf",
        "M2 issues.js must invoke /convert-pdf one-click converter"
      );
    });

    suite.test("GET /api/v1/admin/issues?publication_type=msi_tdr filters by publication_type", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/issues", {
        query: { publication_type: "msi_tdr" },
      });
      c.assert.assertStatus(res, 200);
      const list = unwrap(res);
      c.assert.assertOk(Array.isArray(list));
      for (const item of list) {
        c.assert.assertEqual(item.publication_type, "msi_tdr");
      }
    });
  });

  // ── REQ-020: Partial/Full watermarked PDF export & UI ─────────────────────
  suite.describe("REQ-020 Partial & Full Watermarked PDF Export", () => {
    suite.test("subscriber downloads full watermarked PDF with X-Msi-Drm-Watermark header", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const res = await sub.client.raw("GET", "/api/v1/edergi/286/download-pdf", {
        query: { pages: "all" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.rawBody.subarray(0, 5).toString("ascii"), "%PDF-");
      c.assert.assertOk(Boolean(res.headers.get("x-msi-drm-watermark")), "Missing X-Msi-Drm-Watermark header");
    });

    suite.test("subscriber downloads partial page range ?pages=1-2 as smaller valid PDF", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const pdfBuf = buildMinimalPdf(6, "Slice Test 903");
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: pdfBuf, filename: `slice-903-${uid("s")}.pdf`, contentType: "application/pdf" },
        },
      });
      const uploadedFilename = String(up.json?.url || "").split("/").pop();
      const issueNo = 903;
      const sv = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({
          issue_number: issueNo,
          publication_type: "msi",
          title_tr: "Slice Test 903",
          uploaded_pdf_file: uploadedFilename,
          page_count: 6,
        }),
      });
      c.assert.assertStatus(sv, 200);

      try {
        const sub = await c.auth.createUserWithRole("subscriber_corporate", {
          superAdminClient: sa.client,
          organization_category: "MSB",
          subscription_tier: "kurumsal",
        });
        const fullRes = await sub.client.raw("GET", `/api/v1/edergi/${issueNo}/download-pdf`, {
          query: { pages: "all" },
        });
        const partRes = await sub.client.raw("GET", `/api/v1/edergi/${issueNo}/download-pdf`, {
          query: { pages: "1-2" },
        });
        c.assert.assertStatus(fullRes, 200);
        c.assert.assertStatus(partRes, 200);
        c.assert.assertEqual(partRes.rawBody.subarray(0, 5).toString("ascii"), "%PDF-");
        c.assert.assertOk(
          partRes.rawBody.byteLength < fullRes.rawBody.byteLength,
          "Partial 1-2 slice should be smaller than full 6-page PDF"
        );
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
      }
    });

    suite.test("verified gov subscriber watermark header reflects subscriber identity", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const gov = await c.auth.createUserWithRole("subscriber_public", {
        superAdminClient: sa.client,
        organization_category: "MSB",
        is_gov_verified: true,
      });
      const res = await gov.client.raw("GET", "/api/v1/edergi/286/pdf", { query: { pages: "1" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Boolean(res.headers.get("x-msi-drm-watermark")));
    });

    suite.test("#issue-dialog exposes partial page-range selector UI (M2 contract)", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const res = await sub.client.get("/e-dergi");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, 'id="issue-dialog"', "#issue-dialog must be rendered on /e-dergi");
      c.assert.assertMatch(
        res.text,
        /(id="edergi-pages-input"|name="edergi_export_mode")/i,
        "M2 #issue-dialog must expose #edergi-pages-input / edergi_export_mode page-range selector UI"
      );
      c.assert.assertIncludes(res.text, 'id="edergi-pages-input"', "M2 #issue-dialog must include #edergi-pages-input");
      c.assert.assertIncludes(res.text, 'name="edergi_export_mode"', "M2 #issue-dialog must include edergi_export_mode radio group");
    });

    suite.test("msi-ui.js supports partial page-range download instead of hardcoding pages=all only (M2 contract)", { req: "REQ-020" }, async (c) => {
      const res = await c.http.createClient().get("/js/msi-ui.js");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(
        res.text,
        /(issue-pages-range|issue-page-range|pagesRange|download-pdf\?pages=)/i,
        "M2 msi-ui.js must wire partial page range input to download-pdf?pages="
      );
    });

    suite.test("every seeded issue in GET /api/v1/issues (msi & msi_tdr) streams pages 1-2 and downloads watermarked PDF with zero 404s", { req: ["REQ-018", "REQ-020", "REQ-040"] }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const issuesRes = await c.http.createClient().get("/api/v1/issues");
      c.assert.assertStatus(issuesRes, 200);
      const issues = Array.isArray(issuesRes.json) ? issuesRes.json : [];
      c.assert.assertOk(issues.length >= 1, "Expected seeded issues in /api/v1/issues");

      for (const iss of issues) {
        const num = iss.issue_number;
        const pubType = iss.publication_type || "msi";
        const p1 = await sub.client.raw("GET", `/api/v1/edergi/${num}/pages/1`, {
          query: { publication_type: pubType },
        });
        c.assert.assertStatus(p1, 200, `Issue ${pubType}#${num} page 1 must return 200 (no 404)`);
        c.assert.assertOk(p1.rawBody.byteLength > 50, `Issue ${pubType}#${num} page 1 must be non-empty`);

        if ((iss.page_count || 0) >= 2) {
          const p2 = await sub.client.raw("GET", `/api/v1/edergi/${num}/pages/2`, {
            query: { publication_type: pubType },
          });
          c.assert.assertStatus(p2, 200, `Issue ${pubType}#${num} page 2 must return 200 (no 404)`);
        }

        const pdf = await sub.client.raw("GET", `/api/v1/edergi/${num}/download-pdf`, {
          query: { publication_type: pubType, pages: "all" },
        });
        c.assert.assertStatus(pdf, 200, `Issue ${pubType}#${num} download-pdf must return 200 (no 404)`);
        c.assert.assertEqual(
          pdf.rawBody.subarray(0, 5).toString("ascii"),
          "%PDF-",
          `Issue ${pubType}#${num} download-pdf must start with %PDF-`
        );
        c.assert.assertOk(
          Boolean(pdf.headers.get("x-msi-drm-watermark")),
          `Issue ${pubType}#${num} download-pdf must include X-MSI-DRM-Watermark`
        );
      }
    });
  });

  // ── REQ-040: Composite PK (publication_type, issue_number) ────────────────
  suite.describe("REQ-040 Composite PK (publication_type, issue_number)", () => {
    suite.test("admin can create both msi and msi_tdr issues with the same issue_number", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const issueNo = 905;
      try {
        const r1 = await sa.client.post("/api/v1/admin/issues", {
          json: makeIssuePayload({ issue_number: issueNo, publication_type: "msi", page_count: 32 }),
        });
        const r2 = await sa.client.post("/api/v1/admin/issues", {
          json: makeIssuePayload({ issue_number: issueNo, publication_type: "msi_tdr", page_count: 48 }),
        });
        c.assert.assertStatus(r1, 200);
        c.assert.assertStatus(r2, 200);

        const list = await sa.client.get("/api/v1/admin/issues", { query: { publication_type: "all" } });
        const matching = unwrap(list).filter((i) => i.issue_number === issueNo);
        c.assert.assertEqual(matching.length, 2, "Both msi and msi_tdr issues with same number must coexist");
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=1`);
        await sa.client.delete(`/api/v1/admin/issues/msi_tdr/${issueNo}?force=1`);
      }
    });

    suite.test("GET /api/v1/admin/issues/:publication_type/:issue_number fetches specific edition (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/issues/msi/286");
      c.assert.assertStatus(res, 200, "M2 composite GET /api/v1/admin/issues/:publication_type/:issue_number must return 200");
      c.assert.assertEqual(unwrap(res)?.issue_number, 286);
      c.assert.assertEqual(unwrap(res)?.publication_type, "msi");
    });

    suite.test("PUT /api/v1/admin/issues/:publication_type/:issue_number updates targeted edition (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const issueNo = 906;
      await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({ issue_number: issueNo, publication_type: "msi_tdr", page_count: 20 }),
      });
      const upd = await sa.client.put(`/api/v1/admin/issues/msi_tdr/${issueNo}`, {
        json: makeIssuePayload({
          issue_number: issueNo,
          publication_type: "msi_tdr",
          title_tr: "TDR 906 Updated",
          page_count: 24,
        }),
      });
      try {
        c.assert.assertStatus(upd, 200, "M2 composite PUT must return 200");
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
      }
    });

    suite.test("DELETE /api/v1/admin/issues/:publication_type/:issue_number deletes only targeted edition (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const issueNo = 907;
      for (const pubType of ["msi", "msi_tdr"]) {
        await sa.client.post("/api/v1/admin/issues", {
          json: makeIssuePayload({ issue_number: issueNo, publication_type: pubType, page_count: 16 }),
        });
      }

      const delTdr = await sa.client.delete(`/api/v1/admin/issues/msi_tdr/${issueNo}`);
      try {
        c.assert.assertStatus(delTdr, 200, "M2 composite DELETE must delete only msi_tdr");
        const list = await sa.client.get("/api/v1/admin/issues", { query: { publication_type: "all" } });
        const rem = unwrap(list).filter((i) => i.issue_number === issueNo);
        c.assert.assertEqual(rem.length, 1);
        c.assert.assertEqual(rem[0].publication_type, "msi");
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
      }
    });

    suite.test("Admin issues.js uses composite publication_type/issue_number route (M2/M4 contract)", { req: "REQ-040" }, async (c) => {
      const res = await c.http.createClient().get("/js/admin/issues.js");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(
        res.text,
        /\/api\/v1\/admin\/issues\/\$\{[^}]*\b(?:pt|pubType|publication_type)\b[^}]*\}\/\$\{[^}]*\b(?:num|issueNum|issue_number)\b[^}]*\}/i,
        "M2/M4 issues.js must call /api/v1/admin/issues/:publication_type/:issue_number"
      );
    });
  });
}
