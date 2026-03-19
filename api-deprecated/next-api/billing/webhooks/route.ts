import { db } from "@/lib/db";
import { webhookEndpoints } from "@/lib/db/schema";
import { desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { z } from "zod";
import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

const insertWebhookSchema = z.object({
  url: z.string().url("Valid URL required"),
  description: z.string().optional(),
  events: z.array(z.string()).min(1, "At least one event type is required"),
  status: z.enum(["active", "inactive"]).default("active"),
});

export async function GET() {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const rows = await db
      .select()
      .from(webhookEndpoints)
      .orderBy(desc(webhookEndpoints.createdAt));

    return NextResponse.json(rows);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/webhooks");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertWebhookSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    // Generate a random webhook secret
    const secret = `whsec_${crypto.randomUUID().replace(/-/g, "")}`;

    const [row] = await db.insert(webhookEndpoints).values({
      ...parsed.data,
      secret,
    }).returning();

    return NextResponse.json(row, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/webhooks");
  }
}
