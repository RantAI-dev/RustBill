import { z } from "zod";

// ---- Pricing Plans ----
export const insertPlanSchema = z.object({
  productId: z.string().nullable().optional(),
  name: z.string().min(1, "Name is required"),
  pricingModel: z.enum(["flat", "per_unit", "tiered", "usage_based"]),
  billingCycle: z.enum(["monthly", "quarterly", "yearly"]),
  basePrice: z.number().min(0, "Base price must be non-negative"),
  unitPrice: z.number().min(0).nullable().optional(),
  tiers: z
    .array(z.object({ upTo: z.number().nullable(), price: z.number().min(0) }))
    .nullable()
    .optional(),
  usageMetricName: z.string().nullable().optional(),
  trialDays: z.number().int().min(0).default(0),
  active: z.boolean().default(true),
});

export const updatePlanSchema = insertPlanSchema.partial();

// ---- Subscriptions ----
export const insertSubscriptionSchema = z.object({
  customerId: z.string().min(1, "Customer is required"),
  planId: z.string().min(1, "Plan is required"),
  status: z.enum(["active", "paused", "canceled", "past_due", "trialing"]).optional(), // Auto-set based on plan trialDays
  currentPeriodStart: z.string().min(1, "Period start is required").optional(), // Defaults to today
  currentPeriodEnd: z.string().min(1, "Period end is required").optional(), // Auto-computed from plan billingCycle
  cancelAtPeriodEnd: z.boolean().default(false),
  trialEnd: z.string().nullable().optional(),
  quantity: z.number().int().min(1).default(1),
  metadata: z.record(z.unknown()).nullable().optional(),
  stripeSubscriptionId: z.string().nullable().optional(),
});

export const updateSubscriptionSchema = z.object({
  customerId: z.string().min(1).optional(),
  planId: z.string().min(1).optional(),
  status: z.enum(["active", "paused", "canceled", "past_due", "trialing"]).optional(),
  currentPeriodStart: z.string().min(1).optional(),
  currentPeriodEnd: z.string().min(1).optional(),
  cancelAtPeriodEnd: z.boolean().optional(),
  trialEnd: z.string().nullable().optional(),
  quantity: z.number().int().min(1).optional(),
  metadata: z.record(z.unknown()).nullable().optional(),
  stripeSubscriptionId: z.string().nullable().optional(),
  version: z.number().int().optional(),
});

// ---- Invoices ----
export const insertInvoiceSchema = z.object({
  customerId: z.string().min(1, "Customer is required"),
  subscriptionId: z.string().nullable().optional(),
  status: z.enum(["draft", "issued", "paid", "overdue", "void"]).default("draft"),
  issuedAt: z.string().nullable().optional(),
  dueAt: z.string().nullable().optional(),
  paidAt: z.string().nullable().optional(),
  subtotal: z.number().min(0).default(0),
  tax: z.number().min(0).default(0),
  total: z.number().min(0).default(0),
  currency: z.string().length(3).default("USD"),
  notes: z.string().nullable().optional(),
  items: z
    .array(
      z.object({
        description: z.string().min(1, "Description is required"),
        quantity: z.number().min(0.01, "Quantity must be positive"),
        unitPrice: z.number().min(0.01, "Unit price must be positive"),
      })
    )
    .optional(),
});

export const updateInvoiceSchema = z.object({
  status: z.enum(["draft", "issued", "paid", "overdue", "void"]).optional(),
  dueAt: z.string().nullable().optional(),
  paidAt: z.string().nullable().optional(),
  tax: z.number().min(0).optional(),
  notes: z.string().nullable().optional(),
  version: z.number().int().optional(),
});

// ---- Invoice Items ----
export const insertInvoiceItemSchema = z.object({
  invoiceId: z.string().min(1, "Invoice is required"),
  description: z.string().min(1, "Description is required"),
  quantity: z.number().min(0.01, "Quantity must be positive"),
  unitPrice: z.number().min(0.01, "Unit price must be positive"),
  periodStart: z.string().nullable().optional(),
  periodEnd: z.string().nullable().optional(),
});

