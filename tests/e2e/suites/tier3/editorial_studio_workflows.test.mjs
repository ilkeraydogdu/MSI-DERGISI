import http from "node:http";
import {
  makeTinyPngPart,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 9: Editorial Studio Cross-Feature Workflows", () => {
    suite.test(
      "T3-55: content_editor block draft + lock conflict -> unlock -> editor_in_chief publish -> SSR /icerik/:slug + revision restore reverts SSR",
      { req: ["REQ-042", "REQ-009", "REQ-035"] },
      async (c) => {
        const ce = await c.auth.createUserWithRole("content_editor");
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const slug = `t3-ed-${uid("wf")}`;
        const v1Text = `İlk sürüm savunma analizi ${uid("v1")}`;
        const v2Text = `İkinci sürüm güncel rapor ${uid("v2")}`;

        const cr = await ce.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `T3 Stüdyo Makale V1 ${slug}`,
            spot: "Özet spot V1",
            blocks: [
              { id: "b1", type: "paragraph", content: v1Text },
            ],
            status: "draft",
            slug,
            contentType: "haberler",
            ssbTags: ["hava-araclari"],
            note: "Revizyon 1",
          },
        });
        assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        try {
          // 1. content_editor acquires lock; editor_in_chief sees conflict
          const ceLock = await ce.client.post(`/api/v1/admin/editorial/articles/${artId}/lock`, {
            json: {},
          });
          assert.assertStatus(ceLock, 200);
          assert.assertEqual(ceLock.json?.conflict, false);

          const chiefConflict = await chief.client.post(
            `/api/v1/admin/editorial/articles/${artId}/lock`,
            { json: {} }
          );
          assert.assertStatus(chiefConflict, 200);
          assert.assertEqual(chiefConflict.json?.conflict, true);

          // 2. content_editor submits pending_review & unlocks
          const subRes = await ce.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: artId,
              title: `T3 Stüdyo Makale V1 ${slug}`,
              spot: "Özet spot V1",
              blocks: [{ id: "b1", type: "paragraph", content: v1Text }],
              status: "pending_review",
              slug,
            },
          });
          assert.assertStatus(subRes, 200);
          const relRes = await ce.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`);
          assert.assertStatus(relRes, 200);

          // 3. editor_in_chief acquires lock & publishes V1 -> verify SSR /icerik/:slug
          const chiefLock = await chief.client.post(
            `/api/v1/admin/editorial/articles/${artId}/lock`,
            { json: {} }
          );
          assert.assertStatus(chiefLock, 200);
          assert.assertEqual(chiefLock.json?.conflict, false);

          const pub1 = await chief.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: artId,
              title: `T3 Stüdyo Makale V1 ${slug}`,
              spot: "Özet spot V1",
              blocks: [{ id: "b1", type: "paragraph", content: v1Text }],
              status: "publish",
              slug,
              note: "İlk yayın",
            },
          });
          assert.assertStatus(pub1, 200);

          const pubClient = c.http.createClient();
          const ssr1 = await pubClient.get(`/icerik/${slug}`);
          assert.assertStatus(ssr1, 200, "Published Editorial Studio article must render 200 on /icerik/:slug");
          assert.assertIncludes(ssr1.text, v1Text);

          // 4. Publish V2 with updated body -> verify SSR updated -> restore V1 revision -> verify SSR reverted
          const pub2 = await chief.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: artId,
              title: `T3 Stüdyo Makale V2 ${slug}`,
              spot: "Özet spot V2",
              blocks: [{ id: "b1", type: "paragraph", content: v2Text }],
              status: "publish",
              slug,
              note: "İkinci yayın",
            },
          });
          assert.assertStatus(pub2, 200);

          const ssr2 = await pubClient.get(`/icerik/${slug}`);
          assert.assertStatus(ssr2, 200);
          assert.assertIncludes(ssr2.text, v2Text);

          const revs = await chief.client.get(`/api/v1/admin/editorial/articles/${artId}/revisions`);
          assert.assertStatus(revs, 200);
          const v1Rev = (revs.json?.revisions || []).find((r) =>
            String(r.title || "").includes("V1")
          );
          assert.assertOk(v1Rev?.id > 0, "Expected V1 revision snapshot");

          const restoreRes = await chief.client.post(
            `/api/v1/admin/editorial/articles/${artId}/revisions/${v1Rev.id}/restore`,
            { json: {} }
          );
          assert.assertStatus(restoreRes, 200);

          const ssrRestored = await pubClient.get(`/icerik/${slug}`);
          assert.assertStatus(ssrRestored, 200);
          assert.assertIncludes(ssrRestored.text, v1Text, "Restored revision must revert SSR article body");
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`).catch(() => {});
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-56: Editorial Studio upload + watermark-url -> cover image attached -> /uploads/editorial/* serves 200 on public SSR article",
      { req: ["REQ-042", "REQ-011"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const pngPart = makeTinyPngPart(`cover-${uid("ed")}.png`);

        const upRes = await chief.client.post("/api/v1/admin/editorial/upload?watermark=false", {
          multipart: { image: pngPart, alt: "KAAN Kapak" },
        });
        assert.assertStatus(upRes, 200);
        const rawUrl = upRes.json.url;
        const rawAssetId = upRes.json.asset.id;

        const wmRes = await chief.client.post("/api/v1/admin/editorial/media/watermark-url", {
          json: { url: rawUrl, altText: "KAAN Filigranlı Kapak" },
        });
        assert.assertStatus(wmRes, 200);
        assert.assertEqual(wmRes.json?.watermarked, true);
        const wmUrl = wmRes.json.url;
        const wmAssetId = wmRes.json.asset.id;

        const slug = `t3-ed-media-${uid("m")}`;
        const artRes = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Görselli Haber ${slug}`,
            spot: "Filigranlı kapak görselli haber.",
            content: "<p>Görsel entegrasyon testi.</p>",
            imageUrl: wmUrl,
            status: "publish",
            slug,
          },
        });
        assert.assertStatus(artRes, 200);
        const artId = artRes.json.id;

        try {
          const pubClient = c.http.createClient();
          const ssrRes = await pubClient.get(`/icerik/${slug}`);
          assert.assertStatus(ssrRes, 200);
          assert.assertIncludes(ssrRes.text, wmUrl, "Public SSR article must reference watermarked cover URL");

          const imgFetch = await pubClient.raw("GET", wmUrl);
          assert.assertStatus(imgFetch, 200, "Watermarked /uploads/editorial/* asset must return 200 publicly");
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
          await chief.client.delete(`/api/v1/admin/editorial/media/${rawAssetId}`).catch(() => {});
          await chief.client.delete(`/api/v1/admin/editorial/media/${wmAssetId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-57: Research desk source CRUD + research/search + SSRF-safe research/read end-to-end",
      { req: ["REQ-042", "REQ-033"] },
      async (c) => {
        const ce = await c.auth.createUserWithRole("content_editor");
        const localRssStub = http.createServer((req, res) => {
          res.writeHead(200, { "Content-Type": "application/rss+xml; charset=utf-8" });
          res.end("<rss version='2.0'><channel><title>Local</title></channel></rss>");
        });
        await new Promise((r) => localRssStub.listen(0, "127.0.0.1", r));
        const stubPort = localRssStub.address().port;

        let createdSourceId = null;
        try {
          // Loopback RSS stub must be rejected by SSRF guard on research/sources & research/read
          const ssrfSrc = await ce.client.post("/api/v1/admin/editorial/research/sources", {
            json: { name: "Loopback RSS", url: `http://127.0.0.1:${stubPort}/rss.xml` },
          });
          assert.assertStatus(ssrfSrc, 400, "Loopback RSS URL must be rejected by SSRF guard");

          const ssrfRead = await ce.client.post("/api/v1/admin/editorial/research/read", {
            json: { url: `http://127.0.0.1:${stubPort}/article` },
          });
          assert.assertStatus(ssrfRead, 400, "Loopback article URL must be rejected by SSRF guard");

          // Valid public-syntax source persists cleanly
          const srcCreate = await ce.client.post("/api/v1/admin/editorial/research/sources", {
            json: {
              name: `Savunma Bülteni ${uid("src")}`,
              url: "https://rss.example.invalid/savunma.xml",
              category: "Savunma",
              scope: "tr",
            },
          });
          assert.assertStatus(srcCreate, 200);
          createdSourceId = srcCreate.json?.source?.id;
          assert.assertOk(createdSourceId > 0);

          const searchRes = await ce.client.get("/api/v1/admin/editorial/research/search", {
            query: { q: "savunma", scope: "tr", time: "24h", source_id: String(createdSourceId) },
          });
          assert.assertStatus(searchRes, 200);
          assert.assertEqual(searchRes.json?.success, true);
          assert.assertOk(Array.isArray(searchRes.json?.items), "research/search must return items array");
        } finally {
          if (createdSourceId) {
            await ce.client
              .delete(`/api/v1/admin/editorial/research/sources/${createdSourceId}`)
              .catch(() => {});
          }
          await new Promise((r) => localRssStub.close(r));
        }
      }
    );

    suite.test(
      "T3-58: Social test-twitter, test-telegram & social/share RBAC and validation contracts",
      { req: ["REQ-042"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const ce = await c.auth.createUserWithRole("content_editor");
        const slug = `t3-ed-soc-${uid("s")}`;

        const artRes = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Sosyal Paylaşım Haberi ${slug}`,
            spot: "Sosyal medya test özeti",
            content: "<p>Sosyal medya paylaşım içeriği.</p>",
            status: "publish",
            slug,
          },
        });
        assert.assertStatus(artRes, 200);
        const artId = String(artRes.json.id);

        try {
          // Incomplete Twitter creds -> 400
          const badTw = await chief.client.post("/api/v1/admin/editorial/social/test-twitter", {
            json: { apiKey: "only-key" },
          });
          assert.assertStatus(badTw, 400, "Incomplete Twitter credentials must return 400");

          // Empty Telegram creds -> 200 { success: false }
          const emptyTg = await chief.client.post("/api/v1/admin/editorial/social/test-telegram", {
            json: { botToken: "", chatId: "" },
          });
          assert.assertStatus(emptyTg, 200);
          assert.assertEqual(emptyTg.json?.success, false);

          // content_editor cannot trigger social endpoints (403)
          const ceShare = await ce.client.post("/api/v1/admin/editorial/social/share", {
            json: { articleId: artId, platforms: ["telegram", "twitter"] },
          });
          assert.assertStatus(ceShare, 403);

          // editor_in_chief can invoke social/share cleanly (200 JSON results map)
          const chiefShare = await chief.client.post("/api/v1/admin/editorial/social/share", {
            json: { articleId: artId, platforms: ["telegram", "twitter"] },
          });
          assert.assertStatus(chiefShare, 200);
          assert.assertEqual(chiefShare.json?.success, true);
          assert.assertOk(
            chiefShare.json?.results && typeof chiefShare.json.results === "object",
            "social/share must return results object"
          );
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "T3-59: Editorial Studio slug change auto-creates 301 seo_redirect and serves 301 from old slug to new slug",
      { req: ["REQ-042", "REQ-035"] },
      async (c) => {
        const chief = await c.auth.createUserWithRole("editor_in_chief");
        const oldSlug = `t3-ed-old-${uid("r")}`;
        const newSlug = `t3-ed-new-${uid("r")}`;

        const cr = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Yönlendirme Eski Başlık ${oldSlug}`,
            content: "<p>İlk içerik.</p>",
            status: "publish",
            slug: oldSlug,
          },
        });
        assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        try {
          const upd = await chief.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: artId,
              title: `Yönlendirme Yeni Başlık ${newSlug}`,
              content: "<p>Güncel içerik.</p>",
              status: "publish",
              slug: newSlug,
            },
          });
          assert.assertStatus(upd, 200);

          const redirList = await chief.client.get("/api/v1/admin/editorial/redirects");
          assert.assertStatus(redirList, 200);
          const createdRule = (redirList.json?.redirects || []).find(
            (r) =>
              String(r.old_slug ?? r.oldSlug ?? r.old_path ?? "").includes(oldSlug) &&
              String(r.new_slug ?? r.newSlug ?? r.new_path ?? "").includes(newSlug)
          );
          assert.assertOk(createdRule, "Changing published slug in Editorial Studio must auto-create 301 redirect");

          const pubClient = c.http.createClient();
          const redirRes = await pubClient.get(`/icerik/${oldSlug}`, { redirect: "manual" });
          assert.assertStatus(redirRes, [301, 308]);
          assert.assertIncludes(redirRes.headers.get("location") || "", newSlug);

          if (createdRule?.id) {
            await chief.client.delete(`/api/v1/admin/editorial/redirects/${createdRule.id}`).catch(() => {});
          }
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );
  });
}
