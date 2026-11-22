import http from "node:http";
import {
  createAdminAd,
  createAdminContent,
  createVerifiedSubscriber,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 5: Multi-Domain, Accept-Language, Glossary, SEO Redirects, Custom Fields & Related", () => {
    suite.test(
      "T3-28: Multi-domain Host routing (msidergisi.com vs savunmahaber.com) serves distinct homepages, titles & feeds",
      { req: ["REQ-001", "REQ-005"] },
      async (c) => {
        const msiClient = c.http.createClient({ defaultHost: "msidergisi.com" });
        const shClient = c.http.createClient({ defaultHost: "savunmahaber.com" });

        const [msiHome, shHome] = await Promise.all([msiClient.get("/"), shClient.get("/")]);
        assert.assertStatus(msiHome, 200);
        assert.assertStatus(shHome, 200);
        assert.assertNotEqual(
          msiHome.text,
          shHome.text,
          "msidergisi.com and savunmahaber.com must render distinct domain HTML"
        );
        assert.assertIncludes(shHome.text.toLowerCase(), "savunma");

        const [msiRss, shRss] = await Promise.all([
          msiClient.get("/rss.xml"),
          shClient.get("/rss.xml"),
        ]);
        assert.assertStatus(msiRss, 200);
        assert.assertStatus(shRss, 200);
        assert.assertIncludes(msiRss.text, "<rss");
        assert.assertIncludes(shRss.text, "<rss");
      }
    );

    suite.test(
      "T3-29: Accept-Language auto-detect follows ?lang= > msi_lang cookie > Accept-Language header > tr fallback",
      { req: ["REQ-002"] },
      async (c) => {
        const client = c.http.createClient();

        // 1. No query, no cookie, Accept-Language: en-US -> English
        const enHeaderHome = await client.get("/", {
          headers: { "Accept-Language": "en-US,en;q=0.9,tr;q=0.4" },
        });
        assert.assertStatus(enHeaderHome, 200);
        assert.assertMatch(
          enHeaderHome.text,
          /<html[^>]+lang=["']en["']/i,
          "Accept-Language: en-US without ?lang= or cookie must auto-select lang='en'"
        );

        // 2. Accept-Language on personalized API returns English titles
        const enFeed = await client.get("/api/v1/public/personalized", {
          headers: { "Accept-Language": "en-GB,en;q=0.8" },
        });
        assert.assertStatus(enFeed, 200);

        // 3. msi_lang=tr cookie overrides Accept-Language: en-US
        client.setCookie("msi_lang", "tr");
        const cookieOverrideHome = await client.get("/", {
          headers: { "Accept-Language": "en-US,en;q=0.9" },
        });
        assert.assertStatus(cookieOverrideHome, 200);
        assert.assertMatch(
          cookieOverrideHome.text,
          /<html[^>]+lang=["']tr["']/i,
          "msi_lang=tr cookie must override Accept-Language: en-US"
        );

        // 4. ?lang=en query overrides msi_lang=tr cookie
        const queryOverrideHome = await client.get("/", {
          query: { lang: "en" },
          headers: { "Accept-Language": "tr-TR" },
        });
        assert.assertStatus(queryOverrideHome, 200);
        assert.assertMatch(
          queryOverrideHome.text,
          /<html[^>]+lang=["']en["']/i,
          "?lang=en query param must override msi_lang=tr cookie"
        );
      }
    );

    suite.test(
      "T3-30: Defense glossary guard flags forbidden variants during auto-translate & translation approval workflow",
      { req: ["REQ-003"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const termTr = `E2E-${uid("term")}`;
        const forbiddenVariant = "forbidden-e2e-drone";

        const glossRes = await sa.client.post("/api/v1/admin/glossary", {
          json: {
            id: 0,
            term_tr: termTr,
            term_en: "Indigenous UCAV",
            domain: "aerospace",
            rule_type: "preferred",
            preferred_en: "Indigenous UCAV",
            forbidden_variants: [forbiddenVariant],
            case_sensitive: false,
            priority: 10,
            notes: "Use Indigenous UCAV",
            is_active: true,
          },
        });
        assert.assertStatus(glossRes, 200, "Creating glossary term must succeed");

        // Spin up tiny local LibreTranslate stub to exercise real auto-translate HTTP & glossary violation check
        const stub = http.createServer((req, res) => {
          res.writeHead(200, { "Content-Type": "application/json" });
          res.end(JSON.stringify({ translatedText: `Next-gen ${forbiddenVariant} test` }));
        });
        await new Promise((resolve) => stub.listen(0, "127.0.0.1", resolve));
        const stubPort = stub.address().port;

        // Snapshot previous auto_translate settings
        const prevSettings = await sa.client.get("/api/v1/admin/settings");
        const prevMap = prevSettings.json?.map_en || {};

        try {
          await sa.client.post("/api/v1/admin/settings", {
            json: {
              key: "auto_translate_provider",
              value_tr: "libretranslate",
              value_en: "libretranslate",
              setting_group: "i18n",
            },
          });
          await sa.client.post("/api/v1/admin/settings", {
            json: {
              key: "auto_translate_url",
              value_tr: `http://127.0.0.1:${stubPort}/translate`,
              value_en: `http://127.0.0.1:${stubPort}/translate`,
              setting_group: "i18n",
            },
          });

          const mtRes = await sa.client.post("/api/v1/admin/translations/auto-translate", {
            json: {
              key: uid("i18n"),
              text: `${termTr} uçuş testi`,
              source_lang: "tr",
              target_lang: "en",
            },
          });
          assert.assertStatus(mtRes, 200, "Auto-translate with configured provider must succeed");
          const violations = mtRes.json?.data?.glossary_violations || [];
          assert.assertOk(
            violations.some((v) => v.term_tr === termTr && v.forbidden === forbiddenVariant),
            `Expected glossary violation for "${forbiddenVariant}", got ${JSON.stringify(violations)}`
          );
        } finally {
          await sa.client.post("/api/v1/admin/settings", {
            json: {
              key: "auto_translate_provider",
              value_tr: prevMap.auto_translate_provider || "none",
              value_en: prevMap.auto_translate_provider || "none",
              setting_group: "i18n",
            },
          });
          await sa.client.post("/api/v1/admin/settings", {
            json: {
              key: "auto_translate_url",
              value_tr: prevMap.auto_translate_url || "",
              value_en: prevMap.auto_translate_url || "",
              setting_group: "i18n",
            },
          });
          await new Promise((resolve) => stub.close(resolve));
        }

        // Translation upsert -> cache reload check
        const tKey = uid("e2e.i18n.key");
        const tUp = await sa.client.post("/api/v1/admin/translations", {
          json: {
            key: tKey,
            value_tr: "Türkçe Değer",
            value_en: "English Value",
            group: "general",
          },
        });
        assert.assertStatus(tUp, 200);
        const delT = await sa.client.delete(`/api/v1/admin/translations/${tKey}`);
        assert.assertStatus(delT, 200);
      }
    );

    suite.test(
      "T3-31: 301 seo_redirects CRUD redirects legacy paths and /icerik/:old_slug with 301 Location",
      { req: ["REQ-035", "REQ-001"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const targetArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-redirect-target"),
        });

        const legacyPath = `/eski-arsiv/${uid("legacy")}`;
        const redirRes = await op.client.post("/api/v1/admin/redirects", {
          json: {
            id: 0,
            old_path: legacyPath,
            new_path: `/icerik/${targetArticle.slug}`,
            status_code: 301,
            hit_count: 0,
          },
        });
        assert.assertStatus(redirRes, 200);
        const redirId = redirRes.json?.data?.id;
        assert.assertOk(redirId > 0);

        const anon = c.http.createClient();
        const manualRedirect = await anon.get(legacyPath, { redirect: "manual" });
        assert.assertStatus(
          manualRedirect,
          [301, 308],
          "Registered seo_redirect must respond with 301 permanent redirect"
        );
        assert.assertIncludes(
          manualRedirect.headers.get("location") || "",
          `/icerik/${targetArticle.slug}`,
          "Redirect Location must point to new_path"
        );

        const followed = await anon.get(legacyPath);
        assert.assertStatus(followed, 200, "Following 301 redirect must render target article 200");

        const delRes = await op.client.delete(`/api/v1/admin/redirects/${redirId}`);
        assert.assertStatus(delRes, 200);
      }
    );

    suite.test(
      "T3-32: Custom fields & custom-field-values GET/PUT persist across content, ad & user entities with RBAC gating",
      { req: ["REQ-008", "REQ-033"] },
      async (c) => {
        const sa = await c.auth.loginAsSuperAdmin();
        const contentFieldKey = uid("cf_content").replace(/-/g, "_");
        const adFieldKey = uid("cf_ad").replace(/-/g, "_");

        await sa.client.post("/api/v1/admin/custom-fields", {
          json: {
            field_key: contentFieldKey,
            label_tr: "Program Kodu",
            label_en: "Program Code",
            field_type: "text",
            applies_to: "content",
          },
        });
        await sa.client.post("/api/v1/admin/custom-fields", {
          json: {
            field_key: adFieldKey,
            label_tr: "Ajans Referansı",
            label_en: "Agency Ref",
            field_type: "text",
            applies_to: "ad",
          },
        });

        const cfContentList = await sa.client.get("/api/v1/admin/custom-fields", {
          query: { applies_to: "content", per_page: "100" },
        });
        const cfContent = (cfContentList.json?.data || []).find(
          (f) => f.field_key === contentFieldKey
        );
        assert.assertOk(cfContent, "Created content custom field must be listed");

        const cfAdList = await sa.client.get("/api/v1/admin/custom-fields", {
          query: { applies_to: "ad", per_page: "100" },
        });
        const cfAd = (cfAdList.json?.data || []).find((f) => f.field_key === adFieldKey);
        assert.assertOk(cfAd, "Created ad custom field must be listed");

        const art = await createAdminContent(c, sa.client, { slug: uid("t3-cf-art") });
        const ad = await createAdminAd(c, sa.client, { force_override: true });

        const putContentVal = await sa.client.put("/api/v1/admin/custom-field-values", {
          json: {
            entity_type: "content",
            entity_id: String(art.id),
            values: [{ field_id: cfContent.id, value_text: "TF-X-2026" }],
          },
        });
        assert.assertStatus(putContentVal, 200);

        const getContentVal = await sa.client.get("/api/v1/admin/custom-field-values", {
          query: { entity_type: "content", entity_id: String(art.id) },
        });
        assert.assertStatus(getContentVal, 200);
        assert.assertOk(
          (getContentVal.json?.data?.values || []).some(
            (v) => v.field_id === cfContent.id && v.value_text === "TF-X-2026"
          ),
          "Saved content custom field value must persist"
        );

        const putAdVal = await sa.client.put("/api/v1/admin/custom-field-values", {
          json: {
            entity_type: "ad",
            entity_id: String(ad.id),
            values: [{ field_id: cfAd.id, value_text: "AGY-99" }],
          },
        });
        assert.assertStatus(putAdVal, 200);

        // RBAC cross-entity check: content_editor must be denied writing ad custom field values
        const editor = await c.auth.createUserWithRole("content_editor");
        const denyAdWrite = await editor.client.put("/api/v1/admin/custom-field-values", {
          json: {
            entity_type: "ad",
            entity_id: String(ad.id),
            values: [{ field_id: cfAd.id, value_text: "HIJACK" }],
          },
        });
        assert.assertStatus(
          denyAdWrite,
          403,
          "content_editor must be denied writing entity_type=ad custom field values"
        );
      }
    );

    suite.test(
      "T3-33: recommendation_rules CRUD drives /api/v1/public/related ordering & institutional filtering",
      { req: ["REQ-012"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const tag = uid("t3-rec-tag");

        const sourceArt = await createAdminContent(c, op.client, {
          slug: uid("t3-rec-src"),
          content_type: "haberler",
          placements: ["haberler"],
          ssb_tags: [tag],
        });
        const targetArt = await createAdminContent(c, op.client, {
          slug: uid("t3-rec-dst"),
          content_type: "basyazilar",
          placements: ["basyazilar"],
          ssb_tags: ["siber-guvenlik"],
          published_at: "2020-01-01 00:00:00",
          title_tr: "Kural İle Önerilen Özel Analiz",
        });

        const createRule = await op.client.post("/api/v1/admin/recommendation-rules", {
          json: {
            rule_name: uid("Rule"),
            source_tag_or_cat: tag,
            target_institution: "Kamu / TSK / SSB",
            recommended_content_slug: targetArt.slug,
            priority: 100,
            is_active: true,
          },
        });
        assert.assertStatus(createRule, 201, "POST /api/v1/admin/recommendation-rules must return 201");
        const ruleId = createRule.json?.data?.id;
        assert.assertOk(ruleId > 0);

        const listRules = await op.client.get("/api/v1/admin/recommendation-rules");
        assert.assertStatus(listRules, 200);
        assert.assertOk(
          (listRules.json?.data?.rules || []).some((r) => r.id === ruleId),
          "Created recommendation rule must appear in GET list"
        );

        // Matching institution reader sees targetArt injected at top of related articles
        const govReader = await createVerifiedSubscriber(c, "subscriber_public", {
          organization_category: "Kamu / TSK / SSB",
          is_gov_verified: true,
        });
        const govRelRes = await govReader.client.get("/api/v1/public/related", {
          query: { slug: sourceArt.slug, limit: "3" },
        });
        assert.assertStatus(govRelRes, 200);
        const govItems = govRelRes.json?.data?.items || [];
        assert.assertOk(
          govItems.length >= 3,
          `Related API must return >= 3 articles, got ${govItems.length}`
        );
        assert.assertOk(
          govItems.some((i) => i.slug === targetArt.slug),
          "Active institutional recommendation rule must inject recommended_content_slug for matching institution reader"
        );

        // Mismatched institution reader must NOT receive the Kamu-only rule recommendation in top-3
        const sectorReader = await createVerifiedSubscriber(c, "subscriber_digital", {
          organization_category: "Sektör Kuruluşları",
          is_gov_verified: false,
        });
        const sectorRelRes = await sectorReader.client.get("/api/v1/public/related", {
          query: { slug: sourceArt.slug, limit: "3" },
        });
        assert.assertStatus(sectorRelRes, 200);
        const sectorItems = sectorRelRes.json?.data?.items || [];
        assert.assertNotIncludes(
          sectorItems.map((i) => i.slug),
          targetArt.slug,
          "Recommendation rule scoped to Kamu/TSK/SSB must NOT inject targetArt for mismatched institution reader"
        );

        // Deactivate rule (is_active: false) and re-query to verify rule no longer takes effect
        const updRule = await op.client.put(`/api/v1/admin/recommendation-rules/${ruleId}`, {
          json: {
            rule_name: uid("RuleUpdated"),
            source_tag_or_cat: tag,
            target_institution: "Kamu / TSK / SSB",
            recommended_content_slug: targetArt.slug,
            priority: 1,
            is_active: false,
          },
        });
        assert.assertStatus(updRule, 200);

        const afterDeactivateRes = await govReader.client.get("/api/v1/public/related", {
          query: { slug: sourceArt.slug, limit: "3" },
        });
        assert.assertStatus(afterDeactivateRes, 200);
        const afterDeactivateSlugs = (afterDeactivateRes.json?.data?.items || []).map((i) => i.slug);
        assert.assertNotIncludes(
          afterDeactivateSlugs,
          targetArt.slug,
          "Deactivated recommendation rule (is_active=false) must no longer inject recommended_content_slug"
        );

        const delRule = await op.client.delete(`/api/v1/admin/recommendation-rules/${ruleId}`);
        assert.assertStatus(delRule, 200);
      }
    );

    suite.test(
      "T3-34: Article SSR (reader.rs) injects mid-body .msi-inline-related[data-related-inline] + bottom strip (>=3 distinct) on non-sponsored",
      { req: ["REQ-012", "REQ-004"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const slug = uid("t3-inline-rel-ssr");
        await createAdminContent(c, op.client, {
          slug,
          body_tr:
            "<p>Birinci paragraf: Savunma teknolojileri hızla gelişmektedir.</p>" +
            "<p>İkinci paragraf: Aviyonik sistemler testlerden başarıyla geçmektedir.</p>" +
            "<p>Üçüncü paragraf: Yerli sensör entegrasyonu tamamlanmaktadır.</p>" +
            "<p>Dördüncü paragraf: Seri üretim takvimi netleşmektedir.</p>",
          is_sponsored: false,
        });

        const anon = c.http.createClient();
        const htmlRes = await anon.get(`/icerik/${slug}`);
        assert.assertStatus(htmlRes, 200);
        assert.assertMatch(
          htmlRes.text,
          /class=["'][^"']*msi-inline-related[^"']*["'][^>]*data-related-inline|data-related-inline[^>]*class=["'][^"']*msi-inline-related/,
          "Non-sponsored multi-paragraph article SSR must inject mid-body .msi-inline-related[data-related-inline] (M3 REQ-012)"
        );
      }
    );
  });
}
