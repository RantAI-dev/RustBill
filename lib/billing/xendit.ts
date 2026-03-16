import Xendit from "xendit-node";
import { getProviderSetting, PROVIDER_KEYS } from "./provider-settings";

/**
 * Get Xendit client lazily — reads from DB settings first, env fallback.
 */
async function getXenditClient(): Promise<InstanceType<typeof Xendit> | null> {
  const secretKey = await getProviderSetting(PROVIDER_KEYS.XENDIT_SECRET_KEY);
  if (!secretKey) return null;
  return new Xendit({ secretKey });
}

/**
 * Check if Xendit is configured (has secret key).
 */
export async function isXenditEnabled(): Promise<boolean> {
  const key = await getProviderSetting(PROVIDER_KEYS.XENDIT_SECRET_KEY);
  return !!key;
}

/**
 * Create a Xendit Invoice (their equivalent of a checkout session).
 * Returns the invoice URL for customer redirect, or null if Xendit is not configured.
 */
export async function createXenditInvoice(params: {
  invoiceId: string;
  invoiceNumber: string;
  total: number;
  currency: string;
  customerEmail?: string;
  customerName?: string;
  successUrl: string;
  failureUrl: string;
}): Promise<{ invoiceUrl: string; xenditInvoiceId: string } | null> {
  const client = await getXenditClient();
  if (!client) return null;

  const response = await client.Invoice.createInvoice({
    data: {
      externalId: params.invoiceId,
      amount: params.total,
      currency: params.currency.toUpperCase(),
      description: `Payment for invoice ${params.invoiceNumber}`,
      payerEmail: params.customerEmail,
      successRedirectUrl: params.successUrl,
      failureRedirectUrl: params.failureUrl,
    },
  });

  return {
    invoiceUrl: response.invoiceUrl,
    xenditInvoiceId: response.id!,
  };
}

/**
 * Create a Xendit refund.
 * Returns the refund ID, or null if Xendit is not configured.
 */
export async function createXenditRefund(
  xenditPaymentId: string,
  amount: number,
  currency: string,
  reason?: string,
): Promise<string | null> {
  const client = await getXenditClient();
  if (!client) return null;

  const { CreateRefundReasonEnum } = await import("xendit-node/refund/models");
  const reasonMap: Record<string, (typeof CreateRefundReasonEnum)[keyof typeof CreateRefundReasonEnum]> = {
    fraudulent: CreateRefundReasonEnum.Fraudulent,
    duplicate: CreateRefundReasonEnum.Duplicate,
    requested_by_customer: CreateRefundReasonEnum.RequestedByCustomer,
    cancellation: CreateRefundReasonEnum.Cancellation,
    others: CreateRefundReasonEnum.Others,
  };
  const reasonEnum = (reason ? reasonMap[reason.toLowerCase()] : undefined) ?? CreateRefundReasonEnum.RequestedByCustomer;

  const response = await client.Refund.createRefund({
    data: {
      paymentRequestId: xenditPaymentId,
      amount,
      currency: currency.toUpperCase(),
      reason: reasonEnum,
    },
  });

  return response.id ?? null;
}

/**
 * Verify Xendit webhook authenticity by checking the x-callback-token header.
 */
export async function verifyXenditWebhook(callbackToken: string | null): Promise<boolean> {
  const webhookToken = await getProviderSetting(PROVIDER_KEYS.XENDIT_WEBHOOK_TOKEN);
  if (!webhookToken) return false;
  return callbackToken === webhookToken;
}
