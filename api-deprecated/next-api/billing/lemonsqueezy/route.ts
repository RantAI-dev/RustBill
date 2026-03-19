import { db } from "@/lib/db";
import { invoices, payments } from "@/lib/db/schema";
import { verifyLemonsqueezyWebhook } from "@/lib/billing/lemonsqueezy";
import { handleApiError } from "@/lib/api-utils";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

/**
 * Lemonsqueezy webhook handler.
 * Verified via HMAC-SHA256 signature in x-signature header.
 */
export async function POST(req: NextRequest) {
  try {
    const rawBody = await req.text();
    const signature = req.headers.get("x-signature");

    if (!(await verifyLemonsqueezyWebhook(rawBody, signature))) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }

    const body = JSON.parse(rawBody);
    const eventName = body.meta?.event_name as string | undefined;
    const customData = body.meta?.custom_data as Record<string, string> | undefined;
    const orderId = body.data?.id as string | undefined;
    const attributes = body.data?.attributes as Record<string, unknown> | undefined;

    if (!eventName) {
      return NextResponse.json({ error: "Missing event_name" }, { status: 400 });
    }

    if (eventName === "order_created") {
      const invoiceId = customData?.invoiceId;
      if (!invoiceId || !orderId) {
        return NextResponse.json({ received: true }); // Not our invoice, ignore
      }

      await db.transaction(async (tx) => {
        const [invoice] = await tx
          .select()
          .from(invoices)
          .where(eq(invoices.id, invoiceId));
        if (!invoice || invoice.status === "paid") return;

        // Idempotency: check if payment already recorded
        const [existing] = await tx
          .select()
          .from(payments)
          .where(eq(payments.lemonsqueezyOrderId, orderId));
        if (existing) return;

        const totalInCents = Number(attributes?.total ?? 0);
        const paidAmount = totalInCents / 100;

        await tx.insert(payments).values({
          invoiceId: invoice.id,
          amount: paidAmount,
          method: "lemonsqueezy",
          reference: orderId,
          paidAt: new Date(),
          lemonsqueezyOrderId: orderId,
        });

        await tx
          .update(invoices)
          .set({
            status: "paid",
            paidAt: new Date(),
            lemonsqueezyOrderId: orderId,
            updatedAt: new Date(),
          })
          .where(eq(invoices.id, invoice.id));
      });
    } else if (eventName === "order_refunded") {
      // Refund handling can be expanded later
      // For now, just acknowledge receipt
    }

    return NextResponse.json({ received: true });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/lemonsqueezy");
  }
}
