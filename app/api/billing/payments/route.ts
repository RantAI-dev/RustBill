import { db } from "@/lib/db";
import { payments, invoices, customers } from "@/lib/db/schema";
import { insertPaymentSchema } from "@/lib/validations/billing";
import { withAuth, withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { notifyPaymentReceived, notifyInvoicePaid } from "@/lib/billing/notifications";
import { desc, eq, and, sql, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");

    const conditions = [];
    if (invoiceId) conditions.push(eq(payments.invoiceId, invoiceId));

    // Customer isolation: customers can only see payments for their own invoices
    if (auth.user.role === "customer" && auth.user.customerId) {
      const customerInvoiceIds = db
        .select({ id: invoices.id })
        .from(invoices)
        .where(eq(invoices.customerId, auth.user.customerId));

      conditions.push(sql`${payments.invoiceId} IN (${customerInvoiceIds})`);
    }

    const rows = conditions.length > 0
      ? await db.select().from(payments).where(and(...conditions)).orderBy(desc(payments.createdAt))
      : await db.select().from(payments).orderBy(desc(payments.createdAt));

    return NextResponse.json(rows);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/payments");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertPaymentSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const result = await db.transaction(async (tx) => {
      // Verify invoice exists
      const [invoice] = await tx
        .select()
        .from(invoices)
        .where(and(eq(invoices.id, parsed.data.invoiceId), isNull(invoices.deletedAt)));
      if (!invoice) return null;

      // Idempotency check: if stripePaymentIntentId is provided, check for duplicates
      if (parsed.data.stripePaymentIntentId) {
        const [existing] = await tx
          .select()
          .from(payments)
          .where(eq(payments.stripePaymentIntentId, parsed.data.stripePaymentIntentId));
        if (existing) return { payment: existing, invoice, fullyPaid: false };
      }

      const [newPayment] = await tx
        .insert(payments)
        .values({
          ...parsed.data,
          paidAt: new Date(parsed.data.paidAt),
        })
        .returning();

      // Check if invoice is fully paid (amounts are strings from numeric columns)
      const [totals] = await tx
        .select({ totalPaid: sql<string>`COALESCE(SUM(${payments.amount}), 0)` })
        .from(payments)
        .where(eq(payments.invoiceId, invoice.id));

      let fullyPaid = false;
      if (Number(totals.totalPaid) >= Number(invoice.total)) {
        await tx
          .update(invoices)
          .set({ status: "paid", paidAt: new Date(), updatedAt: new Date() })
          .where(eq(invoices.id, invoice.id));
        fullyPaid = true;
      }

      return { payment: newPayment, invoice, fullyPaid };
    });

    if (!result) {
      return NextResponse.json({ error: "Invoice not found" }, { status: 404 });
    }

    // Send notifications (non-blocking)
    const [customer] = await db
      .select()
      .from(customers)
      .where(eq(customers.id, result.invoice.customerId));

    if (customer) {
      const ctx = {
        customerId: customer.id,
        customerName: customer.name,
        customerEmail: customer.billingEmail ?? customer.email,
      };

      notifyPaymentReceived({
        paymentId: result.payment.id,
        invoiceId: result.invoice.id,
        ...ctx,
        amount: Number(result.payment.amount),
        method: result.payment.method,
      }).catch(() => {});

      if (result.fullyPaid) {
        notifyInvoicePaid({
          invoiceId: result.invoice.id,
          invoiceNumber: result.invoice.invoiceNumber,
          ...ctx,
          total: Number(result.invoice.total),
          currency: result.invoice.currency,
          paidAt: new Date().toISOString(),
        }).catch(() => {});
      }
    }

    return NextResponse.json(result.payment, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/payments");
  }
}
