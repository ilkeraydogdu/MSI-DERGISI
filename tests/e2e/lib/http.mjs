import http from "node:http";
import https from "node:https";

const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);
const REDIRECT_STATUSES = new Set([301, 302, 303, 307, 308]);

function safeDecodeURIComponent(val) {
  try {
    return decodeURIComponent(val);
  } catch {
    return val;
  }
}

function normalizePath(p) {
  if (!p || !p.startsWith("/")) return "/";
  return p;
}

function pathMatches(reqPath, cookiePath) {
  if (cookiePath === "/") return true;
  if (reqPath === cookiePath) return true;
  if (cookiePath.endsWith("/")) return reqPath.startsWith(cookiePath);
  return reqPath.startsWith(cookiePath + "/");
}

function splitCombinedSetCookie(headerStr) {
  // Fallback splitter if a single comma-joined string is encountered
  if (!headerStr) return [];
  const parts = [];
  let start = 0;
  let inExpires = false;
  for (let i = 0; i < headerStr.length; i++) {
    const slice = headerStr.slice(i, i + 8).toLowerCase();
    if (slice === "expires=") inExpires = true;
    if (inExpires && headerStr[i] === ";") inExpires = false;
    if (!inExpires && headerStr[i] === ",") {
      const candidate = headerStr.slice(start, i).trim();
      const after = headerStr.slice(i + 1).trim();
      if (/^[A-Za-z0-9_.-]+=/.test(after)) {
        parts.push(candidate);
        start = i + 1;
      }
    }
  }
  parts.push(headerStr.slice(start).trim());
  return parts.filter(Boolean);
}

/**
 * Stateful HTTP Client built on native Node v22 fetch + node:http (when Host override is active),
 * with per-client cookie jar, automatic CSRF header handling, Host overrides, multipart, and redirect control.
 */
export class Client {
  constructor(options = {}) {
    if (!options.baseUrl) {
      throw new Error("Client requires baseUrl");
    }
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.defaultHost = options.defaultHost || null;
    this.domainAliases = Array.isArray(options.domainAliases)
      ? options.domainAliases.map((d) => String(d).toLowerCase())
      : ["msidergisi.com", "savunmahaber.com"];
    this.defaultHeaders = { ...(options.defaultHeaders || {}) };
    this.timeoutMs = options.timeoutMs ?? 30000;
    this.autoCsrf = options.autoCsrf !== false;
    // Map keyed by `${name}\0${path}` -> { name, value, path, expiresAt, httpOnly, secure }
    this.jar = new Map();
  }

  _parseAndStoreSetCookies(setCookieHeaders, reqPath) {
    for (const raw of setCookieHeaders) {
      if (!raw || typeof raw !== "string") continue;
      const segments = raw.split(";").map((s) => s.trim());
      const first = segments.shift();
      if (!first) continue;
      const eqIdx = first.indexOf("=");
      if (eqIdx <= 0) continue;

      const name = first.slice(0, eqIdx).trim();
      const value = first.slice(eqIdx + 1).trim();
      let cookiePath = "/";
      let expiresAt = null;
      let httpOnly = false;
      let secure = false;
      let maxAge = null;

      for (const attr of segments) {
        const attrEq = attr.indexOf("=");
        const attrName = (attrEq === -1 ? attr : attr.slice(0, attrEq)).trim().toLowerCase();
        const attrVal = attrEq === -1 ? "" : attr.slice(attrEq + 1).trim();
        if (attrName === "path" && attrVal) {
          cookiePath = normalizePath(attrVal);
        } else if (attrName === "max-age") {
          const parsed = Number.parseInt(attrVal, 10);
          if (!Number.isNaN(parsed)) maxAge = parsed;
        } else if (attrName === "expires") {
          const ms = Date.parse(attrVal);
          if (!Number.isNaN(ms)) expiresAt = ms;
        } else if (attrName === "httponly") {
          httpOnly = true;
        } else if (attrName === "secure") {
          secure = true;
        }
      }

      if (maxAge !== null) {
        expiresAt = maxAge <= 0 ? 0 : Date.now() + maxAge * 1000;
      }

      const key = `${name}\0${cookiePath}`;
      if (expiresAt !== null && expiresAt <= Date.now()) {
        this.jar.delete(key);
        continue;
      }
      if (value === "" && maxAge !== null && maxAge <= 0) {
        this.jar.delete(key);
        continue;
      }

      this.jar.set(key, {
        name,
        value,
        path: cookiePath,
        expiresAt,
        httpOnly,
        secure,
      });
    }
  }

