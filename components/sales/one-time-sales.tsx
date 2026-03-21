"use client";

import { useMemo, useState } from "react";
import { Plus, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DeleteDialog } from "@/components/management/delete-dialog";
import { createLicense, createOneTimeSale, deleteOneTimeSale, updateOneTimeSale, useCustomers, useOneTimeSales, useProducts } from "@/hooks/use-api";

type Row = Record<string, unknown>;
type DialogMode = "create" | "view" | "edit";

function toNumber(value: string): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

function OneTimeSaleForm({
  row,
  mode,
  customers,
  customerNameById,
  products,
  loading,
  onCancel,
  onSubmit,
}: {
  row?: Row;
  mode: DialogMode;
  customers: Row[];
  customerNameById: Map<string, string>;
  products: Row[];
  loading: boolean;
  onCancel: () => void;
  onSubmit: (payload: Record<string, unknown>) => void;
}) {
  const isView = mode === "view";
  const isCreate = mode === "create";

  const [form, setForm] = useState({
    customerId: String(row?.customerId ?? row?.customer_id ?? ""),
    currency: String(row?.currency ?? "USD"),
    subtotal: String(row?.subtotal ?? 0),
    tax: String(row?.tax ?? 0),
    dueAt: String(row?.dueAt ?? row?.due_at ?? "").slice(0, 10),
    notes: String(row?.notes ?? ""),
    createLicense: false,
    licenseProductId: "",
    licenseStartsAt: "",
    licenseExpiresAt: "",
    licenseMaxActivations: "",
  });

  const total = useMemo(() => toNumber(form.subtotal) + toNumber(form.tax), [form.subtotal, form.tax]);
  const resolvedCustomerName =
    String(row?.customerName ?? row?.customer_name ?? "") ||
    customerNameById.get(form.customerId) ||
    "—";

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Create One-Time Sale" : mode === "edit" ? "Edit One-Time Sale" : "One-Time Sale Details"}</DialogTitle>
        <DialogDescription>
          {isCreate ? "Issue a one-time invoice sale." : mode === "edit" ? "Update invoice sale fields." : "View invoice sale details."}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <Label>Customer</Label>
            {isView ? (
              <p className="text-sm">{resolvedCustomerName}</p>
            ) : (
              <select className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm" value={form.customerId} onChange={(e) => setForm((p) => ({ ...p, customerId: e.target.value }))} disabled={!isCreate}>
                <option value="">Select customer</option>
                {customers.map((c) => (
                  <option key={String(c.id)} value={String(c.id)}>{String(c.name ?? "Unknown")}</option>
                ))}
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>Currency</Label>
            {isView ? <p className="text-sm">{form.currency}</p> : <Input value={form.currency} onChange={(e) => setForm((p) => ({ ...p, currency: e.target.value.toUpperCase() }))} disabled={!isCreate} />}
          </div>
          <div className="space-y-1.5">
            <Label>Subtotal</Label>
            {isView ? <p className="text-sm">${toNumber(form.subtotal).toLocaleString()}</p> : <Input type="number" min="0" step="0.01" value={form.subtotal} onChange={(e) => setForm((p) => ({ ...p, subtotal: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Tax</Label>
            {isView ? <p className="text-sm">${toNumber(form.tax).toLocaleString()}</p> : <Input type="number" min="0" step="0.01" value={form.tax} onChange={(e) => setForm((p) => ({ ...p, tax: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Due Date</Label>
            {isView ? <p className="text-sm">{form.dueAt || "—"}</p> : <Input type="date" value={form.dueAt} onChange={(e) => setForm((p) => ({ ...p, dueAt: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Notes</Label>
            {isView ? <p className="text-sm">{form.notes || "—"}</p> : <Input value={form.notes} onChange={(e) => setForm((p) => ({ ...p, notes: e.target.value }))} />}
          </div>
          {!isView && (
            <div className="md:col-span-2 space-y-3 rounded-lg border border-border bg-secondary/30 p-3">
              <label className="inline-flex items-center gap-2 text-sm">
                <input
                  type="checkbox"
                  checked={form.createLicense}
                  onChange={(e) => setForm((p) => ({ ...p, createLicense: e.target.checked }))}
                />
                Generate license for this sale
              </label>
              {form.createLicense && (
                <div className="grid grid-cols-1 md:grid-cols-4 gap-3">
                  <div className="space-y-1.5">
                    <Label>License Product</Label>
                    <select className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm" value={form.licenseProductId} onChange={(e) => setForm((p) => ({ ...p, licenseProductId: e.target.value }))}>
                      <option value="">Select product</option>
                      {products.map((p) => (
                        <option key={String(p.id)} value={String(p.id)}>{String(p.name ?? "Product")}</option>
                      ))}
                    </select>
                  </div>
                  <div className="space-y-1.5">
                    <Label>License Starts (optional)</Label>
                    <Input type="date" value={form.licenseStartsAt} onChange={(e) => setForm((p) => ({ ...p, licenseStartsAt: e.target.value }))} />
                  </div>
                  <div className="space-y-1.5">
                    <Label>License Expires (optional)</Label>
                    <Input type="date" value={form.licenseExpiresAt} onChange={(e) => setForm((p) => ({ ...p, licenseExpiresAt: e.target.value }))} />
                  </div>
                  <div className="space-y-1.5">
                    <Label>Max Activations (optional)</Label>
                    <Input type="number" min="1" value={form.licenseMaxActivations} onChange={(e) => setForm((p) => ({ ...p, licenseMaxActivations: e.target.value }))} />
                  </div>
                </div>
              )}
            </div>
          )}
        </div>
        <p className="text-sm text-muted-foreground">Total: <span className="font-medium text-foreground">{form.currency} {total.toLocaleString()}</span></p>
      </div>

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button
            disabled={loading || !form.customerId || toNumber(form.subtotal) <= 0 || (form.createLicense && !form.licenseProductId)}
            onClick={() =>
              onSubmit({
                customerId: form.customerId,
                currency: form.currency,
                subtotal: toNumber(form.subtotal),
                tax: toNumber(form.tax),
                total,
                dueAt: form.dueAt || null,
                notes: form.notes || null,
                status: "issued",
                createLicense: form.createLicense,
                licenseProductId: form.licenseProductId || null,
                licenseStartsAt: form.licenseStartsAt || null,
                licenseExpiresAt: form.licenseExpiresAt || null,
                licenseMaxActivations: form.licenseMaxActivations ? Number(form.licenseMaxActivations) : null,
              })
            }
          >
            {loading ? "Saving..." : isCreate ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function OneTimeSalesSection() {
  const { data: saleList, mutate } = useOneTimeSales();
  const { data: customerList } = useCustomers();
  const { data: productList } = useProducts();

  const rows = (saleList ?? []) as Row[];
  const customers = (customerList ?? []) as Row[];
  const products = (productList ?? []) as Row[];
  const customerNameById = new Map(
    customers.map((c) => [String(c.id), String(c.name ?? "—")]),
  );

  const [dialogOpen, setDialogOpen] = useState(false);
  const [mode, setMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Row | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Row | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  const openCreate = () => {
    setSelected(null);
    setMode("create");
    setDialogOpen(true);
  };

  const openDialog = (row: Row | null, dialogMode: DialogMode) => {
    setSelected(row);
    setMode(dialogMode);
    setDialogOpen(true);
  };

  const submit = async (payload: Record<string, unknown>) => {
    const {
      createLicense: shouldCreateLicense,
      licenseProductId,
      licenseStartsAt,
      licenseExpiresAt,
      licenseMaxActivations,
      ...invoicePayload
    } = payload;

    setSaving(true);
    const result = mode === "create"
      ? await createOneTimeSale(invoicePayload)
      : await updateOneTimeSale(String(selected?.id), invoicePayload);

    if (result.success && shouldCreateLicense) {
      const customerId = String(invoicePayload.customerId ?? "");
      const customer = customers.find((c) => String(c.id) === customerId);
      const product = products.find((p) => String(p.id) === String(licenseProductId));
      const licenseResult = await createLicense({
        customerId,
        customerName: String(customer?.name ?? ""),
        productId: String(licenseProductId ?? ""),
        productName: String(product?.name ?? ""),
        startsAt: licenseStartsAt || null,
        expiresAt: licenseExpiresAt || null,
        maxActivations: licenseMaxActivations ?? null,
        licenseType: "simple",
      });
      if (!licenseResult.success) {
        setSaving(false);
        return toast.error(licenseResult.error ?? "Sale saved, but failed to generate license");
      }
    }

    setSaving(false);
    if (!result.success) return toast.error(result.error ?? "Failed to save one-time sale");
    toast.success(mode === "create" ? "One-time sale created" : "One-time sale updated");
    setDialogOpen(false);
    mutate();
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await deleteOneTimeSale(String(deleteTarget.id));
    setDeleting(false);
    if (!result.success) return toast.error(result.error ?? "Failed to delete one-time sale");
    toast.success("One-time sale deleted");
    setDeleteTarget(null);
    mutate();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">One-Time Sales</h2>
        <Button onClick={openCreate}><Plus className="w-4 h-4 mr-2" />Create</Button>
      </div>

      <div className="rounded-lg border border-border overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Invoice</TableHead>
              <TableHead>Customer</TableHead>
              <TableHead className="text-right">Total</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="w-[52px]" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={String(row.id)} className="cursor-pointer hover:bg-secondary/30" onClick={() => openDialog(row, "view")}>
                <TableCell className="font-mono text-xs">{String(row.invoiceNumber ?? row.invoice_number ?? "—")}</TableCell>
                <TableCell>
                  {String(
                    row.customerName ??
                      row.customer_name ??
                      customerNameById.get(String(row.customerId ?? row.customer_id ?? "")) ??
                      "—",
                  )}
                </TableCell>
                <TableCell className="text-right">${Number(row.total ?? 0).toLocaleString()}</TableCell>
                <TableCell className="capitalize">{String(row.status ?? "draft")}</TableCell>
                <TableCell onClick={(e) => e.stopPropagation()}>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <Button size="icon" variant="ghost"><MoreHorizontal className="w-4 h-4" /></Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={() => openDialog(row, "view")}>View</DropdownMenuItem>
                      <DropdownMenuItem onClick={() => openDialog(row, "edit")}><Pencil className="w-4 h-4 mr-2" />Edit</DropdownMenuItem>
                      <DropdownMenuItem className="text-red-400" onClick={() => setDeleteTarget(row)}><Trash2 className="w-4 h-4 mr-2" />Delete</DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          <OneTimeSaleForm row={selected ?? undefined} mode={mode} customers={customers} customerNameById={customerNameById} products={products} loading={saving} onCancel={() => setDialogOpen(false)} onSubmit={submit} />
          {mode === "view" && (
            <DialogFooter className="w-full sm:justify-between">
              <Button
                variant="outline"
                className="text-destructive hover:text-destructive hover:bg-destructive/10"
                onClick={() => {
                  if (selected) {
                    setDialogOpen(false);
                    setDeleteTarget(selected);
                  }
                }}
              >
                <Trash2 className="w-4 h-4 mr-1" /> Delete
              </Button>
              <Button variant="outline" onClick={() => setMode("edit")}>
                <Pencil className="w-4 h-4 mr-1" /> Edit
              </Button>
            </DialogFooter>
          )}
        </DialogContent>
      </Dialog>

      <DeleteDialog
        open={Boolean(deleteTarget)}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        title="Delete one-time sale"
        description="This will void and remove the invoice sale."
        onConfirm={confirmDelete}
        loading={deleting}
      />
    </div>
  );
}
