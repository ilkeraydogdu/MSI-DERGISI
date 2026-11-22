import fs from "node:fs";
import path from "node:path";
import { AssertionFailure } from "../lib/assertions.mjs";
import { captureScreenshot } from "../lib/browser.mjs";

export const ROLE_A_TOKENS = Object.freeze({
  light: Object.freeze({
    "--ui-brand-bg": "#f6f3ee",
    "--ui-brand-surface": "#fcfbf9",
    "--ui-brand-accent": "#cb382a",
  }),
  dark: Object.freeze({
    "--ui-brand-bg": "#1d222c",
    "--ui-brand-surface": "#262c38",
    "--ui-brand-accent": "#d42020",
  }),
});

export const ROLE_B_TOKENS = Object.freeze({
  light: Object.freeze({
    "--ui-ops-bg": "#f6f7f9",
    "--ui-ops-surface": "#ffffff",
    "--ui-ops-accent": "#3b66c4",
  }),
  dark: Object.freeze({
    "--ui-ops-bg": "#161b24",
    "--ui-ops-surface": "#1e2532",
    "--ui-ops-accent": "#7da4f2",
  }),
});

export const SEMANTIC_TOKENS = Object.freeze({
  light: Object.freeze({
    "--ui-success": "#176b50",
    "--ui-warning": "#8f4f12",
    "--ui-danger": "#a83333",
    "--ui-info": "#1d5fa3",
  }),
  dark: Object.freeze({
    "--ui-success": "#34d399",
    "--ui-warning": "#fbbf24",
    "--ui-danger": "#f87171",
    "--ui-info": "#60a5fa",
  }),
});

export const REQUIRED_SCALE_TOKENS = Object.freeze([
  "--ui-font-sans",
  "--ui-font-serif",
  "--ui-font-mono",
  "--ui-space-1",
  "--ui-space-2",
  "--ui-space-3",
  "--ui-space-4",
  "--ui-space-5",
  "--ui-space-6",
  "--ui-space-8",
  "--ui-space-10",
  "--ui-radius-xs",
  "--ui-radius-sm",
  "--ui-radius-md",
  "--ui-radius-lg",
  "--ui-radius-xl",
  "--ui-radius-pill",
  "--ui-z-sticky",
  "--ui-z-header",
  "--ui-z-drawer",
  "--ui-z-modal",
  "--ui-z-confirm",
  "--ui-z-toast",
  "--ui-success",
  "--ui-warning",
  "--ui-danger",
  "--ui-info",
]);

function unwrapPage(pageOrRes) {
  return pageOrRes && pageOrRes.page ? pageOrRes.page : pageOrRes;
}

function normalizeColorValue(raw) {
  const s = String(raw || "").trim().toLowerCase();
  if (!s) return "";
  if (s.startsWith("#")) {
    if (s.length === 4) {
      return `#${s[1]}${s[1]}${s[2]}${s[2]}${s[3]}${s[3]}`;
    }
    return s;
  }
  const rgbMatch = s.match(
    /^rgba?\(\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)(?:\s*[,/]\s*(\d+(?:\.\d+)?))?\s*\)$/
  );
  if (rgbMatch) {
    const r = Math.round(Number.parseFloat(rgbMatch[1]));
    const g = Math.round(Number.parseFloat(rgbMatch[2]));
    const b = Math.round(Number.parseFloat(rgbMatch[3]));
    const toHex = (n) => Math.max(0, Math.min(255, n)).toString(16).padStart(2, "0");
    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  }
  return s;
}

/**
 * Verify shared `--ui-*` CSS custom properties, Role A (`brand`) or Role B (`ops`)
 * theme palette values, and `IBM Plex Sans` typography (never leaking `"Inter"`).
 */
