"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { useTaxRules, createTaxRule, updateTaxRule, deleteTaxRule } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type TaxRule = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const statusConfig: Record<string, { label: string; className: string }> = {
  active: { label: "Active", className: "bg-emerald-500/20 text-emerald-400" },
  inactive: { label: "Inactive", className: "bg-muted-foreground/20 text-muted-foreground" },
};

function TaxRuleForm({ taxRule, mode, onSubmit, onCancel, loading }: {
  taxRule?: TaxRule;
  mode: DialogMode;
  onSubmit: (data: Record<string, unknown>) => void;
  onCancel: () => void;
  loading: boolean;
}) {
  const isView = mode === "view";
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const inputClass = "w-full h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-colors disabled:opacity-50";

  const [form, setForm] = useState({
    country: (taxRule?.country as string) ?? "",
    region: (taxRule?.region as string) ?? "",
    taxName: (taxRule?.taxName as string) ?? "",
    rate: (taxRule?.rate as number) ?? 0,
    inclusive: (taxRule?.inclusive as boolean) ?? false,
    active: (taxRule?.active as boolean) ?? true,
  });

  return (
    <div>
      <DialogHeader>
        <DialogTitle>{mode === "create" ? "Create Tax Rule" : mode === "edit" ? "Edit Tax Rule" : "Tax Rule Details"}</DialogTitle>
        <DialogDescription>{mode === "create" ? "Create a new tax rule" : mode === "edit" ? "Update tax rule details" : "View tax rule configuration"}</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-4">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Country (2-letter code)</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.country}</p> : (
              <input className={inputClass} value={form.country} onChange={(e) => setForm({ ...form, country: e.target.value.toUpperCase() })} placeholder="US" maxLength={2} />
            )}
          </div>
          <div>
            <label className={labelClass}>Region (optional)</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.region || "—"}</p> : (
              <input className={inputClass} value={form.region} onChange={(e) => setForm({ ...form, region: e.target.value })} placeholder="CA" />
            )}
          </div>
        </div>

        <div>
          <label className={labelClass}>Tax Name</label>
          {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.taxName}</p> : (
            <input className={inputClass} value={form.taxName} onChange={(e) => setForm({ ...form, taxName: e.target.value })} placeholder="VAT" />
          )}
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className={labelClass}>Rate (decimal, e.g. 0.10 = 10%)</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{(form.rate * 100).toFixed(2)}%</p> : (
              <input type="number" step="0.001" className={inputClass} value={form.rate} onChange={(e) => setForm({ ...form, rate: Number(e.target.value) })} placeholder="0.10" />
            )}
          </div>
          <div>
            <label className={labelClass}>Inclusive</label>
            {isView ? <p className="text-sm font-medium text-foreground mt-0.5">{form.inclusive ? "Yes" : "No"}</p> : (
              <div className="flex items-center gap-2 h-9">
                <input
                  type="checkbox"
                  checked={form.inclusive}
                  onChange={(e) => setForm({ ...form, inclusive: e.target.checked })}
                  className="h-4 w-4 rounded border-border text-accent focus:ring-accent"
                />
                <span className="text-sm text-foreground">Tax is included in price</span>
              </div>
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
            country: form.country,
            region: form.region || null,
            taxName: form.taxName,
            rate: form.rate,
            inclusive: form.inclusive,
            active: form.active,
          })}>
            {loading ? "Saving..." : mode === "create" ? "Create" : "Save"}
          </Button>
        </DialogFooter>
      )}
    </div>
  );
}

export function TaxRulesManagement() {
  const { data: taxRules, isLoading, mutate } = useTaxRules();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<TaxRule | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<TaxRule | null>(null);
  const [saving, setSaving] = useState(false);

  const filtered = (taxRules || []).filter((r: TaxRule) =>
    (r.country as string).toLowerCase().includes(search.toLowerCase()) ||
    (r.taxName as string).toLowerCase().includes(search.toLowerCase()) ||
    ((r.region as string) ?? "").toLowerCase().includes(search.toLowerCase())
  );

  const openDialog = (rule: TaxRule | null, mode: DialogMode) => {
    setSelected(rule);
    setDialogMode(mode);
    setDialogOpen(true);
  };

  const handleSubmit = async (data: Record<string, unknown>) => {
    setSaving(true);
    if (dialogMode === "create") {
      const result = await createTaxRule(data);
      if (result.success) {
        toast.success("Tax rule created");
        setDialogOpen(false);
        mutate();
      }
    } else {
      const result = await updateTaxRule(selected!.id as string, data);
      if (result.success) {
        toast.success("Tax rule updated");
        setDialogOpen(false);
        mutate();
      }
    }
    setSaving(false);
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    const result = await deleteTaxRule(deleteTarget.id as string);
    if (result.success) {
      toast.success("Tax rule deleted");
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
            placeholder="Search tax rules..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20"
          />
        </div>
        <Button size="sm" onClick={() => openDialog(null, "create")}>
          <Plus className="w-4 h-4 mr-1" /> New Tax Rule
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Country</TableHead>
              <TableHead>Region</TableHead>
              <TableHead>Tax Name</TableHead>
              <TableHead className="text-right">Rate</TableHead>
              <TableHead>Inclusive</TableHead>
              <TableHead>Status</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={7} className="text-center py-8 text-muted-foreground">No tax rules found</TableCell>
              </TableRow>
            ) : filtered.map((rule: TaxRule) => {
              const status = (rule.active as boolean) ? "active" : "inactive";
              const config = statusConfig[status];
              return (
                <TableRow
                  key={rule.id as string}
                  className="cursor-pointer hover:bg-secondary/30"
                  onClick={() => openDialog(rule, "view")}
                >
                  <TableCell className="font-medium font-mono">{rule.country as string}</TableCell>
                  <TableCell className="text-muted-foreground">{(rule.region as string) || "—"}</TableCell>
                  <TableCell>{rule.taxName as string}</TableCell>
                  <TableCell className="text-right font-medium">{((rule.rate as number) * 100).toFixed(2)}%</TableCell>
                  <TableCell>
                    <span className={cn(
                      "px-2 py-0.5 rounded-full text-xs font-medium",
                      (rule.inclusive as boolean)
                        ? "bg-blue-500/20 text-blue-400"
                        : "bg-muted-foreground/20 text-muted-foreground"
                    )}>
                      {(rule.inclusive as boolean) ? "Yes" : "No"}
                    </span>
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
                        <DropdownMenuItem onClick={(e) => { e.stopPropagation(); openDialog(rule, "edit"); }}>
                          <Pencil className="w-4 h-4 mr-2" /> Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem className="text-destructive" onClick={(e) => { e.stopPropagation(); setDeleteTarget(rule); }}>
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

      {/* Tax Rule dialog */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-lg">
          <TaxRuleForm
            taxRule={selected ?? undefined}
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
        title="Delete Tax Rule"
        description={`Are you sure you want to delete the tax rule "${deleteTarget?.taxName}" for ${deleteTarget?.country}? This cannot be undone.`}
      />
    </div>
  );
}
