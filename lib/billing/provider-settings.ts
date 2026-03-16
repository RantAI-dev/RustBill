import { db } from "@/lib/db";
import { systemSettings } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

/**
 * Known payment provider setting keys.
 * DB keys are stored with this exact prefix convention.
 */
export const PROVIDER_KEYS = {
  // Stripe
  STRIPE_SECRET_KEY: "stripe_secret_key",
  STRIPE_WEBHOOK_SECRET: "stripe_webhook_secret",
  // Xendit
  XENDIT_SECRET_KEY: "xendit_secret_key",
  XENDIT_WEBHOOK_TOKEN: "xendit_webhook_token",
  // Lemonsqueezy
  LEMONSQUEEZY_API_KEY: "lemonsqueezy_api_key",
  LEMONSQUEEZY_STORE_ID: "lemonsqueezy_store_id",
  LEMONSQUEEZY_WEBHOOK_SECRET: "lemonsqueezy_webhook_secret",
} as const;

type ProviderKey = (typeof PROVIDER_KEYS)[keyof typeof PROVIDER_KEYS];

// In-memory cache with 60s TTL to avoid hitting DB on every request
const cache = new Map<string, { value: string; expiresAt: number }>();
const CACHE_TTL = 60_000;

/**
 * Get a provider setting value.
 * Reads from DB first (with cache), falls back to env var.
 */
export async function getProviderSetting(key: ProviderKey): Promise<string> {
  // Check cache first
  const cached = cache.get(key);
  if (cached && cached.expiresAt > Date.now()) {
    return cached.value;
  }

  // Try DB
  try {
    const [row] = await db
      .select()
      .from(systemSettings)
      .where(eq(systemSettings.key, key));

    if (row && row.value) {
      cache.set(key, { value: row.value, expiresAt: Date.now() + CACHE_TTL });
      return row.value;
    }
  } catch {
    // DB might not have the table yet, fall through to env
  }

  // Fallback to env var (map db key back to env name)
  const envKey = key.toUpperCase();
  const envValue = process.env[envKey] ?? "";
  return envValue;
}

/**
 * Get all provider settings for a specific provider.
 * Returns masked values for display (only last 4 chars shown for secrets).
 */
export async function getProviderStatus(): Promise<{
  stripe: { configured: boolean; secretKey: string; webhookSecret: string };
  xendit: { configured: boolean; secretKey: string; webhookToken: string };
  lemonsqueezy: { configured: boolean; apiKey: string; storeId: string; webhookSecret: string };
}> {
  const [
    stripeKey, stripeWebhook,
    xenditKey, xenditToken,
    lsKey, lsStore, lsWebhook,
  ] = await Promise.all([
    getProviderSetting(PROVIDER_KEYS.STRIPE_SECRET_KEY),
    getProviderSetting(PROVIDER_KEYS.STRIPE_WEBHOOK_SECRET),
    getProviderSetting(PROVIDER_KEYS.XENDIT_SECRET_KEY),
    getProviderSetting(PROVIDER_KEYS.XENDIT_WEBHOOK_TOKEN),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_API_KEY),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_STORE_ID),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_WEBHOOK_SECRET),
  ]);

  return {
    stripe: {
      configured: !!stripeKey,
      secretKey: maskValue(stripeKey),
      webhookSecret: maskValue(stripeWebhook),
    },
    xendit: {
      configured: !!xenditKey,
      secretKey: maskValue(xenditKey),
      webhookToken: maskValue(xenditToken),
    },
    lemonsqueezy: {
      configured: !!lsKey && !!lsStore,
      apiKey: maskValue(lsKey),
      storeId: lsStore || "",
      webhookSecret: maskValue(lsWebhook),
    },
  };
}

function maskValue(value: string): string {
  if (!value) return "";
  if (value.length <= 8) return "••••••••";
  return "••••••••" + value.slice(-4);
}

/**
 * Save a provider setting to DB.
 */
export async function saveProviderSetting(
  key: ProviderKey,
  value: string,
  sensitive = true,
): Promise<void> {
  await db
    .insert(systemSettings)
    .values({ key, value, sensitive, updatedAt: new Date() })
    .onConflictDoUpdate({
      target: systemSettings.key,
      set: { value, sensitive, updatedAt: new Date() },
    });

  // Invalidate cache
  cache.delete(key);
}

/**
 * Clear the in-memory settings cache (e.g., after saving new settings).
 */
export function clearProviderCache(): void {
  cache.clear();
}
