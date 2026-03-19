import { db } from "@/lib/db";
import { subscriptions, customers, pricingPlans } from "@/lib/db/schema";
import { updateSubscriptionSchema } from "@/lib/validations/billing";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { id } = await params;
  const [row] = await db
    .select()
    .from(subscriptions)
    .leftJoin(customers, eq(subscriptions.customerId, customers.id))
    .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
    .where(eq(subscriptions.id, id));

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({
    ...row.subscriptions,
    customerName: row.customers?.name ?? null,
    planName: row.pricing_plans?.name ?? null,
    planBasePrice: row.pricing_plans?.basePrice ?? null,
    planBillingCycle: row.pricing_plans?.billingCycle ?? null,
  });
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { id } = await params;
  const body = await req.json();
  const parsed = updateSubscriptionSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const data: Record<string, unknown> = { ...parsed.data, updatedAt: new Date() };
  if (parsed.data.currentPeriodStart) data.currentPeriodStart = new Date(parsed.data.currentPeriodStart);
  if (parsed.data.currentPeriodEnd) data.currentPeriodEnd = new Date(parsed.data.currentPeriodEnd);
  if (parsed.data.status === "canceled") data.canceledAt = new Date();

  const [row] = await db
    .update(subscriptions)
    .set(data)
    .where(eq(subscriptions.id, id))
    .returning();

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}
