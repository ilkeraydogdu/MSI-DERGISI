/**
 * Opaque-box E2E assertion helpers with descriptive failure diagnostics.
 */

export class AssertionFailure extends Error {
  constructor(message, details = undefined) {
    super(message);
    this.name = "AssertionFailure";
    this.details = details;
  }
}

function formatVal(val, maxLen = 240) {
  let str;
  if (typeof val === "string") {
    str = val;
  } else {
    try {
      str = JSON.stringify(val);
    } catch {
      str = String(val);
    }
  }
  if (str === undefined) return "undefined";
  if (str.length > maxLen) {
    return str.slice(0, maxLen) + `... (${str.length} chars)`;
  }
  return str;
}

function isDeepEqual(a, b) {
  if (Object.is(a, b)) return true;
  if (a === null || b === null || typeof a !== "object" || typeof b !== "object") {
    return false;
  }
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a)) {
    if (a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!isDeepEqual(a[i], b[i])) return false;
    }
    return true;
  }
  const keysA = Object.keys(a);
  const keysB = Object.keys(b);
  if (keysA.length !== keysB.length) return false;
  for (const k of keysA) {
    if (!Object.prototype.hasOwnProperty.call(b, k)) return false;
    if (!isDeepEqual(a[k], b[k])) return false;
  }
  return true;
}

export function assertOk(condition, message = "Expected condition to be truthy") {
  if (!condition) {
    throw new AssertionFailure(message);
  }
}

export function assertEqual(actual, expected, message = "Values are not strictly equal") {
  if (!Object.is(actual, expected)) {
    throw new AssertionFailure(
      `${message}: expected ${formatVal(expected)}, got ${formatVal(actual)}`,
      { actual, expected }
    );
  }
}

export function assertNotEqual(actual, unexpected, message = "Values should not be equal") {
  if (Object.is(actual, unexpected)) {
    throw new AssertionFailure(
      `${message}: expected value different from ${formatVal(unexpected)}`,
      { actual, unexpected }
    );
  }
}

export function assertDeepEqual(actual, expected, message = "Objects are not deeply equal") {
  if (!isDeepEqual(actual, expected)) {
    throw new AssertionFailure(
      `${message}: expected ${formatVal(expected, 400)}, got ${formatVal(actual, 400)}`,
      { actual, expected }
    );
  }
}

export function assertStatus(res, expectedStatus, message = "Unexpected HTTP status") {
  if (!res || typeof res.status !== "number") {
    throw new AssertionFailure(`${message}: invalid HTTP response object`);
  }
  const allowed = Array.isArray(expectedStatus) ? expectedStatus : [expectedStatus];
  if (!allowed.includes(res.status)) {
    const bodyPreview = formatVal(res.text ?? res.json ?? "", 320);
    throw new AssertionFailure(
      `${message}: expected status ${allowed.join(" or ")}, got ${res.status} (${res.url || "?"}) — body: ${bodyPreview}`,
      { status: res.status, expected: allowed, url: res.url, body: bodyPreview }
    );
  }
}

export function assertJson(res, predicateOrExpected = null, message = "Unexpected JSON response") {
  if (!res || res.json === null || res.json === undefined || typeof res.json !== "object") {
    throw new AssertionFailure(
      `${message}: response body is not JSON (status=${res?.status}, text=${formatVal(res?.text, 200)})`
    );
  }
  if (typeof predicateOrExpected === "function") {
    const ok = predicateOrExpected(res.json);
    if (!ok) {
      throw new AssertionFailure(
        `${message}: JSON predicate failed for ${formatVal(res.json, 350)}`,
        { json: res.json }
      );
    }
  } else if (predicateOrExpected && typeof predicateOrExpected === "object") {
    for (const [k, v] of Object.entries(predicateOrExpected)) {
      if (!isDeepEqual(res.json[k], v)) {
        throw new AssertionFailure(
          `${message}: key "${k}" expected ${formatVal(v)}, got ${formatVal(res.json[k])}`,
          { key: k, expected: v, actual: res.json[k], json: res.json }
        );
      }
    }
  }
  return res.json;
}

export function assertIncludes(haystack, needle, message = "Expected collection/string to include needle") {
  if (typeof haystack === "string") {
    if (!haystack.includes(String(needle))) {
      throw new AssertionFailure(
        `${message}: missing ${formatVal(needle)} in ${formatVal(haystack, 240)}`
      );
    }
    return;
  }
  if (Array.isArray(haystack)) {
    if (!haystack.includes(needle)) {
      throw new AssertionFailure(
        `${message}: array ${formatVal(haystack, 240)} does not include ${formatVal(needle)}`
      );
    }
    return;
  }
  throw new AssertionFailure(`${message}: haystack is neither string nor array (${typeof haystack})`);
}

export function assertNotIncludes(haystack, needle, message = "Expected collection/string not to include needle") {
  if (typeof haystack === "string") {
    if (haystack.includes(String(needle))) {
      throw new AssertionFailure(
        `${message}: unexpectedly found ${formatVal(needle)} in ${formatVal(haystack, 240)}`
      );
    }
    return;
  }
  if (Array.isArray(haystack)) {
    if (haystack.includes(needle)) {
      throw new AssertionFailure(
        `${message}: array unexpectedly includes ${formatVal(needle)}`
      );
    }
    return;
  }
  throw new AssertionFailure(`${message}: haystack is neither string nor array (${typeof haystack})`);
}

