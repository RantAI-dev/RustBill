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

function toDecimalString(value: unknown): string | undefined {
  if (value === null || value === undefined || value === "") return undefined;
  const num = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(num)) return undefined;
  return String(num);
}

function toInt(value: unknown): number | undefined {
  if (value === null || value === undefined || value === "") return undefined;
  const num = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(num)) return undefined;
  return Math.trunc(num);
}

function toRustProductPayload(data: Record<string, unknown>): Record<string, unknown> {
  const productType = (data.productType as string) ?? (data.product_type as string);
  return {
    name: data.name,
    product_type: productType,
    target: toDecimalString(data.target),
    revenue: toDecimalString(data.revenue),
    change: toDecimalString(data.change),
    units_sold: toInt(data.unitsSold ?? data.units_sold),
    active_licenses: toInt(data.activeLicenses ?? data.active_licenses),
    total_licenses: toInt(data.totalLicenses ?? data.total_licenses),
    mau: toInt(data.mau),
    dau: toInt(data.dau),
    free_users: toInt(data.freeUsers ?? data.free_users),
    paid_users: toInt(data.paidUsers ?? data.paid_users),
    churn_rate: toDecimalString(data.churnRate ?? data.churn_rate),
    api_calls: toInt(data.apiCalls ?? data.api_calls),
    active_developers: toInt(data.activeDevelopers ?? data.active_developers),
    avg_latency: toDecimalString(data.avgLatency ?? data.avg_latency),
  };
}

function fromRustProduct(product: Record<string, unknown>): Record<string, unknown> {
  return {
    ...product,
    productType: (product.product_type as string) ?? product.productType,
    unitsSold: (product.units_sold as number) ?? product.unitsSold,
    activeLicenses: (product.active_licenses as number) ?? product.activeLicenses,
    totalLicenses: (product.total_licenses as number) ?? product.totalLicenses,
    freeUsers: (product.free_users as number) ?? product.freeUsers,
    paidUsers: (product.paid_users as number) ?? product.paidUsers,
    churnRate: (product.churn_rate as number | string) ?? product.churnRate,
    apiCalls: (product.api_calls as number) ?? product.apiCalls,
    activeDevelopers: (product.active_developers as number) ?? product.activeDevelopers,
    avgLatency: (product.avg_latency as number | string) ?? product.avgLatency,
    createdAt: (product.created_at as string) ?? product.createdAt,
    updatedAt: (product.updated_at as string) ?? product.updatedAt,
  };
}

function toRustCustomerPayload(data: Record<string, unknown>): Record<string, unknown> {
  return {
    name: data.name,
    industry: data.industry,
    tier: data.tier,
    location: data.location,
    contact: data.contact,
    email: data.email,
    phone: data.phone,
    billing_email: data.billingEmail ?? data.billing_email ?? null,
    billing_address: data.billingAddress ?? data.billing_address ?? null,
    billing_city: data.billingCity ?? data.billing_city ?? null,
    billing_state: data.billingState ?? data.billing_state ?? null,
    billing_zip: data.billingZip ?? data.billing_zip ?? null,
    billing_country: data.billingCountry ?? data.billing_country ?? null,
    tax_id: data.taxId ?? data.tax_id ?? null,
    default_payment_method: data.defaultPaymentMethod ?? data.default_payment_method ?? null,
    stripe_customer_id: data.stripeCustomerId ?? data.stripe_customer_id ?? null,
    xendit_customer_id: data.xenditCustomerId ?? data.xendit_customer_id ?? null,
  };
}

function fromRustCustomer(customer: Record<string, unknown>): Record<string, unknown> {
  return {
    ...customer,
    totalRevenue: Number((customer.total_revenue as number | string) ?? customer.totalRevenue ?? 0),
    healthScore: Number((customer.health_score as number | string) ?? customer.healthScore ?? 0),
    lastContact: (customer.last_contact as string) ?? customer.lastContact,
    billingEmail: (customer.billing_email as string) ?? customer.billingEmail,
    billingAddress: (customer.billing_address as string) ?? customer.billingAddress,
    billingCity: (customer.billing_city as string) ?? customer.billingCity,
    billingState: (customer.billing_state as string) ?? customer.billingState,
    billingZip: (customer.billing_zip as string) ?? customer.billingZip,
    billingCountry: (customer.billing_country as string) ?? customer.billingCountry,
    taxId: (customer.tax_id as string) ?? customer.taxId,
    defaultPaymentMethod:
      (customer.default_payment_method as string) ?? customer.defaultPaymentMethod,
    stripeCustomerId: (customer.stripe_customer_id as string) ?? customer.stripeCustomerId,
    xenditCustomerId: (customer.xendit_customer_id as string) ?? customer.xenditCustomerId,
    createdAt: (customer.created_at as string) ?? customer.createdAt,
    updatedAt: (customer.updated_at as string) ?? customer.updatedAt,
  };
}

