import { db } from "@/lib/db";
import { webhookEndpoints, webhookDeliveries, billingEvents } from "@/lib/db/schema";
import { eq, desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

const updateWebhookSchema = z.object({
  url: z.string().url().optional(),
  description: z.string().nullable().optional(),
  events: z.array(z.string()).min(1).optional(),
  status: z.enum(["active", "inactive"]).optional(),
});

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const { id } = await params;

    const [endpoint] = await db
      .select()
      .from(webhookEndpoints)
      .where(eq(webhookEndpoints.id, id));

    if (!endpoint) return NextResponse.json({ error: "Not found" }, { status: 404 });

    // Get recent deliveries
    const deliveries = await db
      .select({
        delivery: webhookDeliveries,
        eventType: billingEvents.eventType,
      })
      .from(webhookDeliveries)
      .leftJoin(billingEvents, eq(webhookDeliveries.eventId, billingEvents.id))
      .where(eq(webhookDeliveries.endpointId, id))
      .orderBy(desc(webhookDeliveries.createdAt))
      .limit(50);

    return NextResponse.json({
      ...endpoint,
      deliveries: deliveries.map((d) => ({
        ...d.delivery,
        eventType: d.eventType,
      })),
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/webhooks/[id]");
  }
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const body = await req.json();
    const parsed = updateWebhookSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const [row] = await db
      .update(webhookEndpoints)
      .set({ ...parsed.data, updatedAt: new Date() })
      .where(eq(webhookEndpoints.id, id))
      .returning();

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
    return NextResponse.json(row);
  } catch (error) {
    return handleApiError(error, "PUT /api/billing/webhooks/[id]");
  }
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const [row] = await db
      .delete(webhookEndpoints)
      .where(eq(webhookEndpoints.id, id))
      .returning();

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (error) {
    return handleApiError(error, "DELETE /api/billing/webhooks/[id]");
  }
}