export function assertMatch(text, regex, message = "String did not match pattern") {
  if (typeof text !== "string" || !regex.test(text)) {
    throw new AssertionFailure(
      `${message}: ${formatVal(text, 240)} did not match ${String(regex)}`
    );
  }
}

export function assertNoOverflow(metricsOrPageResult, message = "Horizontal overflow detected") {
  const m = metricsOrPageResult?.overflow ?? metricsOrPageResult;
  if (!m || typeof m.scrollWidth !== "number" || typeof m.innerWidth !== "number") {
    throw new AssertionFailure(`${message}: invalid overflow metrics object`);
  }
  if (m.scrollWidth > m.innerWidth) {
    throw new AssertionFailure(
      `${message}: documentElement.scrollWidth (${m.scrollWidth}) > window.innerWidth (${m.innerWidth})`,
      m
    );
  }
}

export function assertNoBrowserErrors(pageResult, message = "Browser runtime errors detected") {
  if (!pageResult || typeof pageResult !== "object") {
    throw new AssertionFailure(`${message}: invalid pageResult`);
  }
  const consoleErrors = pageResult.consoleErrors || [];
  const pageErrors = pageResult.pageErrors || [];
  const serverErrors = pageResult.serverErrors || [];

  if (consoleErrors.length > 0 || pageErrors.length > 0 || serverErrors.length > 0) {
    const parts = [];
    if (consoleErrors.length > 0) parts.push(`consoleErrors=[${consoleErrors.join(" | ")}]`);
    if (pageErrors.length > 0) parts.push(`pageErrors=[${pageErrors.join(" | ")}]`);
    if (serverErrors.length > 0) {
      parts.push(
        `serverErrors=[${serverErrors.map((e) => `${e.status} ${e.url}`).join(" | ")}]`
      );
    }
    throw new AssertionFailure(`${message}: ${parts.join("; ")}`, {
      consoleErrors,
      pageErrors,
      serverErrors,
    });
  }
}

function matchesAnyPattern(value, url, patterns) {
  if (!Array.isArray(patterns) || patterns.length === 0) return false;
  return patterns.some((pat) => {
    if (typeof pat === "function") {
      return Boolean(pat(value, url));
    }
    if (pat instanceof RegExp) {
      return pat.test(String(value || "")) || pat.test(String(url || ""));
    }
    const needle = String(pat);
    return String(value || "").includes(needle) || String(url || "").includes(needle);
  });
}

export function assertNoAssetOrNetworkErrors(
  pageResult,
  allowedPatterns = [],
  message = "Browser asset or network errors detected"
) {
  if (!pageResult || typeof pageResult !== "object") {
    throw new AssertionFailure(`${message}: invalid pageResult`);
  }

  const consoleErrors = (pageResult.consoleErrors || []).filter(
    (line) => !matchesAnyPattern(line, line, allowedPatterns)
  );
  const pageErrors = (pageResult.pageErrors || []).filter(
    (err) => !matchesAnyPattern(err, err, allowedPatterns)
  );
  const serverErrors = (pageResult.serverErrors || []).filter(
    (e) => !matchesAnyPattern(`${e.status} ${e.url}`, e.url, allowedPatterns)
  );
  const clientErrors = (pageResult.clientErrors || []).filter(
    (e) => !matchesAnyPattern(`${e.status} ${e.url}`, e.url, allowedPatterns)
  );
  const failedRequests = (pageResult.failedRequests || []).filter(
    (e) => !matchesAnyPattern(`${e.failure} ${e.url}`, e.url, allowedPatterns)
  );

  if (
    consoleErrors.length > 0 ||
    pageErrors.length > 0 ||
    serverErrors.length > 0 ||
    clientErrors.length > 0 ||
    failedRequests.length > 0
  ) {
    const parts = [];
    if (consoleErrors.length > 0) parts.push(`consoleErrors=[${consoleErrors.join(" | ")}]`);
    if (pageErrors.length > 0) parts.push(`pageErrors=[${pageErrors.join(" | ")}]`);
    if (serverErrors.length > 0) {
      parts.push(`serverErrors=[${serverErrors.map((e) => `${e.status} ${e.url}`).join(" | ")}]`);
    }
    if (clientErrors.length > 0) {
      parts.push(
        `clientErrors=[${clientErrors.map((e) => `${e.status} (${e.resourceType}) ${e.url}`).join(" | ")}]`
      );
    }
    if (failedRequests.length > 0) {
      parts.push(
        `failedRequests=[${failedRequests.map((e) => `${e.failure} (${e.resourceType}) ${e.url}`).join(" | ")}]`
      );
    }
    throw new AssertionFailure(`${message}: ${parts.join("; ")}`, {
      consoleErrors,
      pageErrors,
      serverErrors,
      clientErrors,
      failedRequests,
    });
  }
}

export async function assertThrows(fn, expectedErrorOrMatch = null, message = "Expected function to throw") {
  let caught = null;
  try {
    await fn();
  } catch (err) {
    caught = err;
  }
  if (!caught) {
    throw new AssertionFailure(message);
  }
  if (expectedErrorOrMatch instanceof RegExp) {
    if (!expectedErrorOrMatch.test(caught.message || String(caught))) {
      throw new AssertionFailure(
        `${message}: thrown error "${caught.message}" did not match ${String(expectedErrorOrMatch)}`
      );
    }
  } else if (typeof expectedErrorOrMatch === "string") {
    if (!(caught.message || String(caught)).includes(expectedErrorOrMatch)) {
      throw new AssertionFailure(
        `${message}: thrown error "${caught.message}" did not include "${expectedErrorOrMatch}"`
      );
    }
  }
  return caught;
}
