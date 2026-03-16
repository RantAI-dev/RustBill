import { appConfig } from "@/lib/app-config";

function wrap(content: string): string {
  return `<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><style>
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 0; background: #f4f4f5; }
  .container { max-width: 600px; margin: 0 auto; padding: 40px 20px; }
  .card { background: #fff; border-radius: 8px; padding: 32px; border: 1px solid #e4e4e7; }
  .header { font-size: 20px; font-weight: 600; color: #18181b; margin-bottom: 16px; }
  .text { font-size: 14px; color: #3f3f46; line-height: 1.6; margin-bottom: 12px; }
  .amount { font-size: 28px; font-weight: 700; color: #18181b; margin: 16px 0; }
  .detail { font-size: 13px; color: #71717a; margin: 4px 0; }
  .footer { margin-top: 24px; padding-top: 16px; border-top: 1px solid #e4e4e7; font-size: 12px; color: #a1a1aa; }
</style></head>
<body><div class="container"><div class="card">${content}<div class="footer">${appConfig.name} — Billing</div></div></div></body>
</html>`;
}

function fmt(amount: number, currency = "USD"): string {
  return new Intl.NumberFormat("en-US", { style: "currency", currency }).format(amount);
}

export function invoiceCreatedHtml(ctx: { invoiceNumber: string; customerName: string; total: number; currency: string }) {
  return wrap(`
    <div class="header">Invoice Created</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">A new invoice has been created for your account.</p>
    <div class="amount">${fmt(ctx.total, ctx.currency)}</div>
    <p class="detail">Invoice: ${ctx.invoiceNumber}</p>
  `);
}

export function invoiceIssuedHtml(ctx: { invoiceNumber: string; customerName: string; total: number; currency: string; dueAt?: string | null }) {
  return wrap(`
    <div class="header">Invoice Ready</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">Your invoice is ready for payment.</p>
    <div class="amount">${fmt(ctx.total, ctx.currency)}</div>
    <p class="detail">Invoice: ${ctx.invoiceNumber}</p>
    ${ctx.dueAt ? `<p class="detail">Due: ${ctx.dueAt}</p>` : ""}
  `);
}

export function invoicePaidHtml(ctx: { invoiceNumber: string; customerName: string; total: number; currency: string }) {
  return wrap(`
    <div class="header">Payment Confirmed</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">We've received your payment. Thank you!</p>
    <div class="amount">${fmt(ctx.total, ctx.currency)}</div>
    <p class="detail">Invoice: ${ctx.invoiceNumber}</p>
  `);
}

export function invoiceOverdueHtml(ctx: { invoiceNumber: string; customerName: string; total: number; currency: string; dueAt?: string | null }) {
  return wrap(`
    <div class="header">Invoice Overdue</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">Your invoice is now past due. Please arrange payment at your earliest convenience.</p>
    <div class="amount">${fmt(ctx.total, ctx.currency)}</div>
    <p class="detail">Invoice: ${ctx.invoiceNumber}</p>
    ${ctx.dueAt ? `<p class="detail">Was due: ${ctx.dueAt}</p>` : ""}
  `);
}

export function paymentReceivedHtml(ctx: { customerName: string; amount: number; method: string }) {
  return wrap(`
    <div class="header">Payment Received</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">We've received your payment.</p>
    <div class="amount">${fmt(ctx.amount)}</div>
    <p class="detail">Method: ${ctx.method}</p>
  `);
}

export function paymentRefundedHtml(ctx: { customerName: string; amount: number; reason: string }) {
  return wrap(`
    <div class="header">Refund Processed</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">A refund has been processed for your account.</p>
    <div class="amount">${fmt(ctx.amount)}</div>
    <p class="detail">Reason: ${ctx.reason}</p>
  `);
}

export function dunningReminderHtml(ctx: { invoiceNumber: string; customerName: string; total: number; currency: string; daysPastDue: number; step: string }) {
  const titles: Record<string, string> = {
    reminder: "Payment Reminder",
    warning: "Payment Warning",
    final_notice: "Final Notice",
    suspension: "Account Suspension",
  };

  return wrap(`
    <div class="header">${titles[ctx.step] ?? "Payment Notice"}</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">Invoice ${ctx.invoiceNumber} is ${ctx.daysPastDue} days past due. Please arrange payment to avoid service interruption.</p>
    <div class="amount">${fmt(ctx.total, ctx.currency)}</div>
  `);
}

export function subscriptionEventHtml(ctx: { customerName: string; planName: string; action: string }) {
  return wrap(`
    <div class="header">Subscription ${ctx.action}</div>
    <p class="text">Hi ${ctx.customerName},</p>
    <p class="text">Your subscription to <strong>${ctx.planName}</strong> has been ${ctx.action.toLowerCase()}.</p>
  `);
}
