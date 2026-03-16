import { db } from "@/lib/db";
import { notifyInvoiceCreated } from "@/lib/billing/notifications";
import {
  invoices,
  invoiceItems,
  customers,
  subscriptions,
  pricingPlans,
  usageEvents,
  subscriptionDiscounts,
  coupons,
} from "@/lib/db/schema";
import { insertInvoiceSchema } from "@/lib/validations/billing";
import { withAuth, withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { desc, eq, and, sql, gte, lte, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

function calculateTieredItems(
  planName: string,
  tiers: { upTo: number | null; price: number }[],
  totalQuantity: number,
): { description: string; quantity: number; unitPrice: number }[] {
  const items: { description: string; quantity: number; unitPrice: number }[] = [];
  let remaining = totalQuantity;
  let prevBound = 0;

  for (const tier of tiers) {
    if (remaining <= 0) break;
    const tierCap = tier.upTo !== null ? tier.upTo - prevBound : remaining;
    const qty = Math.min(remaining, tierCap);
    const rangeLabel =
      tier.upTo !== null ? `${prevBound + 1}–${prevBound + qty}` : `${prevBound + 1}+`;
    items.push({
      description: `${planName} (${rangeLabel})`,
      quantity: qty,
      unitPrice: tier.price,
    });
    remaining -= qty;
    prevBound = tier.upTo ?? prevBound + qty;
  }
  return items;
}

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const status = searchParams.get("status");
    const customerId = searchParams.get("customerId");

    const conditions = [];

    // Always filter out soft-deleted invoices
    conditions.push(isNull(invoices.deletedAt));

    if (status)
      conditions.push(
        eq(invoices.status, status as "draft" | "issued" | "paid" | "overdue" | "void"),
      );

    // Customer isolation: customers can only see their own invoices
    if (auth.user.role === "customer" && auth.user.customerId) {
      conditions.push(eq(invoices.customerId, auth.user.customerId));
    } else if (customerId) {
      conditions.push(eq(invoices.customerId, customerId));
    }

    const query = db
      .select()
      .from(invoices)
      .leftJoin(customers, eq(invoices.customerId, customers.id))
      .leftJoin(subscriptions, eq(invoices.subscriptionId, subscriptions.id));

    const rows = await query.where(and(...conditions)).orderBy(desc(invoices.createdAt));

    const mapped = rows.map((r) => ({
      ...r.invoices,
      customerName: r.customers?.name ?? null,
    }));

    return NextResponse.json(mapped);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/invoices");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertInvoiceSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const { items, ...invoiceData } = parsed.data;

    const result = await db.transaction(async (tx) => {
      // Validate customer exists and has billing address
      const [customer] = await tx
        .select()
        .from(customers)
        .where(eq(customers.id, invoiceData.customerId));

      if (!customer) {
        return { error: "Customer not found", status: 404 } as const;
      }

      if (!customer.billingAddress) {
        return {
          error: "Customer must have a billing address before creating an invoice",
          status: 400,
        } as const;
      }

      // Generate invoice number from database sequence
      const year = new Date().getFullYear();
      const [{ nextval }] = await tx.execute(
        sql`SELECT nextval('invoice_number_seq')`,
      ) as unknown as [{ nextval: string }];
      const invoiceNumber = `INV-${year}-${String(Number(nextval)).padStart(4, "0")}`;

      // If subscriptionId provided, auto-generate items from plan
      let lineItems = items || [];
      let discountAmount = 0;

      if (parsed.data.subscriptionId && lineItems.length === 0) {
        const [sub] = await tx
          .select()
          .from(subscriptions)
          .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
          .where(eq(subscriptions.id, parsed.data.subscriptionId));

        if (sub?.pricing_plans) {
          const plan = sub.pricing_plans;
          const quantity = sub.subscriptions.quantity;
          const periodStart = sub.subscriptions.currentPeriodStart;
          const periodEnd = sub.subscriptions.currentPeriodEnd;

          if (plan.pricingModel === "flat") {
            lineItems = [
              {
                description: `${plan.name} — ${plan.billingCycle}`,
                quantity: 1,
                unitPrice: Number(plan.basePrice),
              },
            ];
          } else if (plan.pricingModel === "per_unit") {
            lineItems = [
              {
                description: `${plan.name} × ${quantity} units`,
                quantity,
                unitPrice: Number(plan.unitPrice ?? plan.basePrice),
              },
            ];
          } else if (plan.pricingModel === "tiered" && plan.tiers) {
            lineItems = calculateTieredItems(plan.name, plan.tiers, quantity);
          } else if (plan.pricingModel === "usage_based") {
            // Aggregate usage for the current billing period
            const [usageResult] = await tx
              .select({ total: sql<string>`COALESCE(SUM(${usageEvents.value}), 0)` })
              .from(usageEvents)
              .where(
                and(
                  eq(usageEvents.subscriptionId, parsed.data.subscriptionId),
                  eq(usageEvents.metricName, plan.usageMetricName ?? ""),
                  gte(usageEvents.timestamp, periodStart),
                  lte(usageEvents.timestamp, periodEnd),
                ),
              );

            const usageTotal = Number(usageResult?.total ?? 0);

            if (plan.tiers && plan.tiers.length > 0) {
              lineItems = calculateTieredItems(
                `${plan.name} — ${plan.usageMetricName ?? "usage"}`,
                plan.tiers,
                usageTotal,
              );
            } else {
              const unitPrice = Number(plan.unitPrice ?? plan.basePrice);
              lineItems = [
                {
                  description: `${plan.name} — ${usageTotal} ${plan.usageMetricName ?? "units"}`,
                  quantity: usageTotal,
                  unitPrice,
                },
              ];
            }

            // Add base fee if exists
            if (Number(plan.basePrice) > 0 && plan.unitPrice !== null) {
              lineItems.unshift({
                description: `${plan.name} — base fee`,
                quantity: 1,
                unitPrice: Number(plan.basePrice),
              });
            }
          } else {
            lineItems = [
              {
                description: `${plan.name} — ${plan.billingCycle}`,
                quantity: 1,
                unitPrice: Number(plan.basePrice),
              },
            ];
          }

          // Apply coupon discounts from subscription
          const activeDiscounts = await tx
            .select()
            .from(subscriptionDiscounts)
            .leftJoin(coupons, eq(subscriptionDiscounts.couponId, coupons.id))
            .where(
              and(
                eq(subscriptionDiscounts.subscriptionId, parsed.data.subscriptionId),
                eq(coupons.active, true),
              ),
            );

          const lineSubtotal = lineItems.reduce((s, i) => s + i.quantity * i.unitPrice, 0);

          for (const d of activeDiscounts) {
            if (!d.coupons) continue;
            // Check expiry
            if (
              d.subscription_discounts.expiresAt &&
              new Date(d.subscription_discounts.expiresAt) < new Date()
            )
              continue;
            // Check coupon validity
            if (d.coupons.validUntil && new Date(d.coupons.validUntil) < new Date()) continue;

            if (d.coupons.discountType === "percentage") {
              discountAmount += lineSubtotal * (Number(d.coupons.discountValue) / 100);
            } else {
              discountAmount += Number(d.coupons.discountValue);
            }
          }

          if (discountAmount > 0) {
            lineItems.push({
              description: "Discount",
              quantity: 1,
              unitPrice: -Math.min(discountAmount, lineSubtotal),
            });
          }
        }
      }

      // Server-compute subtotal/total from line items (ignore client values)
      const subtotal = lineItems.reduce((sum, item) => sum + item.quantity * item.unitPrice, 0);
      const tax = invoiceData.tax ? Number(invoiceData.tax) : 0;
      const total = Math.max(subtotal + tax, 0);

      // Auto-default dueAt to issuedAt + 30 days if not provided
      const issuedAt = invoiceData.issuedAt ? new Date(invoiceData.issuedAt) : null;
      let dueAt = invoiceData.dueAt ? new Date(invoiceData.dueAt) : null;
      if (!dueAt && issuedAt) {
        dueAt = new Date(issuedAt);
        dueAt.setDate(dueAt.getDate() + 30);
      } else if (!dueAt) {
        dueAt = new Date(Date.now() + 30 * 86400000);
      }

      const [invoice] = await tx
        .insert(invoices)
        .values({
          ...invoiceData,
          invoiceNumber,
          subtotal,
          tax,
          total,
          issuedAt,
          dueAt,
          paidAt: invoiceData.paidAt ? new Date(invoiceData.paidAt) : null,
        })
        .returning();

      // Insert line items
      if (lineItems.length > 0) {
        await tx.insert(invoiceItems).values(
          lineItems.map((item) => ({
            invoiceId: invoice.id,
            description: item.description,
            quantity: item.quantity,
            unitPrice: item.unitPrice,
            amount: item.quantity * item.unitPrice,
          })),
        );
      }

      return { data: { ...invoice, items: lineItems }, customer } as const;
    });

    if ("error" in result) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    // Send invoice notification (non-blocking)
    notifyInvoiceCreated({
      invoiceId: result.data.id,
      invoiceNumber: result.data.invoiceNumber,
      customerId: result.data.customerId,
      customerName: result.customer.name,
      customerEmail: result.customer.billingEmail ?? result.customer.email,
      total: Number(result.data.total),
      currency: result.data.currency,
      dueAt: result.data.dueAt?.toISOString() ?? null,
    }).catch(() => {});

    return NextResponse.json(result.data, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/invoices");
  }
}
