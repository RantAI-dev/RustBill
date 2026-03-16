import { db } from "@/lib/db";
import { billingEvents, customers } from "@/lib/db/schema";
import { desc, eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url);
  const customerId = searchParams.get("customerId");
  const eventType = searchParams.get("eventType");
  const limit = Math.min(Number(searchParams.get("limit") ?? 100), 500);

  let query = db
    .select({
      event: billingEvents,
      customerName: customers.name,
    })
    .from(billingEvents)
    .leftJoin(customers, eq(billingEvents.customerId, customers.id))
    .orderBy(desc(billingEvents.createdAt))
    .limit(limit)
    .$dynamic();

  if (customerId) {
    query = query.where(eq(billingEvents.customerId, customerId));
  }
  if (eventType) {
    query = query.where(eq(billingEvents.eventType, eventType as typeof billingEvents.eventType.enumValues[number]));
  }

  const rows = await query;

  return NextResponse.json(
    rows.map((r) => ({
      ...r.event,
      customerName: r.customerName,
    }))
  );
}
