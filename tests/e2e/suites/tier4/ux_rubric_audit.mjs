#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { STAFF_ROLES, READER_ROLES, createAuthHelpers } from "../../lib/auth.mjs";
import {
  VIEWPORT_DESKTOP,
  VIEWPORT_MOBILE,
  captureScreenshot,
  createBrowserManager,
  emulateReducedMotion,
  setTheme,
} from "../../lib/browser.mjs";
import { Client, createHttpFactory } from "../../lib/http.mjs";
import { DEFAULT_REPO_ROOT, startIsolatedServer } from "../../lib/server.mjs";

const __filename = fileURLToPath(import.meta.url);

function parseCliArgs(argv) {
  const opts = {
    baseUrl: null,
    outDir: null,
    theme: "both",
    viewports: ["desktop", "mobile"],
    adminPath: "podmin",
    adminEmail: process.env.E2E_ADMIN_EMAIL || "",
    adminPassword: process.env.E2E_ADMIN_PASSWORD || "",
    roleCredsPath: null,
    provisionRoles: false,
    keepUsers: false,
    strict: false,
    quick: false,
    surfaces: null,
    help: false,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      opts.help = true;
    } else if (arg === "--strict") {
      opts.strict = true;
    } else if (arg === "--quick") {
      opts.quick = true;
    } else if (arg === "--provision-roles") {
      opts.provisionRoles = true;
    } else if (arg === "--keep-users") {
      opts.keepUsers = true;
    } else if (arg.startsWith("--base-url=")) {
      opts.baseUrl = arg.slice("--base-url=".length).trim();
    } else if (arg === "--base-url" && argv[i + 1]) {
      opts.baseUrl = argv[++i].trim();
    } else if (arg.startsWith("--out-dir=")) {
      opts.outDir = arg.slice("--out-dir=".length).trim();
    } else if (arg === "--out-dir" && argv[i + 1]) {
      opts.outDir = argv[++i].trim();
    } else if (arg.startsWith("--theme=")) {
      opts.theme = arg.slice("--theme=".length).trim().toLowerCase();
    } else if (arg === "--theme" && argv[i + 1]) {
      opts.theme = argv[++i].trim().toLowerCase();
    } else if (arg.startsWith("--viewports=")) {
      opts.viewports = arg.slice("--viewports=".length).split(",").map((s) => s.trim()).filter(Boolean);
    } else if (arg === "--viewports" && argv[i + 1]) {
      opts.viewports = argv[++i].split(",").map((s) => s.trim()).filter(Boolean);
    } else if (arg.startsWith("--admin-path=")) {
      opts.adminPath = arg.slice("--admin-path=".length).trim() || "podmin";
    } else if (arg === "--admin-path" && argv[i + 1]) {
      opts.adminPath = argv[++i].trim() || "podmin";
    } else if (arg.startsWith("--admin-email=")) {
      opts.adminEmail = arg.slice("--admin-email=".length).trim();
    } else if (arg === "--admin-email" && argv[i + 1]) {
      opts.adminEmail = argv[++i].trim();
    } else if (arg.startsWith("--admin-password=")) {
      opts.adminPassword = arg.slice("--admin-password=".length);
    } else if (arg === "--admin-password" && argv[i + 1]) {
      opts.adminPassword = argv[++i];
    } else if (arg.startsWith("--role-creds=")) {
      opts.roleCredsPath = arg.slice("--role-creds=".length).trim();
    } else if (arg === "--role-creds" && argv[i + 1]) {
      opts.roleCredsPath = argv[++i].trim();
    } else if (arg.startsWith("--surfaces=")) {
      opts.surfaces = arg.slice("--surfaces=".length).split(",").map((s) => s.trim()).filter(Boolean);
    } else if (arg === "--surfaces" && argv[i + 1]) {
      opts.surfaces = argv[++i].split(",").map((s) => s.trim()).filter(Boolean);
    } else {
      throw new Error(`Unknown CLI argument: ${arg}`);
    }
  }
  return opts;
}

function resolveThemes(themeOpt) {
  if (themeOpt === "light") return ["light"];
  if (themeOpt === "dark") return ["dark"];
  return ["light", "dark"];
}

function resolveViewports(vpList) {
  const out = [];
  for (const item of vpList || ["desktop", "mobile"]) {
    const low = String(item).toLowerCase();
    if (low === "desktop") {
      out.push({ name: "desktop", ...VIEWPORT_DESKTOP });
    } else if (low === "mobile") {
      out.push({ name: "mobile", ...VIEWPORT_MOBILE });
    } else {
      const m = low.match(/^(\d+)x(\d+)$/);
      if (m) {
        out.push({
          name: `${m[1]}x${m[2]}`,
          width: Number.parseInt(m[1], 10),
          height: Number.parseInt(m[2], 10),
        });
      }
    }
  }
  return out.length > 0 ? out : [{ name: "desktop", ...VIEWPORT_DESKTOP }, { name: "mobile", ...VIEWPORT_MOBILE }];
}

/**
 * Evaluate DOM-level UX rubric metrics on an open Playwright page.
 */
