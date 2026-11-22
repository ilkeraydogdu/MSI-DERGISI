import crypto from "node:crypto";
import { assertEqual, assertOk, assertStatus } from "./assertions.mjs";
import { generateTotp } from "./totp.mjs";

export const SYSTEM_STAFF_ROLES = Object.freeze([
  "super_admin",
  "admin",
  "site_operator",
]);

export const EDITABLE_STAFF_ROLES = Object.freeze([
  "editor_in_chief",
  "managing_editor",
  "content_publisher",
  "content_editor",
  "editor",
  "business_dev",
  "ads_manager",
  "finance_viewer",
  "support_agent",
]);

export const STAFF_ROLES = Object.freeze([
  ...SYSTEM_STAFF_ROLES,
  ...EDITABLE_STAFF_ROLES,
]);

export const READER_ROLES = Object.freeze([
  "subscriber_public",
  "subscriber_corporate",
  "subscriber_print_digital",
  "subscriber_digital",
  "registered",
]);

export const ALL_ROLES = Object.freeze([...STAFF_ROLES, ...READER_ROLES]);
export const UNAUTHENTICATED = "unauthenticated";

function uniqueSuffix() {
  return `${Date.now()}_${crypto.randomBytes(4).toString("hex")}`;
}

/**
 * Log in using an existing `Client`. Handles optional 2FA challenge if `totpSecret` or `backupCode` is supplied.
 */
export async function login(client, { email, password, totpSecret = null, backupCode = null }) {
  const res = await client.post("/api/v1/auth/login", {
    json: { email, password },
  });
  assertStatus(res, 200, `Login failed for ${email}`);

  if (res.json?.requires_totp || res.json?.status === "totp_required") {
    const tempToken = res.json.temp_token;
    assertOk(tempToken, `Expected temp_token on TOTP challenge for ${email}`);
    if (!totpSecret && !backupCode) {
      return {
        status: "totp_required",
        requiresTotp: true,
        tempToken,
        user: null,
        response: res,
      };
    }
    const secretOrCode = backupCode || totpSecret;
    const totpRes = await verifyTotpLogin(client, tempToken, secretOrCode);
    return {
      status: totpRes.status,
      requiresTotp: true,
      tempToken,
      user: totpRes.user,
      emailVerified: totpRes.emailVerified,
      response: totpRes.res,
    };
  }

  assertEqual(res.json?.status, "authenticated", `Expected authenticated login status for ${email}`);
  return {
    status: "authenticated",
    requiresTotp: false,
    user: res.json.user,
    emailVerified: Boolean(res.json.email_verified),
    response: res,
  };
}

/**
 * Log in as the bootstrapped `site_operator` (`ADMIN_DEFAULT_USER`).
 */
export async function loginAsAdmin(ctx) {
  const client = ctx.http.createClient();
  const { email, password } = ctx.credentials.admin;
  const authRes = await login(client, { email, password });
  await client.ensureCsrf();
  return {
    client,
    user: authRes.user,
    email,
    password,
    role: authRes.user?.role || "site_operator",
  };
}

/**
 * Log in as the bootstrapped `super_admin` user.
 */
export async function loginAsSuperAdmin(ctx) {
  const client = ctx.http.createClient();
  const { email, password } = ctx.credentials.superAdmin;
  const authRes = await login(client, { email, password });
  await client.ensureCsrf();
  return {
    client,
    user: authRes.user,
    email,
    password,
    role: authRes.user?.role || "super_admin",
  };
}

/**
 * Public registration via `POST /api/v1/auth/register`.
 */
