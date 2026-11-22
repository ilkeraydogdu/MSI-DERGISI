#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import * as assertions from "./lib/assertions.mjs";
import { createAuthHelpers } from "./lib/auth.mjs";
import { createBrowserManager } from "./lib/browser.mjs";
import { createHttpFactory } from "./lib/http.mjs";
import { DEFAULT_REPO_ROOT, startIsolatedServer } from "./lib/server.mjs";
import * as totp from "./lib/totp.mjs";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const SUITES_DIR = path.join(__dirname, "suites");

function parseArgs(argv) {
  const opts = {
    tiers: null,
    suites: [],
    filter: null,
    json: false,
    keepTemp: false,
    build: false,
    perSuite: false,
    list: false,
    baseUrl: process.env.BASE_URL ? process.env.BASE_URL.trim() : null,
    adminPath: process.env.ADMIN_ROOT !== undefined ? process.env.ADMIN_ROOT : undefined,
    screenshotDir: null,
    allowMutating: false,
    help: false,
  };

  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      opts.help = true;
    } else if (arg === "--json") {
      opts.json = true;
    } else if (arg === "--keep-temp") {
      opts.keepTemp = true;
    } else if (arg === "--build") {
      opts.build = true;
    } else if (arg === "--per-suite") {
      opts.perSuite = true;
    } else if (arg === "--list" || arg === "--dry-run") {
      opts.list = true;
    } else if (arg === "--allow-mutating") {
      opts.allowMutating = true;
    } else if (arg.startsWith("--tier=")) {
      opts.tiers = arg.slice("--tier=".length).split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
    } else if (arg === "--tier" && argv[i + 1]) {
      opts.tiers = argv[++i].split(",").map((s) => s.trim().toLowerCase()).filter(Boolean);
    } else if (arg.startsWith("--suite=")) {
      opts.suites.push(...arg.slice("--suite=".length).split(",").map((s) => s.trim()).filter(Boolean));
    } else if (arg === "--suite" && argv[i + 1]) {
      opts.suites.push(...argv[++i].split(",").map((s) => s.trim()).filter(Boolean));
    } else if (arg.startsWith("--filter=")) {
      opts.filter = arg.slice("--filter=".length);
    } else if (arg === "--filter" && argv[i + 1]) {
      opts.filter = argv[++i];
    } else if (arg.startsWith("--base-url=")) {
      opts.baseUrl = arg.slice("--base-url=".length).trim();
    } else if (arg === "--base-url" && argv[i + 1]) {
      opts.baseUrl = argv[++i].trim();
    } else if (arg.startsWith("--admin-path=")) {
      opts.adminPath = arg.slice("--admin-path=".length);
    } else if (arg === "--admin-path" && argv[i + 1] !== undefined) {
      opts.adminPath = argv[++i];
    } else if (arg.startsWith("--screenshot-dir=")) {
      opts.screenshotDir = arg.slice("--screenshot-dir=".length).trim();
    } else if (arg === "--screenshot-dir" && argv[i + 1]) {
      opts.screenshotDir = argv[++i].trim();
    } else {
      throw new Error(`Unknown CLI argument: ${arg}`);
    }
  }

  return opts;
}

function printHelp() {
  console.log(`MSI Platform Isolated E2E Runner

Usage:
  node tests/e2e/run.mjs [options]

Options:
  --tier=1|2|3|4|all|smoke   Run tests for specified tier(s) (default: all)
  --suite=<name>             Run matching suite file(s) under tests/e2e/suites/
  --filter=<pattern>         Filter tests by name or REQ id (substring or /regex/)
  --list                     List matching tests & counts without starting server
  --base-url=<url>           Run against external target URL (skips isolated server;
                             skips mutating tests unless --allow-mutating is passed)
  --allow-mutating           Allow mutating tests when --base-url is set
  --admin-path=<slug>        Override ADMIN_PATH_SECRET / adminPath (supports "" for fallback test)
  --screenshot-dir=<dir>     Directory for Playwright screenshot artifacts
  --json                     Output machine-readable JSON report to stdout
  --keep-temp                Preserve /tmp/msi_e2e_* temp workspace after exit
  --build                    Force cargo build --bin msi-dergisi-platform before run
  --per-suite                Start a fresh isolated server per suite file
  -h, --help                 Show this help message
`);
}

