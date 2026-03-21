import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";
import { SWRConfig } from "swr";
import { http, HttpResponse } from "msw";
import { server } from "../setup";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

import { toast } from "sonner";
import {
  ApiProvider,
  adjustCredits,
  createDeal,
  createLicense,
  createOneTimeSale,
  createProduct,
  createSubscription,
  createUsageEvent,
  deactivateDevice,
  deleteCreditAdjustment,
  deleteDeal,
  deleteLicense,
  deleteOneTimeSale,
  deleteProduct,
  deleteSubscription,
  deleteUsageEvent,
  generateKeypair,
  getCheckout,
  runSales360Backfill,
  signLicenseKey,
  updateCreditAdjustment,
  updateDeal,
  updateLicense,
  updateOneTimeSale,
  updateSubscription,
  updateUsageEvent,
  useCustomerCredits,
  useLicenses,
  useOneTimeSales,
  useSubscriptions,
  useUsageEvents,
  verifyLicenseFile,
} from "@/hooks/use-api";

type MutationCase = {
  name: string;
  method: "post" | "put" | "delete";
  matcher: string;
  expectedPath: string;
  invoke: () => Promise<unknown>;
  successResponse: unknown;
  expectedBody?: Record<string, unknown>;
  expectedQuery?: Record<string, string>;
  failureStatus: number;
  failureError: string;
  fallbackError: string;
};

function createWrapper() {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return React.createElement(
      ApiProvider,
      null,
      React.createElement(
        SWRConfig,
        { value: { provider: () => new Map(), dedupingInterval: 0 } },
        children,
      ),
    );
  };
}

function registerRoute(
  method: MutationCase["method"],
  matcher: string,
  resolver: Parameters<typeof http.post>[1],
) {
  switch (method) {
    case "post":
      server.use(http.post(matcher, resolver));
      break;
    case "put":
      server.use(http.put(matcher, resolver));
      break;
    case "delete":
      server.use(http.delete(matcher, resolver));
      break;
  }
}

