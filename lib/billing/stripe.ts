import Stripe from "stripe";

const STRIPE_SECRET_KEY = process.env.STRIPE_SECRET_KEY;

export const isStripeEnabled = !!STRIPE_SECRET_KEY;
export const STRIPE_WEBHOOK_SECRET = process.env.STRIPE_WEBHOOK_SECRET ?? "";

export const stripe = isStripeEnabled
  ? new Stripe(STRIPE_SECRET_KEY!, { apiVersion: "2026-01-28.clover" })
  : null;

/**
 * Create a Stripe Checkout session for an invoice.
 * Returns the checkout URL, or null if Stripe is not configured.
 */
export async function createCheckoutSession(params: {
  invoiceId: string;
  invoiceNumber: string;
  total: number;
  currency: string;
  customerEmail?: string;
  stripeCustomerId?: string | null;
  successUrl: string;
  cancelUrl: string;
}): Promise<string | null> {
  if (!stripe) return null;

  const session = await stripe.checkout.sessions.create({
    mode: "payment",
    customer: params.stripeCustomerId || undefined,
    customer_email: params.stripeCustomerId ? undefined : params.customerEmail,
    line_items: [
      {
        price_data: {
          currency: params.currency.toLowerCase(),
          unit_amount: Math.round(params.total * 100),
          product_data: { name: `Invoice ${params.invoiceNumber}` },
        },
        quantity: 1,
      },
    ],
    metadata: { invoiceId: params.invoiceId },
    success_url: params.successUrl,
    cancel_url: params.cancelUrl,
  });

  return session.url;
}

/**
 * Create a Stripe refund for a payment intent.
 * Returns the Stripe refund object, or null if Stripe is not configured.
 */
export async function createStripeRefund(
  paymentIntentId: string,
  amountInCents: number,
): Promise<Stripe.Refund | null> {
  if (!stripe) return null;

  return stripe.refunds.create({
    payment_intent: paymentIntentId,
    amount: amountInCents,
  });
}