export async function evaluatePageUxRubric(pageRes, meta = {}) {
  const { surface = "page", theme = "light", isIsolated = true, expectGsap = false } = meta;
  const page = pageRes.page;

  // Give CSS/GSAP settling tick
  await page.waitForTimeout(260);

  const overflow = await pageRes.measureOverflow();

  const domMetrics = await page.evaluate(async ({ surfaceName, activeTheme }) => {
    function parseRgba(str) {
      if (!str || str === "transparent") return { r: 0, g: 0, b: 0, a: 0 };
      const m = str.match(/rgba?\(\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)(?:\s*[,/]\s*(\d+(?:\.\d+)?))?\s*\)/i);
      if (!m) return null;
      return {
        r: Number.parseFloat(m[1]),
        g: Number.parseFloat(m[2]),
        b: Number.parseFloat(m[3]),
        a: m[4] !== undefined ? Number.parseFloat(m[4]) : 1,
      };
    }

    function compositeOver(fg, bg) {
      const a = fg.a;
      return {
        r: Math.round(fg.r * a + bg.r * (1 - a)),
        g: Math.round(fg.g * a + bg.g * (1 - a)),
        b: Math.round(fg.b * a + bg.b * (1 - a)),
        a: 1,
      };
    }

    function relLuminance(c) {
      const chan = [c.r, c.g, c.b].map((v) => {
        const s = v / 255;
        return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
      });
      return 0.2126 * chan[0] + 0.7152 * chan[1] + 0.0722 * chan[2];
    }

    function contrastRatio(c1, c2) {
      const l1 = relLuminance(c1);
      const l2 = relLuminance(c2);
      const hi = Math.max(l1, l2);
      const lo = Math.min(l1, l2);
      return (hi + 0.05) / (lo + 0.05);
    }

    function parseBgGradientAverage(bgImg) {
      if (!bgImg || bgImg === "none" || !bgImg.includes("gradient(")) return null;
      const rgbStops = Array.from(
        bgImg.matchAll(/rgba?\(\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)\s*,\s*(\d+(?:\.\d+)?)(?:\s*[,/]\s*(\d+(?:\.\d+)?))?\s*\)/gi)
      ).map((m) => ({
        r: Number.parseFloat(m[1]),
        g: Number.parseFloat(m[2]),
        b: Number.parseFloat(m[3]),
        a: m[4] !== undefined ? Number.parseFloat(m[4]) : 1,
      }));
      const srgbStops = Array.from(
        bgImg.matchAll(/color\(\s*srgb\s+(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)\s+(\d+(?:\.\d+)?)(?:\s*\/\s*(\d+(?:\.\d+)?%?))?\s*\)/gi)
      ).map((m) => {
        let a = 1;
        if (m[4] !== undefined) {
          a = m[4].endsWith("%") ? Number.parseFloat(m[4]) / 100 : Number.parseFloat(m[4]);
        }
        return {
          r: Math.round(Number.parseFloat(m[1]) * 255),
          g: Math.round(Number.parseFloat(m[2]) * 255),
          b: Math.round(Number.parseFloat(m[3]) * 255),
          a,
        };
      });
      const stops = [...rgbStops, ...srgbStops];
      if (stops.length === 0) return null;
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;
      for (const s of stops) {
        r += s.r;
        g += s.g;
        b += s.b;
        a += s.a;
      }
      return {
        r: Math.round(r / stops.length),
        g: Math.round(g / stops.length),
        b: Math.round(b / stops.length),
        a: a / stops.length,
      };
    }

    function resolveBgColor(el) {
      const fallback = activeTheme === "dark" ? { r: 22, g: 27, b: 36, a: 1 } : { r: 246, g: 247, b: 249, a: 1 };
      const chain = [];
      let cur = el;
      while (cur && cur.nodeType === 1) {
        const st = window.getComputedStyle(cur);
        const grad = parseBgGradientAverage(st.backgroundImage);
        if (grad && grad.a > 0.05) {
          chain.unshift(grad);
          if (grad.a >= 0.99) break;
        }
        const bg = parseRgba(st.backgroundColor);
        if (bg && bg.a > 0.01) {
          chain.unshift(bg);
          if (bg.a >= 0.99) break;
        }
        cur = cur.parentElement;
      }
      let acc = fallback;
      for (const layer of chain) {
        acc = layer.a >= 0.99 ? { ...layer, a: 1 } : compositeOver(layer, acc);
      }
      return acc;
    }

    function isVisuallyClippedOut(st, rect) {
      const clipVal = (st.clip || "").replace(/\s+/g, "").toLowerCase();
      if (clipVal === "rect(0px,0px,0px,0px)" || clipVal === "rect(0,0,0,0)") {
        return true;
      }
      const clipPathVal = (st.clipPath || "").replace(/\s+/g, " ").trim().toLowerCase();
      if (
        /^inset\(\s*50%(?:\s+50%){0,3}\s*\)$/.test(clipPathVal) ||
        /^(?:circle|ellipse)\(\s*0(?:px|%)?\b/.test(clipPathVal)
      ) {
        return true;
      }
      if (
        rect.width <= 1.05 &&
        rect.height <= 1.05 &&
        (st.overflow === "hidden" ||
          st.overflow === "clip" ||
          ((st.overflowX === "hidden" || st.overflowX === "clip") &&
            (st.overflowY === "hidden" || st.overflowY === "clip")))
      ) {
        return true;
      }
      return false;
    }

    function isMediaHiddenByAncestors(el) {
      if (!el || el.nodeType !== 1) return true;
      let prev = el;
      let cur = el;
      while (cur && cur.nodeType === 1) {
        if (cur.hasAttribute("hidden")) return true;
        if (cur !== el && cur.tagName === "DETAILS" && !cur.hasAttribute("open") && prev.tagName !== "SUMMARY") {
          return true;
        }
        const st = window.getComputedStyle(cur);
        if (st.display === "none" || st.visibility === "hidden" || Number.parseFloat(st.opacity) <= 0.02) {
          return true;
        }
        if (cur !== el && cur.classList.contains("admin-drawer") && !cur.classList.contains("open")) {
          return true;
        }
        if (cur !== el && cur.id === "admin-sidebar" && window.innerWidth < 1024 && !cur.classList.contains("open")) {
          return true;
        }
        if (cur !== el && cur.id === "ap-sidebar" && window.innerWidth < 960 && !cur.classList.contains("open")) {
          return true;
        }
        prev = cur;
        cur = cur.parentElement;
      }
      return false;
    }

    function isVisuallyShown(el) {
      if (!el || el.nodeType !== 1) return false;
      const r = el.getBoundingClientRect();
      if (r.width <= 0.5 || r.height <= 0.5) return false;
      let prev = el;
      let cur = el.parentElement;
      // First check self
      if (el.hasAttribute("hidden") || el.getAttribute("aria-hidden") === "true") return false;
      const selfSt = window.getComputedStyle(el);
      if (selfSt.display === "none" || selfSt.visibility === "hidden" || Number.parseFloat(selfSt.opacity) <= 0.02) {
        return false;
      }
      if (isVisuallyClippedOut(selfSt, r)) {
        return false;
      }
      while (cur && cur.nodeType === 1) {
        if (cur.hasAttribute("hidden") || cur.getAttribute("aria-hidden") === "true") return false;
        if (cur.tagName === "DETAILS" && !cur.hasAttribute("open") && prev.tagName !== "SUMMARY") {
          return false;
        }
        const st = window.getComputedStyle(cur);
        if (st.display === "none" || st.visibility === "hidden" || Number.parseFloat(st.opacity) <= 0.02) {
          return false;
        }
        if (isVisuallyClippedOut(st, cur.getBoundingClientRect())) {
          return false;
        }
        // Closed offscreen drawer or collapsed sidebar
        if (cur.classList.contains("admin-drawer") && !cur.classList.contains("open")) {
          return false;
        }
        if (cur.id === "admin-sidebar" && window.innerWidth < 1024 && !cur.classList.contains("open")) {
          return false;
        }
        if (cur.id === "ap-sidebar" && window.innerWidth < 960 && !cur.classList.contains("open")) {
          return false;
        }
        // Clipped completely outside an overflow:hidden / overflow:clip ancestor
        if (st.overflowX === "hidden" || st.overflowX === "clip" || st.overflowY === "hidden" || st.overflowY === "clip") {
          const cr = cur.getBoundingClientRect();
          if (cr.width <= 0.5 || cr.height <= 0.5) return false;
          if (
            (st.overflowX === "hidden" || st.overflowX === "clip") &&
            (r.right <= cr.left + 1 || r.left >= cr.right - 1)
          ) {
            return false;
          }
          if (
            (st.overflowY === "hidden" || st.overflowY === "clip") &&
            (r.bottom <= cr.top + 1 || r.top >= cr.bottom - 1)
          ) {
            return false;
          }
        }
        prev = cur;
        cur = cur.parentElement;
      }
      return true;
    }

    function getLayerRoot(el) {
      let cur = el;
      while (cur && cur.nodeType === 1 && cur !== document.body && cur !== document.documentElement) {
        if (
          cur.tagName === "DIALOG" ||
          cur.classList.contains("admin-drawer") ||
          cur.classList.contains("admin-modal") ||
          cur.classList.contains("studio-modal-overlay") ||
          cur.classList.contains("ad-floating-popup") ||
          cur.classList.contains("cookie-consent-bar") ||
          cur.classList.contains("cookie-banner")
        ) {
          return cur;
        }
        const pos = window.getComputedStyle(cur).position;
        if (pos === "fixed" || pos === "sticky") {
          return cur;
        }
        cur = cur.parentElement;
      }
      return document.body;
    }

    function shortSelector(el) {
      if (el.id) return `#${el.id}`;
      const cls = typeof el.className === "string"
        ? el.className.trim().split(/\s+/).filter(Boolean).slice(0, 2).join(".")
        : "";
      return cls ? `${el.tagName.toLowerCase()}.${cls}` : el.tagName.toLowerCase();
    }

    // 1. Layout simplicity & hierarchy
    const domNodeCount = document.querySelectorAll("*").length;
    const interactiveSelectors = "a[href], button, input:not([type=hidden]), select, textarea, [role=tab], [role=button]";
    const allInteractive = Array.from(document.querySelectorAll(interactiveSelectors)).filter((el) => {
      if (!isVisuallyShown(el)) return false;
      const r = el.getBoundingClientRect();
      return r.right > 0 && r.left < window.innerWidth && r.bottom > 0 && r.top < window.innerHeight * 2;
    });

    const cards = Array.from(document.querySelectorAll(".login-card, .admin-card, .ops-card, .ap-card, .transactional-card, .account-welcome, .editor-sidebar"));
    let minCardPaddingPx = null;
    for (const c of cards) {
      if (!isVisuallyShown(c)) continue;
      const st = window.getComputedStyle(c);
      const pad = Math.min(
        Number.parseFloat(st.paddingTop) || 0,
        Number.parseFloat(st.paddingLeft) || 0
      );
      if (minCardPaddingPx === null || pad < minCardPaddingPx) minCardPaddingPx = pad;
    }

    const bodyStyle = window.getComputedStyle(document.body);
    const bodyFontSize = Number.parseFloat(bodyStyle.fontSize) || 14;
    const h1 = Array.from(document.querySelectorAll("h1")).find((el) => isVisuallyShown(el));
    const h1FontSize = h1 ? Number.parseFloat(window.getComputedStyle(h1).fontSize) : bodyFontSize;
    const headingToBodyRatio = Number((h1FontSize / Math.max(1, bodyFontSize)).toFixed(2));

    const inlineHandlerViolations = [];
    for (const el of document.querySelectorAll("[onclick], [onchange], [onsubmit]")) {
      inlineHandlerViolations.push(shortSelector(el));
    }

    // 2. Computed colors & WCAG contrast
    const bodyBg = bodyStyle.backgroundColor || "";
    const textColor = bodyStyle.color || "";
    const firstCard = cards.find((c) => isVisuallyShown(c));
    const cardBg = firstCard ? window.getComputedStyle(firstCard).backgroundColor : "";

    const effectiveBodyBgRgba = resolveBgColor(document.body);
    const effectiveBodyBg = `rgb(${effectiveBodyBgRgba.r}, ${effectiveBodyBgRgba.g}, ${effectiveBodyBgRgba.b})`;
    const isPureBlack =
      effectiveBodyBgRgba &&
      effectiveBodyBgRgba.r === 0 &&
      effectiveBodyBgRgba.g === 0 &&
      effectiveBodyBgRgba.b === 0 &&
      effectiveBodyBgRgba.a > 0.9;
    const isPureWhite =
      effectiveBodyBgRgba &&
      effectiveBodyBgRgba.r === 255 &&
      effectiveBodyBgRgba.g === 255 &&
      effectiveBodyBgRgba.b === 255 &&
      effectiveBodyBgRgba.a > 0.9;
    const harshBodyBg = Boolean(isPureBlack || isPureWhite);

    let minContrastRatio = 21;
    const contrastViolations = [];
    const textCandidates = Array.from(document.querySelectorAll("h1, h2, h3, h4, p, a, button, label, span, td, th, li, strong, small"));
    for (const el of textCandidates) {
      if (!isVisuallyShown(el)) continue;
      // Only check elements with direct non-whitespace text node
      const hasDirectText = Array.from(el.childNodes).some(
        (n) => n.nodeType === 3 && n.textContent && n.textContent.trim().length > 0
      );
      if (!hasDirectText) continue;
      const r = el.getBoundingClientRect();
      if (r.bottom < 0 || r.top > window.innerHeight * 3) continue;
      const st = window.getComputedStyle(el);
      const fg = parseRgba(st.color);
      if (!fg || fg.a < 0.1) continue;
      const bg = resolveBgColor(el);
      const compositedFg = fg.a >= 0.99 ? fg : compositeOver(fg, bg);
      const ratio = contrastRatio(compositedFg, bg);
      const fs = Number.parseFloat(st.fontSize) || 14;
      const fw = Number.parseInt(st.fontWeight, 10) || 400;
      const isLarge = fs >= 24 || (fw >= 700 && fs >= 18.66);
      const required = isLarge ? 3.0 : 4.5;
      if (ratio < minContrastRatio) minContrastRatio = ratio;
      if (ratio + 0.05 < required) {
        contrastViolations.push({
          selector: shortSelector(el),
          text: (el.innerText || el.textContent || "").trim().slice(0, 48),
          fg: st.color,
          bg: `rgb(${bg.r}, ${bg.g}, ${bg.b})`,
          ratio: Number(ratio.toFixed(2)),
          required,
        });
      }
    }

    // 3. Animation / transition presence
    const gsapLoaded = typeof window.gsap === "object" && window.gsap !== null;
    let cssTransitionsCount = 0;
    for (const el of allInteractive.slice(0, 60)) {
      const st = window.getComputedStyle(el);
      const td = Number.parseFloat(st.transitionDuration) || 0;
      const ad = Number.parseFloat(st.animationDuration) || 0;
      if (td > 0 || ad > 0) cssTransitionsCount += 1;
    }

    function getClippedRect(el) {
      const base = el.getBoundingClientRect();
      let left = base.left;
      let top = base.top;
      let right = base.right;
      let bottom = base.bottom;
      let cur = el.parentElement;
      while (cur && cur.nodeType === 1 && cur !== document.documentElement) {
        const st = window.getComputedStyle(cur);
        const clipX = st.overflowX === "hidden" || st.overflowX === "clip" || st.overflowX === "auto" || st.overflowX === "scroll";
        const clipY = st.overflowY === "hidden" || st.overflowY === "clip" || st.overflowY === "auto" || st.overflowY === "scroll";
        if (clipX || clipY) {
          const cr = cur.getBoundingClientRect();
          if (clipX) {
            left = Math.max(left, cr.left);
            right = Math.min(right, cr.right);
          }
          if (clipY) {
            top = Math.max(top, cr.top);
            bottom = Math.min(bottom, cr.bottom);
          }
        }
        cur = cur.parentElement;
      }
      return {
        left,
        top,
        right,
        bottom,
        width: Math.max(0, right - left),
        height: Math.max(0, bottom - top),
      };
    }

    // 4. Bounding-box overlap, label clipping & media integrity
    const overlapViolations = [];
    const visibleBoxes = allInteractive
      .map((el) => ({ el, rect: getClippedRect(el), sel: shortSelector(el), layer: getLayerRoot(el) }))
      .filter(({ rect }) => rect.width > 2 && rect.height > 2);

    for (let i = 0; i < visibleBoxes.length; i++) {
      for (let j = i + 1; j < visibleBoxes.length; j++) {
        const a = visibleBoxes[i];
        const b = visibleBoxes[j];
        if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
        // Skip intentional trailing input adornment buttons inside password/search wrappers
        const isTrailingInputBtn =
          a.el.parentElement &&
          a.el.parentElement === b.el.parentElement &&
          ((a.el.tagName === "INPUT" && b.el.classList.contains("pw-toggle-btn") && b.rect.left >= a.rect.right - 64) ||
            (b.el.tagName === "INPUT" && a.el.classList.contains("pw-toggle-btn") && a.rect.left >= b.rect.right - 64));
        if (isTrailingInputBtn) continue;
        // Elements in separate fixed/sticky/dialog stacking layers are intentionally layered
        if (a.layer !== b.layer) continue;
        const ox = Math.min(a.rect.right, b.rect.right) - Math.max(a.rect.left, b.rect.left);
        const oy = Math.min(a.rect.bottom, b.rect.bottom) - Math.max(a.rect.top, b.rect.top);
        if (ox > 3 && oy > 3) {
          overlapViolations.push({
            a: a.sel,
            b: b.sel,
            overlapW: Math.round(ox),
            overlapH: Math.round(oy),
          });
        }
      }
    }

    // 4a/4b. Fixed-overlay collisions & fixed overlays obscuring primary CTAs/inputs (when no modal/drawer is open)
    const hasOpenModalOverlay = Boolean(
      document.querySelector(
        "dialog[open], .admin-modal.open, .admin-drawer.open, .studio-modal-overlay.open"
      )
    );
    if (!hasOpenModalOverlay) {
      const floatingFixedOverlays = [];
      for (const el of document.querySelectorAll("body *")) {
        if (!isVisuallyShown(el)) continue;
        const st = window.getComputedStyle(el);
        if (st.position !== "fixed" || st.pointerEvents === "none") continue;
        if (
          el.tagName === "HEADER" ||
          el.tagName === "NAV" ||
          el.tagName === "DIALOG" ||
          el.id === "admin-sidebar" ||
          el.id === "ap-sidebar" ||
          el.classList.contains("sidebar") ||
          el.classList.contains("ap-sidebar") ||
          el.classList.contains("admin-drawer") ||
          el.classList.contains("admin-modal") ||
          el.classList.contains("studio-modal-overlay") ||
          el.classList.contains("ap-sidebar-backdrop") ||
          el.classList.contains("sidebar-overlay") ||
          el.classList.contains("toast-region") ||
          el.classList.contains("ops-toast-region") ||
          el.classList.contains("scroll-telemetry") ||
          el.classList.contains("ambient-stage") ||
          el.classList.contains("login-ambient") ||
          el.classList.contains("ap-ambient-stage") ||
          el.classList.contains("grain-veil") ||
          el.classList.contains("ap-skip-link") ||
          el.classList.contains("skip-link")
        ) {
          continue;
        }
        let hasFixedAncestor = false;
        let anc = el.parentElement;
        while (anc && anc !== document.body) {
          if (window.getComputedStyle(anc).position === "fixed") {
            hasFixedAncestor = true;
            break;
          }
          anc = anc.parentElement;
        }
        if (hasFixedAncestor) continue;
        const r = el.getBoundingClientRect();
        if (r.width > 10 && r.height > 10 && r.height < window.innerHeight * 0.75) {
          floatingFixedOverlays.push({ el, rect: r, sel: shortSelector(el) });
        }
      }

      for (let i = 0; i < floatingFixedOverlays.length; i++) {
        for (let j = i + 1; j < floatingFixedOverlays.length; j++) {
          const a = floatingFixedOverlays[i];
          const b = floatingFixedOverlays[j];
          if (a.el.contains(b.el) || b.el.contains(a.el)) continue;
          const ox = Math.min(a.rect.right, b.rect.right) - Math.max(a.rect.left, b.rect.left);
          const oy = Math.min(a.rect.bottom, b.rect.bottom) - Math.max(a.rect.top, b.rect.top);
          if (ox > 3 && oy > 3) {
            overlapViolations.push({
              type: "fixed-overlay-collision",
              a: a.sel,
              b: b.sel,
              overlapW: Math.round(ox),
              overlapH: Math.round(oy),
            });
          }
        }
      }

      const ctaAndInputSelectors =
        "button, input:not([type=hidden]), select, textarea, [type=submit], a.button, a.btn, .account-welcome a[href], .transactional-card a[href]";
      for (const target of document.querySelectorAll(ctaAndInputSelectors)) {
        if (!isVisuallyShown(target)) continue;
        const tr = getClippedRect(target);
        if (tr.width <= 4 || tr.height <= 4) continue;
        if (tr.bottom <= 0 || tr.top >= window.innerHeight) continue;
        const targetLayer = getLayerRoot(target);
        for (const ov of floatingFixedOverlays) {
          if (ov.el === targetLayer || ov.el.contains(target) || target.contains(ov.el)) continue;
          const ox = Math.min(tr.right, ov.rect.right) - Math.max(tr.left, ov.rect.left);
          const oy = Math.min(tr.bottom, ov.rect.bottom) - Math.max(tr.top, ov.rect.top);
          if (ox > 6 && oy > 6) {
            overlapViolations.push({
              type: "fixed-overlay-obscures-control",
              a: ov.sel,
              b: shortSelector(target),
              overlapW: Math.round(ox),
              overlapH: Math.round(oy),
            });
          }
        }
      }
    }

    const clippedLabels = [];
    const brandSelectors =
      ".ap-brand, .ap-brand-text, .ap-brand-title, .ap-topbar-left, .top-header .header-title, .editor-brand";
    for (const el of document.querySelectorAll(
      `button, [role=tab], .sidebar-link, .ap-nav-btn, .button, ${brandSelectors}`
    )) {
      if (!isVisuallyShown(el)) continue;
      const isBrandEl = el.matches(brandSelectors);
      if (!isBrandEl) {
        if (el.classList.contains("text-ellipsis")) continue;
        const st = window.getComputedStyle(el);
        if (st.textOverflow === "ellipsis") continue;
      }
      if (el.clientWidth > 10 && el.scrollWidth > el.clientWidth + 4) {
        const range = document.createRange();
        range.selectNodeContents(el);
        const contentRect = range.getBoundingClientRect();
        const elRect = el.getBoundingClientRect();
        const contentOverflows =
          contentRect.width > el.clientWidth + 4 ||
          contentRect.right > elRect.right + 2 ||
          contentRect.left < elRect.left - 2;
        if (contentOverflows) {
          clippedLabels.push({
            selector: shortSelector(el),
            text: (el.innerText || "").trim().slice(0, 40),
            scrollWidth: Math.max(el.scrollWidth, Math.round(contentRect.width)),
            clientWidth: el.clientWidth,
          });
        }
      }
      if (isBrandEl && el.classList.contains("ap-brand-title")) {
        const titleRect = el.getBoundingClientRect();
        const brand = el.closest(".ap-brand");
        const backLink = document.querySelector(".ap-back-link");
        if (brand) {
          const br = brand.getBoundingClientRect();
          if (titleRect.right > br.right + 2) {
            clippedLabels.push({
              selector: shortSelector(el),
              text: (el.innerText || "").trim().slice(0, 40),
              scrollWidth: Math.round(titleRect.right - titleRect.left),
              clientWidth: Math.round(br.right - titleRect.left),
            });
          }
        }
        if (backLink && isVisuallyShown(backLink)) {
          const blr = backLink.getBoundingClientRect();
          if (titleRect.right > blr.left - 1 && titleRect.left < blr.right) {
            overlapViolations.push({
              type: "topbar-brand-collision",
              a: shortSelector(el),
              b: shortSelector(backLink),
              overlapW: Math.round(titleRect.right - blr.left),
              overlapH: Math.round(Math.min(titleRect.bottom, blr.bottom) - Math.max(titleRect.top, blr.top)),
            });
          }
        }
      }
    }

    const brokenImages = [];
    for (const img of document.querySelectorAll("img")) {
      if (isMediaHiddenByAncestors(img)) continue;
      if (img.loading === "lazy") img.loading = "eager";
      if (!img.complete) await img.decode().catch(() => {});
      if (!img.complete || img.naturalWidth <= 0) {
        brokenImages.push({ src: img.getAttribute("src") || "", naturalWidth: img.naturalWidth });
      }
    }

    const zeroSizeSvgs = [];
    for (const svg of document.querySelectorAll("svg")) {
      if (!svg.getClientRects().length) continue;
      const st = window.getComputedStyle(svg);
      if (st.display === "none" || st.visibility === "hidden") continue;
      // Ignore SVGs inside hidden ancestor
      let hiddenParent = false;
      let p = svg.parentElement;
      while (p) {
        const pst = window.getComputedStyle(p);
        if (p.hasAttribute("hidden") || pst.display === "none" || pst.visibility === "hidden") {
          hiddenParent = true;
          break;
        }
        p = p.parentElement;
      }
      if (hiddenParent) continue;
      const r = svg.getBoundingClientRect();
      if (r.width <= 0 || r.height <= 0) {
        zeroSizeSvgs.push(shortSelector(svg));
      }
    }

    return {
      domNodeCount,
      visibleInteractiveCount: allInteractive.length,
      minCardPaddingPx: minCardPaddingPx ?? 0,
      headingToBodyRatio,
      inlineHandlerViolations,
      bodyBg,
      effectiveBodyBg,
      cardBg,
      textColor,
      harshBodyBg,
      minContrastRatio: Number(minContrastRatio.toFixed(2)),
      contrastViolations: contrastViolations.slice(0, 25),
      contrastViolationsTotal: contrastViolations.length,
      gsapLoaded,
      cssTransitionsCount,
      overlapViolations: overlapViolations.slice(0, 20),
      overlapViolationsTotal: overlapViolations.length,
      clippedLabels: clippedLabels.slice(0, 20),
      brokenImages,
      zeroSizeSvgs: zeroSizeSvgs.slice(0, 20),
      appliedTheme: document.documentElement.getAttribute("data-theme") || "",
    };
  }, { surfaceName: surface, activeTheme: theme });

  // 5. Console, page & network hygiene
  const consoleErrors = [...(pageRes.consoleErrors || [])];
  const pageErrors = [...(pageRes.pageErrors || [])];
  const rawServerErrors = [...(pageRes.serverErrors || [])];
  const rawClientErrors = [...(pageRes.clientErrors || [])];
  const failedRequests = [...(pageRes.failedRequests || [])];

  // Filter tolerated isolated monitoring/bundle 503 if isolated server
  const filteredServerErrors = rawServerErrors.filter(
    (e) => !(isIsolated && e.status === 503 && e.url.includes("/api/v1/admin/monitoring/bundle"))
  );
  const filteredConsoleErrors = consoleErrors.filter((line) => {
    if (isIsolated && (line.includes("503") || line.includes("Service Unavailable"))) {
      return false;
    }
    return true;
  });

  const httpErrors = [
    ...filteredServerErrors,
    ...rawClientErrors,
    ...failedRequests.map((f) => ({ status: 0, url: f.url, failure: f.failure })),
  ];

  const corePassed =
    !overflow.hasHorizontalOverflow &&
    domMetrics.inlineHandlerViolations.length === 0 &&
    domMetrics.overlapViolationsTotal === 0 &&
    domMetrics.clippedLabels.length === 0 &&
    domMetrics.brokenImages.length === 0 &&
    domMetrics.zeroSizeSvgs.length === 0 &&
    filteredConsoleErrors.length === 0 &&
    pageErrors.length === 0 &&
    httpErrors.length === 0;

  const strictExtrasPassed =
    domMetrics.contrastViolationsTotal === 0 &&
    (!expectGsap || domMetrics.gsapLoaded) &&
    !domMetrics.harshBodyBg;

  return {
    corePassed,
    strictPassed: corePassed && strictExtrasPassed,
    metrics: {
      hasHorizontalOverflow: overflow.hasHorizontalOverflow,
      scrollWidth: overflow.scrollWidth,
      clientWidth: overflow.clientWidth,
      innerWidth: overflow.innerWidth,
      ...domMetrics,
      consoleErrors: filteredConsoleErrors,
      pageErrors,
      httpErrors,
    },
  };
}

