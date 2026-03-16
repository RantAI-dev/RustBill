import useSWR from "swr";

const fetcher = async (url: string) => {
  const res = await fetch(url);
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: "Request failed" }));
    const error = new Error(body.error ?? `Request failed with status ${res.status}`);
    (error as unknown as Record<string, unknown>).status = res.status;
    throw error;
  }
  return res.json();
};

// ---- Products ----
export function useProducts() {
  return useSWR("/api/products", fetcher);
}

export async function createProduct(data: Record<string, unknown>) {
  const res = await fetch("/api/products", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create product");
  return res.json();
}

export async function updateProduct(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/products/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update product");
  return res.json();
}

export async function deleteProduct(id: string) {
  const res = await fetch(`/api/products/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete product");
  return res.json();
}

// ---- Deals ----
export function useDeals() {
  return useSWR("/api/deals", fetcher);
}

export async function createDeal(data: Record<string, unknown>) {
  const res = await fetch("/api/deals", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create deal");
  return res.json();
}

export async function updateDeal(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/deals/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update deal");
  return res.json();
}

export async function deleteDeal(id: string) {
  const res = await fetch(`/api/deals/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete deal");
  return res.json();
}

// ---- Customers ----
export function useCustomers() {
  return useSWR("/api/customers", fetcher);
}

export async function createCustomer(data: Record<string, unknown>) {
  const res = await fetch("/api/customers", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create customer");
  return res.json();
}

export async function updateCustomer(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/customers/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update customer");
  return res.json();
}

export async function deleteCustomer(id: string) {
  const res = await fetch(`/api/customers/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete customer");
  return res.json();
}

// ---- Licenses ----
export function useLicenses() {
  return useSWR("/api/licenses", fetcher);
}

export async function createLicense(data: Record<string, unknown>) {
  const res = await fetch("/api/licenses", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create license");
  return res.json();
}

export async function updateLicense(key: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/licenses/${key}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update license");
  return res.json();
}

export async function deleteLicense(key: string) {
  const res = await fetch(`/api/licenses/${key}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete license");
  return res.json();
}

// ---- License Activations ----
export function useLicenseActivations(key: string | null) {
  return useSWR(key ? `/api/licenses/${key}/activations` : null, fetcher);
}

export async function deactivateDevice(key: string, deviceId: string) {
  const res = await fetch(`/api/licenses/${key}/activations?deviceId=${encodeURIComponent(deviceId)}`, {
    method: "DELETE",
  });
  if (!res.ok) throw new Error("Failed to deactivate device");
  return res.json();
}

// ---- License Signing ----
export function useKeypair() {
  return useSWR("/api/licenses/keypair", fetcher);
}

export async function generateKeypair(confirm?: boolean) {
  const res = await fetch("/api/licenses/keypair", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ confirm }),
  });
  if (!res.ok) {
    const data = await res.json().catch(() => ({}));
    const err = new Error(data.error || "Failed to generate keypair");
    (err as Error & { status: number }).status = res.status;
    throw err;
  }
  return res.json();
}

export async function signLicenseKey(
  key: string,
  data: { features?: string[]; maxActivations?: number; metadata?: Record<string, unknown> },
) {
  const res = await fetch(`/api/licenses/${key}/sign`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) {
    const err = new Error("Failed to sign license");
    (err as Error & { status: number }).status = res.status;
    throw err;
  }
  return res.json();
}

export function getLicenseExportUrl(key: string) {
  return `/api/licenses/${key}/export`;
}

export async function verifyLicenseFile(licenseFile: string) {
  const res = await fetch("/api/licenses/verify", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ licenseFile }),
  });
  if (!res.ok) {
    const err = new Error("Failed to verify license");
    (err as Error & { status: number }).status = res.status;
    throw err;
  }
  return res.json();
}

// ---- API Keys ----
export function useApiKeys() {
  return useSWR("/api/api-keys", fetcher);
}