function toRustDealPayload(data: Record<string, unknown>): Record<string, unknown> {
  const decimalValue = toDecimalString(data.value) ?? "0";
  return {
    customerId: data.customerId ?? data.customer_id ?? null,
    customer_id: data.customerId ?? data.customer_id ?? null,
    company: data.company ?? null,
    contact: data.contact ?? null,
    email: data.email ?? null,
    value: decimalValue,
    productId: data.productId ?? data.product_id ?? null,
    product_id: data.productId ?? data.product_id ?? null,
    productName: data.productName ?? data.product_name ?? null,
    product_name: data.productName ?? data.product_name ?? null,
    productType: data.productType ?? data.product_type ?? null,
    product_type: data.productType ?? data.product_type ?? null,
    dealType: data.dealType ?? data.deal_type ?? "sale",
    deal_type: data.dealType ?? data.deal_type ?? "sale",
    date: data.date ?? null,
    licenseKey: data.licenseKey ?? data.license_key ?? null,
    license_key: data.licenseKey ?? data.license_key ?? null,
    notes: data.notes ?? null,
    usageMetricLabel: data.usageMetricLabel ?? data.usage_metric_label ?? null,
    usage_metric_label: data.usageMetricLabel ?? data.usage_metric_label ?? null,
    usageMetricValue: toInt(data.usageMetricValue ?? data.usage_metric_value),
    usage_metric_value: toInt(data.usageMetricValue ?? data.usage_metric_value),
    autoCreateInvoice: Boolean(data.autoCreateInvoice ?? data.auto_create_invoice ?? false),
    auto_create_invoice: Boolean(data.autoCreateInvoice ?? data.auto_create_invoice ?? false),
  };
}

function fromRustDeal(deal: Record<string, unknown>): Record<string, unknown> {
  return {
    ...deal,
    customerId: (deal.customer_id as string) ?? deal.customerId,
    productId: (deal.product_id as string) ?? deal.productId,
    productName: (deal.product_name as string) ?? deal.productName,
    productType: (deal.product_type as string) ?? deal.productType,
    dealType: (deal.deal_type as string) ?? deal.dealType,
    licenseKey: (deal.license_key as string) ?? deal.licenseKey,
    usageMetricLabel:
      (deal.usage_metric_label as string) ?? deal.usageMetricLabel,
    usageMetricValue:
      (deal.usage_metric_value as number) ?? deal.usageMetricValue,
    createdAt: (deal.created_at as string) ?? deal.createdAt,
    updatedAt: (deal.updated_at as string) ?? deal.updatedAt,
  };
}

function extractPreRenewalInvoiceDays(metadata: unknown): number | undefined {
  if (!metadata || typeof metadata !== "object") return undefined;
  const obj = metadata as Record<string, unknown>;
  const raw = obj.preRenewalInvoiceDays ?? obj.pre_renewal_invoice_days;
  const num = typeof raw === "number" ? raw : Number(raw);
  if (!Number.isFinite(num)) return undefined;
  return Math.trunc(num);
}

function fromRustSubscription(sub: Record<string, unknown>): Record<string, unknown> {
  const metadata = (sub.metadata ?? {}) as Record<string, unknown>;
  return {
    ...sub,
    customerId: (sub.customer_id as string) ?? sub.customerId,
    customerName: (sub.customer_name as string) ?? sub.customerName,
    planId: (sub.plan_id as string) ?? sub.planId,
    planName: (sub.plan_name as string) ?? sub.planName,
    planBasePrice: (sub.plan_base_price as number | string) ?? sub.planBasePrice,
    planBillingCycle: (sub.plan_billing_cycle as string) ?? sub.planBillingCycle,
    currentPeriodStart: (sub.current_period_start as string) ?? sub.currentPeriodStart,
    currentPeriodEnd: (sub.current_period_end as string) ?? sub.currentPeriodEnd,
    cancelAtPeriodEnd: (sub.cancel_at_period_end as boolean) ?? sub.cancelAtPeriodEnd,
    trialEnd: (sub.trial_end as string) ?? sub.trialEnd,
    stripeSubscriptionId: (sub.stripe_subscription_id as string) ?? sub.stripeSubscriptionId,
    createdAt: (sub.created_at as string) ?? sub.createdAt,
    updatedAt: (sub.updated_at as string) ?? sub.updatedAt,
    metadata,
    preRenewalInvoiceDays: extractPreRenewalInvoiceDays(metadata),
  };
}

