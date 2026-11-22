import crypto from "node:crypto";
import path from "node:path";

let seqCounter = 0;

/**
 * Generate a collision-free unique identifier safe for shared-DB runs.
 */
export function uid(prefix = "e2e") {
  seqCounter += 1;
  const rand = crypto.randomBytes(3).toString("hex");
  return `${prefix}-${Date.now().toString(36)}-${seqCounter}-${rand}`;
}

/**
 * Generate a unique positive integer issue number well above seeded issues (~1..300).
 */
export function uniqueIssueNumber() {
  seqCounter += 1;
  const rand = crypto.randomInt(100, 999);
  return 70000 + (seqCounter * 1000) + rand;
}

/**
 * Build a structurally valid multi-page `%PDF-1.4` document with accurate xref byte offsets
 * so `lopdf` and `edergi_pdf_service` can parse page count, slice pages, and watermark cleanly.
 */
export function makeMinimalPdfBuffer(pageCount = 4, title = "MSI E2E Test Issue") {
  const count = Math.max(1, Math.min(Number(pageCount) || 4, 50));
  const objects = [];

  // Object 1: Catalog
  objects.push(`1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n`);

  // Object 2: Pages
  const kidsRefs = [];
  for (let i = 0; i < count; i++) {
    kidsRefs.push(`${3 + i * 2} 0 R`);
  }
  objects.push(
    `2 0 obj\n<< /Type /Pages /Kids [ ${kidsRefs.join(" ")} ] /Count ${count} >>\nendobj\n`
  );

  // Page + Content stream pairs
  const fontObjNum = 3 + count * 2;
  for (let i = 0; i < count; i++) {
    const pageObjNum = 3 + i * 2;
    const contentObjNum = pageObjNum + 1;
    const safeTitle = String(title).replace(/[()\\]/g, "");
    const streamBody = `BT /F1 14 Tf 72 720 Td (${safeTitle} - Page ${i + 1}) Tj ET\n`;
    const streamLen = Buffer.byteLength(streamBody, "utf8");
    objects.push(
      `${pageObjNum} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 ${fontObjNum} 0 R >> >> /Contents ${contentObjNum} 0 R >>\nendobj\n`
    );
    objects.push(
      `${contentObjNum} 0 obj\n<< /Length ${streamLen} >>\nstream\n${streamBody}endstream\nendobj\n`
    );
  }

  // Font object
  objects.push(
    `${fontObjNum} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`
  );

  const chunks = [];
  const header = Buffer.from("%PDF-1.4\n%\xE2\xE3\xCF\xD3\n", "latin1");
  chunks.push(header);
  let offset = header.length;
  const offsets = [0];

  for (const obj of objects) {
    offsets.push(offset);
    const buf = Buffer.from(obj, "utf8");
    chunks.push(buf);
    offset += buf.length;
  }

  const xrefOffset = offset;
  let xref = `xref\n0 ${offsets.length}\n`;
  xref += "0000000000 65535 f \n";
  for (let i = 1; i < offsets.length; i++) {
    xref += `${String(offsets[i]).padStart(10, "0")} 00000 n \n`;
  }
  xref += `trailer\n<< /Size ${offsets.length} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  chunks.push(Buffer.from(xref, "utf8"));

  return Buffer.concat(chunks);
}

/**
 * Build an `http.mjs`-compatible multipart PDF file part (`{ content, filename, contentType }`).
 */
export function makePdfPart(filename = "e2e-issue.pdf", buffer = makeMinimalPdfBuffer(4)) {
  const buf = Buffer.isBuffer(buffer) ? buffer : Buffer.from(buffer);
  return {
    content: buf,
    filename,
    contentType: "application/pdf",
  };
}

/**
 * Valid 1x1 transparent PNG bytes (`\x89PNG\r\n\x1a\n` signature + IHDR/IDAT/IEND).
 */
export function makeTinyPngPart(filename = "e2e-sample.png") {
  const pngBytes = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=",
    "base64"
  );
  return {
    content: pngBytes,
    filename,
    contentType: "image/png",
  };
}

/**
 * Valid minimal RIFF/WEBP lossless 1x1 image bytes.
 */
export function makeTinyWebpPart(filename = "e2e-sample.webp") {
  const webpBytes = Buffer.from(
    "UklGRh4AAABXRUJQVlA4TBEAAAAvAAAAAAfQ//73v/+BiOh/AAA=",
    "base64"
  );
  return {
    content: webpBytes,
    filename,
    contentType: "image/webp",
  };
}

/**
 * Snapshot `/api/v1/admin/design` before running `fn(snapshot)` and always restore in `finally`.
 */
export async function withDesignConfigBackup(adminClient, fn) {
  await adminClient.ensureCsrf();
  const snapRes = await adminClient.get("/api/v1/admin/design");
  if (snapRes.status !== 200 || !snapRes.json?.config) {
    throw new Error(
      `Failed to snapshot design config: HTTP ${snapRes.status} (${snapRes.text?.slice(0, 160)})`
    );
  }
  const originalConfig = structuredClone(snapRes.json.config);
  try {
    return await fn(originalConfig, snapRes.json);
  } finally {
    await adminClient.ensureCsrf();
    const restoreRes = await adminClient.post("/api/v1/admin/design", {
      json: originalConfig,
    });
    if (restoreRes.status !== 200) {
      throw new Error(
        `Failed to restore design config after test: HTTP ${restoreRes.status} (${restoreRes.text?.slice(0, 160)})`
      );
    }
  }
}

/**
 * Verify user email through opaque-box HTTP endpoints.
 *
 * - If `{ token, client }` is supplied, confirms via `GET /hesap-dogrula?token=...&confirm=1`.
 * - If `user` already has `email_verified === true` (e.g. provisioned via `POST /api/v1/admin/users`),
 *   returns `{ verified: true, user }`.
 * - Never touches SQLite directly. If a user is unverified and no token is supplied, throws a clear
 *   error explaining how to provision a verified subscriber (`createVerifiedSubscriber`).
 */
export async function verifyUserEmail(ctx, userOrOpts) {
  if (userOrOpts && typeof userOrOpts === "object" && userOrOpts.token) {
    const res = await ctx.auth.verifyEmailViaToken(
      userOrOpts.token,
      userOrOpts.client || null
    );
    ctx.assert.assertStatus(res, 200, "Email verification via token must return 200");
    return { verified: true, response: res };
  }

  const candidate = userOrOpts?.user || userOrOpts;
  if (candidate && candidate.email_verified === true) {
    return { verified: true, user: candidate };
  }

  throw new Error(
    "Cannot verify an unverified email without a verification token in opaque-box mode; use createVerifiedSubscriber(ctx, role, overrides) or supply { token }."
  );
}

/**
 * Provision an email-verified subscriber (`email_verified = 1` via `POST /api/v1/admin/users`)
 * and optionally set canonical SSB interests / profile details.
 */
export async function createVerifiedSubscriber(ctx, role = "subscriber_public", overrides = {}) {
  const provisioned = await ctx.auth.createUserWithRole(role, overrides);
  if (Array.isArray(overrides.ssb_interests) && overrides.ssb_interests.length > 0) {
    const profRes = await provisioned.client.post("/api/v1/account/profile", {
      json: {
        full_name: provisioned.user?.full_name || `E2E ${role}`,
        organization_category: overrides.organization_category || provisioned.user?.organization_category,
        organization_name: overrides.organization_name || provisioned.user?.organization_name,
        job_title: overrides.job_title || provisioned.user?.job_title,
        ssb_interests: overrides.ssb_interests,
        email_consent: overrides.email_consent ?? true,
      },
    });
    ctx.assert.assertStatus(profRes, 200, "Updating subscriber SSB interests should return 200");
    provisioned.user = profRes.json?.data?.user || provisioned.user;
  }
  return provisioned;
}

/**
 * Seed `msi_cookie_consent` in localStorage so `msi-ui.js` emits `/api/v1/analytics/track` beacons.
 * Works on a Playwright `BrowserContext` (via `addInitScript`) or an open `Page`.
 */
export async function seedCookieConsent(
  target,
  consent = { necessary: true, analytics: true, marketing: true }
) {
  const payload = JSON.stringify({
    necessary: consent.necessary !== false,
    analytics: Boolean(consent.analytics),
    marketing: Boolean(consent.marketing),
    timestamp: new Date().toISOString(),
  });
  if (typeof target?.addInitScript === "function") {
    await target.addInitScript((serialized) => {
      try {
        window.localStorage.setItem("msi_cookie_consent", serialized);
      } catch {
        // ignore non-storage origins
      }
    }, payload);
    return;
  }
  if (typeof target?.evaluate === "function") {
    await target.evaluate((serialized) => {
      try {
        window.localStorage.setItem("msi_cookie_consent", serialized);
      } catch {
        // ignore
      }
    }, payload);
    return;
  }
  throw new Error("seedCookieConsent requires a Playwright BrowserContext or Page");
}

/**
 * Open a Playwright page with consistent `host` binding on both context & navigation,
 * optional cookie consent pre-seeding, and guaranteed context creation/cleanup helper.
 */
export async function openDomainPage(ctx, pathOrUrl, options = {}) {
  const host = options.host || null;
  const context =
    options.context ||
    (await ctx.browser.newContext({
      viewport: options.viewport || ctx.browser.VIEWPORT_DESKTOP,
      locale: options.locale || "tr-TR",
      extraHTTPHeaders: options.extraHTTPHeaders,
      host,
      client: options.client,
      credentials: options.credentials,
    }));
  const ownContext = !options.context;

  try {
    if (options.seedConsent) {
      const consentVal =
        typeof options.seedConsent === "object"
          ? options.seedConsent
          : { necessary: true, analytics: true, marketing: true };
      await seedCookieConsent(context, consentVal);
    }

    const pageRes = await ctx.browser.openPage(pathOrUrl, {
      context,
      host,
      waitUntil: options.waitUntil || "networkidle",
      timeout: options.timeout ?? 25000,
      settleMs: options.settleMs,
      autoAcceptDialogs: options.autoAcceptDialogs,
    });

    const origClose = pageRes.close.bind(pageRes);
    pageRes.close = async () => {
      await origClose();
      if (ownContext) {
        await context.close().catch(() => {});
      }
    };
    return pageRes;
  } catch (err) {
    if (ownContext) {
      await context.close().catch(() => {});
    }
    throw err;
  }
}

/**
 * Convenience wrapper ensuring `pageRes.close()` always runs in `finally`.
 */
export async function withDomainPage(ctx, pathOrUrl, options, fn) {
  const pageRes = await openDomainPage(ctx, pathOrUrl, options);
  try {
    return await fn(pageRes);
  } finally {
    await pageRes.close();
  }
}

/**
 * Assert zero uncaught pageErrors, zero >=500 responses, and only expected console error patterns
 * (e.g. intentional 401/403/409 network logs on negative UI tests).
 */
export function assertNoUnexpectedBrowserErrors(
  ctx,
  pageResult,
  allowedConsolePatterns = [],
  message = "Unexpected browser errors detected"
) {
  const { assert } = ctx;
  assert.assertOk(pageResult && typeof pageResult === "object", `${message}: invalid pageResult`);

  const pageErrors = pageResult.pageErrors || [];
  const serverErrors = pageResult.serverErrors || [];
  const consoleErrors = (pageResult.consoleErrors || []).filter((line) => {
    return !allowedConsolePatterns.some((pat) =>
      pat instanceof RegExp ? pat.test(line) : String(line).includes(String(pat))
    );
  });

  if (consoleErrors.length > 0 || pageErrors.length > 0 || serverErrors.length > 0) {
    const parts = [];
    if (consoleErrors.length > 0) parts.push(`consoleErrors=[${consoleErrors.join(" | ")}]`);
    if (pageErrors.length > 0) parts.push(`pageErrors=[${pageErrors.join(" | ")}]`);
    if (serverErrors.length > 0) {
      parts.push(`serverErrors=[${serverErrors.map((e) => `${e.status} ${e.url}`).join(" | ")}]`);
    }
    assert.assertOk(false, `${message}: ${parts.join("; ")}`);
  }
}

/**
 * Audit a live Playwright UI surface:
 * - Horizontal overflow (`scrollWidth <= innerWidth`)
 * - All visible `<img>` elements decoded (`complete && naturalWidth > 0`)
 * - Non-empty visible text & heading/typography hierarchy
 * - Browser console / uncaught pageerror / >=500 errors
 */
export async function auditUiSurface(ctx, pageRes, opts = {}) {
  const { assert } = ctx;
  const label = opts.label || pageRes.url || "page";

  const defaultAllowedNet = !ctx.isExternalTarget ? ["/api/v1/admin/monitoring/bundle"] : [];
  const allowedNetworkPatterns = [
    ...defaultAllowedNet,
    ...(opts.allowedNetworkPatterns || []),
    ...(opts.allowedConsolePatterns || []),
  ];

  if (opts.checkNetworkErrors === false) {
    if (opts.allowedConsolePatterns) {
      assertNoUnexpectedBrowserErrors(
        ctx,
        pageRes,
        opts.allowedConsolePatterns,
        `${label} unexpected browser errors`
      );
    } else {
      assert.assertNoBrowserErrors(pageRes, `${label} browser/5xx errors`);
    }
  } else {
    assert.assertNoAssetOrNetworkErrors(
      pageRes,
      allowedNetworkPatterns,
      `${label} browser/asset/network errors`
    );
  }

  const overflow = await pageRes.measureOverflow();
  assert.assertNoOverflow(overflow, `${label} horizontal overflow`);

  const audit = await pageRes.page.evaluate(async () => {
    const imgs = Array.from(document.querySelectorAll("img"));
    const brokenImages = [];
    for (const img of imgs) {
      const rects = img.getClientRects();
      if (!rects || rects.length === 0) continue;
      const style = window.getComputedStyle(img);
      if (style.display === "none" || style.visibility === "hidden") continue;
      if (img.loading === "lazy") {
        img.loading = "eager";
      }
      if (!img.complete) {
        await img.decode().catch(() => {});
      }
      if (!img.complete || img.naturalWidth <= 0) {
        brokenImages.push({
          src: img.getAttribute("src") || "",
          complete: img.complete,
          naturalWidth: img.naturalWidth,
        });
      }
    }

    const bodyText = (document.body?.innerText || "").trim();
    const visibleH1 = Array.from(document.querySelectorAll("h1")).filter((el) => {
      const st = window.getComputedStyle(el);
      return el.getClientRects().length > 0 && st.display !== "none" && st.visibility !== "hidden";
    });
    const visibleHeadings = Array.from(document.querySelectorAll("h1, h2, h3")).filter((el) => {
      const st = window.getComputedStyle(el);
      return el.getClientRects().length > 0 && st.display !== "none" && st.visibility !== "hidden";
    });
    const h1FontSize = visibleH1[0] ? parseFloat(window.getComputedStyle(visibleH1[0]).fontSize) : 0;
    const bodyFontSize = document.body
      ? parseFloat(window.getComputedStyle(document.body).fontSize)
      : 0;
    const mainCount = document.querySelectorAll("main").length;

    return {
      brokenImages,
      textLen: bodyText.length,
      visibleH1Count: visibleH1.length,
      visibleHeadingCount: visibleHeadings.length,
      h1FontSize,
      bodyFontSize,
      mainCount,
    };
  });

  assert.assertEqual(
    audit.brokenImages.length,
    0,
    `${label} has broken visible images: ${JSON.stringify(audit.brokenImages)}`
  );
  assert.assertOk(audit.textLen > 20, `${label} rendered empty or near-empty body text`);

  if (opts.requireHeading !== false) {
    assert.assertOk(
      audit.visibleHeadingCount >= 1,
      `${label} must render at least one visible heading (h1/h2/h3)`
    );
  }
  if (opts.requireSingleH1) {
    assert.assertEqual(
      audit.visibleH1Count,
      1,
      `${label} should have exactly 1 visible <h1>`
    );
    if (audit.bodyFontSize > 0) {
      assert.assertOk(
        audit.h1FontSize >= audit.bodyFontSize,
        `${label} h1 font-size (${audit.h1FontSize}) should be >= body font-size (${audit.bodyFontSize})`
      );
    }
  }
  if (opts.requireMain) {
    assert.assertOk(audit.mainCount >= 1, `${label} must include a <main> landmark`);
  }

  return audit;
}

/**
 * Verify an interactive element is visible, has a non-zero bounding box, and fits within viewport.
 */
export async function checkInteractiveVisibilityAndLayout(ctx, page, selector, opts = {}) {
  const { assert } = ctx;
  const label = opts.label || selector;
  const locator = page.locator(selector).first();
  await locator.waitFor({ state: "visible", timeout: opts.timeout ?? 10000 });
  const metrics = await locator.evaluate((el) => {
    const r = el.getBoundingClientRect();
    const st = window.getComputedStyle(el);
    return {
      width: r.width,
      height: r.height,
      left: r.left,
      right: r.right,
      innerWidth: window.innerWidth,
      pointerEvents: st.pointerEvents,
      visibility: st.visibility,
      opacity: parseFloat(st.opacity),
    };
  });
  assert.assertOk(
    metrics.width > 0 && metrics.height > 0,
    `${label} must have non-zero bounding box (got ${metrics.width}x${metrics.height})`
  );
  assert.assertOk(
    metrics.visibility !== "hidden" && metrics.opacity > 0 && metrics.pointerEvents !== "none",
    `${label} must be visually interactive (${JSON.stringify(metrics)})`
  );
  if (opts.checkInViewport !== false) {
    assert.assertOk(
      metrics.left >= -2 && metrics.right <= metrics.innerWidth + 2,
      `${label} overflows horizontal viewport: left=${metrics.left}, right=${metrics.right}, innerWidth=${metrics.innerWidth}`
    );
  }
  return metrics;
}

/**
 * Helper to create a content item via `/api/v1/admin/contents` with safe defaults.
 */
export async function createAdminContent(ctx, adminClient, overrides = {}) {
  await adminClient.ensureCsrf();
  const slug = overrides.slug || uid("e2e-content");
  const payload = {
    id: 0,
    slug,
    title_tr: overrides.title_tr || `E2E Savunma İçeriği ${slug}`,
    title_en: overrides.title_en || `E2E Defense Briefing ${slug}`,
    summary_tr: overrides.summary_tr || "Kısa savunma sanayii özet metni.",
    summary_en: overrides.summary_en || "Brief defense industry summary.",
    body_tr:
      overrides.body_tr ||
      "<p>İlk paragraf: Türk savunma sanayii radar ve hava platformlarında yeni nesil yerli alt sistemler geliştiriyor.</p><p>İkinci paragraf: Aviyonik ve elektronik harp mimarileri yüksek yerlilik oranıyla test ediliyor.</p><p>Üçüncü paragraf: Entegre lojistik destek süreçleri sahadaki hazırlık seviyesini artırıyor.</p><p>Dördüncü paragraf: İhracat odaklı ortaklıklar uluslararası fuarlarda öne çıkıyor.</p>",
    body_en:
      overrides.body_en ||
      "<p>First paragraph: Indigenous defense platforms advance next-generation radar and avionics suites.</p><p>Second paragraph: Electronic warfare subsystems undergo rigorous environmental qualification.</p><p>Third paragraph: Integrated logistics support enhances operational readiness across units.</p><p>Fourth paragraph: International defense partnerships expand export footprints.</p>",
    content_type: overrides.content_type || "haberler",
    source_group: overrides.source_group || "sadece-online",
    cover_image: overrides.cover_image || "images/msi/kaan.webp",
    author_name: overrides.author_name || "E2E Editör",
    author_title: overrides.author_title || "Savunma Analisti",
    issue_number: overrides.issue_number ?? null,
    published_at: overrides.published_at || "2026-11-20 12:00:00",
    read_time_min: overrides.read_time_min ?? 6,
    visibility_level: overrides.visibility_level ?? 0,
    single_price_try: overrides.single_price_try ?? 0.0,
    is_sponsored: overrides.is_sponsored ?? false,
    sponsor_brand: overrides.sponsor_brand ?? null,
    sponsor_legal_label: overrides.sponsor_legal_label ?? null,
    is_featured: overrides.is_featured ?? false,
    video_url: overrides.video_url ?? null,
    view_count: 0,
    true_read_count: 0,
    placements: overrides.placements || [overrides.content_type || "haberler"],
    ssb_tags: overrides.ssb_tags || ["hava-araclari"],
    status: overrides.status || "published",
    en_status: overrides.en_status || "approved",
    target_institutions: overrides.target_institutions || [],
  };

  const res = await adminClient.post("/api/v1/admin/contents", { json: payload });
  if (!overrides.allowError) {
    ctx.assert.assertStatus(res, 200, `Failed to create content ${slug}`);
  }
  return {
    res,
    id: res.json?.data?.id ?? null,
    slug: res.json?.data?.slug || slug,
    payload,
  };
}

/**
 * Helper to create an ad campaign via `/api/v1/admin/ads` with safe defaults.
 */
export async function createAdminAd(ctx, adminClient, overrides = {}) {
  await adminClient.ensureCsrf();
  const sponsorName = overrides.sponsor_name || uid("E2ESponsor");
  const payload = {
    id: overrides.id ?? 0,
    slot_code: overrides.slot_code || "sidebar_300x250",
    badge_label: overrides.badge_label || "SPONSORLU",
    dimensions: overrides.dimensions || "300x250",
    sponsor_name: sponsorName,
    sponsor_type: overrides.sponsor_type || "defense_oem",
    pill_style: overrides.pill_style || "navy",
    title_tr: overrides.title_tr || `${sponsorName} Çözümü`,
    title_en: overrides.title_en || `${sponsorName} Solution`,
    desc_tr: overrides.desc_tr || "Yerli savunma alt sistem çözümü.",
    desc_en: overrides.desc_en || "Indigenous defense subsystem solution.",
    target_url: overrides.target_url || "https://msidergisi.com/e2e-sponsor",
    image_url: overrides.image_url ?? null,
    budget_score: overrides.budget_score ?? 80,
    is_default_fallback: overrides.is_default_fallback ?? false,
    is_floating_popup: overrides.is_floating_popup ?? false,
    is_newsletter_ad: overrides.is_newsletter_ad ?? false,
    target_categories: overrides.target_categories || [],
    target_ssb_tags: overrides.target_ssb_tags || [],
    target_institutions: overrides.target_institutions || [],
    impressions_count: 0,
    clicks_count: 0,
    is_active: overrides.is_active ?? true,
    start_date: overrides.start_date ?? null,
    end_date: overrides.end_date ?? null,
    rotation_mode: overrides.rotation_mode || "weighted",
    ...(overrides.force_override !== undefined
      ? { force_override: overrides.force_override }
      : {}),
  };

  const res = await adminClient.post("/api/v1/admin/ads", { json: payload });
  if (!overrides.allowError && !overrides.allowConflict) {
    ctx.assert.assertStatus(res, 200, `Failed to create ad ${sponsorName}`);
  }
  return {
    res,
    id: res.json?.data?.id ?? null,
    sponsorName,
    payload,
  };
}

/**
 * Helper to fetch `/api/v1/admin/invoices` and locate an invoice by `invoice_no` or `id`.
 */
export async function findAdminInvoice(adminClient, { invoiceNo = null, invoiceId = null } = {}) {
  const res = await adminClient.get("/api/v1/admin/invoices", {
    query: { per_page: "200" },
  });
  const list = Array.isArray(res.json?.data) ? res.json.data : [];
  return (
    list.find(
      (inv) =>
        (invoiceId && inv.id === invoiceId) ||
        (invoiceNo && inv.invoice_no === invoiceNo)
    ) || null
  );
}

/**
 * Compute PayTR callback HMAC-SHA256 hash matching `payment_service::verify_callback_hash`:
 * `base64(HMAC-SHA256(merchant_key, merchant_oid + merchant_salt + status + total_amount))`
 */
export function computePaytrCallbackHash({
  merchantKey,
  merchantSalt,
  merchantOid,
  status,
  totalAmount,
}) {
  const data = `${merchantOid}${merchantSalt}${status}${totalAmount}`;
  return crypto.createHmac("sha256", merchantKey).update(data).digest("base64");
}

export function resolveRepoFixturePath(...segments) {
  return path.resolve(process.cwd(), ...segments);
}
