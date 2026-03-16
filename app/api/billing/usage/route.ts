import { db } from "@/lib/db";
import { usageEvents, subscriptions } from "@/lib/db/schema";
import { insertUsageEventSchema } from "@/lib/validations/billing";
import { eq, and, gte, lte, sql } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const subscriptionId = searchParams.get("subscriptionId");
    const metricName = searchParams.get("metricName");
    const from = searchParams.get("from");
    const to = searchParams.get("to");

    if (!subscriptionId) {
      return NextResponse.json({ error: "subscriptionId is required" }, { status: 400 });
    }

    // Customer isolation: verify the subscription belongs to the user's customer
    if (auth.user.role !== "admin") {
      const [sub] = await db
        .select({ customerId: subscriptions.customerId })
        .from(subscriptions)
        .where(eq(subscriptions.id, subscriptionId));

      if (!sub || sub.customerId !== auth.user.customerId) {
        return NextResponse.json({ error: "Forbidden" }, { status: 403 });
      }
    }

    const conditions = [eq(usageEvents.subscriptionId, subscriptionId)];
    if (metricName) conditions.push(eq(usageEvents.metricName, metricName));
    if (from) conditions.push(gte(usageEvents.timestamp, new Date(from)));
    if (to) conditions.push(lte(usageEvents.timestamp, new Date(to)));

    const aggregated = await db
      .select({
        metricName: usageEvents.metricName,
        totalValue: sql<number>`SUM(${usageEvents.value})`,
        count: sql<number>`COUNT(*)`,
      })
      .from(usageEvents)
      .where(and(...conditions))
      .groupBy(usageEvents.metricName);

    return NextResponse.json(aggregated);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/usage");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertUsageEventSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    // Verify subscription exists and belongs to the user's customer
    const [sub] = await db
      .select()
      .from(subscriptions)
      .where(eq(subscriptions.id, parsed.data.subscriptionId));
    if (!sub) return NextResponse.json({ error: "Subscription not found" }, { status: 404 });

    if (auth.user.role !== "admin" && sub.customerId !== auth.user.customerId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }

    const [event] = await db
      .insert(usageEvents)
      .values({
        ...parsed.data,
        timestamp: parsed.data.timestamp ? new Date(parsed.data.timestamp) : new Date(),
      })
      .returning();

    return NextResponse.json(event, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/usage");
  }
}
