import { db } from "@/lib/db";
import { subscriptions, pricingPlans, invoices, invoiceItems, usageEvents, subscriptionDiscounts, coupons, customers } from "@/lib/db/schema";
import { eq, and, lte, sql, gte, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { verifyCronSecret } from "@/lib/billing/cron-auth";
import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { notifyInvoiceIssued } from "@/lib/billing/notifications";

function addBillingCycle(date: Date, cycle: "monthly" | "quarterly" | "yearly"): Date {
  const next = new Date(date);
  switch (cycle) {
    case "monthly":
      next.setMonth(next.getMonth() + 1);
      break;
    case "quarterly":
      next.setMonth(next.getMonth() + 3);
      break;
    case "yearly":
      next.setFullYear(next.getFullYear() + 1);
      break;
  }
  return next;
}

function calculateTieredAmount(
  tiers: { upTo: number | null; price: number }[],
  totalQuantity: number
): { items: { description: string; quantity: number; unitPrice: number }[]; total: number } {
  const items: { description: string; quantity: number; unitPrice: number }[] = [];
  let remaining = totalQuantity;
  let prevBound = 0;
  let total = 0;

  for (const tier of tiers) {
    if (remaining <= 0) break;
    const tierCap = tier.upTo !== null ? tier.upTo - prevBound : remaining;
    const qty = Math.min(remaining, tierCap);
    const rangeLabel = tier.upTo !== null ? `${prevBound + 1}–${prevBound + qty}` : `${prevBound + 1}+`;
    items.push({ description: `Tier ${rangeLabel}`, quantity: qty, unitPrice: tier.price });
    total += qty * tier.price;
    remaining -= qty;
    prevBound = tier.upTo ?? prevBound + qty;
  }
  return { items, total };
}

export async function POST(req: NextRequest) {
  try {
    // Cron auth: allow cron secret OR admin
    const cronCheck = verifyCronSecret(req);
    if (cronCheck) {
      const auth = await withAdmin();
      if (!auth.success) return cronCheck;
    }

    const now = new Date();
    const results = { renewed: 0, trialsConverted: 0, canceled: 0, invoicesGenerated: 0 };

    // 1. Convert expired trials to active
    const expiredTrials = await db
      .select()
      .from(subscriptions)
      .where(and(
        eq(subscriptions.status, "trialing"),
        lte(subscriptions.trialEnd, now),
        isNull(subscriptions.deletedAt),
      ));

    for (const sub of expiredTrials) {
      await db.transaction(async (tx) => {
        await tx
          .update(subscriptions)
          .set({ status: "active", updatedAt: now })
          .where(eq(subscriptions.id, sub.id));
      });
      results.trialsConverted++;
    }

    // 2. Cancel subscriptions with cancelAtPeriodEnd
    const toCancel = await db
      .select()
      .from(subscriptions)
      .where(and(
        eq(subscriptions.cancelAtPeriodEnd, true),
        lte(subscriptions.currentPeriodEnd, now),
        eq(subscriptions.status, "active"),
        isNull(subscriptions.deletedAt),
      ));

    for (const sub of toCancel) {
      await db.transaction(async (tx) => {
        await tx
          .update(subscriptions)
          .set({ status: "canceled", canceledAt: now, updatedAt: now })
          .where(eq(subscriptions.id, sub.id));
      });
      results.canceled++;
    }

    // 3. Renew active subscriptions whose period has ended
    const toRenew = await db
      .select()
      .from(subscriptions)
      .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
      .where(and(
        eq(subscriptions.status, "active"),
        eq(subscriptions.cancelAtPeriodEnd, false),
        lte(subscriptions.currentPeriodEnd, now),
        isNull(subscriptions.deletedAt),
      ));

    for (const row of toRenew) {
      const sub = row.subscriptions;
      const plan = row.pricing_plans;
      if (!plan) continue;

      await db.transaction(async (tx) => {
        // Idempotency check: skip if invoice already exists for this subscription + period
        const [existingInvoice] = await tx
          .select({ id: invoices.id })
          .from(invoices)
          .innerJoin(invoiceItems, eq(invoiceItems.invoiceId, invoices.id))
          .where(and(
            eq(invoices.subscriptionId, sub.id),
            eq(invoiceItems.periodStart, sub.currentPeriodStart),
            eq(invoiceItems.periodEnd, sub.currentPeriodEnd),
          ))
          .limit(1);

        if (existingInvoice) return; // Already processed

        const newStart = sub.currentPeriodEnd;
        const newEnd = addBillingCycle(newStart, plan.billingCycle);

        // Advance the period
        await tx
          .update(subscriptions)
          .set({ currentPeriodStart: newStart, currentPeriodEnd: newEnd, updatedAt: now })
          .where(eq(subscriptions.id, sub.id));
        results.renewed++;

        // Generate invoice for the old period
        let lineItems: { description: string; quantity: number; unitPrice: number }[] = [];

        const basePrice = Number(plan.basePrice);
        const unitPrice = plan.unitPrice !== null ? Number(plan.unitPrice) : null;

        if (plan.pricingModel === "flat") {
          lineItems = [{ description: `${plan.name} — ${plan.billingCycle}`, quantity: 1, unitPrice: basePrice }];
        } else if (plan.pricingModel === "per_unit") {
          lineItems = [{ description: `${plan.name} × ${sub.quantity} units`, quantity: sub.quantity, unitPrice: unitPrice ?? basePrice }];
        } else if (plan.pricingModel === "tiered" && plan.tiers) {
          const tiers = plan.tiers as { upTo: number | null; price: number }[];
          const { items } = calculateTieredAmount(tiers, sub.quantity);
          lineItems = items.map((i) => ({ ...i, description: `${plan.name} ${i.description}` }));
        } else if (plan.pricingModel === "usage_based") {
          const [usageResult] = await tx
            .select({ total: sql<number>`COALESCE(SUM(${usageEvents.value}), 0)` })
            .from(usageEvents)
            .where(and(
              eq(usageEvents.subscriptionId, sub.id),
              eq(usageEvents.metricName, plan.usageMetricName ?? ""),
              gte(usageEvents.timestamp, sub.currentPeriodStart),
              lte(usageEvents.timestamp, sub.currentPeriodEnd),
            ));

          const usageTotal = Number(usageResult?.total ?? 0);

          if (plan.tiers && (plan.tiers as { upTo: number | null; price: number }[]).length > 0) {
            const tiers = plan.tiers as { upTo: number | null; price: number }[];
            const { items } = calculateTieredAmount(tiers, usageTotal);
            lineItems = items.map((i) => ({ ...i, description: `${plan.name} ${i.description}` }));
          } else {
            lineItems = [{ description: `${plan.name} — ${usageTotal} ${plan.usageMetricName ?? "units"}`, quantity: usageTotal, unitPrice: unitPrice ?? basePrice }];
          }

          if (basePrice > 0 && unitPrice !== null) {
            lineItems.unshift({ description: `${plan.name} — base fee`, quantity: 1, unitPrice: basePrice });
          }
        }

        // Apply subscription discounts
        const activeDiscounts = await tx
          .select()
          .from(subscriptionDiscounts)
          .leftJoin(coupons, eq(subscriptionDiscounts.couponId, coupons.id))
          .where(eq(subscriptionDiscounts.subscriptionId, sub.id));

        const lineSubtotal = lineItems.reduce((s, i) => s + i.quantity * i.unitPrice, 0);
        let discountAmount = 0;

        for (const d of activeDiscounts) {
          if (!d.coupons || !d.coupons.active) continue;
          if (d.subscription_discounts.expiresAt && new Date(d.subscription_discounts.expiresAt) < now) continue;
          if (d.coupons.validUntil && new Date(d.coupons.validUntil) < now) continue;

          if (d.coupons.discountType === "percentage") {
            discountAmount += lineSubtotal * (Number(d.coupons.discountValue) / 100);
          } else {
            discountAmount += Number(d.coupons.discountValue);
          }
        }

        if (discountAmount > 0) {
          lineItems.push({ description: "Discount", quantity: 1, unitPrice: -Math.min(discountAmount, lineSubtotal) });
        }

        const subtotal = lineItems.reduce((s, i) => s + i.quantity * i.unitPrice, 0);
        const total = Math.max(subtotal, 0);

        // Inline sequence-based invoice number generation
        const [{ nextval }] = await tx.execute(sql`SELECT nextval('invoice_number_seq')`);
        const invoiceNumber = `INV-${new Date().getFullYear()}-${String(Number(nextval)).padStart(4, "0")}`;

        const dueAt = new Date(now);
        dueAt.setDate(dueAt.getDate() + 30);

        const [invoice] = await tx
          .insert(invoices)
          .values({
            invoiceNumber,
            customerId: sub.customerId,
            subscriptionId: sub.id,
            status: "issued",
            issuedAt: now,
            dueAt,
            subtotal,
            tax: 0,
            total,
          })
          .returning();

        if (lineItems.length > 0) {
          await tx.insert(invoiceItems).values(
            lineItems.map((item) => ({
              invoiceId: invoice.id,
              description: item.description,
              quantity: item.quantity,
              unitPrice: item.unitPrice,
              amount: item.quantity * item.unitPrice,
              periodStart: sub.currentPeriodStart,
              periodEnd: sub.currentPeriodEnd,
            }))
          );
        }

        results.invoicesGenerated++;

        // Send invoice email to customer (non-blocking, after transaction commits)
        const [customer] = await tx
          .select()
          .from(customers)
          .where(eq(customers.id, sub.customerId));

        if (customer) {
          notifyInvoiceIssued({
            invoiceId: invoice.id,
            invoiceNumber: invoice.invoiceNumber,
            customerId: sub.customerId,
            customerName: customer.name,
            customerEmail: customer.billingEmail ?? customer.email,
            total,
            currency: invoice.currency,
            dueAt: dueAt.toISOString(),
          }).catch(() => {});
        }
      });
    }

    return NextResponse.json(results);
  } catch (error) {
    return handleApiError(error, "POST /api/billing/subscriptions/lifecycle");
  }
}
