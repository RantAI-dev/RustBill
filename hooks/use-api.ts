import useSWR, { SWRConfiguration } from "swr";
import { SWRConfig } from "swr";
import React from "react";
import { toast } from "sonner";
import { BackendStatusProvider, useBackendStatus } from "@/components/backend-banner";

// ---- Fetch with timeout ----
async function fetchWithTimeout(url: string, init?: RequestInit, timeoutMs = 10000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

// ---- SWR fetcher ----
const fetcher = async (url: string) => {
  const res = await fetchWithTimeout(url);
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: "Request failed" }));
    const error = new Error(body.error ?? `Request failed with status ${res.status}`);
    (error as unknown as Record<string, unknown>).status = res.status;
    throw error;
  }
  return res.json();
};

// ---- Mutation result type ----
type MutationResult<T = unknown> =
  | { success: true; data: T }
  | { success: false; error: string; status?: number };

async function mutate<T = unknown>(
  url: string,
  options: RequestInit,
  errorMessage: string,
): Promise<MutationResult<T>> {
  try {
    const res = await fetchWithTimeout(url, {
      headers: { "Content-Type": "application/json" },
      ...options,
    });
    if (!res.ok) {
      const body = await res.json().catch(() => ({ error: errorMessage }));
      const msg = body.error ?? errorMessage;
      toast.error(msg);
      return { success: false, error: msg, status: res.status };
    }
    const data = await res.json();
    return { success: true, data };
  } catch (err) {
    const msg = err instanceof Error && err.name === "AbortError"
      ? "Request timed out"
      : errorMessage;
    toast.error(msg);
    return { success: false, error: msg };
  }
}

// ---- SWR global config ----
function useAppSWRConfig(): SWRConfiguration {
  const { setBackendDown, clearBackendDown } = useBackendStatus();

  return {
    fetcher,
    onErrorRetry(error, _key, _config, revalidate, { retryCount }) {
      if ((error as Record<string, unknown>).status === 401) return;
      if ((error as Record<string, unknown>).status === 404) return;
      if (retryCount >= 3) {
        setBackendDown(true);
        return;
      }
      const delay = Math.min(1000 * 2 ** retryCount, 10000);
      setTimeout(() => revalidate({ retryCount }), delay);
    },
    onSuccess() {
      clearBackendDown();
    },
  };
}

// ---- API Provider (wrap your app with this) ----
export function ApiProvider({ children }: { children: React.ReactNode }) {
  return React.createElement(
    BackendStatusProvider,
    null,
    React.createElement(ApiProviderInner, null, children),
  );
}

function ApiProviderInner({ children }: { children: React.ReactNode }) {
  const config = useAppSWRConfig();
  return React.createElement(SWRConfig, { value: config }, children);
}