export async function createApiKey(data: { name: string }) {
  const res = await fetch("/api/api-keys", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create API key");
  return res.json();
}

export async function revokeApiKey(id: string) {
  const res = await fetch(`/api/api-keys/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to revoke API key");
  return res.json();
}

// ---- Pricing Plans ----
export function usePlans() {
  return useSWR("/api/billing/plans", fetcher);
}

export async function createPlan(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/plans", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create plan");
  return res.json();
}

export async function updatePlan(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/plans/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update plan");
  return res.json();
}

export async function deletePlan(id: string) {
  const res = await fetch(`/api/billing/plans/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete plan");
  return res.json();
}

// ---- Subscriptions ----
export function useSubscriptions() {
  return useSWR("/api/billing/subscriptions", fetcher);
}

export async function createSubscription(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/subscriptions", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create subscription");
  return res.json();
}

export async function updateSubscription(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/subscriptions/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update subscription");
  return res.json();
}

export async function deleteSubscription(id: string) {
  const res = await fetch(`/api/billing/subscriptions/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete subscription");
  return res.json();
}

// ---- Invoices ----
export function useInvoices() {
  return useSWR("/api/billing/invoices", fetcher);
}

export async function createInvoice(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/invoices", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create invoice");
  return res.json();
}

export async function updateInvoice(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/invoices/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update invoice");
  return res.json();
}

export async function deleteInvoice(id: string) {
  const res = await fetch(`/api/billing/invoices/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete invoice");
  return res.json();
}

export async function addInvoiceItem(invoiceId: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/invoices/${invoiceId}/items`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to add invoice item");
  return res.json();
}

// ---- Payments ----
export function usePayments(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/payments?invoiceId=${invoiceId}` : "/api/billing/payments";
  return useSWR(url, fetcher);
}

export async function createPayment(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/payments", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to record payment");
  return res.json();
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
  const res = await fetch("/api/billing/credit-notes", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create credit note");
  return res.json();
}

export async function updateCreditNote(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/credit-notes/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update credit note");
  return res.json();
}

export async function deleteCreditNote(id: string) {
  const res = await fetch(`/api/billing/credit-notes/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete credit note");
  return res.json();
}

// ---- Coupons ----
export function useCoupons() {
  return useSWR("/api/billing/coupons", fetcher);
}

export async function createCoupon(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/coupons", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create coupon");
  return res.json();
}

export async function updateCoupon(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/coupons/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update coupon");
  return res.json();
}

export async function deleteCoupon(id: string) {
  const res = await fetch(`/api/billing/coupons/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete coupon");
  return res.json();
}

export async function applyCoupon(couponId: string, subscriptionId: string, expiresAt?: string) {
  const res = await fetch(`/api/billing/coupons/${couponId}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ action: "apply", subscriptionId, couponId, expiresAt }),
  });
  if (!res.ok) throw new Error("Failed to apply coupon");
  return res.json();
}

// ---- Subscription Lifecycle ----
export async function runBillingLifecycle() {
  const res = await fetch("/api/billing/subscriptions/lifecycle", { method: "POST" });
  if (!res.ok) throw new Error("Failed to run billing lifecycle");
  return res.json();
}

// ---- Refunds ----
export function useRefunds(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/refunds?invoiceId=${invoiceId}` : "/api/billing/refunds";
  return useSWR(url, fetcher);
}

export async function createRefund(data: Record<string, unknown>) {
  const res = await fetch("/api/billing/refunds", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to process refund");
  return res.json();
}

// ---- Dunning ----
export function useDunningLog(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/dunning?invoiceId=${invoiceId}` : "/api/billing/dunning";
  return useSWR(url, fetcher);
}

export async function runDunning(config?: Record<string, number>) {
  const res = await fetch("/api/billing/dunning", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(config ?? {}),
  });
  if (!res.ok) throw new Error("Failed to run dunning");
  return res.json();
}

// ---- Stripe ----
export async function getStripeCheckout(invoiceId: string) {
  const res = await fetch(`/api/billing/stripe?invoiceId=${invoiceId}`);
  if (!res.ok) throw new Error("Failed to get Stripe checkout");
  return res.json();
}

// ---- Unified Checkout ----
export async function getCheckout(invoiceId: string, provider: "stripe" | "xendit" | "lemonsqueezy") {
  const res = await fetch(`/api/billing/checkout?invoiceId=${invoiceId}&provider=${provider}`);
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: "Checkout failed" }));
    throw new Error(body.error ?? `Checkout failed with status ${res.status}`);
  }
  return res.json() as Promise<{ checkoutUrl: string; provider: string }>;
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
  const res = await fetch("/api/billing/webhooks", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to create webhook");
  return res.json();
}

export async function updateWebhook(id: string, data: Record<string, unknown>) {
  const res = await fetch(`/api/billing/webhooks/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(data),
  });
  if (!res.ok) throw new Error("Failed to update webhook");
  return res.json();
}

export async function deleteWebhook(id: string) {
  const res = await fetch(`/api/billing/webhooks/${id}`, { method: "DELETE" });
  if (!res.ok) throw new Error("Failed to delete webhook");
  return res.json();
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

// ---- Search ----
export function useSearch(query: string) {
  return useSWR(
    query && query.length >= 2 ? `/api/search?q=${encodeURIComponent(query)}` : null,
    fetcher,
    { dedupingInterval: 300 },
  );
}