  _findCookie(name, reqPath = null) {
    const now = Date.now();
    const normReqPath = reqPath !== null ? normalizePath(reqPath) : null;
    const matches = [];
    for (const [key, c] of this.jar.entries()) {
      if (c.expiresAt !== null && c.expiresAt <= now) {
        this.jar.delete(key);
        continue;
      }
      if (c.name !== name) continue;
      if (normReqPath !== null && !pathMatches(normReqPath, c.path)) continue;
      matches.push(c);
    }
    if (matches.length === 0) return null;
    matches.sort((a, b) => b.path.length - a.path.length);
    return matches[0];
  }

  getCookie(name, reqPath = null) {
    const c = this._findCookie(name, reqPath);
    return c ? safeDecodeURIComponent(c.value) : null;
  }

  getRawCookie(name, reqPath = null) {
    const c = this._findCookie(name, reqPath);
    return c ? c.value : null;
  }

  setCookie(name, value, path = "/") {
    const cookiePath = normalizePath(path);
    this.jar.set(`${name}\0${cookiePath}`, {
      name,
      value: String(value),
      path: cookiePath,
      expiresAt: null,
      httpOnly: false,
      secure: false,
    });
  }

  deleteCookie(name, path = null) {
    for (const [key, c] of this.jar.entries()) {
      if (c.name === name && (path === null || c.path === normalizePath(path))) {
        this.jar.delete(key);
      }
    }
  }

  clearCookies() {
    this.jar.clear();
  }

  buildCookieHeader(reqPath = "/") {
    const now = Date.now();
    const matched = [];
    for (const [key, c] of this.jar.entries()) {
      if (c.expiresAt !== null && c.expiresAt <= now) {
        this.jar.delete(key);
        continue;
      }
      if (pathMatches(reqPath, c.path)) {
        matched.push(c);
      }
    }
    matched.sort((a, b) => b.path.length - a.path.length);
    return matched.map((c) => `${c.name}=${c.value}`).join("; ");
  }

  /**
   * Export cookies in Playwright `context.addCookies(...)` format.
   */
  exportCookies(baseUrl = this.baseUrl) {
    const origin = new URL(baseUrl);
    const now = Date.now();
    const out = [];
    for (const [key, c] of this.jar.entries()) {
      if (c.expiresAt !== null && c.expiresAt <= now) {
        this.jar.delete(key);
        continue;
      }
      out.push({
        name: c.name,
        value: c.value,
        domain: origin.hostname,
        path: c.path,
        httpOnly: c.httpOnly,
        secure: c.secure,
      });
    }
    return out;
  }

  /**
   * Ensure a valid `csrf_token` cookie is in the jar by hitting `/api/v1/health` if absent.
   */
  async ensureCsrf() {
    if (!this.getRawCookie("csrf_token")) {
      await this.get("/api/v1/health");
    }
    return this.getCookie("csrf_token");
  }

  _resolveUrl(pathOrUrl, query = null) {
    const url = /^https?:\/\//i.test(pathOrUrl)
      ? new URL(pathOrUrl)
      : new URL(pathOrUrl.startsWith("/") ? pathOrUrl : `/${pathOrUrl}`, this.baseUrl);
    if (query && typeof query === "object") {
      for (const [k, v] of Object.entries(query)) {
        if (v === undefined || v === null) continue;
        if (Array.isArray(v)) {
          for (const item of v) url.searchParams.append(k, String(item));
        } else {
          url.searchParams.set(k, String(v));
        }
      }
    }
    return url;
  }

