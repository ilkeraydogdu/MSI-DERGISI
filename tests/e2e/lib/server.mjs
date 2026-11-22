import { execFileSync, spawn, spawnSync } from "node:child_process";
import crypto from "node:crypto";
import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
export const DEFAULT_REPO_ROOT = path.resolve(__dirname, "../../..");

const activeServers = new Set();
let cleanupHooksInstalled = false;

function installGlobalCleanupHooks() {
  if (cleanupHooksInstalled) return;
  cleanupHooksInstalled = true;

  const cleanupSync = () => {
    for (const srv of Array.from(activeServers)) {
      try {
        srv.forceStopSync();
      } catch {
        // best-effort
      }
    }
  };

  process.on("exit", cleanupSync);

  for (const sig of ["SIGINT", "SIGTERM"]) {
    if (process.listenerCount(sig) === 0) {
      process.on(sig, async () => {
        for (const srv of Array.from(activeServers)) {
          try {
            await srv.stop();
          } catch {
            // best-effort
          }
        }
        process.exit(sig === "SIGINT" ? 130 : 143);
      });
    }
  }
}

export async function findFreePort(host = "127.0.0.1") {
  return await new Promise((resolve, reject) => {
    const srv = net.createServer();
    srv.unref();
    srv.on("error", reject);
    srv.listen(0, host, () => {
      const addr = srv.address();
      if (!addr || typeof addr === "string") {
        srv.close(() => reject(new Error("Failed to acquire ephemeral TCP port")));
        return;
      }
      const { port } = addr;
      srv.close((err) => {
        if (err) reject(err);
        else resolve(port);
      });
    });
  });
}

function randHex(bytes = 6) {
  return crypto.randomBytes(bytes).toString("hex");
}

function sqlLiteral(str) {
  return "'" + String(str).replace(/'/g, "''") + "'";
}

function prepareTempWorkspace(repoRoot) {
  const tempDir = path.join("/tmp", `msi_e2e_${Date.now()}_${randHex(4)}`);
  const dbDir = path.join(tempDir, "db");
  const edergiDir = path.join(tempDir, "edergi");
  const backupDir = path.join(tempDir, "backups");
  const logsDir = path.join(tempDir, "logs");
  const publicDir = path.join(tempDir, "public");
  const publicImagesDir = path.join(publicDir, "images");
  const uploadsIssuesDir = path.join(publicImagesDir, "uploads", "issues");

  for (const dir of [dbDir, edergiDir, backupDir, logsDir, uploadsIssuesDir]) {
    fs.mkdirSync(dir, { recursive: true });
  }

  // Symlink read-only static assets from <repo>/public into <tempDir>/public
  const repoPublic = path.join(repoRoot, "public");
  for (const entry of ["css", "js", "fonts", "favicon.ico", "editorial-studio"]) {
    const src = path.join(repoPublic, entry);
    const dst = path.join(publicDir, entry);
    if (fs.existsSync(src)) {
      fs.symlinkSync(src, dst);
    }
  }

  const repoImages = path.join(repoPublic, "images");
  if (fs.existsSync(repoImages)) {
    for (const entry of fs.readdirSync(repoImages)) {
      if (entry === "uploads") continue; // Keep uploads/ real and empty in tempDir
      const src = path.join(repoImages, entry);
      const dst = path.join(publicImagesDir, entry);
      fs.symlinkSync(src, dst);
    }
  }

  // Copy seed PDFs into isolated edergiDir
  const repoEdergi = path.join(repoRoot, "storage", "edergi");
  if (fs.existsSync(repoEdergi)) {
    for (const file of fs.readdirSync(repoEdergi)) {
      if (file.toLowerCase().endsWith(".pdf")) {
        fs.copyFileSync(path.join(repoEdergi, file), path.join(edergiDir, file));
      }
    }
  }

  return {
    tempDir,
    dbPath: path.join(dbDir, "msi_platform.db"),
    edergiDir,
    backupDir,
    logsDir,
    publicDir,
    uploadsDir: path.join(publicImagesDir, "uploads"),
  };
}

function resolveBinary(repoRoot, forceBuild = false) {
  const binaryPath =
    process.env.E2E_BINARY || path.join(repoRoot, "target", "debug", "msi-dergisi-platform");

  if (forceBuild || !fs.existsSync(binaryPath)) {
    const res = spawnSync("cargo", ["build", "--bin", "msi-dergisi-platform"], {
      cwd: repoRoot,
      stdio: "inherit",
    });
    if (res.status !== 0) {
      throw new Error(`cargo build --bin msi-dergisi-platform failed with code ${res.status}`);
    }
    if (!fs.existsSync(binaryPath)) {
      throw new Error(`Expected binary not found after build: ${binaryPath}`);
    }
  }
  return binaryPath;
}

function queryUsersTableColumnsSqlite3(dbPath) {
  const out = execFileSync("sqlite3", [dbPath, "PRAGMA table_info(users);"], {
    encoding: "utf8",
  });
  return out
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => line.split("|")[1])
    .filter(Boolean);
}