// ---- Products ----
export function useProducts() {
  return useSWR("/api/products", fetcher);
}
export async function createProduct(data: Record<string, unknown>) {
  return mutate("/api/products", { method: "POST", body: JSON.stringify(data) }, "Failed to create product");
}
export async function updateProduct(id: string, data: Record<string, unknown>) {
  return mutate(`/api/products/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update product");
}
export async function deleteProduct(id: string) {
  return mutate(`/api/products/${id}`, { method: "DELETE" }, "Failed to delete product");
}

// ---- Deals ----
export function useDeals() {
  return useSWR("/api/deals", fetcher);
}
export async function createDeal(data: Record<string, unknown>) {
  return mutate("/api/deals", { method: "POST", body: JSON.stringify(data) }, "Failed to create deal");
}
export async function updateDeal(id: string, data: Record<string, unknown>) {
  return mutate(`/api/deals/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update deal");
}
export async function deleteDeal(id: string) {
  return mutate(`/api/deals/${id}`, { method: "DELETE" }, "Failed to delete deal");
}

// ---- Customers ----
export function useCustomers() {
  return useSWR("/api/customers", fetcher);
}
export async function createCustomer(data: Record<string, unknown>) {
  return mutate("/api/customers", { method: "POST", body: JSON.stringify(data) }, "Failed to create customer");
}
export async function updateCustomer(id: string, data: Record<string, unknown>) {
  return mutate(`/api/customers/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update customer");
}
export async function deleteCustomer(id: string) {
  return mutate(`/api/customers/${id}`, { method: "DELETE" }, "Failed to delete customer");
}

// ---- Licenses ----
export function useLicenses() {
  return useSWR("/api/licenses", fetcher);
}
export async function createLicense(data: Record<string, unknown>) {
  return mutate("/api/licenses", { method: "POST", body: JSON.stringify(data) }, "Failed to create license");
}
export async function updateLicense(key: string, data: Record<string, unknown>) {
  return mutate(`/api/licenses/${key}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update license");
}
export async function deleteLicense(key: string) {
  return mutate(`/api/licenses/${key}`, { method: "DELETE" }, "Failed to delete license");
}

// ---- License Activations ----
export function useLicenseActivations(key: string | null) {
  return useSWR(key ? `/api/licenses/${key}/activations` : null, fetcher);
}
export async function deactivateDevice(key: string, deviceId: string) {
  return mutate(`/api/licenses/${key}/activations?deviceId=${encodeURIComponent(deviceId)}`, { method: "DELETE" }, "Failed to deactivate device");
}

// ---- License Signing ----
export function useKeypair() {
  return useSWR("/api/licenses/keypair", fetcher);
}
export async function generateKeypair(confirm?: boolean) {
  return mutate("/api/licenses/keypair", { method: "POST", body: JSON.stringify({ confirm }) }, "Failed to generate keypair");
}
export async function signLicenseKey(
  key: string,
  data: { features?: string[]; maxActivations?: number; metadata?: Record<string, unknown> },
) {
  return mutate(`/api/licenses/${key}/sign`, { method: "POST", body: JSON.stringify(data) }, "Failed to sign license");
}
export function getLicenseExportUrl(key: string) {
  return `/api/licenses/${key}/export`;
}
export async function verifyLicenseFile(licenseFile: string) {
  return mutate<{ valid: boolean; expired: boolean; payload: Record<string, unknown> | null; error?: string }>(
    "/api/licenses/verify",
    { method: "POST", body: JSON.stringify({ licenseFile }) },
    "Failed to verify license",
  );
}

// ---- API Keys ----
export function useApiKeys() {
  return useSWR("/api/api-keys", fetcher);
}
export async function createApiKey(data: { name: string }) {
  return mutate<{ key: string; id: string; name: string }>("/api/api-keys", { method: "POST", body: JSON.stringify(data) }, "Failed to create API key");
}
export async function revokeApiKey(id: string) {
  return mutate(`/api/api-keys/${id}`, { method: "DELETE" }, "Failed to revoke API key");
}

// ---- Pricing Plans ----
export function usePlans() {
  return useSWR("/api/billing/plans", fetcher);
}
export async function createPlan(data: Record<string, unknown>) {
  return mutate("/api/billing/plans", { method: "POST", body: JSON.stringify(data) }, "Failed to create plan");
}
export async function updatePlan(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/plans/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update plan");
}
export async function deletePlan(id: string) {
  return mutate(`/api/billing/plans/${id}`, { method: "DELETE" }, "Failed to delete plan");
}

// ---- Subscriptions ----
export function useSubscriptions() {
  return useSWR("/api/billing/subscriptions", fetcher);
}
export async function createSubscription(data: Record<string, unknown>) {
  return mutate("/api/billing/subscriptions", { method: "POST", body: JSON.stringify(data) }, "Failed to create subscription");
}
export async function updateSubscription(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/subscriptions/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update subscription");
}
export async function deleteSubscription(id: string) {
  return mutate(`/api/billing/subscriptions/${id}`, { method: "DELETE" }, "Failed to delete subscription");
}

// ---- Invoices ----
export function useInvoices() {
  return useSWR("/api/billing/invoices", fetcher);
}
export async function createInvoice(data: Record<string, unknown>) {
  return mutate("/api/billing/invoices", { method: "POST", body: JSON.stringify(data) }, "Failed to create invoice");
}
export async function updateInvoice(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/invoices/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update invoice");
}
export async function deleteInvoice(id: string) {
  return mutate(`/api/billing/invoices/${id}`, { method: "DELETE" }, "Failed to delete invoice");
}
export async function addInvoiceItem(invoiceId: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/invoices/${invoiceId}/items`, { method: "POST", body: JSON.stringify(data) }, "Failed to add invoice item");
}

// ---- Payments ----
export function usePayments(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/payments?invoiceId=${invoiceId}` : "/api/billing/payments";
  return useSWR(url, fetcher);
}
export async function createPayment(data: Record<string, unknown>) {
  return mutate("/api/billing/payments", { method: "POST", body: JSON.stringify(data) }, "Failed to record payment");
}

// ---- Usage Events ----
export function useUsageEvents(subscriptionId: string) {
  return useSWR(subscriptionId ? `/api/billing/usage?subscriptionId=${subscriptionId}` : null, fetcher);
}

// ---- Credit Notes ----
export function useCreditNotes(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/credit-notes?invoiceId=${invoiceId}` : "/api/billing/credit-notes";
  return useSWR(url, fetcher);
}
export async function createCreditNote(data: Record<string, unknown>) {
  return mutate("/api/billing/credit-notes", { method: "POST", body: JSON.stringify(data) }, "Failed to create credit note");
}
export async function updateCreditNote(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/credit-notes/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update credit note");
}
export async function deleteCreditNote(id: string) {
  return mutate(`/api/billing/credit-notes/${id}`, { method: "DELETE" }, "Failed to delete credit note");
}

// ---- Coupons ----
export function useCoupons() {
  return useSWR("/api/billing/coupons", fetcher);
}
export async function createCoupon(data: Record<string, unknown>) {
  return mutate("/api/billing/coupons", { method: "POST", body: JSON.stringify(data) }, "Failed to create coupon");
}
export async function updateCoupon(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/coupons/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update coupon");
}
export async function deleteCoupon(id: string) {
  return mutate(`/api/billing/coupons/${id}`, { method: "DELETE" }, "Failed to delete coupon");
}
export async function applyCoupon(couponId: string, subscriptionId: string, expiresAt?: string) {
  return mutate(`/api/billing/coupons/${couponId}`, {
    method: "PUT",
    body: JSON.stringify({ action: "apply", subscriptionId, couponId, expiresAt }),
  }, "Failed to apply coupon");
}

// ---- Subscription Lifecycle ----
export async function runBillingLifecycle() {
  return mutate("/api/billing/subscriptions/lifecycle", { method: "POST" }, "Failed to run billing lifecycle");
}

// ---- Refunds ----
export function useRefunds(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/refunds?invoiceId=${invoiceId}` : "/api/billing/refunds";
  return useSWR(url, fetcher);
}
export async function createRefund(data: Record<string, unknown>) {
  return mutate("/api/billing/refunds", { method: "POST", body: JSON.stringify(data) }, "Failed to process refund");
}

// ---- Dunning ----
export function useDunningLog(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/dunning?invoiceId=${invoiceId}` : "/api/billing/dunning";
  return useSWR(url, fetcher);
}
export async function runDunning(config?: Record<string, number>) {
  return mutate("/api/billing/dunning", { method: "POST", body: JSON.stringify(config ?? {}) }, "Failed to run dunning");
}

// ---- Stripe ----
export async function getStripeCheckout(invoiceId: string) {
  return mutate(`/api/billing/stripe?invoiceId=${invoiceId}`, { method: "GET" }, "Failed to get Stripe checkout");
}

// ---- Unified Checkout ----
export async function getCheckout(invoiceId: string, provider: "stripe" | "xendit" | "lemonsqueezy") {
  return mutate<{ checkoutUrl: string; provider: string }>(
    `/api/billing/checkout?invoiceId=${invoiceId}&provider=${provider}`,
    { method: "GET" },
    "Checkout failed",
  );
}

// ---- Billing Events ----
export function useBillingEvents(customerId?: string, limit?: number) {
  const params = new URLSearchParams();
  if (customerId) params.set("customerId", customerId);
  if (limit) params.set("limit", String(limit));
  const qs = params.toString();
  return useSWR(`/api/billing/events${qs ? `?${qs}` : ""}`, fetcher);
}

// ---- Webhooks ----
export function useWebhooks() {
  return useSWR("/api/billing/webhooks", fetcher);
}
export function useWebhook(id: string) {
  return useSWR(id ? `/api/billing/webhooks/${id}` : null, fetcher);
}
export async function createWebhook(data: Record<string, unknown>) {
  return mutate("/api/billing/webhooks", { method: "POST", body: JSON.stringify(data) }, "Failed to create webhook");
}
export async function updateWebhook(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/webhooks/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update webhook");
}
export async function deleteWebhook(id: string) {
  return mutate(`/api/billing/webhooks/${id}`, { method: "DELETE" }, "Failed to delete webhook");
}

// ---- PDF ----
export function getInvoicePdfUrl(invoiceId: string) {
  return `/api/billing/invoices/${invoiceId}/pdf`;
}

// ---- Analytics ----
export function useOverviewAnalytics() {
  return useSWR("/api/analytics/overview", fetcher);
}
export function useForecastAnalytics() {
  return useSWR("/api/analytics/forecasting", fetcher);
}
export function useReportsAnalytics() {
  return useSWR("/api/analytics/reports", fetcher);
}

// ---- Tax Rules ----
export function useTaxRules() {
  return useSWR("/api/billing/tax-rules", fetcher);
}
export async function createTaxRule(data: Record<string, unknown>) {
  return mutate("/api/billing/tax-rules", { method: "POST", body: JSON.stringify(data) }, "Failed to create tax rule");
}
export async function updateTaxRule(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/tax-rules/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update tax rule");
}
export async function deleteTaxRule(id: string) {
  return mutate(`/api/billing/tax-rules/${id}`, { method: "DELETE" }, "Failed to delete tax rule");
}

// ---- Credits ----
export function useCustomerCredits(customerId: string | undefined) {
  return useSWR(
    customerId ? `/api/billing/credits?customerId=${customerId}` : null,
    fetcher,
  );
}
export async function adjustCredits(data: Record<string, unknown>) {
  return mutate("/api/billing/credits", { method: "POST", body: JSON.stringify(data) }, "Failed to adjust credits");
}

// ---- Saved Payment Methods ----
export function useSavedPaymentMethods(customerId: string | undefined) {
  return useSWR(
    customerId ? `/api/billing/payment-methods?customerId=${customerId}` : null,
    fetcher,
  );
}
export async function deletePaymentMethod(id: string) {
  return mutate(`/api/billing/payment-methods/${id}`, { method: "DELETE" }, "Failed to delete payment method");
}
export async function setDefaultPaymentMethod(id: string) {
  return mutate(`/api/billing/payment-methods/${id}/default`, { method: "POST" }, "Failed to set default payment method");
}

// ---- Search ----
export function useSearch(query: string) {
  return useSWR(
    query && query.length >= 2 ? `/api/search?q=${encodeURIComponent(query)}` : null,
    fetcher,
    { dedupingInterval: 300 },
  );
}
