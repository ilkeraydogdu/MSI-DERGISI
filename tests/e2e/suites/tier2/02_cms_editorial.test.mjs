import {
  getCachedSuperAdmin,
  getSeededContentSlug,
  makeContentPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-007 Boundaries ────────────────────────────────────────────────────
  suite.describe("REQ-007 SSOT Multi-Placement Boundaries", () => {
    suite.test("draft content returns 404 to anonymous visitor on /api/v1/contents/:slug", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-draft");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: `Taslak ${slug}`, status: "draft" }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 404);
    });

    suite.test("content_editor cannot directly publish (status=published returns 403)", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug: uniqueSlug("t2-ed-pub"), title_tr: "Yetkisiz Yayin", status: "published" }),
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("POST /api/v1/admin/contents/bulk rejects invalid action with 400", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/contents/bulk", {
        json: { ids: [1], action: "nuke_all" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("content_editor is denied bulk publish action with 403", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.post("/api/v1/admin/contents/bulk", {
        json: { ids: [1], action: "publish" },
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("non-existent category /kategori/no-such-category renders 404 page without 500", { req: "REQ-007" }, async (c) => {
      const res = await c.http.createClient().get(`/kategori/nonexistent-${uid("cat").toLowerCase()}`);
      c.assert.assertStatus(res, 404);
      c.assert.assertIncludes(res.text, "not-found-card");
    });
  });

  // ── REQ-008 Boundaries: Categories & Custom Field Values ──────────────────
  suite.describe("REQ-008 Categories & Custom Field Boundaries", () => {
    suite.test("POST /api/v1/admin/categories rejects empty name_tr or invalid slug with 400", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/categories", {
        json: {
          id: 0,
          slug: "!!invalid slug!!",
          name_tr: "",
          name_en: "",
          section_group: "icerikler",
          parent_slug: null,
          description_tr: "",
          description_en: "",
          sort_order: 1,
          is_active: true,
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/admin/custom-fields rejects empty field_key with 400", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/custom-fields", {
        json: {
          field_key: "   ",
          label_tr: "Bos Anahtar",
          label_en: "Empty Key",
          field_type: "text",
          applies_to: "content",
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("GET /api/v1/admin/custom-field-values rejects invalid entity_type or empty entity_id with 400", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const r1 = await sa.client.get("/api/v1/admin/custom-field-values", {
        query: { entity_type: "invoice", entity_id: "1" },
      });
      const r2 = await sa.client.get("/api/v1/admin/custom-field-values", {
        query: { entity_type: "content", entity_id: "" },
      });
      c.assert.assertStatus(r1, 400);
      c.assert.assertStatus(r2, 400);
    });

    suite.test("PUT /api/v1/admin/custom-field-values rejects field_id belonging to different entity_type (400)", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fieldKey = `cf_mismatch_${uid("m")}`;
      await sa.client.post("/api/v1/admin/custom-fields", {
        json: {
          field_key: fieldKey,
          label_tr: "Ad Only Field",
          label_en: "Ad Only Field",
          field_type: "text",
          applies_to: "ad",
        },
      });
      const list = await sa.client.get("/api/v1/admin/custom-fields", {
        query: { applies_to: "ad", per_page: "100" },
      });
      const adField = unwrap(list).find((f) => f.field_key === fieldKey);
      c.assert.assertOk(adField?.id > 0);

      try {
        const putBad = await sa.client.put("/api/v1/admin/custom-field-values", {
          json: {
            entity_type: "content",
            entity_id: "1",
            values: [{ field_id: adField.id, value_text: "wrong-type" }],
          },
        });
        c.assert.assertStatus(putBad, 400);
      } finally {
        await sa.client.delete(`/api/v1/admin/custom-fields/${encodeURIComponent(fieldKey)}`);
      }
    });

    suite.test("ads_manager is denied PUT /api/v1/admin/custom-field-values for entity_type=content (403)", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const res = await ads.client.put("/api/v1/admin/custom-field-values", {
        json: { entity_type: "content", entity_id: "1", values: [] },
      });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-009 Boundaries: Approvals & Versioning ────────────────────────────
  suite.describe("REQ-009 Approval & Version Boundaries", () => {
    suite.test("POST /api/v1/admin/contents/:id/versions/:version/restore rejects unknown version with 400", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/contents/1/versions/999999/restore");
      c.assert.assertStatus(res, 400);
    });

    suite.test("content_editor is denied GET /api/v1/admin/approvals/pending (403)", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.get("/api/v1/admin/approvals/pending");
      c.assert.assertStatus(res, 403);
    });

    suite.test("content_editor cannot self-approve submitted approval request (403)", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const draft = await ed.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug: uniqueSlug("t2-selfapp"), title_tr: "Self Approve Block", status: "draft" }),
      });
      c.assert.assertStatus(draft, 200);
      const cid = unwrap(draft)?.id;
      const sub = await ed.client.post(`/api/v1/admin/contents/${cid}/submit-review`, { json: {} });
      c.assert.assertStatus(sub, 200);
      const aid = unwrap(sub)?.approval_id;

      const selfApp = await ed.client.post(`/api/v1/admin/approvals/${aid}/approve`, { json: {} });
      c.assert.assertStatus(selfApp, 403);
    });

    suite.test("ads_manager is denied POST /api/v1/admin/contents/:id/submit-review (403)", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const res = await ads.client.post("/api/v1/admin/contents/1/submit-review", { json: {} });
      c.assert.assertStatus(res, 403);
    });

    suite.test("finance_viewer is denied GET /api/v1/admin/contents/1/versions (403)", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fin = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
      const res = await fin.client.get("/api/v1/admin/contents/1/versions");
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-010 Boundaries: Sponsored Content Labeling ────────────────────────
  suite.describe("REQ-010 Sponsored Content Legal Boundaries", () => {
    suite.test("sponsored content with empty sponsor_legal_label renders fallback legal disclosure", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-spon-def");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Varsayilan Etiket ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "FNSS",
          sponsor_legal_label: "",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /(SPONSORLU|ADVERTORIAL|İLANDIR)/i);
    });

    suite.test("XSS payload inside sponsor_brand is escaped in SSR article page", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-spon-xss");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsor XSS ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: `<script>alert("xss")</script>BRAND`,
          sponsor_legal_label: "SPONSORLU İÇERİK",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, '<script>alert("xss")</script>');
    });

    suite.test("sponsored content is excluded from /api/v1/public/personalized", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-spon-pers");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsorlu Feed Disi ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "BAYKAR",
          ssb_tags: ["hava-araclari"],
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/public/personalized", {
        query: { interests: "hava-araclari", limit: "24" },
      });
      const slugs = (unwrap(res)?.items || []).map((i) => i.slug);
      c.assert.assertNotIncludes(slugs, slug);
    });

    suite.test("GET /api/v1/contents?is_sponsored=true returns only sponsored records", { req: "REQ-010" }, async (c) => {
      const res = await c.http.createClient().get("/api/v1/contents", {
        query: { is_sponsored: "true" },
      });
      c.assert.assertStatus(res, 200);
      for (const item of res.json) {
        c.assert.assertEqual(item.is_sponsored, true);
      }
    });

    suite.test("GET /api/v1/public/related?slug=<sponsored> returns 0 items even with limit=12", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-spon-rel0");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Rel0 ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "NUROL",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/public/related", {
        query: { slug, limit: "12" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual((unwrap(res)?.items || []).length, 0);
    });
  });

  // ── REQ-011 Boundaries: Media Upload & Library ────────────────────────────
  suite.describe("REQ-011 Media Upload & Library Boundaries", () => {
    suite.test("POST /api/v1/admin/upload rejects disallowed .exe / .sh extension with 400", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: Buffer.from("#!/bin/sh\necho hi"), filename: "payload.sh", contentType: "text/plain" },
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/admin/upload rejects spoofed .png with invalid magic bytes (400)", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: Buffer.from("NOT_A_REAL_PNG_FILE_CONTENT"), filename: "fake.png", contentType: "image/png" },
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/admin/upload rejects empty multipart payload with 400", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/upload", {
        multipart: { note: "no-file-here" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("PUT and DELETE /api/v1/admin/media-library/999999 return 404", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const r1 = await sa.client.put("/api/v1/admin/media-library/999999", {
        json: { alt_text_tr: "x", alt_text_en: "x", tags: "" },
      });
      const r2 = await sa.client.delete("/api/v1/admin/media-library/999999");
      c.assert.assertStatus(r1, 404);
      c.assert.assertStatus(r2, 404);
    });

    suite.test("finance_viewer is denied POST /api/v1/admin/upload with 403", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fin = await c.auth.createUserWithRole("finance_viewer", { superAdminClient: sa.client });
      const res = await fin.client.post("/api/v1/admin/upload", {
        multipart: { note: "denied" },
      });
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-012 Boundaries: Related Articles & Recommendation Rules ───────────
  suite.describe("REQ-012 Related Articles & Recommendation Rule Boundaries", () => {
    suite.test("POST /api/v1/admin/recommendation-rules rejects empty required fields with 400", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/recommendation-rules", {
        json: {
          rule_name: "",
          source_tag_or_cat: "",
          target_institution: "MSB",
          recommended_content_slug: "",
        },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("PUT and DELETE /api/v1/admin/recommendation-rules/999999 return 404", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = await getSeededContentSlug(c, true);
      const u = await sa.client.put("/api/v1/admin/recommendation-rules/999999", {
        json: {
          rule_name: "Missing",
          source_tag_or_cat: "haberler",
          target_institution: "*",
          recommended_content_slug: slug,
        },
      });
      const d = await sa.client.delete("/api/v1/admin/recommendation-rules/999999");
      c.assert.assertStatus(u, 404);
      c.assert.assertStatus(d, 404);
    });

    suite.test("GET /api/v1/public/related?limit=1 enforces minimum >= 3 recommendations", { req: "REQ-012" }, async (c) => {
      const slug = await getSeededContentSlug(c, true);
      const res = await c.http.createClient().get("/api/v1/public/related", {
        query: { slug, limit: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk((unwrap(res)?.items || []).length >= 3);
    });

    suite.test("ads_manager is denied POST /api/v1/admin/recommendation-rules (403)", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = await getSeededContentSlug(c, true);
      const ads = await c.auth.createUserWithRole("ads_manager", { superAdminClient: sa.client });
      const res = await ads.client.post("/api/v1/admin/recommendation-rules", {
        json: {
          rule_name: "Denied",
          source_tag_or_cat: "haberler",
          target_institution: "*",
          recommended_content_slug: slug,
        },
      });
      c.assert.assertStatus(res, 403);
    });

    suite.test("sponsored multi-paragraph article SSR does NOT render .msi-inline-related or .msi-related-strip (M3 contract)", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-spon-no-inline");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsorlu Uzun ${slug}`,
          body_tr: "<p>P1</p><p>P2</p><p>P3</p><p>P4</p>",
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "ASPILSAN",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, "msi-inline-related");
      c.assert.assertNotIncludes(res.text, "msi-related-strip");
    });
  });

  // ── REQ-039 Boundaries: Target Institutions ───────────────────────────────
  suite.describe("REQ-039 Target Institutions Boundaries", () => {
    suite.test("anonymous visitor is locked out of target_institutions-restricted article", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-inst-anon");
      const marker = `ANON_HIDDEN_${uid("a")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Anon Kilitli ${slug}`,
          body_tr: `<p>Giris.</p><p>Ikinci.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["Hv.K.K.lığı"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("subscriber_public from Dz.K.K.lığı is denied Hv.K.K.lığı-only content", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-inst-mismatch");
      const marker = `HVKK_ONLY_${uid("h")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `HVKK Ozel ${slug}`,
          body_tr: `<p>Acilis.</p><p>Devam.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["Hv.K.K.lığı"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const navy = await c.auth.createUserWithRole("subscriber_public", {
        superAdminClient: sa.client,
        organization_category: "Dz.K.K.lığı",
      });
      const res = await navy.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("target_institutions-restricted article hides full body on SSR /icerik/:slug for unauthorized user", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-inst-ssr");
      const marker = `SSR_INST_SECRET_${uid("s")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSR Kurum ${slug}`,
          body_tr: `<p>Acilis.</p><p>Ikinci.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["SSB"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, marker);
    });

    suite.test("target_institutions-restricted content does not leak full body in /api/v1/contents list", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-inst-list");
      const marker = `LIST_INST_SECRET_${uid("l")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Liste Kurum ${slug}`,
          body_tr: `<p>Bir.</p><p>Iki.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["MİT"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/contents", { query: { q: slug } });
      c.assert.assertStatus(res, 200);
      const item = res.json.find((x) => x.slug === slug);
      c.assert.assertOk(item);
      c.assert.assertNotIncludes(item.body_tr || "", marker);
    });

    suite.test("Admin editorial.js populates and serializes target_institutions on edit & save (M4 contract)", { req: "REQ-039" }, async (c) => {
      const res = await c.http.createClient().get("/js/admin/editorial.js");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(
        res.text,
        "target_institutions",
        "M4 editorial.js must bind target_institutions in edit/save lifecycle"
      );
    });
  });
}
