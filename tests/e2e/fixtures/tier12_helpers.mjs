import crypto from "node:crypto";
import zlib from "node:zlib";

let seq = 0;

/**
 * Unique identifier generator for isolated E2E test entities.
 */
export function uid(prefix = "e2e") {
  seq += 1;
  const rand = crypto.randomBytes(3).toString("hex");
  return `${prefix}_${Date.now()}_${seq}_${rand}`;
}

export function uniqueEmail(prefix = "e2e", domain = "example.com") {
  return `${uid(prefix).toLowerCase()}@${domain}`;
}

export function uniqueSlug(prefix = "e2e-item") {
  return uid(prefix).toLowerCase().replace(/[^a-z0-9-]+/g, "-");
}

/**
 * Anti-bot rendered timestamp (`_form_ts`) offset in seconds (default 5s ago).
 */
export function formTs(offsetSec = -5) {
  return Math.floor(Date.now() / 1000) + offsetSec;
}

export function antiBotFields(offsetSec = -5) {
  return {
    website_url: "",
    _form_ts: formTs(offsetSec),
  };
}

/**
 * Unwrap both enveloped `{ status: "ok", data: ... }` and raw JSON responses.
 */
export function unwrap(res) {
  if (!res || res.json === null || res.json === undefined) return null;
  if (typeof res.json === "object" && !Array.isArray(res.json) && "data" in res.json && res.json.data !== undefined) {
    return res.json.data;
  }
  return res.json;
}

/**
 * Build a genuine, byte-offset-accurate %PDF-1.4 document with `pagesCount` pages.
 */
export function buildMinimalPdf(pagesCount = 3, title = "MSI E2E Magazine Issue") {
  const count = Math.max(1, Math.min(200, Number(pagesCount) || 1));
  const sanitizePdfText = (s) =>
    String(s)
      .replace(/[^\x20-\x7e]/g, "_")
      .replace(/[()\\]/g, "\\$&");
  const safeTitle = sanitizePdfText(title);

  const pageIds = [];
  for (let i = 0; i < count; i++) {
    pageIds.push(4 + i * 2);
  }

  const objects = [
    { id: 1, body: `1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n` },
    {
      id: 2,
      body: `2 0 obj\n<< /Type /Pages /Kids [${pageIds.map((id) => `${id} 0 R`).join(" ")}] /Count ${count} >>\nendobj\n`,
    },
    {
      id: 3,
      body: `3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n`,
    },
  ];

  for (let i = 0; i < count; i++) {
    const pageId = 4 + i * 2;
    const contentId = 5 + i * 2;
    const streamStr = `BT\n/F1 12 Tf\n72 720 Td\n(${safeTitle} - Page ${i + 1} of ${count}) Tj\nET\n`;
    const streamLen = Buffer.byteLength(streamStr, "latin1");
    objects.push({
      id: pageId,
      body: `${pageId} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents ${contentId} 0 R >>\nendobj\n`,
    });
    objects.push({
      id: contentId,
      body: `${contentId} 0 obj\n<< /Length ${streamLen} >>\nstream\n${streamStr}endstream\nendobj\n`,
    });
  }

  const chunks = [];
  let offset = 0;
  const headerBuf = Buffer.from("%PDF-1.4\n%\xE2\xE3\xCF\xD3\n", "latin1");
  chunks.push(headerBuf);
  offset += headerBuf.length;

  const totalObjs = objects.length + 1;
  const offsets = new Array(totalObjs).fill(0);
  for (const obj of objects) {
    offsets[obj.id] = offset;
    const buf = Buffer.from(obj.body, "latin1");
    chunks.push(buf);
    offset += buf.length;
  }

  const xrefOffset = offset;
  let xref = `xref\n0 ${totalObjs}\n0000000000 65535 f \n`;
  for (let id = 1; id < totalObjs; id++) {
    xref += `${String(offsets[id]).padStart(10, "0")} 00000 n \n`;
  }
  const trailer = `trailer\n<< /Size ${totalObjs} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF\n`;
  chunks.push(Buffer.from(xref + trailer, "latin1"));
  return Buffer.concat(chunks);
}

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    c ^= buf[i];
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
  }
  return (c ^ 0xffffffff) >>> 0;
}

