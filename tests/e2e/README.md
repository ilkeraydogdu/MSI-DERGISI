# MSI Dergisi Platform — Isolated E2E Test Suite

## Quick Start

```bash
# Run smoke verification (isolated ephemeral server + Playwright desktop/mobile)
node tests/e2e/run.mjs --suite=smoke

# Run all tiers (smoke + tier1..tier4)
node tests/e2e/run.mjs --tier=all

# Run specific tier with JSON output
node tests/e2e/run.mjs --tier=1 --json
```

## CLI Flags

- `--tier=1|2|3|4|all|smoke` — Select test tiers to execute (supports comma-separated values).
- `--suite=<name>` — Run matching suite file(s) under `tests/e2e/suites/` (e.g. `--suite=smoke`).
- `--filter=<pattern>` — Filter tests by substring or `/regex/` against test name, suite name, or `REQ-*` id.
- `--json` — Emit machine-readable JSON summary to `stdout`.
- `--keep-temp` — Preserve `/tmp/msi_e2e_*` workspace directory after completion for debugging.
- `--build` — Force `cargo build --bin msi-dergisi-platform` before starting the server.
- `--per-suite` — Spawn a fresh isolated server instance for each suite file.

## Environment Overrides

- `E2E_BINARY` — Path to `msi-dergisi-platform` binary (defaults to `target/debug/msi-dergisi-platform`).
- `PLAYWRIGHT_MODULE` — Explicit path to Playwright Node module (auto-discovered from repo/host caches if unset).
- `PLAYWRIGHT_EXECUTABLE_PATH` — Explicit path to Chromium binary (auto-discovered from `/root/.cache/ms-playwright` if unset).

## Test Module Contract (`E2E-M2` / `E2E-M3`)

Each file under `tests/e2e/suites/tier{1,2,3,4}/*.test.mjs` must export:

```js
export function registerTests(suite, ctx) {
  suite.test("descriptive behavior check", { req: "REQ-001" }, async (c) => {
    const client = c.http.createClient({ defaultHost: "savunmahaber.com" });
    const res = await client.get("/");
    c.assert.assertStatus(res, 200);
  });
}
```

### `ctx` API Surface

- `ctx.baseUrl` — Ephemeral URL (`http://127.0.0.1:<free_port>`).
- `ctx.port`, `ctx.adminPath`, `ctx.tempDir`, `ctx.dbPath`, `ctx.edergiDir`, `ctx.backupDir`, `ctx.uploadsDir`.
- `ctx.credentials` — `{ admin: { email, password, role: "site_operator" }, superAdmin: { email, password, role: "super_admin" } }`.
- `ctx.http` — `{ Client, createClient(opts) }` with per-client cookie jar (longest-path-first `getCookie`), auto `X-CSRF-Token`, `Host` header override (`options.host`, `defaultHost`, or `headers.Host`), isolated local-alias redirect rewriting, JSON/form/multipart, and `redirect: "manual"`.
- `ctx.auth` — Constants (`ALL_ROLES`, `STAFF_ROLES`, `READER_ROLES`, `UNAUTHENTICATED`) and helpers (`login`, `loginAsAdmin`, `loginAsSuperAdmin`, `registerUser`, `createUserWithRole`, `approveCorporateRequest`, `verifyEmailViaToken`, `enableTotp`, `verifyTotpLogin`, `disableTotp`).
- `ctx.totp` — RFC 6238 SHA1 6-digit 30s generator (`generateTotp`, `base32Decode`, `parseOtpAuthUri`).
- `ctx.browser` — Lazy Playwright Chromium launcher (`--host-resolver-rules=MAP * 127.0.0.1` for genuine multi-domain `options.host` routing, `openPage`, `newContext`, `authenticateContext` with TOTP 2FA support, `measureOverflow`, `VIEWPORT_DESKTOP`, `VIEWPORT_MOBILE`).
- `ctx.assert` — Opaque-box assertions (`assertOk`, `assertEqual`, `assertDeepEqual`, `assertStatus`, `assertJson`, `assertIncludes`, `assertNotIncludes`, `assertMatch`, `assertNoOverflow`, `assertNoBrowserErrors`, `assertThrows`).

## Isolation Guarantees

1. Server always spawns with `cwd: /tmp/msi_e2e_<rand>` so `dotenvy::dotenv()` never loads repo `.env`.
2. `SQLITE_DB_PATH`, `EDERGI_DIR`, `BACKUP_DIR`, and `STATIC_DIR/images/uploads` point exclusively inside `/tmp/msi_e2e_<rand>`.
3. `MAIL_DELIVERY_ENABLED=false`, `MONITORING_ENABLED=false`, `BACKUP_ENABLED=false`, `PAYTR_TEST_MODE=true`, `PAYTR_MERCHANT_ID=""`, `INDEXNOW_API_URL` and `PAYTR_API_URL` pointed to closed local endpoints.
4. `SIGINT`/`SIGTERM`/exit hooks kill the child server (`SIGTERM` -> `SIGKILL` fallback) and remove `/tmp/msi_e2e_<rand>` unless `--keep-temp` is passed.