export async function assertUiTokensPresent(pageOrRes, { role = "brand", theme = "light" } = {}) {
  const page = unwrapPage(pageOrRes);
  const normRole = role === "ops" ? "ops" : "brand";
  const normTheme = theme === "dark" ? "dark" : "light";

  const roleMap = normRole === "ops" ? ROLE_B_TOKENS[normTheme] : ROLE_A_TOKENS[normTheme];
  const semanticMap = SEMANTIC_TOKENS[normTheme];
  const allTokenNames = [
    ...REQUIRED_SCALE_TOKENS,
    ...Object.keys(roleMap),
  ];

  const data = await page.evaluate((tokenNames) => {
    const rootStyle = window.getComputedStyle(document.documentElement);
    const tokens = {};
    for (const name of tokenNames) {
      tokens[name] = rootStyle.getPropertyValue(name).trim();
    }

    const bodyStyle = document.body ? window.getComputedStyle(document.body) : null;
    const bodyFontFamily = bodyStyle ? bodyStyle.fontFamily : "";

    const sampleFonts = [bodyFontFamily];
    for (const sel of ["h1", "button", ".hf-studio", "#hf-panel-group", ".ops-card", ".ap-shell"]) {
      const el = document.querySelector(sel);
      if (el) {
        sampleFonts.push(window.getComputedStyle(el).fontFamily);
      }
    }

    const hasDesignTokensLink = Array.from(document.querySelectorAll('link[rel="stylesheet"]')).some((l) =>
      (l.getAttribute("href") || "").includes("/css/design-tokens.css")
    );

    return {
      appliedTheme: document.documentElement.getAttribute("data-theme") || "",
      tokens,
      bodyFontFamily,
      sampleFonts,
      hasDesignTokensLink,
    };
  }, allTokenNames);

  const missing = [];
  for (const name of REQUIRED_SCALE_TOKENS) {
    if (!data.tokens[name]) {
      missing.push(name);
    }
  }
  if (missing.length > 0) {
    throw new AssertionFailure(
      `Missing required --ui-* design tokens on ${page.url()} (${normRole}/${normTheme}): ${missing.join(", ")}`,
      { url: page.url(), role: normRole, theme: normTheme, missing, tokens: data.tokens }
    );
  }

  const mismatches = [];
  for (const [token, expectedHex] of Object.entries(roleMap)) {
    const actualNorm = normalizeColorValue(data.tokens[token]);
    const expectedNorm = normalizeColorValue(expectedHex);
    if (actualNorm !== expectedNorm) {
      mismatches.push(`${token}: expected ${expectedNorm}, got ${data.tokens[token] || "(empty)"}`);
    }
  }
  for (const [token, expectedHex] of Object.entries(semanticMap)) {
    const actualNorm = normalizeColorValue(data.tokens[token]);
    const expectedNorm = normalizeColorValue(expectedHex);
    if (actualNorm !== expectedNorm) {
      mismatches.push(`${token}: expected ${expectedNorm}, got ${data.tokens[token] || "(empty)"}`);
    }
  }
  if (mismatches.length > 0) {
    throw new AssertionFailure(
      `Design token value mismatch on ${page.url()} (${normRole}/${normTheme}): ${mismatches.join("; ")}`,
      { url: page.url(), role: normRole, theme: normTheme, mismatches }
    );
  }

  const fontSansToken = (data.tokens["--ui-font-sans"] || "").toLowerCase();
  if (!fontSansToken.includes("ibm plex sans")) {
    throw new AssertionFailure(
      `--ui-font-sans must resolve IBM Plex Sans, got "${data.tokens["--ui-font-sans"]}"`,
      { url: page.url(), fontSansToken }
    );
  }

  const bodyFontLower = (data.bodyFontFamily || "").toLowerCase();
  if (!bodyFontLower.includes("ibm plex sans")) {
    throw new AssertionFailure(
      `document.body fontFamily must resolve IBM Plex Sans on ${page.url()}, got "${data.bodyFontFamily}"`,
      { url: page.url(), bodyFontFamily: data.bodyFontFamily }
    );
  }

  for (const ff of data.sampleFonts) {
    const tokens = String(ff || "")
      .split(",")
      .map((s) => s.replace(/["']/g, "").trim().toLowerCase())
      .filter(Boolean);
    if (tokens.includes("inter")) {
      throw new AssertionFailure(
        `Forbidden "Inter" font-family leak detected on ${page.url()}: "${ff}"`,
        { url: page.url(), fontFamily: ff }
      );
    }
  }

  return data;
}

/**
 * Confirm modal/dialog visibility, full viewport containment (`1440x900` & `390x844`),
 * and spacious interior padding.
 */
export async function assertModalGeometry(pageOrRes, selector, { minPadding = 16 } = {}) {
  const page = unwrapPage(pageOrRes);
  const loc = page.locator(selector).first();
  await loc.waitFor({ state: "visible", timeout: 10000 });
  await page
    .waitForFunction(
      (sel) => {
        const el = document.querySelector(sel);
        if (!el) return false;
        const st = window.getComputedStyle(el);
        return st.display !== "none" && st.visibility !== "hidden" && Number.parseFloat(st.opacity || "1") >= 0.5;
      },
      selector,
      { timeout: 2000 }
    )
    .catch(() => {});

  const metrics = await page.evaluate(({ sel }) => {
    const el = document.querySelector(sel);
    if (!el) return { found: false };
    const st = window.getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    const isVisible =
      st.display !== "none" &&
      st.visibility !== "hidden" &&
      Number.parseFloat(st.opacity || "1") > 0 &&
      rect.width > 0 &&
      rect.height > 0;

    // Identify the modal dialog surface card when selector is a backdrop overlay
    const isOverlay =
      el.classList.contains("modal-overlay") ||
      el.classList.contains("studio-modal-overlay") ||
      el.classList.contains("hf-centered-modal-wrap") ||
      (rect.width >= window.innerWidth - 4 && rect.height >= window.innerHeight - 4 && el.tagName !== "DIALOG");

    let card = el;
    if (isOverlay) {
      card =
        el.querySelector(
          ".studio-modal-card, .modal-content, .modal-card, .modal-dialog, .hf-confirm-modal-card, .hf-modal-card, .ops-modal-card"
        ) ||
        el.firstElementChild ||
        el;
    }

    const cardRect = card.getBoundingClientRect();
    const cardStyle = window.getComputedStyle(card);
    const pt = Number.parseFloat(cardStyle.paddingTop) || 0;
    const pr = Number.parseFloat(cardStyle.paddingRight) || 0;
    const pb = Number.parseFloat(cardStyle.paddingBottom) || 0;
    const pl = Number.parseFloat(cardStyle.paddingLeft) || 0;
    let effectiveMinPadding = Math.min(pt, pr, pb, pl);

    // Some modal cards apply padding to header/body/footer slots instead of outer card
    if (effectiveMinPadding < 4) {
      const innerCandidates = Array.from(
        card.querySelectorAll(
          ".modal-body, .studio-modal-body, .dialog-body, .dialog-content, .hf-modal-body, form, section, div"
        )
      ).filter((child) => {
        const cr = child.getBoundingClientRect();
        const cs = window.getComputedStyle(child);
        return cr.width > 0 && cr.height > 0 && cs.display !== "none";
      });
      let bestPad = 0;
      for (const child of innerCandidates) {
        const cs = window.getComputedStyle(child);
        const cMin = Math.min(
          Number.parseFloat(cs.paddingTop) || 0,
          Number.parseFloat(cs.paddingRight) || 0,
          Number.parseFloat(cs.paddingBottom) || 0,
          Number.parseFloat(cs.paddingLeft) || 0
        );
        if (cMin > bestPad) bestPad = cMin;
      }
      effectiveMinPadding = bestPad;
    }

    return {
      found: true,
      isVisible,
      innerWidth: window.innerWidth,
      innerHeight: window.innerHeight,
      rect: {
        left: cardRect.left,
        right: cardRect.right,
        top: cardRect.top,
        bottom: cardRect.bottom,
        width: cardRect.width,
        height: cardRect.height,
      },
      effectiveMinPadding,
      directPadding: { pt, pr, pb, pl },
    };
  }, { sel: selector });

  if (!metrics.found || !metrics.isVisible) {
    throw new AssertionFailure(`Modal ${selector} is not visible on ${page.url()}`, metrics);
  }
  const r = metrics.rect;
  if (r.width <= 0 || r.height <= 0) {
    throw new AssertionFailure(`Modal ${selector} has zero dimensions: ${r.width}x${r.height}`, metrics);
  }
  if (
    r.left < -1 ||
    r.top < -1 ||
    r.right > metrics.innerWidth + 1 ||
    r.bottom > metrics.innerHeight + 1
  ) {
    throw new AssertionFailure(
      `Modal ${selector} clips outside viewport (${metrics.innerWidth}x${metrics.innerHeight}): ` +
        `left=${Math.round(r.left)}, top=${Math.round(r.top)}, right=${Math.round(r.right)}, bottom=${Math.round(r.bottom)}`,
      metrics
    );
  }
  if (metrics.effectiveMinPadding < minPadding) {
    throw new AssertionFailure(
      `Modal ${selector} padding (${metrics.effectiveMinPadding}px) is below spacious minimum ${minPadding}px`,
      metrics
    );
  }
  return metrics;
}

/**
 * Verify shared floating toast contract:
 * - fixed bottom-right on desktop (`bottom <= 48`, `right <= 48`, `position === "fixed"`)
 * - >= 16px margin on mobile (`innerWidth <= 640`)
 * - semantic `role` (`"alert"` / `"status"`)
 * - close button present
 */
export async function assertToastContract(
  pageOrRes,
  containerSelector,
  toastSelector,
  { expectedRole = null } = {}
) {
  const page = unwrapPage(pageOrRes);
  const toastLoc = page.locator(toastSelector).first();
  await toastLoc.waitFor({ state: "visible", timeout: 10000 });

  const metrics = await page.evaluate(
    ({ cSel, tSel }) => {
      function queryFirstByPriority(selectorStr) {
        const parts = String(selectorStr || "")
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        for (const p of parts) {
          const hit = document.querySelector(p);
          if (hit) return hit;
        }
        return document.querySelector(selectorStr);
      }
      const container = queryFirstByPriority(cSel);
      const toast = queryFirstByPriority(tSel);
      if (!container || !toast) {
        return { found: false, hasContainer: Boolean(container), hasToast: Boolean(toast) };
      }
      const cStyle = window.getComputedStyle(container);
      const tStyle = window.getComputedStyle(toast);
      // Either container or toast may carry fixed positioning if toast-slot is fixed
      const anchoredEl = cStyle.position === "fixed" ? container : tStyle.position === "fixed" ? toast : container;
      const anchoredPos = window.getComputedStyle(anchoredEl).position;
      const anchorRect = anchoredEl.getBoundingClientRect();
      const toastRect = toast.getBoundingClientRect();
      const closeBtn = toast.querySelector(
        'button, [aria-label*="Kapat" i], [aria-label*="Close" i], .toast-close, .ops-toast-close, .ap-toast-close, [data-toast-close]'
      );

      return {
        found: true,
        innerWidth: window.innerWidth,
        innerHeight: window.innerHeight,
        position: anchoredPos,
        bottomGap: window.innerHeight - toastRect.bottom,
        rightGap: window.innerWidth - toastRect.right,
        leftGap: toastRect.left,
        anchorBottomGap: window.innerHeight - anchorRect.bottom,
        anchorRightGap: window.innerWidth - anchorRect.right,
        role: toast.getAttribute("role") || "",
        hasCloseButton: Boolean(closeBtn),
        toastCount: document.querySelectorAll(tSel).length,
      };
    },
    { cSel: containerSelector, tSel: toastSelector }
  );

  if (!metrics.found) {
    throw new AssertionFailure(
      `Toast elements not found (container=${containerSelector}, toast=${toastSelector})`,
      metrics
    );
  }
  if (metrics.position !== "fixed") {
    throw new AssertionFailure(
      `Toast container ${containerSelector} must use position: fixed (got "${metrics.position}")`,
      metrics
    );
  }

  if (metrics.innerWidth <= 640) {
    if (metrics.leftGap < 15 || metrics.rightGap < 15 || metrics.bottomGap < 15) {
      throw new AssertionFailure(
        `Mobile toast (${metrics.innerWidth}px) must keep >=16px viewport margins, got left=${Math.round(metrics.leftGap)}, right=${Math.round(metrics.rightGap)}, bottom=${Math.round(metrics.bottomGap)}`,
        metrics
      );
    }
  } else {
    if (metrics.bottomGap < 0 || metrics.bottomGap > 48 || metrics.rightGap < 0 || metrics.rightGap > 48) {
      throw new AssertionFailure(
        `Desktop toast (${metrics.innerWidth}px) must anchor bottom-right (bottom<=48, right<=48), got bottom=${Math.round(metrics.bottomGap)}px, right=${Math.round(metrics.rightGap)}px`,
        metrics
      );
    }
  }

  if (expectedRole) {
    if (metrics.role !== expectedRole) {
      throw new AssertionFailure(
        `Toast ${toastSelector} expected role="${expectedRole}", got "${metrics.role}"`,
        metrics
      );
    }
  } else if (metrics.role !== "alert" && metrics.role !== "status") {
    throw new AssertionFailure(
      `Toast ${toastSelector} must set role="alert" or role="status", got "${metrics.role}"`,
      metrics
    );
  }

  if (!metrics.hasCloseButton) {
    throw new AssertionFailure(
      `Toast ${toastSelector} must include a manual close button`,
      metrics
    );
  }

  return metrics;
}

/**
 * Verify button minimum heights (`>=36px` ops, `>=40px` public/member primary),
 * readable label font size (`>=11px`), and zero clipped text (`scrollWidth <= clientWidth + 4`).
 */
export async function assertButtonHierarchyAndTypeFloor(pageOrRes, selectors, opts = {}) {
  const page = unwrapPage(pageOrRes);
  const selList = Array.isArray(selectors) ? selectors : [selectors];
  const minFontSize = opts.minFontSize ?? 11;

  const url = page.url();
  const inferredOps = opts.role === "ops" || (!opts.role && /\/(podmin|tasarim|editor|e2e-admin)/i.test(url));
  const defaultMinHeight = opts.minHeight ?? (inferredOps ? 36 : 40);

  const report = await page.evaluate(
    ({ items, fallbackMinH, minFs }) => {
      const checked = [];
      const violations = [];

      for (const item of items) {
        const sel = typeof item === "string" ? item : item.selector;
        const reqMinH = typeof item === "object" && item.minHeight ? item.minHeight : fallbackMinH;
        const reqMinFs = typeof item === "object" && item.minFontSize ? item.minFontSize : minFs;

        const els = Array.from(document.querySelectorAll(sel));
        const visibleEls = els.filter((el) => {
          const st = window.getComputedStyle(el);
          const r = el.getBoundingClientRect();
          return st.display !== "none" && st.visibility !== "hidden" && r.width > 0 && r.height > 0;
        });

        if (visibleEls.length === 0) {
          violations.push({ selector: sel, issue: "no visible matching button" });
          continue;
        }

        for (const el of visibleEls) {
          const st = window.getComputedStyle(el);
          const r = el.getBoundingClientRect();
          const fontSize = Number.parseFloat(st.fontSize) || 0;
          const height = r.height;
          const clipped = el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 4;
          const text = (el.innerText || el.textContent || "").trim().slice(0, 40);

          checked.push({ selector: sel, text, height: Math.round(height), fontSize, clipped });

          if (height + 0.5 < reqMinH) {
            violations.push({
              selector: sel,
              text,
              issue: `height ${height.toFixed(1)}px < ${reqMinH}px`,
            });
          }
          if (fontSize + 0.1 < reqMinFs) {
            violations.push({
              selector: sel,
              text,
              issue: `fontSize ${fontSize.toFixed(1)}px < ${reqMinFs}px`,
            });
          }
          if (clipped) {
            violations.push({
              selector: sel,
              text,
              issue: `clipped text scrollWidth=${el.scrollWidth} > clientWidth=${el.clientWidth}`,
            });
          }
        }
      }
      return { checked, violations };
    },
    { items: selList, fallbackMinH: defaultMinHeight, minFs: minFontSize }
  );

  if (report.violations.length > 0) {
    throw new AssertionFailure(
      `Button hierarchy/type floor violations on ${url}: ${report.violations.map((v) => `${v.selector} (${v.issue})`).join("; ")}`,
      report
    );
  }
  return report;
}

const FORBIDDEN_MARKERS = [
  { name: "goofy-kalam", rx: /goofy-kalam/i },
  { name: "haberflow", rx: /haberflow/i },
  { name: "fonts.googleapis", rx: /fonts\.googleapis\.com/i },
  { name: "fonts.gstatic", rx: /fonts\.gstatic\.com/i },
  { name: "cdnjs.cloudflare", rx: /cdnjs\.cloudflare\.com/i },
  { name: "unpkg.com", rx: /unpkg\.com/i },
  { name: "cdn.jsdelivr", rx: /cdn\.jsdelivr\.net/i },
  { name: "mockdata", rx: /\bmock_?data\b/i },
  { name: "dummy_data", rx: /\bdummy_?data\b/i },
  { name: "lorem ipsum", rx: /lorem\s+ipsum/i },
  { name: "via.placeholder", rx: /via\.placeholder\.com/i },
  { name: "picsum.photos", rx: /picsum\.photos/i },
];

/**
 * Scan HTML/CSS/JS payloads for forbidden external CDNs and mockdata markers.
 */
export function assertNoExternalCdnsOrMockdata(text, label = "payload") {
  const src = String(text || "");
  const hits = [];
  for (const marker of FORBIDDEN_MARKERS) {
    if (marker.rx.test(src)) {
      hits.push(marker.name);
    }
  }
  if (hits.length > 0) {
    throw new AssertionFailure(
      `Forbidden external CDN or mockdata marker(s) in ${label}: ${hits.join(", ")}`,
      { label, hits }
    );
  }
}

/**
 * Capture a PNG screenshot safely into `ctx.screenshotDir` (or `/tmp/msi_e2e_screenshots`),
 * never inside `.agents/teamwork/` or the git repository.
 */
export async function captureSurfaceScreenshot(pageRes, filename, opts = {}) {
  const page = unwrapPage(pageRes);
  const rawName = String(filename || "surface.png");
  const baseDir =
    opts.screenshotDir ||
    opts.ctx?.screenshotDir ||
    process.env.E2E_SCREENSHOT_DIR ||
    "/tmp/msi_e2e_screenshots";

  const targetPath = path.isAbsolute(rawName) ? path.resolve(rawName) : path.resolve(baseDir, rawName);
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  return await captureScreenshot(page, targetPath, opts);
}
