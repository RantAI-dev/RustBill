import { db } from "@/lib/db";
import { invoices, payments } from "@/lib/db/schema";
import { verifyXenditWebhook } from "@/lib/billing/xendit";
import { handleApiError } from "@/lib/api-utils";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

/**
 * Xendit webhook handler.
 * Xendit sends callback notifications for invoice status changes.
 * Authenticated via x-callback-token header.
 */
export async function POST(req: NextRequest) {
  try {
    // Verify webhook authenticity
    const callbackToken = req.headers.get("x-callback-token");
    if (!(await verifyXenditWebhook(callbackToken))) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }

    const body = await req.json();

    // Xendit invoice callbacks use external_id to reference our invoice
    const externalId = body.external_id as string | undefined;
    const xenditInvoiceId = body.id as string | undefined;
    const status = body.status as string | undefined;

    if (!externalId || !xenditInvoiceId) {
      return NextResponse.json({ error: "Missing external_id or id" }, { status: 400 });
    }

    if (status === "PAID" || status === "SETTLED") {
      await db.transaction(async (tx) => {
        const [invoice] = await tx
          .select()
          .from(invoices)
          .where(eq(invoices.id, externalId));
        if (!invoice || invoice.status === "paid") return;

        // Idempotency: check if payment already recorded by xendit payment ID
        const xenditPaymentId = (body.payment_id as string) ?? xenditInvoiceId;
        const [existing] = await tx
          .select()
          .from(payments)
          .where(eq(payments.xenditPaymentId, xenditPaymentId));
        if (existing) return;

        const paidAmount = Number(body.paid_amount ?? body.amount ?? 0);

        await tx.insert(payments).values({
          invoiceId: invoice.id,
          amount: paidAmount,
          method: "xendit",
          reference: xenditPaymentId,
          paidAt: new Date(),
          xenditPaymentId,
        });

        await tx
          .update(invoices)
          .set({
            status: "paid",
            paidAt: new Date(),
            xenditInvoiceId,
            updatedAt: new Date(),
          })
          .where(eq(invoices.id, invoice.id));
      });
    } else if (status === "EXPIRED") {
      await db.transaction(async (tx) => {
        const [invoice] = await tx
          .select()
          .from(invoices)
          .where(eq(invoices.id, externalId));
        if (!invoice || invoice.status === "paid") return;

        await tx
          .update(invoices)
          .set({ status: "overdue", updatedAt: new Date() })
          .where(eq(invoices.id, invoice.id));
      });
    }

    return NextResponse.json({ received: true });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/xendit");
  }
}
