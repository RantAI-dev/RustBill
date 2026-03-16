"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Download, FileText, CreditCard, Activity, ChevronDown, ExternalLink, Loader2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useCustomers, useInvoices, useSubscriptions, useBillingEvents, getInvoicePdfUrl, getCheckout } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";

type PortalTab = "invoices" | "subscriptions" | "activity";

const statusColors: Record<string, string> = {
  draft: "bg-muted-foreground/20 text-muted-foreground",
  issued: "bg-blue-500/20 text-blue-400",
  paid: "bg-emerald-500/20 text-emerald-400",
  overdue: "bg-red-500/20 text-red-400",
  void: "bg-zinc-500/20 text-zinc-400",
  active: "bg-emerald-500/20 text-emerald-400",
  paused: "bg-yellow-500/20 text-yellow-400",
  canceled: "bg-red-500/20 text-red-400",
  past_due: "bg-red-500/20 text-red-400",
  trialing: "bg-blue-500/20 text-blue-400",
  pending: "bg-yellow-500/20 text-yellow-400",
  completed: "bg-emerald-500/20 text-emerald-400",
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

const providerInfo: Record<PaymentProvider, { label: string; description: string }> = {
  xendit: { label: "Xendit", description: "Bank transfer, e-wallet, QRIS, VA (Indonesia)" },
  lemonsqueezy: { label: "Lemonsqueezy", description: "International cards, PayPal (Global)" },
  stripe: { label: "Stripe", description: "Credit/debit cards (International)" },
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

  const { data: events, isLoading: loadingEvents } = useBillingEvents(
    selectedCustomerId || undefined,
    50
  );

  const handlePay = async (provider: PaymentProvider) => {
    if (!payInvoice) return;
    setPayingWith(provider);
    try {
      const { checkoutUrl } = await getCheckout(payInvoice.id as string, provider);
      window.open(checkoutUrl, "_blank");
      setPayDialogOpen(false);
      setPayInvoice(null);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Checkout failed");
    } finally {
      setPayingWith(null);
    }
  };

  // Auto-select first customer
  const effectiveCustomerId = selectedCustomerId || (customers[0]?.id as string) || "";
  const selectedCustomer = customers.find((c) => (c.id as string) === effectiveCustomerId);

  // Filter data by customer
  const invoices = ((allInvoices ?? []) as Record<string, unknown>[]).filter(
    (i) => (i.customerId as string) === effectiveCustomerId
  );
  const subs = ((allSubs ?? []) as Record<string, unknown>[]).filter(
    (s) => (s.customerId as string) === effectiveCustomerId
  );

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
        </div>

        {/* Tabs */}
        <div className="flex items-center gap-2">
          {([
            { key: "invoices" as const, label: "Invoices", icon: FileText },
            { key: "subscriptions" as const, label: "Subscriptions", icon: CreditCard },
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
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Issued</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Due</TableHead>
                  <TableHead className="text-xs font-semibold uppercase tracking-wider">Paid</TableHead>
                  <TableHead className="w-10" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {invoices.map((inv) => (
                  <TableRow key={inv.id as string}>
                    <TableCell className="font-mono text-xs">{inv.invoiceNumber as string}</TableCell>
                    <TableCell>
                      <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[(inv.status as string)] ?? "bg-secondary")}>
                        {inv.status as string}
                      </span>
                    </TableCell>
                    <TableCell className="text-right font-medium">${(inv.total as number).toLocaleString()}</TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {inv.issuedAt ? new Date(inv.issuedAt as string).toLocaleDateString() : "—"}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {inv.dueAt ? new Date(inv.dueAt as string).toLocaleDateString() : "—"}
                    </TableCell>
                    <TableCell className="text-muted-foreground text-xs">
                      {inv.paidAt ? new Date(inv.paidAt as string).toLocaleDateString() : "—"}
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
                ))}
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
                        {new Date(sub.currentPeriodStart as string).toLocaleDateString()} — {new Date(sub.currentPeriodEnd as string).toLocaleDateString()}
                      </p>
                    </div>
                    <div>
                      <p className="text-[10px] text-muted-foreground uppercase">Quantity</p>
                      <p className="text-sm text-foreground">{sub.quantity as number}</p>
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
