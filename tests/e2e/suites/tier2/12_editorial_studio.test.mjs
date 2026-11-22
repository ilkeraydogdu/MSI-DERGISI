import {
  getCachedRoleSession,
  getCachedSuperAdmin,
  uid,
  uniqueSlug,
} from "../../fixtures/tier12_helpers.mjs";

const NON_EDITORIAL_STAFF_ROLES = [
  "ads_manager",
  "finance_viewer",
  "support_agent",
  "business_dev",
];

export function registerTests(suite) {
  suite.describe("REQ-042 Editorial Studio Boundaries, RBAC, CSRF, Lock Conflicts & SSRF", () => {
    suite.test(
      "unauthenticated requests to /podmin/editor -> 303 redirect and /api/v1/admin/editorial/session -> 401",
      { req: "REQ-042", readOnly: true },
      async (c) => {
        const guest = c.http.createClient();
        for (const [p, expectedLoc] of [
          ["/podmin/editor", "/podmin"],
          [`/${c.adminPath}/editor`, `/${c.adminPath}`],
        ]) {
          const pageRes = await guest.get(p, { redirect: "manual" });
          c.assert.assertStatus(pageRes, 303, `Unauthenticated GET ${p} must return 303 redirect`);
          c.assert.assertEqual(pageRes.headers.get("location"), expectedLoc);
        }

        const jsRes = await guest.get(`/${c.adminPath}/editor/studio.js`, { redirect: "manual" });
        c.assert.assertStatus(jsRes, 303, "Unauthenticated studio.js must redirect");

        const apiRes = await guest.get("/api/v1/admin/editorial/session");
        c.assert.assertStatus(apiRes, 401, "Unauthenticated editorial/session must return 401");
      }
    );

    suite.test(
      "non-contents:write staff roles receive 403 SSR forbidden page on /podmin/editor and 403 JSON on editorial APIs",
      { req: "REQ-042" },
      async (c) => {
        for (const role of NON_EDITORIAL_STAFF_ROLES) {
          const actor = await getCachedRoleSession(c, role);
          const pageRes = await actor.client.get("/podmin/editor", { redirect: "manual" });
          c.assert.assertStatus(pageRes, 403, `Staff role ${role} must get 403 on /podmin/editor`);
          c.assert.assertIncludes(pageRes.text, "msi-studio-forbidden");
          c.assert.assertIncludes(pageRes.text, "contents:write");

          const wasmRes = await actor.client.raw("GET", "/podmin/editor/studio.wasm", {
            redirect: "manual",
          });
          c.assert.assertStatus(wasmRes, 403, `Staff role ${role} must get 403 on studio.wasm`);

          const apiRes = await actor.client.get("/api/v1/admin/editorial/session");
          c.assert.assertStatus(apiRes, 403, `Staff role ${role} must get 403 on editorial/session`);
        }
      }
    );

    suite.test(
      "all 5 READER_ROLES receive 303 on /podmin/editor and 403 on /api/v1/admin/editorial/*",
      { req: "REQ-042" },
      async (c) => {
        for (const role of c.auth.READER_ROLES) {
          const reader = await getCachedRoleSession(c, role);
          const pageRes = await reader.client.get("/podmin/editor", { redirect: "manual" });
          c.assert.assertStatus(pageRes, 303, `Reader role ${role} must get 303 on /podmin/editor`);

          const apiRes = await reader.client.get("/api/v1/admin/editorial/articles");
          c.assert.assertStatus(apiRes, 403, `Reader role ${role} must get 403 on editorial/articles`);
        }
      }
    );

    suite.test(
      "content_editor is denied contents:publish, contents:delete, media:delete, social/share, and settings write (403)",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");
        const chief = await getCachedRoleSession(c, "editor_in_chief");
        const slug = uniqueSlug("t2-ed-rbac");

        // Create draft as content_editor
        const draftRes = await ce.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Yetki Sınırı Taslağı ${slug}`,
            content: "<p>Taslak gövde.</p>",
            status: "draft",
            slug,
          },
        });
        c.assert.assertStatus(draftRes, 200);
        const draftId = draftRes.json.id;

        try {
          // 1. Cannot publish
          const pubAttempt = await ce.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: draftId,
              title: `Yetki Sınırı Taslağı ${slug}`,
              content: "<p>İzinsiz yayın.</p>",
              status: "published",
              slug,
            },
          });
          c.assert.assertStatus(pubAttempt, 403, "content_editor must get 403 on status=published");

          // 2. Cannot schedule
          const schedAttempt = await ce.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: draftId,
              title: `Yetki Sınırı Taslağı ${slug}`,
              content: "<p>İzinsiz zamanlama.</p>",
              status: "scheduled",
              scheduledAt: "2030-01-01T10:00:00Z",
              slug,
            },
          });
          c.assert.assertStatus(schedAttempt, 403, "content_editor must get 403 on status=scheduled");

          // 3. Cannot delete article
          const delAttempt = await ce.client.delete(`/api/v1/admin/editorial/articles/${draftId}`);
          c.assert.assertStatus(delAttempt, 403, "content_editor must get 403 on DELETE article");

          // 4. Cannot trigger social share or save editorial settings
          const shareAttempt = await ce.client.post("/api/v1/admin/editorial/social/share", {
            json: { articleId: draftId, platforms: ["telegram"] },
          });
          c.assert.assertStatus(shareAttempt, 403, "content_editor must get 403 on social/share");

          const settingsAttempt = await ce.client.post("/api/v1/admin/editorial/settings", {
            json: { social_config: "{}" },
          });
          c.assert.assertStatus(settingsAttempt, 403, "content_editor must get 403 on settings write");

          // 5. Cannot overwrite an already published article even with status=draft
          const pubSlug = uniqueSlug("t2-ed-pub-prot");
          const chiefPub = await chief.client.post("/api/v1/admin/editorial/articles", {
            json: {
              id: "0",
              title: `Yayınlı Koruma ${pubSlug}`,
              content: "<p>Yayınlı içerik.</p>",
              status: "publish",
              slug: pubSlug,
            },
          });
          c.assert.assertStatus(chiefPub, 200);
          const pubId = chiefPub.json.id;
          try {
            const overwriteAttempt = await ce.client.post("/api/v1/admin/editorial/articles", {
              json: {
                id: pubId,
                title: "Yetkisiz Değişiklik",
                content: "<p>Değiştirildi</p>",
                status: "draft",
                slug: pubSlug,
              },
            });
            c.assert.assertStatus(
              overwriteAttempt,
              403,
              "content_editor must be blocked from overwriting published article"
            );
          } finally {
            await chief.client.delete(`/api/v1/admin/editorial/articles/${pubId}`).catch(() => {});
          }
        } finally {
          await chief.client.delete(`/api/v1/admin/editorial/articles/${draftId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "POST /api/v1/admin/editorial/articles enforces CSRF token (403 on missing/forged X-CSRF-Token)",
      { req: "REQ-042" },
      async (c) => {
        const sa = await getCachedSuperAdmin(c);
        const slug = uniqueSlug("t2-ed-csrf");

        const forged = await sa.client.post("/api/v1/admin/editorial/articles", {
          headers: { "x-csrf-token": "forged-editorial-csrf-token" },
          json: {
            id: "0",
            title: "CSRF Test",
            content: "<p>Test</p>",
            status: "draft",
            slug,
          },
        });
        c.assert.assertStatus(forged, 403, "Forged X-CSRF-Token must return 403");

        const missing = await sa.client.post("/api/v1/admin/editorial/articles", {
          csrf: false,
          json: {
            id: "0",
            title: "CSRF Missing Test",
            content: "<p>Test</p>",
            status: "draft",
            slug,
          },
        });
        c.assert.assertStatus(missing, 403, "Missing X-CSRF-Token must return 403");
      }
    );

    suite.test(
      "Article lock enforces single-owner conflict (conflict: true on acquire, 409 on non-owner release)",
      { req: "REQ-042" },
      async (c) => {
        const userA = await c.auth.createUserWithRole("content_editor");
        const userB = await c.auth.createUserWithRole("editor_in_chief");
        const slug = uniqueSlug("t2-ed-lock");

        const cr = await userA.client.post("/api/v1/admin/editorial/articles", {
          json: {
            id: "0",
            title: `Kilit Çakışması ${slug}`,
            content: "<p>Test</p>",
            status: "draft",
            slug,
          },
        });
        c.assert.assertStatus(cr, 200);
        const artId = String(cr.json.id);

        try {
          const lockA = await userA.client.post(`/api/v1/admin/editorial/articles/${artId}/lock`, {
            json: {},
          });
          c.assert.assertStatus(lockA, 200);
          c.assert.assertEqual(lockA.json?.conflict, false);

          const lockB = await userB.client.post(`/api/v1/admin/editorial/articles/${artId}/lock`, {
            json: {},
          });
          c.assert.assertStatus(lockB, 200);
          c.assert.assertEqual(lockB.json?.conflict, true, "User B must see lock conflict");
          c.assert.assertEqual(
            Number(lockB.json?.lockedBy?.user_id ?? lockB.json?.lockedBy?.userId),
            Number(userA.id),
            "lockedBy must identify user A"
          );

          const delByB = await userB.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`);
          c.assert.assertStatus(delByB, 409, "Non-owner DELETE lock must return 409 Conflict");

          const delByA = await userA.client.delete(`/api/v1/admin/editorial/articles/${artId}/lock`);
          c.assert.assertStatus(delByA, 200);
        } finally {
          await userB.client.delete(`/api/v1/admin/editorial/articles/${artId}`).catch(() => {});
        }
      }
    );

    suite.test(
      "SSRF guard blocks loopback, link-local metadata, IPv6 ::1, and non-HTTP schemes across fetch-remote-image & research endpoints",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");

        for (const badUrl of [
          "http://127.0.0.1/secret.png",
          "http://localhost:8090/secret.png",
          "http://169.254.169.254/latest/meta-data/",
          "http://[::1]/secret.png",
        ]) {
          const imgRes = await ce.client.post("/api/v1/admin/editorial/fetch-remote-image", {
            json: { url: badUrl, watermark: false },
          });
          c.assert.assertStatus(
            imgRes,
            [400, 403],
            `fetch-remote-image must block SSRF URL ${badUrl}`
          );

          const srcRes = await ce.client.post("/api/v1/admin/editorial/research/sources", {
            json: { name: "SSRF Test", url: badUrl, category: "Test", scope: "tr" },
          });
          c.assert.assertStatus(
            srcRes,
            [400, 403],
            `research/sources must block SSRF URL ${badUrl}`
          );

          const readRes = await ce.client.post("/api/v1/admin/editorial/research/read", {
            json: { url: badUrl },
          });
          c.assert.assertStatus(
            readRes,
            [400, 403],
            `research/read must block SSRF URL ${badUrl}`
          );
        }

        const fileImg = await ce.client.post("/api/v1/admin/editorial/fetch-remote-image", {
          json: { url: "file:///etc/passwd", watermark: false },
        });
        c.assert.assertStatus(fileImg, [400, 403]);
        const fileRead = await ce.client.post("/api/v1/admin/editorial/research/read", {
          json: { url: "file:///etc/passwd" },
        });
        c.assert.assertStatus(fileRead, [400, 403]);
      }
    );

    suite.test(
      "Editorial upload rejects spoofed magic bytes and empty payload with 400",
      { req: "REQ-042" },
      async (c) => {
        const ce = await getCachedRoleSession(c, "content_editor");

        const spoofed = await ce.client.post("/api/v1/admin/editorial/upload", {
          multipart: {
            image: {
              content: Buffer.from("MZ_EXECUTABLE_SPOOFED_AS_PNG"),
              filename: "fake.png",
              contentType: "image/png",
            },
          },
        });
        c.assert.assertStatus(spoofed, 400, "Spoofed image magic bytes must return 400");

        const empty = await ce.client.post("/api/v1/admin/editorial/upload", {
          multipart: {
            image: {
              content: Buffer.alloc(0),
              filename: "empty.png",
              contentType: "image/png",
            },
          },
        });
        c.assert.assertStatus(empty, 400, "Empty upload must return 400");
      }
    );

    suite.test(
      "Invalid article IDs, missing article, and empty title return clean 400/404 without 500",
      { req: "REQ-042" },
      async (c) => {
        const chief = await getCachedRoleSession(c, "editor_in_chief");

        const badId = await chief.client.get("/api/v1/admin/editorial/articles/not-a-number");
        c.assert.assertStatus(badId, 400);

        const missingId = await chief.client.get("/api/v1/admin/editorial/articles/99999999");
        c.assert.assertStatus(missingId, 404);

        const emptyTitle = await chief.client.post("/api/v1/admin/editorial/articles", {
          json: { id: "0", title: "   ", content: "<p>Body</p>", status: "draft" },
        });
        c.assert.assertStatus(emptyTitle, 400);

        const badWm = await chief.client.post("/api/v1/admin/editorial/media/watermark-url", {
          json: { url: "/etc/passwd" },
        });
        c.assert.assertStatus(badWm, [400, 404]);
      }
    );
  });
}
