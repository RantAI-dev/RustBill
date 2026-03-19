import { db } from "@/lib/db";
import { subscriptions, customers, pricingPlans } from "@/lib/db/schema";
import { insertSubscriptionSchema } from "@/lib/validations/billing";
import { desc, eq, and, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { withAuth, withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const status = searchParams.get("status");
    const customerId = searchParams.get("customerId");

    const conditions = [isNull(subscriptions.deletedAt)];
    if (status) conditions.push(eq(subscriptions.status, status as "active" | "paused" | "canceled" | "past_due" | "trialing"));

    // Non-admin users can only see their own subscriptions
    if (auth.user.role !== "admin") {
      if (auth.user.customerId) {
        conditions.push(eq(subscriptions.customerId, auth.user.customerId));
      } else {
        return NextResponse.json([]);
      }
    } else if (customerId) {
      conditions.push(eq(subscriptions.customerId, customerId));
    }

    const rows = await db
      .select()
      .from(subscriptions)
      .leftJoin(customers, eq(subscriptions.customerId, customers.id))
      .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
      .where(and(...conditions))
      .orderBy(desc(subscriptions.createdAt));

    const mapped = rows.map((r) => ({
      ...r.subscriptions,
      customerName: r.customers?.name ?? null,
      planName: r.pricing_plans?.name ?? null,
      planBasePrice: r.pricing_plans?.basePrice ?? null,
      planBillingCycle: r.pricing_plans?.billingCycle ?? null,
    }));

    return NextResponse.json(mapped);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/subscriptions");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertSubscriptionSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    // Look up the plan to auto-compute period end & status
    const [plan] = await db
      .select()
      .from(pricingPlans)
      .where(eq(pricingPlans.id, parsed.data.planId));

    if (!plan) {
      return NextResponse.json({ error: "Plan not found" }, { status: 404 });
    }

    // Auto-compute currentPeriodStart (default to now)
    const periodStart = parsed.data.currentPeriodStart
      ? new Date(parsed.data.currentPeriodStart)
      : new Date();

    // Auto-compute currentPeriodEnd from plan billingCycle if not provided
    let periodEnd: Date;
    if (parsed.data.currentPeriodEnd) {
      periodEnd = new Date(parsed.data.currentPeriodEnd);
    } else {
      periodEnd = new Date(periodStart);
      switch (plan.billingCycle) {
        case "monthly":
          periodEnd.setMonth(periodEnd.getMonth() + 1);
          break;
        case "quarterly":
          periodEnd.setMonth(periodEnd.getMonth() + 3);
          break;
        case "yearly":
          periodEnd.setFullYear(periodEnd.getFullYear() + 1);
          break;
      }
    }

    // Validate period end is after period start
    if (periodEnd <= periodStart) {
      return NextResponse.json(
        { error: "Period end must be after period start" },
        { status: 400 }
      );
    }

    // Auto-set status based on plan trialDays
    let status = parsed.data.status ?? "active";
    let trialEnd: Date | null = parsed.data.trialEnd ? new Date(parsed.data.trialEnd) : null;

    if (!parsed.data.status && plan.trialDays > 0) {
      status = "trialing";
      if (!trialEnd) {
        trialEnd = new Date(periodStart);
        trialEnd.setDate(trialEnd.getDate() + plan.trialDays);
      }
    }

    const data = {
      ...parsed.data,
      status: status as "active" | "paused" | "canceled" | "past_due" | "trialing",
      currentPeriodStart: periodStart,
      currentPeriodEnd: periodEnd,
      trialEnd,
    };

    const [row] = await db.insert(subscriptions).values(data).returning();
    return NextResponse.json(row, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/subscriptions");
  }
}
