import crypto from "node:crypto";

const BASE32_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/**
 * Decode an RFC 4648 Base32 string into a Buffer.
 */
export function base32Decode(secretBase32) {
  if (typeof secretBase32 !== "string") {
    throw new Error("TOTP secret must be a string");
  }
  const cleaned = secretBase32.replace(/[\s=-]+/g, "").toUpperCase();
  if (!cleaned) {
    throw new Error("TOTP secret is empty");
  }

  let bits = 0;
  let value = 0;
  const bytes = [];

  for (let i = 0; i < cleaned.length; i++) {
    const ch = cleaned[i];
    const idx = BASE32_ALPHABET.indexOf(ch);
    if (idx === -1) {
      throw new Error(`Invalid Base32 character '${ch}' at index ${i}`);
    }
    value = (value << 5) | idx;
    bits += 5;
    if (bits >= 8) {
      bytes.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }

  return Buffer.from(bytes);
}

/**
 * Generate an RFC 6238 TOTP code matching `src/services/totp_service.rs`
 * (SHA1, 6 digits, 30s period).
 */
export function generateTotp(secretBase32, opts = {}) {
  const {
    timestampMs = Date.now(),
    period = 30,
    digits = 6,
    algorithm = "sha1",
    stepOffset = 0,
  } = opts;

  const key = base32Decode(secretBase32);
  const counter = BigInt(Math.floor(timestampMs / 1000 / period) + stepOffset);
  const counterBuf = Buffer.alloc(8);
  counterBuf.writeBigUInt64BE(counter, 0);

  const hmac = crypto.createHmac(algorithm.toLowerCase(), key).update(counterBuf).digest();
  const offset = hmac[hmac.length - 1] & 0x0f;
  const binary =
    ((hmac[offset] & 0x7f) << 24) |
    ((hmac[offset + 1] & 0xff) << 16) |
    ((hmac[offset + 2] & 0xff) << 8) |
    (hmac[offset + 3] & 0xff);

  const mod = 10 ** digits;
  return String(binary % mod).padStart(digits, "0");
}

/**
 * Parse an `otpauth://totp/...` URI into its components.
 */
export function parseOtpAuthUri(uri) {
  const parsed = new URL(uri);
  if (parsed.protocol !== "otpauth:" || parsed.hostname.toLowerCase() !== "totp") {
    throw new Error(`Unsupported otpauth URI: ${uri}`);
  }
  const secret = parsed.searchParams.get("secret") || "";
  const issuer = parsed.searchParams.get("issuer") || "";
  const algorithm = (parsed.searchParams.get("algorithm") || "SHA1").toLowerCase();
  const digits = Number.parseInt(parsed.searchParams.get("digits") || "6", 10);
  const period = Number.parseInt(parsed.searchParams.get("period") || "30", 10);
  const label = decodeURIComponent(parsed.pathname.replace(/^\/+/, ""));

  return { label, secret, issuer, algorithm, digits, period };
}

/**
 * Convenience helper to generate a current TOTP code directly from an `otpauth://` URI.
 */
export function generateTotpFromUri(uri, opts = {}) {
  const info = parseOtpAuthUri(uri);
  return generateTotp(info.secret, {
    algorithm: info.algorithm,
    digits: info.digits,
    period: info.period,
    ...opts,
  });
}