async function bootstrapSuperAdminAccount(dbPath, adminEmail, superAdminEmail) {
  const overrides = {
    email: sqlLiteral(superAdminEmail.toLowerCase()),
    full_name: sqlLiteral("MSI Super Admin"),
    role: sqlLiteral("super_admin"),
    job_title: sqlLiteral("Sistem Süper Yöneticisi"),
    is_gov_verified: "1",
    email_verified: "1",
    reminders_halted: "1",
    totp_secret: "NULL",
    totp_backup_codes_json: sqlLiteral("[]"),
    totp_enabled: "0",
  };

  let sqlite3Available = false;
  try {
    execFileSync("which", ["sqlite3"], { stdio: "ignore" });
    sqlite3Available = true;
  } catch {
    sqlite3Available = false;
  }

  if (sqlite3Available) {
    const cols = queryUsersTableColumnsSqlite3(dbPath).filter((c) => c !== "id");
    if (cols.length === 0) {
      throw new Error(`users table columns could not be inspected in ${dbPath}`);
    }
    const selectExprs = cols.map((col) => (col in overrides ? overrides[col] : col));
    const sql = [
      `DELETE FROM users WHERE LOWER(email) = LOWER(${sqlLiteral(superAdminEmail)});`,
      `INSERT INTO users (${cols.join(", ")})`,
      `SELECT ${selectExprs.join(", ")}`,
      `FROM users WHERE LOWER(email) = LOWER(${sqlLiteral(adminEmail)}) LIMIT 1;`,
      `SELECT role || '|' || COALESCE(email_verified, 0) || '|' || COALESCE(is_gov_verified, 0) FROM users WHERE LOWER(email) = LOWER(${sqlLiteral(superAdminEmail)});`,
    ].join("\n");

    const verifyOut = execFileSync("sqlite3", [dbPath, sql], { encoding: "utf8" }).trim();
    if (!verifyOut.endsWith("super_admin|1|1")) {
      throw new Error(
        `Failed to bootstrap super_admin account in ${dbPath}; verification output: "${verifyOut}"`
      );
    }
    return;
  }

  // Fallback to Node built-in node:sqlite if sqlite3 CLI is unavailable
  const { DatabaseSync } = await import("node:sqlite");
  const db = new DatabaseSync(dbPath);
  try {
    const infoRows = db.prepare("PRAGMA table_info(users)").all();
    const cols = infoRows.map((r) => r.name).filter((c) => c !== "id");
    const selectExprs = cols.map((col) => (col in overrides ? overrides[col] : col));
    db.prepare(`DELETE FROM users WHERE LOWER(email) = LOWER(?)`).run(superAdminEmail);
    db.exec(
      `INSERT INTO users (${cols.join(", ")}) SELECT ${selectExprs.join(", ")} FROM users WHERE LOWER(email) = LOWER(${sqlLiteral(adminEmail)}) LIMIT 1;`
    );
    const row = db
      .prepare("SELECT role, email_verified, is_gov_verified FROM users WHERE LOWER(email) = LOWER(?)")
      .get(superAdminEmail);
    if (!row || row.role !== "super_admin" || row.email_verified !== 1) {
      throw new Error(`node:sqlite super_admin bootstrap failed: ${JSON.stringify(row)}`);
    }
  } finally {
    db.close();
  }
}

