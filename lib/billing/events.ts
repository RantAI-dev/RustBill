import { db } from "@/lib/db";
import { billingEvents, webhookEndpoints, webhookDeliveries } from "@/lib/db/schema";
import { eq } from "drizzle-orm";

type BillingEventType =
  | "invoice.created" | "invoice.issued" | "invoice.paid" | "invoice.overdue" | "invoice.voided"
  | "payment.received" | "payment.refunded"
  | "subscription.created" | "subscription.renewed" | "subscription.canceled" | "subscription.paused"
  | "dunning.reminder" | "dunning.warning" | "dunning.final_notice" | "dunning.suspension";

interface EmitOptions {
  eventType: BillingEventType;
  resourceType: string;
  resourceId: string;
  customerId?: string | null;
  data?: Record<string, unknown>;
}

const MAX_ATTEMPTS = 3;
const BACKOFF_BASE_MS = 1000; // 1s, 4s, 16s

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Emit a billing event — logs it and dispatches to any matching webhook endpoints.
 */
export async function emitBillingEvent(opts: EmitOptions) {
  // 1. Log the event
  const [event] = await db.insert(billingEvents).values({
    eventType: opts.eventType,
    resourceType: opts.resourceType,
    resourceId: opts.resourceId,
    customerId: opts.customerId ?? null,
    data: opts.data ?? null,
  }).returning();

  // 2. Find matching webhook endpoints
  const endpoints = await db
    .select()
    .from(webhookEndpoints)
    .where(eq(webhookEndpoints.status, "active"));

  const matchingEndpoints = endpoints.filter((ep) => {
    const subscribed = ep.events as string[];
    return subscribed.includes(opts.eventType) || subscribed.includes("*");
  });

  // 3. Dispatch to each endpoint with retry (non-blocking)
  for (const endpoint of matchingEndpoints) {
    dispatchWebhookWithRetry(endpoint, event).catch((err) => {
      console.error(`Webhook dispatch failed for endpoint ${endpoint.id}:`, err);
    });
  }

  return event;
}

async function dispatchWebhookWithRetry(
  endpoint: typeof webhookEndpoints.$inferSelect,
  event: typeof billingEvents.$inferSelect,
) {
  const payload = {
    id: event.id,
    type: event.eventType,
    resourceType: event.resourceType,
    resourceId: event.resourceId,
    data: event.data,
    createdAt: event.createdAt.toISOString(),
  };

  // Create delivery record
  const [delivery] = await db.insert(webhookDeliveries).values({
    endpointId: endpoint.id,
    eventId: event.id,
    payload,
    attempts: 0,
  }).returning();

  const body = JSON.stringify(payload);
  const signature = await computeHmac(body, endpoint.secret);

  for (let attempt = 1; attempt <= MAX_ATTEMPTS; attempt++) {
    try {
      const response = await fetch(endpoint.url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "X-Webhook-Signature": signature,
          "X-Webhook-Event": event.eventType,
          "X-Webhook-Id": event.id,
        },
        body,
        signal: AbortSignal.timeout(10000),
      });

      await db.update(webhookDeliveries).set({
        responseCode: response.status,
        responseBody: (await response.text()).slice(0, 1000),
        attempts: attempt,
        deliveredAt: response.ok ? new Date() : null,
      }).where(eq(webhookDeliveries.id, delivery.id));

      if (response.ok) return; // Success

      // Non-retryable client errors (except 429)
      if (response.status >= 400 && response.status < 500 && response.status !== 429) {
        return;
      }
    } catch (err) {
      await db.update(webhookDeliveries).set({
        responseCode: 0,
        responseBody: err instanceof Error ? err.message : "Unknown error",
        attempts: attempt,
      }).where(eq(webhookDeliveries.id, delivery.id));
    }

    // Exponential backoff before next attempt
    if (attempt < MAX_ATTEMPTS) {
      await sleep(BACKOFF_BASE_MS * Math.pow(4, attempt - 1));
    }
  }
}

async function computeHmac(data: string, secret: string): Promise<string> {
  const encoder = new TextEncoder();
  const key = await crypto.subtle.importKey(
    "raw", encoder.encode(secret), { name: "HMAC", hash: "SHA-256" }, false, ["sign"],
  );
  const signature = await crypto.subtle.sign("HMAC", key, encoder.encode(data));
  return Array.from(new Uint8Array(signature)).map((b) => b.toString(16).padStart(2, "0")).join("");
}