function fromRustPlan(plan: Record<string, unknown>): Record<string, unknown> {
  return {
    ...plan,
    productId: (plan.product_id as string) ?? plan.productId,
    productName: (plan.product_name as string) ?? plan.productName,
    pricingModel: (plan.pricing_model as string) ?? plan.pricingModel,
    billingCycle: (plan.billing_cycle as string) ?? plan.billingCycle,
    basePrice: Number((plan.base_price as number | string) ?? plan.basePrice ?? 0),
    unitPrice: Number((plan.unit_price as number | string) ?? plan.unitPrice ?? 0),
    usageMetricName: (plan.usage_metric_name as string) ?? plan.usageMetricName,
    trialDays: Number((plan.trial_days as number | string) ?? plan.trialDays ?? 0),
    createdAt: (plan.created_at as string) ?? plan.createdAt,
    updatedAt: (plan.updated_at as string) ?? plan.updatedAt,
  };
}

function fromRustInvoiceItem(item: Record<string, unknown>): Record<string, unknown> {
  return {
    ...item,
    invoiceId: (item.invoice_id as string) ?? item.invoiceId,
    unitPrice: Number((item.unit_price as number | string) ?? item.unitPrice ?? 0),
    periodStart: (item.period_start as string) ?? item.periodStart,
    periodEnd: (item.period_end as string) ?? item.periodEnd,
    createdAt: (item.created_at as string) ?? item.createdAt,
    updatedAt: (item.updated_at as string) ?? item.updatedAt,
  };
}

function fromRustPayment(payment: Record<string, unknown>): Record<string, unknown> {
  return {
    ...payment,
    customerId: (payment.customer_id as string) ?? payment.customerId,
    invoiceId: (payment.invoice_id as string) ?? payment.invoiceId,
    providerReference: (payment.provider_reference as string) ?? payment.providerReference,
    paidAt: (payment.paid_at as string) ?? payment.paidAt,
    createdAt: (payment.created_at as string) ?? payment.createdAt,
    updatedAt: (payment.updated_at as string) ?? payment.updatedAt,
  };
}

function fromRustInvoice(inv: Record<string, unknown>): Record<string, unknown> {
  const items = Array.isArray(inv.items)
    ? (inv.items as Record<string, unknown>[]).map((item) => fromRustInvoiceItem(item))
    : [];
  const payments = Array.isArray(inv.payments)
    ? (inv.payments as Record<string, unknown>[]).map((payment) => fromRustPayment(payment))
    : [];

  return {
    ...inv,
    invoiceNumber: (inv.invoice_number as string) ?? inv.invoiceNumber,
    customerId: (inv.customer_id as string) ?? inv.customerId,
    customerName: (inv.customer_name as string) ?? inv.customerName,
    subscriptionId: (inv.subscription_id as string) ?? inv.subscriptionId,
    issuedAt: (inv.issued_at as string) ?? inv.issuedAt,
    dueAt: (inv.due_at as string) ?? inv.dueAt,
    paidAt: (inv.paid_at as string) ?? inv.paidAt,
    taxName: (inv.tax_name as string) ?? inv.taxName,
    taxRate: Number((inv.tax_rate as number | string) ?? inv.taxRate ?? 0),
    taxInclusive: (inv.tax_inclusive as boolean) ?? inv.taxInclusive,
    amountDue: Number((inv.amount_due as number | string) ?? inv.amountDue ?? 0),
    creditsApplied: Number((inv.credits_applied as number | string) ?? inv.creditsApplied ?? 0),
    createdAt: (inv.created_at as string) ?? inv.createdAt,
    updatedAt: (inv.updated_at as string) ?? inv.updatedAt,
    items,
    payments,
  };
}

function fromRustCoupon(coupon: Record<string, unknown>): Record<string, unknown> {
  return {
    ...coupon,
    discountType: (coupon.discount_type as string) ?? coupon.discountType,
    discountValue: Number((coupon.discount_value as number | string) ?? coupon.discountValue ?? 0),
    maxRedemptions: (coupon.max_redemptions as number | null) ?? coupon.maxRedemptions,
    timesRedeemed: Number((coupon.times_redeemed as number | string) ?? coupon.timesRedeemed ?? 0),
    validFrom: (coupon.valid_from as string) ?? coupon.validFrom,
    validUntil: (coupon.valid_until as string) ?? coupon.validUntil,
    appliesTo: (coupon.applies_to as unknown) ?? coupon.appliesTo,
    createdAt: (coupon.created_at as string) ?? coupon.createdAt,
    updatedAt: (coupon.updated_at as string) ?? coupon.updatedAt,
    deletedAt: (coupon.deleted_at as string | null) ?? coupon.deletedAt,
  };
}

