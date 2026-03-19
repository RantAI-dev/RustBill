"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Download, FileText, CreditCard, Activity, ChevronDown, ExternalLink, Loader2, Wallet, Trash2, Star } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useCustomers, useInvoices, useSubscriptions, useBillingEvents, useCustomerCredits, useSavedPaymentMethods, deletePaymentMethod, setDefaultPaymentMethod, createPaymentMethodSetup, getInvoicePdfUrl, getCheckout } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";

type PortalTab = "invoices" | "subscriptions" | "payment-methods" | "activity";

const statusColors: Record<string, string> = {
  draft: "bg-muted-foreground/20 text-muted-foreground",
  issued: "bg-blue-500/20 text-blue-400",
  paid: "bg-sky-500/20 text-sky-400",
  overdue: "bg-red-500/20 text-red-400",
  void: "bg-zinc-500/20 text-zinc-400",
  active: "bg-sky-500/20 text-sky-400",
  paused: "bg-yellow-500/20 text-yellow-400",
  canceled: "bg-red-500/20 text-red-400",
  past_due: "bg-red-500/20 text-red-400",
  trialing: "bg-blue-500/20 text-blue-400",
  pending: "bg-yellow-500/20 text-yellow-400",
  completed: "bg-emerald-500/20 text-emerald-400",
  expired: "bg-red-500/20 text-red-400",
};

const eventTypeLabels: Record<string, string> = {
  "invoice.created": "Invoice Created",
  "invoice.issued": "Invoice Issued",
  "invoice.paid": "Invoice Paid",
  "invoice.overdue": "Invoice Overdue",
  "invoice.voided": "Invoice Voided",
  "payment.received": "Payment Received",
  "payment.refunded": "Payment Refunded",
  "subscription.created": "Subscription Created",
  "subscription.renewed": "Subscription Renewed",
  "subscription.canceled": "Subscription Canceled",
  "subscription.paused": "Subscription Paused",
  "dunning.reminder": "Payment Reminder",
  "dunning.warning": "Payment Warning",
  "dunning.final_notice": "Final Notice",
  "dunning.suspension": "Account Suspended",
};

type PaymentProvider = "stripe" | "xendit" | "lemonsqueezy";

function getSubPreRenewalInvoiceDays(sub: Record<string, unknown>): number {
  const direct = sub.preRenewalInvoiceDays;
  if (typeof direct === "number" && Number.isFinite(direct)) return Math.trunc(direct);
  const metadata = (sub.metadata as Record<string, unknown> | undefined) ?? {};
  const fromMeta = metadata.preRenewalInvoiceDays ?? metadata.pre_renewal_invoice_days;
  const num = typeof fromMeta === "number" ? fromMeta : Number(fromMeta);
  return Number.isFinite(num) ? Math.trunc(num) : 7;
}

const providerInfo: Record<PaymentProvider, { label: string; description: string }> = {
  xendit: { label: "Xendit", description: "Bank transfer, e-wallet, QRIS, VA (Indonesia)" },
  lemonsqueezy: { label: "Lemonsqueezy", description: "International cards, PayPal (Global)" },
  stripe: { label: "Stripe", description: "Credit/debit cards (International)" },
};

const providerBadgeColors: Record<string, string> = {
  stripe: "bg-purple-500/20 text-purple-400",
  xendit: "bg-blue-500/20 text-blue-400",
  lemonsqueezy: "bg-yellow-500/20 text-yellow-400",
  paypal: "bg-blue-500/20 text-blue-400",
};

