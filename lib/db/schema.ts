import { pgTable, text, integer, numeric, timestamp, pgEnum, varchar, jsonb, boolean, unique, index } from "drizzle-orm/pg-core";
import { relations } from "drizzle-orm";

// ---- Enums ----
export const productTypeEnum = pgEnum("product_type", ["licensed", "saas", "api"]);
export const licenseStatusEnum = pgEnum("license_status", ["active", "expired", "revoked", "suspended"]);
export const trendEnum = pgEnum("trend", ["up", "down", "stable"]);
export const dealTypeEnum = pgEnum("deal_type", ["sale", "trial", "partner"]);
export const apiKeyStatusEnum = pgEnum("api_key_status", ["active", "revoked"]);
export const userRoleEnum = pgEnum("user_role", ["admin", "customer"]);
export const billingCycleEnum = pgEnum("billing_cycle", ["monthly", "quarterly", "yearly"]);
export const pricingModelEnum = pgEnum("pricing_model", ["flat", "per_unit", "tiered", "usage_based"]);
export const subscriptionStatusEnum = pgEnum("subscription_status", ["active", "paused", "canceled", "past_due", "trialing"]);
export const invoiceStatusEnum = pgEnum("invoice_status", ["draft", "issued", "paid", "overdue", "void"]);
export const paymentMethodEnum = pgEnum("payment_method", ["manual", "stripe", "xendit", "lemonsqueezy", "bank_transfer", "check"]);
export const creditNoteStatusEnum = pgEnum("credit_note_status", ["draft", "issued", "void"]);
export const discountTypeEnum = pgEnum("discount_type", ["percentage", "fixed_amount"]);
export const refundStatusEnum = pgEnum("refund_status", ["pending", "completed", "failed"]);
export const dunningStepEnum = pgEnum("dunning_step", ["reminder", "warning", "final_notice", "suspension"]);
export const webhookStatusEnum = pgEnum("webhook_status", ["active", "inactive"]);
export const billingEventTypeEnum = pgEnum("billing_event_type", [
  "invoice.created", "invoice.issued", "invoice.paid", "invoice.overdue", "invoice.voided",
  "payment.received", "payment.refunded",
  "subscription.created", "subscription.renewed", "subscription.canceled", "subscription.paused",
  "dunning.reminder", "dunning.warning", "dunning.final_notice", "dunning.suspension",
]);

