import { db } from "@/lib/db";
import { refunds, payments, invoices } from "@/lib/db/schema";
import { insertRefundSchema } from "@/lib/validations/billing";
import { withAuth, withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { desc, eq, and, sql } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");
    const paymentId = searchParams.get("paymentId");

    const conditions = [];
    if (invoiceId) conditions.push(eq(refunds.invoiceId, invoiceId));
    if (paymentId) conditions.push(eq(refunds.paymentId, paymentId));

    // Customer isolation: customers can only see refunds for their own invoices
    if (auth.user.role === "customer" && auth.user.customerId) {
      const customerInvoiceIds = db
        .select({ id: invoices.id })
        .from(invoices)
        .where(eq(invoices.customerId, auth.user.customerId));

      conditions.push(sql`${refunds.invoiceId} IN (${customerInvoiceIds})`);
    }

    const rows = conditions.length > 0
      ? await db.select().from(refunds).where(and(...conditions)).orderBy(desc(refunds.createdAt))
      : await db.select().from(refunds).orderBy(desc(refunds.createdAt));

    return NextResponse.json(rows);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/refunds");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertRefundSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const result = await db.transaction(async (tx) => {
      // Lock the payment row to prevent race conditions
      const lockedRows = await tx.execute(
        sql`SELECT * FROM payments WHERE id = ${parsed.data.paymentId} FOR UPDATE`
      );
      const payment = (lockedRows as unknown as Record<string, unknown>[])?.[0];
      if (!payment) {
        return { error: "Payment not found", status: 404 } as const;
      }

      // Verify refund amount doesn't exceed payment (amounts are strings from numeric columns)
      const [existingRefunds] = await tx
        .select({ total: sql<string>`COALESCE(SUM(${refunds.amount}), 0)` })
        .from(refunds)
        .where(eq(refunds.paymentId, parsed.data.paymentId));

      const alreadyRefunded = Number(existingRefunds?.total ?? 0);
      const paymentAmount = Number(payment.amount);
      if (alreadyRefunded + parsed.data.amount > paymentAmount) {
        return {
          error: `Refund exceeds payment amount. Already refunded: $${alreadyRefunded}, payment: $${paymentAmount}`,
          status: 400,
        } as const;
      }

      const [refund] = await tx
        .insert(refunds)
        .values({
          ...parsed.data,
          processedAt: parsed.data.status === "completed" ? new Date() : null,
        })
        .returning();

      // If refund is completed, check if invoice needs status update
      if (parsed.data.status === "completed") {
        const [invoice] = await tx
          .select()
          .from(invoices)
          .where(eq(invoices.id, parsed.data.invoiceId));

        if (invoice && invoice.status === "paid") {
          const [paymentTotals] = await tx
            .select({ total: sql<string>`COALESCE(SUM(${payments.amount}), 0)` })
            .from(payments)
            .where(eq(payments.invoiceId, invoice.id));

          const [refundTotals] = await tx
            .select({ total: sql<string>`COALESCE(SUM(${refunds.amount}), 0)` })
            .from(refunds)
            .where(eq(refunds.invoiceId, invoice.id));

          const netPaid = Number(paymentTotals?.total ?? 0) - Number(refundTotals?.total ?? 0);

          // If net paid is less than total, revert invoice to issued
          if (netPaid < Number(invoice.total)) {
            await tx
              .update(invoices)
              .set({ status: "issued", paidAt: null, updatedAt: new Date() })
              .where(eq(invoices.id, invoice.id));
          }
        }
      }

      return { data: refund } as const;
    });

    if ("error" in result) {
      return NextResponse.json({ error: result.error }, { status: result.status });
    }

    return NextResponse.json(result.data, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/refunds");
  }
}
