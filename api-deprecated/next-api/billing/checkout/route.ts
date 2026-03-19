import { db } from "@/lib/db";
import { invoices, customers } from "@/lib/db/schema";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { isStripeEnabled, createCheckoutSession } from "@/lib/billing/stripe";
import { isXenditEnabled, createXenditInvoice } from "@/lib/billing/xendit";
import { isLemonsqueezyEnabled, createLemonsqueezyCheckout } from "@/lib/billing/lemonsqueezy";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

/**
 * Unified checkout endpoint.
 * GET /api/billing/checkout?invoiceId=xxx&provider=stripe|xendit|lemonsqueezy
 *
 * Returns { checkoutUrl } for the requested provider.
 */
export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");
    const provider = searchParams.get("provider");

    if (!invoiceId) {
      return NextResponse.json({ error: "invoiceId is required" }, { status: 400 });
    }
    if (!provider || !["stripe", "xendit", "lemonsqueezy"].includes(provider)) {
      return NextResponse.json(
        { error: "provider must be one of: stripe, xendit, lemonsqueezy" },
        { status: 400 },
      );
    }

    const [invoice] = await db.select().from(invoices).where(eq(invoices.id, invoiceId));
    if (!invoice) {
      return NextResponse.json({ error: "Invoice not found" }, { status: 404 });
    }

    if (invoice.status === "paid") {
      return NextResponse.json({ error: "Invoice is already paid" }, { status: 400 });
    }

    const [customer] = await db
      .select()
      .from(customers)
      .where(eq(customers.id, invoice.customerId));

    const origin = req.headers.get("origin") ?? req.nextUrl.origin;
    const successUrl = `${origin}/billing?paid=${invoice.id}`;
    const cancelUrl = `${origin}/billing?canceled=${invoice.id}`;

    // --- Stripe ---
    if (provider === "stripe") {
      if (!isStripeEnabled) {
        return NextResponse.json(
          { error: "Stripe is not configured. Set STRIPE_SECRET_KEY to enable." },
          { status: 503 },
        );
      }

      const checkoutUrl = await createCheckoutSession({
        invoiceId: invoice.id,
        invoiceNumber: invoice.invoiceNumber,
        total: Number(invoice.total),
        currency: invoice.currency,
        customerEmail: customer?.billingEmail ?? customer?.email,
        stripeCustomerId: customer?.stripeCustomerId,
        successUrl,
        cancelUrl,
      });

      if (checkoutUrl) {
        return NextResponse.json({ checkoutUrl, provider: "stripe" });
      }
      return NextResponse.json({ error: "Failed to create Stripe checkout" }, { status: 500 });
    }

    // --- Xendit ---
    if (provider === "xendit") {
      if (!(await isXenditEnabled())) {
        return NextResponse.json(
          { error: "Xendit is not configured. Add your Xendit credentials in Settings > Payment Providers." },
          { status: 503 },
        );
      }

      const result = await createXenditInvoice({
        invoiceId: invoice.id,
        invoiceNumber: invoice.invoiceNumber,
        total: Number(invoice.total),
        currency: invoice.currency,
        customerEmail: customer?.billingEmail ?? customer?.email,
        customerName: customer?.name,
        successUrl,
        failureUrl: cancelUrl,
      });

      if (result) {
        // Store xendit invoice ID on our invoice for lookup during webhook
        await db
          .update(invoices)
          .set({ xenditInvoiceId: result.xenditInvoiceId, updatedAt: new Date() })
          .where(eq(invoices.id, invoice.id));

        return NextResponse.json({ checkoutUrl: result.invoiceUrl, provider: "xendit" });
      }
      return NextResponse.json({ error: "Failed to create Xendit invoice" }, { status: 500 });
    }

    // --- Lemonsqueezy ---
    if (provider === "lemonsqueezy") {
      if (!(await isLemonsqueezyEnabled())) {
        return NextResponse.json(
          { error: "Lemonsqueezy is not configured. Add your Lemonsqueezy credentials in Settings > Payment Providers." },
          { status: 503 },
        );
      }

      const result = await createLemonsqueezyCheckout({
        invoiceId: invoice.id,
        invoiceNumber: invoice.invoiceNumber,
        total: Number(invoice.total),
        currency: invoice.currency,
        customerEmail: customer?.billingEmail ?? customer?.email,
        customerName: customer?.name,
        successUrl,
      });

      if (result) {
        return NextResponse.json({ checkoutUrl: result.checkoutUrl, provider: "lemonsqueezy" });
      }
      return NextResponse.json({ error: "Failed to create Lemonsqueezy checkout" }, { status: 500 });
    }

    return NextResponse.json({ error: "Unknown provider" }, { status: 400 });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/checkout");
  }
}
