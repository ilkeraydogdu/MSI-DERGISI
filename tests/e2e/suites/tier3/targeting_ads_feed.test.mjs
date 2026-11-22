import {
  createAdminAd,
  createAdminContent,
  createVerifiedSubscriber,
  uid,
} from "../../fixtures/tier34_helpers.mjs";

export function registerTests(suite, ctx) {
  const { assert } = ctx;

  suite.describe("Pairwise Cluster 1: Targeting, Ads, Competitor Protection & 14 SSB Feed", () => {
    suite.test(
      "T3-01: Personalized 14 SSB feed ranks canonical hyphenated SSB tags for authenticated reader vs ?interests=",
      { req: ["REQ-006", "REQ-021"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();

        const radarArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-radar-article"),
          title_tr: "AESA Radar ve Elektro-Optik Sensör Entegrasyonu",
          ssb_tags: ["radarlar-ve-sensorler"],
          content_type: "haberler",
          placements: ["haberler"],
          is_featured: false,
        });
        const navalArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-naval-article"),
          title_tr: "MİLGEM Fırkateyn Savaş Yönetim Sistemi",
          ssb_tags: ["deniz-araclari"],
          content_type: "haberler",
          placements: ["haberler"],
          is_featured: false,
        });

        const radarReader = await createVerifiedSubscriber(c, "subscriber_digital", {
          ssb_interests: ["radarlar-ve-sensorler"],
        });

        const authFeed = await radarReader.client.get("/api/v1/public/personalized", {
          query: { limit: "12" },
        });
        assert.assertStatus(authFeed, 200, "Authenticated personalized feed must succeed");
        const authItems = authFeed.json?.data?.items || [];
        assert.assertOk(
          authFeed.json?.data?.personalized === true,
          "Authenticated feed must flag personalized=true"
        );
        assert.assertDeepEqual(
          authFeed.json?.data?.interests,
          ["radarlar-ve-sensorler"],
          "Feed must reflect saved canonical hyphenated SSB interests"
        );

        const radarIdx = authItems.findIndex((x) => x.slug === radarArticle.slug);
        const navalIdx = authItems.findIndex((x) => x.slug === navalArticle.slug);
        assert.assertOk(radarIdx >= 0, "Matching radarlar-ve-sensorler article must appear in feed");
        if (navalIdx >= 0) {
          assert.assertOk(
            radarIdx < navalIdx,
            `Matching SSB tag article (idx=${radarIdx}) must rank ahead of non-matching (idx=${navalIdx})`
          );
        }

        // Anonymous ?interests= query fallback
        const anonClient = c.http.createClient();
        const anonFeed = await anonClient.get("/api/v1/public/personalized", {
          query: { interests: "deniz-araclari", limit: "12" },
        });
        assert.assertStatus(anonFeed, 200, "Anonymous personalized feed with ?interests= must succeed");
        const anonItems = anonFeed.json?.data?.items || [];
        assert.assertEqual(anonFeed.json?.data?.personalized, false);
        assert.assertOk(
          anonItems.some((x) => x.slug === navalArticle.slug),
          "Anonymous ?interests=deniz-araclari should surface naval article"
        );
      }
    );

    suite.test(
      "T3-02: Institutional targeting (target_institutions) restricts content body & personalized feed by organization",
      { req: ["REQ-021", "REQ-039"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const secretMarker = `KAMU-EXCLUSIVE-BODY-${uid("marker")}`;
        const instSlug = uid("t3-inst-only");

        await createAdminContent(c, op.client, {
          slug: instSlug,
          title_tr: "TSK ve SSB Kuruma Özel Tedarik Dosyası",
          body_tr: `<p>Birinci cümle giriş bilgisidir.</p><p>İkinci cümle özettir.</p><p>${secretMarker} teknik detayları yalnızca yetkili kurum personeline açıktır.</p>`,
          visibility_level: 0,
          ssb_tags: ["hava-araclari"],
          target_institutions: ["Kamu / TSK / SSB"],
        });

        const govReader = await createVerifiedSubscriber(c, "subscriber_public", {
          organization_category: "Kamu / TSK / SSB",
          is_gov_verified: true,
          ssb_interests: ["hava-araclari"],
        });
        const sectorReader = await createVerifiedSubscriber(c, "subscriber_digital", {
          organization_category: "Sektör Kuruluşları",
          is_gov_verified: false,
          ssb_interests: ["hava-araclari"],
        });

        const govDetail = await govReader.client.get(`/api/v1/contents/${instSlug}`);
        assert.assertStatus(govDetail, 200);
        assert.assertEqual(
          govDetail.json?.paywall?.is_locked,
          false,
          "Matching Kamu/TSK/SSB reader must unlock institution-targeted content"
        );
        assert.assertIncludes(
          govDetail.json?.paywall?.rendered_body || "",
          secretMarker,
          "Matching institutional reader must see full body"
        );

        const sectorDetail = await sectorReader.client.get(`/api/v1/contents/${instSlug}`);
        assert.assertStatus(sectorDetail, 200);
        assert.assertEqual(
          sectorDetail.json?.paywall?.is_locked,
          true,
          "Non-matching organization reader must be locked out of institution-targeted content"
        );
        assert.assertNotIncludes(
          sectorDetail.json?.paywall?.rendered_body || "",
          secretMarker,
          "Non-matching reader must only see truncated teaser"
        );

        const sectorFeed = await sectorReader.client.get("/api/v1/public/personalized", {
          query: { limit: "24" },
        });
        assert.assertStatus(sectorFeed, 200);
        const sectorSlugs = (sectorFeed.json?.data?.items || []).map((i) => i.slug);
        assert.assertNotIncludes(
          sectorSlugs,
          instSlug,
          "Institution-targeted content must not appear in non-matching reader's personalized feed"
        );
      }
    );

    suite.test(
      "T3-03: Competitor protection hides defense_oem ads on sponsored content and serves auxiliary_service/in_house fallback",
      { req: ["REQ-021", "REQ-023"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();

        const oemAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("OemRival"),
          sponsor_type: "defense_oem",
          slot_code: "sidebar_300x250",
          target_categories: ["advertorial"],
          target_ssb_tags: ["hava-araclari"],
          budget_score: 99,
          is_default_fallback: false,
          force_override: true,
        });

        const auxAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("AuxServiceFallback"),
          sponsor_type: "auxiliary_service",
          slot_code: "sidebar_300x250",
          target_categories: [],
          target_ssb_tags: ["hizmetler-danismanlik"],
          budget_score: 50,
          is_default_fallback: true,
        });

        const sponsoredContent = await createAdminContent(c, op.client, {
          slug: uid("t3-sponsored-oem"),
          content_type: "advertorial",
          placements: ["advertorial"],
          is_sponsored: true,
          sponsor_brand: "Sponsor Savunma A.Ş.",
          sponsor_legal_label: "SPONSORLU İÇERİK",
          ssb_tags: ["hava-araclari"],
        });

        const anon = c.http.createClient();
        const detail = await anon.get(`/api/v1/contents/${sponsoredContent.slug}`);
        assert.assertStatus(detail, 200);
        assert.assertEqual(
          detail.json?.paywall?.competitor_protection_active,
          true,
          "Sponsored content must activate competitor protection"
        );
        assert.assertDeepEqual(
          detail.json?.related_articles,
          [],
          "Sponsored content must return zero related_articles"
        );

        const ctxAds = detail.json?.context_ads || [];
        assert.assertOk(
          ctxAds.length > 0,
          "Sponsored content must still serve non-competing auxiliary_service/in_house fallback ads"
        );
        for (const ad of ctxAds) {
          assert.assertNotEqual(
            ad.sponsor_type,
            "defense_oem",
            `Competitor protection violated: defense_oem ad "${ad.sponsor_name}" leaked onto sponsored article`
          );
          assert.assertOk(
            ad.sponsor_type === "auxiliary_service" || ad.sponsor_type === "in_house",
            `Expected auxiliary_service/in_house ad, got ${ad.sponsor_type}`
          );
        }
        assert.assertOk(
          ctxAds.some((a) => a.id === auxAd.id || a.is_default_fallback),
          "Auxiliary fallback ad should be eligible under competitor protection"
        );
        assert.assertOk(
          !ctxAds.some((a) => a.id === oemAd.id),
          "Competing defense_oem ad must be suppressed on sponsored content"
        );
      }
    );

    suite.test(
      "T3-04: Ad quota conflict returns 409 without force_override and succeeds with force_override=true",
      { req: ["REQ-022"] },
      async (c) => {
        const adsMgr = await c.auth.createUserWithRole("ads_manager");
        const uniqueTag = uid("t3-quota-tag");
        const slot = "inline_article_728x90";

        // Create ads up to default quota (3 per keyword/slot) -> 4th should 409
        for (let i = 1; i <= 3; i++) {
          await createAdminAd(c, adsMgr.client, {
            sponsor_name: uid(`QuotaAd${i}`),
            slot_code: slot,
            target_ssb_tags: [uniqueTag],
            is_active: true,
            is_default_fallback: false,
            start_date: "2026-01-01",
            end_date: "2026-12-31",
          });
        }

        const conflictAttempt = await createAdminAd(c, adsMgr.client, {
          sponsor_name: uid("QuotaAd4Conflict"),
          slot_code: slot,
          target_ssb_tags: [uniqueTag],
          is_active: true,
          is_default_fallback: false,
          start_date: "2026-06-01",
          end_date: "2026-08-31",
          allowConflict: true,
        });
        assert.assertStatus(
          conflictAttempt.res,
          409,
          "4th overlapping active non-fallback ad must return 409 Conflict without force_override"
        );
        assert.assertEqual(conflictAttempt.res.json?.error_code, "CONFLICT");
        assert.assertEqual(conflictAttempt.res.json?.conflict_check?.has_conflict, true);

        const overrideRes = await createAdminAd(c, adsMgr.client, {
          sponsor_name: uid("QuotaAd3Override"),
          slot_code: slot,
          target_ssb_tags: [uniqueTag],
          is_active: true,
          is_default_fallback: false,
          start_date: "2026-06-01",
          end_date: "2026-08-31",
          force_override: true,
        });
        assert.assertStatus(
          overrideRes.res,
          200,
          "Explicit force_override=true must save ad despite quota conflict"
        );
        assert.assertOk(overrideRes.id > 0, "Overridden ad must persist with positive id");
      }
    );

    suite.test(
      "T3-05: Ad schedule dates (start_date/end_date/rotation_mode) persist, validate bounds, and check-conflict under /api/v1/admin/ads",
      { req: ["REQ-022", "REQ-033"] },
      async (c) => {
        const adsMgr = await c.auth.createUserWithRole("ads_manager");
        const tag = uid("t3-sched-tag");

        // Invalid date window: start_date > end_date -> 400
        const invalidRes = await createAdminAd(c, adsMgr.client, {
          sponsor_name: uid("InvalidWindowAd"),
          target_ssb_tags: [tag],
          start_date: "2026-12-20",
          end_date: "2026-11-01",
          rotation_mode: "even",
          allowError: true,
        });
        assert.assertStatus(
          invalidRes.res,
          400,
          "Ad campaign with end_date < start_date must be rejected with 400"
        );

        const created = await createAdminAd(c, adsMgr.client, {
          sponsor_name: uid("SchedPersistAd"),
          slot_code: "header_leaderboard",
          target_ssb_tags: [tag],
          start_date: "2026-03-15",
          end_date: "2026-09-30",
          rotation_mode: "even",
        });

        const listRes = await adsMgr.client.get("/api/v1/admin/ads");
        assert.assertStatus(listRes, 200);
        const adsList = Array.isArray(listRes.json) ? listRes.json : [];
        const saved = adsList.find((a) => a.id === created.id);
        assert.assertOk(saved, "Created scheduled ad must appear in /api/v1/admin/ads");
        assert.assertEqual(saved.start_date, "2026-03-15", "start_date must persist");
        assert.assertEqual(saved.end_date, "2026-09-30", "end_date must persist");
        assert.assertEqual(saved.rotation_mode, "even", "rotation_mode must persist");

        // Canonical check-conflict endpoint
        const checkRes = await adsMgr.client.post("/api/v1/admin/ads/check-conflict", {
          json: {
            slot_code: "header_leaderboard",
            sponsor_name: "Probe Sponsor",
            target_ssb_tags: [tag],
            start_date: "2026-04-01",
            end_date: "2026-05-01",
            rotation_mode: "even",
          },
        });
        assert.assertStatus(checkRes, 200, "POST /api/v1/admin/ads/check-conflict must succeed");
        assert.assertOk(
          typeof checkRes.json?.has_conflict === "boolean",
          "Conflict check response must include boolean has_conflict"
        );
      }
    );

    suite.test(
      "T3-06: Multi-dimensional ad targeting (category + ssb_tag + organization) ranks & filters /api/v1/ads?content_slug=",
      { req: ["REQ-021", "REQ-022"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const targetSsb = "itki-tahrik-ve-guc-sistemleri";

        const targetedAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("TargetedEngineAd"),
          slot_code: "sidebar_300x250",
          target_categories: ["makaleler-analizler"],
          target_ssb_tags: [targetSsb],
          target_institutions: ["Kamu / TSK / SSB"],
          budget_score: 95,
          rotation_mode: "weighted",
          force_override: true,
        });
        const sectorAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("TargetedSectorAd"),
          slot_code: "sidebar_300x250",
          target_categories: ["haberler"],
          target_ssb_tags: ["deniz-araclari"],
          target_institutions: ["Sektör Kuruluşları"],
          budget_score: 95,
          rotation_mode: "weighted",
          force_override: true,
        });
        const expiredAd = await createAdminAd(c, op.client, {
          sponsor_name: uid("ExpiredTargetedAd"),
          slot_code: "sidebar_300x250",
          target_categories: ["makaleler-analizler"],
          target_ssb_tags: [targetSsb],
          target_institutions: ["Kamu / TSK / SSB"],
          start_date: "2020-01-01",
          end_date: "2020-12-31",
          budget_score: 100,
          force_override: true,
        });

        const article = await createAdminContent(c, op.client, {
          slug: uid("t3-ad-match-article"),
          content_type: "makaleler-analizler",
          placements: ["makaleler-analizler"],
          ssb_tags: [targetSsb],
        });
        const otherArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-ad-sector-article"),
          content_type: "haberler",
          placements: ["haberler"],
          ssb_tags: ["deniz-araclari"],
        });

        const govUser = await createVerifiedSubscriber(c, "subscriber_public", {
          organization_category: "Kamu / TSK / SSB",
          is_gov_verified: true,
        });
        const adsForGov = await govUser.client.get("/api/v1/ads", {
          query: { category: "makaleler-analizler", content_slug: article.slug },
        });
        assert.assertStatus(adsForGov, 200);
        const govAdIds = (Array.isArray(adsForGov.json) ? adsForGov.json : []).map((a) => a.id);
        assert.assertOk(
          govAdIds.includes(targetedAd.id),
          "Matching gov reader + category + ssb_tag should include targetedAd"
        );
        assert.assertOk(
          govAdIds.indexOf(targetedAd.id) < govAdIds.indexOf(sectorAd.id),
          "Matching targetedAd must outrank non-matching sectorAd for govUser on matching article"
        );
        assert.assertOk(
          !govAdIds.includes(expiredAd.id),
          "Out-of-schedule expiredAd must be filtered out completely"
        );

        // Negative control: non-matching organization/category reader must not rank targetedAd ahead of sectorAd
        const sectorUser = await createVerifiedSubscriber(c, "subscriber_digital", {
          organization_category: "Sektör Kuruluşları",
          is_gov_verified: false,
          ssb_interests: ["deniz-araclari"],
        });
        const adsForSector = await sectorUser.client.get("/api/v1/ads", {
          query: { category: "haberler", content_slug: otherArticle.slug },
        });
        assert.assertStatus(adsForSector, 200);
        const sectorAdIds = (Array.isArray(adsForSector.json) ? adsForSector.json : []).map((a) => a.id);
        assert.assertOk(
          sectorAdIds.includes(sectorAd.id),
          "Sector reader on haberler/deniz-araclari must receive matching sectorAd"
        );
        assert.assertOk(
          sectorAdIds.indexOf(sectorAd.id) < sectorAdIds.indexOf(targetedAd.id),
          "Non-matching gov/makaleler targetedAd must NOT outrank matching sectorAd for sectorUser"
        );
        assert.assertNotEqual(
          sectorAdIds[0],
          targetedAd.id,
          "Non-matching targetedAd must NOT be selected as primary top ad for sectorUser"
        );
      }
    );

    suite.test(
      "T3-07: Sponsored advertorial renders legal label on SSR /icerik/:slug and is excluded from organic recommendations",
      { req: ["REQ-010", "REQ-023"] },
      async (c) => {
        const op = await c.auth.loginAsAdmin();
        const sponsoredSlug = uid("t3-legal-advertorial");
        await createAdminContent(c, op.client, {
          slug: sponsoredSlug,
          title_tr: "Sponsorlu İleri Aviyonik Dosyası",
          content_type: "advertorial",
          placements: ["advertorial", "haberler"],
          ssb_tags: ["hava-araclari"],
          is_sponsored: true,
          sponsor_brand: "Aselsan Test Partner",
          sponsor_legal_label: "SPONSORLU İÇERİK / İLANDIR",
        });

        const anon = c.http.createClient();
        const ssrRes = await anon.get(`/icerik/${sponsoredSlug}`);
        assert.assertStatus(ssrRes, 200, "Sponsored article SSR page must return 200");
        assert.assertIncludes(
          ssrRes.text,
          "SPONSORLU İÇERİK / İLANDIR",
          "SSR article must display mandatory sponsor legal label"
        );
        assert.assertIncludes(
          ssrRes.text,
          "Aselsan Test Partner",
          "SSR article must display sponsor brand"
        );

        const organicArticle = await createAdminContent(c, op.client, {
          slug: uid("t3-organic-check"),
          content_type: "haberler",
          ssb_tags: ["hava-araclari"],
          is_sponsored: false,
        });

        const relRes = await anon.get("/api/v1/public/related", {
          query: { slug: organicArticle.slug, limit: "12" },
        });
        assert.assertStatus(relRes, 200);
        const relSlugs = (relRes.json?.data?.items || []).map((i) => i.slug);
        assert.assertNotIncludes(
          relSlugs,
          sponsoredSlug,
          "Sponsored advertorial must be excluded from organic related recommendations"
        );
      }
    );
  });
}
