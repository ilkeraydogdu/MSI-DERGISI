import {
  antiBotFields,
  getCachedSuperAdmin,
  makeContentPayload,
  makeIssuePayload,
  uniqueEmail,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-018 Boundaries: Flipbook /pages/:page_num ─────────────────────────
  suite.describe("REQ-018 Flipbook & Page Endpoint Boundaries", () => {
    suite.test("unauthenticated GET /api/v1/edergi/286/pages/1 returns 401 (M2 contract)", { req: "REQ-018" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/edergi/286/pages/1");
      c.assert.assertStatus(res, 401, "Unauthenticated flipbook page request must return 401");
    });

    suite.test("registered non-subscriber GET /api/v1/edergi/286/pages/1 returns 403 (M2 contract)", { req: "REQ-018" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const reg = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const res = await reg.client.get("/api/v1/edergi/286/pages/1");
      c.assert.assertStatus(res, 403, "Non-subscriber flipbook page request must return 403");
    });

    suite.test("out-of-range page_num=9999 on /api/v1/edergi/286/pages/9999 returns 400 or 404 (M2 contract)", { req: "REQ-018" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get("/api/v1/edergi/286/pages/9999");
      c.assert.assertStatus(res, [400, 404]);
    });

    suite.test("non-existent issue on /api/v1/edergi/99999/pages/1 returns 404 (M2 contract)", { req: "REQ-018" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get("/api/v1/edergi/99999/pages/1");
      c.assert.assertStatus(res, 404);
    });

    suite.test("unauthenticated browser Accept:text/html on /edergi/286/download-pdf redirects to /hesabim", { req: "REQ-018" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/edergi/286/download-pdf", {
        headers: { Accept: "text/html" },
        redirect: "manual",
      });
      c.assert.assertStatus(res, [302, 303, 307]);
      c.assert.assertIncludes(res.headers.get("location") || "", "/hesabim");
    });
  });

  // ── REQ-019 Boundaries: PDF Converter & Issue Validation ──────────────────
  suite.describe("REQ-019 Issue & PDF Converter Boundaries", () => {
    suite.test("POST /api/v1/admin/issues rejects issue_number<=0, year<1990, or empty title_tr (400)", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const r1 = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({ issue_number: 0, slug: "bad-0", title_tr: "Bad" }),
      });
      const r2 = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({ issue_number: 999, slug: "bad-year", title_tr: "Bad Year", year: 1850 }),
      });
      c.assert.assertStatus(r1, 400);
      c.assert.assertStatus(r2, 400);
    });

    suite.test("GET /api/v1/admin/edergi-files/not-a-pdf.txt rejects non-PDF with 400", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/edergi-files/not-a-pdf.txt");
      c.assert.assertStatus(res, 400);
    });

    suite.test("GET /api/v1/admin/edergi-files/%2e%2e%2f%2e%2e%2f.env path traversal blocked with 403", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/edergi-files/%2e%2e%2f%2e%2e%2f.env");
      c.assert.assertStatus(res, 403);
    });

    suite.test("POST /api/v1/admin/issues/msi/99999/convert-pdf returns 404 for missing issue (M2 contract)", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/issues/msi/99999/convert-pdf", { json: {} });
      c.assert.assertStatus(res, 404);
    });

    suite.test("content_editor is denied POST /api/v1/admin/issues/msi/286/convert-pdf with 403 (M2 contract)", { req: "REQ-019" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.post("/api/v1/admin/issues/msi/286/convert-pdf", { json: {} });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-020 Boundaries: Watermarked PDF Export ────────────────────────────
  suite.describe("REQ-020 Watermarked PDF Export Boundaries", () => {
    suite.test("unauthenticated GET /api/v1/edergi/286/download-pdf returns 401 JSON", { req: "REQ-020" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/edergi/286/download-pdf", {
        headers: { Accept: "application/json" },
      });
      c.assert.assertStatus(res, 401);
    });

    suite.test("registered non-subscriber gets 403 SUBSCRIBER_ACCESS_REQUIRED", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const reg = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const res = await reg.client.get("/api/v1/edergi/286/download-pdf", {
        headers: { Accept: "application/json" },
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("unverified email subscriber_public (.gov.tr before email verify) gets 403 email_verified=false", { req: "REQ-020" }, async (c) => {
      const gov = await c.auth.registerUser({
        email: uniqueEmail("unvergov", "msb.gov.tr"),
        organization_category: "MSB",
        ...antiBotFields(),
      });
      c.assert.assertEqual(gov.user?.role, "subscriber_public");
      const res = await gov.client.get("/api/v1/edergi/286/download-pdf", {
        headers: { Accept: "application/json" },
      });
      c.assert.assertStatus(res, 403);
      c.assert.assertEqual(res.json?.data?.email_verified, false);
    });

    suite.test("GET /api/v1/edergi/99999/download-pdf returns 404 for nonexistent issue", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get("/api/v1/edergi/99999/download-pdf", {
        headers: { Accept: "application/json" },
      });
      c.assert.assertStatus(res, 404);
    });

    suite.test("malformed ?pages=abc-xyz returns 400 BAD_REQUEST on download-pdf endpoint", { req: "REQ-020" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get("/api/v1/edergi/286/download-pdf", {
        query: { pages: "abc-xyz,99999" },
        headers: { Accept: "application/json" },
      });
      c.assert.assertStatus(res, 400);
      c.assert.assertEqual(res.json?.error_code, "BAD_REQUEST");
    });
  });

  // ── REQ-040 Boundaries: Composite PK (publication_type, issue_number) ─────
  suite.describe("REQ-040 Composite PK Boundaries", () => {
    suite.test("GET /api/v1/admin/issues/msi/99999 returns 404 (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/issues/msi/99999");
      c.assert.assertStatus(res, 404);
    });

    suite.test("PUT /api/v1/admin/issues/invalid_pub/286 rejects unknown publication_type (400/404, M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.put("/api/v1/admin/issues/invalid_pub/286", {
        json: makeIssuePayload({ issue_number: 286, publication_type: "invalid_pub", title_tr: "Bad" }),
      });
      c.assert.assertStatus(res, [400, 404]);
    });

    suite.test("DELETE /api/v1/admin/issues/msi_tdr/99999 returns 404 (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.delete("/api/v1/admin/issues/msi_tdr/99999");
      c.assert.assertStatus(res, 404);
    });

    suite.test("legacy DELETE /api/v1/admin/issues/:issue_number returns 409 when msi and msi_tdr share issue_number (M2 contract)", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const issueNo = 908;
      for (const pubType of ["msi", "msi_tdr"]) {
        const sv = await sa.client.post("/api/v1/admin/issues", {
          json: makeIssuePayload({ issue_number: issueNo, publication_type: pubType, page_count: 10 }),
        });
        c.assert.assertStatus(sv, 200);
      }

      try {
        const ambigDel = await sa.client.delete(`/api/v1/admin/issues/${issueNo}`);
        c.assert.assertStatus(ambigDel, 409, "Ambiguous DELETE without publication_type must return 409 Conflict");
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/msi/${issueNo}?force=1`);
        await sa.client.delete(`/api/v1/admin/issues/msi_tdr/${issueNo}?force=1`);
      }
    });

    suite.test("DELETE /api/v1/admin/issues/:issue_number returns 409 when referenced by content without ?force=1", { req: "REQ-040" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const issueNo = 909;
      const sv = await sa.client.post("/api/v1/admin/issues", {
        json: makeIssuePayload({ issue_number: issueNo, publication_type: "msi", title_tr: `Ref Issue ${issueNo}` }),
      });
      c.assert.assertStatus(sv, 200);
      const contentRes = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug: uniqueSlug("t2-issue-ref"),
          title_tr: "Sayiya Bagli Haber",
          issue_number: issueNo,
        }),
      });
      c.assert.assertStatus(contentRes, 200);
      const contentId = unwrap(contentRes)?.id;
      try {
        const del = await sa.client.delete(`/api/v1/admin/issues/${issueNo}`);
        c.assert.assertStatus(del, 409);
      } finally {
        await sa.client.delete(`/api/v1/admin/issues/${issueNo}?force=1`);
        if (contentId) await sa.client.delete(`/api/v1/admin/contents/${contentId}`);
      }
    });
  });
}