async function runAuditCheck(bm, {
  id,
  surface,
  route,
  role,
  theme,
  viewport,
  context,
  outDir,
  isIsolated,
  expectGsap = false,
  afterOpen = null,
}) {
  const pageRes = await bm.openPage(route, {
    context,
    waitUntil: "networkidle",
    settleMs: 120,
  });
  try {
    await setTheme(pageRes.page, theme);
    if (afterOpen) {
      await afterOpen(pageRes);
    }

    const shotsDir = path.join(outDir, "screenshots");
    const safeRole = String(role || "guest").replace(/[^a-z0-9_-]/gi, "_");
    const safeSurface = String(surface).replace(/[^a-z0-9_-]/gi, "_");
    const baseName = `${safeSurface}_${safeRole}_${theme}_${viewport.name}`;
    const fullPath = path.join(shotsDir, `${baseName}_full.png`);
    const viewportPath = path.join(shotsDir, `${baseName}_viewport.png`);

    await captureScreenshot(pageRes.page, fullPath, { fullPage: true });
    await captureScreenshot(pageRes.page, viewportPath, { fullPage: false });

    const evalRes = await evaluatePageUxRubric(pageRes, {
      surface,
      theme,
      isIsolated,
      expectGsap,
    });

    return {
      id,
      surface,
      route,
      role,
      theme,
      viewport: `${viewport.width}x${viewport.height}`,
      status: pageRes.status,
      screenshotFullPath: fullPath,
      screenshotViewportPath: viewportPath,
      corePassed: evalRes.corePassed,
      strictPassed: evalRes.strictPassed,
      metrics: evalRes.metrics,
    };
  } finally {
    await pageRes.close();
  }
}