function fromRustLicense(license: Record<string, unknown>): Record<string, unknown> {
  const signature = (license.signature as string | null | undefined) ?? null;
  const signedPayload = (license.signed_payload as string | null | undefined) ?? null;
  return {
    ...license,
    customerId: (license.customer_id as string) ?? license.customerId,
    customerName: (license.customer_name as string) ?? license.customerName,
    customer: (license.customer_name as string) ?? license.customer ?? "—",
    productId: (license.product_id as string) ?? license.productId,
    productName: (license.product_name as string) ?? license.productName,
    product: (license.product_name as string) ?? license.product ?? "—",
    licenseType: (license.license_type as string) ?? license.licenseType,
    maxActivations: (license.max_activations as number) ?? license.maxActivations,
    createdAt: (license.created_at as string) ?? license.createdAt,
    expiresAt: (license.expires_at as string) ?? license.expiresAt,
    signedPayload,
    signature,
    hasCertificate: Boolean(signature && signedPayload),
  };
}

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
      let msg = body.error ?? errorMessage;
      if (typeof msg !== "string") {
        const fieldErrors = (body?.error as { fieldErrors?: Record<string, string[]> } | undefined)?.fieldErrors;
        const firstField = fieldErrors
          ? Object.entries(fieldErrors).find(([, errs]) => Array.isArray(errs) && errs.length > 0)
          : undefined;
        if (firstField) {
          const [field, errs] = firstField;
          msg = `${field}: ${errs[0]}`;
        } else {
          msg = errorMessage;
        }
      }
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
  return useSWR("/api/products", async (url: string) => {
    const rows = (await fetcher(url)) as Record<string, unknown>[];
    return rows.map(fromRustProduct);
  });
}
export async function createProduct(data: Record<string, unknown>) {
  return mutate(
    "/api/products",
    { method: "POST", body: JSON.stringify(toRustProductPayload(data)) },
    "Failed to create product",
  );
}
export async function updateProduct(id: string, data: Record<string, unknown>) {
  return mutate(
    `/api/products/${id}`,
    { method: "PUT", body: JSON.stringify(toRustProductPayload(data)) },
    "Failed to update product",
  );
}
export async function deleteProduct(id: string) {
  return mutate(`/api/products/${id}`, { method: "DELETE" }, "Failed to delete product");
}

// ---- Deals ----
export function useDeals() {
  return useSWR("/api/deals", async (url: string) => {
    const rows = (await fetcher(url)) as Record<string, unknown>[];
    return rows.map(fromRustDeal);
  });
}
export async function createDeal(data: Record<string, unknown>) {
  return mutate(
    "/api/deals",
    { method: "POST", body: JSON.stringify(toRustDealPayload(data)) },
    "Failed to create deal",
  );
}
export async function updateDeal(id: string, data: Record<string, unknown>) {
  return mutate(
    `/api/deals/${id}`,
    { method: "PUT", body: JSON.stringify(toRustDealPayload(data)) },
    "Failed to update deal",
  );
}
export async function deleteDeal(id: string) {
  return mutate(`/api/deals/${id}`, { method: "DELETE" }, "Failed to delete deal");
}

// ---- Customers ----
export function useCustomers() {
  return useSWR("/api/customers", async (url: string) => {
    const rows = (await fetcher(url)) as Record<string, unknown>[];
    return rows.map(fromRustCustomer);
  });
}
export async function createCustomer(data: Record<string, unknown>) {
  return mutate(
    "/api/customers",
    { method: "POST", body: JSON.stringify(toRustCustomerPayload(data)) },
    "Failed to create customer",
  );
}
export async function updateCustomer(id: string, data: Record<string, unknown>) {
  return mutate(
    `/api/customers/${id}`,
    { method: "PUT", body: JSON.stringify(toRustCustomerPayload(data)) },
    "Failed to update customer",
  );
}
export async function deleteCustomer(id: string) {
  return mutate(`/api/customers/${id}`, { method: "DELETE" }, "Failed to delete customer");
}

