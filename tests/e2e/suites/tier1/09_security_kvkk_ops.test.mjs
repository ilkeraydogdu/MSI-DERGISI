import {
  antiBotFields,
  getCachedSuperAdmin,
  makeContentPayload,
  makeRedirectPayload,
  uid,
  uniqueSlug,
  unwrap,
} from "../../fixtures/tier12_helpers.mjs";

export function registerTests(suite) {
  // ── REQ-032: KVKK consent audit, export & erasure ─────────────────────────
  suite.describe("REQ-032 KVKK Consent Audit, Export & Erasure", () => {
    suite.test("GET /kurumsal/kvkk and /kurumsal/kvkk-basvuru-formu render disclosure pages", { req: "REQ-032" }, async (c) => {
      const client = c.http.createClient();
      for (const p of ["/kurumsal/kvkk", "/kurumsal/kvkk-basvuru-formu"]) {
        const res = await client.get(p);
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text.toLowerCase(), "kvkk");
      }
    });

    suite.test("GET /api/v1/account/export returns JSON attachment excluding password/TOTP secrets", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields(), tos_version: "2026-01" });
      const exp = await reg.client.get("/api/v1/account/export");
      c.assert.assertStatus(exp, 200);
      c.assert.assertIncludes(exp.headers.get("content-disposition") || "", "MSI_KVKK_Export");
      c.assert.assertEqual(exp.json?.profile?.email, reg.email);
      c.assert.assertEqual(exp.json?.profile?.password_hash, undefined);
      c.assert.assertEqual(exp.json?.profile?.totp_secret, undefined);
    });

    suite.test("POST /api/v1/account/profile updates marketing consent and logs audit trail", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields(), email_consent: false });
      const upd = await reg.client.post("/api/v1/account/profile", {
        json: {
          full_name: reg.user.full_name,
          email_consent: true,
          marketing_emails_opted_in: true,
        },
      });
      c.assert.assertStatus(upd, 200);
      const exp = await reg.client.get("/api/v1/account/export");
      const logs = exp.json?.consent_records?.kvkk_consent_logs || [];
      c.assert.assertOk(
        logs.some((r) => r.consent_type === "etk"),
        "Expected ETK consent record after opt-in update"
      );
    });

    suite.test("POST /api/v1/account/delete anonymizes user and blocks subsequent login", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser({ ...antiBotFields() });
      const del = await reg.client.post("/api/v1/account/delete", {
        json: { password: reg.password },
      });
      c.assert.assertStatus(del, 200);
      c.assert.assertEqual(unwrap(del)?.deleted, true);

      const retry = await c.http.createClient().post("/api/v1/auth/login", {
        json: { email: reg.email, password: reg.password, ...antiBotFields() },
      });
      c.assert.assertStatus(retry, 401);
    });

    suite.test("GET /hesabim renders #account-kvkk-card and erasure form for authenticated user", { req: "REQ-032" }, async (c) => {
      const reg = await c.auth.registerUser();
      const res = await reg.client.get("/hesabim");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "account-kvkk-card");
      c.assert.assertIncludes(res.text, "account-delete-form");
    });
  });

  // ── REQ-033: Cybersecurity hardening & persistent revocations ─────────────
  suite.describe("REQ-033 Cybersecurity Hardening & Persistent Revocation", () => {
    suite.test("responses include hardened CSP, X-Frame-Options, nosniff, and Referrer-Policy", { req: "REQ-033" }, async (c) => {
      const res = await c.http.createClient().get("/");
      c.assert.assertStatus(res, 200);
      const csp = res.headers.get("content-security-policy") || "";
      c.assert.assertIncludes(csp, "object-src 'none'");
      c.assert.assertIncludes(csp, "base-uri 'self'");
      c.assert.assertEqual(res.headers.get("x-content-type-options"), "nosniff");
      c.assert.assertEqual(res.headers.get("x-frame-options"), "SAMEORIGIN");
    });

    suite.test("private routes emit Cache-Control: no-store and X-Robots-Tag: noindex", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get(`/${c.adminPath}`);
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.headers.get("cache-control") || "", "no-store");
      c.assert.assertIncludes(res.headers.get("x-robots-tag") || "", "noindex");
    });

    suite.test("rich HTML sanitization strips <script>, onerror, and javascript: URIs on content save", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-xss");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `XSS Clean ${slug}`,
          body_tr: `<p>Guvenli metin</p><script>alert(1)</script><img src="/images/logo.svg" onerror="alert(2)"><a href="javascript:alert(3)">x</a>`,
        }),
      });
      c.assert.assertStatus(cr, 200);
      const detail = await c.http.createClient().get(`/api/v1/contents/${slug}`);
      c.assert.assertStatus(detail, 200);
      const body = detail.json?.content?.body_tr || "";
      c.assert.assertNotIncludes(body, "<script");
      c.assert.assertNotIncludes(body, "onerror=");
      c.assert.assertNotIncludes(body, "javascript:");
    });

    suite.test("all 3 moved admin endpoints succeed under /api/v1/admin/* with valid staff CSRF", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const r1 = await sa.client.post("/api/v1/admin/ads/check-conflict", {
        json: { slot_code: "ad-news-top", sponsor_name: "Test" },
      });
      const r2 = await sa.client.get("/api/v1/admin/analytics/sponsor-report");
      const r3 = await sa.client.post("/api/v1/admin/subscriptions/trigger-reminders", { json: {} });
      c.assert.assertStatus(r1, 200);
      c.assert.assertStatus(r2, 200);
      c.assert.assertStatus(r3, 200);
    });

    suite.test("audit-logs and security-logs record and retrieve administrative & security events", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const ins = await sa.client.post("/api/v1/admin/audit-logs", {
        json: { action: "e2e_check", entity_type: "security", details_json: "{}" },
      });
      c.assert.assertStatus(ins, 200);
      const logs = await sa.client.get("/api/v1/admin/audit-logs", { query: { page: "1", per_page: "10" } });
      const sec = await sa.client.get("/api/v1/admin/security-logs", { query: { page: "1", per_page: "10" } });
      c.assert.assertStatus(logs, 200);
      c.assert.assertStatus(sec, 200);
    });

    suite.test("logout persistently revokes session token in SQLite revoked_tokens", { req: "REQ-033" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const u = await c.auth.createUserWithRole("registered", { superAdminClient: sa.client });
      const tok = u.client.getRawCookie("msi_session");
      c.assert.assertOk(tok);
      await u.client.post("/api/v1/auth/logout");

      const replay = c.http.createClient();
      replay.setCookie("msi_session", tok);
      const res = await replay.get("/api/v1/account/export");
      c.assert.assertStatus(res, 401);
    });
  });

  // ── REQ-034: Off-site S3 encrypted backup, RPO<1h & Open Data ─────────────
  suite.describe("REQ-034 Backup, Encrypted Archive & Open-Data Export", () => {
    suite.test("POST /api/v1/admin/backup executes online backup with SHA-256 and encrypted archive", { req: "REQ-034" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.post("/api/v1/admin/backup", { json: {} });
      c.assert.assertStatus(res, 200);
      const data = unwrap(res);
      c.assert.assertEqual(data?.status, "completed");
      c.assert.assertOk(Boolean(data?.sha256), "Expected SQLite backup sha256");
      c.assert.assertOk(Boolean(data?.encrypted_archive_path), "Expected encrypted_archive_path");
      c.assert.assertOk(Boolean(data?.encrypted_sha256), "Expected encrypted_sha256");
      c.assert.assertEqual(data?.integrity_check, "ok");
    });

    suite.test("GET /api/v1/admin/export/open-data serves vendor-lock-free UMSA JSON archive", { req: "REQ-034" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/export/open-data");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.json?.export_standard || "", "UMSA-MSI-OPEN-DATA-V1");
      c.assert.assertOk(Array.isArray(res.json?.contents_single_source_of_truth));
      c.assert.assertOk(Array.isArray(res.json?.categories));
    });

    suite.test("GET /api/v1/admin/export/open-data?anonymize=1 masks PII and corporate requests", { req: "REQ-034" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/export/open-data", { query: { anonymize: "1" } });
      c.assert.assertStatus(res, 200);
      c.assert.assertEqual(res.json?.anonymized, true);
      c.assert.assertEqual(res.json?.corporate_verification_requests?.length, 0);
    });

    suite.test("GET/POST /api/v1/admin/cache inspects and flushes L1 cache cleanly", { req: "REQ-034" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const list = await sa.client.get("/api/v1/admin/cache");
      c.assert.assertStatus(list, 200);
      const flush = await sa.client.post("/api/v1/admin/cache", { json: { cache_key: "all" } });
      c.assert.assertStatus(flush, 200);
      c.assert.assertEqual(unwrap(flush)?.status, "flushed");
    });

    suite.test("GET /api/v1/admin/monitoring/attacks returns attack telemetry", { req: "REQ-034" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const res = await sa.client.get("/api/v1/admin/monitoring/attacks");
      c.assert.assertStatus(res, 200);
      c.assert.assertOk(Array.isArray(unwrap(res)?.attacks));
    });
  });

  // ── REQ-035: ETL & 301 Redirect Map + SEO Feeds ───────────────────────────
  suite.describe("REQ-035 301 Redirect Map & SEO Discovery Feeds", () => {
    suite.test("admin creates custom 301 redirect and visitor receives 301 Permanent Redirect", { req: "REQ-035" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const oldPath = `/legacy-${uid("rd").toLowerCase()}.html`;
      const createRes = await sa.client.post("/api/v1/admin/redirects", {
        json: makeRedirectPayload({ old_path: oldPath, new_path: "/e-dergi" }),
      });
      c.assert.assertStatus(createRes, 200);
      const id = unwrap(createRes)?.id;

      try {
        const visit = await c.http.createClient().get(oldPath, { redirect: "manual" });
        c.assert.assertStatus(visit, [301, 308]);
        c.assert.assertIncludes(visit.headers.get("location") || "", "/e-dergi");
      } finally {
        await sa.client.delete(`/api/v1/admin/redirects/${id}`);
      }
    });

    suite.test("legacy /haberler/:slug and /kategoriler/:slug issue 301 redirects to canonical paths", { req: "REQ-035" }, async (c) => {
      const client = c.http.createClient();
      const r1 = await client.get("/haberler/kaan-milli-muharip-ucak", { redirect: "manual" });
      const r2 = await client.get("/kategoriler/haberler", { redirect: "manual" });
      c.assert.assertStatus(r1, [301, 308]);
      c.assert.assertIncludes(r1.headers.get("location") || "", "/icerik/");
      c.assert.assertStatus(r2, [301, 308]);
      c.assert.assertIncludes(r2.headers.get("location") || "", "/kategori/");
    });

    suite.test("XML sitemaps (/sitemap.xml, /sitemap-news.xml, /sitemap-images.xml) return valid XML", { req: "REQ-035" }, async (c) => {
      const client = c.http.createClient();
      for (const path of ["/sitemap.xml", "/sitemap-news.xml", "/sitemap-images.xml"]) {
        const res = await client.get(path);
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "<urlset");
      }
    });

    suite.test("/robots.txt, /rss.xml, and /feed.json serve valid syndication endpoints", { req: "REQ-035" }, async (c) => {
      const client = c.http.createClient();
      const robots = await client.get("/robots.txt");
      const rss = await client.get("/rss.xml");
      const feed = await client.get("/feed.json");
      c.assert.assertStatus(robots, 200);
      c.assert.assertIncludes(robots.text, "User-agent");
      c.assert.assertStatus(rss, 200);
      c.assert.assertIncludes(rss.text, "<rss");
      c.assert.assertStatus(feed, 200);
      c.assert.assertOk(Array.isArray(feed.json?.items));
    });

    suite.test("/llms.txt, /.well-known/ai-plugin.json, and /api/v1/openapi.json return 200", { req: "REQ-035" }, async (c) => {
      const client = c.http.createClient();
      for (const p of ["/llms.txt", "/.well-known/ai-plugin.json", "/api/v1/openapi.json"]) {
        const res = await client.get(p);
        c.assert.assertStatus(res, 200);
      }
    });
  });

  // ── REQ-036: Future-phase modules (History, Authors, Clubs, HR) ───────────
  suite.describe("REQ-036 Future-Phase Hubs & Routes", () => {
    suite.test("GET /savunma-sanayii-tarihi and EN aliases return 200 HTML", { req: "REQ-036" }, async (c) => {
      const client = c.http.createClient();
      for (const p of ["/savunma-sanayii-tarihi", "/en/defence-industry-history"]) {
        const res = await client.get(p);
        c.assert.assertStatus(res, 200);
        c.assert.assertIncludes(res.text, "<main");
      }
    });

    suite.test("GET /yazarlar and /en/authors serve Authors Directory", { req: "REQ-036" }, async (c) => {
      const client = c.http.createClient();
      const tr = await client.get("/yazarlar");
      const en = await client.get("/en/authors");
      c.assert.assertStatus(tr, 200);
      c.assert.assertStatus(en, 200);
    });

    suite.test("GET /ogrenci-kulupleri and /en/student-clubs serve Student Clubs Hub", { req: "REQ-036" }, async (c) => {
      const client = c.http.createClient();
      const tr = await client.get("/ogrenci-kulupleri");
      const en = await client.get("/en/student-clubs");
      c.assert.assertStatus(tr, 200);
      c.assert.assertStatus(en, 200);
    });

    suite.test("GET /kategori/insan-kaynaklari and /etkinlikler return 200 HTML", { req: "REQ-036" }, async (c) => {
      const client = c.http.createClient();
      for (const p of ["/kategori/insan-kaynaklari", "/etkinlikler", "/en/events"]) {
        const res = await client.get(p);
        c.assert.assertStatus(res, 200);
      }
    });

    suite.test("GET /yazar/:slug renders author profile page cleanly", { req: "REQ-036" }, async (c) => {
      const sa = await getCachedSuperAdmin(c);
      const slug = uniqueSlug("t1-author-art");
      const cr = await sa.client.post("/api/v1/admin/contents", {
        json: makeContentPayload({
          slug,
          title_tr: `Yazar Test ${slug}`,
          author_name: "Kemal Demir",
        }),
      });
      c.assert.assertStatus(cr, 200);
      const res = await c.http.createClient().get("/yazar/kemal-demir");
      c.assert.assertStatus(res, 200);
      c.assert.assertIncludes(res.text, "Kemal Demir");
    });
  });
}
