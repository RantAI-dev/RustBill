import { expect, test } from "@playwright/test";

test("@provider stripe setup session works with sandbox secret", async ({ request }) => {
  const stripeKey = process.env.STRIPE_SECRET_KEY;
  test.skip(!stripeKey, "STRIPE_SECRET_KEY not set");

  const providerSave = await request.put("/api/settings/payment-providers", {
    data: {
      provider: "stripe",
      settings: {
        secretKey: stripeKey,
        webhookSecret: process.env.STRIPE_WEBHOOK_SECRET ?? "",
      },
    },
  });
  expect(providerSave.ok()).toBeTruthy();

  const customersRes = await request.get("/api/customers");
  expect(customersRes.ok()).toBeTruthy();
  const customers = (await customersRes.json()) as Array<Record<string, unknown>>;

  let customerId = customers[0]?.id as string | undefined;
  if (!customerId) {
    const createRes = await request.post("/api/customers", {
      data: {
        name: "E2E Sandbox Customer",
        industry: "Software",
        tier: "Growth",
        location: "Jakarta",
        contact: "Sandbox",
        email: "e2e-sandbox@example.com",
        phone: "+620000000",
      },
    });
    expect(createRes.ok()).toBeTruthy();
    const created = (await createRes.json()) as Record<string, unknown>;
    customerId = created.id as string;
  }

  const setupRes = await request.post("/api/billing/payment-methods/setup", {
    data: {
      customerId,
      provider: "stripe",
      successUrl: "https://example.com/success",
      cancelUrl: "https://example.com/cancel",
    },
  });

  expect(setupRes.ok()).toBeTruthy();
  const body = (await setupRes.json()) as Record<string, unknown>;
  const setupUrl = body.setupUrl as string | undefined;
  const actions = body.actions as Array<{ url?: string }> | undefined;
  expect(Boolean(setupUrl || actions?.find((a) => a.url))).toBeTruthy();
});