const mutationCases: MutationCase[] = [
  {
    name: "createOneTimeSale",
    method: "post",
    matcher: "/api/billing/one-time-sales",
    expectedPath: "/api/billing/one-time-sales",
    invoke: () => createOneTimeSale({ customerId: "cust-1", currency: "USD", subtotal: 100, tax: 10, total: 110 }),
    successResponse: { id: "sale-1", status: "issued" },
    expectedBody: { customerId: "cust-1", currency: "USD", subtotal: 100, tax: 10, total: 110 },
    failureStatus: 422,
    failureError: "Invalid one-time sale",
    fallbackError: "Failed to create one-time sale",
  },
  {
    name: "updateOneTimeSale",
    method: "put",
    matcher: "/api/billing/one-time-sales/:id",
    expectedPath: "/api/billing/one-time-sales/sale-1",
    invoke: () => updateOneTimeSale("sale-1", { status: "void", notes: "Customer canceled" }),
    successResponse: { id: "sale-1", status: "void" },
    expectedBody: { status: "void", notes: "Customer canceled" },
    failureStatus: 409,
    failureError: "One-time sale cannot be updated",
    fallbackError: "Failed to update one-time sale",
  },
  {
    name: "deleteOneTimeSale",
    method: "delete",
    matcher: "/api/billing/one-time-sales/:id",
    expectedPath: "/api/billing/one-time-sales/sale-1",
    invoke: () => deleteOneTimeSale("sale-1"),
    successResponse: { success: true },
    failureStatus: 404,
    failureError: "One-time sale not found",
    fallbackError: "Failed to delete one-time sale",
  },
  {
    name: "createSubscription",
    method: "post",
    matcher: "/api/billing/subscriptions",
    expectedPath: "/api/billing/subscriptions",
    invoke: () => createSubscription({ customerId: "cust-1", planId: "plan-1", status: "active" }),
    successResponse: { id: "sub-1", status: "active" },
    expectedBody: { customerId: "cust-1", planId: "plan-1", status: "active" },
    failureStatus: 422,
    failureError: "Invalid subscription",
    fallbackError: "Failed to create subscription",
  },
  {
    name: "updateSubscription",
    method: "put",
    matcher: "/api/billing/subscriptions/:id",
    expectedPath: "/api/billing/subscriptions/sub-1",
    invoke: () => updateSubscription("sub-1", { cancelAtPeriodEnd: true }),
    successResponse: { id: "sub-1", cancelAtPeriodEnd: true },
    expectedBody: { cancelAtPeriodEnd: true },
    failureStatus: 409,
    failureError: "Subscription update conflict",
    fallbackError: "Failed to update subscription",
  },
  {
    name: "deleteSubscription",
    method: "delete",
    matcher: "/api/billing/subscriptions/:id",
    expectedPath: "/api/billing/subscriptions/sub-1",
    invoke: () => deleteSubscription("sub-1"),
    successResponse: { success: true },
    failureStatus: 404,
    failureError: "Subscription not found",
    fallbackError: "Failed to delete subscription",
  },
  {
    name: "createUsageEvent",
    method: "post",
    matcher: "/api/billing/usage",
    expectedPath: "/api/billing/usage",
    invoke: () => createUsageEvent({ subscriptionId: "sub-1", metricName: "api_calls", value: 12 }),
    successResponse: { id: "usage-1" },
    expectedBody: { subscriptionId: "sub-1", metricName: "api_calls", value: 12 },
    failureStatus: 422,
    failureError: "Invalid usage event",
    fallbackError: "Failed to record usage event",
  },
  {
    name: "updateUsageEvent",
    method: "put",
    matcher: "/api/billing/usage/:id",
    expectedPath: "/api/billing/usage/usage-1",
    invoke: () => updateUsageEvent("usage-1", { value: 15 }),
    successResponse: { id: "usage-1", value: 15 },
    expectedBody: { value: 15 },
    failureStatus: 409,
    failureError: "Usage event conflict",
    fallbackError: "Failed to update usage event",
  },
  {
    name: "deleteUsageEvent",
    method: "delete",
    matcher: "/api/billing/usage/:id",
    expectedPath: "/api/billing/usage/usage-1",
    invoke: () => deleteUsageEvent("usage-1"),
    successResponse: { success: true },
    failureStatus: 404,
    failureError: "Usage event not found",
    fallbackError: "Failed to delete usage event",
  },
  {
    name: "adjustCredits",
    method: "post",
    matcher: "/api/billing/credits/adjust",
    expectedPath: "/api/billing/credits/adjust",
    invoke: () => adjustCredits({ customerId: "cust-1", currency: "USD", amount: 25, description: "Manual credit" }),
    successResponse: { id: "credit-1" },
    expectedBody: { customerId: "cust-1", currency: "USD", amount: 25, description: "Manual credit" },
    failureStatus: 422,
    failureError: "Invalid credit adjustment",
    fallbackError: "Failed to adjust credits",
  },
  {
    name: "updateCreditAdjustment",
    method: "put",
    matcher: "/api/billing/credits/adjust/:id",
    expectedPath: "/api/billing/credits/adjust/credit-1",
    invoke: () => updateCreditAdjustment("credit-1", { amount: 15, description: "Revised" }),
    successResponse: { id: "credit-1", amount: 15 },
    expectedBody: { amount: 15, description: "Revised" },
    failureStatus: 409,
    failureError: "Credit adjustment conflict",
    fallbackError: "Failed to update credit adjustment",
  },
  {
    name: "deleteCreditAdjustment",
    method: "delete",
    matcher: "/api/billing/credits/adjust/:id",
    expectedPath: "/api/billing/credits/adjust/credit-1",
    invoke: () => deleteCreditAdjustment("credit-1"),
    successResponse: { success: true },
    failureStatus: 404,
    failureError: "Credit adjustment not found",
    fallbackError: "Failed to delete credit adjustment",
  },
  {
    name: "createLicense",
    method: "post",
    matcher: "/api/licenses",
    expectedPath: "/api/licenses",
    invoke: () => createLicense({ customerId: "cust-1", productId: "prod-1", licenseType: "simple" }),
    successResponse: { key: "LIC-1" },
    expectedBody: { customerId: "cust-1", productId: "prod-1", licenseType: "simple" },
    failureStatus: 422,
    failureError: "Invalid license payload",
    fallbackError: "Failed to create license",
  },
  {
    name: "updateLicense",
    method: "put",
    matcher: "/api/licenses/:key",
    expectedPath: "/api/licenses/LIC-1",
    invoke: () => updateLicense("LIC-1", { status: "suspended" }),
    successResponse: { key: "LIC-1", status: "suspended" },
    expectedBody: { status: "suspended" },
    failureStatus: 409,
    failureError: "License update conflict",
    fallbackError: "Failed to update license",
  },
  {
    name: "deleteLicense",
    method: "delete",
    matcher: "/api/licenses/:key",
    expectedPath: "/api/licenses/LIC-1",
    invoke: () => deleteLicense("LIC-1"),
    successResponse: { success: true },
    failureStatus: 404,
    failureError: "License not found",
    fallbackError: "Failed to delete license",
  },
  {
    name: "signLicenseKey",
    method: "post",
    matcher: "/api/licenses/:key/sign",
    expectedPath: "/api/licenses/LIC-1/sign",
    invoke: () => signLicenseKey("LIC-1", { features: ["pro"], maxActivations: 3, metadata: { source: "test" } }),
    successResponse: { success: true, signature: "sig" },
    expectedBody: { features: ["pro"], maxActivations: 3, metadata: { source: "test" } },
    failureStatus: 409,
    failureError: "License signing conflict",
    fallbackError: "Failed to sign license",
  },
  {
    name: "deactivateDevice",
    method: "delete",
    matcher: "/api/licenses/:key/activations",
    expectedPath: "/api/licenses/LIC-1/activations",
    invoke: () => deactivateDevice("LIC-1", "device-1"),
    successResponse: { success: true },
    expectedQuery: { deviceId: "device-1" },
    failureStatus: 404,
    failureError: "Activation not found",
    fallbackError: "Failed to deactivate device",
  },
  {
    name: "verifyLicenseFile",
    method: "post",
    matcher: "/api/licenses/verify",
    expectedPath: "/api/licenses/verify",
    invoke: () => verifyLicenseFile("SIGNED_LICENSE_FILE"),
    successResponse: { valid: true, expired: false, payload: { key: "LIC-1" } },
    expectedBody: { licenseFile: "SIGNED_LICENSE_FILE" },
    failureStatus: 422,
    failureError: "Invalid license file",
    fallbackError: "Failed to verify license",
  },
];

