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
import { createLicense, createUsageEvent, deleteUsageEvent, updateUsageEvent, usePlans, useProducts, useSubscriptions, useUsageEvents } from "@/hooks/use-api";

type Row = Record<string, unknown>;
type DialogMode = "create" | "view" | "edit";

function toNumber(value: string): number {
  const n = Number(value);
  return Number.isFinite(n) ? n : 0;
}

function UsageForm({
  mode,
  row,
  subscriptions,
  products,
  loading,
  onCancel,
  onSubmit,
}: {
  mode: DialogMode;
  row?: Row;
  subscriptions: Row[];
  products: Row[];
  loading: boolean;
  onCancel: () => void;
  onSubmit: (payload: Record<string, unknown>) => void;
}) {
  const isView = mode === "view";
  const isCreate = mode === "create";

  const [form, setForm] = useState({
    subscriptionId: String(row?.subscription_id ?? row?.subscriptionId ?? ""),
    metricName: String(row?.metric_name ?? row?.metricName ?? "api_calls"),
    value: String(row?.value ?? 1),
    timestamp: String(row?.timestamp ?? "").slice(0, 16),
    createLicense: false,
    licenseProductId: "",
    licenseStartsAt: "",
    licenseExpiresAt: "",
    licenseMaxActivations: "",
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Record Usage Event" : mode === "edit" ? "Edit Usage Event" : "Usage Event Details"}</DialogTitle>
        <DialogDescription>
          {isCreate ? "Record metered usage for a subscription sale." : mode === "edit" ? "Update usage event fields." : "View usage event details."}
        </DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-1.5">
            <Label>Subscription</Label>
            {isView ? (
              <p className="text-sm">{form.subscriptionId || "—"}</p>
            ) : (
              <select className="w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm" value={form.subscriptionId} onChange={(e) => setForm((p) => ({ ...p, subscriptionId: e.target.value }))} disabled={!isCreate}>
                <option value="">Select subscription</option>
                {subscriptions.map((s) => (
                  <option key={String(s.id)} value={String(s.id)}>
                    {String(s.customerName ?? s.customer_name ?? "Customer")} - {String(s.planName ?? s.plan_name ?? "Plan")}
                  </option>
                ))}
              </select>
            )}
          </div>
          <div className="space-y-1.5">
            <Label>Metric Name</Label>
            {isView ? <p className="text-sm">{form.metricName}</p> : <Input value={form.metricName} onChange={(e) => setForm((p) => ({ ...p, metricName: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Value</Label>
            {isView ? <p className="text-sm">{Number(form.value).toLocaleString()}</p> : <Input type="number" min="0" step="1" value={form.value} onChange={(e) => setForm((p) => ({ ...p, value: e.target.value }))} />}
          </div>
          <div className="space-y-1.5">
            <Label>Timestamp</Label>
            {isView ? <p className="text-sm">{form.timestamp || "—"}</p> : <Input type="datetime-local" value={form.timestamp} onChange={(e) => setForm((p) => ({ ...p, timestamp: e.target.value }))} />}
          </div>
          {!isView && (
            <div className="md:col-span-2 space-y-3 rounded-lg border border-border bg-secondary/30 p-3">
              <label className="inline-flex items-center gap-2 text-sm">
                <input type="checkbox" checked={form.createLicense} onChange={(e) => setForm((p) => ({ ...p, createLicense: e.target.checked }))} />
                Generate license for this usage sale
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
            disabled={loading || !form.subscriptionId || toNumber(form.value) <= 0 || (form.createLicense && !form.licenseProductId)}
            onClick={() =>
              onSubmit({
                subscriptionId: form.subscriptionId,
                metricName: form.metricName,
                value: toNumber(form.value),
                timestamp: form.timestamp ? new Date(form.timestamp).toISOString() : undefined,
                idempotencyKey: isCreate ? `usage-${Date.now()}` : undefined,
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

export function UsageSalesSection() {
  const { data: allSubscriptions } = useSubscriptions();
  const { data: planList } = usePlans();
  const { data: productList } = useProducts();
  const subscriptions = useMemo(
    () => (allSubscriptions ?? []) as Row[],
    [allSubscriptions],
  );
  const plans = (planList ?? []) as Row[];
  const products = (productList ?? []) as Row[];

  const [activeSubscriptionId, setActiveSubscriptionId] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [mode, setMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Row | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Row | null>(null);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    if (!activeSubscriptionId && subscriptions.length > 0) {
      setActiveSubscriptionId(String(subscriptions[0].id));
    }
  }, [activeSubscriptionId, subscriptions]);

  const { data: eventList, mutate } = useUsageEvents(activeSubscriptionId || "");
  const rows = (eventList ?? []) as Row[];

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
      ...usagePayload
    } = payload;

    setSaving(true);
    const result = mode === "create"
      ? await createUsageEvent(usagePayload)
      : await updateUsageEvent(String(selected?.id), usagePayload);

    if (result.success && shouldCreateLicense) {
      const subId = String(usagePayload.subscriptionId ?? "");
      const sub = subscriptions.find((s) => String(s.id) === subId);
      const customerId = String(sub?.customerId ?? sub?.customer_id ?? "");
      const customerName = String(sub?.customerName ?? sub?.customer_name ?? "");
      const planId = String(sub?.planId ?? sub?.plan_id ?? "");
      const plan = plans.find((p) => String(p.id) === planId);
      const fallbackProductId = String(plan?.productId ?? plan?.product_id ?? "");
      const productId = String(licenseProductId ?? fallbackProductId);
      const product = products.find((p) => String(p.id) === productId);

      const licenseResult = await createLicense({
        customerId,
        customerName,
        productId,
        productName: String(product?.name ?? ""),
        startsAt: licenseStartsAt || null,
        expiresAt: licenseExpiresAt || null,
        maxActivations: licenseMaxActivations ?? null,
        licenseType: "simple",
      });
      if (!licenseResult.success) {
        setSaving(false);
        return toast.error(licenseResult.error ?? "Usage saved, but failed to generate license");
      }
    }

    setSaving(false);
    if (!result.success) return toast.error(result.error ?? "Failed to save usage event");
    toast.success(mode === "create" ? "Usage event recorded" : "Usage event updated");
    setDialogOpen(false);
    mutate();
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setDeleting(true);
    const result = await deleteUsageEvent(String(deleteTarget.id));
    setDeleting(false);
    if (!result.success) return toast.error(result.error ?? "Failed to delete usage event");
    toast.success("Usage event deleted");
    setDeleteTarget(null);
    mutate();
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <h2 className="text-lg font-semibold">Usage Event Sales</h2>
        <div className="flex items-center gap-2">
          <select className="h-9 rounded-lg border border-border bg-secondary px-3 text-sm" value={activeSubscriptionId} onChange={(e) => setActiveSubscriptionId(e.target.value)}>
            <option value="">Select subscription</option>
            {subscriptions.map((s) => (
              <option key={String(s.id)} value={String(s.id)}>
                {String(s.customerName ?? s.customer_name ?? "Customer")} - {String(s.planName ?? s.plan_name ?? "Plan")}
              </option>
            ))}
          </select>
          <Button onClick={openCreate}><Plus className="w-4 h-4 mr-2" />Create</Button>
        </div>
      </div>

      <div className="rounded-lg border border-border overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Timestamp</TableHead>
              <TableHead>Metric</TableHead>
              <TableHead className="text-right">Value</TableHead>
              <TableHead>Idempotency</TableHead>
              <TableHead className="w-[52px]" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={String(row.id)} className="cursor-pointer hover:bg-secondary/30" onClick={() => openDialog(row, "view")}>
                <TableCell>{new Date(String(row.timestamp ?? Date.now())).toLocaleString()}</TableCell>
                <TableCell>{String(row.metric_name ?? row.metricName ?? "—")}</TableCell>
                <TableCell className="text-right">{Number(row.value ?? 0).toLocaleString()}</TableCell>
                <TableCell className="font-mono text-xs">{String(row.idempotency_key ?? row.idempotencyKey ?? "—")}</TableCell>
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
          <UsageForm mode={mode} row={selected ?? undefined} subscriptions={subscriptions} products={products} loading={saving} onCancel={() => setDialogOpen(false)} onSubmit={submit} />
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
        title="Delete usage event"
        description="This permanently removes the selected usage event."
        onConfirm={confirmDelete}
        loading={deleting}
      />
    </div>
  );
}
