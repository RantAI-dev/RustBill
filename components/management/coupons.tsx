"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { useCoupons, createCoupon, updateCoupon, deleteCoupon } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Coupon = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const discountTypeLabels: Record<string, string> = {
  percentage: "Percentage",
  fixed_amount: "Fixed Amount",
};

function getStatus(coupon: Coupon): "active" | "inactive" | "expired" {
  if (coupon.validUntil && new Date(coupon.validUntil as string) < new Date()) return "expired";
  if (coupon.active) return "active";
  return "inactive";
}

const statusConfig: Record<string, { label: string; className: string }> = {
  active: { label: "Active", className: "bg-emerald-500/20 text-emerald-400" },
  inactive: { label: "Inactive", className: "bg-muted-foreground/20 text-muted-foreground" },
  expired: { label: "Expired", className: "bg-red-500/20 text-red-400" },
};

function CouponForm({ coupon, mode, onSubmit, onCancel, loading }: {
  coupon?: Coupon;
  mode: DialogMode;
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const isView = mode === "view";
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors disabled:opacity-50";

  const [form, setForm] = useState({
    code: (coupon?.code as string) ?? "",
    name: (coupon?.name as string) ?? "",
    discountType: (coupon?.discountType as string) ?? "percentage",
    discountValue: (coupon?.discountValue as number) ?? 0,
    maxRedemptions: (coupon?.maxRedemptions as number | null) ?? null,
    validUntil: (coupon?.validUntil as string)?.slice(0, 10) ?? "",
    active: (coupon?.active as boolean) ?? true,
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{mode === "create" ? "Create Coupon" : mode === "edit" ? "Edit Coupon" : "Coupon Details"}</DialogTitle>
        <DialogDescription>{mode === "create" ? "Create a new discount coupon" : mode === "edit" ? "Update coupon details" : "View coupon configuration"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div>
          <label className={labelClass}>Code</label>
          {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.code}</p> : (
            <input className={inputClass} value={form.code} onChange={(e) => setForm({ ...form, code: e.target.value.toUpperCase() })} placeholder="SAVE20" maxLength={50} />
          )}
        </div>

        <div>
          <label className={labelClass}>Name</label>
          {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.name}</p> : (
            <input className={inputClass} value={form.name} onChange={(e) => setForm({ ...form, name: e.target.value })} placeholder="20% Off Annual Plans" />
          )}
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Discount Type</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{discountTypeLabels[form.discountType]}</p> : (
              <select className={inputClass} value={form.discountType} onChange={(e) => setForm({ ...form, discountType: e.target.value })}>
                <option value="percentage">Percentage</option>
                <option value="fixed_amount">Fixed Amount</option>
              </select>
            )}
          </div>
          <div>
            <label className={labelClass}>Discount Value {form.discountType === "percentage" ? "(%)" : "($)"}</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.discountType === "percentage" ? `${form.discountValue}%` : `$${form.discountValue}`}</p> : (
              <input type="number" className={inputClass} value={form.discountValue} onChange={(e) => setForm({ ...form, discountValue: Number(e.target.value) })} />
            )}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Max Redemptions</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.maxRedemptions ?? "Unlimited"}</p> : (
              <input type="number" className={inputClass} value={form.maxRedemptions ?? ""} onChange={(e) => setForm({ ...form, maxRedemptions: e.target.value ? Number(e.target.value) : null })} placeholder="Unlimited" />
            )}
          </div>
          <div>
            <label className={labelClass}>Valid Until</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.validUntil || "No expiry"}</p> : (
              <input type="date" className={inputClass} value={form.validUntil} onChange={(e) => setForm({ ...form, validUntil: e.target.value })} />
            )}
          </div>
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

      {!isView && (
        <DialogFooter className="mt-6">
          <Button variant="outline" onClick={onCancel}>Cancel</Button>
          <Button disabled={loading} onClick={() => onSubmit({
            ...form,
            maxRedemptions: form.maxRedemptions ?? null,
            validUntil: form.validUntil || null,
          })}>
            {loading ? "Saving..." : mode === "create" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function ManageCouponsSection() {
  const { data: coupons, isLoading, mutate } = useCoupons();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Coupon | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Coupon | null>(null);
  const [saving, setSaving] = useState(false);

  const filtered = (coupons || []).filter((c: Coupon) =>
    (c.code as string).toLowerCase().includes(search.toLowerCase()) ||
    (c.name as string).toLowerCase().includes(search.toLowerCase())
  );

  const openDialog = (coupon: Coupon | null, mode: DialogMode) => {
    setSelected(coupon);
    setDialogMode(mode);
    setDialogOpen(true);
  };

  const handleSubmit = async (data: Record<string, unknown>) => {
    setSaving(true);
    if (dialogMode === "create") {
      const result = await createCoupon(data);
      if (result.success) {
        toast.success("Coupon created");
        setDialogOpen(false);
        mutate();
      }
    } else {
      const result = await updateCoupon(selected!.id as string, data);
      if (result.success) {
        toast.success("Coupon updated");
        setDialogOpen(false);
        mutate();
      }
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const result = await deleteCoupon(deleteTarget.id as string);
    if (result.success) {
      toast.success("Coupon deleted");
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
            placeholder="Search coupons..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20"
          />
        </div>
        <Button size="sm" onClick={() => openDialog(null, "create")}>
          <Plus className="w-4 h-4 mr-1" /> New Coupon
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Code</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Type</TableHead>
              <TableHead className="text-right">Value</TableHead>
              <TableHead>Redemptions</TableHead>
              <TableHead>Valid Until</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-8 text-muted-foreground">No coupons found</TableCell>
              </TableRow>
            ) : filtered.map((coupon: Coupon) => {
              const status = getStatus(coupon);
              const config = statusConfig[status];
              return (
                <TableRow
                  key={coupon.id as string}
                  className="cursor-pointer hover:bg-secondary/30"
                  onClick={() => openDialog(coupon, "view")}
                >
                  <TableCell className="font-medium font-mono">{coupon.code as string}</TableCell>
                  <TableCell className="text-muted-foreground">{coupon.name as string}</TableCell>
                  <TableCell>{(coupon.discountType as string) === "percentage" ? "%" : "$"}</TableCell>
                  <TableCell className="text-right font-medium">
                    {(coupon.discountType as string) === "percentage"
                      ? `${coupon.discountValue as number}%`
                      : `$${(coupon.discountValue as number).toLocaleString()}`}
                  </TableCell>
                  <TableCell>
                    {coupon.timesRedeemed as number}/{coupon.maxRedemptions != null ? coupon.maxRedemptions as number : "\u221E"}
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {coupon.validUntil ? new Date(coupon.validUntil as string).toLocaleDateString() : "No expiry"}
                  </TableCell>
                  <TableCell>
                    <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium", config.className)}>
                      {config.label}
                    </span>
                  </TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild onClick={(e) => e.stopPropagation()}>
                        <Button variant="ghost" size="icon" className="h-8 w-8"><MoreHorizontal className="w-4 h-4" /></Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openDialog(coupon, "edit"); }}>
                          <Pencil className="w-4 h-4 mr-2" /> Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem className="text-destructive" onClick={(e) => { e.stopPropagation(); setDeleteTarget(coupon); }}>
                          <Trash2 className="w-4 h-4 mr-2" /> Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </div>

      {/* Coupon dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <CouponForm
            coupon={selected ?? undefined}
            mode={dialogMode}
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
        title="Delete Coupon"
        description={`Are you sure you want to delete "${deleteTarget?.code}"? This will also remove it from any active subscriptions.`}
      />
    </div>
  );
}
