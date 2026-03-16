"use client";

import React, { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2, KeyRound, Copy, Check, Users, Zap, DollarSign, Calendar, Building2, ChevronsUpDown, Package, FlaskConical, Handshake } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useDeals, useCustomers, useProducts, createDeal, updateDeal, deleteDeal } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Deal = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";
type DealType = "sale" | "trial" | "partner";

const dealTypeBadgeConfig: Record<DealType, { label: string; className: string; icon: React.ElementType }> = {
  sale: { label: "Sale", className: "bg-accent/10 text-accent border-accent/20", icon: DollarSign },
  trial: { label: "Trial", className: "bg-chart-3/10 text-chart-3 border-chart-3/20", icon: FlaskConical },
  partner: { label: "Partner", className: "bg-chart-1/10 text-chart-1 border-chart-1/20", icon: Handshake },
};

function DealTypeBadge({ type }: { type: DealType }) {
  const config = dealTypeBadgeConfig[type];
  if (!config) return null;
  const Icon = config.icon;
  return (
    <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border", config.className)}>
      <Icon className="w-3 h-3" />
      {config.label}
    </span>
  );
}

/* ---------- detail view ---------- */

function DealDetail({ deal, onEdit, onDelete, onGenerateKey, onCopyKey, copiedKey }: {
  deal: Deal;
  onEdit: () => void;
  onDelete: () => void;
  onGenerateKey: (id: string) => void;
  onCopyKey: (key: string) => void;
  copiedKey: string | null;
}) {
  const productType = deal.productType as ProductType;
  const licenseKey = deal.licenseKey as string | null;
  const usageLabel = deal.usageMetricLabel as string | null;
  const usageValue = deal.usageMetricValue as number | null;

  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const valueClass = "text-sm font-medium text-foreground mt-0.5";

  return (
    <div>
      <DialogHeader>
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center text-sm font-semibold text-muted-foreground">
            {(deal.company as string).charAt(0)}
          </div>
          <div>
            <DialogTitle className="text-lg">{deal.company as string}</DialogTitle>
            <DialogDescription className="flex items-center gap-2 mt-0.5">
              <span>{deal.contact as string}</span>
              <span className="text-muted-foreground/50">&middot;</span>
              <span>{deal.email as string}</span>
            </DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <div className="mt-6 space-y-6">
        {/* Key metrics */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className={labelClass}>Value</p>
            <p className={cn(valueClass, "flex items-center gap-1.5 text-base")}>
              <DollarSign className="w-4 h-4 text-accent" />
              ${(deal.value as number).toLocaleString()}
            </p>
          </div>
          <div>
            <p className={labelClass}>Date</p>
            <p className={cn(valueClass, "flex items-center gap-1.5")}>
              <Calendar className="w-3.5 h-3.5 text-muted-foreground" />
              {deal.date as string}
            </p>
          </div>
        </div>

        {/* Deal Type & Product */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className={labelClass}>Deal Type</p>
            <div className="mt-1">
              <DealTypeBadge type={(deal.dealType as DealType) ?? "sale"} />
            </div>
          </div>
          <div>
            <p className={labelClass}>Product</p>
            <div className="flex items-center gap-2 mt-0.5">
              <span className="text-sm font-medium text-foreground">{deal.productName as string}</span>
              <ProductTypeBadge type={productType} />
            </div>
          </div>
        </div>

        {/* Notes (for trial/partner) */}
        {!!deal.notes && (
          <div>
            <p className={labelClass}>Notes</p>
            <p className={cn(valueClass, "whitespace-pre-wrap")}>{deal.notes as string}</p>
          </div>
        )}

        {/* License key or usage metric */}
        {productType === "licensed" && licenseKey && (
          <div>
            <p className={labelClass}>License Key</p>
            <div className="mt-1">
              <button onClick={() => onCopyKey(licenseKey)} className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-secondary hover:bg-secondary/80 text-xs font-mono text-muted-foreground hover:text-foreground transition-colors">
                <KeyRound className="w-3 h-3 shrink-0" />
                <span>{licenseKey}</span>
                {copiedKey === licenseKey ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
              </button>
            </div>
          </div>
        )}
        {usageLabel && (
          <div>
            <p className={labelClass}>{usageLabel}</p>
            <p className={cn(valueClass, "flex items-center gap-1.5")}>
              {productType === "saas" ? <Users className="w-3.5 h-3.5 text-chart-3" /> : <Zap className="w-3.5 h-3.5 text-chart-5" />}
              {formatUsageMetric(usageValue ?? 0)}
            </p>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-6 mt-6 border-t border-border">
        <Button variant="outline" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={onDelete}>
          <Trash2 className="w-4 h-4 mr-1.5" />
          Delete
        </Button>
        <div className="flex items-center gap-2">
          {productType === "licensed" && !licenseKey && (
            <Button variant="outline" size="sm" onClick={() => onGenerateKey(deal.id as string)}>
              <KeyRound className="w-4 h-4 mr-1.5" />
              Generate Key
            </Button>
          )}
          <Button size="sm" onClick={onEdit}>
            <Pencil className="w-4 h-4 mr-1.5" />
            Edit
          </Button>
        </div>
      </div>
    </div>
  );
}

/* ---------- deal form ---------- */

function DealForm({ deal, onClose, onSuccess }: { deal: Deal | null; onClose: () => void; onSuccess: () => void }) {
  const isEditing = !!deal;
  const { data: allCustomers } = useCustomers();
  const { data: allProducts } = useProducts();
  const customerList = (allCustomers ?? []) as Record<string, unknown>[];
  const productList = (allProducts ?? []) as Record<string, unknown>[];

  const [customerId, setCustomerId] = useState<string | null>((deal?.customerId as string) ?? null);
  const [productId, setProductId] = useState<string | null>((deal?.productId as string) ?? null);
  const [dealType, setDealType] = useState<DealType>((deal?.dealType as DealType) ?? "sale");
  const [value, setValue] = useState((deal?.value as number) ?? 0);
  const [date, setDate] = useState((deal?.date as string) ?? new Date().toISOString().split("T")[0]);
  const [notes, setNotes] = useState((deal?.notes as string) ?? "");
  const [licenseExpiresAt, setLicenseExpiresAt] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [customerOpen, setCustomerOpen] = useState(false);
  const [productOpen, setProductOpen] = useState(false);

  const selectedCustomer = customerList.find((c) => c.id === customerId);
  const selectedProduct = productList.find((p) => p.id === productId);

  const inputClass = "w-full h-9 mt-1 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent";

  const handleSubmit = async () => {
    if (!customerId) {
      toast.error("Customer is required");
      return;
    }
    if (!productId) {
      toast.error("Product is required");
      return;
    }
    setSubmitting(true);
    try {
      const data: Record<string, unknown> = { customerId, productId, dealType, value, date, notes: notes || null, company: "_", contact: "_", email: "x@x.com", productName: "_", productType: "licensed" as const };
      if (licenseExpiresAt) data.licenseExpiresAt = licenseExpiresAt;
      if (isEditing) {
        await updateDeal(deal.id as string, data);
        toast.success("Deal updated");
      } else {
        await createDeal(data);
        toast.success("Deal created");
      }
      onSuccess();
      onClose();
    } catch {
      toast.error(isEditing ? "Failed to update" : "Failed to create");
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="space-y-4">
      {/* Customer selector */}
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Customer</label>
        <Popover open={customerOpen} onOpenChange={setCustomerOpen}>
          <PopoverTrigger asChild>
            <Button variant="outline" role="combobox" aria-expanded={customerOpen} className="w-full justify-between mt-1 h-9 bg-secondary border-border text-sm font-normal">
              {selectedCustomer ? (
                <span className="flex items-center gap-2">
                  <Building2 className="w-3.5 h-3.5 text-muted-foreground" />
                  {selectedCustomer.name as string}
                </span>
              ) : (
                <span className="text-muted-foreground">Select customer...</span>
              )}
              <ChevronsUpDown className="ml-2 h-3.5 w-3.5 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-[--radix-popover-trigger-width] p-0" align="start">
            <Command>
              <CommandInput placeholder="Search customers..." />
              <CommandList>
                <CommandEmpty>No customers found.</CommandEmpty>
                <CommandGroup>
                  {customerList.map((c) => (
                    <CommandItem key={c.id as string} value={c.name as string} onSelect={() => { setCustomerId(c.id as string); setCustomerOpen(false); }}>
                      <div className="flex items-center gap-2">
                        <div className="w-6 h-6 rounded-md bg-secondary flex items-center justify-center text-[10px] font-semibold text-muted-foreground">{(c.name as string).charAt(0)}</div>
                        <div>
                          <span className="text-sm">{c.name as string}</span>
                          <span className="text-xs text-muted-foreground ml-2">{c.industry as string}</span>
                        </div>
                      </div>
                      {customerId === c.id && <Check className="ml-auto h-4 w-4" />}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
        {selectedCustomer && (
          <div className="mt-2 px-3 py-2 rounded-lg bg-secondary/50 border border-border/50 text-xs text-muted-foreground space-y-0.5">
            <p><span className="font-medium text-foreground">{selectedCustomer.contact as string}</span> &middot; {selectedCustomer.email as string}</p>
            <p>{selectedCustomer.location as string}</p>
          </div>
        )}
      </div>

      {/* Product selector */}
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Product</label>
        <Popover open={productOpen} onOpenChange={setProductOpen}>
          <PopoverTrigger asChild>
            <Button variant="outline" role="combobox" aria-expanded={productOpen} className="w-full justify-between mt-1 h-9 bg-secondary border-border text-sm font-normal">
              {selectedProduct ? (
                <span className="flex items-center gap-2">
                  <Package className="w-3.5 h-3.5 text-muted-foreground" />
                  {selectedProduct.name as string}
                  <ProductTypeBadge type={selectedProduct.productType as ProductType} />
                </span>
              ) : (
                <span className="text-muted-foreground">Select product...</span>
              )}
              <ChevronsUpDown className="ml-2 h-3.5 w-3.5 shrink-0 opacity-50" />
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-[--radix-popover-trigger-width] p-0" align="start">
            <Command>
              <CommandInput placeholder="Search products..." />
              <CommandList>
                <CommandEmpty>No products found.</CommandEmpty>
                <CommandGroup>
                  {productList.map((p) => (
                    <CommandItem key={p.id as string} value={p.name as string} onSelect={() => { setProductId(p.id as string); setProductOpen(false); }}>
                      <div className="flex items-center gap-2">
                        <span className="text-sm">{p.name as string}</span>
                        <ProductTypeBadge type={p.productType as ProductType} />
                      </div>
                      {productId === p.id && <Check className="ml-auto h-4 w-4" />}
                    </CommandItem>
                  ))}
                </CommandGroup>
              </CommandList>
            </Command>
          </PopoverContent>
        </Popover>
      </div>

      {/* Deal Type */}
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Deal Type</label>
        <div className="flex items-center gap-2 mt-1">
          {(["sale", "trial", "partner"] as DealType[]).map((t) => {
            const cfg = dealTypeBadgeConfig[t];
            return (
              <button key={t} type="button" onClick={() => setDealType(t)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all border", dealType === t ? cfg.className : "bg-secondary text-muted-foreground border-border hover:text-foreground")}>
                {cfg.label}
              </button>
            );
          })}
        </div>
      </div>

      {/* Value & Date */}
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Value ($)</label>
          <input type="number" value={value} onChange={(e) => setValue(Number(e.target.value))} className={inputClass} />
        </div>
        <div>
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Date</label>
          <input type="date" value={date} onChange={(e) => setDate(e.target.value)} className={inputClass} />
        </div>
      </div>

      {/* License Expiry (for trial/partner with licensed products) */}
      {(dealType === "trial" || dealType === "partner") && selectedProduct && (selectedProduct.productType as string) === "licensed" && (
        <div>
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">License Expires At</label>
          <input type="date" value={licenseExpiresAt} onChange={(e) => setLicenseExpiresAt(e.target.value)} className={inputClass} placeholder="Auto: 14 days for trial" />
          <p className="text-xs text-muted-foreground mt-1">Leave empty for default (14 days trial / 90 days partner)</p>
        </div>
      )}

      {/* Notes */}
      {(dealType === "trial" || dealType === "partner") && (
        <div>
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Notes</label>
          <textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} className={cn(inputClass, "h-auto py-2 resize-none")} placeholder="e.g., 30-day evaluation, partner agreement details..." />
        </div>
      )}
      <DialogFooter className="pt-4 border-t border-border">
        <Button variant="outline" onClick={onClose}>Cancel</Button>
        <Button onClick={handleSubmit} disabled={submitting}>{submitting ? "Saving..." : isEditing ? "Save Changes" : "Create Deal"}</Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- main ---------- */

export function ManageDealsSection() {
  const { data: allDeals, isLoading, mutate } = useDeals();
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedDeal, setSelectedDeal] = useState<Deal | null>(null);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [deleteTarget, setDeleteTarget] = useState<Deal | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const deals = (allDeals ?? []) as Deal[];
  const filtered = deals.filter((d) => {
    const company = (d.company as string).toLowerCase();
    const contact = (d.contact as string).toLowerCase();
    const q = searchQuery.toLowerCase();
    const matchesSearch = company.includes(q) || contact.includes(q);
    const matchesType = typeFilter === "all" || d.productType === typeFilter;
    return matchesSearch && matchesType;
  });

  const openDetail = (deal: Deal) => {
    setSelectedDeal(deal);
    setDialogMode("view");
    setDialogOpen(true);
  };

  const openEdit = (deal: Deal) => {
    setSelectedDeal(deal);
    setDialogMode("edit");
    setDialogOpen(true);
  };

  const openCreate = () => {
    setSelectedDeal(null);
    setDialogMode("create");
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setSelectedDeal(null);
    setDialogMode("view");
  };

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleGenerateKey = async (dealId: string) => {
    try {
      // The API auto-generates key + license record for licensed deals
      await updateDeal(dealId, {});
      mutate();
      toast.success("License key generated");
    } catch {
      toast.error("Failed to generate key");
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    try {
      await deleteDeal(deleteTarget.id as string);
      mutate();
      toast.success("Deal deleted");
      setDeleteTarget(null);
    } catch {
      toast.error("Failed to delete deal");
    } finally {
      setDeleteLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[500px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Filters */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input type="text" placeholder="Search deals..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all" />
          </div>
          <div className="flex items-center gap-2">
            {[{ key: "all", label: "All Types" }, { key: "licensed", label: "Licensed" }, { key: "saas", label: "Platform" }, { key: "api", label: "API" }].map((tab) => (
              <button key={tab.key} onClick={() => setTypeFilter(tab.key)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", typeFilter === tab.key ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground")}>{tab.label}</button>
            ))}
          </div>
        </div>
        <Button onClick={openCreate}>
          <Plus className="w-4 h-4 mr-2" />
          Add Deal
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="bg-secondary/50">
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Company</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Type</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Product</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Value</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Key / Usage</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Date</TableHead>
              <TableHead className="w-12" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((deal) => {
              const id = deal.id as string;
              const productType = deal.productType as ProductType;
              const licenseKey = deal.licenseKey as string | null;
              const usageLabel = deal.usageMetricLabel as string | null;
              const usageValue = deal.usageMetricValue as number | null;

              return (
                <TableRow key={id} className="cursor-pointer" onClick={() => openDetail(deal)}>
                  <TableCell className="px-4">
                    <div className="flex items-center gap-2">
                      <div className="w-7 h-7 rounded-md bg-secondary flex items-center justify-center text-xs font-semibold text-muted-foreground">{(deal.company as string).charAt(0)}</div>
                      <span className="font-medium">{deal.company as string}</span>
                    </div>
                  </TableCell>
                  <TableCell className="px-4">
                    <DealTypeBadge type={(deal.dealType as DealType) ?? "sale"} />
                  </TableCell>
                  <TableCell className="px-4">
                    <div className="flex items-center gap-2"><span className="text-sm">{deal.productName as string}</span><ProductTypeBadge type={productType} /></div>
                  </TableCell>
                  <TableCell className="px-4 font-semibold">${(deal.value as number).toLocaleString()}</TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    {productType === "licensed" && licenseKey ? (
                      <button onClick={() => copyKey(licenseKey)} className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary hover:bg-secondary/80 text-xs font-mono text-muted-foreground hover:text-foreground transition-colors">
                        <KeyRound className="w-3 h-3 shrink-0" /><span>{licenseKey}</span>
                        {copiedKey === licenseKey ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
                      </button>
                    ) : usageLabel ? (
                      <span className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary text-xs text-muted-foreground">
                        {productType === "saas" ? <Users className="w-3 h-3" /> : <Zap className="w-3 h-3" />}
                        {formatUsageMetric(usageValue ?? 0)} {usageLabel}
                      </span>
                    ) : (
                      <span className="text-muted-foreground/50">--</span>
                    )}
                  </TableCell>
                  <TableCell className="px-4 text-muted-foreground">{deal.date as string}</TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                          <MoreHorizontal className="w-4 h-4" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => openEdit(deal)}>
                          <Pencil className="w-4 h-4" />Edit
                        </DropdownMenuItem>
                        {productType === "licensed" && !licenseKey && (
                          <DropdownMenuItem onClick={() => handleGenerateKey(id)}>
                            <KeyRound className="w-4 h-4" />Generate Key
                          </DropdownMenuItem>
                        )}
                        {licenseKey && (
                          <DropdownMenuItem onClick={() => copyKey(licenseKey)}>
                            <Copy className="w-4 h-4" />Copy Key
                          </DropdownMenuItem>
                        )}
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(deal)}>
                          <Trash2 className="w-4 h-4" />Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-secondary/30">
          <span className="text-sm text-muted-foreground">Showing {filtered.length} of {deals.length} deals</span>
        </div>
      </div>

      {/* Detail / Edit / Create Dialog */}
      <Dialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          {dialogMode === "view" && selectedDeal ? (
            <DealDetail
              deal={selectedDeal}
              onEdit={() => setDialogMode("edit")}
              onDelete={() => { closeDialog(); setDeleteTarget(selectedDeal); }}
              onGenerateKey={handleGenerateKey}
              onCopyKey={copyKey}
              copiedKey={copiedKey}
            />
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>{selectedDeal ? "Edit Deal" : "Add Deal"}</DialogTitle>
                <DialogDescription>{selectedDeal ? "Update deal details" : "Create a new deal"}</DialogDescription>
              </DialogHeader>
              <DealForm key={selectedDeal?.id as string ?? "new"} deal={selectedDeal} onClose={closeDialog} onSuccess={() => mutate()} />
            </>
          )}
        </DialogContent>
      </Dialog>

      <DeleteDialog open={!!deleteTarget} onOpenChange={(open) => { if (!open) setDeleteTarget(null); }} title={`Delete deal for "${deleteTarget?.company ?? ""}"?`} description="This action cannot be undone." onConfirm={handleDelete} loading={deleteLoading} />
    </div>
  );
}
