"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2, DollarSign, Eye, FileText, RotateCcw, Download, ExternalLink, Loader2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import {
  useInvoices, useCustomers, useSubscriptions,
  createInvoice, updateInvoice, deleteInvoice,
  createPayment, createCreditNote, createRefund,
  getInvoicePdfUrl, getCheckout,
} from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Inv = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

function normalizeInvoicePayload(invoice: Inv): Inv {
  const items = Array.isArray(invoice.items)
    ? (invoice.items as Inv[]).map((item) => ({
        ...item,
        unitPrice: (item.unit_price as number) ?? item.unitPrice,
        periodStart: (item.period_start as string) ?? item.periodStart,
        periodEnd: (item.period_end as string) ?? item.periodEnd,
      }))
    : [];

  const payments = Array.isArray(invoice.payments)
    ? (invoice.payments as Inv[]).map((payment) => ({
        ...payment,
        paidAt: (payment.paid_at as string) ?? payment.paidAt,
      }))
    : [];

  return {
    ...invoice,
    invoiceNumber: (invoice.invoice_number as string) ?? invoice.invoiceNumber,
    customerName: (invoice.customer_name as string) ?? invoice.customerName,
    customerId: (invoice.customer_id as string) ?? invoice.customerId,
    subscriptionId: (invoice.subscription_id as string) ?? invoice.subscriptionId,
    issuedAt: (invoice.issued_at as string) ?? invoice.issuedAt,
    dueAt: (invoice.due_at as string) ?? invoice.dueAt,
    paidAt: (invoice.paid_at as string) ?? invoice.paidAt,
    items,
    payments,
  };
}

const statusColors: Record<string, string> = {
  draft: "bg-muted-foreground/20 text-muted-foreground",
  issued: "bg-blue-500/20 text-blue-400",
  paid: "bg-sky-500/20 text-sky-400",
  overdue: "bg-red-500/20 text-red-400",
  void: "bg-zinc-500/20 text-zinc-400",
};