export async function registerUser(ctx, overrides = {}) {
  const suffix = uniqueSuffix();
  const client = overrides.client || ctx.http.createClient();
  const email = overrides.email || `e2e.reg.${suffix}@example.com`;
  const password = overrides.password || "E2eReader!2026";
  const payload = {
    email,
    full_name: overrides.full_name || `E2E Reader ${suffix}`,
    password,
    kvkk_consent: true,
    kvkk_acknowledged: true,
    tos_accepted: true,
    organization_category: overrides.organization_category || "Sektör Kuruluşları",
    organization_name: overrides.organization_name || "E2E Savunma A.Ş.",
    job_title: overrides.job_title || "Sistem Mühendisi",
    email_consent: overrides.email_consent ?? false,
    ssb_interests: overrides.ssb_interests || ["hava_savunma"],
  };

  if (overrides.request_message !== undefined) payload.request_message = overrides.request_message;
  if (overrides.tos_version !== undefined) payload.tos_version = overrides.tos_version;
  if (overrides.kvkk_consent !== undefined) payload.kvkk_consent = overrides.kvkk_consent;
  if (overrides.kvkk_acknowledged !== undefined) {
    payload.kvkk_acknowledged = overrides.kvkk_acknowledged;
  }
  if (overrides.tos_accepted !== undefined) payload.tos_accepted = overrides.tos_accepted;
  if (overrides.website_url !== undefined) payload.website_url = overrides.website_url;
  if (overrides._form_ts !== undefined) payload._form_ts = overrides._form_ts;

  const res = await client.post("/api/v1/auth/register", { json: payload });
  if (!overrides.allowError) {
    assertStatus(res, 200, `User registration failed for ${email}`);
  }

  return {
    res,
    client,
    email,
    password,
    user: res.json?.user || null,
    govAutoSubscribed: Boolean(res.json?.gov_auto_subscribed),
    corporateVerificationQueued: Boolean(res.json?.corporate_verification_queued),
    emailVerificationSent: Boolean(res.json?.email_verification_sent),
  };
}

/**
 * Provision a user with any of the 17 canonical roles (or `unauthenticated`) via `super_admin`
 * `POST /api/v1/admin/users`, and log in as that user.
 */
export async function createUserWithRole(ctx, role, overrides = {}) {
  if (!role || role === UNAUTHENTICATED) {
    return {
      id: null,
      user: null,
      email: null,
      password: null,
      role: UNAUTHENTICATED,
      client: ctx.http.createClient(),
    };
  }

  const suffix = uniqueSuffix();
  const safeRole = role.replace(/[^a-z0-9_]/gi, "_");
  const email = overrides.email || `e2e.${safeRole}.${suffix}@msidergisi-test.com`;
  const password = overrides.password || `E2eRole!${randPassSuffix()}`;

  const superAdmin = overrides.superAdminClient
    ? { client: overrides.superAdminClient }
    : await loginAsSuperAdmin(ctx);

  await superAdmin.client.ensureCsrf();
  const createRes = await superAdmin.client.post("/api/v1/admin/users", {
    json: {
      email,
      password,
      full_name: overrides.full_name || `E2E ${role} ${suffix}`,
      role,
      organization_category: overrides.organization_category || "Sektör Kuruluşları",
      organization_name: overrides.organization_name || "E2E Test Kurumu",
      job_title: overrides.job_title || role,
      is_gov_verified: overrides.is_gov_verified ?? role === "subscriber_public",
      subscription_tier: overrides.subscription_tier || "",
    },
  });
  assertStatus(createRes, 200, `Failed to create user with role ${role}`);
  const createdId = createRes.json?.data?.id;
  assertOk(createdId, `Expected created user id for role ${role}`);

  const client = ctx.http.createClient();
  let user = null;
  if (!overrides.skipLogin) {
    const authRes = await login(client, { email, password });
    user = authRes.user;
    await client.ensureCsrf();
  }

  return {
    id: createdId,
    user,
    email,
    password,
    role,
    client,
  };
}

function randPassSuffix() {
  return crypto.randomBytes(4).toString("hex");
}

/**
 * Approve or reject a pending corporate verification request (`Tab 4`).
 */
export async function approveCorporateRequest(
  ctx,
  requestId,
  status = "approved",
  adminClient = null
) {
  const actor = adminClient || (await loginAsSuperAdmin(ctx)).client;
  await actor.ensureCsrf();
  const res = await actor.post(`/api/v1/admin/corporate-requests/${requestId}/decide`, {
    json: { status },
  });
  assertStatus(res, 200, `Failed to decide corporate request ${requestId} -> ${status}`);
  return res;
}