function pngChunk(type, data) {
  const typeBuf = Buffer.from(type, "ascii");
  const lenBuf = Buffer.alloc(4);
  lenBuf.writeUInt32BE(data.length, 0);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([lenBuf, typeBuf, data, crcBuf]);
}

/**
 * Build a genuine 1x1 RGBA PNG binary Buffer.
 */
export function buildMinimalPng() {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(1, 0); // width
  ihdr.writeUInt32BE(1, 4); // height
  ihdr[8] = 8; // bit depth
  ihdr[9] = 6; // RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  // Filter byte 0 + 1 RGBA pixel (navy blue)
  const rawScanline = Buffer.from([0x00, 0x10, 0x24, 0x48, 0xff]);
  const idat = zlib.deflateSync(rawScanline);
  return Buffer.concat([
    sig,
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", idat),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

/**
 * Build a minimal valid 1x1 lossless WebP (RIFF/WEBP/VP8L) binary Buffer.
 */
export function buildMinimalWebp() {
  // Valid 1x1 transparent lossless VP8L bitstream (12 bytes)
  const vp8lData = Buffer.from([
    0x2f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00,
  ]);
  const riffSize = 4 + 8 + vp8lData.length;
  const out = Buffer.alloc(8 + riffSize);
  out.write("RIFF", 0, "ascii");
  out.writeUInt32LE(riffSize, 4);
  out.write("WEBP", 8, "ascii");
  out.write("VP8L", 12, "ascii");
  out.writeUInt32LE(vp8lData.length, 16);
  vp8lData.copy(out, 20);
  return out;
}

/**
 * Compute PayTR callback HMAC-SHA256 hash matching src/services/payment_service.rs.
 */
export function computePaytrHash(merchantOid, merchantKey, merchantSalt, status, totalAmountKurus) {
  const payload = `${merchantOid}${merchantSalt}${status}${totalAmountKurus}`;
  return crypto.createHmac("sha256", merchantKey).update(payload).digest("base64");
}

export function toMerchantOid(invoiceNo) {
  return `MSI${String(invoiceNo || "").replace(/[^a-zA-Z0-9]/g, "")}`;
}

const serverCaches = new Map();

function getServerCache(ctx) {
  const key = ctx.baseUrl;
  let entry = serverCaches.get(key);
  if (!entry) {
    entry = {
      superAdmin: null,
      admin: null,
      roleSessions: new Map(),
    };
    serverCaches.set(key, entry);
  }
  return entry;
}

/**
 * Cached reusable super_admin session per isolated server instance.
 * Never mutate/lock out this seeded account!
 */
export async function getCachedSuperAdmin(ctx) {
  const cache = getServerCache(ctx);
  if (!cache.superAdmin) {
    cache.superAdmin = await ctx.auth.loginAsSuperAdmin();
  }
  await cache.superAdmin.client.ensureCsrf();
  return cache.superAdmin;
}

/**
 * Cached reusable site_operator session per isolated server instance.
 * Never mutate/lock out this seeded account!
 */
export async function getCachedAdmin(ctx) {
  const cache = getServerCache(ctx);
  if (!cache.admin) {
    cache.admin = await ctx.auth.loginAsAdmin();
  }
  await cache.admin.client.ensureCsrf();
  return cache.admin;
}

/**
 * Provision a fresh disposable user with any role using the cached super_admin client.
 */
export async function createDisposableRoleUser(ctx, role, overrides = {}) {
  const sa = await getCachedSuperAdmin(ctx);
  return await ctx.auth.createUserWithRole(role, {
    superAdminClient: sa.client,
    ...overrides,
  });
}

/**
 * Cached read-only role session for matrix checks (returns fresh client for UNAUTHENTICATED).
 * Do NOT use for destructive/lockout/profile-mutation tests — use createDisposableRoleUser for those.
 */
export async function getCachedRoleSession(ctx, role) {
  if (!role || role === ctx.auth.UNAUTHENTICATED) {
    return await ctx.auth.createUserWithRole(ctx.auth.UNAUTHENTICATED);
  }
  const cache = getServerCache(ctx);
  if (!cache.roleSessions.has(role)) {
    const sa = await getCachedSuperAdmin(ctx);
    const session = await ctx.auth.createUserWithRole(role, {
      superAdminClient: sa.client,
      subscription_tier:
        role === "subscriber_digital"
          ? "dijital"
          : role === "subscriber_print_digital"
            ? "basili-dijital"
            : role === "subscriber_corporate"
              ? "kurumsal"
              : "",
    });
    cache.roleSessions.set(role, session);
  }
  const session = cache.roleSessions.get(role);
  await session.client.ensureCsrf();
  return session;
}

/**
 * Canonical 14 SSB tags seeded in migrations/seed.
 */
export const CANONICAL_SSB_TAGS = Object.freeze([
  "kara-araclari",
  "deniz-araclari",
  "hava-araclari",
  "muhimmatlar-ve-fuze-sistemleri",
  "namlulu-silahlar",
  "haberlesme-ve-muharebe-destek-sistemleri",
  "radarlar-ve-sensorler",
  "simulatorler-ve-egitim-sistemleri",
  "elektronik-harp-sistemleri",
  "itki-tahrik-ve-guc-sistemleri",
  "hizmetler-danismanlik",
  "hizmetler-egitim",
  "hizmetler-kurumsal-iletisim",
  "hizmetler-kurumsal-catering",
]);

/**
 * Struct-complete payload builders matching Rust serde models in src/models/*.rs.
 */
export function makeContentPayload(overrides = {}) {
  const slug = overrides.slug || uniqueSlug("e2e-cnt");
  return {
    id: 0,
    slug,
    title_tr: `E2E Icerik ${slug}`,
    title_en: `E2E Content ${slug}`,
    summary_tr: "E2E yonetici ozeti.",
    summary_en: "E2E executive summary.",
    body_tr: "<p>E2E teknik icerik metni.</p>",
    body_en: "<p>E2E technical content body.</p>",
    content_type: "haberler",
    source_group: "gunluk-haber",
    cover_image: "images/msi/dergi-286.webp",
    author_name: "MSI Editoryal Masasi",
    author_title: "Savunma Analisti",
    issue_number: null,
    published_at: new Date().toISOString().slice(0, 16).replace("T", " "),
    read_time_min: 5,
    visibility_level: 0,
    single_price_try: 0,
    is_sponsored: false,
    sponsor_brand: null,
    sponsor_legal_label: null,
    is_featured: false,
    video_url: null,
    view_count: 0,
    true_read_count: 0,
    placements: ["haberler"],
    ssb_tags: [],
    status: "published",
    target_institutions: [],
    ...overrides,
  };
}

export function makeAdPayload(overrides = {}) {
  return {
    id: 0,
    slot_code: "ad-news-infeed",
    badge_label: "SPONSORLU",
    dimensions: "728x90",
    sponsor_name: `Sponsor_${uid("sp")}`,
    sponsor_type: "auxiliary_service",
    pill_style: "blue",
    title_tr: "E2E Kampanya",
    title_en: "E2E Campaign",
    desc_tr: "Kampanya alt metni",
    desc_en: "Campaign subtitle",
    target_url: "https://msidergisi.com",
    image_url: null,
    budget_score: 80,
    is_default_fallback: false,
    is_floating_popup: false,
    is_newsletter_ad: false,
    target_categories: [],
    target_ssb_tags: [],
    target_institutions: [],
    impressions_count: 0,
    clicks_count: 0,
    is_active: true,
    start_date: null,
    end_date: null,
    rotation_mode: "weighted",
    ...overrides,
  };
}

export function makeIssuePayload(overrides = {}) {
  const issueNumber = overrides.issue_number ?? 900;
  const pubType = overrides.publication_type || "msi";
  return {
    issue_number: issueNumber,
    slug: overrides.slug || `${pubType === "msi_tdr" ? "msi-tdr-issue" : "msi-dergisi-sayi"}-${issueNumber}`,
    title_tr: `MSI Sayi ${issueNumber}`,
    title_en: `MSI Issue ${issueNumber}`,
    publish_month_tr: "Kasim 2026",
    publish_month_en: "November 2026",
    year: 2026,
    cover_image: "images/msi/dergi-286.webp",
    is_special_issue: false,
    event_name: null,
    page_count: 16,
    dossier_count: 0,
    summary_tr: "Sayi ozeti",
    summary_en: "Issue summary",
    pdf_path: `/api/v1/edergi/${issueNumber}/download-pdf`,
    publication_type: pubType,
    uploaded_pdf_file: null,
    spreads: [],
    ...overrides,
  };
}

export function makePlanPayload(overrides = {}) {
  const code = overrides.code || `e2e-plan-${uid("pl").toLowerCase().replace(/[^a-z0-9-]/g, "-")}`;
  return {
    code,
    name_tr: "E2E Paket",
    name_en: "E2E Plan",
    price_try: 3500,
    period_label_tr: "Yıllık",
    period_label_en: "Annual",
    features_tr: ["Tum arsiv erisimi"],
    features_en: ["Full archive access"],
    is_featured: false,
    ...overrides,
  };
}

export function makeProductPayload(overrides = {}) {
  const sku = overrides.sku || `E2E-SKU-${uid("p").toUpperCase()}`;
  return {
    id: 0,
    sku,
    product_type: "book",
    title_tr: "E2E Savunma Kitabi",
    title_en: "E2E Defence Book",
    author_or_source: "MSI Yayinlari",
    description_tr: "Test aciklama",
    description_en: "Test description",
    price_try: 490,
    cover_image: "images/msi/dergi-286.webp",
    file_resolution_or_pages: "240 sayfa",
    is_available: true,
    ...overrides,
  };
}

export function makeRedirectPayload(overrides = {}) {
  return {
    id: 0,
    old_path: `/legacy-${uid("rd").toLowerCase()}.html`,
    new_path: "/e-dergi",
    status_code: 301,
    hit_count: 0,
    ...overrides,
  };
}

export function makeGlossaryPayload(overrides = {}) {
  return {
    id: 0,
    term_tr: `Terim ${uid("gl")}`,
    term_en: "Defense Term",
    domain: "defense",
    preferred_en: "Defense Term",
    notes: "E2E glossary",
    is_active: true,
    ...overrides,
  };
}

export function makeLocalePayload(overrides = {}) {
  const code = overrides.code || "de";
  return {
    code,
    name_native: "Deutsch",
    name_en: "German",
    hreflang: code,
    og_locale: `${code}_${code.toUpperCase()}`,
    dir: "ltr",
    is_default: false,
    is_enabled: true,
    fallback_code: "tr",
    sort_order: 10,
    ...overrides,
  };
}

export async function getSeededContentSlug(ctx, preferNonSponsored = true) {
  const client = ctx.http.createClient();
  const res = await client.get("/api/v1/contents");
  const list = Array.isArray(res.json) ? res.json : [];
  const item = preferNonSponsored ? list.find((c) => !c.is_sponsored) || list[0] : list[0];
  return item?.slug || "ctech-ve-skyline-uydu-haberlesmesinde-is-birligi-yapacak";
}

