import { db } from "@/lib/db";
import { invoices, invoiceItems, payments, customers } from "@/lib/db/schema";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { id } = await params;
  const [row] = await db
    .select()
    .from(invoices)
    .leftJoin(customers, eq(invoices.customerId, customers.id))
    .where(eq(invoices.id, id));

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

  const items = await db.select().from(invoiceItems).where(eq(invoiceItems.invoiceId, id));
  const pmts = await db.select().from(payments).where(eq(payments.invoiceId, id));

  return NextResponse.json({
    ...row.invoices,
    customerName: row.customers?.name ?? null,
    items,
    payments: pmts,
  });
}