// ---- Products ----
// Single-table inheritance for the discriminated union (licensed/saas/api).
// Type-specific fields are nullable; productType determines which are relevant.
export const products = pgTable("products", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  name: varchar("name", { length: 255 }).notNull(),
  productType: productTypeEnum("product_type").notNull(),
  revenue: numeric("revenue", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  target: numeric("target", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  change: numeric("change", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  // Licensed-specific
  unitsSold: integer("units_sold"),
  activeLicenses: integer("active_licenses"),
  totalLicenses: integer("total_licenses"),
  // SaaS-specific
  mau: integer("mau"),
  dau: integer("dau"),
  freeUsers: integer("free_users"),
  paidUsers: integer("paid_users"),
  churnRate: numeric("churn_rate", { precision: 12, scale: 4 }).$type<number>(),
  // API-specific
  apiCalls: integer("api_calls"),
  activeDevelopers: integer("active_developers"),
  avgLatency: numeric("avg_latency", { precision: 12, scale: 4 }).$type<number>(),
  // Timestamps
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Customers ----
export const customers = pgTable("customers", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  name: varchar("name", { length: 255 }).notNull(),
  industry: varchar("industry", { length: 255 }).notNull(),
  tier: varchar("tier", { length: 50 }).notNull(),
  location: varchar("location", { length: 255 }).notNull(),
  contact: varchar("contact", { length: 255 }).notNull(),
  email: varchar("email", { length: 255 }).notNull(),
  phone: varchar("phone", { length: 50 }).notNull(),
  totalRevenue: numeric("total_revenue", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  healthScore: integer("health_score").notNull().default(50),
  trend: trendEnum("trend").notNull().default("stable"),
  lastContact: varchar("last_contact", { length: 100 }).notNull(),
  // Billing profile
  billingEmail: varchar("billing_email", { length: 255 }),
  billingAddress: text("billing_address"),
  billingCity: varchar("billing_city", { length: 100 }),
  billingState: varchar("billing_state", { length: 100 }),
  billingZip: varchar("billing_zip", { length: 20 }),
  billingCountry: varchar("billing_country", { length: 100 }),
  taxId: varchar("tax_id", { length: 50 }),
  defaultPaymentMethod: paymentMethodEnum("default_payment_method"),
  stripeCustomerId: varchar("stripe_customer_id", { length: 255 }),
  xenditCustomerId: varchar("xendit_customer_id", { length: 255 }),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Customer-Product Junction ----
export const customerProducts = pgTable("customer_products", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  customerId: text("customer_id").notNull().references(() => customers.id, { onDelete: "cascade" }),
  productId: text("product_id").notNull().references(() => products.id, { onDelete: "cascade" }),
  licenseKeys: jsonb("license_keys").$type<string[]>(),
  mau: integer("mau"),
  apiCalls: integer("api_calls"),
});

// ---- Deals ----
export const deals = pgTable("deals", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  customerId: text("customer_id").references(() => customers.id, { onDelete: "set null" }),
  company: varchar("company", { length: 255 }).notNull(),
  contact: varchar("contact", { length: 255 }).notNull(),
  email: varchar("email", { length: 255 }).notNull(),
  value: numeric("value", { precision: 12, scale: 2 }).$type<number>().notNull(),
  productId: text("product_id").references(() => products.id, { onDelete: "set null" }),
  productName: varchar("product_name", { length: 255 }).notNull(),
  productType: productTypeEnum("product_type").notNull(),
  dealType: dealTypeEnum("deal_type").notNull().default("sale"),
  date: varchar("date", { length: 20 }).notNull(),
  licenseKey: varchar("license_key", { length: 50 }),
  notes: text("notes"),
  usageMetricLabel: varchar("usage_metric_label", { length: 50 }),
  usageMetricValue: integer("usage_metric_value"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Licenses ----
export const licenses = pgTable("licenses", {
  key: varchar("key", { length: 50 }).primaryKey(),
  customerId: text("customer_id").references(() => customers.id, { onDelete: "set null" }),
  customerName: varchar("customer_name", { length: 255 }).notNull(),
  productId: text("product_id").references(() => products.id, { onDelete: "set null" }),
  productName: varchar("product_name", { length: 255 }).notNull(),
  status: licenseStatusEnum("status").notNull().default("active"),
  createdAt: varchar("created_at", { length: 20 }).notNull(),
  expiresAt: varchar("expires_at", { length: 20 }).notNull(),
  // Offline license signing fields
  licenseType: varchar("license_type", { length: 10 }).notNull().default("simple"),
  signedPayload: text("signed_payload"),
  signature: text("signature"),
  features: jsonb("features").$type<string[]>(),
  maxActivations: integer("max_activations"),
});

// ---- License Activations ----
export const licenseActivations = pgTable("license_activations", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  licenseKey: varchar("license_key", { length: 50 }).notNull().references(() => licenses.key, { onDelete: "cascade" }),
  deviceId: varchar("device_id", { length: 255 }).notNull(),
  deviceName: varchar("device_name", { length: 255 }),
  ipAddress: varchar("ip_address", { length: 45 }),
  activatedAt: timestamp("activated_at").defaultNow().notNull(),
  lastSeenAt: timestamp("last_seen_at").defaultNow().notNull(),
}, (t) => [
  unique("license_activations_key_device_unique").on(t.licenseKey, t.deviceId),
  index("license_activations_license_key_idx").on(t.licenseKey),
]);

// ---- API Keys ----
export const apiKeys = pgTable("api_keys", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  name: varchar("name", { length: 255 }).notNull(),
  keyHash: varchar("key_hash", { length: 128 }).notNull(),
  keyPrefix: varchar("key_prefix", { length: 12 }).notNull(),
  status: apiKeyStatusEnum("status").notNull().default("active"),
  lastUsedAt: timestamp("last_used_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
});

// ---- Users ----
export const users = pgTable("users", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  email: varchar("email", { length: 255 }).notNull().unique(),
  name: varchar("name", { length: 255 }).notNull(),
  passwordHash: varchar("password_hash", { length: 255 }),
  role: userRoleEnum("role").notNull().default("customer"),
  authProvider: varchar("auth_provider", { length: 20 }).notNull().default("default"),
  customerId: text("customer_id").references(() => customers.id, { onDelete: "set null" }),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Sessions ----
export const sessions = pgTable("sessions", {
  id: varchar("id", { length: 64 }).primaryKey(), // the token itself
  userId: text("user_id").notNull().references(() => users.id, { onDelete: "cascade" }),
  expiresAt: timestamp("expires_at").notNull(),
  createdAt: timestamp("created_at").defaultNow().notNull(),
});

// ---- Pricing Plans ----
export const pricingPlans = pgTable("pricing_plans", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  productId: text("product_id").references(() => products.id, { onDelete: "set null" }),
  name: varchar("name", { length: 255 }).notNull(),
  pricingModel: pricingModelEnum("pricing_model").notNull(),
  billingCycle: billingCycleEnum("billing_cycle").notNull(),
  basePrice: numeric("base_price", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  unitPrice: numeric("unit_price", { precision: 12, scale: 2 }).$type<number>(),
  tiers: jsonb("tiers").$type<{ upTo: number | null; price: number }[]>(),
  usageMetricName: varchar("usage_metric_name", { length: 100 }),
  trialDays: integer("trial_days").notNull().default(0),
  active: boolean("active").notNull().default(true),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Subscriptions ----
export const subscriptions = pgTable("subscriptions", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  customerId: text("customer_id").notNull().references(() => customers.id, { onDelete: "cascade" }),
  planId: text("plan_id").notNull().references(() => pricingPlans.id, { onDelete: "cascade" }),
  status: subscriptionStatusEnum("status").notNull().default("active"),
  currentPeriodStart: timestamp("current_period_start").notNull(),
  currentPeriodEnd: timestamp("current_period_end").notNull(),
  canceledAt: timestamp("canceled_at"),
  cancelAtPeriodEnd: boolean("cancel_at_period_end").notNull().default(false),
  trialEnd: timestamp("trial_end"),
  quantity: integer("quantity").notNull().default(1),
  metadata: jsonb("metadata").$type<Record<string, unknown>>(),
  stripeSubscriptionId: varchar("stripe_subscription_id", { length: 255 }),
  version: integer("version").notNull().default(1),
  deletedAt: timestamp("deleted_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
}, (t) => [
  index("subscriptions_customer_id_idx").on(t.customerId),
  index("subscriptions_status_idx").on(t.status),
]);

// ---- Invoices ----
export const invoices = pgTable("invoices", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  invoiceNumber: varchar("invoice_number", { length: 20 }).notNull().unique(),
  customerId: text("customer_id").notNull().references(() => customers.id, { onDelete: "cascade" }),
  subscriptionId: text("subscription_id").references(() => subscriptions.id, { onDelete: "set null" }),
  status: invoiceStatusEnum("status").notNull().default("draft"),
  issuedAt: timestamp("issued_at"),
  dueAt: timestamp("due_at"),
  paidAt: timestamp("paid_at"),
  subtotal: numeric("subtotal", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  tax: numeric("tax", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  total: numeric("total", { precision: 12, scale: 2 }).$type<number>().notNull().default(0),
  currency: varchar("currency", { length: 3 }).notNull().default("USD"),
  notes: text("notes"),
  stripeInvoiceId: varchar("stripe_invoice_id", { length: 255 }),
  xenditInvoiceId: varchar("xendit_invoice_id", { length: 255 }),
  lemonsqueezyOrderId: varchar("lemonsqueezy_order_id", { length: 255 }),
  version: integer("version").notNull().default(1),
  deletedAt: timestamp("deleted_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
}, (t) => [
  index("invoices_customer_id_idx").on(t.customerId),
  index("invoices_status_idx").on(t.status),
]);

// ---- Invoice Items ----
export const invoiceItems = pgTable("invoice_items", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  invoiceId: text("invoice_id").notNull().references(() => invoices.id, { onDelete: "cascade" }),
  description: varchar("description", { length: 500 }).notNull(),
  quantity: numeric("quantity", { precision: 12, scale: 2 }).$type<number>().notNull(),
  unitPrice: numeric("unit_price", { precision: 12, scale: 2 }).$type<number>().notNull(),
  amount: numeric("amount", { precision: 12, scale: 2 }).$type<number>().notNull(),
  periodStart: timestamp("period_start"),
  periodEnd: timestamp("period_end"),
});

// ---- Payments ----
export const payments = pgTable("payments", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  invoiceId: text("invoice_id").notNull().references(() => invoices.id, { onDelete: "cascade" }),
  amount: numeric("amount", { precision: 12, scale: 2 }).$type<number>().notNull(),
  method: paymentMethodEnum("method").notNull(),
  reference: varchar("reference", { length: 255 }),
  paidAt: timestamp("paid_at").notNull(),
  notes: text("notes"),
  stripePaymentIntentId: varchar("stripe_payment_intent_id", { length: 255 }),
  xenditPaymentId: varchar("xendit_payment_id", { length: 255 }),
  lemonsqueezyOrderId: varchar("lemonsqueezy_order_id", { length: 255 }),
  createdAt: timestamp("created_at").defaultNow().notNull(),
}, (t) => [
  index("payments_invoice_id_idx").on(t.invoiceId),
]);

// ---- Usage Events ----
export const usageEvents = pgTable("usage_events", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  subscriptionId: text("subscription_id").notNull().references(() => subscriptions.id, { onDelete: "cascade" }),
  metricName: varchar("metric_name", { length: 100 }).notNull(),
  value: numeric("value", { precision: 12, scale: 4 }).$type<number>().notNull(),
  timestamp: timestamp("timestamp").defaultNow().notNull(),
  idempotencyKey: varchar("idempotency_key", { length: 255 }),
  properties: jsonb("properties").$type<Record<string, unknown>>(),
}, (t) => [
  unique("usage_events_idempotency_key_unique").on(t.idempotencyKey),
  index("usage_events_subscription_id_idx").on(t.subscriptionId),
]);

// ---- Credit Notes ----
export const creditNotes = pgTable("credit_notes", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  creditNoteNumber: varchar("credit_note_number", { length: 20 }).notNull().unique(),
  invoiceId: text("invoice_id").notNull().references(() => invoices.id, { onDelete: "cascade" }),
  customerId: text("customer_id").notNull().references(() => customers.id, { onDelete: "cascade" }),
  reason: varchar("reason", { length: 500 }).notNull(),
  amount: numeric("amount", { precision: 12, scale: 2 }).$type<number>().notNull(),
  status: creditNoteStatusEnum("status").notNull().default("draft"),
  issuedAt: timestamp("issued_at"),
  deletedAt: timestamp("deleted_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Credit Note Items ----
export const creditNoteItems = pgTable("credit_note_items", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  creditNoteId: text("credit_note_id").notNull().references(() => creditNotes.id, { onDelete: "cascade" }),
  description: varchar("description", { length: 500 }).notNull(),
  quantity: numeric("quantity", { precision: 12, scale: 2 }).$type<number>().notNull(),
  unitPrice: numeric("unit_price", { precision: 12, scale: 2 }).$type<number>().notNull(),
  amount: numeric("amount", { precision: 12, scale: 2 }).$type<number>().notNull(),
});

// ---- Coupons ----
export const coupons = pgTable("coupons", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  code: varchar("code", { length: 50 }).notNull().unique(),
  name: varchar("name", { length: 255 }).notNull(),
  discountType: discountTypeEnum("discount_type").notNull(),
  discountValue: numeric("discount_value", { precision: 12, scale: 2 }).$type<number>().notNull(),
  currency: varchar("currency", { length: 3 }).notNull().default("USD"),
  maxRedemptions: integer("max_redemptions"),
  timesRedeemed: integer("times_redeemed").notNull().default(0),
  validFrom: timestamp("valid_from").defaultNow().notNull(),
  validUntil: timestamp("valid_until"),
  active: boolean("active").notNull().default(true),
  appliesTo: jsonb("applies_to").$type<string[]>(),
  deletedAt: timestamp("deleted_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Subscription Discounts (junction) ----
export const subscriptionDiscounts = pgTable("subscription_discounts", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  subscriptionId: text("subscription_id").notNull().references(() => subscriptions.id, { onDelete: "cascade" }),
  couponId: text("coupon_id").notNull().references(() => coupons.id, { onDelete: "cascade" }),
  appliedAt: timestamp("applied_at").defaultNow().notNull(),
  expiresAt: timestamp("expires_at"),
});

// ---- Refunds ----
export const refunds = pgTable("refunds", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  paymentId: text("payment_id").notNull().references(() => payments.id, { onDelete: "cascade" }),
  invoiceId: text("invoice_id").notNull().references(() => invoices.id, { onDelete: "cascade" }),
  amount: numeric("amount", { precision: 12, scale: 2 }).$type<number>().notNull(),
  reason: varchar("reason", { length: 500 }).notNull(),
  status: refundStatusEnum("status").notNull().default("pending"),
  stripeRefundId: varchar("stripe_refund_id", { length: 255 }),
  processedAt: timestamp("processed_at"),
  deletedAt: timestamp("deleted_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
}, (t) => [
  index("refunds_payment_id_idx").on(t.paymentId),
]);

// ---- Dunning Log ----
export const dunningLog = pgTable("dunning_log", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  invoiceId: text("invoice_id").notNull().references(() => invoices.id, { onDelete: "cascade" }),
  subscriptionId: text("subscription_id").references(() => subscriptions.id, { onDelete: "set null" }),
  step: dunningStepEnum("step").notNull(),
  scheduledAt: timestamp("scheduled_at").notNull(),
  executedAt: timestamp("executed_at"),
  notes: text("notes"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
}, (t) => [
  index("dunning_log_invoice_id_idx").on(t.invoiceId),
]);

// ---- Billing Events ----
export const billingEvents = pgTable("billing_events", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  eventType: billingEventTypeEnum("event_type").notNull(),
  resourceType: varchar("resource_type", { length: 50 }).notNull(), // invoice, payment, subscription, etc.
  resourceId: text("resource_id").notNull(),
  customerId: text("customer_id").references(() => customers.id, { onDelete: "set null" }),
  data: jsonb("data").$type<Record<string, unknown>>(),
  createdAt: timestamp("created_at").defaultNow().notNull(),
}, (t) => [
  index("billing_events_customer_id_idx").on(t.customerId),
]);

// ---- Webhook Endpoints ----
export const webhookEndpoints = pgTable("webhook_endpoints", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  url: varchar("url", { length: 500 }).notNull(),
  description: varchar("description", { length: 255 }),
  secret: varchar("secret", { length: 255 }).notNull(),
  events: jsonb("events").$type<string[]>().notNull(), // list of event types to subscribe to
  status: webhookStatusEnum("status").notNull().default("active"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- System Settings (key-value store for admin configuration) ----
export const systemSettings = pgTable("system_settings", {
  key: varchar("key", { length: 100 }).primaryKey(),
  value: text("value").notNull(),
  sensitive: boolean("sensitive").notNull().default(false),
  updatedAt: timestamp("updated_at").defaultNow().notNull(),
});

// ---- Webhook Deliveries ----
export const webhookDeliveries = pgTable("webhook_deliveries", {
  id: text("id").primaryKey().$defaultFn(() => crypto.randomUUID()),
  endpointId: text("endpoint_id").notNull().references(() => webhookEndpoints.id, { onDelete: "cascade" }),
  eventId: text("event_id").notNull().references(() => billingEvents.id, { onDelete: "cascade" }),
  payload: jsonb("payload").$type<Record<string, unknown>>().notNull(),
  responseCode: integer("response_code"),
  responseBody: text("response_body"),
  attempts: integer("attempts").notNull().default(0),
  deliveredAt: timestamp("delivered_at"),
  createdAt: timestamp("created_at").defaultNow().notNull(),
});

// ---- Relations ----
export const productsRelations = relations(products, ({ many }) => ({
  customerProducts: many(customerProducts),
  deals: many(deals),
  licenses: many(licenses),
  pricingPlans: many(pricingPlans),
}));

export const customersRelations = relations(customers, ({ many }) => ({
  customerProducts: many(customerProducts),
  deals: many(deals),
  licenses: many(licenses),
  subscriptions: many(subscriptions),
  invoices: many(invoices),
  creditNotes: many(creditNotes),
}));

export const customerProductsRelations = relations(customerProducts, ({ one }) => ({
  customer: one(customers, { fields: [customerProducts.customerId], references: [customers.id] }),
  product: one(products, { fields: [customerProducts.productId], references: [products.id] }),
}));

export const dealsRelations = relations(deals, ({ one }) => ({
  customer: one(customers, { fields: [deals.customerId], references: [customers.id] }),
  product: one(products, { fields: [deals.productId], references: [products.id] }),
}));

export const licensesRelations = relations(licenses, ({ one, many }) => ({
  customer: one(customers, { fields: [licenses.customerId], references: [customers.id] }),
  product: one(products, { fields: [licenses.productId], references: [products.id] }),
  activations: many(licenseActivations),
}));

export const licenseActivationsRelations = relations(licenseActivations, ({ one }) => ({
  license: one(licenses, { fields: [licenseActivations.licenseKey], references: [licenses.key] }),
}));

export const usersRelations = relations(users, ({ many }) => ({
  sessions: many(sessions),
}));

export const sessionsRelations = relations(sessions, ({ one }) => ({
  user: one(users, { fields: [sessions.userId], references: [users.id] }),
}));

export const pricingPlansRelations = relations(pricingPlans, ({ one, many }) => ({
  product: one(products, { fields: [pricingPlans.productId], references: [products.id] }),
  subscriptions: many(subscriptions),
}));

export const subscriptionsRelations = relations(subscriptions, ({ one, many }) => ({
  customer: one(customers, { fields: [subscriptions.customerId], references: [customers.id] }),
  plan: one(pricingPlans, { fields: [subscriptions.planId], references: [pricingPlans.id] }),
  invoices: many(invoices),
  usageEvents: many(usageEvents),
  discounts: many(subscriptionDiscounts),
}));

export const invoicesRelations = relations(invoices, ({ one, many }) => ({
  customer: one(customers, { fields: [invoices.customerId], references: [customers.id] }),
  subscription: one(subscriptions, { fields: [invoices.subscriptionId], references: [subscriptions.id] }),
  items: many(invoiceItems),
  payments: many(payments),
  creditNotes: many(creditNotes),
}));

export const invoiceItemsRelations = relations(invoiceItems, ({ one }) => ({
  invoice: one(invoices, { fields: [invoiceItems.invoiceId], references: [invoices.id] }),
}));

export const paymentsRelations = relations(payments, ({ one, many }) => ({
  invoice: one(invoices, { fields: [payments.invoiceId], references: [invoices.id] }),
  refunds: many(refunds),
}));

export const usageEventsRelations = relations(usageEvents, ({ one }) => ({
  subscription: one(subscriptions, { fields: [usageEvents.subscriptionId], references: [subscriptions.id] }),
}));

export const creditNotesRelations = relations(creditNotes, ({ one, many }) => ({
  invoice: one(invoices, { fields: [creditNotes.invoiceId], references: [invoices.id] }),
  customer: one(customers, { fields: [creditNotes.customerId], references: [customers.id] }),
  items: many(creditNoteItems),
}));

export const creditNoteItemsRelations = relations(creditNoteItems, ({ one }) => ({
  creditNote: one(creditNotes, { fields: [creditNoteItems.creditNoteId], references: [creditNotes.id] }),
}));

export const couponsRelations = relations(coupons, ({ many }) => ({
  subscriptionDiscounts: many(subscriptionDiscounts),
}));

export const subscriptionDiscountsRelations = relations(subscriptionDiscounts, ({ one }) => ({
  subscription: one(subscriptions, { fields: [subscriptionDiscounts.subscriptionId], references: [subscriptions.id] }),
  coupon: one(coupons, { fields: [subscriptionDiscounts.couponId], references: [coupons.id] }),
}));

export const refundsRelations = relations(refunds, ({ one }) => ({
  payment: one(payments, { fields: [refunds.paymentId], references: [payments.id] }),
  invoice: one(invoices, { fields: [refunds.invoiceId], references: [invoices.id] }),
}));

export const dunningLogRelations = relations(dunningLog, ({ one }) => ({
  invoice: one(invoices, { fields: [dunningLog.invoiceId], references: [invoices.id] }),
  subscription: one(subscriptions, { fields: [dunningLog.subscriptionId], references: [subscriptions.id] }),
}));

export const billingEventsRelations = relations(billingEvents, ({ one, many }) => ({
  customer: one(customers, { fields: [billingEvents.customerId], references: [customers.id] }),
  deliveries: many(webhookDeliveries),
}));

export const webhookEndpointsRelations = relations(webhookEndpoints, ({ many }) => ({
  deliveries: many(webhookDeliveries),
}));

export const webhookDeliveriesRelations = relations(webhookDeliveries, ({ one }) => ({
  endpoint: one(webhookEndpoints, { fields: [webhookDeliveries.endpointId], references: [webhookEndpoints.id] }),
  event: one(billingEvents, { fields: [webhookDeliveries.eventId], references: [billingEvents.id] }),
}));

