import { db } from "@/lib/db";
import { subscriptions, customers, pricingPlans } from "@/lib/db/schema";
import { insertSubscriptionSchema } from "@/lib/validations/billing";
import { authenticateApiKey } from "@/lib/api-auth";
import { desc, eq, and } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { searchParams } = new URL(req.url);
  const status = searchParams.get("status");
  const customerId = searchParams.get("customerId");

  const conditions = [];
  if (status) conditions.push(eq(subscriptions.status, status as "active" | "paused" | "canceled" | "past_due" | "trialing"));
  if (customerId) conditions.push(eq(subscriptions.customerId, customerId));

  const query = db
    .select()
    .from(subscriptions)
    .leftJoin(customers, eq(subscriptions.customerId, customers.id))
    .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id));

  const rows = conditions.length > 0
    ? await query.where(and(...conditions)).orderBy(desc(subscriptions.createdAt))
    : await query.orderBy(desc(subscriptions.createdAt));

  const mapped = rows.map((r) => ({
    ...r.subscriptions,
    customerName: r.customers?.name ?? null,
    planName: r.pricing_plans?.name ?? null,
    planBasePrice: r.pricing_plans?.basePrice ?? null,
    planBillingCycle: r.pricing_plans?.billingCycle ?? null,
  }));

  return NextResponse.json(mapped);
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const body = await req.json();
  const parsed = insertSubscriptionSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const data = {
    ...parsed.data,
    currentPeriodStart: new Date(parsed.data.currentPeriodStart!),
    currentPeriodEnd: new Date(parsed.data.currentPeriodEnd!),
    ...(parsed.data.trialEnd ? { trialEnd: new Date(parsed.data.trialEnd) } : {}),
  };

  const [row] = await db.insert(subscriptions).values(data as typeof subscriptions.$inferInsert).returning();
  return NextResponse.json(row, { status: 201 });
}
