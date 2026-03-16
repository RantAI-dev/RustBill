import crypto from "crypto";
import { getProviderSetting, PROVIDER_KEYS } from "./provider-settings";

const API_BASE = "https://api.lemonsqueezy.com/v1";

/**
 * Check if Lemonsqueezy is configured (has API key + store ID).
 */
export async function isLemonsqueezyEnabled(): Promise<boolean> {
  const [apiKey, storeId] = await Promise.all([
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_API_KEY),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_STORE_ID),
  ]);
  return !!apiKey && !!storeId;
}

/**
 * Create a Lemonsqueezy checkout for a one-time payment.
 * Returns the checkout URL, or null if Lemonsqueezy is not configured.
 */
export async function createLemonsqueezyCheckout(params: {
  invoiceId: string;
  invoiceNumber: string;
  total: number;
  currency: string;
  customerEmail?: string;
  customerName?: string;
  successUrl: string;
}): Promise<{ checkoutUrl: string; checkoutId: string } | null> {
  const [apiKey, storeId] = await Promise.all([
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_API_KEY),
    getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_STORE_ID),
  ]);
  if (!apiKey || !storeId) return null;

  // Lemonsqueezy expects amounts in cents
  const amountInCents = Math.round(params.total * 100);

  const response = await fetch(`${API_BASE}/checkouts`, {
    method: "POST",
    headers: {
      Accept: "application/vnd.api+json",
      "Content-Type": "application/vnd.api+json",
      Authorization: `Bearer ${apiKey}`,
    },
    body: JSON.stringify({
      data: {
        type: "checkouts",
        attributes: {
          custom_price: amountInCents,
          product_options: {
            name: `Invoice ${params.invoiceNumber}`,
            description: `Payment for invoice ${params.invoiceNumber}`,
            enabled_variants: [],
          },
          checkout_options: {
            embed: false,
          },
          checkout_data: {
            email: params.customerEmail,
            name: params.customerName,
            custom: {
              invoiceId: params.invoiceId,
            },
          },
          success_url: params.successUrl,
        },
        relationships: {
          store: {
            data: {
              type: "stores",
              id: storeId,
            },
          },
        },
      },
    }),
  });

  if (!response.ok) {
    const body = await response.text();
    throw new Error(`Lemonsqueezy checkout creation failed: ${response.status} ${body}`);
  }

  const json = await response.json();
  const checkoutUrl = json.data.attributes.url;
  const checkoutId = json.data.id;

  return { checkoutUrl, checkoutId };
}

/**
 * Verify Lemonsqueezy webhook signature using HMAC-SHA256.
 */
export async function verifyLemonsqueezyWebhook(
  rawBody: string,
  signature: string | null,
): Promise<boolean> {
  const webhookSecret = await getProviderSetting(PROVIDER_KEYS.LEMONSQUEEZY_WEBHOOK_SECRET);
  if (!webhookSecret || !signature) return false;

  const hmac = crypto.createHmac("sha256", webhookSecret);
  hmac.update(rawBody);
  const digest = hmac.digest("hex");

  try {
    return crypto.timingSafeEqual(
      Buffer.from(digest, "hex"),
      Buffer.from(signature, "hex"),
    );
  } catch {
    return false;
  }
}