  async _prepareBodyAndHeaders(options, method) {
    const headers = new Headers();
    for (const [k, v] of Object.entries(this.defaultHeaders)) {
      if (v !== undefined && v !== null) headers.set(k, String(v));
    }
    if (options.headers) {
      const entries =
        options.headers instanceof Headers
          ? options.headers.entries()
          : Object.entries(options.headers);
      for (const [k, v] of entries) {
        if (v === undefined || v === null) {
          headers.delete(k);
        } else {
          headers.set(k, String(v));
        }
      }
    }

    if (options.host !== undefined) {
      if (options.host) {
        headers.set("host", String(options.host));
      } else {
        headers.delete("host");
      }
    } else if (!headers.has("host") && this.defaultHost) {
      headers.set("host", String(this.defaultHost));
    }
    const hostOverride = headers.get("host") || null;

    let body = undefined;
    if (options.json !== undefined) {
      body = JSON.stringify(options.json);
      if (!headers.has("content-type")) {
        headers.set("content-type", "application/json");
      }
    } else if (options.form !== undefined) {
      const params =
        options.form instanceof URLSearchParams
          ? options.form
          : new URLSearchParams(
              Object.entries(options.form).flatMap(([k, v]) =>
                v === undefined || v === null
                  ? []
                  : Array.isArray(v)
                    ? v.map((x) => [k, String(x)])
                    : [[k, String(v)]]
              )
            );
      body = params.toString();
      if (!headers.has("content-type")) {
        headers.set("content-type", "application/x-www-form-urlencoded");
      }
    } else if (options.multipart !== undefined) {
      if (options.multipart instanceof FormData) {
        body = options.multipart;
      } else {
        const fd = new FormData();
        for (const [k, v] of Object.entries(options.multipart)) {
          if (v === undefined || v === null) continue;
          if (v instanceof Blob) {
            fd.append(k, v, v.name || "upload.bin");
          } else if (
            typeof v === "object" &&
            v !== null &&
            (v.buffer instanceof ArrayBuffer || Buffer.isBuffer(v.content) || typeof v.content === "string")
          ) {
            const blob = new Blob([v.content ?? v.buffer], {
              type: v.contentType || "application/octet-stream",
            });
            fd.append(k, blob, v.filename || "upload.bin");
          } else {
            fd.append(k, String(v));
          }
        }
        body = fd;
      }
      // Let fetch/Response generate multipart boundary
      headers.delete("content-type");
    } else if (options.body !== undefined) {
      body = options.body;
    }

    if (
      MUTATING_METHODS.has(method) &&
      this.autoCsrf &&
      options.csrf !== false &&
      !headers.has("x-csrf-token")
    ) {
      const csrfVal = this.getCookie("csrf_token");
      if (csrfVal) {
        headers.set("x-csrf-token", csrfVal);
      }
    }

    return { headers, body, hostOverride };
  }

