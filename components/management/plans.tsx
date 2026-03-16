"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { usePlans, useProducts, createPlan, updatePlan, deletePlan } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Plan = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const pricingModelLabels: Record<string, string> = {
  flat: "Flat Rate",
  per_unit: "Per Unit",
  tiered: "Tiered",
  usage_based: "Usage Based",
};

const billingCycleLabels: Record<string, string> = {
  monthly: "Monthly",
  quarterly: "Quarterly",
  yearly: "Yearly",
};

function PlanForm({ plan, mode, products, onSubmit, onCancel, loading }: {
  plan?: Plan;
  mode: DialogMode;
  products: Plan[];
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const isView = mode === "view";
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors disabled:opacity-50";

  const [form, setForm] = useState({
    name: (plan?.name as string) ?? "",
    productId: (plan?.productId as string) ?? "",
    pricingModel: (plan?.pricingModel as string) ?? "flat",
    billingCycle: (plan?.billingCycle as string) ?? "monthly",
    basePrice: (plan?.basePrice as number) ?? 0,
    unitPrice: (plan?.unitPrice as number) ?? 0,
    usageMetricName: (plan?.usageMetricName as string) ?? "",
    trialDays: (plan?.trialDays as number) ?? 0,
    active: (plan?.active as boolean) ?? true,
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{mode === "create" ? "Create Plan" : mode === "edit" ? "Edit Plan" : "Plan Details"}</DialogTitle>
        <DialogDescription>{mode === "create" ? "Create a new pricing plan" : mode === "edit" ? "Update plan details" : "View plan configuration"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div>
          <label className={labelClass}>Name</label>
          {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.name}</p> : (
            <input className={inputClass} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="Enterprise Monthly" />
          )}
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Product</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{(plan?.productName as string) ?? "None"}</p> : (
              <select className={inputClass} value={form.productId} onChange={(e) => setForm({ ...form, productId: e.target.value })}>
                <option value="">None</option>
                {products.map((p) => (
                  <option key={p.id as string} value={p.id as string}>{p.name as string}</option>
                ))}
              </select>
            )}
          </div>
          <div>
            <label className={labelClass}>Billing Cycle</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{billingCycleLabels[form.billingCycle]}</p> : (
              <select className={inputClass} value={form.billingCycle} onChange={(e) => setForm({ ...form, billingCycle: e.target.value })}>
                <option value="monthly">Monthly</option>
                <option value="quarterly">Quarterly</option>
                <option value="yearly">Yearly</option>
              </select>
            )}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Pricing Model</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{pricingModelLabels[form.pricingModel]}</p> : (
              <select className={inputClass} value={form.pricingModel} onChange={(e) => setForm({ ...form, pricingModel: e.target.value })}>
                <option value="flat">Flat Rate</option>
                <option value="per_unit">Per Unit</option>
                <option value="tiered">Tiered</option>
                <option value="usage_based">Usage Based</option>
              </select>
            )}
          </div>
          <div>
            <label className={labelClass}>Base Price ($)</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">${form.basePrice.toLocaleString()}</p> : (
              <input type="number" className={inputClass} value={form.basePrice} onChange={(e) => setForm({ ...form, basePrice: Number(e.target.value) })} />
            )}
          </div>
        </div>

        {(form.pricingModel === "per_unit" || form.pricingModel === "usage_based") && (
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className={labelClass}>Unit Price ($)</label>
              {isView ? <p className="text-sm font-medium text-foreground mt-0.5">${form.unitPrice}</p> : (
                <input type="number" step="0.001" className={inputClass} value={form.unitPrice} onChange={(e) => setForm({ ...form, unitPrice: Number(e.target.value) })} />
              )}
            </div>
            {form.pricingModel === "usage_based" && (
              <div>
                <label className={labelClass}>Usage Metric Name</label>
                {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.usageMetricName || "—"}</p> : (
                  <input className={inputClass} value={form.usageMetricName} onChange={(e) => setForm({ ...form, usageMetricName: e.target.value })} placeholder="api_calls" />
                )}
              </div>
            )}
          </div>
        )}

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Trial Days</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.trialDays}</p> : (
              <input type="number" className={inputClass} value={form.trialDays} onChange={(e) => setForm({ ...form, trialDays: Number(e.target.value) })} />
            )}
          </div>
          <div>
            <label className={labelClass}>Status</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.active ? "Active" : "Inactive"}</p> : (
              <select className={inputClass} value={form.active ? "true" : "false"} onChange={(e) => setForm({ ...form, active: e.target.value === "true" })}>
                <option value="true">Active</option>
                <option value="false">Inactive</option>
              </select>
            )}
          </div>
        </div>
      </div>

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button disabled={loading} onClick={() => onSubmit({
            ...form,
            productId: form.productId || null,
            usageMetricName: form.usageMetricName || null,
          })}>
            {loading ? "Saving..." : mode === "create" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function ManagePlansSection() {
  const { data: plans, isLoading, mutate } = usePlans();
  const { data: productList } = useProducts();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Plan | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Plan | null>(null);
  const [saving, setSaving] = useState(false);

  const filtered = (plans || []).filter((p: Plan) =>
    (p.name as string).toLowerCase().includes(search.toLowerCase())
  );

  const openDialog = (plan: Plan | null, mode: DialogMode) => {
    setSelected(plan);
    setDialogMode(mode);
    setDialogOpen(true);
  };

  const handleSubmit = async (data: Record<string, unknown>) => {
    setSaving(true);
    if (dialogMode === "create") {
      const result = await createPlan(data);
      if (result.success) {
        toast.success("Plan created");
        setDialogOpen(false);
        mutate();
      }
    } else {
      const result = await updatePlan(selected!.id as string, data);
      if (result.success) {
        toast.success("Plan updated");
        setDialogOpen(false);
        mutate();
      }
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const result = await deletePlan(deleteTarget.id as string);
    if (result.success) {
      toast.success("Plan deleted");
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
      {/* Toolbar */}
      <div className="flex items-center justify-between gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input
            type="text"
            placeholder="Search plans..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20"
          />
        </div>
        <Button size="sm" onClick={() => openDialog(null, "create")}>
          <Plus className="w-4 h-4 mr-1" /> New Plan
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Name</TableHead>
              <TableHead>Product</TableHead>
              <TableHead>Model</TableHead>
              <TableHead>Cycle</TableHead>
              <TableHead className="text-right">Base Price</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">No plans found</TableCell>
              </TableRow>
            ) : filtered.map((plan: Plan) => (
              <TableRow
                key={plan.id as string}
                className="cursor-pointer hover:bg-secondary/30"
                onClick={() => openDialog(plan, "view")}
              >
                <TableCell className="font-medium">{plan.name as string}</TableCell>
                <TableCell className="text-muted-foreground">{(plan.productName as string) ?? "—"}</TableCell>
                <TableCell>{pricingModelLabels[(plan.pricingModel as string)] ?? plan.pricingModel as string}</TableCell>
                <TableCell>{billingCycleLabels[(plan.billingCycle as string)] ?? plan.billingCycle as string}</TableCell>
                <TableCell className="text-right font-medium">${(plan.basePrice as number).toLocaleString()}</TableCell>
                <TableCell>
                  <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium", plan.active ? "bg-emerald-500/20 text-emerald-400" : "bg-muted-foreground/20 text-muted-foreground")}>
                    {plan.active ? "Active" : "Inactive"}
                  </span>
                </TableCell>
                <TableCell>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <Button variant="ghost" size="icon" className="h-8 w-8"><MoreHorizontal className="w-4 h-4" /></Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openDialog(plan, "edit"); }}>
                        <Pencil className="w-4 h-4 mr-2" /> Edit
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem className="text-destructive" onClick={(e) => { e.stopPropagation(); setDeleteTarget(plan); }}>
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

      {/* Plan dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <PlanForm
            plan={selected ?? undefined}
            mode={dialogMode}
            products={productList || []}
            onSubmit={handleSubmit}
            onCancel={() => setDialogOpen(false)}
            loading={saving}
          />
          {dialogMode === "view" && (
            <DialogFooter>
              <Button variant="outline" onClick={() => setDialogMode("edit")}>
                <Pencil className="w-4 h-4 mr-1" /> Edit
              </Button>
            </DialogFooter>
          )}
        </DialogContent>
      </Dialog>

      {/* Delete dialog */}
      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        onConfirm={handleDelete}
        title="Delete Plan"
        description={`Are you sure you want to delete "${deleteTarget?.name}"? This will also remove associated subscriptions.`}
      />
    </div>
  );
}
