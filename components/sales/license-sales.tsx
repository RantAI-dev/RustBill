"use client";

import { useState } from "react";
import { MoreHorizontal, Pencil, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DeleteDialog } from "@/components/management/delete-dialog";
import { createLicense, deleteLicense, updateLicense, useCustomers, useLicenses, useProducts } from "@/hooks/use-api";

type Row = Record<string, unknown>;
type DialogMode = "create" | "view" | "edit";

function LicenseForm({
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
    customerId: String(row?.customerId ?? row?.customer_id ?? ""),
    productId: String(row?.productId ?? row?.product_id ?? ""),
    customerName: String(row?.customerName ?? row?.customer_name ?? ""),
    productName: String(row?.productName ?? row?.product_name ?? ""),
    startsAt: String(row?.createdAt ?? row?.created_at ?? "").slice(0, 10),
    expiresAt: String(row?.expiresAt ?? row?.expires_at ?? "").slice(0, 10),
    maxActivations: String(row?.maxActivations ?? row?.max_activations ?? ""),
    licenseType: String(row?.licenseType ?? row?.license_type ?? "simple"),
    features: Array.isArray(row?.features) ? (row?.features as string[]).join(",") : "",
    status: String(row?.status ?? "active"),
  });

  const selectedCustomer = customers.find((c) => String(c.id) === form.customerId);
  const selectedProduct = products.find((p) => String(p.id) === form.productId);

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Create License Sale" : mode === "edit" ? "Edit License Sale" : "License Sale Details"}</DialogTitle>
        <DialogDescription>
          {isCreate ? "Issue a license for a sales transaction." : mode === "edit" ? "Update license sale fields." : "View license sale details."}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <Label>Customer</Label>
            {isView ? (
              <p className="text-sm">{form.customerName || "—"}</p>
            ) : (
              <select
                className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm"
                value={form.customerId}
                onChange={(e) => {
                  const customerId = e.target.value;
                  const customer = customers.find((c) => String(c.id) === customerId);
                  setForm((p) => ({ ...p, customerId, customerName: String(customer?.name ?? "") }));
                }}
                disabled={!isCreate}
              >
                <option value="">Select customer</option>
                {customers.map((c) => (
                  <option key={String(c.id)} value={String(c.id)}>{String(c.name ?? "Unknown")}</option>
                ))}
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>Product</Label>
            {isView ? (
              <p className="text-sm">{form.productName || "—"}</p>
            ) : (
              <select
                className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm"
                value={form.productId}
                onChange={(e) => {
                  const productId = e.target.value;
                  const product = products.find((p) => String(p.id) === productId);
                  setForm((p) => ({ ...p, productId, productName: String(product?.name ?? "") }));
                }}
                disabled={!isCreate}
              >
                <option value="">Select product</option>
                {products.map((p) => (
                  <option key={String(p.id)} value={String(p.id)}>{String(p.name ?? "Product")}</option>
                ))}
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>License Type</Label>
            {isView ? <p className="text-sm capitalize">{form.licenseType}</p> : (
              <select className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm" value={form.licenseType} onChange={(e) => setForm((p) => ({ ...p, licenseType: e.target.value }))}>
                <option value="simple">Simple</option>
                <option value="signed">Signed</option>
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>Status</Label>
            {isView ? <p className="text-sm capitalize">{form.status}</p> : (
              <select className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm" value={form.status} onChange={(e) => setForm((p) => ({ ...p, status: e.target.value }))}>
                <option value="active">Active</option>
                <option value="suspended">Suspended</option>
                <option value="revoked">Revoked</option>
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>Starts At</Label>
            {isView ? <p className="text-sm">{form.startsAt || "—"}</p> : <Input type="date" value={form.startsAt} onChange={(e) => setForm((p) => ({ ...p, startsAt: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Expires At</Label>
            {isView ? <p className="text-sm">{form.expiresAt || "—"}</p> : <Input type="date" value={form.expiresAt} onChange={(e) => setForm((p) => ({ ...p, expiresAt: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Max Activations</Label>
            {isView ? <p className="text-sm">{form.maxActivations || "Unlimited"}</p> : <Input type="number" min="1" value={form.maxActivations} onChange={(e) => setForm((p) => ({ ...p, maxActivations: e.target.value }))} />}
          </div>
        </div>
        <div className="space-y-1.5">
          <Label>Features (comma separated)</Label>
          {isView ? <p className="text-sm">{form.features || "—"}</p> : <Input value={form.features} onChange={(e) => setForm((p) => ({ ...p, features: e.target.value }))} />}
        </div>
      </div>

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button
            disabled={loading || !form.customerId || !form.productId}
            onClick={() => onSubmit({
              customerId: form.customerId,
              productId: form.productId,
              customerName: form.customerName || String(selectedCustomer?.name ?? ""),
              productName: form.productName || String(selectedProduct?.name ?? ""),
              startsAt: form.startsAt || null,
              expiresAt: form.expiresAt || null,
              maxActivations: form.maxActivations ? Number(form.maxActivations) : null,
              licenseType: form.licenseType,
              status: form.status,
              features: form.features
                ? form.features.split(",").map((f) => f.trim()).filter(Boolean)
                : [],
            })}
          >
            {loading ? "Saving..." : isCreate ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function LicenseSalesSection() {
  const { data: licenseList, mutate } = useLicenses();
  const { data: customerList } = useCustomers();
  const { data: productList } = useProducts();

  const rows = (licenseList ?? []) as Row[];
  const customers = (customerList ?? []) as Row[];
  const products = (productList ?? []) as Row[];

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
    setSaving(true);
    const result = mode === "create"
      ? await createLicense(payload)
      : await updateLicense(String(selected?.key), payload);
    setSaving(false);
    if (!result.success) return toast.error(result.error ?? "Failed to save license sale");
    toast.success(mode === "create" ? "License created" : "License updated");
    setDialogOpen(false);
    mutate();
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await deleteLicense(String(deleteTarget.key));
    setDeleting(false);
    if (!result.success) return toast.error(result.error ?? "Failed to delete license");
    toast.success("License deleted");
    setDeleteTarget(null);
    mutate();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">License Sales</h2>
        <Button onClick={openCreate}><Plus className="w-4 h-4 mr-2" />Create</Button>
      </div>

      <div className="rounded-lg border border-border overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>License Key</TableHead>
              <TableHead>Customer</TableHead>
              <TableHead>Product</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Expires</TableHead>
              <TableHead className="w-[52px]" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={String(row.key)} className="cursor-pointer hover:bg-secondary/30" onClick={() => openDialog(row, "view")}>
                <TableCell className="font-mono text-xs">{String(row.key)}</TableCell>
                <TableCell>{String(row.customerName ?? row.customer_name ?? "—")}</TableCell>
                <TableCell>{String(row.productName ?? row.product_name ?? "—")}</TableCell>
                <TableCell className="capitalize">{String(row.status ?? "active")}</TableCell>
                <TableCell>{String(row.expiresAt ?? row.expires_at ?? "—")}</TableCell>
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
          <LicenseForm mode={mode} row={selected ?? undefined} customers={customers} products={products} loading={saving} onCancel={() => setDialogOpen(false)} onSubmit={submit} />
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
        title="Delete license"
        description="This permanently deletes the selected license."
        onConfirm={confirmDelete}
        loading={deleting}
      />
    </div>
  );
}
