import fs from "node:fs";
import path from "node:path";
import {
  buildMinimalPng,
  getCachedRoleSession,
  getCachedSuperAdmin,
  uid,
  uniqueSlug,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  suite.describe("REQ-042 Editorial Studio SSR, WASM Assets & API Happy Paths", () => {
    suite.test(
      "GET /podmin/editor and /{adminPath}/editor return 200 with WASM module tags and CSP wasm-unsafe-eval for editorial roles",
      { req: "REQ-042" },
      async (c) => {
        for (const role of ["super_admin", "editor_in_chief", "content_editor"]) {
          const session = await getCachedRoleSession(c, role);
          for (const route of ["/podmin/editor", `/${c.adminPath}/editor`]) {
            const res = await session.client.get(route);
            c.assert.assertStatus(res, 200, `${role} GET ${route} must return 200`);
            const csp = (res.headers.get("content-security-policy") || "").toLowerCase();
            c.assert.assertIncludes(
              csp,
              "wasm-unsafe-eval",
              `${route} CSP must allow wasm-unsafe-eval`
            );
            c.assert.assertIncludes(res.text, 'id="root"');
            c.assert.assertIncludes(res.text, "/css/admin/tokens.css");
            c.assert.assertIncludes(res.text, "/css/admin/editorial-studio.css");
            c.assert.assertMatch(
              res.text,
              /\/editor\/studio\.js/,
              `${route} must reference editorial studio JS bootstrap`
            );
            c.assert.assertMatch(
              res.text,
              /\/editor\/studio\.wasm/,
              `${route} must reference editorial studio WASM module`
            );
          }
        }
      }
    );

    suite.test(
      "Editorial Studio CSS, JS bootstrap, and WASM binary assets serve 200 with valid content-types and magic bytes",
      { req: "REQ-042" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const [tokensRes, studioCssRes, jsRes, wasmRes, podJsRes, podWasmRes] = await Promise.all([
          sa.client.get("/css/admin/tokens.css"),
          sa.client.get("/css/admin/editorial-studio.css"),
          sa.client.get(`/${c.adminPath}/editor/studio.js`),
          sa.client.raw("GET", `/${c.adminPath}/editor/studio.wasm`),
          sa.client.get("/podmin/editor/studio.js"),
          sa.client.raw("GET", "/podmin/editor/studio.wasm"),
        ]);

        c.assert.assertStatus(tokensRes, 200);
        c.assert.assertStatus(studioCssRes, 200);
        c.assert.assertOk(studioCssRes.text.length > 1000, "editorial-studio.css must be non-empty");

        for (const r of [jsRes, podJsRes]) {
          c.assert.assertStatus(r, 200);
          c.assert.assertIncludes(
            (r.headers.get("content-type") || "").toLowerCase(),
            "javascript"
          );
          c.assert.assertOk(r.text.length > 500, "studio.js must be non-empty");
        }

        for (const r of [wasmRes, podWasmRes]) {
          c.assert.assertStatus(r, 200);
          c.assert.assertIncludes(
            (r.headers.get("content-type") || "").toLowerCase(),
            "application/wasm"
          );
          c.assert.assertOk(r.rawBody.byteLength > 1024, "studio.wasm must be non-empty");
          // WebAssembly magic bytes: \0asm
          c.assert.assertEqual(
            r.rawBody.subarray(0, 4).toString("latin1"),
            "\0asm",
            "studio.wasm must start with WebAssembly magic header \\0asm"
          );
        }
      }
    );

    suite.test(
      "GET /api/v1/admin/editorial/{session,meta,categories,tags,media,settings,redirects,research/sources} return 200 with expected shapes",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");
        const chief = await getCachedRoleSession(c, "editor_in_chief");

        const ceSess = await ce.client.get("/api/v1/admin/editorial/session");
        c.assert.assertStatus(ceSess, 200);
        c.assert.assertEqual(ceSess.json?.success, true);
        c.assert.assertEqual(ceSess.json?.authenticated, true);
        c.assert.assertEqual(ceSess.json?.user?.role, "content_editor");
        c.assert.assertEqual(ceSess.json?.user?.capabilities?.canWrite, true);
        c.assert.assertEqual(ceSess.json?.user?.capabilities?.canPublish, false);
        c.assert.assertEqual(ceSess.json?.user?.capabilities?.canDelete, false);
        c.assert.assertEqual(ceSess.json?.user?.capabilities?.canUploadMedia, true);

        const chiefSess = await chief.client.get("/api/v1/admin/editorial/session");
        c.assert.assertStatus(chiefSess, 200);
        c.assert.assertEqual(chiefSess.json?.user?.capabilities?.canWrite, true);
        c.assert.assertEqual(chiefSess.json?.user?.capabilities?.canPublish, true);
        c.assert.assertEqual(chiefSess.json?.user?.capabilities?.canDelete, true);

        const [metaRes, catsRes, tagsRes, mediaRes, settingsRes, redirRes, srcRes] =
          await Promise.all([
            ce.client.get("/api/v1/admin/editorial/meta"),
            ce.client.get("/api/v1/admin/editorial/categories"),
            ce.client.get("/api/v1/admin/editorial/tags"),
            ce.client.get("/api/v1/admin/editorial/media"),
            ce.client.get("/api/v1/admin/editorial/settings"),
            ce.client.get("/api/v1/admin/editorial/redirects"),
            ce.client.get("/api/v1/admin/editorial/research/sources"),
          ]);

        c.assert.assertStatus(metaRes, 200);
        c.assert.assertOk(Array.isArray(metaRes.json?.ssbTags), "meta.ssbTags must be array");
        c.assert.assertOk(Array.isArray(metaRes.json?.issues), "meta.issues must be array");
        c.assert.assertOk(Array.isArray(metaRes.json?.glossary), "meta.glossary must be array");

        c.assert.assertStatus(catsRes, 200);
        c.assert.assertOk(
          Array.isArray(catsRes.json?.categories) && catsRes.json.categories.length > 0,
          "editorial/categories must return non-empty array"
        );

        c.assert.assertStatus(tagsRes, 200);
        c.assert.assertOk(Array.isArray(tagsRes.json?.tags), "editorial/tags must return array");

        c.assert.assertStatus(mediaRes, 200);
        c.assert.assertOk(Array.isArray(mediaRes.json?.assets), "editorial/media.assets must be array");

        c.assert.assertStatus(settingsRes, 200);
        c.assert.assertOk(
          settingsRes.json?.settings && typeof settingsRes.json.settings === "object",
          "editorial/settings must return settings object"
        );

        c.assert.assertStatus(redirRes, 200);
        c.assert.assertOk(
          Array.isArray(redirRes.json?.redirects),
          "editorial/redirects must return array"
        );

        c.assert.assertStatus(srcRes, 200);
        c.assert.assertOk(
          Array.isArray(srcRes.json?.sources),
          "editorial/research/sources must return array"
        );
      }
    );

    suite.test(
      "Editorial article CRUD + refresh-stats + revisions list & restore work end-to-end",
      { req: "REQ-042" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const slug = uniqueSlug("t1-ed-art");

        const createRes = await sa.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `İlk Editör Başlığı ${slug}`,
            spot: "İlk spot özeti.",
            content: "<p>İlk paragraf içeriği.</p>",
            blocks: [
              { id: "b1", type: "paragraph", content: "İlk blok paragrafı." },
            ],
            status: "draft",
            slug,
            contentType: "haberler",
            ssbTags: ["hava-araclari"],
            note: "İlk taslak",
          },
        });
        c.assert.assertStatus(createRes, 200);
        c.assert.assertEqual(createRes.json?.success, true);
        c.assert.assertEqual(createRes.json?.wasInsert, true);
        const artId = createRes.json?.id;
        c.assert.assertOk(Boolean(artId), "Expected created article id");

        try {
          // Update article -> creates second revision
          const updRes = await sa.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: artId,
              title: `İkinci Revizyon Başlığı ${slug}`,
              spot: "Güncellenmiş spot özeti.",
              content: "<p>İkinci paragraf içeriği.</p>",
              status: "draft",
              slug,
              note: "İkinci revizyon",
            },
          });
          c.assert.assertStatus(updRes, 200);
          c.assert.assertEqual(updRes.json?.wasInsert, false);

          const getRes = await sa.client.get(`/api/v1/admin/editorial/articles/${artId}`);
          c.assert.assertStatus(getRes, 200);
          c.assert.assertEqual(getRes.json?.article?.title, `İkinci Revizyon Başlığı ${slug}`);

          const listRes = await sa.client.get("/api/v1/admin/editorial/articles", {
            query: { q: slug, status: "draft" },
          });
          c.assert.assertStatus(listRes, 200);
          c.assert.assertOk(
            (listRes.json?.articles || []).some((a) => String(a.id) === String(artId)),
            "Updated draft article must appear in editorial/articles search"
          );

          const statsRes = await sa.client.post(
            `/api/v1/admin/editorial/articles/${artId}/refresh-stats`,
            { json: {} }
          );
          c.assert.assertStatus(statsRes, 200);
          c.assert.assertEqual(statsRes.json?.success, true);
          c.assert.assertOk(typeof statsRes.json?.views === "number");

          const revsRes = await sa.client.get(
            `/api/v1/admin/editorial/articles/${artId}/revisions`
          );
          c.assert.assertStatus(revsRes, 200);
          const revisions = revsRes.json?.revisions || [];
          c.assert.assertOk(revisions.length >= 2, `Expected >=2 revisions, got ${revisions.length}`);

          const firstRev = revisions.find((r) => String(r.title || "").includes("İlk Editör Başlığı")) || revisions[revisions.length - 1];
          c.assert.assertOk(firstRev?.id > 0, "Expected valid revision id");

          const restoreRes = await sa.client.post(
            `/api/v1/admin/editorial/articles/${artId}/revisions/${firstRev.id}/restore`,
            { json: {} }
          );
          c.assert.assertStatus(restoreRes, 200);
          c.assert.assertEqual(restoreRes.json?.success, true);
          c.assert.assertEqual(
            restoreRes.json?.article?.title,
            `İlk Editör Başlığı ${slug}`,
            "Restoring first revision must revert article title"
          );
        } finally {
          await sa.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "Article lock acquire, list, refresh, and release lifecycle succeeds for editorial staff",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");
        const slug = uniqueSlug("t1-ed-lock");
        const cr = await ce.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Kilit Testi ${slug}`,
            content: "<p>Kilit testi içeriği.</p>",
            status: "draft",
            slug,
          },
        });
        c.assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        try {
          const acq = await ce.client.post(`/api/v1/admin/editorial/articles/${artId}/lock`, {
            json: {},
          });
          c.assert.assertStatus(acq, 200);
          c.assert.assertEqual(acq.json?.success, true);
          c.assert.assertEqual(acq.json?.conflict, false);
          c.assert.assertEqual(String(acq.json?.lock?.article_id ?? acq.json?.lock?.articleId), artId);

          const locksList = await ce.client.get("/api/v1/admin/editorial/locks");
          c.assert.assertStatus(locksList, 200);
          c.assert.assertOk(
            (locksList.json?.locks || []).some(
              (l) => String(l.article_id ?? l.articleId) === artId
            ),
            "Acquired lock must appear in GET /api/v1/admin/editorial/locks"
          );

          const rel = await ce.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`);
          c.assert.assertStatus(rel, 200);
          c.assert.assertEqual(rel.json?.success, true);

          const afterRel = await ce.client.get("/api/v1/admin/editorial/locks");
          c.assert.assertStatus(afterRel, 200);
          c.assert.assertEqual(
            (afterRel.json?.locks || []).some(
              (l) => String(l.article_id ?? l.articleId) === artId
            ),
            false,
            "Released lock must be removed from active locks"
          );
        } finally {
          const sa = await getCachedSuperAdmin(c);
          await sa.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "Editorial media upload (PNG & WebP -> /uploads/editorial/* 200), watermark-url, and delete",
      { req: "REQ-042" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const png = buildMinimalPng();
        const webp = fs.readFileSync(path.join(c.repoRoot, "public", "images", "msi", "kaan.webp"));

        const upPng = await sa.client.post("/api/v1/admin/editorial/upload?watermark=false", {
          multipart: {
            image: { content: png, filename: `ed-${uid("png")}.png`, contentType: "image/png" },
            alt: "Editorial PNG test",
          },
        });
        c.assert.assertStatus(upPng, 200);
        c.assert.assertEqual(upPng.json?.success, true);
        c.assert.assertIncludes(upPng.json?.url || "", "/uploads/editorial/");
        const pngAssetId = upPng.json?.asset?.id;
        c.assert.assertOk(pngAssetId > 0);

        const servePng = await c.http.createClient().raw("GET", upPng.json.url);
        c.assert.assertStatus(servePng, 200, "Uploaded editorial image must serve 200");
        c.assert.assertOk(servePng.rawBody.byteLength > 10);

        const upWebp = await sa.client.post("/api/v1/admin/editorial/upload?watermark=false", {
          multipart: {
            image: { content: webp, filename: `ed-${uid("webp")}.webp`, contentType: "image/webp" },
          },
        });
        c.assert.assertStatus(upWebp, 200);
        c.assert.assertIncludes(upWebp.json?.url || "", "/uploads/editorial/");
        const webpAssetId = upWebp.json?.asset?.id;

        // Watermark existing uploaded URL
        const wmRes = await sa.client.post("/api/v1/admin/editorial/media/watermark-url", {
          json: { url: upPng.json.url, altText: "Filigranlı görsel" },
        });
        c.assert.assertStatus(wmRes, 200);
        c.assert.assertEqual(wmRes.json?.watermarked, true);
        c.assert.assertIncludes(wmRes.json?.url || "", "/uploads/editorial/");
        const wmAssetId = wmRes.json?.asset?.id;

        for (const id of [pngAssetId, webpAssetId, wmAssetId]) {
          if (id) {
            const del = await sa.client.delete(`/api/v1/admin/editorial/media/${id}`);
            c.assert.assertStatus(del, 200);
          }
        }
      }
    );

    suite.test(
      "Research source CRUD (GET/POST/PUT/DELETE /api/v1/admin/editorial/research/sources)",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");
        const srcName = `E2E Defense RSS ${uid("rss")}`;
        const createRes = await ce.client.post("/api/v1/admin/editorial/research/sources", {
          json: {
            name: srcName,
            url: `https://rss.example.invalid/${uid("feed")}.xml`,
            category: "Savunma",
            scope: "tr",
          },
        });
        c.assert.assertStatus(createRes, 200);
        c.assert.assertEqual(createRes.json?.success, true);
        const srcId = createRes.json?.source?.id;
        c.assert.assertOk(srcId > 0, "Created research source must have positive id");

        try {
          const listRes = await ce.client.get("/api/v1/admin/editorial/research/sources");
          c.assert.assertStatus(listRes, 200);
          c.assert.assertOk(
            (listRes.json?.sources || []).some((s) => s.id === srcId && s.name === srcName)
          );

          const updRes = await ce.client.put(
            `/api/v1/admin/editorial/research/sources/${srcId}`,
            {
              json: {
                name: `${srcName} Updated`,
                enabled: false,
                scope: "global",
              },
            }
          );
          c.assert.assertStatus(updRes, 200);
          c.assert.assertEqual(updRes.json?.source?.name, `${srcName} Updated`);
          c.assert.assertEqual(updRes.json?.source?.enabled, false);
        } finally {
          const delRes = await ce.client.delete(
            `/api/v1/admin/editorial/research/sources/${srcId}`
          );
          c.assert.assertStatus(delRes, 200);
        }
      }
    );

    suite.test(
      "Scheduled article save (status=future, scheduledAt) & social settings GET/POST",
      { req: "REQ-042" },
      async (c) => {
        const chief = await getCachedRoleSession(c, "editor_in_chief");
        const slug = uniqueSlug("t1-ed-sched");
        const schedIso = "2029-05-15T10:30:00Z";

        const schedRes = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Zamanlanmış Haber ${slug}`,
            spot: "İleri tarihli savunma haberi.",
            content: "<p>Zamanlanmış içerik gövdesi.</p>",
            status: "future",
            scheduledAt: schedIso,
            slug,
          },
        });
        c.assert.assertStatus(schedRes, 200);
        c.assert.assertEqual(schedRes.json?.article?.status, "future");
        const artId = schedRes.json?.id;

        try {
          const origSettings = await chief.client.get("/api/v1/admin/editorial/settings");
          c.assert.assertStatus(origSettings, 200);
          const prevSocial = origSettings.json?.settings?.social_config || "{}";
          const prevWp = origSettings.json?.settings?.wp_config || "{}";

          const testSocialCfg = JSON.stringify({
            telegram: { botToken: "", chatId: "" },
            autoShareOnPublish: false,
            e2eMarker: uid("cfg"),
          });
          const saveCfg = await chief.client.post("/api/v1/admin/editorial/settings", {
            json: {
              wp_config: prevWp,
              social_config: testSocialCfg,
            },
          });
          c.assert.assertStatus(saveCfg, 200);

          const afterCfg = await chief.client.get("/api/v1/admin/editorial/settings");
          c.assert.assertStatus(afterCfg, 200);
          c.assert.assertIncludes(afterCfg.json?.settings?.social_config || "", "e2eMarker");

          // Restore previous settings
          await chief.client.post("/api/v1/admin/editorial/settings", {
            json: { wp_config: prevWp, social_config: prevSocial },
          });
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "Editorial categories and glossary CRUD succeed for editor_in_chief",
      { req: "REQ-042" },
      async (c) => {
        const chief = await getCachedRoleSession(c, "editor_in_chief");
        const catSlug = uniqueSlug("ed-cat");
        const catRes = await chief.client.post("/api/v1/admin/editorial/categories", {
          json: { name: `Stüdyo Kat ${catSlug}`, slug: catSlug },
        });
        c.assert.assertStatus(catRes, 200);
        const catId = catRes.json?.category?.id;
        c.assert.assertOk(catId > 0);

        const updCat = await chief.client.put(`/api/v1/admin/editorial/categories/${catId}`, {
          json: { name: `Güncel Stüdyo Kat ${catSlug}`, slug: catSlug },
        });
        c.assert.assertStatus(updCat, 200);

        const delCat = await chief.client.delete(`/api/v1/admin/editorial/categories/${catId}`);
        c.assert.assertStatus(delCat, 200);

        const termEn = `E2E Defense Term ${uid("ge")}`;
        const glRes = await chief.client.post("/api/v1/admin/editorial/glossary", {
          json: {
            id: 0,
            termTr: `E2E Terim ${uid("g")}`,
            termEn,
            category: "technical",
            ruleType: "fixed",
            preferredEn: termEn,
            forbiddenVariants: [],
            caseSensitive: false,
          },
        });
        c.assert.assertStatus(glRes, 200);
        const added = (glRes.json?.glossary || []).find((g) => g.termEn === termEn);
        c.assert.assertOk(added?.id > 0, "Expected created glossary rule in camelCase response");

        const delGl = await chief.client.delete(`/api/v1/admin/editorial/glossary/${added.id}`);
        c.assert.assertStatus(delGl, 200);
      }
    );
  });
}