beforeEach(() => {
  vi.clearAllMocks();
});

describe("use-api mutation helpers", () => {
  it("createProduct returns success on 200", async () => {
    const product = { id: "p1", name: "Widget", productType: "licensed" };
    server.use(
      http.post("/api/products", () => HttpResponse.json(product, { status: 200 })),
    );

    const result = await createProduct({ name: "Widget", productType: "licensed" });
    expect(result).toEqual({ success: true, data: product });
  });

  it("createProduct returns error on 400", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.json({ error: "Invalid data" }, { status: 400 })),
    );

    const result = await createProduct({ name: "" });
    expect(result).toEqual({
      success: false,
      error: "Invalid data",
      status: 400,
    });
  });

  it("deleteProduct calls correct URL", async () => {
    let calledUrl = "";
    server.use(
      http.delete("/api/products/:id", ({ request }) => {
        calledUrl = new URL(request.url).pathname;
        return HttpResponse.json({ deleted: true });
      }),
    );

    const result = await deleteProduct("abc-123");
    expect(calledUrl).toBe("/api/products/abc-123");
    expect(result.success).toBe(true);
  });

  it("mutation shows toast on error", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.json({ error: "Server error" }, { status: 500 })),
    );

    await createProduct({ name: "fail" });
    expect(toast.error).toHaveBeenCalledWith("Server error");
  });

  it("getCheckout returns structured result", async () => {
    const checkoutData = { checkoutUrl: "https://pay.example.com/sess_1", provider: "stripe" };
    server.use(
      http.get("/api/billing/checkout", () => HttpResponse.json(checkoutData, { status: 200 })),
    );

    const result = await getCheckout("inv-1", "stripe");
    expect(result).toEqual({ success: true, data: checkoutData });
  });

  it("generateKeypair preserves status code on 409", async () => {
    server.use(
      http.post("/api/licenses/keypair", () => HttpResponse.json({ error: "Keypair already exists" }, { status: 409 })),
    );

    const result = await generateKeypair();
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.status).toBe(409);
      expect(result.error).toBe("Keypair already exists");
    }
  });

  it("fetcher throws on non-200 via mutate path", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.json({ error: "Not found" }, { status: 404 })),
    );

    const result = await createProduct({ name: "test" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.status).toBe(404);
    }
  });

  it("mutation handles network error", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.error()),
    );

    const result = await createProduct({ name: "test" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("Failed to create product");
      expect(result.status).toBeUndefined();
    }
    expect(toast.error).toHaveBeenCalledWith("Failed to create product");
  });

  it("mutation uses fallback error when response has no error field", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.json({}, { status: 422 })),
    );

    const result = await createProduct({ name: "x" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("Failed to create product");
      expect(result.status).toBe(422);
    }
  });

  it("mutation toast shows error from response body", async () => {
    server.use(
      http.post("/api/products", () => HttpResponse.json({ error: "Duplicate product name" }, { status: 409 })),
    );

    await createProduct({ name: "dup" });
    expect(toast.error).toHaveBeenCalledWith("Duplicate product name");
  });

  it("createDeal sends normalized sales payload", async () => {
    let requestBody: Record<string, unknown> | null = null;
    server.use(
      http.post("/api/deals", async ({ request }) => {
        requestBody = (await request.json()) as Record<string, unknown>;
        return HttpResponse.json({ id: "deal-1" }, { status: 200 });
      }),
    );

    const result = await createDeal({
      customerId: "cust-1",
      productId: "prod-1",
      productType: "saas",
      dealType: "trial",
      value: 15000,
      usageMetricLabel: "api_calls",
      usageMetricValue: 120,
      autoCreateInvoice: true,
    });

    expect(result.success).toBe(true);
    expect(requestBody).toMatchObject({
      customerId: "cust-1",
      customer_id: "cust-1",
      productId: "prod-1",
      product_id: "prod-1",
      productType: "saas",
      product_type: "saas",
      dealType: "trial",
      deal_type: "trial",
      value: "15000",
      usageMetricLabel: "api_calls",
      usage_metric_label: "api_calls",
      usageMetricValue: 120,
      usage_metric_value: 120,
      autoCreateInvoice: true,
      auto_create_invoice: true,
    });
  });

  it("updateDeal calls the deal update endpoint", async () => {
    let calledPath = "";
    server.use(
      http.put("/api/deals/:id", ({ request }) => {
        calledPath = new URL(request.url).pathname;
        return HttpResponse.json({ ok: true }, { status: 200 });
      }),
    );

    const result = await updateDeal("deal-42", { notes: "updated" });

    expect(result.success).toBe(true);
    expect(calledPath).toBe("/api/deals/deal-42");
  });

  it("deleteDeal calls the deal delete endpoint", async () => {
    let calledPath = "";
    server.use(
      http.delete("/api/deals/:id", ({ request }) => {
        calledPath = new URL(request.url).pathname;
        return HttpResponse.json({ deleted: true }, { status: 200 });
      }),
    );

    const result = await deleteDeal("deal-9");

    expect(result.success).toBe(true);
    expect(calledPath).toBe("/api/deals/deal-9");
  });

  it("runSales360Backfill calls analytics backfill endpoint", async () => {
    let called = false;
    server.use(
      http.post("/api/analytics/sales-360/backfill", () => {
        called = true;
        return HttpResponse.json({ success: true }, { status: 200 });
      }),
    );

    const result = await runSales360Backfill();

    expect(called).toBe(true);
    expect(result.success).toBe(true);
  });

  for (const testCase of mutationCases) {
    it(`${testCase.name} returns success and sends the correct request`, async () => {
      let requestBody: unknown;
      let requestPath = "";
      let requestQuery: URLSearchParams | null = null;

      registerRoute(testCase.method, testCase.matcher, async ({ request }) => {
        requestPath = new URL(request.url).pathname;
        requestQuery = new URL(request.url).searchParams;
        if (testCase.expectedBody) {
          requestBody = await request.json();
        }
        return HttpResponse.json(testCase.successResponse, { status: 200 });
      });

      const result = await testCase.invoke();

      expect(result).toEqual({ success: true, data: testCase.successResponse });
      expect(requestPath).toBe(testCase.expectedPath);
      if (testCase.expectedBody) {
        expect(requestBody).toEqual(testCase.expectedBody);
      }
      if (testCase.expectedQuery) {
        expect(Object.fromEntries(requestQuery?.entries() ?? [])).toEqual(testCase.expectedQuery);
      }
    });

    it(`${testCase.name} propagates validation and business failures`, async () => {
      registerRoute(testCase.method, testCase.matcher, () =>
        HttpResponse.json({ error: testCase.failureError }, { status: testCase.failureStatus }),
      );

      const result = await testCase.invoke();

      expect(result).toEqual({
        success: false,
        error: testCase.failureError,
        status: testCase.failureStatus,
      });
      expect(toast.error).toHaveBeenCalledWith(testCase.failureError);
    });

    it(`${testCase.name} handles network failures`, async () => {
      registerRoute(testCase.method, testCase.matcher, () => HttpResponse.error());

      const result = await testCase.invoke();

      expect(result).toEqual({
        success: false,
        error: testCase.fallbackError,
      });
      expect(toast.error).toHaveBeenCalledWith(testCase.fallbackError);
    });
  }
});

