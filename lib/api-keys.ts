import { createHash } from "node:crypto";

/**
 * Generate a Stripe-style API key: pk_live_<40 random chars>
 * Uses crypto.getRandomValues for cryptographic security.
 */
export function generateApiKey(): string {
  const chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
  const array = new Uint8Array(40);
  crypto.getRandomValues(array);
  let random = "";
  for (const byte of array) {
    random += chars[byte % chars.length];
  }
  return `pk_live_${random}`;
}

/** SHA-256 hash of an API key for secure storage. */
export function hashApiKey(key: string): string {
  return createHash("sha256").update(key).digest("hex");
}

/** First 12 characters of the key for display identification. */
export function getKeyPrefix(key: string): string {
  return key.substring(0, 12);
}
