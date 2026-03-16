import { db } from "@/lib/db";
import { invoices, payments, customers, subscriptions } from "@/lib/db/schema";
import { stripe, STRIPE_WEBHOOK_SECRET, isStripeEnabled, createCheckoutSession } from "@/lib/billing/stripe";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

// Stripe webhook handler
export async function POST(req: NextRequest) {
  try {
    if (!isStripeEnabled || !stripe) {
      return NextResponse.json(
        { error: "Stripe is not configured" },
        { status: 503 },
      );
    }

    // Verify webhook signature
    const rawBody = await req.text();
    const sig = req.headers.get("stripe-signature");
    if (!sig) {
      return NextResponse.json({ error: "Missing stripe-signature header" }, { status: 400 });
    }

    let event;
    try {
      event = stripe.webhooks.constructEvent(rawBody, sig, STRIPE_WEBHOOK_SECRET);
    } catch (err) {
      console.error("Stripe signature verification failed:", err);
      return NextResponse.json({ error: "Invalid signature" }, { status: 400 });
    }

    switch (event.type) {
      case "checkout.session.completed": {
        const session = event.data.object;
        if (session.metadata?.invoiceId) {
          await db.transaction(async (tx) => {
            const [invoice] = await tx
              .select()
              .from(invoices)
              .where(eq(invoices.id, session.metadata!.invoiceId!));
            if (!invoice) return;

            const stripePaymentIntentId = (session.payment_intent as string) ?? null;

            // Idempotency: check if payment already recorded
            if (stripePaymentIntentId) {
              const [existing] = await tx
                .select()
                .from(payments)
                .where(eq(payments.stripePaymentIntentId, stripePaymentIntentId));
              if (existing) return;
            }

            await tx.insert(payments).values({
              invoiceId: invoice.id,
              amount: (session.amount_total ?? 0) / 100,
              method: "stripe",
              reference: stripePaymentIntentId ?? session.id,
              paidAt: new Date(),
              stripePaymentIntentId,
            });

            await tx
              .update(invoices)
              .set({
                status: "paid",
                paidAt: new Date(),
                stripeInvoiceId: (session.invoice as string) ?? null,
                updatedAt: new Date(),
              })
              .where(eq(invoices.id, invoice.id));
          });
        }
        break;
      }

      case "invoice.paid": {
        const stripeInvoice = event.data.object;
        if (stripeInvoice.id) {
          await db.transaction(async (tx) => {
            const [invoice] = await tx
              .select()
              .from(invoices)
              .where(eq(invoices.stripeInvoiceId, stripeInvoice.id!));
            if (!invoice || invoice.status === "paid") return;

            const stripePaymentIntentId = ((stripeInvoice as unknown as Record<string, unknown>).payment_intent as string) ?? null;

            // Idempotency check
            if (stripePaymentIntentId) {
              const [existing] = await tx
                .select()
                .from(payments)
                .where(eq(payments.stripePaymentIntentId, stripePaymentIntentId));
              if (existing) return;
            }

            await tx.insert(payments).values({
              invoiceId: invoice.id,
              amount: (stripeInvoice.amount_paid ?? 0) / 100,
              method: "stripe",
              reference: ((stripeInvoice as unknown as Record<string, unknown>).charge as string) ?? stripePaymentIntentId,
              paidAt: new Date(),
              stripePaymentIntentId,
            });

            await tx
              .update(invoices)
              .set({
                status: "paid",
                paidAt: new Date(),
                updatedAt: new Date(),
              })
              .where(eq(invoices.id, invoice.id));
          });
        }
        break;
      }

      case "invoice.payment_failed": {
        const stripeInvoice = event.data.object;
        if (stripeInvoice.id) {
          await db.transaction(async (tx) => {
            const [invoice] = await tx
              .select()
              .from(invoices)
              .where(eq(invoices.stripeInvoiceId, stripeInvoice.id!));
            if (!invoice) return;

            await tx
              .update(invoices)
              .set({ status: "overdue", updatedAt: new Date() })
              .where(eq(invoices.id, invoice.id));

            // Mark linked subscription as past_due
            if (invoice.subscriptionId) {
              await tx
                .update(subscriptions)
                .set({ status: "past_due", updatedAt: new Date() })
                .where(eq(subscriptions.id, invoice.subscriptionId));
            }
          });
        }
        break;
      }

      case "customer.subscription.deleted": {
        const stripeSub = event.data.object;
        if (stripeSub.id) {
          await db.transaction(async (tx) => {
            const [sub] = await tx
              .select()
              .from(subscriptions)
              .where(eq(subscriptions.stripeSubscriptionId, stripeSub.id));
            if (!sub) return;

            await tx
              .update(subscriptions)
              .set({
                status: "canceled",
                canceledAt: new Date(),
                updatedAt: new Date(),
              })
              .where(eq(subscriptions.id, sub.id));
          });
        }
        break;
      }

      case "charge.refunded": {
        // Handle Stripe-initiated refunds
        // Can be expanded to auto-create refund records
        break;
      }

      default:
        // Unhandled event type
        break;
    }

    return NextResponse.json({ received: true });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/stripe");
  }
}

// GET: Create a Stripe Checkout session URL for an invoice
export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");

    if (!invoiceId) {
      return NextResponse.json({ error: "invoiceId is required" }, { status: 400 });
    }

    const [invoice] = await db.select().from(invoices).where(eq(invoices.id, invoiceId));
    if (!invoice) return NextResponse.json({ error: "Invoice not found" }, { status: 404 });

    const [customer] = await db
      .select()
      .from(customers)
      .where(eq(customers.id, invoice.customerId));

    // Use real Stripe checkout when enabled
    if (isStripeEnabled) {
      const origin = req.headers.get("origin") ?? req.nextUrl.origin;
      const checkoutUrl = await createCheckoutSession({
        invoiceId: invoice.id,
        invoiceNumber: invoice.invoiceNumber,
        total: Number(invoice.total),
        currency: invoice.currency,
        customerEmail: customer?.billingEmail ?? customer?.email,
        stripeCustomerId: customer?.stripeCustomerId,
        successUrl: `${origin}/billing?paid=${invoice.id}`,
        cancelUrl: `${origin}/billing?canceled=${invoice.id}`,
      });

      if (checkoutUrl) {
        return NextResponse.json({ checkoutUrl });
      }
    }

    // Fallback stub when Stripe is not configured
    return NextResponse.json({
      message: "Stripe integration pending — set STRIPE_SECRET_KEY to enable",
      invoice: {
        id: invoice.id,
        invoiceNumber: invoice.invoiceNumber,
        total: invoice.total,
        currency: invoice.currency,
      },
      customer: customer
        ? {
            name: customer.name,
            email: customer.billingEmail ?? customer.email,
            stripeCustomerId: customer.stripeCustomerId,
          }
        : null,
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/stripe");
  }
}
