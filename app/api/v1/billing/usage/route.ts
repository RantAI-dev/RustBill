import { db } from "@/lib/db";
import { usageEvents, subscriptions } from "@/lib/db/schema";
import { insertUsageEventSchema } from "@/lib/validations/billing";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq, and, gte, lte, sql } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { searchParams } = new URL(req.url);
  const subscriptionId = searchParams.get("subscriptionId");
  const metricName = searchParams.get("metricName");
  const from = searchParams.get("from");
  const to = searchParams.get("to");

  if (!subscriptionId) {
    return NextResponse.json({ error: "subscriptionId is required" }, { status: 400 });
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
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const body = await req.json();

  // Support batch events
  const events = Array.isArray(body) ? body : [body];
  const results = [];

  for (const event of events) {
    const parsed = insertUsageEventSchema.safeParse(event);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten(), event }, { status: 400 });
    }

    // Verify subscription exists
    const [sub] = await db
      .select()
      .from(subscriptions)
      .where(eq(subscriptions.id, parsed.data.subscriptionId));
    if (!sub) {
      return NextResponse.json({ error: "Subscription not found", subscriptionId: parsed.data.subscriptionId }, { status: 404 });
    }

    const [inserted] = await db
      .insert(usageEvents)
      .values({
        ...parsed.data,
        timestamp: parsed.data.timestamp ? new Date(parsed.data.timestamp) : new Date(),
      })
      .returning();

    results.push(inserted);
  }

  return NextResponse.json(results.length === 1 ? results[0] : results, { status: 201 });
}
