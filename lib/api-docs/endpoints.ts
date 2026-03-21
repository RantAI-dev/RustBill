export type ApiScope = "public" | "admin";

export type ApiAuth = "apiKey" | "session" | "none";

export type ApiMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export type ApiEndpoint = {
  id: string;
  scope: ApiScope;
  group: string;
  method: ApiMethod;
  path: string;
  description: string;
  auth: ApiAuth;
  isSensitive?: boolean;
  requestExample?: string;
  queryHint?: string;
};

export const API_ENDPOINTS: ApiEndpoint[] = [
  // Public v1
  { id: "v1-products-list", scope: "public", group: "Products", method: "GET", path: "/api/v1/products", description: "List products", auth: "apiKey" },
  { id: "v1-products-get", scope: "public", group: "Products", method: "GET", path: "/api/v1/products/{id}", description: "Get product by ID", auth: "apiKey" },

  { id: "v1-customers-list", scope: "public", group: "Customers", method: "GET", path: "/api/v1/customers", description: "List customers", auth: "apiKey" },
  {
    id: "v1-customers-create",
    scope: "public",
    group: "Customers",
    method: "POST",
    path: "/api/v1/customers",
    description: "Create customer",
    auth: "apiKey",
    requestExample: JSON.stringify({
      name: "Acme Corp",
      industry: "Technology",
      tier: "Enterprise",
      location: "San Francisco",
      contact: "Jane Smith",
      email: "jane@acme.com",
      phone: "+1-555-0100",
    }, null, 2),
  },
  { id: "v1-customers-get", scope: "public", group: "Customers", method: "GET", path: "/api/v1/customers/{id}", description: "Get customer", auth: "apiKey" },
  {
    id: "v1-customers-update",
    scope: "public",
    group: "Customers",
    method: "PUT",
    path: "/api/v1/customers/{id}",
    description: "Update customer",
    auth: "apiKey",
    requestExample: JSON.stringify({ tier: "Growth", billingEmail: "billing@acme.com" }, null, 2),
  },
  { id: "v1-customers-delete", scope: "public", group: "Customers", method: "DELETE", path: "/api/v1/customers/{id}", description: "Delete customer", auth: "apiKey" },

  { id: "v1-deals-list", scope: "public", group: "Deals (Legacy)", method: "GET", path: "/api/v1/deals", description: "List deals", auth: "apiKey", queryHint: "?productType=licensed&dealType=sale" },
  {
    id: "v1-deals-create",
    scope: "public",
    group: "Deals (Legacy)",
    method: "POST",
    path: "/api/v1/deals",
    description: "Create legacy deal",
    auth: "apiKey",
    requestExample: JSON.stringify({
      customerId: "CUSTOMER_ID",
      productId: "PRODUCT_ID",
      value: 125000,
      dealType: "sale",
      date: "2026-03-01",
    }, null, 2),
  },
  { id: "v1-deals-get", scope: "public", group: "Deals (Legacy)", method: "GET", path: "/api/v1/deals/{id}", description: "Get deal", auth: "apiKey" },
  { id: "v1-deals-update", scope: "public", group: "Deals (Legacy)", method: "PUT", path: "/api/v1/deals/{id}", description: "Update deal", auth: "apiKey", requestExample: JSON.stringify({ notes: "Updated note" }, null, 2) },
  { id: "v1-deals-delete", scope: "public", group: "Deals (Legacy)", method: "DELETE", path: "/api/v1/deals/{id}", description: "Delete deal", auth: "apiKey" },

  { id: "v1-licenses-list", scope: "public", group: "Licenses", method: "GET", path: "/api/v1/licenses", description: "List licenses", auth: "apiKey", queryHint: "?status=active" },
  {
    id: "v1-licenses-create",
    scope: "public",
    group: "Licenses",
    method: "POST",
    path: "/api/v1/licenses",
    description: "Create license",
    auth: "apiKey",
    requestExample: JSON.stringify({
      customerId: "CUSTOMER_ID",
      productId: "PRODUCT_ID",
      customerName: "Acme Corp",
      productName: "RantAI Pro",
      maxActivations: 5,
    }, null, 2),
  },
  { id: "v1-licenses-get", scope: "public", group: "Licenses", method: "GET", path: "/api/v1/licenses/{key}", description: "Get license", auth: "apiKey" },
  { id: "v1-licenses-update", scope: "public", group: "Licenses", method: "PUT", path: "/api/v1/licenses/{key}", description: "Update license", auth: "apiKey", requestExample: JSON.stringify({ status: "suspended" }, null, 2) },
  { id: "v1-licenses-delete", scope: "public", group: "Licenses", method: "DELETE", path: "/api/v1/licenses/{key}", description: "Delete license", auth: "apiKey" },
  {
    id: "v1-licenses-verify",
    scope: "public",
    group: "Licenses",
    method: "POST",
    path: "/api/v1/licenses/verify",
    description: "Verify license",
    auth: "apiKey",
    requestExample: JSON.stringify({ key: "LIC-XXXX", deviceId: "device-1" }, null, 2),
  },
  { id: "v1-licenses-activations", scope: "public", group: "Licenses", method: "GET", path: "/api/v1/licenses/{key}/activations", description: "List license activations", auth: "apiKey" },

  { id: "v1-billing-subs-list", scope: "public", group: "Billing: Subscriptions", method: "GET", path: "/api/v1/billing/subscriptions", description: "List subscriptions", auth: "apiKey", queryHint: "?status=active&customerId=..." },
  { id: "v1-billing-subs-create", scope: "public", group: "Billing: Subscriptions", method: "POST", path: "/api/v1/billing/subscriptions", description: "Create subscription", auth: "apiKey", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", planId: "PLAN_ID", quantity: 1 }, null, 2) },
  { id: "v1-billing-subs-get", scope: "public", group: "Billing: Subscriptions", method: "GET", path: "/api/v1/billing/subscriptions/{id}", description: "Get subscription", auth: "apiKey" },
  { id: "v1-billing-subs-update", scope: "public", group: "Billing: Subscriptions", method: "PUT", path: "/api/v1/billing/subscriptions/{id}", description: "Update subscription", auth: "apiKey", requestExample: JSON.stringify({ status: "paused" }, null, 2) },
  { id: "v1-billing-subs-change-plan", scope: "public", group: "Billing: Subscriptions", method: "POST", path: "/api/v1/billing/subscriptions/{id}/change-plan", description: "Change subscription plan", auth: "apiKey", requestExample: JSON.stringify({ planId: "NEW_PLAN_ID", idempotencyKey: "chp-1" }, null, 2) },

  { id: "v1-billing-invoices-list", scope: "public", group: "Billing: Invoices", method: "GET", path: "/api/v1/billing/invoices", description: "List invoices", auth: "apiKey", queryHint: "?status=paid&customerId=..." },
  { id: "v1-billing-invoices-get", scope: "public", group: "Billing: Invoices", method: "GET", path: "/api/v1/billing/invoices/{id}", description: "Get invoice", auth: "apiKey" },

  { id: "v1-billing-usage-list", scope: "public", group: "Billing: Usage", method: "GET", path: "/api/v1/billing/usage", description: "List usage events", auth: "apiKey", queryHint: "?subscriptionId=...&metric=api_calls" },
  { id: "v1-billing-usage-create", scope: "public", group: "Billing: Usage", method: "POST", path: "/api/v1/billing/usage", description: "Record usage (single or batch)", auth: "apiKey", requestExample: JSON.stringify({ subscriptionId: "SUB_ID", metricName: "api_calls", value: 100, idempotencyKey: "evt-1" }, null, 2) },

  { id: "v1-billing-credits-get", scope: "public", group: "Billing: Credits", method: "GET", path: "/api/v1/billing/credits", description: "Get scoped credits", auth: "apiKey", queryHint: "?customerId=...&currency=USD" },

  { id: "v1-billing-pm-list", scope: "public", group: "Billing: Payment Methods", method: "GET", path: "/api/v1/billing/payment-methods", description: "List payment methods", auth: "apiKey", queryHint: "?customerId=..." },
  { id: "v1-billing-pm-create", scope: "public", group: "Billing: Payment Methods", method: "POST", path: "/api/v1/billing/payment-methods", description: "Create payment method", auth: "apiKey", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", provider: "stripe", providerToken: "pm_xxx", methodType: "card", label: "Visa" }, null, 2) },
  { id: "v1-billing-pm-setup", scope: "public", group: "Billing: Payment Methods", method: "POST", path: "/api/v1/billing/payment-methods/setup", description: "Create setup session", auth: "apiKey", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", provider: "stripe" }, null, 2) },
  { id: "v1-billing-pm-delete", scope: "public", group: "Billing: Payment Methods", method: "DELETE", path: "/api/v1/billing/payment-methods/{id}", description: "Delete payment method", auth: "apiKey", queryHint: "?customerId=..." },
  { id: "v1-billing-pm-default", scope: "public", group: "Billing: Payment Methods", method: "POST", path: "/api/v1/billing/payment-methods/{id}/default", description: "Set default payment method", auth: "apiKey", queryHint: "?customerId=..." },

  // Admin
  { id: "admin-auth-login", scope: "admin", group: "Auth", method: "POST", path: "/api/auth/login", description: "Login (session cookie)", auth: "none", requestExample: JSON.stringify({ email: "admin@rustbill.local", password: "admin123" }, null, 2) },
  { id: "admin-auth-logout", scope: "admin", group: "Auth", method: "POST", path: "/api/auth/logout", description: "Logout", auth: "session" },
  { id: "admin-auth-me", scope: "admin", group: "Auth", method: "GET", path: "/api/auth/me", description: "Current session user", auth: "session" },
  { id: "admin-auth-keycloak-login", scope: "admin", group: "Auth", method: "GET", path: "/api/auth/keycloak/login", description: "Start Keycloak login", auth: "none" },
  { id: "admin-auth-keycloak-callback", scope: "admin", group: "Auth", method: "GET", path: "/api/auth/keycloak/callback", description: "Keycloak callback", auth: "none", queryHint: "?code=...&state=..." },

  { id: "admin-products-list", scope: "admin", group: "Products", method: "GET", path: "/api/products", description: "List products", auth: "session" },
  { id: "admin-products-create", scope: "admin", group: "Products", method: "POST", path: "/api/products", description: "Create product", auth: "session", requestExample: JSON.stringify({ name: "RantAI Pro", productType: "saas", target: "100000" }, null, 2) },
  { id: "admin-products-get", scope: "admin", group: "Products", method: "GET", path: "/api/products/{id}", description: "Get product", auth: "session" },
  { id: "admin-products-update", scope: "admin", group: "Products", method: "PUT", path: "/api/products/{id}", description: "Update product", auth: "session", requestExample: JSON.stringify({ name: "RantAI Pro Plus" }, null, 2) },
  { id: "admin-products-delete", scope: "admin", group: "Products", method: "DELETE", path: "/api/products/{id}", description: "Delete product", auth: "session" },

  { id: "admin-customers-list", scope: "admin", group: "Customers", method: "GET", path: "/api/customers", description: "List customers", auth: "session" },
  { id: "admin-customers-create", scope: "admin", group: "Customers", method: "POST", path: "/api/customers", description: "Create customer", auth: "session", requestExample: JSON.stringify({ name: "Acme", industry: "SaaS", tier: "Growth", location: "Jakarta", contact: "Alex", email: "alex@acme.com", phone: "+62-21" }, null, 2) },
  { id: "admin-customers-get", scope: "admin", group: "Customers", method: "GET", path: "/api/customers/{id}", description: "Get customer", auth: "session" },
  { id: "admin-customers-update", scope: "admin", group: "Customers", method: "PUT", path: "/api/customers/{id}", description: "Update customer", auth: "session", requestExample: JSON.stringify({ billingEmail: "billing@acme.com" }, null, 2) },
  { id: "admin-customers-delete", scope: "admin", group: "Customers", method: "DELETE", path: "/api/customers/{id}", description: "Delete customer", auth: "session" },

  { id: "admin-deals-list", scope: "admin", group: "Deals (Legacy)", method: "GET", path: "/api/deals", description: "List deals", auth: "session", queryHint: "?productType=licensed&dealType=sale" },
  { id: "admin-deals-create", scope: "admin", group: "Deals (Legacy)", method: "POST", path: "/api/deals", description: "Create deal", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", productId: "PRODUCT_ID", value: 90000, dealType: "sale" }, null, 2) },
  { id: "admin-deals-get", scope: "admin", group: "Deals (Legacy)", method: "GET", path: "/api/deals/{id}", description: "Get deal", auth: "session" },
  { id: "admin-deals-update", scope: "admin", group: "Deals (Legacy)", method: "PUT", path: "/api/deals/{id}", description: "Update deal", auth: "session", requestExample: JSON.stringify({ notes: "Follow-up done" }, null, 2) },
  { id: "admin-deals-delete", scope: "admin", group: "Deals (Legacy)", method: "DELETE", path: "/api/deals/{id}", description: "Delete deal", auth: "session" },

  { id: "admin-licenses-verify", scope: "admin", group: "Licenses", method: "POST", path: "/api/licenses/verify", description: "Public license verify", auth: "none", requestExample: JSON.stringify({ key: "LIC-XXXX", deviceId: "device-1" }, null, 2) },
  { id: "admin-licenses-list", scope: "admin", group: "Licenses", method: "GET", path: "/api/licenses", description: "List licenses", auth: "session" },
  { id: "admin-licenses-create", scope: "admin", group: "Licenses", method: "POST", path: "/api/licenses", description: "Create license", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", productId: "PRODUCT_ID", startsAt: "2026-03-01", expiresAt: "2027-03-01" }, null, 2) },
  { id: "admin-licenses-keypair-get", scope: "admin", group: "Licenses", method: "GET", path: "/api/licenses/keypair", description: "Get signing keypair info", auth: "session" },
  { id: "admin-licenses-keypair-create", scope: "admin", group: "Licenses", method: "POST", path: "/api/licenses/keypair", description: "Generate signing keypair", auth: "session" },
  { id: "admin-licenses-update", scope: "admin", group: "Licenses", method: "PUT", path: "/api/licenses/{key}", description: "Update license", auth: "session", requestExample: JSON.stringify({ status: "active" }, null, 2) },
  { id: "admin-licenses-delete", scope: "admin", group: "Licenses", method: "DELETE", path: "/api/licenses/{key}", description: "Delete license", auth: "session" },
  { id: "admin-licenses-sign", scope: "admin", group: "Licenses", method: "POST", path: "/api/licenses/{key}/sign", description: "Sign license", auth: "session" },
  { id: "admin-licenses-export", scope: "admin", group: "Licenses", method: "GET", path: "/api/licenses/{key}/export", description: "Export .lic file", auth: "session" },
  { id: "admin-licenses-activations", scope: "admin", group: "Licenses", method: "GET", path: "/api/licenses/{key}/activations", description: "List activations", auth: "session" },
  { id: "admin-licenses-activation-delete", scope: "admin", group: "Licenses", method: "DELETE", path: "/api/licenses/{key}/activations", description: "Deactivate device", auth: "session", queryHint: "?deviceId=..." },

  { id: "admin-api-keys-list", scope: "admin", group: "API Keys", method: "GET", path: "/api/api-keys", description: "List API keys", auth: "session" },
  { id: "admin-api-keys-create", scope: "admin", group: "API Keys", method: "POST", path: "/api/api-keys", description: "Create API key", auth: "session", requestExample: JSON.stringify({ name: "Mobile App", customerId: "CUSTOMER_ID" }, null, 2) },
  { id: "admin-api-keys-revoke", scope: "admin", group: "API Keys", method: "DELETE", path: "/api/api-keys/{id}", description: "Revoke API key", auth: "session" },

  { id: "admin-search", scope: "admin", group: "Search", method: "GET", path: "/api/search", description: "Global search", auth: "session", queryHint: "?q=acme&limit=20" },

  { id: "admin-settings-get", scope: "admin", group: "Settings", method: "GET", path: "/api/settings/payment-providers", description: "Get provider config status", auth: "session" },
  { id: "admin-settings-update", scope: "admin", group: "Settings", method: "PUT", path: "/api/settings/payment-providers", description: "Update provider settings", auth: "session", requestExample: JSON.stringify({ provider: "stripe", settings: { secretKey: "sk_live_...", webhookSecret: "whsec_..." } }, null, 2) },

  { id: "admin-analytics-overview", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/overview", description: "Overview metrics", auth: "session" },
  { id: "admin-analytics-forecasting", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/forecasting", description: "Forecasting", auth: "session" },
  { id: "admin-analytics-reports", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/reports", description: "Reports", auth: "session" },
  { id: "admin-analytics-sales-summary", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/sales-360/summary", description: "Sales 360 summary", auth: "session", queryHint: "?from=2026-03-01&to=2026-03-31&timezone=UTC&currency=USD" },
  { id: "admin-analytics-sales-timeseries", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/sales-360/timeseries", description: "Sales 360 timeseries", auth: "session" },
  { id: "admin-analytics-sales-breakdown", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/sales-360/breakdown", description: "Sales 360 breakdown", auth: "session" },
  { id: "admin-analytics-sales-reconcile", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/sales-360/reconcile", description: "Sales 360 reconcile", auth: "session" },
  { id: "admin-analytics-sales-export", scope: "admin", group: "Analytics", method: "GET", path: "/api/analytics/sales-360/export", description: "Sales 360 CSV export", auth: "session" },
  { id: "admin-analytics-sales-backfill", scope: "admin", group: "Analytics", method: "POST", path: "/api/analytics/sales-360/backfill", description: "Sales 360 backfill", auth: "session", isSensitive: true },

  { id: "admin-billing-plans-list", scope: "admin", group: "Billing: Plans", method: "GET", path: "/api/billing/plans", description: "List plans", auth: "session" },
  { id: "admin-billing-plans-create", scope: "admin", group: "Billing: Plans", method: "POST", path: "/api/billing/plans", description: "Create plan", auth: "session", requestExample: JSON.stringify({ name: "Growth", billingPeriod: "monthly", basePrice: "99" }, null, 2) },
  { id: "admin-billing-plans-get", scope: "admin", group: "Billing: Plans", method: "GET", path: "/api/billing/plans/{id}", description: "Get plan", auth: "session" },
  { id: "admin-billing-plans-update", scope: "admin", group: "Billing: Plans", method: "PUT", path: "/api/billing/plans/{id}", description: "Update plan", auth: "session", requestExample: JSON.stringify({ name: "Growth Plus" }, null, 2) },
  { id: "admin-billing-plans-delete", scope: "admin", group: "Billing: Plans", method: "DELETE", path: "/api/billing/plans/{id}", description: "Delete plan", auth: "session" },

  { id: "admin-billing-subs-list", scope: "admin", group: "Billing: Subscriptions", method: "GET", path: "/api/billing/subscriptions", description: "List subscriptions", auth: "session" },
  { id: "admin-billing-subs-create", scope: "admin", group: "Billing: Subscriptions", method: "POST", path: "/api/billing/subscriptions", description: "Create subscription", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", planId: "PLAN_ID", status: "active" }, null, 2) },
  { id: "admin-billing-subs-lifecycle", scope: "admin", group: "Billing: Subscriptions", method: "POST", path: "/api/billing/subscriptions/lifecycle", description: "Run lifecycle manually", auth: "session" },
  { id: "admin-billing-subs-get", scope: "admin", group: "Billing: Subscriptions", method: "GET", path: "/api/billing/subscriptions/{id}", description: "Get subscription", auth: "session" },
  { id: "admin-billing-subs-update", scope: "admin", group: "Billing: Subscriptions", method: "PUT", path: "/api/billing/subscriptions/{id}", description: "Update subscription", auth: "session", requestExample: JSON.stringify({ cancelAtPeriodEnd: true }, null, 2) },
  { id: "admin-billing-subs-delete", scope: "admin", group: "Billing: Subscriptions", method: "DELETE", path: "/api/billing/subscriptions/{id}", description: "Delete subscription", auth: "session" },
  { id: "admin-billing-subs-change-plan", scope: "admin", group: "Billing: Subscriptions", method: "POST", path: "/api/billing/subscriptions/{id}/change-plan", description: "Change plan with proration", auth: "session", requestExample: JSON.stringify({ planId: "NEW_PLAN" }, null, 2) },

  { id: "admin-billing-one-time-sales-list", scope: "admin", group: "Billing: One-Time Sales", method: "GET", path: "/api/billing/one-time-sales", description: "List one-time sales", auth: "session" },
  { id: "admin-billing-one-time-sales-create", scope: "admin", group: "Billing: One-Time Sales", method: "POST", path: "/api/billing/one-time-sales", description: "Create one-time sale", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", currency: "USD", subtotal: 100, tax: 10, total: 110 }, null, 2) },
  { id: "admin-billing-one-time-sales-get", scope: "admin", group: "Billing: One-Time Sales", method: "GET", path: "/api/billing/one-time-sales/{id}", description: "Get one-time sale", auth: "session" },
  { id: "admin-billing-one-time-sales-update", scope: "admin", group: "Billing: One-Time Sales", method: "PUT", path: "/api/billing/one-time-sales/{id}", description: "Update one-time sale", auth: "session", requestExample: JSON.stringify({ status: "void", notes: "Customer requested cancellation" }, null, 2) },
  { id: "admin-billing-one-time-sales-delete", scope: "admin", group: "Billing: One-Time Sales", method: "DELETE", path: "/api/billing/one-time-sales/{id}", description: "Delete one-time sale", auth: "session" },

  { id: "admin-billing-invoices-list", scope: "admin", group: "Billing: Invoices", method: "GET", path: "/api/billing/invoices", description: "List invoices", auth: "session" },
  { id: "admin-billing-invoices-create", scope: "admin", group: "Billing: Invoices", method: "POST", path: "/api/billing/invoices", description: "Create invoice", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", currency: "USD" }, null, 2) },
  { id: "admin-billing-invoices-get", scope: "admin", group: "Billing: Invoices", method: "GET", path: "/api/billing/invoices/{id}", description: "Get invoice", auth: "session" },
  { id: "admin-billing-invoices-update", scope: "admin", group: "Billing: Invoices", method: "PUT", path: "/api/billing/invoices/{id}", description: "Update invoice", auth: "session", requestExample: JSON.stringify({ status: "issued" }, null, 2) },
  { id: "admin-billing-invoices-delete", scope: "admin", group: "Billing: Invoices", method: "DELETE", path: "/api/billing/invoices/{id}", description: "Delete invoice", auth: "session" },
  { id: "admin-billing-invoices-items-list", scope: "admin", group: "Billing: Invoices", method: "GET", path: "/api/billing/invoices/{id}/items", description: "List invoice items", auth: "session" },
  { id: "admin-billing-invoices-items-add", scope: "admin", group: "Billing: Invoices", method: "POST", path: "/api/billing/invoices/{id}/items", description: "Add invoice item", auth: "session", requestExample: JSON.stringify({ description: "Extra seat", quantity: 1, unitPrice: "19" }, null, 2) },
  { id: "admin-billing-invoices-pdf", scope: "admin", group: "Billing: Invoices", method: "GET", path: "/api/billing/invoices/{id}/pdf", description: "Get invoice PDF", auth: "session" },

  { id: "admin-billing-payments-list", scope: "admin", group: "Billing: Payments", method: "GET", path: "/api/billing/payments", description: "List payments", auth: "session" },
  { id: "admin-billing-payments-create", scope: "admin", group: "Billing: Payments", method: "POST", path: "/api/billing/payments", description: "Create payment", auth: "session", requestExample: JSON.stringify({ invoiceId: "INV_ID", amount: "100", currency: "USD", provider: "stripe" }, null, 2) },
  { id: "admin-billing-payments-get", scope: "admin", group: "Billing: Payments", method: "GET", path: "/api/billing/payments/{id}", description: "Get payment", auth: "session" },
  { id: "admin-billing-payments-update", scope: "admin", group: "Billing: Payments", method: "PUT", path: "/api/billing/payments/{id}", description: "Update payment", auth: "session", requestExample: JSON.stringify({ status: "succeeded" }, null, 2) },
  { id: "admin-billing-payments-delete", scope: "admin", group: "Billing: Payments", method: "DELETE", path: "/api/billing/payments/{id}", description: "Delete payment", auth: "session" },

  { id: "admin-billing-checkout", scope: "admin", group: "Billing: Checkout", method: "GET", path: "/api/billing/checkout", description: "Get checkout URL", auth: "session", queryHint: "?invoiceId=...&provider=stripe" },

  { id: "admin-billing-credit-notes-list", scope: "admin", group: "Billing: Credit Notes", method: "GET", path: "/api/billing/credit-notes", description: "List credit notes", auth: "session" },
  { id: "admin-billing-credit-notes-create", scope: "admin", group: "Billing: Credit Notes", method: "POST", path: "/api/billing/credit-notes", description: "Create credit note", auth: "session", requestExample: JSON.stringify({ invoiceId: "INV_ID", amount: "10", reason: "service_issue" }, null, 2) },
  { id: "admin-billing-credit-notes-get", scope: "admin", group: "Billing: Credit Notes", method: "GET", path: "/api/billing/credit-notes/{id}", description: "Get credit note", auth: "session" },
  { id: "admin-billing-credit-notes-update", scope: "admin", group: "Billing: Credit Notes", method: "PUT", path: "/api/billing/credit-notes/{id}", description: "Update credit note", auth: "session", requestExample: JSON.stringify({ reason: "duplicate_charge" }, null, 2) },
  { id: "admin-billing-credit-notes-delete", scope: "admin", group: "Billing: Credit Notes", method: "DELETE", path: "/api/billing/credit-notes/{id}", description: "Delete credit note", auth: "session" },

  { id: "admin-billing-coupons-list", scope: "admin", group: "Billing: Coupons", method: "GET", path: "/api/billing/coupons", description: "List coupons", auth: "session" },
  { id: "admin-billing-coupons-create", scope: "admin", group: "Billing: Coupons", method: "POST", path: "/api/billing/coupons", description: "Create coupon", auth: "session", requestExample: JSON.stringify({ code: "PROMO10", discountType: "percent", discountValue: "10" }, null, 2) },
  { id: "admin-billing-coupons-get", scope: "admin", group: "Billing: Coupons", method: "GET", path: "/api/billing/coupons/{id}", description: "Get coupon", auth: "session" },
  { id: "admin-billing-coupons-update", scope: "admin", group: "Billing: Coupons", method: "PUT", path: "/api/billing/coupons/{id}", description: "Update coupon", auth: "session", requestExample: JSON.stringify({ active: false }, null, 2) },
  { id: "admin-billing-coupons-delete", scope: "admin", group: "Billing: Coupons", method: "DELETE", path: "/api/billing/coupons/{id}", description: "Delete coupon", auth: "session" },

  { id: "admin-billing-refunds-list", scope: "admin", group: "Billing: Refunds", method: "GET", path: "/api/billing/refunds", description: "List refunds", auth: "session" },
  { id: "admin-billing-refunds-create", scope: "admin", group: "Billing: Refunds", method: "POST", path: "/api/billing/refunds", description: "Create refund", auth: "session", requestExample: JSON.stringify({ paymentId: "PAYMENT_ID", amount: "5", reason: "requested_by_customer" }, null, 2) },
  { id: "admin-billing-refunds-get", scope: "admin", group: "Billing: Refunds", method: "GET", path: "/api/billing/refunds/{id}", description: "Get refund", auth: "session" },
  { id: "admin-billing-refunds-update", scope: "admin", group: "Billing: Refunds", method: "PUT", path: "/api/billing/refunds/{id}", description: "Update refund", auth: "session", requestExample: JSON.stringify({ reason: "duplicate" }, null, 2) },
  { id: "admin-billing-refunds-delete", scope: "admin", group: "Billing: Refunds", method: "DELETE", path: "/api/billing/refunds/{id}", description: "Delete refund", auth: "session" },

  { id: "admin-billing-usage-list", scope: "admin", group: "Billing: Usage", method: "GET", path: "/api/billing/usage", description: "List usage events", auth: "session", queryHint: "?subscriptionId=...&metric=api_calls" },
  { id: "admin-billing-usage-record", scope: "admin", group: "Billing: Usage", method: "POST", path: "/api/billing/usage", description: "Record usage", auth: "session", requestExample: JSON.stringify({ subscriptionId: "SUB_ID", metricName: "api_calls", value: 10 }, null, 2) },
  { id: "admin-billing-usage-update", scope: "admin", group: "Billing: Usage", method: "PUT", path: "/api/billing/usage/{id}", description: "Update usage event", auth: "session", requestExample: JSON.stringify({ value: 12 }, null, 2) },
  { id: "admin-billing-usage-delete", scope: "admin", group: "Billing: Usage", method: "DELETE", path: "/api/billing/usage/{id}", description: "Delete usage event", auth: "session" },
  { id: "admin-billing-usage-summary", scope: "admin", group: "Billing: Usage", method: "GET", path: "/api/billing/usage/{subscriptionId}/summary", description: "Usage summary", auth: "session", queryHint: "?metric=api_calls" },

  { id: "admin-billing-dunning-list", scope: "admin", group: "Billing: Dunning", method: "GET", path: "/api/billing/dunning", description: "List dunning jobs", auth: "session" },
  { id: "admin-billing-dunning-create", scope: "admin", group: "Billing: Dunning", method: "POST", path: "/api/billing/dunning", description: "Create dunning entry", auth: "session", requestExample: JSON.stringify({ invoiceId: "INV_ID" }, null, 2) },
  { id: "admin-billing-dunning-get", scope: "admin", group: "Billing: Dunning", method: "GET", path: "/api/billing/dunning/{id}", description: "Get dunning entry", auth: "session" },

  { id: "admin-billing-events-list", scope: "admin", group: "Billing: Events", method: "GET", path: "/api/billing/events", description: "List billing events", auth: "session" },
  { id: "admin-billing-events-get", scope: "admin", group: "Billing: Events", method: "GET", path: "/api/billing/events/{id}", description: "Get billing event", auth: "session" },

  { id: "admin-billing-webhooks-list", scope: "admin", group: "Billing: Webhooks", method: "GET", path: "/api/billing/webhooks", description: "List webhooks", auth: "session" },
  { id: "admin-billing-webhooks-create", scope: "admin", group: "Billing: Webhooks", method: "POST", path: "/api/billing/webhooks", description: "Create webhook", auth: "session", requestExample: JSON.stringify({ provider: "stripe", url: "https://example.com/webhook" }, null, 2) },
  { id: "admin-billing-webhooks-get", scope: "admin", group: "Billing: Webhooks", method: "GET", path: "/api/billing/webhooks/{id}", description: "Get webhook", auth: "session" },
  { id: "admin-billing-webhooks-update", scope: "admin", group: "Billing: Webhooks", method: "PUT", path: "/api/billing/webhooks/{id}", description: "Update webhook", auth: "session", requestExample: JSON.stringify({ active: true }, null, 2) },
  { id: "admin-billing-webhooks-delete", scope: "admin", group: "Billing: Webhooks", method: "DELETE", path: "/api/billing/webhooks/{id}", description: "Delete webhook", auth: "session" },
  { id: "admin-billing-webhooks-test", scope: "admin", group: "Billing: Webhooks", method: "POST", path: "/api/billing/webhooks/{id}/test", description: "Send test webhook", auth: "session" },

  { id: "admin-billing-cron-run", scope: "admin", group: "Billing: Cron", method: "POST", path: "/api/billing/cron/run", description: "Run all billing cron tasks", auth: "session", isSensitive: true },
  { id: "admin-billing-cron-renew", scope: "admin", group: "Billing: Cron", method: "POST", path: "/api/billing/cron/renew-subscriptions", description: "Renew subscriptions", auth: "session", isSensitive: true },
  { id: "admin-billing-cron-invoices", scope: "admin", group: "Billing: Cron", method: "POST", path: "/api/billing/cron/generate-invoices", description: "Generate invoices", auth: "session", isSensitive: true },
  { id: "admin-billing-cron-dunning", scope: "admin", group: "Billing: Cron", method: "POST", path: "/api/billing/cron/process-dunning", description: "Process dunning", auth: "session", isSensitive: true },
  { id: "admin-billing-cron-expire", scope: "admin", group: "Billing: Cron", method: "POST", path: "/api/billing/cron/expire-licenses", description: "Expire licenses", auth: "session", isSensitive: true },

  { id: "admin-billing-credits-adjust", scope: "admin", group: "Billing: Credits", method: "POST", path: "/api/billing/credits/adjust", description: "Adjust customer credit", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", currency: "USD", amount: "20", description: "Manual credit" }, null, 2) },
  { id: "admin-billing-credits-adjust-update", scope: "admin", group: "Billing: Credits", method: "PUT", path: "/api/billing/credits/adjust/{id}", description: "Update credit adjustment", auth: "session", requestExample: JSON.stringify({ amount: "15", description: "Revised" }, null, 2) },
  { id: "admin-billing-credits-adjust-delete", scope: "admin", group: "Billing: Credits", method: "DELETE", path: "/api/billing/credits/adjust/{id}", description: "Delete credit adjustment", auth: "session" },
  { id: "admin-billing-credits-customer", scope: "admin", group: "Billing: Credits", method: "GET", path: "/api/billing/credits/{customerId}", description: "Get customer credits", auth: "session", queryHint: "?currency=USD" },

  { id: "admin-billing-tax-rules-list", scope: "admin", group: "Billing: Tax Rules", method: "GET", path: "/api/billing/tax-rules", description: "List tax rules", auth: "session" },
  { id: "admin-billing-tax-rules-create", scope: "admin", group: "Billing: Tax Rules", method: "POST", path: "/api/billing/tax-rules", description: "Create tax rule", auth: "session", requestExample: JSON.stringify({ countryCode: "US", taxRate: "0.1" }, null, 2) },
  { id: "admin-billing-tax-rules-update", scope: "admin", group: "Billing: Tax Rules", method: "PUT", path: "/api/billing/tax-rules/{id}", description: "Update tax rule", auth: "session", requestExample: JSON.stringify({ taxRate: "0.11" }, null, 2) },
  { id: "admin-billing-tax-rules-delete", scope: "admin", group: "Billing: Tax Rules", method: "DELETE", path: "/api/billing/tax-rules/{id}", description: "Delete tax rule", auth: "session" },

  { id: "admin-billing-pm-list", scope: "admin", group: "Billing: Payment Methods", method: "GET", path: "/api/billing/payment-methods", description: "List payment methods", auth: "session", queryHint: "?customerId=..." },
  { id: "admin-billing-pm-create", scope: "admin", group: "Billing: Payment Methods", method: "POST", path: "/api/billing/payment-methods", description: "Create payment method", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", provider: "stripe", providerToken: "pm_123", methodType: "card", label: "Visa" }, null, 2) },
  { id: "admin-billing-pm-setup", scope: "admin", group: "Billing: Payment Methods", method: "POST", path: "/api/billing/payment-methods/setup", description: "Create setup session", auth: "session", requestExample: JSON.stringify({ customerId: "CUSTOMER_ID", provider: "stripe" }, null, 2) },
  { id: "admin-billing-pm-delete", scope: "admin", group: "Billing: Payment Methods", method: "DELETE", path: "/api/billing/payment-methods/{id}", description: "Delete payment method", auth: "session", queryHint: "?customerId=..." },
  { id: "admin-billing-pm-default", scope: "admin", group: "Billing: Payment Methods", method: "POST", path: "/api/billing/payment-methods/{id}/default", description: "Set default payment method", auth: "session", queryHint: "?customerId=..." },
];

export function endpointPathParams(path: string): string[] {
  return [...path.matchAll(/\{([a-zA-Z0-9_]+)\}/g)].map((match) => match[1]);
}

export function endpointGroups(scope: ApiScope): string[] {
  return [...new Set(API_ENDPOINTS.filter((endpoint) => endpoint.scope === scope).map((endpoint) => endpoint.group))];
}
