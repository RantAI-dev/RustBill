"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { useSubscriptions, useCustomers, usePlans, createSubscription, updateSubscription, deleteSubscription } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Sub = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const statusColors: Record<string, string> = {
  active: "bg-sky-500/20 text-sky-400",
  paused: "bg-amber-500/20 text-amber-400",
  canceled: "bg-zinc-500/20 text-zinc-400",
  past_due: "bg-red-500/20 text-red-400",
  trialing: "bg-blue-500/20 text-blue-400",
};

function SubForm({ sub, mode, customers, plans, onSubmit, onCancel, loading }: {
  sub?: Sub;
  mode: DialogMode;
  customers: Sub[];
  plans: Sub[];
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const isView = mode === "view";
  const isCreate = mode === "create";
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors disabled:opacity-50";

  const now = new Date();

  const [form, setForm] = useState({
    customerId: (sub?.customerId as string) ?? "",
    planId: (sub?.planId as string) ?? "",
    status: (sub?.status as string) ?? "active",
    quantity: (sub?.quantity as number) ?? 1,
    currentPeriodStart: sub?.currentPeriodStart
      ? new Date(sub.currentPeriodStart as string).toISOString().split("T")[0]
      : now.toISOString().split("T")[0],
    currentPeriodEnd: sub?.currentPeriodEnd
      ? new Date(sub.currentPeriodEnd as string).toISOString().split("T")[0]
      : "",
  });

  // Auto-compute period end and status when plan changes (create mode only)
  const handlePlanChange = (planId: string) => {
    const selectedPlan = plans.find((p) => (p.id as string) === planId);
    if (selectedPlan && isCreate) {
      const start = new Date(form.currentPeriodStart || now.toISOString().split("T")[0]);
      const end = new Date(start);
      const billingCycle = selectedPlan.billingCycle as string;

      switch (billingCycle) {
        case "monthly":
          end.setMonth(end.getMonth() + 1);
          break;
        case "quarterly":
          end.setMonth(end.getMonth() + 3);
          break;
        case "yearly":
          end.setFullYear(end.getFullYear() + 1);
          break;
      }

      const trialDays = (selectedPlan.trialDays as number) ?? 0;
      const newStatus = trialDays > 0 ? "trialing" : "active";

      setForm({
        ...form,
        planId,
        currentPeriodEnd: end.toISOString().split("T")[0],
        status: newStatus,
      });
    } else {
      setForm({ ...form, planId });
    }
  };

  // Also recompute when period start changes
  const handlePeriodStartChange = (dateStr: string) => {
    const selectedPlan = plans.find((p) => (p.id as string) === form.planId);
    if (selectedPlan && isCreate) {
      const start = new Date(dateStr);
      const end = new Date(start);
      const billingCycle = selectedPlan.billingCycle as string;

      switch (billingCycle) {
        case "monthly":
          end.setMonth(end.getMonth() + 1);
          break;
        case "quarterly":
          end.setMonth(end.getMonth() + 3);
          break;
        case "yearly":
          end.setFullYear(end.getFullYear() + 1);
          break;
      }

      setForm({
        ...form,
        currentPeriodStart: dateStr,
        currentPeriodEnd: end.toISOString().split("T")[0],
      });
    } else {
      setForm({ ...form, currentPeriodStart: dateStr });
    }
  };

  // For create: find selected plan to display info
  const selectedPlan = plans.find((p) => (p.id as string) === form.planId);
  const trialDays = selectedPlan ? ((selectedPlan.trialDays as number) ?? 0) : 0;

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{isCreate ? "Create Subscription" : mode === "edit" ? "Edit Subscription" : "Subscription Details"}</DialogTitle>
        <DialogDescription>{isCreate ? "Create a new subscription" : mode === "edit" ? "Update subscription" : "View subscription details"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Customer</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{(sub?.customerName as string) ?? "—"}</p> : (
              <select className={inputClass} value={form.customerId} onChange={(e) => setForm({ ...form, customerId: e.target.value })} disabled={!isCreate}>
                <option value="">Select customer</option>
                {customers.map((c) => <option key={c.id as string} value={c.id as string}>{c.name as string}</option>)}
              </select>
            )}
          </div>
          <div>
            <label className={labelClass}>Plan</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{(sub?.planName as string) ?? "—"}</p> : (
              <select className={inputClass} value={form.planId} onChange={(e) => handlePlanChange(e.target.value)} disabled={!isCreate}>
                <option value="">Select plan</option>
                {plans.filter((p) => p.active).map((p) => <option key={p.id as string} value={p.id as string}>{p.name as string} — ${(p.basePrice as number).toLocaleString()}/{p.billingCycle as string}</option>)}
              </select>
            )}
          </div>
        </div>

        {/* Show auto-computed info for create mode */}
        {isCreate && selectedPlan && (
          <div className="rounded-lg bg-secondary/50 border border-border p-3 space-y-1">
            <p className="text-xs text-muted-foreground">
              Period end auto-computed from <span className="font-medium text-foreground">{selectedPlan.billingCycle as string}</span> billing cycle.
              {trialDays > 0 && <> Status set to <span className="font-medium text-blue-400">trialing</span> ({trialDays} day trial).</>}
            </p>
          </div>
        )}

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Status</label>
            {isView ? (
              <p className="mt-0.5">
                <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[form.status])}>
                  {form.status.replace("_", " ")}
                </span>
              </p>
            ) : (
              <select className={inputClass} value={form.status} onChange={(e) => setForm({ ...form, status: e.target.value })} disabled={isCreate}>
                <option value="active">Active</option>
                <option value="paused">Paused</option>
                <option value="canceled">Canceled</option>
                <option value="past_due">Past Due</option>
                <option value="trialing">Trialing</option>
              </select>
            )}
          </div>
          <div>
            <label className={labelClass}>Quantity</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.quantity}</p> : (
              <input type="number" min={1} className={inputClass} value={form.quantity} onChange={(e) => setForm({ ...form, quantity: Number(e.target.value) })} />
            )}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Period Start</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.currentPeriodStart}</p> : (
              <input type="date" className={inputClass} value={form.currentPeriodStart} onChange={(e) => handlePeriodStartChange(e.target.value)} />
            )}
          </div>
          <div>
            <label className={labelClass}>Period End {isCreate && "(auto)"}</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.currentPeriodEnd}</p> : (
              <input type="date" className={inputClass} value={form.currentPeriodEnd} onChange={(e) => setForm({ ...form, currentPeriodEnd: e.target.value })} disabled={isCreate} />
            )}
          </div>
        </div>
      </div>

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button disabled={loading} onClick={() => onSubmit(form)}>
            {loading ? "Saving..." : isCreate ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function ManageSubscriptionsSection() {
  const { data: subs, isLoading, mutate } = useSubscriptions();
  const { data: customerList } = useCustomers();
  const { data: planList } = usePlans();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Sub | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Sub | null>(null);
  const [saving, setSaving] = useState(false);

  const filtered = (subs || []).filter((s: Sub) =>
    ((s.customerName as string) ?? "").toLowerCase().includes(search.toLowerCase()) ||
    ((s.planName as string) ?? "").toLowerCase().includes(search.toLowerCase())
  );

  const openDialog = (sub: Sub | null, mode: DialogMode) => {
    setSelected(sub);
    setDialogMode(mode);
    setDialogOpen(true);
  };

  const handleSubmit = async (data: Record<string, unknown>) => {
    setSaving(true);
    if (dialogMode === "create") {
      const result = await createSubscription(data);
      if (result.success) {
        toast.success("Subscription created");
        setDialogOpen(false);
        mutate();
      }
    } else {
      const result = await updateSubscription(selected!.id as string, data);
      if (result.success) {
        toast.success("Subscription updated");
        setDialogOpen(false);
        mutate();
      }
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const result = await deleteSubscription(deleteTarget.id as string);
    if (result.success) {
      toast.success("Subscription deleted");
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
            placeholder="Search subscriptions..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20"
          />
        </div>
        <Button size="sm" onClick={() => openDialog(null, "create")}>
          <Plus className="w-4 h-4 mr-1" /> New Subscription
        </Button>
      </div>

      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Customer</TableHead>
              <TableHead>Plan</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Qty</TableHead>
              <TableHead>Period</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">No subscriptions found</TableCell>
              </TableRow>
            ) : filtered.map((sub: Sub) => (
              <TableRow
                key={sub.id as string}
                className="cursor-pointer hover:bg-secondary/30"
                onClick={() => openDialog(sub, "view")}
              >
                <TableCell className="font-medium">{(sub.customerName as string) ?? "—"}</TableCell>
                <TableCell className="text-muted-foreground">{(sub.planName as string) ?? "—"}</TableCell>
                <TableCell>
                  <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", statusColors[(sub.status as string)] ?? "bg-secondary text-muted-foreground")}>
                    {(sub.status as string).replace("_", " ")}
                  </span>
                </TableCell>
                <TableCell>{sub.quantity as number}</TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {new Date(sub.currentPeriodStart as string).toLocaleDateString()} — {new Date(sub.currentPeriodEnd as string).toLocaleDateString()}
                </TableCell>
                <TableCell>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                      <Button variant="ghost" size="icon" className="h-8 w-8"><MoreHorizontal className="w-4 h-4" /></Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openDialog(sub, "edit"); }}>
                        <Pencil className="w-4 h-4 mr-2" /> Edit
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem className="text-destructive" onClick={(e) => { e.stopPropagation(); setDeleteTarget(sub); }}>
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

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <SubForm
            sub={selected ?? undefined}
            mode={dialogMode}
            customers={customerList || []}
            plans={planList || []}
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

      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        onConfirm={handleDelete}
        title="Delete Subscription"
        description={`Are you sure you want to delete this subscription for "${deleteTarget?.customerName}"?`}
      />
    </div>
  );
}