/**
 * Confirm email verification via `/hesap-dogrula?token=<tok>&confirm=1`.
 */
export async function verifyEmailViaToken(ctx, token, client = null) {
  const c = client || ctx.http.createClient();
  const res = await c.get("/hesap-dogrula", {
    query: { token, confirm: "1" },
  });
  return res;
}

/**
 * Enroll the authenticated client in TOTP 2FA: setup -> compute code -> verify-setup.
 */
export async function enableTotp(client) {
  const setupRes = await client.post("/api/v1/auth/totp/setup");
  assertStatus(setupRes, 200, "TOTP setup failed");
  const secret = setupRes.json?.data?.secret;
  const otpauthUri = setupRes.json?.data?.otpauth_uri;
  assertOk(secret, "Expected Base32 secret from /api/v1/auth/totp/setup");

  const code = generateTotp(secret);
  const verifyRes = await client.post("/api/v1/auth/totp/verify-setup", {
    json: { code, secret },
  });
  assertStatus(verifyRes, 200, "TOTP verify-setup failed");
  const backupCodes = verifyRes.json?.data?.backup_codes || [];
  assertOk(Array.isArray(backupCodes) && backupCodes.length > 0, "Expected TOTP backup codes");

  return {
    secret,
    otpauthUri,
    qrSvg: setupRes.json?.data?.qr_svg || "",
    backupCodes,
    response: verifyRes,
  };
}

/**
 * Complete the second step of TOTP login using either a Base32 secret (>=16 chars),
 * a 6-digit TOTP code, or an 8-char backup recovery code.
 */
export async function verifyTotpLogin(client, tempToken, secretOrCode) {
  let code;
  if (typeof secretOrCode === "object" && secretOrCode !== null) {
    code = secretOrCode.code || generateTotp(secretOrCode.secret);
  } else {
    const raw = String(secretOrCode || "").trim();
    const cleaned = raw.replace(/\s+/g, "");
    // Base32 secrets are >= 16 chars (26-32 chars from totp-rs), whereas TOTP codes are 6 digits and backup codes are 8 chars
    code = cleaned.length >= 16 && /^[A-Z2-7=]+$/i.test(cleaned) ? generateTotp(cleaned) : raw;
  }

  const res = await client.post("/api/v1/auth/totp/login", {
    json: { temp_token: tempToken, code },
  });
  assertStatus(res, 200, "TOTP login verification failed");
  assertEqual(res.json?.status, "authenticated", "Expected authenticated status after TOTP login");
  return {
    res,
    status: res.json.status,
    user: res.json.user,
    emailVerified: Boolean(res.json.email_verified),
  };
}

/**
 * Disable TOTP 2FA on the authenticated client using `{ password }`, `{ code }`, or `{ secret }`.
 */
export async function disableTotp(client, creds = {}) {
  const payload = {};
  if (creds.password) payload.password = creds.password;
  if (creds.code) {
    payload.code = creds.code;
  } else if (creds.secret) {
    payload.code = generateTotp(creds.secret);
  }
  const res = await client.post("/api/v1/auth/totp/disable", { json: payload });
  assertStatus(res, 200, "TOTP disable failed");
  return res;
}

export function createAuthHelpers(ctx) {
  return {
    ALL_ROLES,
    SYSTEM_STAFF_ROLES,
    EDITABLE_STAFF_ROLES,
    STAFF_ROLES,
    READER_ROLES,
    UNAUTHENTICATED,
    login,
    loginAsAdmin: () => loginAsAdmin(ctx),
    loginAsSuperAdmin: () => loginAsSuperAdmin(ctx),
    registerUser: (overrides = {}) => registerUser(ctx, overrides),
    createUserWithRole: (role, overrides = {}) => createUserWithRole(ctx, role, overrides),
    approveCorporateRequest: (requestId, status = "approved", adminClient = null) =>
      approveCorporateRequest(ctx, requestId, status, adminClient),
    verifyEmailViaToken: (token, client = null) => verifyEmailViaToken(ctx, token, client),
    enableTotp,
    verifyTotpLogin,
    disableTotp,
  };
}
