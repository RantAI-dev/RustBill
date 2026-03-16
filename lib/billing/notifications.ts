import { emitBillingEvent } from "./events";
import { sendBillingEmail } from "./email";
import {
  invoiceCreatedHtml,
  invoiceIssuedHtml,
  invoicePaidHtml,
  invoiceOverdueHtml,
  paymentReceivedHtml,
  paymentRefundedHtml,
  dunningReminderHtml,
  subscriptionEventHtml,
} from "./email-templates";

/**
 * Billing notification service.
 * Emits billing events (triggering webhooks) AND sends emails via Resend.
 * Email sending is non-blocking — failures are logged but don't break the flow.
 */

interface InvoiceContext {
  invoiceId: string;
  invoiceNumber: string;
  customerId: string;
  customerName: string;
  customerEmail: string;
  total: number;
  currency: string;
  dueAt?: string | null;
  paidAt?: string | null;
}

interface PaymentContext {
  paymentId: string;
  invoiceId: string;
  customerId: string;
  customerName: string;
  customerEmail: string;
  amount: number;
  method: string;
}

interface SubscriptionContext {
  subscriptionId: string;
  customerId: string;
  customerName: string;
  customerEmail: string;
  planName: string;
  status: string;
}

export async function notifyInvoiceCreated(ctx: InvoiceContext) {
  await emitBillingEvent({
    eventType: "invoice.created",
    resourceType: "invoice",
    resourceId: ctx.invoiceId,
    customerId: ctx.customerId,
    data: {
      invoiceNumber: ctx.invoiceNumber,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      total: ctx.total,
      currency: ctx.currency,
      subject: `New invoice ${ctx.invoiceNumber}`,
      template: "invoice_created",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `New invoice ${ctx.invoiceNumber}`,
    html: invoiceCreatedHtml(ctx),
  }).catch(() => {});
}

export async function notifyInvoiceIssued(ctx: InvoiceContext) {
  await emitBillingEvent({
    eventType: "invoice.issued",
    resourceType: "invoice",
    resourceId: ctx.invoiceId,
    customerId: ctx.customerId,
    data: {
      invoiceNumber: ctx.invoiceNumber,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      total: ctx.total,
      currency: ctx.currency,
      dueAt: ctx.dueAt,
      subject: `Invoice ${ctx.invoiceNumber} is ready — $${ctx.total.toLocaleString()} due`,
      template: "invoice_issued",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Invoice ${ctx.invoiceNumber} is ready — $${ctx.total.toLocaleString()} due`,
    html: invoiceIssuedHtml(ctx),
  }).catch(() => {});
}

export async function notifyInvoicePaid(ctx: InvoiceContext) {
  await emitBillingEvent({
    eventType: "invoice.paid",
    resourceType: "invoice",
    resourceId: ctx.invoiceId,
    customerId: ctx.customerId,
    data: {
      invoiceNumber: ctx.invoiceNumber,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      total: ctx.total,
      currency: ctx.currency,
      paidAt: ctx.paidAt,
      subject: `Payment received for invoice ${ctx.invoiceNumber}`,
      template: "invoice_paid",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Payment received for invoice ${ctx.invoiceNumber}`,
    html: invoicePaidHtml(ctx),
  }).catch(() => {});
}

export async function notifyInvoiceOverdue(ctx: InvoiceContext) {
  await emitBillingEvent({
    eventType: "invoice.overdue",
    resourceType: "invoice",
    resourceId: ctx.invoiceId,
    customerId: ctx.customerId,
    data: {
      invoiceNumber: ctx.invoiceNumber,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      total: ctx.total,
      currency: ctx.currency,
      dueAt: ctx.dueAt,
      subject: `Invoice ${ctx.invoiceNumber} is overdue — $${ctx.total.toLocaleString()}`,
      template: "invoice_overdue",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Invoice ${ctx.invoiceNumber} is overdue — $${ctx.total.toLocaleString()}`,
    html: invoiceOverdueHtml(ctx),
  }).catch(() => {});
}

export async function notifyPaymentReceived(ctx: PaymentContext) {
  await emitBillingEvent({
    eventType: "payment.received",
    resourceType: "payment",
    resourceId: ctx.paymentId,
    customerId: ctx.customerId,
    data: {
      invoiceId: ctx.invoiceId,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      amount: ctx.amount,
      method: ctx.method,
      subject: `Payment of $${ctx.amount.toLocaleString()} received`,
      template: "payment_received",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Payment of $${ctx.amount.toLocaleString()} received`,
    html: paymentReceivedHtml(ctx),
  }).catch(() => {});
}

export async function notifyPaymentRefunded(ctx: PaymentContext & { reason: string }) {
  await emitBillingEvent({
    eventType: "payment.refunded",
    resourceType: "payment",
    resourceId: ctx.paymentId,
    customerId: ctx.customerId,
    data: {
      invoiceId: ctx.invoiceId,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      amount: ctx.amount,
      reason: ctx.reason,
      subject: `Refund of $${ctx.amount.toLocaleString()} processed`,
      template: "payment_refunded",
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Refund of $${ctx.amount.toLocaleString()} processed`,
    html: paymentRefundedHtml(ctx),
  }).catch(() => {});
}

export async function notifySubscriptionEvent(
  type: "subscription.created" | "subscription.renewed" | "subscription.canceled" | "subscription.paused",
  ctx: SubscriptionContext,
) {
  const labels: Record<string, string> = {
    "subscription.created": "activated",
    "subscription.renewed": "renewed",
    "subscription.canceled": "canceled",
    "subscription.paused": "paused",
  };

  await emitBillingEvent({
    eventType: type,
    resourceType: "subscription",
    resourceId: ctx.subscriptionId,
    customerId: ctx.customerId,
    data: {
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      planName: ctx.planName,
      status: ctx.status,
      subject: `Subscription ${labels[type]}: ${ctx.planName}`,
      template: `subscription_${labels[type]}`,
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `Subscription ${labels[type]}: ${ctx.planName}`,
    html: subscriptionEventHtml({ customerName: ctx.customerName, planName: ctx.planName, action: labels[type] }),
  }).catch(() => {});
}

export async function notifyDunningStep(
  step: "reminder" | "warning" | "final_notice" | "suspension",
  ctx: InvoiceContext & { daysPastDue: number },
) {
  const stepLabels: Record<string, string> = {
    reminder: "Payment Reminder",
    warning: "Payment Warning",
    final_notice: "Final Notice",
    suspension: "Account Suspension Notice",
  };

  await emitBillingEvent({
    eventType: `dunning.${step}` as "dunning.reminder" | "dunning.warning" | "dunning.final_notice" | "dunning.suspension",
    resourceType: "invoice",
    resourceId: ctx.invoiceId,
    customerId: ctx.customerId,
    data: {
      invoiceNumber: ctx.invoiceNumber,
      customerName: ctx.customerName,
      email: ctx.customerEmail,
      total: ctx.total,
      currency: ctx.currency,
      daysPastDue: ctx.daysPastDue,
      subject: `${stepLabels[step]}: Invoice ${ctx.invoiceNumber}`,
      template: `dunning_${step}`,
    },
  });

  sendBillingEmail({
    to: ctx.customerEmail,
    subject: `${stepLabels[step]}: Invoice ${ctx.invoiceNumber}`,
    html: dunningReminderHtml({ ...ctx, step }),
  }).catch(() => {});
}