function readTail(filePath, maxLines = 80) {
  try {
    if (!fs.existsSync(filePath)) return "(log file missing)";
    const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
    return lines.slice(-maxLines).join("\n");
  } catch (err) {
    return `(failed to read log: ${err.message})`;
  }
}

async function waitForHealth({ baseUrl, child, logPath, timeoutMs = 30000 }) {
  const deadline = Date.now() + timeoutMs;
  let lastError = null;

  while (Date.now() < deadline) {
    if (child.exitCode !== null) {
      throw new Error(
        `Server exited early with code ${child.exitCode} before becoming healthy.\n--- server.log tail ---\n${readTail(logPath)}`
      );
    }

    try {
      const resp = await fetch(`${baseUrl}/api/v1/health`, { redirect: "manual" });
      if (resp.status === 200) {
        const data = await resp.json();
        if (data && data.status === "ok") {
          return data;
        }
      }
      lastError = `HTTP ${resp.status}`;
    } catch (err) {
      lastError = err.message;
    }

    await new Promise((r) => setTimeout(r, 150));
  }

  throw new Error(
    `Timed out waiting for ${baseUrl}/api/v1/health (${lastError}).\n--- server.log tail ---\n${readTail(logPath)}`
  );
}

/**
 * Start an isolated ephemeral `msi-dergisi-platform` instance.
 */
