import { db } from "@/lib/db";
import { subscriptions, customers, pricingPlans } from "@/lib/db/schema";
import { updateSubscriptionSchema } from "@/lib/validations/billing";
import { eq, and, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const [row] = await db
      .select()
      .from(subscriptions)
      .leftJoin(customers, eq(subscriptions.customerId, customers.id))
      .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
      .where(and(eq(subscriptions.id, id), isNull(subscriptions.deletedAt)));

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

    // Non-admin users can only see their own subscriptions
    if (auth.user.role !== "admin" && row.subscriptions.customerId !== auth.user.customerId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }

    return NextResponse.json({
      ...row.subscriptions,
      customerName: row.customers?.name ?? null,
      planName: row.pricing_plans?.name ?? null,
      planBasePrice: row.pricing_plans?.basePrice ?? null,
      planBillingCycle: row.pricing_plans?.billingCycle ?? null,
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/subscriptions/[id]");
  }
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const body = await req.json();
    const parsed = updateSubscriptionSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const { version, ...rest } = parsed.data as Record<string, unknown> & { version?: number };

    const data: Record<string, unknown> = { ...rest, updatedAt: new Date() };
    if (parsed.data.currentPeriodStart) data.currentPeriodStart = new Date(parsed.data.currentPeriodStart);
    if (parsed.data.currentPeriodEnd) data.currentPeriodEnd = new Date(parsed.data.currentPeriodEnd);
    if (parsed.data.trialEnd) data.trialEnd = new Date(parsed.data.trialEnd);
    if (parsed.data.status === "canceled") data.canceledAt = new Date();

    // Optimistic locking: increment version and check current version
    const result = await db
      .update(subscriptions)
      .set({ ...data, version: (version ?? 0) + 1 })
      .where(
        and(
          eq(subscriptions.id, id),
          isNull(subscriptions.deletedAt),
          ...(version !== undefined ? [eq(subscriptions.version, version)] : []),
        )
      )
      .returning();

    if (result.length === 0) {
      // Check if subscription exists to differentiate 404 from conflict
      const [existing] = await db
        .select({ id: subscriptions.id, version: subscriptions.version })
        .from(subscriptions)
        .where(and(eq(subscriptions.id, id), isNull(subscriptions.deletedAt)));

      if (!existing) return NextResponse.json({ error: "Not found" }, { status: 404 });
      return NextResponse.json(
        { error: "Conflict: subscription was modified by another request", currentVersion: existing.version },
        { status: 409 },
      );
    }

    return NextResponse.json(result[0]);
  } catch (error) {
    return handleApiError(error, "PUT /api/billing/subscriptions/[id]");
  }
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;

    // Soft delete
    const [row] = await db
      .update(subscriptions)
      .set({ deletedAt: new Date(), updatedAt: new Date() })
      .where(and(eq(subscriptions.id, id), isNull(subscriptions.deletedAt)))
      .returning();

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (error) {
    return handleApiError(error, "DELETE /api/billing/subscriptions/[id]");
  }
}
