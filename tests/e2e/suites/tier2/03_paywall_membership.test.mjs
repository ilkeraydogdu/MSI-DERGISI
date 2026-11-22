import {
  antiBotFields,
  formTs,
  getCachedSuperAdmin,
  makeContentPayload,
  uid,
  uniqueEmail,
  uniqueSlug,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-013 Boundaries ────────────────────────────────────────────────────
  suite.describe("REQ-013 Paywall Visibility Boundaries", () => {
    suite.test("anonymous user cannot bypass visibility_level=2 via query params", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-bypass");
      const marker = `SECRET_PAYWALL_${uid("p")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Kilitli ${slug}`,
          body_tr: `<p>Acilis.</p><p>Devam.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`, {
        query: { preview: "1", unlock: "true", bypass: "1" },
      });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("registered user cannot read visibility_level=2 content", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-reg-lvl2");
      const marker = `LVL2_ONLY_${uid("l2")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Seviye 2 ${slug}`,
          body_tr: `<p>Ilk.</p><p>Ikinci.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const reg = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const res = await reg.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("subscriber_digital cannot read unpublished draft article via /api/v1/contents/:slug", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-sub-draft");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({ slug, title_tr: `Taslak ${slug}`, status: "draft" }),
      });
      c.assert.assertStatus(cr, 200);
      const sub = await c.auth.createUserWithRole("subscriber_digital", { superAdminClient: sa.client });
      const res = await sub.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 404);
    });

    suite.test("visibility_level=2 premium body does not leak into /rss.xml syndication feed", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-rss-gate");
      const marker = `RSS_GATED_SECRET_${uid("r")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `RSS Kilitli ${slug}`,
          body_tr: `<p>Acilis.</p><p>Ikinci.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const rss = await c.http.createClient().get("/rss.xml");
      c.assert.assertStatus(rss, 200);
      c.assert.assertNotIncludes(rss.text, marker);
    });

    suite.test("deleted user session cannot unlock visibility_level=1 content", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-del-vis1");
      const marker = `DEL_LOCKED_${uid("dl")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Silinen Okur ${slug}`,
          body_tr: `<p>Bir.</p><p>Iki.</p><p>${marker}</p>`,
          visibility_level: 1,
        }),
      });
      c.assert.assertStatus(cr, 200);
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      await reg.client.post("/api/v1/account/delete", { json: { password: reg.password } });
      const res = await reg.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", marker);
    });
  });

  // ── REQ-014 Boundaries: Anonymous Teaser Truncation ───────────────────────
  suite.describe("REQ-014 Teaser Truncation Boundaries", () => {
    suite.test("single-paragraph multi-sentence gated article truncates later sentences", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-1para");
      const secret = `TAIL_SENTENCE_SECRET_${uid("t")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Tek Paragraf ${slug}`,
          body_tr: `<p>Birinci cumle burada. Ikinci cumle devam ediyor. Ucuncu cumle detay verir. Dorduncu cumlede ${secret} yer alir.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", secret);
    });

    suite.test("short 1-sentence gated article handles teaser boundary without crashing", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-short");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Kisa Kilitli ${slug}`,
          body_tr: "<p>Tek kisa cumle.</p>",
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
    });

    suite.test("anonymous SSR /icerik/:slug never renders unclosed script or secret tail for gated article", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-ssr-tail");
      const secret = `SSR_TAIL_LOCK_${uid("st")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSR Tail ${slug}`,
          body_tr: `<p>Birinci.</p><p>Ikinci.</p><p>Ucuncu ${secret}.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, secret);
    });

    suite.test("GET /api/v1/contents hides gated body tail even when filtered by category", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-cat-tail");
      const secret = `CAT_TAIL_SECRET_${uid("ct")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Cat Tail ${slug}`,
          body_tr: `<p>Acilis.</p><p>Ikinci.</p><p>Ucuncu.</p><p>${secret}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/contents", {
        query: { category: "makaleler-analizler", per_page: "100" },
      });
      c.assert.assertStatus(res, 200);
      const item = res.json.find((x) => x.slug === slug);
      c.assert.assertOk(item, `Expected created article ${slug} in /api/v1/contents?category=makaleler-analizler`);
      c.assert.assertNotIncludes(item.body_tr || "", secret);
    });

    suite.test("GET /api/v1/search?q=... hides gated body tail for anonymous visitor", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t2-srch-tail");
      const secret = `SRCH_TAIL_SECRET_${uid("srt")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Srch Tail ${slug}`,
          body_tr: `<p>Bir.</p><p>Iki.</p><p>${secret}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/search", { query: { q: slug } });
      c.assert.assertStatus(res, 200);
      const item = (res.json?.items || []).find((x) => x.slug === slug);
      c.assert.assertOk(item);
      c.assert.assertNotIncludes(item.body_tr || "", secret);
    });
  });

  // ── REQ-015 Boundaries: Registration Validation & Anti-Bot ────────────────
  suite.describe("REQ-015 Registration Validation & Anti-Bot Boundaries", () => {
    suite.test("POST /api/v1/auth/register rejects kvkk_consent=false with 400", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({
        allowError: true,
        kvkk_consent: false,
        ...antiBotFields(),
      });
      c.assert.assertStatus(reg.res, 400);
    });

    suite.test("POST /api/v1/auth/register rejects tos_version with tos_accepted=false (400)", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({
        allowError: true,
        tos_version: "2026-01",
        tos_accepted: false,
        ...antiBotFields(),
      });
      c.assert.assertStatus(reg.res, 400);
    });

    suite.test("POST /api/v1/auth/register rejects weak password (<8 chars) with 400", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({
        allowError: true,
        password: "short",
        ...antiBotFields(),
      });
      c.assert.assertStatus(reg.res, 400);
    });

    suite.test("POST /api/v1/auth/register rejects duplicate email with 409 Conflict", { req: "REQ-015" }, async (c) => {
      const email = uniqueEmail("dup");
      await c.auth.registerUser({ email, ...antiBotFields() });
      const dup = await c.auth.registerUser({ email, allowError: true, ...antiBotFields() });
      c.assert.assertStatus(dup.res, 409);
    });

    suite.test("POST /api/v1/auth/register blocks fast _form_ts timing violation with 429", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({
        allowError: true,
        website_url: "",
        _form_ts: formTs(0), // 0s elapsed < 2s minimum
      });
      c.assert.assertStatus(reg.res, 429);
    });
  });

  // ── REQ-016 Boundaries: Gov Auto-Verify & Corporate Pool ──────────────────
  suite.describe("REQ-016 Gov Verify & Corporate Pool Boundaries", () => {
    suite.test("lookalike domain evilgov.tr does NOT auto-verify as subscriber_public", { req: "REQ-016" }, async (c) => {
      const reg = await c.auth.registerUser({
        email: uniqueEmail("fake", "evilgov.tr"),
        organization_category: "MSB",
        ...antiBotFields(),
      });
      c.assert.assertEqual(reg.govAutoSubscribed, false);
      c.assert.assertEqual(reg.user?.role, "registered");
      c.assert.assertEqual(reg.user?.is_gov_verified, false);
    });

    suite.test("free webmail claiming Sektör Kuruluşları queues verification without auto-grant", { req: "REQ-016" }, async (c) => {
      const reg = await c.auth.registerUser({
        email: uniqueEmail("sec", "gmail.com"),
        organization_category: "Sektör Kuruluşları",
        organization_name: "Baykar",
        request_message: "Sektor calisaniyim",
        ...antiBotFields(),
      });
      c.assert.assertEqual(reg.govAutoSubscribed, false);
      c.assert.assertEqual(reg.corporateVerificationQueued, true);
      c.assert.assertEqual(reg.user?.role, "registered");
    });

    suite.test("POST /api/v1/admin/corporate-requests/:id/decide rejects invalid status with 400", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/corporate-requests/1/decide", {
        json: { status: "granted_forever" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("POST /api/v1/admin/corporate-requests/decide rejects missing/zero id with 400", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/corporate-requests/decide", {
        json: { id: 0, status: "approved" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("content_editor is denied GET /api/v1/admin/corporate-requests with 403", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ed = await c.auth.createUserWithRole("content_editor", { superAdminClient: sa.client });
      const res = await ed.client.get("/api/v1/admin/corporate-requests");
      c.assert.assertStatus(res, 403);
    });
  });

  // ── REQ-017 Boundaries: Auth, 2FA, Password Reset & Verify ────────────────
  suite.describe("REQ-017 Auth, 2FA & Token Boundaries", () => {
    suite.test("POST /api/v1/auth/login rejects wrong password with 401 on disposable user", { req: "REQ-017" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const res = await c.http.createClient().post("/api/v1/auth/login", {
        json: { email: reg.email, password: "WrongPassword!999", ...antiBotFields() },
      });
      c.assert.assertStatus(res, 401);
    });

    suite.test("POST /api/v1/auth/totp/login rejects tampered temp_token with 401", { req: "REQ-017" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/auth/totp/login", {
        json: { temp_token: "tampered.invalid.token", code: "123456" },
      });
      c.assert.assertStatus(res, 401);
    });

    suite.test("POST /api/v1/auth/totp/disable rejects wrong password and wrong code with 401", { req: "REQ-017" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      await c.auth.enableTotp(u.client);
      const res = await u.client.post("/api/v1/auth/totp/disable", {
        json: { password: "WrongPassword!000" },
      });
      c.assert.assertStatus(res, 401);
    });

    suite.test("POST /api/v1/auth/reset-password rejects invalid/expired token with 400", { req: "REQ-017" }, async (c) => {
      const res = await c.http.createClient().post("/api/v1/auth/reset-password", {
        json: { token: "nonexistent-reset-token-123", new_password: "NewValidPassword!2026" },
      });
      c.assert.assertStatus(res, 400);
    });

    suite.test("GET /hesap-dogrula?token=bad&confirm=1 rejects invalid token with 400", { req: "REQ-017" }, async (c) => {
      const res = await c.http.createClient().get("/hesap-dogrula", {
        query: { token: "invalid-verify-token-999", confirm: "1" },
      });
      c.assert.assertStatus(res, 400);
    });
  });
}