  async _sendOnceFetch(url, method, headers, body, timeoutMs) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(new Error(`Request timeout (${timeoutMs}ms): ${method} ${url}`)), timeoutMs);
    try {
      const resp = await fetch(url.toString(), {
        method,
        headers,
        body,
        redirect: "manual",
        signal: controller.signal,
      });
      const setCookies =
        typeof resp.headers.getSetCookie === "function"
          ? resp.headers.getSetCookie()
          : splitCombinedSetCookie(resp.headers.get("set-cookie"));
      const rawBuf = Buffer.from(await resp.arrayBuffer());
      return {
        status: resp.status,
        headers: resp.headers,
        setCookies,
        rawBuf,
        url: url.toString(),
      };
    } finally {
      clearTimeout(timer);
    }
  }

  async _sendOnceNodeHttp(url, method, headers, body, timeoutMs) {
    // Convert FormData or Web Body to Buffer + exact content-type header when sending via node:http
    let payloadBuf = null;
    if (body instanceof FormData) {
      const encoded = new Response(body);
      const ct = encoded.headers.get("content-type");
      if (ct) headers.set("content-type", ct);
      payloadBuf = Buffer.from(await encoded.arrayBuffer());
    } else if (body instanceof Uint8Array || Buffer.isBuffer(body)) {
      payloadBuf = Buffer.from(body);
    } else if (typeof body === "string") {
      payloadBuf = Buffer.from(body, "utf8");
    } else if (body !== undefined && body !== null) {
      const encoded = new Response(body);
      const ct = encoded.headers.get("content-type");
      if (ct && !headers.has("content-type")) headers.set("content-type", ct);
      payloadBuf = Buffer.from(await encoded.arrayBuffer());
    }

    const reqHeaders = {};
    for (const [k, v] of headers.entries()) {
      reqHeaders[k] = v;
    }
    if (payloadBuf !== null && !("content-length" in reqHeaders)) {
      reqHeaders["content-length"] = String(payloadBuf.byteLength);
    }

    const transport = url.protocol === "https:" ? https : http;
    return await new Promise((resolve, reject) => {
      const req = transport.request(
        {
          protocol: url.protocol,
          hostname: url.hostname,
          port: url.port || (url.protocol === "https:" ? 443 : 80),
          path: `${url.pathname}${url.search}`,
          method,
          headers: reqHeaders,
        },
        (res) => {
          const chunks = [];
          res.on("data", (c) => chunks.push(c));
          res.on("end", () => {
            const rawBuf = Buffer.concat(chunks);
            const webHeaders = new Headers();
            const setCookies = [];
            for (const [k, v] of Object.entries(res.headers)) {
              if (v === undefined) continue;
              if (k.toLowerCase() === "set-cookie") {
                const arr = Array.isArray(v) ? v : [v];
                for (const item of arr) {
                  setCookies.push(item);
                  webHeaders.append("set-cookie", item);
                }
              } else {
                webHeaders.set(k, Array.isArray(v) ? v.join(", ") : String(v));
              }
            }
            resolve({
              status: res.statusCode || 0,
              headers: webHeaders,
              setCookies,
              rawBuf,
              url: url.toString(),
            });
          });
        }
      );

      req.setTimeout(timeoutMs, () => {
        req.destroy(new Error(`Request timeout (${timeoutMs}ms): ${method} ${url}`));
      });
      req.on("error", reject);
      if (payloadBuf !== null) {
        req.write(payloadBuf);
      }
      req.end();
    });
  }

  async request(method, pathOrUrl, options = {}) {
    const upperMethod = method.toUpperCase();
    const redirectMode = options.redirect || "follow";
    const maxRedirects = options.maxRedirects ?? 10;
    const timeoutMs = options.timeoutMs ?? this.timeoutMs;

    let currentUrl = this._resolveUrl(pathOrUrl, options.query);
    let currentMethod = upperMethod;
    let currentOpts = { ...options };
    let redirected = false;

    for (let hop = 0; hop <= maxRedirects; hop++) {
      const { headers, body, hostOverride } = await this._prepareBodyAndHeaders(
        currentOpts,
        currentMethod
      );

      if (options.cookies !== false) {
        const cookieHdr = this.buildCookieHeader(currentUrl.pathname);
        if (cookieHdr) {
          const existing = headers.get("cookie");
          headers.set("cookie", existing ? `${existing}; ${cookieHdr}` : cookieHdr);
        }
      }

      // Node.js v22 fetch() ignores/drops custom Host headers; use node:http when Host differs from url.host
      const useNodeHttp = Boolean(hostOverride && hostOverride !== currentUrl.host);
      const rawRes = useNodeHttp
        ? await this._sendOnceNodeHttp(currentUrl, currentMethod, headers, body, timeoutMs)
        : await this._sendOnceFetch(currentUrl, currentMethod, headers, body, timeoutMs);

      if (options.cookies !== false && rawRes.setCookies.length > 0) {
        this._parseAndStoreSetCookies(rawRes.setCookies, currentUrl.pathname);
      }

      const location = rawRes.headers.get("location");
      if (redirectMode === "follow" && REDIRECT_STATUSES.has(rawRes.status) && location) {
        if (hop === maxRedirects) {
          throw new Error(`Exceeded max redirects (${maxRedirects}) at ${currentUrl}`);
        }
        redirected = true;
        const nextUrl = new URL(location, currentUrl);
        const baseParsed = new URL(this.baseUrl);
        const nextHostLower = nextUrl.hostname.toLowerCase();
        const activeHostLower = hostOverride ? hostOverride.split(":")[0].toLowerCase() : null;
        const isBaseHost =
          nextHostLower === baseParsed.hostname.toLowerCase() ||
          nextHostLower === "127.0.0.1" ||
          nextHostLower === "localhost";
        const matchesAlias = this.domainAliases.some(
          (alias) => nextHostLower === alias || nextHostLower.endsWith(`.${alias}`)
        );
        const matchesActiveHost = Boolean(activeHostLower && nextHostLower === activeHostLower);

        if (isBaseHost) {
          if (nextUrl.port && nextUrl.port !== baseParsed.port) {
            throw new Error(
              `Refusing to follow redirect to different local port ${nextUrl.toString()} from ${this.baseUrl}`
            );
          }
          currentUrl = nextUrl;
        } else if (matchesActiveHost || matchesAlias) {
          currentOpts.host = nextUrl.hostname;
          nextUrl.protocol = baseParsed.protocol;
          nextUrl.hostname = baseParsed.hostname;
          nextUrl.port = baseParsed.port;
          currentUrl = nextUrl;
        } else if (options.allowExternalRedirects === true) {
          currentUrl = nextUrl;
        } else {
          throw new Error(
            `Refusing to follow external redirect to ${nextUrl.toString()} from isolated test client`
          );
        }

        if (rawRes.status === 303 || ((rawRes.status === 301 || rawRes.status === 302) && currentMethod === "POST")) {
          currentMethod = "GET";
          delete currentOpts.json;
          delete currentOpts.form;
          delete currentOpts.multipart;
          delete currentOpts.body;
        }
        continue;
      }

      const ct = (rawRes.headers.get("content-type") || "").toLowerCase();
      const isBinary =
        options.binary === true ||
        ct.startsWith("application/pdf") ||
        ct.startsWith("image/") ||
        ct.startsWith("application/octet-stream");
      const text = isBinary ? "" : rawRes.rawBuf.toString("utf8");
      let json = null;
      if (!isBinary && text.length > 0 && (ct.includes("json") || text.trimStart().startsWith("{") || text.trimStart().startsWith("["))) {
        try {
          json = JSON.parse(text);
        } catch {
          json = null;
        }
      }

      return {
        status: rawRes.status,
        ok: rawRes.status >= 200 && rawRes.status < 300,
        headers: rawRes.headers,
        url: rawRes.url,
        redirected,
        rawBody: rawRes.rawBuf,
        text,
        json,
        data: json && typeof json === "object" && "data" in json ? json.data : json,
        cookies: rawRes.setCookies,
      };
    }

    throw new Error("Unreachable redirect loop end");
  }

  get(path, opts = {}) {
    return this.request("GET", path, opts);
  }

  post(path, opts = {}) {
    return this.request("POST", path, opts);
  }

  put(path, opts = {}) {
    return this.request("PUT", path, opts);
  }

  patch(path, opts = {}) {
    return this.request("PATCH", path, opts);
  }

  delete(path, opts = {}) {
    return this.request("DELETE", path, opts);
  }

  raw(method, path, opts = {}) {
    return this.request(method, path, { binary: true, ...opts });
  }
}

export function createHttpFactory({ baseUrl, defaultHost = null } = {}) {
  return {
    baseUrl,
    Client,
    createClient(opts = {}) {
      return new Client({ baseUrl, defaultHost, ...opts });
    },
  };
}