export async function runUxRubricAudit(rawOpts = {}) {
  const opts = {
    baseUrl: rawOpts.baseUrl || null,
    outDir: rawOpts.outDir || `/tmp/msi_ux_audit_${Date.now()}`,
    theme: rawOpts.theme || "both",
    viewports: rawOpts.viewports || ["desktop", "mobile"],
    adminPath: rawOpts.adminPath || "podmin",
    adminEmail: rawOpts.adminEmail || "",
    adminPassword: rawOpts.adminPassword || "",
    roleCredsPath: rawOpts.roleCredsPath || null,
    provisionRoles: Boolean(rawOpts.provisionRoles),
    keepUsers: Boolean(rawOpts.keepUsers),
    strict: Boolean(rawOpts.strict),
    quick: Boolean(rawOpts.quick),
    surfaces: rawOpts.surfaces || null,
  };

  const outDir = path.resolve(opts.outDir);
  fs.mkdirSync(path.join(outDir, "screenshots"), { recursive: true });

  const themes = resolveThemes(opts.theme);
  const viewports = resolveViewports(opts.viewports);
  const adminSeg = String(opts.adminPath || "podmin").replace(/^\/+|\/+$/g, "") || "podmin";

  let server = null;
  let isIsolated = false;
  let baseUrl = opts.baseUrl ? opts.baseUrl.replace(/\/+$/, "") : null;

  if (!baseUrl) {
    server = await startIsolatedServer({
      repoRoot: DEFAULT_REPO_ROOT,
      adminPath: adminSeg,
    });
    baseUrl = server.baseUrl;
    isIsolated = true;
  }

  const http = createHttpFactory({ baseUrl });
  const bm = createBrowserManager({
    baseUrl,
    repoRoot: DEFAULT_REPO_ROOT,
    screenshotDir: path.join(outDir, "screenshots"),
  });

  const createdUserIds = [];
  let adminClient = null;
  const roleCreds = {};

  try {
    const auth = createAuthHelpers({
      baseUrl,
      http,
      credentials: server ? server.credentials : { admin: { email: opts.adminEmail, password: opts.adminPassword }, superAdmin: { email: opts.adminEmail, password: opts.adminPassword } },
    });

    if (opts.roleCredsPath && fs.existsSync(opts.roleCredsPath)) {
      const parsed = JSON.parse(fs.readFileSync(opts.roleCredsPath, "utf8"));
      Object.assign(roleCreds, parsed);
    }

    if (isIsolated) {
      const sa = await auth.loginAsSuperAdmin();
      adminClient = sa.client;
      roleCreds.super_admin = {
        email: sa.email,
        password: sa.password,
        client: sa.client,
      };
      roleCreds.site_operator = {
        email: server.credentials.admin.email,
        password: server.credentials.admin.password,
      };
    } else if (opts.adminEmail && opts.adminPassword) {
      const c = http.createClient();
      await auth.login(c, { email: opts.adminEmail, password: opts.adminPassword });
      await c.ensureCsrf();
      adminClient = c;
      roleCreds.super_admin = { email: opts.adminEmail, password: opts.adminPassword, client: c };
    }

    const getOrProvisionRoleClient = async (role) => {
      if (roleCreds[role]?.client) return roleCreds[role].client;
      if (roleCreds[role]?.email && roleCreds[role]?.password) {
        const c = http.createClient();
        await auth.login(c, roleCreds[role]);
        await c.ensureCsrf();
        roleCreds[role].client = c;
        return c;
      }
      if (adminClient && (isIsolated || opts.provisionRoles)) {
        const created = await auth.createUserWithRole(role, {
          superAdminClient: adminClient,
          subscription_tier:
            role === "subscriber_digital"
              ? "dijital"
              : role === "subscriber_print_digital"
                ? "basili-dijital"
                : role === "subscriber_corporate"
                  ? "kurumsal"
                  : "",
        });
        if (created.id) createdUserIds.push(created.id);
        roleCreds[role] = created;
        return created.client;
      }
      return null;
    };

    const allowSurface = (name) => !opts.surfaces || opts.surfaces.includes(name);
    const results = [];

    const staffRolesToAudit = opts.quick
      ? ["super_admin", "editor_in_chief", "ads_manager"]
      : STAFF_ROLES;
    const readerRolesToAudit = opts.quick
      ? ["registered", "subscriber_public"]
      : READER_ROLES;
    const hesabimStaffToAudit = opts.quick
      ? ["super_admin", "content_editor"]
      : ["super_admin", "editor_in_chief", "content_editor"];

    // 1. Unauthenticated surfaces
    if (allowSurface("public_auth")) {
      const unauthTargets = [
        { surface: "admin_login", route: `/${adminSeg}`, expectGsap: true },
        { surface: "hesabim_guest", route: "/hesabim", expectGsap: false },
        {
          surface: "public_auth_dialog",
          route: "/hesabim",
          expectGsap: false,
          afterOpen: async (pRes) => {
            await pRes.page.locator('a[href="#modal-login"]').first().click();
            await pRes.page.locator("#auth-dialog").waitFor({ state: "visible", timeout: 5000 });
          },
        },
        { surface: "reset_password", route: "/sifre-sifirla?token=preview", expectGsap: false },
      ];

      for (const vp of viewports) {
        for (const theme of themes) {
          const ctx = await bm.newContext({ viewport: { width: vp.width, height: vp.height } });
          try {
            await ctx.addInitScript((t) => {
              try {
                window.localStorage.setItem("msi_admin_theme", t);
                window.localStorage.setItem("msi_theme", t);
              } catch {
                // ignore
              }
            }, theme);
            for (const target of unauthTargets) {
              const entry = await runAuditCheck(bm, {
                id: `${target.surface}:guest:${theme}:${vp.name}`,
                surface: target.surface,
                route: target.route,
                role: "guest",
                theme,
                viewport: vp,
                context: ctx,
                outDir,
                isIsolated,
                expectGsap: target.expectGsap,
                afterOpen: target.afterOpen,
              });
              results.push(entry);
            }
          } finally {
            await ctx.close();
          }
        }
      }
    }

    // 2. Design Studio (/podmin/tasarim)
    if (allowSurface("design_studio")) {
      const saClient = await getOrProvisionRoleClient("super_admin");
      if (saClient) {
        for (const vp of viewports) {
          for (const theme of themes) {
            const ctx = await bm.newContext({
              client: saClient,
              viewport: { width: vp.width, height: vp.height },
            });
            try {
              await ctx.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_admin_theme", t);
                  window.localStorage.setItem("msi_theme", t);
                } catch {
                  // ignore
                }
              }, theme);
              const entry = await runAuditCheck(bm, {
                id: `design_studio:super_admin:${theme}:${vp.name}`,
                surface: "design_studio",
                route: `/${adminSeg}/tasarim`,
                role: "super_admin",
                theme,
                viewport: vp,
                context: ctx,
                outDir,
                isIsolated,
                afterOpen: async (pRes) => {
                  await pRes.page.waitForFunction(
                    () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
                    { timeout: 15000 }
                  );
                },
              });
              results.push(entry);
            } finally {
              await ctx.close();
            }
          }
        }
      }
    }

    // 2b. Editorial Studio (/podmin/editor)
    if (allowSurface("editorial_studio")) {
      const chiefClient = await getOrProvisionRoleClient("editor_in_chief");
      if (chiefClient) {
        for (const vp of viewports) {
          for (const theme of themes) {
            const ctx = await bm.newContext({
              client: chiefClient,
              viewport: { width: vp.width, height: vp.height },
            });
            try {
              await ctx.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_admin_theme", t);
                  window.localStorage.setItem("msi_theme", t);
                } catch {
                  // ignore
                }
              }, theme);
              const entry = await runAuditCheck(bm, {
                id: `editorial_studio:editor_in_chief:${theme}:${vp.name}`,
                surface: "editorial_studio",
                route: `/${adminSeg}/editor`,
                role: "editor_in_chief",
                theme,
                viewport: vp,
                context: ctx,
                outDir,
                isIsolated,
                afterOpen: async (pRes) => {
                  await pRes.page.locator("#hf-header-slot").waitFor({ state: "visible", timeout: 20000 });
                  await pRes.page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
                },
              });
              results.push(entry);
            } finally {
              await ctx.close();
            }
          }
        }
      }
    }

    // 3. Admin Command Center (/podmin) — all 12 staff roles
    if (allowSurface("admin_command")) {
      for (const role of staffRolesToAudit) {
        const client = await getOrProvisionRoleClient(role);
        if (!client) continue;

        for (const vp of viewports) {
          for (const theme of themes) {
            const ctx = await bm.newContext({
              client,
              viewport: { width: vp.width, height: vp.height },
            });
            try {
              await ctx.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_admin_theme", t);
                  window.localStorage.setItem("msi_theme", t);
                } catch {
                  // ignore
                }
              }, theme);
              const entry = await runAuditCheck(bm, {
                id: `admin_command:${role}:${theme}:${vp.name}`,
                surface: "admin_command",
                route: `/${adminSeg}`,
                role,
                theme,
                viewport: vp,
                context: ctx,
                outDir,
                isIsolated,
                expectGsap: true,
                afterOpen: async (pRes) => {
                  await pRes.page.waitForFunction(
                    () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
                    { timeout: 15000 }
                  );
                  await pRes.page.waitForTimeout(180);

                  // Interactive drawer & tab transition settling check for super_admin on desktop light/dark
                  if (role === "super_admin" && vp.name === "desktop") {
                    const openBtn = pRes.page.locator('[data-admin-action="open-content-drawer"]').first();
                    if (await openBtn.isVisible().catch(() => false)) {
                      await openBtn.click();
                      await pRes.page.locator("#drawer-content").waitFor({ state: "visible", timeout: 5000 });
                      await pRes.page.waitForTimeout(160);
                      await pRes.page.locator('#drawer-content [data-admin-action="close-drawer"]').first().click();
                      await pRes.page.locator("#drawer-content").waitFor({ state: "hidden", timeout: 5000 });
                    }
                  }
                },
              });
              results.push(entry);
            } finally {
              await ctx.close();
            }
          }
        }
      }
    }

    // 4. Account Role Portal (/hesabim) — 5 reader roles + representative staff
    if (allowSurface("account_portal")) {
      for (const role of [...readerRolesToAudit, ...hesabimStaffToAudit]) {
        const client = await getOrProvisionRoleClient(role);
        if (!client) continue;

        for (const vp of viewports) {
          for (const theme of themes) {
            const ctx = await bm.newContext({
              client,
              viewport: { width: vp.width, height: vp.height },
            });
            try {
              await ctx.addInitScript((t) => {
                try {
                  window.localStorage.setItem("msi_admin_theme", t);
                  window.localStorage.setItem("msi_theme", t);
                } catch {
                  // ignore
                }
              }, theme);
              const entry = await runAuditCheck(bm, {
                id: `account_portal:${role}:${theme}:${vp.name}`,
                surface: "account_portal",
                route: "/hesabim",
                role,
                theme,
                viewport: vp,
                context: ctx,
                outDir,
                isIsolated,
              });
              results.push(entry);
            } finally {
              await ctx.close();
            }
          }
        }
      }
    }

    // 5. Reduced-motion settling checks across all allowed surfaces
    const reducedMotionChecks = [];
    if (allowSurface("public_auth") || allowSurface("admin_login")) {
      const ctx = await bm.newContext({ viewport: VIEWPORT_DESKTOP });
      try {
        const pRes = await bm.openPage(`/${adminSeg}`, { context: ctx });
        try {
          await emulateReducedMotion(pRes.page, true);
          await pRes.page.reload({ waitUntil: "networkidle" });
          await pRes.page.waitForTimeout(120);
          const opacity = await pRes.page.locator(".login-card").evaluate((el) =>
            Number.parseFloat(window.getComputedStyle(el).opacity)
          );
          reducedMotionChecks.push({ route: `/${adminSeg} (guest)`, settled: opacity >= 0.9, opacity });
        } finally {
          await pRes.close();
        }
      } finally {
        await ctx.close();
      }
    }

    if (allowSurface("admin_command")) {
      const saClient = await getOrProvisionRoleClient("super_admin");
      if (saClient) {
        const ctx = await bm.newContext({ client: saClient, viewport: VIEWPORT_DESKTOP });
        try {
          const pRes = await bm.openPage(`/${adminSeg}`, { context: ctx });
          try {
            await emulateReducedMotion(pRes.page, true);
            await pRes.page.reload({ waitUntil: "networkidle" });
            await pRes.page.waitForFunction(
              () => Boolean(window.MSIAdmin && window.MSIAdmin._me && window.MSIAdmin._me.role),
              { timeout: 15000 }
            );
            await pRes.page.waitForTimeout(150);
            const cmdState = await pRes.page.evaluate(() => {
              const sidebar = document.querySelector("#admin-sidebar");
              const pane = document.querySelector(".tab-pane.active") || document.querySelector("#tab-1");
              const sidebarOp = sidebar ? Number.parseFloat(window.getComputedStyle(sidebar).opacity) : 0;
              const paneOp = pane ? Number.parseFloat(window.getComputedStyle(pane).opacity) : 0;
              return { sidebarOp, paneOp };
            });
            reducedMotionChecks.push({
              route: `/${adminSeg} (super_admin)`,
              settled: cmdState.sidebarOp >= 0.9 && cmdState.paneOp >= 0.9,
              opacity: Math.min(cmdState.sidebarOp, cmdState.paneOp),
              sidebarOpacity: cmdState.sidebarOp,
              activePaneOpacity: cmdState.paneOp,
            });
          } finally {
            await pRes.close();
          }
        } finally {
          await ctx.close();
        }
      }
    }

    if (allowSurface("account_portal")) {
      const regClient = await getOrProvisionRoleClient("registered");
      if (regClient) {
        const ctx = await bm.newContext({ client: regClient, viewport: VIEWPORT_DESKTOP });
        try {
          const pRes = await bm.openPage("/hesabim", { context: ctx });
          try {
            await emulateReducedMotion(pRes.page, true);
            await pRes.page.reload({ waitUntil: "networkidle" });
            await pRes.page.waitForTimeout(150);
            const opacity = await pRes.page.locator("#ap-panel-overview").evaluate((el) =>
              Number.parseFloat(window.getComputedStyle(el).opacity)
            );
            reducedMotionChecks.push({
              route: "/hesabim (registered)",
              settled: opacity >= 0.9,
              opacity,
            });
          } finally {
            await pRes.close();
          }
        } finally {
          await ctx.close();
        }
      }
    }

    if (allowSurface("design_studio")) {
      const saClient = await getOrProvisionRoleClient("super_admin");
      if (saClient) {
        const ctx = await bm.newContext({ client: saClient, viewport: VIEWPORT_DESKTOP });
        try {
          const pRes = await bm.openPage(`/${adminSeg}/tasarim`, { context: ctx });
          try {
            await emulateReducedMotion(pRes.page, true);
            await pRes.page.reload({ waitUntil: "networkidle" });
            await pRes.page.waitForFunction(
              () => Boolean(document.querySelector("#design-preview")?.contentDocument?.body?.innerText?.trim()),
              { timeout: 15000 }
            );
            await pRes.page.waitForTimeout(150);
            const studioState = await pRes.page.evaluate(() => {
              const hdr = document.querySelector(".editor-header");
              const frame = document.querySelector("#design-preview");
              const hdrOp = hdr ? Number.parseFloat(window.getComputedStyle(hdr).opacity) : 0;
              const frameOp = frame ? Number.parseFloat(window.getComputedStyle(frame).opacity) : 0;
              const frameRect = frame ? frame.getBoundingClientRect() : { width: 0, height: 0 };
              return { hdrOp, frameOp, frameReady: frameRect.width > 0 && frameRect.height > 0 };
            });
            reducedMotionChecks.push({
              route: `/${adminSeg}/tasarim (super_admin)`,
              settled: studioState.hdrOp >= 0.9 && studioState.frameOp >= 0.9 && studioState.frameReady,
              opacity: Math.min(studioState.hdrOp, studioState.frameOp),
              headerOpacity: studioState.hdrOp,
              previewOpacity: studioState.frameOp,
              previewReady: studioState.frameReady,
            });
          } finally {
            await pRes.close();
          }
        } finally {
          await ctx.close();
        }
      }
    }

    if (allowSurface("editorial_studio")) {
      const chiefClient = await getOrProvisionRoleClient("editor_in_chief");
      if (chiefClient) {
        const ctx = await bm.newContext({ client: chiefClient, viewport: VIEWPORT_DESKTOP });
        try {
          const pRes = await bm.openPage(`/${adminSeg}/editor`, { context: ctx });
          try {
            await emulateReducedMotion(pRes.page, true);
            await pRes.page.reload({ waitUntil: "networkidle" });
            await pRes.page.locator("#hf-header-slot").waitFor({ state: "visible", timeout: 20000 });
            await pRes.page.locator("#hf-panel-group").waitFor({ state: "visible", timeout: 20000 });
            await pRes.page.waitForTimeout(150);
            const edOpacity = await pRes.page.locator("#hf-panel-group").evaluate((el) =>
              Number.parseFloat(window.getComputedStyle(el).opacity)
            );
            reducedMotionChecks.push({
              route: `/${adminSeg}/editor (editor_in_chief)`,
              settled: edOpacity >= 0.9,
              opacity: edOpacity,
            });
          } finally {
            await pRes.close();
          }
        } finally {
          await ctx.close();
        }
      }
    }

    const totalChecks = results.length;
    const passedChecks = results.filter((r) => (opts.strict ? r.strictPassed : r.corePassed)).length;
    const failedChecks = totalChecks - passedChecks;
    const reducedMotionOk = reducedMotionChecks.every((c) => c.settled);

    const report = {
      summary: {
        totalChecks,
        passed: passedChecks,
        failed: failedChecks + (reducedMotionOk ? 0 : 1),
        corePassedCount: results.filter((r) => r.corePassed).length,
        strictPassedCount: results.filter((r) => r.strictPassed).length,
        reducedMotionOk,
        strictMode: opts.strict,
        isIsolated,
        baseUrl,
        themes,
        viewports: viewports.map((v) => `${v.width}x${v.height}`),
        timestamp: new Date().toISOString(),
      },
      reducedMotionChecks,
      results,
    };

    const reportPath = path.join(outDir, "ux_rubric_report.json");
    fs.writeFileSync(reportPath, JSON.stringify(report, null, 2), "utf8");
    return { report, reportPath, outDir };
  } finally {
    // Cleanup ephemeral provisioned users on live targets
    if (!isIsolated && adminClient && createdUserIds.length > 0 && !opts.keepUsers) {
      await adminClient.ensureCsrf().catch(() => {});
      for (const uid of createdUserIds) {
        await adminClient.delete(`/api/v1/admin/users/${uid}`).catch(() => {});
      }
    }
    await bm.close().catch(() => {});
    if (server) {
      await server.stop().catch(() => {});
    }
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === __filename) {
  (async () => {
    const opts = parseCliArgs(process.argv.slice(2));
    if (opts.help) {
      console.log(`MSI UX Rubric Audit Harness

Usage:
  node tests/e2e/suites/tier4/ux_rubric_audit.mjs [options]

Options:
  --base-url=<url>        Audit external target (default: isolated server with adminPath=podmin)
  --out-dir=<path>        Output directory for screenshots/ & ux_rubric_report.json
  --theme=light|dark|both Theme(s) to audit (default: both)
  --viewports=desktop,mobile
  --admin-path=<slug>     Admin route slug (default: podmin)
  --admin-email=<email>   Admin email for external targets
  --admin-password=<pw>   Admin password for external targets
  --role-creds=<json>     JSON file mapping role -> { email, password }
  --provision-roles       Provision temporary role accounts on external target (cleaned up on exit)
  --surfaces=<list>       Filter surfaces: public_auth,design_studio,editorial_studio,admin_command,account_portal
  --quick                 Run compact representative role subset
  --strict                Fail if WCAG contrast, harsh body bg, or GSAP checks fail
`);
      process.exit(0);
    }

    const { report, reportPath } = await runUxRubricAudit(opts);
    console.log(
      `UX Rubric Audit complete: ${report.summary.passed}/${report.summary.totalChecks} passed (core=${report.summary.corePassedCount}, strict=${report.summary.strictPassedCount}, reducedMotion=${report.summary.reducedMotionOk})`
    );
    console.log(`Report written: ${reportPath}`);
    for (const r of report.results) {
      const ok = opts.strict ? r.strictPassed : r.corePassed;
      if (!ok) {
        console.error(
          `  FAIL [${r.id}] overflow=${r.metrics.hasHorizontalOverflow} overlaps=${r.metrics.overlapViolationsTotal} clipped=${r.metrics.clippedLabels.length} contrastViolations=${r.metrics.contrastViolationsTotal} harshBg=${r.metrics.harshBodyBg} brokenImgs=${r.metrics.brokenImages.length} zeroSvgs=${r.metrics.zeroSizeSvgs.length} httpErrs=${r.metrics.httpErrors.length} consoleErrs=${r.metrics.consoleErrors.length}`
        );
      }
    }
    process.exit(report.summary.failed === 0 ? 0 : 1);
  })().catch((err) => {
    console.error("UX Rubric Audit fatal error:", err?.stack || err);
    process.exit(1);
  });
}
