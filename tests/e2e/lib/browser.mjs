import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { Client } from "./http.mjs";
import { generateTotp } from "./totp.mjs";

export const VIEWPORT_DESKTOP = Object.freeze({ width: 1440, height: 900 });
export const VIEWPORT_MOBILE = Object.freeze({ width: 390, height: 844 });

function globMatches(baseDir, subPatternFn) {
  try {
    if (!fs.existsSync(baseDir)) return [];
    const results = [];
    for (const entry of fs.readdirSync(baseDir)) {
      const candidate = subPatternFn(path.join(baseDir, entry));
      if (candidate && fs.existsSync(candidate)) {
        results.push(candidate);
      }
    }
    return results;
  } catch {
    return [];
  }
}

export function discoverPlaywrightModule(repoRoot) {
  const require = createRequire(import.meta.url);
  const candidates = [];

  if (process.env.PLAYWRIGHT_MODULE) {
    candidates.push(process.env.PLAYWRIGHT_MODULE);
  }
  candidates.push(path.join(repoRoot, "node_modules", "playwright"));
  candidates.push(
    ...globMatches("/root/.npm/_npx", (dir) => path.join(dir, "node_modules", "playwright"))
  );
  candidates.push(
    ...globMatches("/var/www/vhosts", (dir) =>
      path.join(dir, "httpdocs", "node_modules", "playwright")
    )
  );
  candidates.push("playwright");

  for (const modPath of candidates) {
    try {
      const resolved = require.resolve(modPath);
      const pw = require(resolved);
      if (pw && pw.chromium) {
        return { playwright: pw, modulePath: resolved };
      }
    } catch {
      // continue probing
    }
  }

  throw new Error(
    `Playwright module not found. Probed: ${candidates.join(", ")}. Set PLAYWRIGHT_MODULE to override.`
  );
}

export function discoverChromiumExecutable() {
  const candidates = [];
  if (process.env.PLAYWRIGHT_EXECUTABLE_PATH) {
    candidates.push(process.env.PLAYWRIGHT_EXECUTABLE_PATH);
  }

  const cacheDir = "/root/.cache/ms-playwright";
  if (fs.existsSync(cacheDir)) {
    const entries = fs.readdirSync(cacheDir).sort().reverse();
    for (const entry of entries) {
      if (entry.startsWith("chromium-")) {
        candidates.push(path.join(cacheDir, entry, "chrome-linux", "chrome"));
      }
      if (entry.startsWith("chromium_headless_shell-")) {
        candidates.push(path.join(cacheDir, entry, "chrome-linux", "headless_shell"));
      }
    }
  }

  candidates.push("/usr/bin/chromium", "/usr/bin/chromium-browser", "/usr/bin/google-chrome");

  for (const bin of candidates) {
    if (bin && fs.existsSync(bin)) {
      return bin;
    }
  }
  return null;
}

export async function measureOverflow(page) {
  return await page.evaluate(() => {
    const doc = document.documentElement;
    return {
      scrollWidth: doc.scrollWidth,
      clientWidth: doc.clientWidth,
      innerWidth: window.innerWidth,
      hasHorizontalOverflow: doc.scrollWidth > window.innerWidth,
    };
  });
}

export async function captureScreenshot(page, outPath, opts = {}) {
  const raw = String(outPath);
  const targetPath = path.isAbsolute(raw)
    ? path.resolve(raw)
    : path.resolve("/tmp/msi_e2e_screenshots", raw);
  fs.mkdirSync(path.dirname(targetPath), { recursive: true });
  await page.screenshot({
    path: targetPath,
    fullPage: opts.fullPage ?? true,
    ...(opts.animations ? { animations: opts.animations } : {}),
    ...(opts.clip ? { clip: opts.clip } : {}),
  });
  return targetPath;
}

export async function setTheme(page, theme = "light") {
  const normalized = theme === "dark" ? "dark" : "light";
  await page.evaluate((t) => {
    try {
      window.localStorage.setItem("msi_admin_theme", t);
      window.localStorage.setItem("msi_theme", t);
    } catch {
      // ignore storage errors on non-storage origins
    }
    const sec = typeof location !== "undefined" && location.protocol === "https:" ? "; Secure" : "";
    document.cookie = `msi_admin_theme=${encodeURIComponent(t)}; path=/; max-age=31536000; SameSite=Lax${sec}`;
    if (window.MSIAdminTheme && typeof window.MSIAdminTheme.applyTheme === "function") {
      window.MSIAdminTheme.applyTheme(t);
    } else {
      document.documentElement.setAttribute("data-theme", t);
      if (document.body) {
        document.body.classList.toggle("admin-dark-mode", t === "dark");
        document.body.classList.toggle("admin-light-mode", t === "light");
        document.body.classList.toggle("gw-light-mode", t === "light");
      }
    }
    window.dispatchEvent(new CustomEvent("msi:admin:theme", { detail: { theme: t } }));
    window.dispatchEvent(new CustomEvent("msi:theme", { detail: { theme: t } }));
  }, normalized);
  await page.waitForTimeout(180);
  return normalized;
}