// ---- Licenses ----
export function useLicenses() {
  return useSWR("/api/licenses", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustLicense(row as Record<string, unknown>))
      : [];
  });
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
  return useSWR("/api/licenses/keypair", async (url: string) => {
    const data = (await fetcher(url)) as Record<string, unknown>;
    const hasKeypair =
      (data.hasKeypair as boolean | undefined) ??
      (data.exists as boolean | undefined) ??
      false;
    return {
      ...data,
      hasKeypair,
      publicKey: (data.publicKey as string | null | undefined) ?? "",
    };
  });
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
  return useSWR("/api/billing/plans", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustPlan(row as Record<string, unknown>))
      : [];
  });
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
  return useSWR("/api/billing/subscriptions", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustSubscription(row as Record<string, unknown>))
      : [];
  });
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
  return useSWR("/api/billing/invoices", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustInvoice(row as Record<string, unknown>))
      : [];
  });
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

// ---- One-Time Sales ----
export function useOneTimeSales() {
  return useSWR("/api/billing/one-time-sales", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustInvoice(row as Record<string, unknown>))
      : [];
  });
}
export async function createOneTimeSale(data: Record<string, unknown>) {
  return mutate("/api/billing/one-time-sales", { method: "POST", body: JSON.stringify(data) }, "Failed to create one-time sale");
}
export async function updateOneTimeSale(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/one-time-sales/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update one-time sale");
}
export async function deleteOneTimeSale(id: string) {
  return mutate(`/api/billing/one-time-sales/${id}`, { method: "DELETE" }, "Failed to delete one-time sale");
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
export async function createUsageEvent(data: Record<string, unknown>) {
  return mutate("/api/billing/usage", { method: "POST", body: JSON.stringify(data) }, "Failed to record usage event");
}
export async function updateUsageEvent(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/usage/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update usage event");
}
export async function deleteUsageEvent(id: string) {
  return mutate(`/api/billing/usage/${id}`, { method: "DELETE" }, "Failed to delete usage event");
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
  return useSWR("/api/billing/coupons", async (url: string) => {
    const rows = await fetcher(url);
    return Array.isArray(rows)
      ? rows.map((row) => fromRustCoupon(row as Record<string, unknown>))
      : [];
  });
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
export function useSales360Summary(from?: string, to?: string, timezone?: string, currency?: string) {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  if (timezone) params.set("timezone", timezone);
  if (currency) params.set("currency", currency);
  const suffix = params.toString() ? `?${params.toString()}` : "";
  return useSWR(`/api/analytics/sales-360/summary${suffix}`, fetcher);
}
export function useSales360Timeseries(from?: string, to?: string, timezone?: string, currency?: string) {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  if (timezone) params.set("timezone", timezone);
  if (currency) params.set("currency", currency);
  const suffix = params.toString() ? `?${params.toString()}` : "";
  return useSWR(`/api/analytics/sales-360/timeseries${suffix}`, fetcher);
}
export function useSales360Breakdown(from?: string, to?: string, timezone?: string, currency?: string) {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  if (timezone) params.set("timezone", timezone);
  if (currency) params.set("currency", currency);
  const suffix = params.toString() ? `?${params.toString()}` : "";
  return useSWR(`/api/analytics/sales-360/breakdown${suffix}`, fetcher);
}
export function useSales360Reconcile(from?: string, to?: string, timezone?: string, currency?: string) {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  if (timezone) params.set("timezone", timezone);
  if (currency) params.set("currency", currency);
  const suffix = params.toString() ? `?${params.toString()}` : "";
  return useSWR(`/api/analytics/sales-360/reconcile${suffix}`, fetcher);
}
export async function runSales360Backfill() {
  return mutate(
    "/api/analytics/sales-360/backfill",
    { method: "POST" },
    "Failed to backfill sales ledger",
  );
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
  return useSWR(customerId ? `/api/billing/credits/${customerId}` : null, fetcher);
}
export async function adjustCredits(data: Record<string, unknown>) {
  return mutate("/api/billing/credits/adjust", { method: "POST", body: JSON.stringify(data) }, "Failed to adjust credits");
}
export async function updateCreditAdjustment(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/credits/adjust/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update credit adjustment");
}
export async function deleteCreditAdjustment(id: string) {
  return mutate(`/api/billing/credits/adjust/${id}`, { method: "DELETE" }, "Failed to delete credit adjustment");
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
export async function createPaymentMethodSetup(data: Record<string, unknown>) {
  return mutate(
    "/api/billing/payment-methods/setup",
    { method: "POST", body: JSON.stringify(data) },
    "Failed to create payment method setup",
  );
}

// ---- Search ----
export function useSearch(query: string) {
  return useSWR(
    query && query.length >= 2 ? `/api/search?q=${encodeURIComponent(query)}` : null,
    fetcher,
    { dedupingInterval: 300 },
  );
}
