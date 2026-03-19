import { db } from "@/lib/db";
import { systemSettings } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

export const PROVIDER_KEYS = {
  STRIPE_SECRET_KEY: "stripe_secret_key",
  STRIPE_WEBHOOK_SECRET: "stripe_webhook_secret",
  XENDIT_SECRET_KEY: "xendit_secret_key",
  XENDIT_WEBHOOK_TOKEN: "xendit_webhook_token",
  LEMONSQUEEZY_API_KEY: "lemonsqueezy_api_key",
  LEMONSQUEEZY_STORE_ID: "lemonsqueezy_store_id",
  LEMONSQUEEZY_WEBHOOK_SECRET: "lemonsqueezy_webhook_secret",
  EXTERNAL_TAX_PROVIDER: "external_tax_provider",
  TAXJAR_API_KEY: "taxjar_api_key",
} as const;

type ProviderKey = (typeof PROVIDER_KEYS)[keyof typeof PROVIDER_KEYS];

const cache = new Map<string, { value: string; expiresAt: number }>();
const CACHE_TTL = 60_000;

export async function getProviderSetting(key: ProviderKey): Promise<string> {
  const cached = cache.get(key);
  if (cached && cached.expiresAt > Date.now()) {
    return cached.value;
  }

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
    // no-op
  }

  const envKey = key.toUpperCase();
  return process.env[envKey] ?? "";
}

export async function getProviderStatus(): Promise<{
  stripe: { configured: boolean; secretKey: string; webhookSecret: string };
  xendit: { configured: boolean; secretKey: string; webhookToken: string };
  lemonsqueezy: { configured: boolean; apiKey: string; storeId: string; webhookSecret: string };
  tax: { configured: boolean; externalProvider: string; taxjarApiKey: string };
}> {
  const [
    stripeKey, stripeWebhook,
    xenditKey, xenditToken,
    lsKey, lsStore, lsWebhook,
    externalTaxProvider, taxjarApiKey,
  ] = await Promise.all([
    getProviderSetting(PROVIDER_KEYS.STRIPE_SECRET_KEY),
    getProviderSetting(PROVIDER_KEYS.STRIPE_WEBHOOK_SECRET),
    getProviderSetting(PROVIDER_KEYS.XENDIT_SECRET_KEY),
    getProviderSetting(PROVIDER_KEYS.XENDIT_WEBHOOK_TOKEN),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_API_KEY),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_STORE_ID),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_WEBHOOK_SECRET),
    getProviderSetting(PROVIDER_KEYS.EXTERNAL_TAX_PROVIDER),
    getProviderSetting(PROVIDER_KEYS.TAXJAR_API_KEY),
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
    tax: {
      configured: !!externalTaxProvider,
      externalProvider: externalTaxProvider || "",
      taxjarApiKey: maskValue(taxjarApiKey),
    },
  };
}

function maskValue(value: string): string {
  if (!value) return "";
  if (value.length <= 8) return "••••••••";
  return "••••••••" + value.slice(-4);
}

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

  cache.delete(key);
}

export function clearProviderCache(): void {
  cache.clear();
}
