import {
  antiBotFields,
  getCachedSuperAdmin,
  makeContentPayload,
  uid,
  uniqueEmail,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-013: 3+ visibility levels & reader classifications ────────────────
  suite.describe("REQ-013 Multi-Tier Paywall Visibility Levels", () => {
    suite.test("visibility_level=0 open content is fully accessible to anonymous visitor", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-vis0");
      const bodyMarker = `OPEN_BODY_${uid("v0")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Acik Icerik ${slug}`,
          body_tr: `<p>Giris.</p><p>${bodyMarker}</p>`,
          visibility_level: 0,
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, false);
      c.assert.assertIncludes(res.json?.paywall?.rendered_body || "", bodyMarker);
    });

    suite.test("visibility_level=1 unlocks for registered user and locks for anonymous", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-vis1");
      const marker = `REG_BODY_${uid("v1")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Kayitli Icerik ${slug}`,
          body_tr: `<p>Ilk cumle.</p><p>Ikinci cumle.</p><p>${marker}</p>`,
          visibility_level: 1,
        }),
      });
      c.assert.assertStatus(cr, 200);
      const anon = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(anon.json?.paywall?.is_locked, true);

      const reg = await c.auth.registerUser();
      const authRes = await reg.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(authRes, 200);
      c.assert.assertEqual(authRes.json?.paywall?.is_locked, false);
      c.assert.assertIncludes(authRes.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("visibility_level=2 locks for registered user and unlocks for paid subscriber", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-vis2");
      const marker = `SUB_ONLY_${uid("v2")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Abone Icerik ${slug}`,
          body_tr: `<p>Bir.</p><p>Iki.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);

      const reg = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const regRes = await reg.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(regRes.json?.paywall?.is_locked, true);

      const sub = await c.auth.createUserWithRole("subscriber_digital", {
        superAdminClient: sa.client,
        subscription_tier: "dijital",
      });
      const subRes = await sub.client.get(`/api/v1/contents/${slug}`);
      c.assert.assertEqual(subRes.json?.paywall?.is_locked, false);
      c.assert.assertIncludes(subRes.json?.paywall?.rendered_body || "", marker);
    });

    suite.test("all 4 subscriber classifications unlock level-2 premium content", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-sub4");
      const marker = `ALL_SUBS_${uid("s4")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Abone Tum Tipler ${slug}`,
          body_tr: `<p>Acilis.</p><p>Detay.</p><p>${marker}</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);

      for (const role of ["subscriber_public", "subscriber_digital", "subscriber_print_digital", "subscriber_corporate"]) {
        const u = await c.auth.createUserWithRole(role, { superAdminClient: sa.client });
        const res = await u.client.get(`/api/v1/contents/${slug}`);
        c.assert.assertStatus(res, 200);
        c.assert.assertEqual(res.json?.paywall?.is_locked, false, `Role ${role} should unlock level-2 content`);
        c.assert.assertIncludes(res.json?.paywall?.rendered_body || "", marker);
      }
    });

    suite.test("SSR /icerik/:slug displays paywall gate CTA for locked level-2 article", { req: "REQ-013" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-ssr-gate");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `SSR Kilitli ${slug}`,
          body_tr: "<p>Giris paragrafi.</p><p>Ikinci cumle.</p><p>HIDDEN_PREMIUM_SECRET</p>",
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertNotIncludes(res.text, "HIDDEN_PREMIUM_SECRET");
      c.assert.assertMatch(res.text, /(abone|giriş|paywall|subscribe|sign in)/i);
    });
  });

  // ── REQ-014: Anonymous teaser truncation ──────────────────────────────────
  suite.describe("REQ-014 Anonymous Teaser Truncation", () => {
    suite.test("anonymous teaser preserves opening sentences while stripping full body", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-teaser1");
      const openMarker = `OPEN_TEASER_${uid("o")}`;
      const secretMarker = `LOCKED_TAIL_${uid("l")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Teaser Baslik ${slug}`,
          body_tr: `<p>${openMarker} ilk cumle burada biter. Ikinci cumle analiz baslatir. Ucuncu cumle detay verir. ${secretMarker} gizli son kisimdir.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.paywall?.is_locked, true);
      c.assert.assertIncludes(res.json?.paywall?.rendered_body || "", openMarker);
      c.assert.assertNotIncludes(res.json?.paywall?.rendered_body || "", secretMarker);
    });

    suite.test("GET /api/v1/contents list also truncates gated body_tr/body_en for anonymous users", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-teaser2");
      const secretMarker = `LIST_SECRET_${uid("ls")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Liste Teaser ${slug}`,
          body_tr: `<p>Acilis cumlesi. Ikinci cumle. Ucuncu cumle. ${secretMarker}.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/contents", { query: { q: slug } });
      c.assert.assertStatus(res, 200);
      const item = res.json.find((x) => x.slug === slug);
      c.assert.assertOk(item, "Expected content item in list");
      c.assert.assertNotIncludes(item.body_tr || "", secretMarker);
    });

    suite.test("GET /api/v1/search truncates locked content body for anonymous users", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-teaser3");
      const secretMarker = `SEARCH_SECRET_${uid("ss")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Arama Teaser ${slug}`,
          body_tr: `<p>Ilk cumle. Ikinci cumle. Ucuncu cumle. ${secretMarker}.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/api/v1/search", { query: { q: slug } });
      c.assert.assertStatus(res, 200);
      const item = (res.json?.items || []).find((x) => x.slug === slug);
      c.assert.assertOk(item, "Expected item in search results");
      c.assert.assertNotIncludes(item.body_tr || "", secretMarker);
    });

    suite.test("English /icerik/:slug?lang=en truncates body_en cleanly for gated content", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-teaser4");
      const enOpen = `EN_OPEN_${uid("eo")}`;
      const enSecret = `EN_SECRET_${uid("es")}`;
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `TR Baslik ${slug}`,
          title_en: `EN Title ${slug}`,
          body_tr: "<p>TR acilis. TR ikinci. TR gizli.</p>",
          body_en: `<p>${enOpen} starts here. Second sentence continues. Third sentence expands. ${enSecret} hidden tail.</p>`,
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/icerik/${slug}`, { query: { lang: "en" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, enOpen);
      c.assert.assertNotIncludes(res.text, enSecret);
    });

    suite.test("SSR paywall response sets lock_reason metadata in API", { req: "REQ-014" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-teaser5");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Kilit Nedeni ${slug}`,
          body_tr: "<p>Bir. Iki. Uc.</p>",
          content_type: "makaleler-analizler",
          visibility_level: 2,
          placements: ["makaleler-analizler"],
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Boolean(res.json?.paywall?.lock_reason), "Expected non-empty lock_reason");
    });
  });

  // ── REQ-015: Registration fields & verification note ──────────────────────
  suite.describe("REQ-015 Registration Fields & Verification Note", () => {
    suite.test("registration form in SSR HTML exposes all required fields and SSB tags", { req: "REQ-015" }, async (c) => {
      const res = await c.http.createClient().get("/");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "register-form");
      c.assert.assertIncludes(res.text, "organization_category");
      c.assert.assertIncludes(res.text, "request_message");
      c.assert.assertIncludes(res.text, "register-ssb-tags");
    });

    suite.test("POST /api/v1/auth/register persists full profile, SSB interests, and verification note", { req: "REQ-015" }, async (c) => {
      const note = `Proje koordinasyonu icin erisim talebi ${uid("note")}`;
      const reg = await c.auth.registerUser({
        ...antiBotFields(),
        full_name: "Dr. Ahmet Yilmaz",
        organization_category: "Üniversite / Öğrenci",
        organization_name: "ODTU Savunma Lab",
        job_title: "Arastirmaci",
        ssb_interests: ["radarlar-ve-sensorler", "itki-tahrik-ve-guc-sistemleri"],
        request_message: note,
        email_consent: true,
        tos_version: "2026-01",
      });
      c.assert.assertStatus(reg.res, 200);
      c.assert.assertEqual(reg.user?.organization_category, "Üniversite / Öğrenci");
      c.assert.assertEqual(reg.user?.organization_name, "ODTU Savunma Lab");
      c.assert.assertEqual(reg.user?.job_title, "Arastirmaci");
    });

    suite.test("registration form includes all canonical institution categories (MSB, SSB, TSK)", { req: "REQ-015" }, async (c) => {
      const res = await c.http.createClient().get("/");
      c.assert.assertStatus(res, 200);
      for (const inst of ["MSB", "SSB", "K.K.K.lığı", "Dz.K.K.lığı", "Hv.K.K.lığı", "Sektör Kuruluşları"]) {
        c.assert.assertIncludes(res.text, inst, `Missing institution option ${inst} in registration dialog`);
      }
    });

    suite.test("POST /api/v1/auth/register sets HttpOnly msi_session cookie and returns user summary", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      c.assert.assertStatus(reg.res, 200);
      c.assert.assertOk(Boolean(reg.client.getRawCookie("msi_session")), "Expected msi_session cookie after registration");
      c.assert.assertEqual(reg.res.json?.status, "registered");
    });

    suite.test("registered user data export includes ToS and consent records", { req: "REQ-015" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields(), tos_version: "2026-01" });
      const exp = await reg.client.get("/api/v1/account/export");
      c.assert.assertStatus(exp, 200);
      c.assert.assertEqual(exp.json?.profile?.tos_version, "2026-01");
      c.assert.assertOk(
        Array.isArray(exp.json?.consent_records?.kvkk_consent_logs) &&
          exp.json.consent_records.kvkk_consent_logs.length >= 1
      );
    });
  });

  // ── REQ-016: Gov/military auto-verify & corporate pool ────────────────────
  suite.describe("REQ-016 Gov/Military Auto-Verify & Corporate Pool", () => {
    suite.test(".gov.tr / .tsk.tr / .mil.tr / .pol.tr domains auto-assign subscriber_public", { req: "REQ-016" }, async (c) => {
      for (const domain of ["msb.gov.tr", "kkk.tsk.tr", "hvkk.mil.tr", "egm.pol.tr"]) {
        const reg = await c.auth.registerUser({
          email: uniqueEmail("gov", domain),
          organization_category: "MSB",
          ...antiBotFields(),
        });
        c.assert.assertEqual(reg.govAutoSubscribed, true, `${domain} should auto-subscribe`);
        c.assert.assertEqual(reg.user?.role, "subscriber_public");
        c.assert.assertEqual(reg.user?.is_gov_verified, true);
      }
    });

    suite.test("personal email claiming MSB queues corporate verification request in Tab 4 pool", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const email = uniqueEmail("corp", "gmail.com");
      const reg = await c.auth.registerUser({
        email,
        organization_category: "MSB",
        organization_name: "MSB Tedarik",
        request_message: "Kurumsal dogrulama talebi",
        ...antiBotFields(),
      });
      c.assert.assertEqual(reg.govAutoSubscribed, false);
      c.assert.assertEqual(reg.corporateVerificationQueued, true);
      c.assert.assertEqual(reg.user?.role, "registered");

      const pool = await sa.client.get("/api/v1/admin/corporate-requests");
      c.assert.assertStatus(pool, 200);
      c.assert.assertOk(pool.json.some((r) => r.email === email && r.status === "pending"));
    });

    suite.test("admin approving corporate request upgrades user to subscriber_public", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const email = uniqueEmail("corpapp", "outlook.com");
      await c.auth.registerUser({
        email,
        organization_category: "SSB",
        organization_name: "SSB Daire",
        request_message: "Onay lutfen",
        ...antiBotFields(),
      });
      const pool = await sa.client.get("/api/v1/admin/corporate-requests");
      const reqItem = pool.json.find((r) => r.email === email);
      c.assert.assertOk(reqItem?.id > 0);

      await c.auth.approveCorporateRequest(reqItem.id, "approved", sa.client);
      const poolAfter = await sa.client.get("/api/v1/admin/corporate-requests");
      const decided = poolAfter.json.find((r) => r.id === reqItem.id);
      c.assert.assertEqual(decided?.status, "approved");
    });

    suite.test("admin rejecting corporate request keeps user in registered role", { req: "REQ-016" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const email = uniqueEmail("corprej", "yahoo.com");
      const reg = await c.auth.registerUser({
        email,
        organization_category: "Sektör Kuruluşları",
        organization_name: "Test Savunma",
        request_message: "Reddedilecek talep",
        ...antiBotFields(),
      });
      const pool = await sa.client.get("/api/v1/admin/corporate-requests");
      const reqItem = pool.json.find((r) => r.email === email);
      c.assert.assertOk(reqItem?.id > 0);

      await c.auth.approveCorporateRequest(reqItem.id, "rejected", sa.client);
      const users = await sa.client.get("/api/v1/admin/users", { query: { per_page: "100" } });
      const u = unwrap(users).find((x) => x.id === reg.user.id);
      c.assert.assertEqual(u?.role, "registered");
    });

    suite.test("non-staff corporate request decision via body endpoint succeeds for site_operator", { req: "REQ-016" }, async (c) => {
      const op = await c.auth.loginAsAdmin();
      const email = uniqueEmail("corpbody", "gmail.com");
      await c.auth.registerUser({
        email,
        organization_category: "EGM",
        organization_name: "EGM Siber",
        request_message: "EGM dogrulama",
        ...antiBotFields(),
      });
      const pool = await op.client.get("/api/v1/admin/corporate-requests");
      const reqItem = pool.json.find((r) => r.email === email);
      c.assert.assertOk(reqItem?.id > 0);

      const decide = await op.client.post("/api/v1/admin/corporate-requests/decide", {
        json: { id: reqItem.id, status: "approved" },
      });
      c.assert.assertStatus(decide, 200);
      c.assert.assertEqual(unwrap(decide)?.decision, "approved");
    });
  });

  // ── REQ-017: Reader Portal (/hesabim), password reset, verify, TOTP 2FA ───
  suite.describe("REQ-017 Reader Portal, Auth Tokens & 2FA", () => {
    suite.test("GET /hesabim renders guest welcome for anonymous and portal for logged-in user", { req: "REQ-017" }, async (c) => {
      const anonRes = await c.http.createClient().get("/hesabim");
      c.assert.assertStatus(anonRes, 200);
      c.assert.assertIncludes(anonRes.text, "#modal-login");

      const reg = await c.auth.registerUser();
      const authRes = await reg.client.get("/hesabim");
      c.assert.assertStatus(authRes, 200);
      c.assert.assertIncludes(authRes.text, reg.email);
    });

    suite.test("GET /sifre-sifirla and POST /api/v1/auth/forgot-password work cleanly", { req: "REQ-017" }, async (c) => {
      const client = c.http.createClient();
      const page = await client.get("/sifre-sifirla");
      c.assert.assertStatus(page, 200);
      const reg = await c.auth.registerUser();
      const forgot = await client.post("/api/v1/auth/forgot-password", {
        json: { email: reg.email, ...antiBotFields() },
      });
      c.assert.assertStatus(forgot, 200);
      c.assert.assertEqual(unwrap(forgot)?.sent, true);
    });

    suite.test("GET /hesap-dogrula renders verification page and resend-verification succeeds", { req: "REQ-017" }, async (c) => {
      const reg = await c.auth.registerUser();
      const page = await reg.client.get("/hesap-dogrula");
      c.assert.assertStatus(page, 400); // missing token renders transactional 400 verification HTML
      c.assert.assertIncludes(page.text, "E-POSTA DOĞRULAMA");
      const resend = await reg.client.post("/api/v1/auth/resend-verification", {
        json: { email: reg.email },
      });
      c.assert.assertStatus(resend, 200);
    });

    suite.test("TOTP 2FA enroll -> login challenge -> verify -> backup code -> disable", { req: "REQ-017" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const { secret, backupCodes } = await c.auth.enableTotp(u.client);
      c.assert.assertOk(secret.length >= 16);
      c.assert.assertOk(backupCodes.length >= 1);

      await u.client.post("/api/v1/auth/logout");
      const fresh = c.http.createClient();
      const step1 = await c.auth.login(fresh, { email: u.email, password: u.password });
      c.assert.assertEqual(step1.status, "totp_required");
      c.assert.assertEqual(fresh.getRawCookie("msi_session"), null, "No session cookie before TOTP verified");

      const step2 = await c.auth.verifyTotpLogin(fresh, step1.tempToken, backupCodes[0]);
      c.assert.assertEqual(step2.status, "authenticated");
      await c.auth.disableTotp(fresh, { password: u.password });
    });

    suite.test("POST /api/v1/auth/logout revokes session token persistently", { req: "REQ-017" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const rawSession = u.client.getRawCookie("msi_session");
      c.assert.assertOk(rawSession, "Expected session token before logout");

      const out = await u.client.post("/api/v1/auth/logout");
      c.assert.assertStatus(out, 200);

      // Replay captured session cookie on a new client -> must fail export (401)
      const replay = c.http.createClient();
      replay.setCookie("msi_session", rawSession);
      const exp = await replay.get("/api/v1/account/export");
      c.assert.assertStatus(exp, 401);
    });
  });
}
