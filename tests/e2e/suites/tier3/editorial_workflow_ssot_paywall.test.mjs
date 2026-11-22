import {
  createAdminContent,
  createVerifiedSubscriber,
  findAdminInvoice,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 6: Editorial Workflow, Versioning, SSOT, Paywall & DRP", () => {
    suite.test(
      "T3-35: Multi-role editorial workflow: content_editor draft -> submit-review -> editor_in_chief approves -> public visibility",
      { req: ["REQ-009", "REQ-033"] },
      async (c) => {
        const editor = await c.auth.createUserWithRole("content_editor");
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const slug = uid("t3-workflow");

        // content_editor cannot directly publish -> 403
        const directPub = await createAdminContent(c, editor.client, {
          slug: uid("t3-forbidden-pub"),
          status: "published",
          allowError: true,
        });
        assert.assertStatus(
          directPub.res,
          403,
          "content_editor must be forbidden (403) from directly publishing content"
        );

        // content_editor saves draft
        const draft = await createAdminContent(c, editor.client, {
          slug,
          status: "draft",
          title_tr: "Taslak Savunma Dosyası",
        });
        assert.assertOk(draft.id > 0);

        const anon = c.http.createClient();
        const beforeApprove = await anon.get(`/api/v1/contents/${slug}`);
        assert.assertStatus(beforeApprove, 404, "Draft content must not be visible publicly");

        // Submit for editorial review
        const submitRes = await editor.client.post(
          `/api/v1/admin/contents/${draft.id}/submit-review`,
          { json: {} }
        );
        assert.assertStatus(submitRes, 200);
        const approvalId = submitRes.json?.data?.approval_id;
        assert.assertOk(approvalId > 0, "submit-review must create an approval_request");

        // Verify pending list contains approval
        const pendingRes = await eic.client.get("/api/v1/admin/approvals/pending");
        assert.assertStatus(pendingRes, 200);
        assert.assertOk(
          (pendingRes.json?.data || []).some((a) => a.id === approvalId),
          "Pending approvals queue must list submitted request"
        );

        // editor_in_chief approves
        const approveRes = await eic.client.post(
          `/api/v1/admin/approvals/${approvalId}/approve`,
          { json: { note: "E2E Yayın Kurulu Onayı" } }
        );
        assert.assertStatus(approveRes, 200);

        const afterApprove = await anon.get(`/api/v1/contents/${slug}`);
        assert.assertStatus(
          afterApprove,
          200,
          "Approved content must immediately become visible on public API"
        );
      }
    );

    suite.test(
      "T3-36: Content version history records edits and restores earlier snapshot cleanly",
      { req: ["REQ-009"] },
      async (c) => {
        const eic = await c.auth.createUserWithRole("editor_in_chief");
        const slug = uid("t3-versions");
        const v1Title = `Versiyon 1 Başlık ${uid("v1")}`;
        const v2Title = `Versiyon 2 Başlık ${uid("v2")}`;

        const created = await createAdminContent(c, eic.client, {
          slug,
          title_tr: v1Title,
        });

        await eic.client.post("/api/v1/admin/contents", {
          json: {
            ...created.payload,
            id: created.id,
            title_tr: v2Title,
          },
        });

        const versRes = await eic.client.get(
          `/api/v1/admin/contents/${created.id}/versions`
        );
        assert.assertStatus(versRes, 200);
        const versions = versRes.json?.data || [];
        assert.assertOk(versions.length >= 2, "Expected >= 2 recorded content versions");
        const firstVersionNum = Math.min(...versions.map((v) => v.version ?? v.version_number));

        const restoreRes = await eic.client.post(
          `/api/v1/admin/contents/${created.id}/versions/${firstVersionNum}/restore`,
          { json: {} }
        );
        assert.assertStatus(restoreRes, 200, "Version restore must return 200");

        const anon = c.http.createClient();
        const detail = await anon.get(`/api/v1/contents/${slug}`);
        assert.assertStatus(detail, 200);
        assert.assertEqual(
          detail.json?.content?.title_tr,
          v1Title,
          "Restored content must revert public title_tr to v1"
        );
      }
    );

    suite.test(
      "T3-37: SSOT multi-placement publishing surfaces one canonical record across multiple category/placement endpoints",
      { req: ["REQ-007", "REQ-008"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const slug = uid("t3-ssot-multi");

        await createAdminContent(c, op.client, {
          slug,
          title_tr: "SSOT Çoklu Yerleşim Analizi",
          content_type: "haberler",
          placements: ["haberler", "ozel-dosyalar", "tskdan-haberler"],
        });

        const anon = c.http.createClient();
        for (const cat of ["haberler", "ozel-dosyalar", "tskdan-haberler"]) {
          const searchRes = await anon.get("/api/v1/search", {
            query: { category: cat, per_page: "50" },
          });
          assert.assertStatus(searchRes, 200);
          const found = (searchRes.json?.items || []).some((i) => i.slug === slug);
          assert.assertOk(
            found,
            `SSOT content must appear in placement category "${cat}"`
          );

          const ssrCat = await anon.get(`/kategori/${cat}`);
          assert.assertStatus(ssrCat, 200);
        }

        const canonicalDetail = await anon.get(`/icerik/${slug}`);
        assert.assertStatus(canonicalDetail, 200);
        assert.assertIncludes(canonicalDetail.text, `/icerik/${slug}`);
      }
    );

    suite.test(
      "T3-38: 3-tier paywall visibility (0 open, 1 registered+, 2 subscriber+) enforces teaser truncation & role unlocks",
      { req: ["REQ-013", "REQ-014"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const sec1 = `LVL1-SECRET-${uid("s1")}`;
        const sec2 = `LVL2-SECRET-${uid("s2")}`;

        const c0 = await createAdminContent(c, op.client, {
          slug: uid("t3-vis0"),
          visibility_level: 0,
          body_tr: "<p>Açık cümle bir.</p><p>Açık cümle iki.</p><p>OPEN-FULL-TEXT.</p>",
        });
        const c1 = await createAdminContent(c, op.client, {
          slug: uid("t3-vis1"),
          visibility_level: 1,
          body_tr: `<p>Kayıtlı giriş cümlesi bir.</p><p>İkinci giriş cümlesi.</p><p>${sec1} yalnızca üyelere.</p>`,
        });
        const c2 = await createAdminContent(c, op.client, {
          slug: uid("t3-vis2"),
          visibility_level: 2,
          single_price_try: 75,
          body_tr: `<p>Abone analiz cümlesi bir.</p><p>İkinci analiz cümlesi.</p><p>${sec2} yalnızca abonelere.</p>`,
        });

        const anon = c.http.createClient();
        const reg = await createVerifiedSubscriber(c, "registered");
        const sub = await createVerifiedSubscriber(c, "subscriber_digital");

        // Anonymous: 0 unlocked; 1 & 2 locked teaser
        const a0 = await anon.get(`/api/v1/contents/${c0.slug}`);
        const a1 = await anon.get(`/api/v1/contents/${c1.slug}`);
        const a2 = await anon.get(`/api/v1/contents/${c2.slug}`);
        assert.assertEqual(a0.json?.paywall?.is_locked, false);
        assert.assertEqual(a1.json?.paywall?.is_locked, true);
        assert.assertNotIncludes(a1.json?.paywall?.rendered_body || "", sec1);
        assert.assertEqual(a2.json?.paywall?.is_locked, true);
        assert.assertNotIncludes(a2.json?.paywall?.rendered_body || "", sec2);

        // Registered: 1 unlocked; 2 still locked
        const r1 = await reg.client.get(`/api/v1/contents/${c1.slug}`);
        const r2 = await reg.client.get(`/api/v1/contents/${c2.slug}`);
        assert.assertEqual(r1.json?.paywall?.is_locked, false);
        assert.assertIncludes(r1.json?.paywall?.rendered_body || "", sec1);
        assert.assertEqual(r2.json?.paywall?.is_locked, true);
        assert.assertNotIncludes(r2.json?.paywall?.rendered_body || "", sec2);

        // Subscriber: 2 unlocked
        const s2 = await sub.client.get(`/api/v1/contents/${c2.slug}`);
        assert.assertEqual(s2.json?.paywall?.is_locked, false);
        assert.assertIncludes(s2.json?.paywall?.rendered_body || "", sec2);
      }
    );

    suite.test(
      "T3-39: Single-article purchase + admin mark-paid unlocks visibility_level=2 article for registered reader",
      { req: ["REQ-013", "REQ-031"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const paywallSecret = `SINGLE-BUY-UNLOCKED-${uid("sb")}`;
        const paidArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-single-buy"),
          visibility_level: 2,
          single_price_try: 120.0,
          body_tr: `<p>Ön izleme cümle bir.</p><p>Ön izleme cümle iki.</p><p>${paywallSecret} tam metin.</p>`,
        });

        const buyer = await createVerifiedSubscriber(c, "registered");
        const beforeBuy = await buyer.client.get(`/api/v1/contents/${paidArticle.slug}`);
        assert.assertEqual(beforeBuy.json?.paywall?.is_locked, true);

        const checkout = await buyer.client.post("/api/v1/subscriptions/checkout", {
          json: {
            user_email: buyer.email,
            full_name: buyer.user.full_name,
            item_type: "single_article",
            item_code: paidArticle.slug,
            distance_sales_consent: true,
          },
        });
        assert.assertStatus(checkout, 202);
        const invoiceId = checkout.json?.invoice_id;
        assert.assertOk(invoiceId > 0);

        const markPaid = await op.client.post(`/api/v1/admin/invoices/${invoiceId}/mark-paid`, {
          json: {},
        });
        assert.assertStatus(markPaid, 200);

        const afterBuy = await buyer.client.get(`/api/v1/contents/${paidArticle.slug}`);
        assert.assertEqual(
          afterBuy.json?.paywall?.is_locked,
          false,
          "Purchased single_article must unlock for registered reader after mark-paid"
        );
        assert.assertIncludes(afterBuy.json?.paywall?.rendered_body || "", paywallSecret);
      }
    );

    suite.test(
      "T3-40: Future-phase hubs & AI/SEO discovery endpoints return 200 with valid content",
      { req: ["REQ-036", "REQ-004"] },
      async (c) => {
        const anon = c.http.createClient();
        for (const path of [
          "/savunma-sanayii-tarihi",
          "/en/defence-industry-history",
          "/yazarlar",
          "/en/authors",
          "/ogrenci-kulupleri",
          "/kategori/insan-kaynaklari",
          "/sitemap.xml",
          "/llms.txt",
          "/api/v1/openapi.json",
        ]) {
          const res = await anon.get(path);
          assert.assertStatus(res, 200, `Hub/SEO route ${path} must return 200`);
          assert.assertOk(res.text.length > 50, `Route ${path} must return non-empty payload`);
        }
      }
    );

    suite.test(
      "T3-41: Backup trigger enforces super_admin RBAC (403 site_operator) & Open Data export produces portable anonymized JSON",
      { req: ["REQ-034", "REQ-033"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const sa = await c.auth.loginAsSuperAdmin();

        const opBackup = await op.client.post("/api/v1/admin/backup", { json: {} });
        assert.assertStatus(
          opBackup,
          403,
          "site_operator lacks backup:manage and must be denied POST /api/v1/admin/backup"
        );

        const saBackup = await sa.client.post("/api/v1/admin/backup", { json: {} });
        assert.assertStatus(saBackup, 200, "super_admin backup trigger must return 200");
        assert.assertEqual(saBackup.json?.data?.status, "completed");
        assert.assertOk(
          Boolean(saBackup.json?.data?.sha256),
          "Backup result must include SHA-256 checksum"
        );

        const openData = await sa.client.get("/api/v1/admin/export/open-data", {
          query: { anonymize: "true" },
        });
        assert.assertStatus(openData, 200);
        assert.assertEqual(
          openData.json?.export_standard,
          "UMSA-MSI-OPEN-DATA-V1 (Zero Vendor-Lock Universal JSON/PostgreSQL/SQLite Portable Archive)"
        );
        assert.assertEqual(openData.json?.anonymized, true);
        assert.assertOk(Array.isArray(openData.json?.contents_single_source_of_truth));
      }
    );
  });
}