/* ---------- Invoice detail view ---------- */
function InvoiceDetail({ invoice, onEdit, onRecordPayment, onCreditNote, onRefund, onCheckout, onDelete }: {
  invoice: Inv;
  onEdit: () => void;
  onRecordPayment: () => void;
  onCreditNote: () => void;
  onRefund: () => void;
  onCheckout: () => void;
  onDelete: () => void;
}) {
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const items = (invoice.items as Array<Record<string, unknown>>) || [];
  const payments = (invoice.payments as Array<Record<string, unknown>>) || [];

  return (
    <div>
      <DialogHeader>
        <div className="flex items-center gap-3">
          <DialogTitle className="text-lg font-mono">{invoice.invoiceNumber as string}</DialogTitle>
          <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[(invoice.status as string)] ?? "bg-secondary")}>
            {invoice.status as string}
          </span>
        </div>
        <DialogDescription>{(invoice.customerName as string) ?? "Unknown customer"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-5">
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className={labelClass}>Subtotal</p>
            <p className="text-sm font-medium text-foreground mt-0.5">${(invoice.subtotal as number).toLocaleString()}</p>
          </div>
          <div>
            <p className={labelClass}>Tax</p>
            <p className="text-sm font-medium text-foreground mt-0.5">${(invoice.tax as number).toLocaleString()}</p>
          </div>
          <div>
            <p className={labelClass}>Total</p>
            <p className="text-lg font-bold text-foreground">${(invoice.total as number).toLocaleString()}</p>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className={labelClass}>Issued</p>
            <p className="text-sm font-medium text-foreground mt-0.5">
              {invoice.issuedAt ? new Date(invoice.issuedAt as string).toLocaleDateString() : "—"}
            </p>
          </div>
          <div>
            <p className={labelClass}>Due</p>
            <p className="text-sm font-medium text-foreground mt-0.5">
              {invoice.dueAt ? new Date(invoice.dueAt as string).toLocaleDateString() : "—"}
            </p>
          </div>
          <div>
            <p className={labelClass}>Paid</p>
            <p className="text-sm font-medium text-foreground mt-0.5">
              {invoice.paidAt ? new Date(invoice.paidAt as string).toLocaleDateString() : "—"}
            </p>
          </div>
        </div>

        {/* Line items */}
        {items.length > 0 && (
          <div>
            <p className={cn(labelClass, "mb-2")}>Line Items</p>
            <div className="border border-border rounded-lg overflow-hidden">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-secondary/50">
                    <th className="px-3 py-2 text-left text-xs font-medium text-muted-foreground">Description</th>
                    <th className="px-3 py-2 text-right text-xs font-medium text-muted-foreground">Qty</th>
                    <th className="px-3 py-2 text-right text-xs font-medium text-muted-foreground">Unit Price</th>
                    <th className="px-3 py-2 text-right text-xs font-medium text-muted-foreground">Amount</th>
                  </tr>
                </thead>
                <tbody>
                  {items.map((item, idx) => (
                    <tr key={idx} className="border-b border-border/50">
                      <td className="px-3 py-2 text-foreground">{item.description as string}</td>
                      <td className="px-3 py-2 text-right text-muted-foreground">{item.quantity as number}</td>
                      <td className="px-3 py-2 text-right text-muted-foreground">${(item.unitPrice as number).toLocaleString()}</td>
                      <td className="px-3 py-2 text-right font-medium text-foreground">${(item.amount as number).toLocaleString()}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Payments */}
        {payments.length > 0 && (
          <div>
            <p className={cn(labelClass, "mb-2")}>Payments</p>
            <div className="space-y-2">
              {payments.map((p, idx) => (
                <div key={idx} className="flex items-center justify-between px-3 py-2 bg-secondary/50 rounded-lg">
                  <div>
                    <span className="text-sm font-medium text-foreground">${(p.amount as number).toLocaleString()}</span>
                    <span className="text-xs text-muted-foreground ml-2 capitalize">{(p.method as string).replace("_", " ")}</span>
                    {!!p.reference && <span className="text-xs text-muted-foreground ml-2">Ref: {p.reference as string}</span>}
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-muted-foreground">{new Date(p.paidAt as string).toLocaleDateString()}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <DialogFooter className="mt-6 w-full sm:justify-between">
        <Button
          variant="outline"
          type="button"
          className="text-destructive hover:text-destructive hover:bg-destructive/10"
          onClick={onDelete}
        >
          <Trash2 className="w-4 h-4 mr-1" /> Delete
        </Button>
        <div className="flex flex-wrap items-center gap-2 justify-end">
          <a href={getInvoicePdfUrl(invoice.id as string)} target="_blank" rel="noopener noreferrer">
            <Button variant="outline" type="button">
              <Download className="w-4 h-4 mr-1" /> PDF
            </Button>
          </a>
          <Button variant="outline" onClick={onEdit}>
            <Pencil className="w-4 h-4 mr-1" /> Edit
          </Button>
          {(invoice.status === "paid" || invoice.status === "issued" || invoice.status === "overdue") && (
            <Button variant="outline" onClick={onCreditNote}>
              <FileText className="w-4 h-4 mr-1" /> Credit Note
            </Button>
          )}
          {invoice.status === "paid" && payments.length > 0 && (
            <Button variant="outline" onClick={onRefund}>
              <RotateCcw className="w-4 h-4 mr-1" /> Refund
            </Button>
          )}
          {(invoice.status === "issued" || invoice.status === "overdue") && (
            <>
              <Button variant="outline" onClick={onCheckout}>
                <ExternalLink className="w-4 h-4 mr-1" /> Checkout Link
              </Button>
              <Button onClick={onRecordPayment}>
                <DollarSign className="w-4 h-4 mr-1" /> Record Payment
              </Button>
            </>
          )}
        </div>
      </DialogFooter>
    </div>
  );
}

/* ---------- Credit Note Form ---------- */
function CreditNoteForm({ invoice, onSubmit, onCancel, loading }: {
  invoice: Inv;
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors";

  const invoiceItems = (invoice.items as Array<Record<string, unknown>>) || [];
  const [reason, setReason] = useState("");
  const [items, setItems] = useState(
    invoiceItems.length > 0
      ? invoiceItems.map((i) => ({
          description: i.description as string,
          quantity: i.quantity as number,
          unitPrice: i.unitPrice as number,
          selected: false,
        }))
      : [{ description: "", quantity: 1, unitPrice: 0, selected: true }]
  );

  const selectedItems = items.filter((i) => i.selected);
  const totalCredit = selectedItems.reduce((s, i) => s + i.quantity * i.unitPrice, 0);

  return (
    <div>
      <DialogHeader>
        <DialogTitle>Issue Credit Note</DialogTitle>
        <DialogDescription>
          Invoice {invoice.invoiceNumber as string} — Total ${(invoice.total as number).toLocaleString()}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4 max-h-[60vh] overflow-y-auto pr-1">
        <div>
          <label className={labelClass}>Reason</label>
          <input className={inputClass} value={reason} onChange={(e) => setReason(e.target.value)} placeholder="Billing error, partial refund, etc." />
        </div>

        <div>
          <label className={cn(labelClass, "mb-2 block")}>Select Items to Credit</label>
          <div className="space-y-2">
            {items.map((item, idx) => (
              <div key={idx} className="flex items-center gap-3 px-3 py-2 bg-secondary/50 rounded-lg">
                <input
                  type="checkbox"
                  checked={item.selected}
                  onChange={(e) => {
                    const updated = [...items];
                    updated[idx] = { ...updated[idx], selected: e.target.checked };
                    setItems(updated);
                  }}
                  className="h-4 w-4 rounded border-border"
                />
                <span className="flex-1 text-sm text-foreground">{item.description}</span>
                <span className="text-sm text-muted-foreground">{item.quantity} x ${item.unitPrice.toLocaleString()}</span>
                <span className="text-sm font-medium text-foreground">${(item.quantity * item.unitPrice).toLocaleString()}</span>
              </div>
            ))}
          </div>
        </div>

        <div className="flex items-center justify-between px-3 py-2 bg-accent/10 rounded-lg">
          <span className="text-sm font-medium text-foreground">Credit Total</span>
          <span className="text-lg font-bold text-foreground">${totalCredit.toLocaleString()}</span>
        </div>
      </div>

      <DialogFooter className="mt-6">
        <Button variant="outline" onClick={onCancel}>Cancel</Button>
        <Button
          disabled={loading || selectedItems.length === 0 || !reason}
          onClick={() => onSubmit({
            invoiceId: invoice.id,
            customerId: invoice.customerId,
            reason,
            items: selectedItems.map((i) => ({
              description: `Credit: ${i.description}`,
              quantity: i.quantity,
              unitPrice: i.unitPrice,
            })),
          })}
        >
          {loading ? "Issuing..." : "Issue Credit Note"}
        </Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- Refund Form ---------- */
function RefundForm({ invoice, onSubmit, onCancel, loading }: {
  invoice: Inv;
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors";

  const payments = (invoice.payments as Array<Record<string, unknown>>) || [];
  const [selectedPaymentId, setSelectedPaymentId] = useState(payments[0]?.id as string ?? "");
  const selectedPayment = payments.find((p) => (p.id as string) === selectedPaymentId);
  const [amount, setAmount] = useState((selectedPayment?.amount as number) ?? 0);
  const [reason, setReason] = useState("");

  return (
    <div>
      <DialogHeader>
        <DialogTitle>Issue Refund</DialogTitle>
        <DialogDescription>
          Invoice {invoice.invoiceNumber as string} — Total ${(invoice.total as number).toLocaleString()}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div>
          <label className={labelClass}>Payment to Refund</label>
          <select
            className={inputClass}
            value={selectedPaymentId}
            onChange={(e) => {
              setSelectedPaymentId(e.target.value);
              const p = payments.find((p) => (p.id as string) === e.target.value);
              if (p) setAmount(p.amount as number);
            }}
          >
            {payments.map((p) => (
              <option key={p.id as string} value={p.id as string}>
                ${(p.amount as number).toLocaleString()} — {(p.method as string).replace("_", " ")} ({new Date(p.paidAt as string).toLocaleDateString()})
              </option>
            ))}
          </select>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Refund Amount ($)</label>
            <input
              type="number"
              step="0.01"
              className={inputClass}
              value={amount}
              onChange={(e) => setAmount(Number(e.target.value))}
            />
            {selectedPayment && amount > (selectedPayment.amount as number) && (
              <p className="text-xs text-destructive mt-1">Cannot exceed payment amount (${(selectedPayment.amount as number).toLocaleString()})</p>
            )}
          </div>
          <div>
            <label className={labelClass}>Status</label>
            <select className={inputClass} disabled>
              <option value="completed">Completed</option>
            </select>
          </div>
        </div>

        <div>
          <label className={labelClass}>Reason</label>
          <input
            className={inputClass}
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Customer request, billing error, etc."
          />
        </div>
      </div>

      <DialogFooter className="mt-6">
        <Button variant="outline" onClick={onCancel}>Cancel</Button>
        <Button
          disabled={loading || !reason || !selectedPaymentId || amount <= 0 || (selectedPayment && amount > (selectedPayment.amount as number))}
          onClick={() => onSubmit({
            paymentId: selectedPaymentId,
            invoiceId: invoice.id,
            amount,
            reason,
            status: "completed",
          })}
        >
          {loading ? "Processing..." : "Issue Refund"}
        </Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- Create/Edit form ---------- */
function InvoiceForm({ invoice, mode, customers, subscriptions, onSubmit, onCancel, loading }: {
  invoice?: Inv;
  mode: DialogMode;
  customers: Inv[];
  subscriptions: Inv[];
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const isCreate = mode === "create";
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors disabled:opacity-50";

  const today = new Date().toISOString().split("T")[0];
  const thirtyDays = new Date(Date.now() + 30 * 86400000).toISOString().split("T")[0];

  const [form, setForm] = useState({
    customerId: (invoice?.customerId as string) ?? "",
    subscriptionId: (invoice?.subscriptionId as string) ?? "",
    status: (invoice?.status as string) ?? "draft",
    issuedAt: invoice?.issuedAt ? new Date(invoice.issuedAt as string).toISOString().split("T")[0] : today,
    dueAt: invoice?.dueAt ? new Date(invoice.dueAt as string).toISOString().split("T")[0] : thirtyDays,
    tax: (invoice?.tax as number) ?? 0,
    notes: (invoice?.notes as string) ?? "",
    // Manual line items for create
    items: [{ description: "", quantity: 1, unitPrice: 0 }],
  });

  const addItem = () => setForm({ ...form, items: [...form.items, { description: "", quantity: 1, unitPrice: 0 }] });
  const removeItem = (idx: number) => setForm({ ...form, items: form.items.filter((_, i) => i !== idx) });
  const updateItem = (idx: number, field: string, value: string | number) => {
    const items = [...form.items];
    items[idx] = { ...items[idx], [field]: value };
    setForm({ ...form, items });
  };

  const filteredSubs = form.customerId
    ? subscriptions.filter((s) => (s.customerId as string) === form.customerId)
    : subscriptions;

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Create Invoice" : "Edit Invoice"}</DialogTitle>
        <DialogDescription>{isCreate ? "Create a new invoice" : "Update invoice details"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4 max-h-[60vh] overflow-y-auto pr-1">
        {isCreate && (
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className={labelClass}>Customer</label>
              <select
                data-testid="invoice-form-customer"
                className={inputClass}
                value={form.customerId}
                onChange={(e) => setForm({ ...form, customerId: e.target.value, subscriptionId: "" })}
              >
                <option value="">Select customer</option>
                {customers.map((c) => <option key={c.id as string} value={c.id as string}>{c.name as string}</option>)}
              </select>
            </div>
            <div>
              <label className={labelClass}>Subscription (optional)</label>
              <select
                data-testid="invoice-form-subscription"
                className={inputClass}
                value={form.subscriptionId}
                onChange={(e) => setForm({ ...form, subscriptionId: e.target.value })}
              >
                <option value="">None (manual items)</option>
                {filteredSubs.map((s) => <option key={s.id as string} value={s.id as string}>{(s.planName as string) ?? "Plan"} — {(s.customerName as string) ?? ""}</option>)}
              </select>
            </div>
          </div>
        )}

        <div className="grid grid-cols-3 gap-4">
          <div>
            <label className={labelClass}>Status</label>
            <select
              data-testid="invoice-form-status"
              className={inputClass}
              value={form.status}
              onChange={(e) => setForm({ ...form, status: e.target.value })}
            >
              <option value="draft">Draft</option>
              <option value="issued">Issued</option>
              <option value="paid">Paid</option>
              <option value="overdue">Overdue</option>
              <option value="void">Void</option>
            </select>
          </div>
          <div>
            <label className={labelClass}>Issued Date</label>
            <input type="date" className={inputClass} value={form.issuedAt} onChange={(e) => setForm({ ...form, issuedAt: e.target.value })} />
          </div>
          <div>
            <label className={labelClass}>Due Date</label>
            <input type="date" className={inputClass} value={form.dueAt} onChange={(e) => setForm({ ...form, dueAt: e.target.value })} />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Tax ($)</label>
            <input type="number" className={inputClass} value={form.tax} onChange={(e) => setForm({ ...form, tax: Number(e.target.value) })} />
          </div>
          <div>
            <label className={labelClass}>Notes</label>
            <input className={inputClass} value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} placeholder="Optional notes" />
          </div>
        </div>

        {/* Line items (only for create without subscription) */}
        {isCreate && !form.subscriptionId && (
          <div>
            <div className="flex items-center justify-between mb-2">
              <label className={labelClass}>Line Items</label>
              <Button variant="ghost" size="sm" onClick={addItem} className="h-7 text-xs">
                <Plus className="w-3 h-3 mr-1" /> Add Item
              </Button>
            </div>
            <div className="space-y-2">
              {form.items.map((item, idx) => (
                <div key={idx} className="flex items-center gap-2">
                  <input
                    data-testid={`invoice-item-description-${idx}`}
                    className={cn(inputClass, "flex-1")}
                    placeholder="Description"
                    value={item.description}
                    onChange={(e) => updateItem(idx, "description", e.target.value)}
                  />
                  <input
                    data-testid={`invoice-item-qty-${idx}`}
                    type="number"
                    className={cn(inputClass, "w-20")}
                    placeholder="Qty"
                    value={item.quantity}
                    onChange={(e) => updateItem(idx, "quantity", Number(e.target.value))}
                  />
                  <input
                    data-testid={`invoice-item-price-${idx}`}
                    type="number"
                    className={cn(inputClass, "w-28")}
                    placeholder="Price"
                    value={item.unitPrice}
                    onChange={(e) => updateItem(idx, "unitPrice", Number(e.target.value))}
                  />
                  {form.items.length > 1 && (
                    <Button variant="ghost" size="icon" className="h-8 w-8 shrink-0" onClick={() => removeItem(idx)}>
                      <Trash2 className="w-3.5 h-3.5 text-muted-foreground" />
                    </Button>
                  )}
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <DialogFooter className="mt-6">
        <Button variant="outline" onClick={onCancel}>Cancel</Button>
        <Button data-testid="invoice-form-submit" disabled={loading || (isCreate && !form.customerId)} onClick={() => {
          const submitData: Record<string, unknown> = {
            status: form.status,
            tax: form.tax,
            notes: form.notes || null,
            issuedAt: form.issuedAt,
            dueAt: form.dueAt,
          };
          if (isCreate) {
            submitData.customerId = form.customerId;
            submitData.subscriptionId = form.subscriptionId || null;
            if (!form.subscriptionId) {
              submitData.items = form.items.filter((i) => i.description);
            }
          }
          onSubmit(submitData);
        }}>
          {loading ? "Saving..." : isCreate ? "Create" : "Save"}
        </Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- Record Payment dialog ---------- */
function PaymentForm({ invoice, onSubmit, onCancel, loading }: {
  invoice: Inv;
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors";
  const today = new Date().toISOString().split("T")[0];

  const [form, setForm] = useState({
    amount: (invoice.total as number) ?? 0,
    method: "manual",
    reference: "",
    paidAt: today,
    notes: "",
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>Record Payment</DialogTitle>
        <DialogDescription>Invoice {invoice.invoiceNumber as string} — Total ${(invoice.total as number).toLocaleString()}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Amount ($)</label>
            <input type="number" step="0.01" className={inputClass} value={form.amount} onChange={(e) => setForm({ ...form, amount: Number(e.target.value) })} />
          </div>
          <div>
            <label className={labelClass}>Method</label>
            <select className={inputClass} value={form.method} onChange={(e) => setForm({ ...form, method: e.target.value })}>
              <option value="manual">Manual</option>
              <option value="bank_transfer">Bank Transfer</option>
              <option value="check">Check</option>
              <option value="stripe">Stripe</option>
              <option value="xendit">Xendit</option>
              <option value="lemonsqueezy">Lemonsqueezy</option>
            </select>
          </div>
        </div>
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Payment Date</label>
            <input type="date" className={inputClass} value={form.paidAt} onChange={(e) => setForm({ ...form, paidAt: e.target.value })} />
          </div>
          <div>
            <label className={labelClass}>Reference</label>
            <input className={inputClass} value={form.reference} onChange={(e) => setForm({ ...form, reference: e.target.value })} placeholder="Check #, transfer ref, etc." />
          </div>
        </div>
        <div>
          <label className={labelClass}>Notes</label>
          <input className={inputClass} value={form.notes} onChange={(e) => setForm({ ...form, notes: e.target.value })} placeholder="Optional" />
        </div>
      </div>

      <DialogFooter className="mt-6">
        <Button variant="outline" onClick={onCancel}>Cancel</Button>
        <Button disabled={loading} onClick={() => onSubmit({
          invoiceId: invoice.id,
          amount: form.amount,
          method: form.method,
          paidAt: form.paidAt,
          reference: form.reference || null,
          notes: form.notes || null,
        })}>
          {loading ? "Recording..." : "Record Payment"}
        </Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- Main section ---------- */
export function ManageInvoicesSection() {
  const { data: invs, isLoading, mutate } = useInvoices();
  const { data: customerList } = useCustomers();
  const { data: subList } = useSubscriptions();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Inv | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Inv | null>(null);
  const [paymentDialogOpen, setPaymentDialogOpen] = useState(false);
  const [paymentTarget, setPaymentTarget] = useState<Inv | null>(null);
  const [creditNoteDialogOpen, setCreditNoteDialogOpen] = useState(false);
  const [creditNoteTarget, setCreditNoteTarget] = useState<Inv | null>(null);
  const [refundDialogOpen, setRefundDialogOpen] = useState(false);
  const [refundTarget, setRefundTarget] = useState<Inv | null>(null);
  const [checkoutDialogOpen, setCheckoutDialogOpen] = useState(false);
  const [checkoutTarget, setCheckoutTarget] = useState<Inv | null>(null);
  const [checkoutLoading, setCheckoutLoading] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // For view mode, fetch full invoice with items + payments
  const [fullInvoice, setFullInvoice] = useState<Inv | null>(null);

  const customers = (customerList || []) as Inv[];
  const subscriptions = (subList || []) as Inv[];
  const customerNameById = new Map(
    customers.map((c) => [c.id as string, (c.name as string) ?? "—"]),
  );
  const subscriptionById = new Map(
    subscriptions.map((s) => [s.id as string, s]),
  );

  const invoices = ((invs || []) as Inv[]).map((inv) => {
    const customerId = inv.customerId as string;
    const subscriptionId = inv.subscriptionId as string;
    const sub = subscriptionById.get(subscriptionId);
    const subCustomerName = (sub?.customerName as string) ?? "";
    return {
      ...inv,
      customerName:
        (inv.customerName as string) ??
        customerNameById.get(customerId) ??
        (subCustomerName || "—"),
    };
  });

  const filtered = invoices.filter((i: Inv) =>
    ((i.invoiceNumber as string) ?? "").toLowerCase().includes(search.toLowerCase()) ||
    ((i.customerName as string) ?? "").toLowerCase().includes(search.toLowerCase())
  );

  const openView = async (inv: Inv) => {
    // Fetch full invoice with items and payments
    try {
      const res = await fetch(`/api/billing/invoices/${inv.id}`);
      const data = await res.json();
      const normalized = normalizeInvoicePayload(data as Inv);
      const customerId = normalized.customerId as string;
      const subscriptionId = normalized.subscriptionId as string;
      const sub = subscriptionById.get(subscriptionId);
      setFullInvoice({
        ...normalized,
        customerName:
          (normalized.customerName as string) ??
          customerNameById.get(customerId) ??
          (sub?.customerName as string) ??
          "—",
      });
      setSelected(inv);
      setDialogMode("view");
      setDialogOpen(true);
    } catch {
      toast.error("Failed to load invoice details");
    }
  };

  const openDialog = (inv: Inv | null, mode: DialogMode) => {
    setSelected(inv);
    setFullInvoice(null);
    setDialogMode(mode);
    setDialogOpen(true);
  };

  const handleSubmit = async (data: Record<string, unknown>) => {
    setSaving(true);
    if (dialogMode === "create") {
      const result = await createInvoice(data);
      if (result.success) {
        toast.success("Invoice created");
        setDialogOpen(false);
        mutate();
      }
    } else {
      const result = await updateInvoice(selected!.id as string, data);
      if (result.success) {
        toast.success("Invoice updated");
        setDialogOpen(false);
        mutate();
      }
    }
    setSaving(false);
  };

  const handlePayment = async (data: Record<string, unknown>) => {
    setSaving(true);
    const result = await createPayment(data);
    if (result.success) {
      toast.success("Payment recorded");
      setPaymentDialogOpen(false);
      setPaymentTarget(null);
      mutate();
    }
    setSaving(false);
  };

  const handleCreditNote = async (data: Record<string, unknown>) => {
    setSaving(true);
    const result = await createCreditNote(data);
    if (result.success) {
      toast.success("Credit note issued");
      setCreditNoteDialogOpen(false);
      setCreditNoteTarget(null);
      mutate();
    }
    setSaving(false);
  };

  const handleRefund = async (data: Record<string, unknown>) => {
    setSaving(true);
    const result = await createRefund(data);
    if (result.success) {
      toast.success("Refund processed");
      setRefundDialogOpen(false);
      setRefundTarget(null);
      mutate();
    }
    setSaving(false);
  };

  const handleCheckout = async (provider: "stripe" | "xendit" | "lemonsqueezy") => {
    if (!checkoutTarget) return;
    setCheckoutLoading(provider);
    const result = await getCheckout(checkoutTarget.id as string, provider);
    setCheckoutLoading(null);
    if (!result.success) {
      toast.error(result.error ?? "Failed to generate checkout link");
      return;
    }
    navigator.clipboard.writeText(result.data.checkoutUrl);
    toast.success(`${provider} checkout link copied to clipboard`);
    window.open(result.data.checkoutUrl, "_blank");
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const result = await deleteInvoice(deleteTarget.id as string);
    if (result.success) {
      toast.success("Invoice deleted");
      setDeleteTarget(null);
      mutate();
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-4">
        <Skeleton className="h-10 w-full rounded-lg" />
        {[...Array(5)].map((_, i) => <Skeleton key={i} className="h-14 w-full rounded-lg" />)}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search invoices..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20"
          />
        </div>
        <Button data-testid="new-invoice-button" size="sm" onClick={() => openDialog(null, "create")}>
          <Plus className="w-4 h-4 mr-1" /> New Invoice
        </Button>
      </div>

      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Invoice #</TableHead>
              <TableHead>Customer</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="text-right">Total</TableHead>
              <TableHead>Issued</TableHead>
              <TableHead>Due</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">No invoices found</TableCell>
              </TableRow>
            ) : filtered.map((inv: Inv) => (
              <TableRow
                data-testid={`invoice-row-${inv.id as string}`}
                key={inv.id as string}
                className="cursor-pointer hover:bg-secondary/30"
                onClick={() => openView(inv)}
              >
                <TableCell className="font-mono text-xs">{inv.invoiceNumber as string}</TableCell>
                <TableCell className="font-medium">{(inv.customerName as string) ?? "—"}</TableCell>
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
                <TableCell>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <Button data-testid={`invoice-row-menu-${inv.id as string}`} variant="ghost" size="icon" className="h-8 w-8"><MoreHorizontal className="w-4 h-4" /></Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openView(inv); }}>
                        <Eye className="w-4 h-4 mr-2" /> View
                      </DropdownMenuItem>
                      <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openDialog(inv, "edit"); }}>
                        <Pencil className="w-4 h-4 mr-2" /> Edit
                      </DropdownMenuItem>
                      {(inv.status === "issued" || inv.status === "overdue") && (
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); setPaymentTarget(inv); setPaymentDialogOpen(true); }}>
                          <DollarSign className="w-4 h-4 mr-2" /> Record Payment
                        </DropdownMenuItem>
                      )}
                      <DropdownMenuSeparator />
                      <DropdownMenuItem className="text-destructive" onClick={(e) => { e.stopPropagation(); setDeleteTarget(inv); }}>
                        <Trash2 className="w-4 h-4 mr-2" /> Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* View / Create / Edit dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          {dialogMode === "view" && fullInvoice ? (
            <InvoiceDetail
              invoice={fullInvoice}
              onEdit={() => setDialogMode("edit")}
              onDelete={() => {
                setDialogOpen(false);
                setDeleteTarget(fullInvoice);
              }}
              onRecordPayment={() => {
                setDialogOpen(false);
                setPaymentTarget(fullInvoice);
                setPaymentDialogOpen(true);
              }}
              onCreditNote={() => {
                setDialogOpen(false);
                setCreditNoteTarget(fullInvoice);
                setCreditNoteDialogOpen(true);
              }}
              onRefund={() => {
                setDialogOpen(false);
                setRefundTarget(fullInvoice);
                setRefundDialogOpen(true);
              }}
              onCheckout={() => {
                setDialogOpen(false);
                setCheckoutTarget(fullInvoice);
                setCheckoutDialogOpen(true);
              }}
            />
          ) : (
            <InvoiceForm
              invoice={selected ?? undefined}
              mode={dialogMode}
              customers={customers}
              subscriptions={subscriptions}
              onSubmit={handleSubmit}
              onCancel={() => setDialogOpen(false)}
              loading={saving}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Payment dialog */}
      <Dialog open={paymentDialogOpen} onOpenChange={setPaymentDialogOpen}>
        <DialogContent className="max-w-md">
          {paymentTarget && (
            <PaymentForm
              invoice={paymentTarget}
              onSubmit={handlePayment}
              onCancel={() => { setPaymentDialogOpen(false); setPaymentTarget(null); }}
              loading={saving}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Credit Note dialog */}
      <Dialog open={creditNoteDialogOpen} onOpenChange={setCreditNoteDialogOpen}>
        <DialogContent className="max-w-lg">
          {creditNoteTarget && (
            <CreditNoteForm
              invoice={creditNoteTarget}
              onSubmit={handleCreditNote}
              onCancel={() => { setCreditNoteDialogOpen(false); setCreditNoteTarget(null); }}
              loading={saving}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Refund dialog */}
      <Dialog open={refundDialogOpen} onOpenChange={setRefundDialogOpen}>
        <DialogContent className="max-w-md">
          {refundTarget && (
            <RefundForm
              invoice={refundTarget}
              onSubmit={handleRefund}
              onCancel={() => { setRefundDialogOpen(false); setRefundTarget(null); }}
              loading={saving}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Delete dialog */}
      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        onConfirm={handleDelete}
        title="Delete Invoice"
        description={`Are you sure you want to delete invoice "${deleteTarget?.invoiceNumber}"? This will also remove all associated payments.`}
      />

      {/* Checkout link dialog */}
      <Dialog open={checkoutDialogOpen} onOpenChange={(open) => { if (!open) { setCheckoutDialogOpen(false); setCheckoutTarget(null); } }}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Generate Checkout Link</DialogTitle>
            <DialogDescription>
              {checkoutTarget && <>Invoice {checkoutTarget.invoiceNumber as string} — ${(checkoutTarget.total as number).toLocaleString()}</>}
            </DialogDescription>
          </DialogHeader>
          <div className="mt-2 space-y-2">
            {(["xendit", "lemonsqueezy", "stripe"] as const).map((provider) => {
              const labels: Record<string, { name: string; desc: string }> = {
                xendit: { name: "Xendit", desc: "VA, e-wallet, QRIS, bank transfer" },
                lemonsqueezy: { name: "Lemonsqueezy", desc: "International cards, PayPal" },
                stripe: { name: "Stripe", desc: "Credit/debit cards" },
              };
              return (
                <button
                  key={provider}
                  onClick={() => handleCheckout(provider)}
                  disabled={!!checkoutLoading}
                  className={cn(
                    "w-full flex items-center justify-between p-3 rounded-lg border border-border bg-secondary/30 hover:bg-secondary/60 transition-colors text-left",
                    checkoutLoading === provider && "ring-2 ring-accent"
                  )}
                >
                  <div>
                    <p className="text-sm font-medium text-foreground">{labels[provider].name}</p>
                    <p className="text-xs text-muted-foreground">{labels[provider].desc}</p>
                  </div>
                  {checkoutLoading === provider ? (
                    <Loader2 className="w-4 h-4 animate-spin text-accent shrink-0" />
                  ) : (
                    <ExternalLink className="w-4 h-4 text-muted-foreground shrink-0" />
                  )}
                </button>
              );
            })}
          </div>
          <DialogFooter className="mt-4">
            <Button variant="outline" onClick={() => { setCheckoutDialogOpen(false); setCheckoutTarget(null); }}>
              Close
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