export async function emulateReducedMotion(page, enabled = true) {
  await page.emulateMedia({
    reducedMotion: enabled ? "reduce" : "no-preference",
  });
  return Boolean(enabled);
}

export function createBrowserManager({ baseUrl, repoRoot, screenshotDir = null }) {
  let cachedBrowser = null;
  let launchPromise = null;
  let discoveredInfo = null;
  const contextHosts = new WeakMap();
  const parsedBaseUrl = new URL(baseUrl);
  const isLocalTarget =
    parsedBaseUrl.hostname === "127.0.0.1" || parsedBaseUrl.hostname === "localhost";

  function resolveEffectiveBaseUrl(host = null) {
    if (!host) return baseUrl;
    const rawHost = String(host).trim();
    if (!rawHost) return baseUrl;
    const hasPort = /:\d+$/.test(rawHost);
    const hostWithPort = hasPort || !parsedBaseUrl.port ? rawHost : `${rawHost}:${parsedBaseUrl.port}`;
    return `${parsedBaseUrl.protocol}//${hostWithPort}`;
  }

  function resolveTargetUrl(pathOrUrl, host = null) {
    const effectiveBase = resolveEffectiveBaseUrl(host);
    if (/^https?:\/\//i.test(pathOrUrl)) {
      if (!host) return pathOrUrl;
      const target = new URL(pathOrUrl);
      const eff = new URL(effectiveBase);
      if (target.hostname === parsedBaseUrl.hostname || target.hostname === "127.0.0.1" || target.hostname === "localhost") {
        target.protocol = eff.protocol;
        target.hostname = eff.hostname;
        target.port = eff.port;
      }
      return target.toString();
    }
    return `${effectiveBase}${pathOrUrl.startsWith("/") ? "" : "/"}${pathOrUrl}`;
  }

  async function getBrowser() {
    if (cachedBrowser && cachedBrowser.isConnected()) {
      return cachedBrowser;
    }
    if (launchPromise) {
      return await launchPromise;
    }

    launchPromise = (async () => {
      const { playwright, modulePath } = discoverPlaywrightModule(repoRoot);
      const executablePath = discoverChromiumExecutable();
      discoveredInfo = { modulePath, executablePath };

      const launchArgs = ["--no-sandbox"];
      if (isLocalTarget) {
        launchArgs.push("--host-resolver-rules=MAP * 127.0.0.1");
      }

      const browser = await playwright.chromium.launch({
        headless: true,
        ...(executablePath ? { executablePath } : {}),
        args: launchArgs,
      });
      cachedBrowser = browser;
      return browser;
    })();

    try {
      return await launchPromise;
    } finally {
      launchPromise = null;
    }
  }

  async function authenticateContext(context, clientOrCreds, options = {}) {
    if (!clientOrCreds) return;
    const activeHost = options.host ?? contextHosts.get(context) ?? null;
    const effectiveBase = resolveEffectiveBaseUrl(activeHost);

    if (typeof clientOrCreds.exportCookies === "function") {
      const cookies = clientOrCreds.exportCookies(effectiveBase);
      if (cookies.length > 0) {
        await context.addCookies(cookies);
      }
      return;
    }
    if (clientOrCreds.email && clientOrCreds.password) {
      const httpClient = new Client({
        baseUrl,
        ...(activeHost ? { defaultHost: activeHost } : {}),
      });
      const resp = await httpClient.post("/api/v1/auth/login", {
        json: {
          email: clientOrCreds.email,
          password: clientOrCreds.password,
        },
      });
      if (!resp.ok) {
        throw new Error(
          `Playwright context login failed (${resp.status}) for ${clientOrCreds.email}`
        );
      }
      const data = resp.json;
      if (!data || typeof data !== "object") {
        throw new Error(
          `Playwright context login returned non-JSON response for ${clientOrCreds.email}`
        );
      }

      if (data.status === "totp_required" || data.requires_totp) {
        const tempToken = data.temp_token;
        if (!tempToken) {
          throw new Error(
            `Playwright context login for ${clientOrCreds.email} returned totp_required without temp_token`
          );
        }
        const code = clientOrCreds.backupCode || (clientOrCreds.totpSecret ? generateTotp(clientOrCreds.totpSecret) : null);
        if (!code) {
          throw new Error(
            `Playwright context login for ${clientOrCreds.email} requires TOTP 2FA (status="totp_required"), but no totpSecret or backupCode was provided`
          );
        }
        const totpResp = await httpClient.post("/api/v1/auth/totp/login", {
          json: {
            temp_token: tempToken,
            code,
          },
        });
        if (!totpResp.ok) {
          throw new Error(
            `Playwright context TOTP login failed (${totpResp.status}) for ${clientOrCreds.email}`
          );
        }
        const totpData = totpResp.json;
        if (totpData?.status !== "authenticated") {
          throw new Error(
            `Playwright context TOTP login for ${clientOrCreds.email} returned unexpected status "${totpData?.status}"`
          );
        }
      } else if (data.status !== "authenticated") {
        throw new Error(
          `Playwright context login for ${clientOrCreds.email} returned unexpected status "${data.status}"`
        );
      }

      const cookies = httpClient.exportCookies(effectiveBase);
      if (cookies.length > 0) {
        await context.addCookies(cookies);
      }
    }
  }

  async function newContext(options = {}) {
    const browser = await getBrowser();
    const extraHTTPHeaders = { ...(options.extraHTTPHeaders || {}) };
    for (const key of Object.keys(extraHTTPHeaders)) {
      if (key.toLowerCase() === "host") {
        delete extraHTTPHeaders[key];
      }
    }

    const context = await browser.newContext({
      viewport: options.viewport || VIEWPORT_DESKTOP,
      locale: options.locale || "tr-TR",
      ...(Object.keys(extraHTTPHeaders).length > 0 ? { extraHTTPHeaders } : {}),
    });

    if (options.host) {
      contextHosts.set(context, options.host);
    }

    if (options.cookies && Array.isArray(options.cookies) && options.cookies.length > 0) {
      await context.addCookies(options.cookies);
    }
    if (options.client) {
      await authenticateContext(context, options.client, { host: options.host });
    } else if (options.credentials) {
      await authenticateContext(context, options.credentials, { host: options.host });
    }

    return context;
  }

  async function openPage(pathOrUrl, options = {}) {
    const ownContext = !options.context;
    const context = options.context || (await newContext(options));
    const activeHost = options.host ?? contextHosts.get(context) ?? null;
    const page = await context.newPage();

    const nativeDialogs = [];
    page.on("dialog", (d) => {
      nativeDialogs.push({
        type: d.type(),
        message: d.message(),
      });
      if (options.autoAcceptDialogs === false) {
        d.dismiss().catch(() => {});
      } else {
        d.accept().catch(() => {});
      }
    });

    const consoleErrors = [];
    const pageErrors = [];
    const serverErrors = [];
    const clientErrors = [];
    const failedRequests = [];

    page.on("console", (msg) => {
      if (msg.type() === "error") {
        consoleErrors.push(msg.text());
      }
    });
    page.on("pageerror", (err) => {
      pageErrors.push(err?.message || String(err));
    });
    page.on("response", (res) => {
      const status = res.status();
      if (status >= 500) {
        serverErrors.push({ url: res.url(), status });
      } else if (status >= 400) {
        clientErrors.push({
          url: res.url(),
          status,
          resourceType: res.request().resourceType(),
          method: res.request().method(),
        });
      }
    });
    page.on("requestfailed", (req) => {
      failedRequests.push({
        url: req.url(),
        resourceType: req.resourceType(),
        method: req.method(),
        failure: req.failure()?.errorText || "requestfailed",
      });
    });

    const targetUrl = resolveTargetUrl(pathOrUrl, activeHost);

    const navResponse = await page.goto(targetUrl, {
      waitUntil: options.waitUntil || "networkidle",
      timeout: options.timeout ?? 25000,
    });

    if (options.settleMs) {
      await page.waitForTimeout(options.settleMs);
    }

    const overflow = await measureOverflow(page);

    const baseScreenshotDir = screenshotDir || "/tmp/msi_e2e_screenshots";

    return {
      context,
      page,
      url: page.url(),
      status: navResponse ? navResponse.status() : null,
      nativeDialogs,
      consoleErrors,
      pageErrors,
      serverErrors,
      clientErrors,
      failedRequests,
      overflow,
      measureOverflow: () => measureOverflow(page),
      captureScreenshot: (nameOrPath, shotOpts = {}) => {
        const resolved = !path.isAbsolute(String(nameOrPath))
          ? path.join(baseScreenshotDir, String(nameOrPath))
          : String(nameOrPath);
        return captureScreenshot(page, resolved, shotOpts);
      },
      setTheme: (theme) => setTheme(page, theme),
      emulateReducedMotion: (enabled = true) => emulateReducedMotion(page, enabled),
      async close() {
        await page.close().catch(() => {});
        if (ownContext) {
          await context.close().catch(() => {});
        }
      },
    };
  }

  async function close() {
    if (cachedBrowser) {
      const b = cachedBrowser;
      cachedBrowser = null;
      await b.close().catch(() => {});
    }
  }

  return {
    VIEWPORT_DESKTOP,
    VIEWPORT_MOBILE,
    screenshotDir,
    getBrowser,
    newContext,
    authenticateContext,
    openPage,
    measureOverflow,
    captureScreenshot: (page, nameOrPath, shotOpts = {}) => {
      const baseScreenshotDir = screenshotDir || "/tmp/msi_e2e_screenshots";
      const resolved = !path.isAbsolute(String(nameOrPath))
        ? path.join(baseScreenshotDir, String(nameOrPath))
        : String(nameOrPath);
      return captureScreenshot(page, resolved, shotOpts);
    },
    setTheme,
    emulateReducedMotion,
    close,
    getDiscoveryInfo: () => discoveredInfo,
  };
}