describe("use-api read hooks", () => {
  it("useOneTimeSales returns normalized one-time sales", async () => {
    const wrapper = createWrapper();
    server.use(
      http.get("/api/billing/one-time-sales", () =>
        HttpResponse.json([
          {
            id: "sale-1",
            invoice_number: "INV-00000001",
            customer_id: "cust-1",
            customer_name: "Acme",
            subscription_id: null,
            subtotal: 100,
            tax: 10,
            total: 110,
            created_at: "2026-03-01T00:00:00Z",
          },
        ]),
      ),
    );

    const { result } = renderHook(() => useOneTimeSales(), { wrapper });

    await waitFor(() => expect(result.current.data).toHaveLength(1));
    expect(result.current.data?.[0]).toMatchObject({
      id: "sale-1",
      invoiceNumber: "INV-00000001",
      customerId: "cust-1",
      customerName: "Acme",
      total: 110,
    });
  });

  it("useSubscriptions returns normalized subscriptions", async () => {
    const wrapper = createWrapper();
    server.use(
      http.get("/api/billing/subscriptions", () =>
        HttpResponse.json([
          {
            id: "sub-1",
            customer_id: "cust-1",
            customer_name: "Acme",
            plan_id: "plan-1",
            plan_name: "Growth",
            cancel_at_period_end: true,
            created_at: "2026-03-01T00:00:00Z",
          },
        ]),
      ),
    );

    const { result } = renderHook(() => useSubscriptions(), { wrapper });

    await waitFor(() => expect(result.current.data).toHaveLength(1));
    expect(result.current.data?.[0]).toMatchObject({
      id: "sub-1",
      customerId: "cust-1",
      customerName: "Acme",
      planId: "plan-1",
      planName: "Growth",
      cancelAtPeriodEnd: true,
    });
  });

  it("useUsageEvents returns usage rows for an active subscription", async () => {
    const wrapper = createWrapper();
    server.use(
      http.get("/api/billing/usage", () =>
        HttpResponse.json([
          {
            id: "usage-1",
            subscriptionId: "sub-1",
            metricName: "api_calls",
            value: 15,
          },
        ]),
      ),
    );

    const { result } = renderHook(() => useUsageEvents("sub-1"), { wrapper });

    await waitFor(() => expect(result.current.data).toHaveLength(1));
    expect(result.current.data?.[0]).toMatchObject({
      id: "usage-1",
      subscriptionId: "sub-1",
      metricName: "api_calls",
      value: 15,
    });
  });

  it("useUsageEvents does not fetch when subscriptionId is empty", async () => {
    const wrapper = createWrapper();
    let requested = false;
    server.use(
      http.get("/api/billing/usage", () => {
        requested = true;
        return HttpResponse.json([]);
      }),
    );

    const { result } = renderHook(() => useUsageEvents(""), { wrapper });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.data).toBeUndefined();
    expect(requested).toBe(false);
  });

  it("useCustomerCredits returns credit data", async () => {
    const wrapper = createWrapper();
    server.use(
      http.get("/api/billing/credits/:customerId", () =>
        HttpResponse.json({
          balance: 25,
          history: [{ id: "credit-1", amount: 25, reason: "manual" }],
        }),
      ),
    );

    const { result } = renderHook(() => useCustomerCredits("cust-1"), { wrapper });

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.data).toMatchObject({
      balance: 25,
      history: [{ id: "credit-1", amount: 25, reason: "manual" }],
    });
  });

  it("useCustomerCredits does not fetch when customerId is missing", async () => {
    const wrapper = createWrapper();
    let requested = false;
    server.use(
      http.get("/api/billing/credits/:customerId", () => {
        requested = true;
        return HttpResponse.json({ balance: 0, history: [] });
      }),
    );

    const { result } = renderHook(() => useCustomerCredits(undefined), { wrapper });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.data).toBeUndefined();
    expect(requested).toBe(false);
  });

  it("useLicenses returns normalized license rows", async () => {
    const wrapper = createWrapper();
    server.use(
      http.get("/api/licenses", () =>
        HttpResponse.json([
          {
            key: "LIC-1",
            customer_id: "cust-1",
            customer_name: "Acme",
            product_id: "prod-1",
            product_name: "Pro",
            license_type: "signed",
            max_activations: 3,
            created_at: "2026-03-01T00:00:00Z",
            expires_at: "2027-03-01T00:00:00Z",
            signed_payload: "payload",
            signature: "signature",
          },
        ]),
      ),
    );

    const { result } = renderHook(() => useLicenses(), { wrapper });

    await waitFor(() => expect(result.current.data).toHaveLength(1));
    expect(result.current.data?.[0]).toMatchObject({
      key: "LIC-1",
      customerId: "cust-1",
      customerName: "Acme",
      productId: "prod-1",
      productName: "Pro",
      licenseType: "signed",
      maxActivations: 3,
      hasCertificate: true,
    });
  });
});