export async function startIsolatedServer(options = {}) {
  installGlobalCleanupHooks();

  const repoRoot = options.repoRoot || DEFAULT_REPO_ROOT;
  const keepTemp = Boolean(options.keepTemp);
  const binaryPath = resolveBinary(repoRoot, Boolean(options.build));
  const port = options.port || (await findFreePort("127.0.0.1"));
  const paths = prepareTempWorkspace(repoRoot);

  const hasExplicitAdminPath =
    Object.prototype.hasOwnProperty.call(options, "adminPath") &&
    options.adminPath !== undefined &&
    options.adminPath !== null;
  const configuredAdminPath = hasExplicitAdminPath
    ? String(options.adminPath)
    : `e2e-admin-${randHex(6)}`;
  const adminPath = configuredAdminPath.trim() === "" ? "podmin" : configuredAdminPath;
  const adminEmail = options.adminEmail || "operator@msidergisi.com";
  const adminPassword = options.adminPassword || `E2eOp!${randHex(6)}`;
  const superAdminEmail = options.superAdminEmail || "superadmin@msidergisi.com";
  const baseUrl = `http://127.0.0.1:${port}`;

  const childEnv = {
    PATH: process.env.PATH || "/usr/local/bin:/usr/bin:/bin",
    HOME: paths.tempDir,
    LANG: process.env.LANG || "C.UTF-8",
    APP_ENV: "test",
    HOST: "127.0.0.1",
    PORT: String(port),
    PUBLIC_URL: baseUrl,
    PRIMARY_DOMAIN: "msidergisi.com",
    NEWS_DOMAIN: "savunmahaber.com",
    SQLITE_DB_PATH: paths.dbPath,
    BACKUP_DIR: paths.backupDir,
    BACKUP_ENABLED: "false",
    EDERGI_DIR: paths.edergiDir,
    STATIC_DIR: paths.publicDir,
    MSI_ARCHIVE_JSON_PATH: path.join(repoRoot, "storage", "seed", "live_msi_archive.json"),
    SESSION_SECRET: `e2e-session-${randHex(16)}`,
    LEGACY_SEED_SECRET: `e2e-legacy-${randHex(16)}`,
    WATERMARK_SECRET: `e2e-watermark-${randHex(16)}`,
    ADMIN_PATH_SECRET: configuredAdminPath,
    ADMIN_DEFAULT_USER: adminEmail,
    ADMIN_DEFAULT_PASS: adminPassword,
    MAIL_DELIVERY_ENABLED: "false",
    MONITORING_ENABLED: "false",
    PAYTR_TEST_MODE: "true",
    PAYTR_MERCHANT_ID: "",
    PAYTR_MERCHANT_KEY: "",
    PAYTR_MERCHANT_SALT: "",
    PAYTR_API_URL: "http://127.0.0.1:1/paytr-disabled",
    INDEXNOW_API_URL: "http://127.0.0.1:1/indexnow-disabled",
    RATE_LIMIT_MAX_RPM: String(options.rateLimitMaxRpm ?? 100000),
    RATE_LIMIT_WINDOW_SECS: String(options.rateLimitWindowSecs ?? 60),
    RUST_LOG: options.rustLog || "info",
    ...(options.envOverrides || {}),
  };

  let child = null;
  let logFd = null;
  let stopPromise = null;
  let tempRemoved = false;

  function removeTempDirSync() {
    if (keepTemp || tempRemoved) return;
    if (fs.existsSync(paths.tempDir)) {
      fs.rmSync(paths.tempDir, { recursive: true, force: true });
    }
    tempRemoved = true;
  }

  const instance = {
    port,
    baseUrl,
    adminPath,
    configuredAdminPath,
    repoRoot,
    binaryPath,
    tempDir: paths.tempDir,
    dbPath: paths.dbPath,
    edergiDir: paths.edergiDir,
    backupDir: paths.backupDir,
    publicDir: paths.publicDir,
    uploadsDir: paths.uploadsDir,
    logPath: path.join(paths.logsDir, "server.log"),
    migrateLogPath: path.join(paths.logsDir, "migrate.log"),
    env: childEnv,
    credentials: {
      admin: { email: adminEmail, password: adminPassword, role: "site_operator" },
      superAdmin: { email: superAdminEmail, password: adminPassword, role: "super_admin" },
    },
    readLogTail(lines = 80) {
      return readTail(instance.logPath, lines);
    },
    forceStopSync() {
      if (child && child.exitCode === null) {
        try {
          child.kill("SIGKILL");
        } catch {
          // ignore
        }
      }
      if (logFd !== null) {
        try {
          fs.closeSync(logFd);
        } catch {
          // ignore
        }
        logFd = null;
      }
      try {
        removeTempDirSync();
      } catch {
        // best-effort
      }
      activeServers.delete(instance);
    },
    async stop() {
      if (stopPromise) {
        return await stopPromise;
      }
      stopPromise = (async () => {
        if (child && child.exitCode === null) {
          await new Promise((resolve) => {
            const killTimer = setTimeout(() => {
              if (child && child.exitCode === null) {
                try {
                  child.kill("SIGKILL");
                } catch {
                  // ignore
                }
              }
            }, 5000);

            child.once("exit", () => {
              clearTimeout(killTimer);
              resolve();
            });

            try {
              child.kill("SIGTERM");
            } catch {
              clearTimeout(killTimer);
              resolve();
            }
          });
        }

        if (logFd !== null) {
          try {
            fs.closeSync(logFd);
          } catch {
            // ignore
          }
          logFd = null;
        }

        removeTempDirSync();
        activeServers.delete(instance);
      })();
      return await stopPromise;
    },
  };

  activeServers.add(instance);

  try {
    // Step 1: Run --migrate-only to initialize isolated SQLite schema, seed data/roles & operator user
    const migrateRes = spawnSync(binaryPath, ["--migrate-only"], {
      cwd: paths.tempDir,
      env: childEnv,
      encoding: "utf8",
      maxBuffer: 16 * 1024 * 1024,
    });
    fs.writeFileSync(
      instance.migrateLogPath,
      [migrateRes.stdout || "", migrateRes.stderr || ""].filter(Boolean).join("\n"),
      "utf8"
    );
    if (migrateRes.status !== 0) {
      throw new Error(
        `--migrate-only failed (exit ${migrateRes.status}):\n${readTail(instance.migrateLogPath, 60)}`
      );
    }

    // Step 2: Bootstrap super_admin user in isolated SQLite DB while keeping operator as site_operator
    await bootstrapSuperAdminAccount(paths.dbPath, adminEmail, superAdminEmail);

    // Step 3: Spawn live server with cwd = tempDir so dotenvy never reads repo .env
    logFd = fs.openSync(instance.logPath, "a");
    child = spawn(binaryPath, [], {
      cwd: paths.tempDir,
      env: childEnv,
      stdio: ["ignore", logFd, logFd],
    });

    const health = await waitForHealth({
      baseUrl,
      child,
      logPath: instance.logPath,
      timeoutMs: options.startupTimeoutMs ?? 30000,
    });
    instance.health = health;
    return instance;
  } catch (err) {
    await instance.stop();
    throw err;
  }
}