export function BillingPortalSection() {
  const { data: customerList, isLoading: loadingCustomers } = useCustomers();
  const { data: allInvoices, isLoading: loadingInvoices } = useInvoices();
  const { data: allSubs, isLoading: loadingSubs } = useSubscriptions();

  const customers = (customerList ?? []) as Record<string, unknown>[];
  const [selectedCustomerId, setSelectedCustomerId] = useState<string>("");
  const [tab, setTab] = useState<PortalTab>("invoices");
  const [payDialogOpen, setPayDialogOpen] = useState(false);
  const [payInvoice, setPayInvoice] = useState<Record<string, unknown> | null>(null);
  const [payingWith, setPayingWith] = useState<PaymentProvider | null>(null);
  const [creatingSetupFor, setCreatingSetupFor] = useState<PaymentProvider | null>(null);

  // Auto-select first customer
  const effectiveCustomerId = selectedCustomerId || (customers[0]?.id as string) || "";
  const selectedCustomer = customers.find((c) => (c.id as string) === effectiveCustomerId);

  const { data: events, isLoading: loadingEvents } = useBillingEvents(
    effectiveCustomerId || undefined,
    50
  );

  const { data: credits } = useCustomerCredits(effectiveCustomerId || undefined);
  const { data: paymentMethods, isLoading: loadingPaymentMethods, mutate: mutatePaymentMethods } = useSavedPaymentMethods(effectiveCustomerId || undefined);

  const creditBalance = (credits as Record<string, unknown>)?.balance as number ?? 0;

  const handlePay = async (provider: PaymentProvider) => {
    if (!payInvoice) return;
    setPayingWith(provider);
    const result = await getCheckout(payInvoice.id as string, provider);
    setPayingWith(null);
    if (!result.success) {
      toast.error(result.error ?? "Checkout failed");
      return;
    }
    window.open(result.data.checkoutUrl, "_blank");
    setPayDialogOpen(false);
    setPayInvoice(null);
  };

  const handleDeletePaymentMethod = async (id: string) => {
    const result = await deletePaymentMethod(id);
    if (result.success) {
      toast.success("Payment method removed");
      mutatePaymentMethods();
    }
  };

  const handleSetDefault = async (id: string) => {
    const result = await setDefaultPaymentMethod(id);
    if (result.success) {
      toast.success("Default payment method updated");
      mutatePaymentMethods();
    }
  };

  const handleAddPaymentMethod = async (provider: PaymentProvider) => {
    if (!effectiveCustomerId) {
      toast.error("Select a customer first");
      return;
    }
    setCreatingSetupFor(provider);
    const result = await createPaymentMethodSetup({
      customerId: effectiveCustomerId,
      provider,
      successUrl: `${window.location.origin}/dashboard/billing/payment-methods/success`,
      cancelUrl: `${window.location.origin}/dashboard/billing/payment-methods/cancel`,
    });
    setCreatingSetupFor(null);

    if (!result.success) {
      toast.error(result.error ?? "Failed to start setup");
      return;
    }

    const data = result.data as Record<string, unknown>;
    const setupUrl = data.setupUrl as string | undefined;
    if (setupUrl) {
      window.open(setupUrl, "_blank", "noopener,noreferrer");
      return;
    }

    const actions = data.actions as Array<{ url?: string }> | undefined;
    const actionUrl = actions?.find((a) => typeof a?.url === "string")?.url;
    if (actionUrl) {
      window.open(actionUrl, "_blank", "noopener,noreferrer");
      return;
    }

    toast.success("Setup session created");
  };

  // Filter data by customer
  const invoices = ((allInvoices ?? []) as Record<string, unknown>[]).filter(
    (i) => ((i.customerId as string) ?? (i.customer_id as string)) === effectiveCustomerId
  );
  const subs = ((allSubs ?? []) as Record<string, unknown>[]).filter(
    (s) => ((s.customerId as string) ?? (s.customer_id as string)) === effectiveCustomerId
  );
  const methods = (paymentMethods ?? []) as Record<string, unknown>[];

  const selectClass = "h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground appearance-none pr-8 focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent";

  if (loadingCustomers) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-64 rounded-lg" />
        <Skeleton className="h-[400px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          {customers.length === 0 ? (
            <span className="text-sm text-muted-foreground">No customers yet</span>
          ) : (
            <>
              <div className="relative">
                <select
                  className={selectClass}
                  value={effectiveCustomerId}
                  onChange={(e) => setSelectedCustomerId(e.target.value)}
                >
                  {customers.map((c) => (
                    <option key={c.id as string} value={c.id as string}>
                      {c.name as string}
                    </option>
                  ))}
                </select>
                <ChevronDown className="absolute right-2.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
              </div>
              {selectedCustomer && (
                <span className="text-sm text-muted-foreground">
                  {selectedCustomer.email as string}
                </span>
              )}
            </>
          )}
          {/* Credit Balance */}
          <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-secondary border border-border">
            <Wallet className="w-3.5 h-3.5 text-accent" />
            <span className="text-xs text-muted-foreground">Credits:</span>
            <span className="text-sm font-semibold text-foreground">${creditBalance.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</span>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex items-center gap-2">
          {([
            { key: "invoices" as const, label: "Invoices", icon: FileText },
            { key: "subscriptions" as const, label: "Subscriptions", icon: CreditCard },
            { key: "payment-methods" as const, label: "Payment Methods", icon: Wallet },
            { key: "activity" as const, label: "Activity", icon: Activity },
          ]).map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={cn(
                "flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all",
                tab === t.key
                  ? "bg-accent text-accent-foreground"
                  : "bg-secondary text-muted-foreground hover:text-foreground"
              )}
            >
              <t.icon className="w-3.5 h-3.5" />
              {t.label}
            </button>
          ))}
        </div>
      </div>

      {/* Content */}
      {tab === "invoices" && (
        <div className="bg-card border border-border rounded-xl overflow-hidden">
          {loadingInvoices ? (
            <div className="p-6 space-y-3">
              {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-12 w-full rounded-lg" />)}
            </div>
          ) : invoices.length === 0 ? (
            <div className="p-12 text-center text-muted-foreground">No invoices found</div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow className="bg-secondary/50">
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Invoice #</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Status</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider text-right">Total</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider text-right">Tax</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider text-right">Credits Applied</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider text-right">Amount Due</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Issued</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Due</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Paid</TableHead>
                  <TableHead className="w-10" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {invoices.map((inv) => {
                  const total = Number((inv.total as number | string) ?? 0);
                  const tax = Number((inv.tax as number | string) ?? (inv.taxAmount as number | string) ?? 0);
                  const creditsApplied = Number(
                    (inv.creditsApplied as number | string) ??
                    (inv.credits_applied as number | string) ??
                    0,
                  );
                  const amountDue = Number(
                    (inv.amountDue as number | string) ??
                    (inv.amount_due as number | string) ??
                    total,
                  );
                  const invoiceNumber =
                    (inv.invoiceNumber as string) ?? (inv.invoice_number as string) ?? "-";
                  const issuedAt = (inv.issuedAt as string) ?? (inv.issued_at as string);
                  const dueAt = (inv.dueAt as string) ?? (inv.due_at as string);
                  const paidAt = (inv.paidAt as string) ?? (inv.paid_at as string);
                  return (
                    <TableRow key={inv.id as string}>
                      <TableCell className="font-mono text-xs">{invoiceNumber}</TableCell>
                      <TableCell>
                        <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[(inv.status as string)] ?? "bg-secondary")}>
                          {inv.status as string}
                        </span>
                      </TableCell>
                      <TableCell className="text-right font-medium">${total.toLocaleString()}</TableCell>
                      <TableCell className="text-right text-muted-foreground">{tax > 0 ? `$${tax.toLocaleString()}` : "—"}</TableCell>
                      <TableCell className="text-right text-muted-foreground">{creditsApplied > 0 ? `$${creditsApplied.toLocaleString()}` : "—"}</TableCell>
                      <TableCell className="text-right font-medium">${amountDue.toLocaleString()}</TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {issuedAt ? new Date(issuedAt).toLocaleDateString() : "—"}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {dueAt ? new Date(dueAt).toLocaleDateString() : "—"}
                      </TableCell>
                      <TableCell className="text-muted-foreground text-xs">
                        {paidAt ? new Date(paidAt).toLocaleDateString() : "—"}
                      </TableCell>
                      <TableCell>
                        <div className="flex items-center gap-1 justify-end">
                          {(inv.status === "issued" || inv.status === "overdue") && (
                            <Button
                              variant="default"
                              size="sm"
                              className="h-7 text-xs bg-accent hover:bg-accent/90 text-accent-foreground"
                              onClick={() => { setPayInvoice(inv); setPayDialogOpen(true); }}
                            >
                              <ExternalLink className="w-3 h-3 mr-1" />
                              Pay
                            </Button>
                          )}
                          {inv.status !== "draft" && (
                            <a href={getInvoicePdfUrl(inv.id as string)} target="_blank" rel="noopener noreferrer">
                              <Button variant="ghost" size="icon" className="h-8 w-8">
                                <Download className="w-4 h-4" />
                              </Button>
                            </a>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </div>
      )}

      {tab === "subscriptions" && (
        <div>
          {loadingSubs ? (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {[...Array(3)].map((_, i) => <Skeleton key={i} className="h-32 rounded-xl" />)}
            </div>
          ) : subs.length === 0 ? (
            <div className="bg-card border border-border rounded-xl p-12 text-center text-muted-foreground">No subscriptions found</div>
          ) : (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              {subs.map((sub) => (
                <div key={sub.id as string} className="bg-card border border-border rounded-xl p-5 space-y-3">
                  <div className="flex items-center justify-between">
                    <h3 className="font-semibold text-foreground">{(sub.planName as string) ?? "Plan"}</h3>
                    <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[(sub.status as string)] ?? "bg-secondary")}>
                      {(sub.status as string).replace("_", " ")}
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <p className="text-[10px] text-muted-foreground uppercase">Period</p>
                      <p className="text-sm text-foreground">
                        {new Date(((sub.currentPeriodStart as string) ?? (sub.current_period_start as string))).toLocaleDateString()} — {new Date(((sub.currentPeriodEnd as string) ?? (sub.current_period_end as string))).toLocaleDateString()}
                      </p>
                    </div>
                    <div>
                      <p className="text-[10px] text-muted-foreground uppercase">Quantity</p>
                      <p className="text-sm text-foreground">{sub.quantity as number}</p>
                    </div>
                    <div>
                      <p className="text-[10px] text-muted-foreground uppercase">Invoice Lead</p>
                      <p className="text-sm text-foreground">
                        {(() => {
                          const days = getSubPreRenewalInvoiceDays(sub);
                          return days === 0 ? "Disabled" : `${days} day${days === 1 ? "" : "s"}`;
                        })()}
                      </p>
                    </div>
                  </div>
                  {!!sub.cancelAtPeriodEnd && (
                    <p className="text-xs text-yellow-400">Cancels at period end</p>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {tab === "payment-methods" && (
        <div className="bg-card border border-border rounded-xl overflow-hidden">
          <div className="px-5 py-3 border-b border-border bg-secondary/20 flex items-center justify-between">
            <p className="text-xs text-muted-foreground">Add a tokenized payment method via provider setup flow.</p>
            <div className="flex items-center gap-2">
              {(["stripe", "xendit"] as const).map((provider) => (
                <Button
                  key={provider}
                  size="sm"
                  variant="outline"
                  className="h-8 text-xs"
                  onClick={() => handleAddPaymentMethod(provider)}
                  disabled={creatingSetupFor !== null}
                >
                  {creatingSetupFor === provider ? <Loader2 className="w-3.5 h-3.5 mr-1 animate-spin" /> : null}
                  Add via {providerInfo[provider].label}
                </Button>
              ))}
            </div>
          </div>
          {loadingPaymentMethods ? (
            <div className="p-6 space-y-3">
              {[...Array(3)].map((_, i) => <Skeleton key={i} className="h-16 w-full rounded-lg" />)}
            </div>
          ) : methods.length === 0 ? (
            <div className="p-12 text-center text-muted-foreground">No saved payment methods</div>
          ) : (
            <div className="divide-y divide-border">
              {methods.map((pm) => {
                const isDefault = pm.isDefault as boolean;
                const provider = (pm.provider as string) ?? "";
                const status = (pm.status as string) ?? "active";
                return (
                  <div key={pm.id as string} className="flex items-center justify-between px-5 py-4">
                    <div className="flex items-center gap-3">
                      <CreditCard className="w-5 h-5 text-muted-foreground" />
                      <div>
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-foreground">
                            {(pm.label as string) ?? "Card"}
                          </span>
                          {pm.lastFour ? (
                            <span className="text-xs text-muted-foreground font-mono">
                              **** {pm.lastFour as string}
                            </span>
                          ) : null}
                          {isDefault && (
                            <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-accent/20 text-accent">
                              DEFAULT
                            </span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 mt-0.5">
                          {provider && (
                            <span className={cn("px-2 py-0.5 rounded-full text-[10px] font-medium capitalize", providerBadgeColors[provider] ?? "bg-secondary text-muted-foreground")}>
                              {provider}
                            </span>
                          )}
                          <span className={cn("px-2 py-0.5 rounded-full text-[10px] font-medium capitalize", statusColors[status] ?? "bg-secondary")}>
                            {status}
                          </span>
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1">
                      {!isDefault && status === "active" && (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-8 text-xs"
                          onClick={() => handleSetDefault(pm.id as string)}
                        >
                          <Star className="w-3.5 h-3.5 mr-1" />
                          Set Default
                        </Button>
                      )}
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-8 w-8 text-destructive hover:text-destructive"
                        onClick={() => handleDeletePaymentMethod(pm.id as string)}
                      >
                        <Trash2 className="w-4 h-4" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {tab === "activity" && (
        <div className="bg-card border border-border rounded-xl overflow-hidden">
          {loadingEvents ? (
            <div className="p-6 space-y-3">
              {[...Array(5)].map((_, i) => <Skeleton key={i} className="h-10 w-full rounded-lg" />)}
            </div>
          ) : !events || (events as Record<string, unknown>[]).length === 0 ? (
            <div className="p-12 text-center text-muted-foreground">No billing activity yet</div>
          ) : (
            <div className="divide-y divide-border">
              {(events as Record<string, unknown>[]).map((evt) => {
                const data = (evt.data ?? {}) as Record<string, unknown>;
                return (
                  <div key={evt.id as string} className="flex items-center justify-between px-5 py-3.5">
                    <div className="flex items-center gap-3">
                      <div className="w-2 h-2 rounded-full bg-accent shrink-0" />
                      <div>
                        <p className="text-sm font-medium text-foreground">
                          {eventTypeLabels[evt.eventType as string] ?? (evt.eventType as string)}
                        </p>
                        {!!data.subject && (
                          <p className="text-xs text-muted-foreground">{data.subject as string}</p>
                        )}
                      </div>
                    </div>
                    <span className="text-xs text-muted-foreground shrink-0">
                      {new Date(evt.createdAt as string).toLocaleString()}
                    </span>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* Pay Invoice — Provider Selection Dialog */}
      <Dialog open={payDialogOpen} onOpenChange={(open) => { if (!open) { setPayDialogOpen(false); setPayInvoice(null); } }}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Pay Invoice</DialogTitle>
            <DialogDescription>
              {payInvoice && (
                <>Invoice {payInvoice.invoiceNumber as string} — ${(payInvoice.total as number).toLocaleString()}</>
              )}
            </DialogDescription>
          </DialogHeader>
          <div className="mt-2 space-y-2">
            {(Object.entries(providerInfo) as [PaymentProvider, { label: string; description: string }][]).map(
              ([key, info]) => (
                <button
                  key={key}
                  onClick={() => handlePay(key)}
                  disabled={!!payingWith}
                  className={cn(
                    "w-full flex items-center justify-between p-4 rounded-lg border border-border bg-secondary/30 hover:bg-secondary/60 transition-colors text-left",
                    payingWith === key && "ring-2 ring-accent"
                  )}
                >
                  <div>
                    <p className="text-sm font-medium text-foreground">{info.label}</p>
                    <p className="text-xs text-muted-foreground mt-0.5">{info.description}</p>
                  </div>
                  {payingWith === key ? (
                    <Loader2 className="w-4 h-4 animate-spin text-accent shrink-0" />
                  ) : (
                    <ExternalLink className="w-4 h-4 text-muted-foreground shrink-0" />
                  )}
                </button>
              )
            )}
          </div>
          <DialogFooter className="mt-4">
            <Button variant="outline" onClick={() => { setPayDialogOpen(false); setPayInvoice(null); }}>
              Cancel
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
