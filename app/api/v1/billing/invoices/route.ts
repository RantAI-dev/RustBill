import { db } from "@/lib/db";
import { invoices, customers } from "@/lib/db/schema";
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
  if (status) conditions.push(eq(invoices.status, status as "draft" | "issued" | "paid" | "overdue" | "void"));
  if (customerId) conditions.push(eq(invoices.customerId, customerId));

  const query = db
    .select()
    .from(invoices)
    .leftJoin(customers, eq(invoices.customerId, customers.id));

  const rows = conditions.length > 0
    ? await query.where(and(...conditions)).orderBy(desc(invoices.createdAt))
    : await query.orderBy(desc(invoices.createdAt));

  const mapped = rows.map((r) => ({
    ...r.invoices,
    customerName: r.customers?.name ?? null,
  }));

  return NextResponse.json(mapped);
}
