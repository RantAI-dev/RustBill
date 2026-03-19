"use client";

import { useEffect, useMemo, useState } from "react";
import { MoreHorizontal, Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DeleteDialog } from "@/components/management/delete-dialog";
import { adjustCredits, createLicense, deleteCreditAdjustment, updateCreditAdjustment, useCustomerCredits, useCustomers, useProducts } from "@/hooks/use-api";

type Row = Record<string, unknown>;
type DialogMode = "create" | "view" | "edit";

function toNumber(value: string): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

function CreditForm({
  mode,
  row,
  customers,
  products,
  loading,
  onCancel,
  onSubmit,
}: {
  mode: DialogMode;
  row?: Row;
  customers: Row[];
  products: Row[];
  loading: boolean;
  onCancel: () => void;
  onSubmit: (payload: Record<string, unknown>) => void;
}) {
  const isView = mode === "view";
  const isCreate = mode === "create";
  const [form, setForm] = useState({
    customerId: String(row?.customer_id ?? row?.customerId ?? ""),
    currency: String(row?.currency ?? "USD"),
    amount: String(Math.abs(Number(row?.amount ?? 0))),
    description: String(row?.description ?? "Manual token/credit grant"),
    createLicense: false,
    licenseProductId: "",
    licenseStartsAt: "",
    licenseExpiresAt: "",
    licenseMaxActivations: "",
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Create Token/Credit Sale" : mode === "edit" ? "Edit Token/Credit Sale" : "Token/Credit Sale Details"}</DialogTitle>
        <DialogDescription>
          {isCreate ? "Create a manual credit adjustment." : mode === "edit" ? "Edit this manual credit adjustment." : "View token/credit adjustment details."}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <Label>Customer</Label>
            {isView ? (
              <p className="text-sm">{String(form.customerId || "—")}</p>
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
            <Label>Amount</Label>
            {isView ? <p className="text-sm">{Number(form.amount).toLocaleString()}</p> : <Input type="number" min="0" step="0.01" value={form.amount} onChange={(e) => setForm((p) => ({ ...p, amount: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Description</Label>
            {isView ? <p className="text-sm">{form.description || "—"}</p> : <Input value={form.description} onChange={(e) => setForm((p) => ({ ...p, description: e.target.value }))} />}
          </div>
          {!isView && (
            <div className="md:col-span-2 space-y-3 rounded-lg border border-border bg-secondary/30 p-3">
              <label className="inline-flex items-center gap-2 text-sm">
                <input type="checkbox" checked={form.createLicense} onChange={(e) => setForm((p) => ({ ...p, createLicense: e.target.checked }))} />
                Generate license for this token/credit sale
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
      </div>

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button
            disabled={loading || !form.customerId || toNumber(form.amount) <= 0 || (form.createLicense && !form.licenseProductId)}
            onClick={() => onSubmit({
              customerId: form.customerId,
              currency: form.currency,
              amount: toNumber(form.amount),
              description: form.description,
              createLicense: form.createLicense,
              licenseProductId: form.licenseProductId || null,
              licenseStartsAt: form.licenseStartsAt || null,
              licenseExpiresAt: form.licenseExpiresAt || null,
              licenseMaxActivations: form.licenseMaxActivations ? Number(form.licenseMaxActivations) : null,
            })}
          >
            {loading ? "Saving..." : isCreate ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function TokenCreditsSalesSection() {
  const { data: customerList } = useCustomers();
  const { data: productList } = useProducts();
  const customers = useMemo(() => (customerList ?? []) as Row[], [customerList]);
  const products = (productList ?? []) as Row[];

  const [customerId, setCustomerId] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [mode, setMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Row | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Row | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!customerId && customers.length > 0) {
      setCustomerId(String(customers[0].id));
    }
  }, [customerId, customers]);

  const { data: credits, mutate } = useCustomerCredits(customerId || undefined);
  const balance = Number((credits as Row | undefined)?.balance ?? 0);
  const history = (((credits as Row | undefined)?.history ?? []) as Row[])
    .filter((row) => String(row.reason ?? "") === "manual")
    .filter((row) => !(row.invoiceId as string) && !(row.invoice_id as string));

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
      ...creditPayload
    } = payload;

    setSaving(true);
    const result = mode === "create"
      ? await adjustCredits(creditPayload)
      : await updateCreditAdjustment(String(selected?.id), {
          amount: creditPayload.amount,
          description: creditPayload.description,
        });

    if (result.success && shouldCreateLicense) {
      const customer = customers.find((c) => String(c.id) === String(creditPayload.customerId));
      const product = products.find((p) => String(p.id) === String(licenseProductId));
      const licenseResult = await createLicense({
        customerId: String(creditPayload.customerId),
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
        return toast.error(licenseResult.error ?? "Credit sale saved, but failed to generate license");
      }
    }

    setSaving(false);
    if (!result.success) return toast.error(result.error ?? "Failed to save token/credit sale");
    toast.success(mode === "create" ? "Token/credit sale created" : "Token/credit sale updated");
    setDialogOpen(false);
    mutate();
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await deleteCreditAdjustment(String(deleteTarget.id));
    setDeleting(false);
    if (!result.success) return toast.error(result.error ?? "Failed to delete token/credit sale");
    toast.success("Token/credit sale deleted");
    setDeleteTarget(null);
    mutate();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">Token Credits Sales</h2>
        <div className="flex items-center gap-2">
          <select className="h-9 rounded-lg border border-border bg-secondary px-3 text-sm" value={customerId} onChange={(e) => setCustomerId(e.target.value)}>
            <option value="">Select customer</option>
            {customers.map((c) => (
              <option key={String(c.id)} value={String(c.id)}>{String(c.name ?? "Unknown")}</option>
            ))}
          </select>
          <Button onClick={openCreate}><Plus className="w-4 h-4 mr-2" />Create</Button>
        </div>
      </div>

      <p className="text-sm text-muted-foreground">Current balance: <span className="font-medium text-foreground">{Number(balance).toLocaleString()}</span></p>

      <div className="rounded-lg border border-border overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Date</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="text-right">Amount</TableHead>
              <TableHead className="text-right">Balance After</TableHead>
              <TableHead className="w-[52px]" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {history.map((row) => (
              <TableRow key={String(row.id)} className="cursor-pointer hover:bg-secondary/30" onClick={() => openDialog(row, "view")}>
                <TableCell>{new Date(String(row.createdAt ?? row.created_at ?? Date.now())).toLocaleString()}</TableCell>
                <TableCell>{String(row.description ?? "—")}</TableCell>
                <TableCell className="text-right">{String(row.amount ?? 0)}</TableCell>
                <TableCell className="text-right">{String(row.balanceAfter ?? row.balance_after ?? 0)}</TableCell>
                <TableCell onClick={(e) => e.stopPropagation()}>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <Button variant="ghost" size="icon"><MoreHorizontal className="w-4 h-4" /></Button>
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
          <CreditForm mode={mode} row={selected ?? undefined} customers={customers} products={products} loading={saving} onCancel={() => setDialogOpen(false)} onSubmit={submit} />
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
        title="Delete token/credit sale"
        description="This adds a reversal adjustment to remove the selected manual credit amount."
        onConfirm={confirmDelete}
        loading={deleting}
      />
    </div>
  );
}
