import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import {
  getProviderStatus,
  saveProviderSetting,
  clearProviderCache,
  PROVIDER_KEYS,
} from "@/lib/billing/provider-settings";
import { NextRequest, NextResponse } from "next/server";

/**
 * GET /api/settings/payment-providers
 * Returns the configuration status of all payment providers (masked secrets).
 */
export async function GET() {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const status = await getProviderStatus();
    return NextResponse.json(status);
  } catch (error) {
    return handleApiError(error, "GET /api/settings/payment-providers");
  }
}

/**
 * PUT /api/settings/payment-providers
 * Save payment provider credentials.
 * Body: { provider: "stripe"|"xendit"|"lemonsqueezy", settings: { key: value } }
 * Empty string values are skipped (don't overwrite existing).
 */
export async function PUT(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const { provider, settings } = body as {
      provider: string;
      settings: Record<string, string>;
    };

    if (!provider || !settings) {
      return NextResponse.json(
        { error: "provider and settings are required" },
        { status: 400 },
      );
    }

    // Map provider + field to DB keys
    const keyMap: Record<string, Record<string, { key: string; sensitive: boolean }>> = {
      stripe: {
        secretKey: { key: PROVIDER_KEYS.STRIPE_SECRET_KEY, sensitive: true },
        webhookSecret: { key: PROVIDER_KEYS.STRIPE_WEBHOOK_SECRET, sensitive: true },
      },
      xendit: {
        secretKey: { key: PROVIDER_KEYS.XENDIT_SECRET_KEY, sensitive: true },
        webhookToken: { key: PROVIDER_KEYS.XENDIT_WEBHOOK_TOKEN, sensitive: true },
      },
      lemonsqueezy: {
        apiKey: { key: PROVIDER_KEYS.LEMONSQUEEZY_API_KEY, sensitive: true },
        storeId: { key: PROVIDER_KEYS.LEMONSQUEEZY_STORE_ID, sensitive: false },
        webhookSecret: { key: PROVIDER_KEYS.LEMONSQUEEZY_WEBHOOK_SECRET, sensitive: true },
      },
    };

    const providerMap = keyMap[provider];
    if (!providerMap) {
      return NextResponse.json(
        { error: `Unknown provider: ${provider}` },
        { status: 400 },
      );
    }

    for (const [field, value] of Object.entries(settings)) {
      const mapping = providerMap[field];
      if (!mapping) continue;
      // Skip empty values (don't overwrite existing config)
      if (!value) continue;
      await saveProviderSetting(
        mapping.key as (typeof PROVIDER_KEYS)[keyof typeof PROVIDER_KEYS],
        value,
        mapping.sensitive,
      );
    }

    clearProviderCache();

    const status = await getProviderStatus();
    return NextResponse.json(status);
  } catch (error) {
    return handleApiError(error, "PUT /api/settings/payment-providers");
  }
}
