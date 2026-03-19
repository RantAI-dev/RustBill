import { db } from "@/lib/db";
import { invoiceItems, invoices } from "@/lib/db/schema";
import { insertInvoiceItemSchema } from "@/lib/validations/billing";
import { eq, sql } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function POST(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;

  // Verify invoice exists
  const [invoice] = await db.select().from(invoices).where(eq(invoices.id, id));
  if (!invoice) return NextResponse.json({ error: "Invoice not found" }, { status: 404 });

  const body = await req.json();
  const parsed = insertInvoiceItemSchema.safeParse({ ...body, invoiceId: id });
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const amount = parsed.data.quantity * parsed.data.unitPrice;
  const [item] = await db
    .insert(invoiceItems)
    .values({
      ...parsed.data,
      amount,
      periodStart: parsed.data.periodStart ? new Date(parsed.data.periodStart) : null,
      periodEnd: parsed.data.periodEnd ? new Date(parsed.data.periodEnd) : null,
    })
    .returning();

  // Update invoice totals
  await db
    .update(invoices)
    .set({
      subtotal: sql`${invoices.subtotal} + ${amount}`,
      total: sql`${invoices.subtotal} + ${amount} + ${invoices.tax}`,
      updatedAt: new Date(),
    })
    .where(eq(invoices.id, id));

  return NextResponse.json(item, { status: 201 });
}