// ---- Payments ----
export const insertPaymentSchema = z.object({
  invoiceId: z.string().min(1, "Invoice is required"),
  amount: z.number().min(0.01, "Amount must be positive"),
  method: z.enum(["manual", "stripe", "xendit", "lemonsqueezy", "bank_transfer", "check"]),
  reference: z.string().nullable().optional(),
  paidAt: z.string().min(1, "Payment date is required"),
  notes: z.string().nullable().optional(),
  stripePaymentIntentId: z.string().nullable().optional(),
  xenditPaymentId: z.string().nullable().optional(),
  lemonsqueezyOrderId: z.string().nullable().optional(),
});

// ---- Usage Events ----
export const insertUsageEventSchema = z.object({
  subscriptionId: z.string().min(1, "Subscription is required"),
  metricName: z.string().min(1, "Metric name is required"),
  value: z.number().min(0, "Value must be non-negative"),
  timestamp: z.string().optional(),
  idempotencyKey: z.string().nullable().optional(),
  properties: z.record(z.unknown()).nullable().optional(),
});

// ---- Credit Notes ----
export const insertCreditNoteSchema = z.object({
  invoiceId: z.string().min(1, "Invoice is required"),
  customerId: z.string().min(1, "Customer is required"),
  reason: z.string().min(1, "Reason is required"),
  status: z.enum(["draft", "issued", "void"]).default("draft"),
  issuedAt: z.string().nullable().optional(),
  items: z
    .array(
      z.object({
        description: z.string().min(1, "Description is required"),
        quantity: z.number().min(0.01, "Quantity must be positive"),
        unitPrice: z.number().min(0.01, "Unit price must be positive"),
      })
    )
    .min(1, "At least one item is required"),
});

export const updateCreditNoteSchema = z.object({
  status: z.enum(["draft", "issued", "void"]).optional(),
  reason: z.string().min(1).optional(),
  issuedAt: z.string().nullable().optional(),
});

// ---- Coupons ----
export const insertCouponSchema = z.object({
  code: z.string().min(1, "Code is required").max(50),
  name: z.string().min(1, "Name is required"),
  discountType: z.enum(["percentage", "fixed_amount"]),
  discountValue: z.number().min(0.01, "Discount value must be positive"),
  currency: z.string().length(3).default("USD"),
  maxRedemptions: z.number().int().min(1).nullable().optional(),
  validFrom: z.string().optional(),
  validUntil: z.string().nullable().optional(),
  active: z.boolean().default(true),
  appliesTo: z.array(z.string()).nullable().optional(),
}).superRefine((data, ctx) => {
  if (data.discountType === "percentage" && data.discountValue > 100) {
    ctx.addIssue({
      code: z.ZodIssueCode.custom,
      path: ["discountValue"],
      message: "Percentage discount cannot exceed 100%",
    });
  }
});

export const updateCouponSchema = z.object({
  code: z.string().min(1).max(50).optional(),
  name: z.string().min(1).optional(),
  discountType: z.enum(["percentage", "fixed_amount"]).optional(),
  discountValue: z.number().min(0.01).optional(),
  currency: z.string().length(3).optional(),
  maxRedemptions: z.number().int().min(1).nullable().optional(),
  validFrom: z.string().optional(),
  validUntil: z.string().nullable().optional(),
  active: z.boolean().optional(),
  appliesTo: z.array(z.string()).nullable().optional(),
});

// ---- Subscription Discounts ----
export const insertSubscriptionDiscountSchema = z.object({
  subscriptionId: z.string().min(1, "Subscription is required"),
  couponId: z.string().min(1, "Coupon is required"),
  expiresAt: z.string().nullable().optional(),
});

// ---- Refunds ----
export const insertRefundSchema = z.object({
  paymentId: z.string().min(1, "Payment is required"),
  invoiceId: z.string().min(1, "Invoice is required"),
  amount: z.number().min(0.01, "Refund amount must be positive"),
  reason: z.string().min(1, "Reason is required"),
  status: z.enum(["pending", "completed", "failed"]).default("pending"),
});
