import {
  buildMinimalPng,
  buildMinimalWebp,
  getCachedSuperAdmin,
  getSeededContentSlug,
  makeContentPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-007: SSOT multi-placement publishing ──────────────────────────────
  suite.describe("REQ-007 SSOT Multi-Placement Publishing", () => {
    suite.test("single content record with multiple placements appears in each category", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ssot");
      const titleTr = `SSOT Coklu Yerlesim ${uid("s")}`;
      const createRes = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: titleTr,
          title_en: `SSOT Multi Placement ${slug}`,
          summary_tr: "SSOT ozeti",
          summary_en: "SSOT summary",
          body_tr: "<p>SSOT govde.</p>",
          body_en: "<p>SSOT body.</p>",
          content_type: "haberler",
          source_group: "dergi-icerigi",
          status: "published",
          visibility_level: 0,
          placements: ["mesajlar", "dergi-icerikleri", "tskdan-haberler"],
        }),
      });
      c.assert.assertStatus(createRes, 200);
      c.assert.assertEqual(unwrap(createRes)?.placements_count, 3);

      const pub = c.http.createClient();
      for (const cat of ["mesajlar", "dergi-icerikleri", "tskdan-haberler"]) {
        const res = await pub.get(`/kategori/${cat}`);
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, slug, `Expected SSOT slug ${slug} in /kategori/${cat}`);
      }
    });

    suite.test("updating single SSOT record propagates immediately across placements without duplication", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ssot-upd");
      const r1 = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: "Ilk Baslik SSOT",
          body_tr: "<p>Ilk govde.</p>",
          placements: ["haberler", "kapak-dosyalari"],
        }),
      });
      c.assert.assertStatus(r1, 200);
      const id = unwrap(r1)?.id;

      const updatedTitle = `Guncel Baslik SSOT ${uid("u")}`;
      const r2 = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          id,
          slug,
          title_tr: updatedTitle,
          body_tr: "<p>Guncel govde.</p>",
          placements: ["haberler", "kapak-dosyalari"],
        }),
      });
      c.assert.assertStatus(r2, 200);
      c.assert.assertEqual(unwrap(r2)?.id, id);

      const pub = c.http.createClient();
      const res = await pub.get("/kategori/kapak-dosyalari");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, updatedTitle);
    });

    suite.test("GET /api/v1/contents?category=kapak-dosyalari resolves placement records", { req: "REQ-007" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/contents", { query: { category: "kapak-dosyalari" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(res.json), "Expected array of contents");
    });

    suite.test("bulk content operation sets featured flag across multiple IDs", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-bulk-ssot");
      const created = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Bulk SSOT ${slug}`,
          is_featured: false,
          placements: ["haberler"],
        }),
      });
      c.assert.assertStatus(created, 200);
      const id = unwrap(created)?.id;

      const bulk = await sa.client.post("/api/v1/admin/contents/bulk", {
        json: { ids: [id], action: "set_featured", value: "true" },
      });
      c.assert.assertStatus(bulk, 200);
      c.assert.assertEqual(unwrap(bulk)?.affected, 1);
    });

    suite.test("DELETE /api/v1/admin/contents/:id removes content from all placements", { req: "REQ-007" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ssot-del");
      const created = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Silinecek ${slug}`,
          placements: ["haberler", "ozel-dosyalar"],
        }),
      });
      c.assert.assertStatus(created, 200);
      const id = unwrap(created)?.id;

      const del = await sa.client.delete(`/api/v1/admin/contents/${id}`);
      c.assert.assertStatus(del, 200);
      const pub = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(pub, 404);
    });
  });

  // ── REQ-008: Categories (29) + Custom Fields & Values ─────────────────────
  suite.describe("REQ-008 Categories & Custom Fields", () => {
    suite.test("GET /api/v1/categories includes all required editorial categories", { req: "REQ-008" }, async (c) => {
      const client = c.http.createClient();
      const res = await client.get("/api/v1/categories");
      c.assert.assertStatus(res, 200);
      const cats = unwrap(res);
      c.assert.assertOk(Array.isArray(cats) && cats.length >= 25, `Expected >=25 seeded categories, got ${cats?.length}`);
      const slugs = cats.map((x) => x.slug);
      for (const reqSlug of [
        "haberler",
        "ozel-haberler",
        "soylesiler",
        "mesajlar",
        "tanitici-makaleler",
        "makaleler-analizler",
        "advertorial",
        "basyazi",
      ]) {
        c.assert.assertIncludes(slugs, reqSlug, `Missing editorial category ${reqSlug}`);
      }
    });

    suite.test("admin can create and delete a custom category", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-cat");
      const createRes = await sa.client.post("/api/v1/admin/categories", {
        json: {
          id: 0,
          slug,
          name_tr: `Kategori ${slug}`,
          name_en: `Category ${slug}`,
          group_type: "icerikler",
          parent_slug: null,
          description_tr: "E2E kategori",
          description_en: "E2E category",
          sort_order: 99,
        },
      });
      c.assert.assertStatus(createRes, 200);
      const catId = unwrap(createRes)?.id;
      c.assert.assertOk(catId > 0);
      const delRes = await sa.client.delete(`/api/v1/admin/categories/${catId}`);
      c.assert.assertStatus(delRes, 200);
    });

    suite.test("custom-fields CRUD + custom-field-values GET/PUT persists for content", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fieldKey = `cf_${uid("k")}`;
      const cfCreate = await sa.client.post("/api/v1/admin/custom-fields", {
        json: {
          field_key: fieldKey,
          label_tr: "Proje Kodu",
          label_en: "Project Code",
          field_type: "text",
          applies_to: "content",
        },
      });
      c.assert.assertStatus(cfCreate, 200);

      const listCf = await sa.client.get("/api/v1/admin/custom-fields", {
        query: { applies_to: "content", per_page: "100" },
      });
      c.assert.assertStatus(listCf, 200);
      const fieldObj = unwrap(listCf).find((f) => f.field_key === fieldKey);
      c.assert.assertOk(fieldObj?.id > 0, "Expected created custom field id");

      const contentRes = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug: uniqueSlug("cf-cnt"), title_tr: "Ozel Alan Icerigi" }),
      });
      c.assert.assertStatus(contentRes, 200);
      const contentId = String(unwrap(contentRes)?.id);

      const putVal = await sa.client.put("/api/v1/admin/custom-field-values", {
        json: {
          entity_type: "content",
          entity_id: contentId,
          values: [{ field_id: fieldObj.id, value_text: "KAAN-BLK2" }],
        },
      });
      c.assert.assertStatus(putVal, 200);
      c.assert.assertEqual(unwrap(putVal)?.updated_count, 1);

      const getVal = await sa.client.get("/api/v1/admin/custom-field-values", {
        query: { entity_type: "content", entity_id: contentId },
      });
      c.assert.assertStatus(getVal, 200);
      const values = unwrap(getVal)?.values || [];
      c.assert.assertOk(values.some((v) => v.field_id === fieldObj.id && v.value_text === "KAAN-BLK2"));

      await sa.client.delete(`/api/v1/admin/custom-fields/${encodeURIComponent(fieldKey)}`);
    });

    suite.test("custom-field-values supports ad and user entity types", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const fieldKey = `cf_ad_${uid("k")}`;
      await sa.client.post("/api/v1/admin/custom-fields", {
        json: {
          field_key: fieldKey,
          label_tr: "Kampanya Referansi",
          label_en: "Campaign Ref",
          field_type: "text",
          applies_to: "ad",
        },
      });
      const listCf = await sa.client.get("/api/v1/admin/custom-fields", {
        query: { applies_to: "ad", per_page: "100" },
      });
      const fieldObj = unwrap(listCf).find((f) => f.field_key === fieldKey);
      c.assert.assertOk(fieldObj?.id > 0);

      const putRes = await sa.client.put("/api/v1/admin/custom-field-values", {
        json: {
          entity_type: "ad",
          entity_id: "1",
          values: [{ field_id: fieldObj.id, value_text: "PO-2026-88" }],
        },
      });
      c.assert.assertStatus(putRes, 200);

      const getRes = await sa.client.get("/api/v1/admin/custom-field-values", {
        query: { entity_type: "ad", entity_id: "1" },
      });
      c.assert.assertStatus(getRes, 200);
      c.assert.assertOk(
        (unwrap(getRes)?.values || []).some((v) => v.field_id === fieldObj.id && v.value_text === "PO-2026-88")
      );
      await sa.client.delete(`/api/v1/admin/custom-fields/${encodeURIComponent(fieldKey)}`);
    });

    suite.test("Admin Content Editor UI wires custom-field-values persistence (M4 contract)", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [shellRes, editorialJsRes, coreJsRes] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/editorial.js"),
        sa.client.get("/js/admin/core.js"),
      ]);
      c.assert.assertStatus(shellRes, 200);
      c.assert.assertStatus(editorialJsRes, 200);
      c.assert.assertStatus(coreJsRes, 200);
      c.assert.assertIncludes(
        shellRes.text,
        'id="content-custom-fields"',
        "M4 Admin Content Editor drawer must render #content-custom-fields container"
      );
      c.assert.assertMatch(
        editorialJsRes.text,
        /loadCustomFieldValues\s*\(\s*["']content-custom-fields["']/,
        "M4 editorial.js must call loadCustomFieldValues('content-custom-fields', ...)"
      );
      c.assert.assertMatch(
        editorialJsRes.text,
        /saveCustomFieldValues\s*\(\s*["']content-custom-fields["']/,
        "M4 editorial.js must call saveCustomFieldValues('content-custom-fields', ...)"
      );
      c.assert.assertIncludes(
        coreJsRes.text,
        "/api/v1/admin/custom-field-values",
        "M4 core.js shared custom-field helpers must call /api/v1/admin/custom-field-values"
      );
    });

    suite.test("GET /api/v1/admin/custom-fields returns populated non-mock operational defaults", { req: "REQ-008" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/custom-fields", {
        query: { per_page: "100" },
      });
      c.assert.assertStatus(res, 200);
      const fields = unwrap(res);
      c.assert.assertOk(
        Array.isArray(fields) && fields.length >= 1,
        `Expected >= 1 seeded operational default custom_field, got ${fields?.length ?? 0}`
      );
      for (const f of fields) {
        c.assert.assertOk(
          typeof f.field_key === "string" && f.field_key.trim().length > 0,
          "Default custom_field must have non-empty field_key"
        );
        c.assert.assertOk(
          typeof f.label_tr === "string" && f.label_tr.trim().length > 0,
          `Default custom_field ${f.field_key} must have non-empty label_tr`
        );
        c.assert.assertOk(
          typeof f.label_en === "string" && f.label_en.trim().length > 0,
          `Default custom_field ${f.field_key} must have non-empty label_en`
        );
        c.assert.assertNotIncludes(
          `${f.field_key} ${f.label_tr} ${f.label_en}`.toLowerCase(),
          "lorem",
          `Default custom_field ${f.field_key} must not contain placeholder text`
        );
        c.assert.assertNotIncludes(
          `${f.field_key} ${f.label_tr} ${f.label_en}`.toLowerCase(),
          "dummy",
          `Default custom_field ${f.field_key} must not contain dummy text`
        );
      }
    });
  });

  // ── REQ-009: Editorial approval workflow & content versioning ─────────────
  suite.describe("REQ-009 Editorial Approval & Versioning", () => {
    suite.test("content_editor creates draft, submits review, editor_in_chief approves", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const editor = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const chief = await c.auth.createUserWithRole("editor_in_chief", { superAdminClient: sa.client });

      const slug = uniqueSlug("t1-appr");
      const draftRes = await editor.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: `Onay Bekleyen ${slug}`, status: "draft" }),
      });
      c.assert.assertStatus(draftRes, 200);
      const contentId = unwrap(draftRes)?.id;

      const submitRes = await editor.client.post(`/api/v1/admin/contents/${contentId}/submit-review`, {
        json: { note: "Lutfen inceleyin" },
      });
      c.assert.assertStatus(submitRes, 200);
      const approvalId = unwrap(submitRes)?.approval_id;
      c.assert.assertOk(approvalId > 0);

      const pendingRes = await chief.client.get("/api/v1/admin/approvals/pending");
      c.assert.assertStatus(pendingRes, 200);
      c.assert.assertOk(unwrap(pendingRes).some((a) => a.id === approvalId));

      const approveRes = await chief.client.post(`/api/v1/admin/approvals/${approvalId}/approve`, {
        json: { note: "Yayina uygun" },
      });
      c.assert.assertStatus(approveRes, 200);
      c.assert.assertEqual(unwrap(approveRes)?.status, "approved");
    });

    suite.test("approver can reject a submitted content review with reason", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const editor = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const slug = uniqueSlug("t1-rej");
      const draftRes = await editor.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: `Reddedilecek ${slug}`, status: "draft" }),
      });
      c.assert.assertStatus(draftRes, 200);
      const contentId = unwrap(draftRes)?.id;
      const submitRes = await editor.client.post(`/api/v1/admin/contents/${contentId}/submit-review`, { json: {} });
      c.assert.assertStatus(submitRes, 200);
      const approvalId = unwrap(submitRes)?.approval_id;

      const rejRes = await sa.client.post(`/api/v1/admin/approvals/${approvalId}/reject`, {
        json: { note: "Kaynak eksik" },
      });
      c.assert.assertStatus(rejRes, 200);
      c.assert.assertEqual(unwrap(rejRes)?.status, "rejected");
    });

    suite.test("GET /api/v1/admin/contents/:id/versions lists auto-created versions", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ver");
      const created = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: "Versiyon 1" }),
      });
      c.assert.assertStatus(created, 200);
      const id = unwrap(created)?.id;
      const vers = await sa.client.get(`/api/v1/admin/contents/${id}/versions`);
      c.assert.assertStatus(vers, 200);
      const list = unwrap(vers);
      c.assert.assertOk(Array.isArray(list) && list.length >= 1, "Expected at least 1 version entry");
    });

    suite.test("POST /api/v1/admin/contents/:id/versions creates explicit version snapshot", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const created = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug: uniqueSlug("t1-ver2"), title_tr: "Ilk Baslik", status: "draft" }),
      });
      c.assert.assertStatus(created, 200);
      const id = unwrap(created)?.id;

      const snap = await sa.client.post(`/api/v1/admin/contents/${id}/versions`, {
        json: {
          content_id: id,
          title_tr: "Ikinci Snapshot",
          body_tr: "<p>Ikinci govde</p>",
          summary_tr: "Ozet 2",
          change_note: "E2E revizyon",
        },
      });
      c.assert.assertStatus(snap, 200);
      c.assert.assertEqual(unwrap(snap)?.status, "created");
    });

    suite.test("POST /api/v1/admin/contents/:id/versions/:version/restore rolls back content", { req: "REQ-009" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-restore");
      const r1 = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: "Orijinal Baslik V1" }),
      });
      c.assert.assertStatus(r1, 200);
      const id = unwrap(r1)?.id;

      await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ id, slug, title_tr: "Degistirilmis Baslik V2" }),
      });

      const restore = await sa.client.post(`/api/v1/admin/contents/${id}/versions/1/restore`);
      c.assert.assertStatus(restore, 200);
      c.assert.assertEqual(unwrap(restore)?.status, "restored");

      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.content?.title_tr, "Orijinal Baslik V1");
    });
  });

  // ── REQ-010: Legal labeling of sponsored/advertorial content ──────────────
  suite.describe("REQ-010 Sponsored & Advertorial Legal Labels", () => {
    suite.test("sponsored content renders legal label SPONSORLU İÇERİK on detail page", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-spon1");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsorlu Yayin ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "ASELSAN",
          sponsor_legal_label: "SPONSORLU İÇERİK",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "SPONSORLU İÇERİK");
      c.assert.assertIncludes(res.text, "ASELSAN");
    });

    suite.test("advertorial with İLANDIR label persists and renders legally", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-spon2");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Ilan Yayini ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "ROKETSAN",
          sponsor_legal_label: "İLANDIR",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "İLANDIR");
    });

    suite.test("category listing /kategori/advertorial displays sponsored badge", { req: "REQ-010" }, async (c) => {
      const res = await c.http.createClient().get("/kategori/advertorial");
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /(SPONSORLU|ADVERTORIAL|İLANDIR)/i);
    });

    suite.test("content API /api/v1/contents/:slug exposes is_sponsored and sponsor_legal_label", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-spon3");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `API Sponsorlu ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "HAVELSAN",
          sponsor_legal_label: "ADVERTORIAL",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.content?.is_sponsored, true);
      c.assert.assertEqual(res.json?.content?.sponsor_legal_label, "ADVERTORIAL");
    });

    suite.test("sponsored content activates competitor protection flag in API response", { req: "REQ-010" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-spon4");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Koruma Aktif ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "STM",
          sponsor_legal_label: "SPONSORLU İÇERİK",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.competitor_protection_active, true);
    });
  });

  // ── REQ-011: Media technical specs & image library ────────────────────────
  suite.describe("REQ-011 Media Upload & Image Library", () => {
    suite.test("POST /api/v1/admin/upload accepts valid PNG image and returns URL", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const png = buildMinimalPng();
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: png, filename: `test-${uid("img")}.png`, contentType: "image/png" },
        },
      });
      c.assert.assertStatus(up, 200);
      c.assert.assertEqual(up.json?.status, "uploaded");
      c.assert.assertOk(typeof up.json?.url === "string" && up.json.url.length > 0);
    });

    suite.test("POST /api/v1/admin/upload accepts valid WebP image", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const webp = buildMinimalWebp();
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: {
          file: { content: webp, filename: `test-${uid("webp")}.webp`, contentType: "image/webp" },
        },
      });
      c.assert.assertStatus(up, 200);
      c.assert.assertIncludes(up.json?.url || "", ".webp");
    });

    suite.test("GET /api/v1/admin/media-library lists uploaded media assets with pagination", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/media-library", {
        query: { page: "1", per_page: "20" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)), "Media library must return array");
    });

    suite.test("PUT /api/v1/admin/media-library/:id updates bilingual alt text and tags", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const png = buildMinimalPng();
      const fname = `alttest-${uid("m").replace(/[^a-z0-9]/gi, "")}.png`;
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: { file: { content: png, filename: fname, contentType: "image/png" } },
      });
      c.assert.assertStatus(up, 200);
      const list = await sa.client.get("/api/v1/admin/media-library", { query: { per_page: "100" } });
      const item = unwrap(list).find((m) => m.url === up.json?.url || (m.original_name || "") === fname);
      c.assert.assertOk(item?.id > 0, "Uploaded media item should appear in media library");

      const upd = await sa.client.put(`/api/v1/admin/media-library/${item.id}`, {
        json: {
          alt_text_tr: "KAAN ucak gorseli",
          alt_text_en: "KAAN aircraft image",
          tags: "hava-araclari,kaan",
        },
      });
      c.assert.assertStatus(upd, 200);
      c.assert.assertEqual(unwrap(upd)?.status, "updated");
    });

    suite.test("DELETE /api/v1/admin/media-library/:id removes media record", { req: "REQ-011" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const png = buildMinimalPng();
      const fname = `delm-${uid("d").replace(/[^a-z0-9]/gi, "")}.png`;
      const up = await sa.client.post("/api/v1/admin/upload", {
        multipart: { file: { content: png, filename: fname, contentType: "image/png" } },
      });
      c.assert.assertStatus(up, 200);
      const list = await sa.client.get("/api/v1/admin/media-library", { query: { per_page: "100" } });
      const item = unwrap(list).find((m) => m.url === up.json?.url || (m.original_name || "") === fname);
      c.assert.assertOk(item?.id > 0, "Uploaded media item must exist before delete");

      const del = await sa.client.delete(`/api/v1/admin/media-library/${item.id}`);
      c.assert.assertStatus(del, 200);
      c.assert.assertEqual(unwrap(del)?.status, "deleted");
    });
  });

  // ── REQ-012: Related articles mid-body + bottom + rules CRUD ──────────────
  suite.describe("REQ-012 Related Articles & Recommendation Rules", () => {
    suite.test("GET /api/v1/public/related returns >=3 recommendations for non-sponsored content", { req: "REQ-012" }, async (c) => {
      const client = c.http.createClient();
      const slug = await getSeededContentSlug(c, true);
      const res = await client.get("/api/v1/public/related", {
        query: { slug, limit: "4" },
      });
      c.assert.assertStatus(res, 200);
      const items = unwrap(res)?.items || [];
      c.assert.assertOk(items.length >= 3, `Expected >=3 related articles, got ${items.length}`);
    });

    suite.test("GET /api/v1/public/related returns empty items for sponsored content", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-rel-spon");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Sponsorlu Icerik ${slug}`,
          content_type: "advertorial",
          is_sponsored: true,
          sponsor_brand: "TEI",
          placements: ["advertorial"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/public/related", { query: { slug } });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual((unwrap(res)?.items || []).length, 0, "Sponsored article must not show related articles");
    });

    suite.test("recommendation-rules CRUD creates, lists, updates, and deletes rule", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const targetSlug = await getSeededContentSlug(c, true);
      const ruleName = `Rule_${uid("r")}`;
      const createRes = await sa.client.post("/api/v1/admin/recommendation-rules", {
        json: {
          rule_name: ruleName,
          source_tag_or_cat: "hava-araclari",
          target_institution: "MSB",
          recommended_content_slug: targetSlug,
          priority: 25,
          is_active: true,
        },
      });
      c.assert.assertStatus(createRes, 201);
      const id = unwrap(createRes)?.id;
      c.assert.assertOk(id > 0);

      const listRes = await sa.client.get("/api/v1/admin/recommendation-rules");
      c.assert.assertStatus(listRes, 200);
      c.assert.assertOk((unwrap(listRes)?.rules || []).some((r) => r.id === id && r.rule_name === ruleName));

      const updRes = await sa.client.put(`/api/v1/admin/recommendation-rules/${id}`, {
        json: {
          rule_name: `${ruleName}_updated`,
          source_tag_or_cat: "hava-araclari",
          target_institution: "SSB",
          recommended_content_slug: targetSlug,
          priority: 50,
          is_active: true,
        },
      });
      c.assert.assertStatus(updRes, 200);

      const delRes = await sa.client.delete(`/api/v1/admin/recommendation-rules/${id}`);
      c.assert.assertStatus(delRes, 200);
    });

    suite.test("long non-sponsored article SSR injects inline mid-body .msi-inline-related block (M3 contract)", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-mid-related");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Uzun Analiz ${slug}`,
          body_tr:
            "<p>Birinci paragraf savunma teknolojileri analizine giris yapar.</p>" +
            "<p>Ikinci paragraf yerli radar ve aviyonik sistemleri detaylandirir.</p>" +
            "<p>Ucuncu paragraf ihracat potansiyeli ve tedarik zincirini inceler.</p>" +
            "<p>Dorduncu paragraf gelecek projeksiyonlarini sunar.</p>",
          ssb_tags: ["hava-araclari"],
          placements: ["haberler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertMatch(res.text, /class="[^"]*\bstories\b/, "Bottom related stories grid must be rendered");
      c.assert.assertIncludes(res.text, "msi-related-strip", "Bottom related strip container must be rendered");
      c.assert.assertIncludes(
        res.text,
        "msi-inline-related",
        "M3 requires mid-body <aside class='msi-inline-related' data-related-inline> inside article body"
      );
    });

    suite.test("Admin Panel UI includes recommendation_rules builder controls (M4 contract)", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [panelRes, jsRes] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/editorial.js"),
      ]);
      c.assert.assertStatus(panelRes, 200);
      const combined = `${panelRes.text}\n${jsRes.text}`;
      c.assert.assertIncludes(
        combined,
        "recommendation-rules",
        "M4 Admin UI must expose recommendation-rules management UI"
      );
    });

    suite.test("GET /api/v1/admin/recommendation-rules returns active defense-sector defaults pointing to valid published slugs", { req: "REQ-012" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/recommendation-rules");
      c.assert.assertStatus(res, 200);
      const rules = unwrap(res)?.rules || [];
      c.assert.assertOk(
        Array.isArray(rules) && rules.length >= 1,
        `Expected >= 1 seeded operational default recommendation_rule, got ${rules?.length ?? 0}`
      );
      const activeRules = rules.filter((r) => Boolean(r.is_active));
      c.assert.assertOk(
        activeRules.length >= 1,
        "At least one default recommendation_rule must be active"
      );
      const pubClient = c.http.createClient();
      for (const r of activeRules.slice(0, 5)) {
        c.assert.assertOk(
          typeof r.rule_name === "string" && r.rule_name.trim().length > 0,
          "Default recommendation_rule must have non-empty rule_name"
        );
        c.assert.assertOk(
          typeof r.recommended_content_slug === "string" && r.recommended_content_slug.trim().length > 0,
          `Default recommendation_rule ${r.rule_name} must reference recommended_content_slug`
        );
        const detail = await pubClient.get(`/api/v1/contents/${encodeURIComponent(r.recommended_content_slug)}`);
        c.assert.assertStatus(
          detail,
          200,
          `Default recommendation_rule ${r.rule_name} slug "${r.recommended_content_slug}" must resolve 200`
        );
      }
    });
  });

  // ── REQ-039: Option-B target_institutions persistence & UI wiring ─────────
  suite.describe("REQ-039 Target Institutions Content Visibility & UI", () => {
    suite.test("content created with target_institutions persists field in API", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-inst1");
      const createRes = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Kuruma Ozel ${slug}`,
          content_type: "makaleler-analizler",
          target_institutions: ["MSB", "SSB"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(createRes, 200);

      const listRes = await sa.client.get("/api/v1/admin/contents", { query: { q: slug } });
      c.assert.assertStatus(listRes, 200);
      const item = unwrap(listRes).find((x) => x.slug === slug);
      c.assert.assertIncludes(item?.target_institutions || [], "MSB");
      c.assert.assertIncludes(item?.target_institutions || [], "SSB");
    });

    suite.test("user from matching target institution receives unlocked content", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-inst2");
      const marker = `SECRET_MSB_MARKER_${uid("x")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `MSB Ozel ${slug}`,
          body_tr: `<p>Acilis cumlesi.</p><p>Ikinci cumle.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["MSB"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);

      const msbUser = await c.auth.createUserWithRole("subscriber_public", {
        superAdminClient: sa.client,
        organization_category: "MSB",
      });
      const detail = await msbUser.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.paywall?.is_locked, false);
      c.assert.assertIncludes(detail.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("user from non-matching institution sees institutional lock on target_institutions content", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-inst3");
      const marker = `LOCKED_SSB_MARKER_${uid("y")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSB Ozel ${slug}`,
          body_tr: `<p>Birinci cumle.</p><p>Ikinci cumle.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          target_institutions: ["SSB"],
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);

      const otherUser = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        organization_category: "Üniversite / Öğrenci",
      });
      const detail = await otherUser.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      c.assert.assertEqual(detail.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(detail.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("clearing target_institutions restores open access for all visitors", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-inst4");
      const r1 = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Gecici Kisitli ${slug}`,
          body_tr: "<p>Bir.</p><p>Iki.</p><p>OPEN_NOW_BODY</p>",
          target_institutions: ["MİT"],
        }),
      });
      c.assert.assertStatus(r1, 200);
      const id = unwrap(r1)?.id;

      await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          id,
          slug,
          title_tr: `Gecici Kisitli ${slug}`,
          body_tr: "<p>Bir.</p><p>Iki.</p><p>OPEN_NOW_BODY</p>",
          target_institutions: [],
        }),
      });

      const anon = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(anon, 200);
      c.assert.assertEqual(anon.json?.paywall?.is_locked, false);
    });

    suite.test("Admin Content Editor UI provides target_institutions multi-select controls (M4 contract)", { req: "REQ-039" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const [panelRes, jsRes] = await Promise.all([
        sa.client.get(`/${c.adminPath}`),
        sa.client.get("/js/admin/editorial.js"),
      ]);
      c.assert.assertStatus(panelRes, 200);
      c.assert.assertStatus(jsRes, 200);
      c.assert.assertIncludes(
        jsRes.text,
        "target_institutions",
        "M4 editorial.js must read/write target_institutions in Content Editor UI"
      );
      c.assert.assertMatch(
        panelRes.text,
        /c-target-institutions|c_institution|target_institutions/i,
        "M4 admin content drawer HTML must expose target_institutions control"
      );
    });
  });
}