function discoverSuiteFiles(dir) {
  if (!fs.existsSync(dir)) return [];
  const entries = fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name));
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...discoverSuiteFiles(full));
    } else if (entry.isFile() && entry.name.endsWith(".test.mjs")) {
      files.push(full);
    }
  }
  return files;
}

function inferTierFromRelPath(relPath) {
  const normalized = relPath.replace(/\\/g, "/");
  if (normalized === "smoke.test.mjs" || normalized.startsWith("smoke/")) return "smoke";
  const m = normalized.match(/^tier([1-4])\//);
  if (m) return Number.parseInt(m[1], 10);
  return "other";
}

function suiteMatchesFilter(relPath, suitePatterns) {
  if (!suitePatterns || suitePatterns.length === 0) return true;
  const normalized = relPath.replace(/\\/g, "/").toLowerCase();
  const base = path.basename(normalized, ".test.mjs");
  const withoutExt = normalized.replace(/\.test\.mjs$/, "");
  return suitePatterns.some((pat) => {
    const p = pat.toLowerCase();
    return base === p || withoutExt === p || normalized.includes(p);
  });
}

function tierMatches(tierValue, requestedTiers) {
  if (!requestedTiers || requestedTiers.includes("all")) {
    return true;
  }
  const tStr = String(tierValue).toLowerCase();
  return requestedTiers.includes(tStr);
}

function buildTestMatcher(filterStr) {
  if (!filterStr) return () => true;
  const regexMatch = filterStr.match(/^\/(.+)\/([gimsuy]*)$/);
  if (regexMatch) {
    const safeFlags = regexMatch[2].replace(/g/g, "");
    const rx = new RegExp(regexMatch[1], safeFlags);
    return (t) => rx.test(t.name) || rx.test(t.suite) || t.reqs.some((r) => rx.test(r));
  }
  const needle = filterStr.toLowerCase();
  return (t) =>
    t.name.toLowerCase().includes(needle) ||
    t.suite.toLowerCase().includes(needle) ||
    t.reqs.some((r) => r.toLowerCase().includes(needle));
}

function buildContext(server, browserMgr, opts = {}) {
  const ctx = {
    baseUrl: server.baseUrl,
    port: server.port,
    adminPath: server.adminPath,
    isExternalTarget: Boolean(server.isExternalTarget),
    allowMutating: Boolean(opts.allowMutating),
    screenshotDir: opts.screenshotDir || null,
    repoRoot: server.repoRoot,
    tempDir: server.tempDir,
    dbPath: server.dbPath,
    edergiDir: server.edergiDir,
    backupDir: server.backupDir,
    uploadsDir: server.uploadsDir,
    logPath: server.logPath,
    credentials: server.credentials,
    http: createHttpFactory({ baseUrl: server.baseUrl }),
    browser: browserMgr,
    assert: assertions,
    totp,
    readServerLogTail: (lines = 80) =>
      typeof server.readLogTail === "function" ? server.readLogTail(lines) : "(external target: no local server log)",
  };
  ctx.auth = createAuthHelpers(ctx);
  return ctx;
}

function buildExternalTargetDescriptor(opts) {
  const normalizedBaseUrl = String(opts.baseUrl || "").replace(/\/+$/, "");
  const parsed = new URL(normalizedBaseUrl);
  const rawAdmin = opts.adminPath !== undefined ? String(opts.adminPath).trim().replace(/^\/+|\/+$/g, "") : "podmin";
  return {
    baseUrl: normalizedBaseUrl,
    port: Number(parsed.port) || (parsed.protocol === "https:" ? 443 : 80),
    adminPath: rawAdmin || "podmin",
    isExternalTarget: true,
    repoRoot: DEFAULT_REPO_ROOT,
    tempDir: null,
    dbPath: null,
    edergiDir: null,
    backupDir: null,
    uploadsDir: null,
    logPath: null,
    credentials: {
      admin: {
        email: process.env.E2E_ADMIN_EMAIL || "",
        password: process.env.E2E_ADMIN_PASSWORD || "",
        role: "site_operator",
      },
      superAdmin: {
        email: process.env.E2E_SUPERADMIN_EMAIL || process.env.E2E_ADMIN_EMAIL || "",
        password: process.env.E2E_SUPERADMIN_PASSWORD || process.env.E2E_ADMIN_PASSWORD || "",
        role: "super_admin",
      },
    },
    readLogTail: () => "(external target: no local server log)",
    stop: async () => {},
  };
}

function createSuiteCollector(relPath, defaultTier) {
  const tests = [];
  const hooks = {
    beforeAll: [],
    afterAll: [],
    beforeEach: [],
    afterEach: [],
  };
  const prefixStack = [];

  const suite = {
    name: relPath.replace(/\.test\.mjs$/, ""),
    tier: defaultTier,
    describe(groupName, fn) {
      prefixStack.push(groupName);
      try {
        fn();
      } finally {
        prefixStack.pop();
      }
    },
    test(name, optsOrFn, maybeFn) {
      let opts = {};
      let fn = optsOrFn;
      if (typeof optsOrFn === "object" && optsOrFn !== null) {
        opts = optsOrFn;
        fn = maybeFn;
      }
      if (typeof fn !== "function") {
        throw new Error(`suite.test("${name}") requires a test function`);
      }

      const fullName = prefixStack.length > 0 ? `${prefixStack.join(" > ")} > ${name}` : name;
      const rawReqs = opts.reqs ?? opts.req ?? [];
      const reqs = Array.isArray(rawReqs) ? rawReqs : rawReqs ? [rawReqs] : [];
      const tier = opts.tier ?? defaultTier;

      tests.push({
        suite: suite.name,
        name: fullName,
        tier,
        reqs,
        tags: opts.tags || [],
        readOnly: opts.readOnly === true,
        mutating: opts.mutating !== undefined ? Boolean(opts.mutating) : undefined,
        fn,
      });
    },
    beforeAll(fn) {
      hooks.beforeAll.push(fn);
    },
    afterAll(fn) {
      hooks.afterAll.push(fn);
    },
    beforeEach(fn) {
      hooks.beforeEach.push(fn);
    },
    afterEach(fn) {
      hooks.afterEach.push(fn);
    },
  };

  return { suite, tests, hooks };
}

function recordTestResult(report, resultEntry, opts, errStack = null) {
  report.tests.push(resultEntry);

  const tierKey = String(resultEntry.tier);
  if (!report.byTier[tierKey]) {
    report.byTier[tierKey] = { total: 0, passed: 0, failed: 0, skipped: 0, durationMs: 0 };
  }
  report.byTier[tierKey].total += 1;
  report.byTier[tierKey].durationMs += resultEntry.durationMs;
  if (resultEntry.skipped) report.byTier[tierKey].skipped += 1;
  else if (resultEntry.passed) report.byTier[tierKey].passed += 1;
  else report.byTier[tierKey].failed += 1;

  for (const reqId of resultEntry.reqs || []) {
    if (!report.byReq[reqId]) {
      report.byReq[reqId] = { total: 0, passed: 0, failed: 0, skipped: 0, durationMs: 0 };
    }
    report.byReq[reqId].total += 1;
    report.byReq[reqId].durationMs += resultEntry.durationMs;
    if (resultEntry.skipped) report.byReq[reqId].skipped += 1;
    else if (resultEntry.passed) report.byReq[reqId].passed += 1;
    else report.byReq[reqId].failed += 1;
  }

  if (resultEntry.skipped) {
    report.summary.skipped = (report.summary.skipped || 0) + 1;
    if (!opts.json) {
      console.log(`  SKIP  [${tierKey}] ${resultEntry.suite} :: ${resultEntry.name} (${resultEntry.skipReason || "skipped"})`);
    }
  } else if (resultEntry.passed) {
    report.summary.passed += 1;
    if (!opts.json) {
      console.log(`  PASS  [${tierKey}] ${resultEntry.suite} :: ${resultEntry.name} (${resultEntry.durationMs}ms)`);
    }
  } else {
    report.summary.failed += 1;
    report.failures.push({ ...resultEntry, stack: errStack });
    if (!opts.json) {
      console.error(`  FAIL  [${tierKey}] ${resultEntry.suite} :: ${resultEntry.name} (${resultEntry.durationMs}ms)`);
      console.error(`        ${resultEntry.error}`);
    }
  }
  report.summary.total += 1;
}

async function runSuiteWithContext(fileInfo, ctx, testMatcher, opts, report, shouldStop = () => false) {
  const mod = await import(pathToFileURL(fileInfo.fullPath).href);
  if (typeof mod.registerTests !== "function") {
    throw new Error(`Suite ${fileInfo.relPath} does not export registerTests(suite, ctx)`);
  }

  const collector = createSuiteCollector(fileInfo.relPath, fileInfo.defaultTier);
  await mod.registerTests(collector.suite, ctx);

  const selectedTests = collector.tests.filter(
    (t) => tierMatches(t.tier, opts.tiers) && testMatcher(t)
  );
  if (selectedTests.length === 0) return;

  const isExternalProtected = Boolean(ctx.isExternalTarget && !opts.allowMutating);
  const runnableTests = isExternalProtected
    ? selectedTests.filter((t) => t.readOnly === true && t.mutating !== true)
    : selectedTests;

  if (isExternalProtected && runnableTests.length === 0) {
    for (const t of selectedTests) {
      recordTestResult(
        report,
        {
          suite: t.suite,
          name: t.name,
          tier: t.tier,
          reqs: t.reqs,
          passed: true,
          skipped: true,
          skipReason: "external target read-only guard (pass --allow-mutating to run)",
          durationMs: 0,
        },
        opts
      );
    }
    return;
  }

  try {
    let beforeAllError = null;
    const beforeT0 = Date.now();
    try {
      for (const fn of collector.hooks.beforeAll) {
        if (shouldStop()) return;
        await fn(ctx);
      }
    } catch (err) {
      beforeAllError = err;
    }

    if (beforeAllError) {
      recordTestResult(
        report,
        {
          suite: collector.suite.name,
          name: "<beforeAll>",
          tier: fileInfo.defaultTier,
          reqs: [],
          passed: false,
          durationMs: Date.now() - beforeT0,
          error: beforeAllError?.message || String(beforeAllError),
        },
        opts,
        beforeAllError?.stack || null
      );
      return;
    }

    for (const t of selectedTests) {
      if (shouldStop()) break;
      if (isExternalProtected && !(t.readOnly === true && t.mutating !== true)) {
        recordTestResult(
          report,
          {
            suite: t.suite,
            name: t.name,
            tier: t.tier,
            reqs: t.reqs,
            passed: true,
            skipped: true,
            skipReason: "external target read-only guard (pass --allow-mutating to run)",
            durationMs: 0,
          },
          opts
        );
        continue;
      }

      const t0 = Date.now();
      let passed = false;
      let errMsg = null;
      let errStack = null;

      try {
        for (const fn of collector.hooks.beforeEach) {
          await fn(ctx);
        }
        try {
          await t.fn(ctx);
          passed = true;
        } finally {
          for (const fn of collector.hooks.afterEach) {
            await fn(ctx);
          }
        }
      } catch (err) {
        passed = false;
        errMsg = err?.message || String(err);
        errStack = err?.stack || null;
      }

      if (shouldStop()) break;

      const durationMs = Date.now() - t0;
      const resultEntry = {
        suite: t.suite,
        name: t.name,
        tier: t.tier,
        reqs: t.reqs,
        passed,
        durationMs,
        ...(passed ? {} : { error: errMsg }),
      };
      recordTestResult(report, resultEntry, opts, errStack);
    }
  } finally {
    for (const fn of collector.hooks.afterAll) {
      try {
        await fn(ctx);
      } catch (err) {
        if (!shouldStop()) {
          recordTestResult(
            report,
            {
              suite: collector.suite.name,
              name: "<afterAll>",
              tier: fileInfo.defaultTier,
              reqs: [],
              passed: false,
              durationMs: 0,
              error: err?.message || String(err),
            },
            opts,
            err?.stack || null
          );
        }
      }
    }
  }
}

function writeJsonStdoutAndExit(payload, code) {
  const text = JSON.stringify(payload, null, 2) + "\n";
  // Drain/flush synchronously so large >64KB JSON payloads are never truncated on piped stdout
  try {
    let offset = 0;
    const buf = Buffer.from(text, "utf8");
    while (offset < buf.length) {
      try {
        offset += fs.writeSync(1, buf, offset, buf.length - offset);
      } catch (err) {
        if (err && (err.code === "EAGAIN" || err.code === "EWOULDBLOCK")) {
          continue;
        }
        throw err;
      }
    }
    process.exit(code);
  } catch {
    process.stdout.write(text, () => process.exit(code));
  }
}

function printHumanSummary(report, opts) {
  console.log("\n=== E2E Run Summary ===");
  const skippedPart = report.summary.skipped ? ` | Skipped: ${report.summary.skipped}` : "";
  console.log(
    `Total: ${report.summary.total} | Passed: ${report.summary.passed} | Failed: ${report.summary.failed}${skippedPart} | Duration: ${report.summary.durationMs}ms`
  );

  const tierKeys = Object.keys(report.byTier).sort();
  if (tierKeys.length > 0) {
    console.log("\nBy Tier:");
    for (const k of tierKeys) {
      const s = report.byTier[k];
      const tierSkipped = s.skipped ? `, ${s.skipped} skipped` : "";
      console.log(`  tier=${k}: ${s.passed}/${s.total} passed (${s.failed} failed${tierSkipped}, ${s.durationMs}ms)`);
    }
  }

  const reqKeys = Object.keys(report.byReq).sort();
  if (reqKeys.length > 0) {
    console.log("\nBy Requirement:");
    for (const k of reqKeys) {
      const s = report.byReq[k];
      console.log(`  ${k}: ${s.passed}/${s.total} passed (${s.failed} failed, ${s.durationMs}ms)`);
    }
  }

  if (opts.keepTemp && report.tempDirs.length > 0) {
    console.log(`\nPreserved temp dir(s): ${report.tempDirs.join(", ")}`);
  }
}

async function main() {
  const opts = parseArgs(process.argv.slice(2));
  if (opts.help) {
    printHelp();
    process.exit(0);
  }

  if (!opts.tiers && opts.suites.length === 0) {
    opts.tiers = ["all"];
  }

  const allFiles = discoverSuiteFiles(SUITES_DIR).map((fullPath) => {
    const relPath = path.relative(SUITES_DIR, fullPath);
    return {
      fullPath,
      relPath,
      defaultTier: inferTierFromRelPath(relPath),
    };
  });

  const matchingFiles = allFiles.filter(
    (f) => suiteMatchesFilter(f.relPath, opts.suites) && tierMatches(f.defaultTier, opts.tiers)
  );

  if (matchingFiles.length === 0) {
    const msg = `No E2E suite files matched (tier=${opts.tiers?.join(",") || "any"}, suite=${opts.suites.join(",") || "any"})`;
    if (opts.json) {
      console.log(JSON.stringify({ summary: { total: 0, passed: 0, failed: 1, error: msg } }, null, 2));
    } else {
      console.error(`ERROR: ${msg}`);
    }
    process.exit(1);
  }

  const testMatcher = buildTestMatcher(opts.filter);

  if (opts.list) {
    const dummyTarget = opts.baseUrl
      ? buildExternalTargetDescriptor(opts)
      : {
          baseUrl: "http://127.0.0.1:0",
          port: 0,
          adminPath:
            opts.adminPath !== undefined && String(opts.adminPath).trim() !== ""
              ? String(opts.adminPath).trim()
              : "podmin",
          isExternalTarget: false,
          repoRoot: DEFAULT_REPO_ROOT,
          tempDir: null,
          dbPath: null,
          edergiDir: null,
          backupDir: null,
          uploadsDir: null,
          logPath: null,
          credentials: {
            admin: { email: "", password: "", role: "site_operator" },
            superAdmin: { email: "", password: "", role: "super_admin" },
          },
          readLogTail: () => "",
        };
    const dummyCtx = buildContext(dummyTarget, { VIEWPORT_DESKTOP: { width: 1440, height: 900 }, VIEWPORT_MOBILE: { width: 390, height: 844 } }, opts);
    const listed = [];
    const byTier = {};
    for (const fileInfo of matchingFiles) {
      const mod = await import(pathToFileURL(fileInfo.fullPath).href);
      if (typeof mod.registerTests !== "function") {
        throw new Error(`Suite ${fileInfo.relPath} does not export registerTests(suite, ctx)`);
      }
      const collector = createSuiteCollector(fileInfo.relPath, fileInfo.defaultTier);
      await mod.registerTests(collector.suite, dummyCtx);
      const selected = collector.tests.filter(
        (t) => tierMatches(t.tier, opts.tiers) && testMatcher(t)
      );
      for (const t of selected) {
        const tierKey = String(t.tier);
        byTier[tierKey] = (byTier[tierKey] || 0) + 1;
        listed.push({
          tier: t.tier,
          suite: t.suite,
          name: t.name,
          reqs: t.reqs,
          readOnly: t.readOnly,
        });
      }
    }
    if (opts.json) {
      writeJsonStdoutAndExit({ total: listed.length, byTier, tests: listed }, 0);
    } else {
      for (const item of listed) {
        const reqPart = item.reqs.length > 0 ? ` [${item.reqs.join(", ")}]` : "";
        const roPart = item.readOnly ? " (readOnly)" : "";
        console.log(`  [${item.tier}] ${item.suite} :: ${item.name}${reqPart}${roPart}`);
      }
      console.log(`\nListed ${listed.length} matching test(s):`);
      for (const k of Object.keys(byTier).sort()) {
        console.log(`  tier=${k}: ${byTier[k]}`);
      }
    }
    process.exit(0);
  }

  const startedAt = Date.now();
  const report = {
    summary: {
      total: 0,
      passed: 0,
      failed: 0,
      skipped: 0,
      durationMs: 0,
    },
    byTier: {},
    byReq: {},
    tests: [],
    failures: [],
    tempDirs: [],
  };

  let activeServer = null;
  let activeBrowser = null;
  let signalShuttingDown = false;
  let signalShutdownPromise = null;

  const cleanupResources = async () => {
    if (activeBrowser) {
      const b = activeBrowser;
      activeBrowser = null;
      await Promise.race([
        b.close().catch(() => {}),
        new Promise((r) => setTimeout(r, 2000)),
      ]);
    }
    if (activeServer) {
      const s = activeServer;
      activeServer = null;
      await s.stop().catch(() => {});
    }
  };

  for (const sig of ["SIGINT", "SIGTERM"]) {
    process.on(sig, () => {
      signalShuttingDown = true;
      if (!signalShutdownPromise) {
        signalShutdownPromise = cleanupResources().finally(() => {
          process.exit(sig === "SIGINT" ? 130 : 143);
        });
      }
    });
  }

  const serverOptsBase = {
    repoRoot: DEFAULT_REPO_ROOT,
    keepTemp: opts.keepTemp,
    ...(opts.adminPath !== undefined ? { adminPath: opts.adminPath } : {}),
  };

  if (opts.baseUrl) {
    const extTarget = buildExternalTargetDescriptor(opts);
    const browserMgr = createBrowserManager({
      baseUrl: extTarget.baseUrl,
      repoRoot: DEFAULT_REPO_ROOT,
      screenshotDir: opts.screenshotDir || null,
    });
    activeBrowser = browserMgr;
    const ctx = buildContext(extTarget, browserMgr, opts);

    if (!opts.json) {
      console.log(
        `External target mode: ${extTarget.baseUrl} (adminPath=${extTarget.adminPath}, allowMutating=${Boolean(opts.allowMutating)})`
      );
    }

    try {
      for (const fileInfo of matchingFiles) {
        if (signalShuttingDown) break;
        if (!opts.json) {
          console.log(`\nSuite: ${fileInfo.relPath}`);
        }
        await runSuiteWithContext(fileInfo, ctx, testMatcher, opts, report, () => signalShuttingDown);
      }
    } finally {
      if (activeBrowser === browserMgr) {
        activeBrowser = null;
        await browserMgr.close().catch(() => {});
      }
    }
  } else if (opts.perSuite) {
    let firstBuild = opts.build;
    for (const fileInfo of matchingFiles) {
      if (signalShuttingDown) break;
      const server = await startIsolatedServer({
        ...serverOptsBase,
        build: firstBuild,
      });
      activeServer = server;
      firstBuild = false;
      report.tempDirs.push(server.tempDir);
      const browserMgr = createBrowserManager({
        baseUrl: server.baseUrl,
        repoRoot: DEFAULT_REPO_ROOT,
        screenshotDir: opts.screenshotDir || null,
      });
      activeBrowser = browserMgr;
      const ctx = buildContext(server, browserMgr, opts);
      if (!opts.json) {
        console.log(`\nRunning suite ${fileInfo.relPath} against ${server.baseUrl}`);
      }
      try {
        await runSuiteWithContext(fileInfo, ctx, testMatcher, opts, report, () => signalShuttingDown);
      } finally {
        if (activeBrowser === browserMgr) {
          activeBrowser = null;
          await browserMgr.close().catch(() => {});
        }
        if (activeServer === server) {
          activeServer = null;
        }
        await server.stop();
      }
    }
  } else {
    const server = await startIsolatedServer({
      ...serverOptsBase,
      build: opts.build,
    });
    activeServer = server;
    report.tempDirs.push(server.tempDir);
    const browserMgr = createBrowserManager({
      baseUrl: server.baseUrl,
      repoRoot: DEFAULT_REPO_ROOT,
      screenshotDir: opts.screenshotDir || null,
    });
    activeBrowser = browserMgr;
    const ctx = buildContext(server, browserMgr, opts);

    if (!opts.json) {
      console.log(`Isolated live server ready at ${server.baseUrl} (tempDir=${server.tempDir})`);
    }

    try {
      for (const fileInfo of matchingFiles) {
        if (signalShuttingDown) break;
        if (!opts.json) {
          console.log(`\nSuite: ${fileInfo.relPath}`);
        }
        await runSuiteWithContext(fileInfo, ctx, testMatcher, opts, report, () => signalShuttingDown);
      }
    } finally {
      if (activeBrowser === browserMgr) {
        activeBrowser = null;
        await browserMgr.close().catch(() => {});
      }
      if (activeServer === server) {
        activeServer = null;
      }
      await server.stop();
    }
  }

  if (signalShuttingDown && signalShutdownPromise) {
    await signalShutdownPromise;
    return;
  }

  report.summary.durationMs = Date.now() - startedAt;

  if (report.summary.total === 0) {
    const msg = "0 tests matched the requested filters";
    if (opts.json) {
      report.summary.error = msg;
      writeJsonStdoutAndExit(report, 1);
    } else {
      console.error(`ERROR: ${msg}`);
    }
    process.exit(1);
  }

  if (opts.json) {
    writeJsonStdoutAndExit(report, report.summary.failed === 0 ? 0 : 1);
  } else {
    printHumanSummary(report, opts);
  }

  process.exit(report.summary.failed === 0 ? 0 : 1);
}

main().catch((err) => {
  console.error("E2E runner fatal error:", err?.stack || err);
  process.exit(1);
});
