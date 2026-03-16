"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2, TrendingUp, TrendingDown, Minus, ChevronDown, MapPin, Mail, Phone, DollarSign, Building2, CreditCard } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { useCustomers, createCustomer, updateCustomer, deleteCustomer } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Customer = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const trendConfig = {
  up: { icon: TrendingUp, color: "text-success", label: "Up" },
  down: { icon: TrendingDown, color: "text-destructive", label: "Down" },
  stable: { icon: Minus, color: "text-muted-foreground", label: "Stable" },
};

const tierColors: Record<string, string> = {
  Enterprise: "bg-violet-500/10 text-violet-500",
  Growth: "bg-blue-500/10 text-blue-500",
  Starter: "bg-emerald-500/10 text-emerald-500",
};

/* ---------- detail view ---------- */

function CustomerDetail({ customer, onEdit, onDelete }: { customer: Customer; onEdit: () => void; onDelete: () => void }) {
  const health = customer.healthScore as number;
  const t = customer.trend as "up" | "down" | "stable";
  const tConfig = trendConfig[t];
  const TrendIcon = tConfig.icon;
  const tierCls = tierColors[(customer.tier as string)] ?? "bg-secondary text-muted-foreground";
  const products = (customer.products as { type: string; name: string }[]) ?? [];

  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const valueClass = "text-sm font-medium text-foreground mt-0.5";

  const getHealthColor = (score: number) => {
    if (score >= 80) return "text-success";
    if (score >= 50) return "text-warning";
    return "text-destructive";
  };

  return (
    <div>
      <DialogHeader>
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center text-sm font-semibold text-muted-foreground">
            {(customer.name as string).charAt(0)}
          </div>
          <div>
            <div className="flex items-center gap-2">
              <DialogTitle className="text-lg">{customer.name as string}</DialogTitle>
              <span className={cn("inline-flex px-2 py-0.5 rounded-md text-xs font-medium", tierCls)}>{customer.tier as string}</span>
            </div>
            <DialogDescription>{customer.industry as string}</DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <div className="mt-6 space-y-6">
        {/* Contact info */}
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className={labelClass}>Location</p>
            <p className={cn(valueClass, "flex items-center gap-1.5")}>
              <MapPin className="w-3.5 h-3.5 text-muted-foreground" />
              {customer.location as string}
            </p>
          </div>
          <div>
            <p className={labelClass}>Contact</p>
            <p className={valueClass}>{customer.contact as string}</p>
          </div>
          <div>
            <p className={labelClass}>Last Contact</p>
            <p className={valueClass}>{customer.lastContact as string}</p>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className={labelClass}>Email</p>
            <p className={cn(valueClass, "flex items-center gap-1.5")}>
              <Mail className="w-3.5 h-3.5 text-muted-foreground" />
              {customer.email as string}
            </p>
          </div>
          <div>
            <p className={labelClass}>Phone</p>
            <p className={cn(valueClass, "flex items-center gap-1.5")}>
              <Phone className="w-3.5 h-3.5 text-muted-foreground" />
              {customer.phone as string}
            </p>
          </div>
        </div>

        {/* Revenue & health */}
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className={labelClass}>Revenue</p>
            <p className={cn(valueClass, "flex items-center gap-1.5 text-base")}>
              <DollarSign className="w-4 h-4 text-accent" />
              ${(customer.totalRevenue as number).toLocaleString()}
            </p>
          </div>
          <div>
            <p className={labelClass}>Health Score</p>
            <div className="flex items-center gap-2 mt-0.5">
              <div className="w-20 h-2 bg-secondary rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full transition-all"
                  style={{
                    width: `${health}%`,
                    backgroundColor: health >= 80 ? "oklch(0.7 0.18 145)" : health >= 50 ? "oklch(0.75 0.18 55)" : "oklch(0.65 0.2 25)",
                  }}
                />
              </div>
              <span className={cn("text-sm font-semibold", getHealthColor(health))}>{health}%</span>
            </div>
          </div>
          <div>
            <p className={labelClass}>Trend</p>
            <div className={cn("flex items-center gap-1.5 mt-0.5", tConfig.color)}>
              <TrendIcon className="w-4 h-4" />
              <span className="text-sm font-medium">{tConfig.label}</span>
            </div>
          </div>
        </div>

        {/* Billing Profile */}
        {!!(customer.billingEmail || customer.billingAddress || customer.taxId || customer.defaultPaymentMethod) && (
          <div>
            <p className={cn(labelClass, "mb-2 flex items-center gap-1.5")}><Building2 className="w-3.5 h-3.5" /> Billing Profile</p>
            <div className="grid grid-cols-2 gap-3 p-3 bg-secondary/30 rounded-lg">
              {!!customer.billingEmail && (
                <div>
                  <p className="text-[10px] text-muted-foreground uppercase">Billing Email</p>
                  <p className="text-sm text-foreground">{customer.billingEmail as string}</p>
                </div>
              )}
              {!!customer.defaultPaymentMethod && (
                <div>
                  <p className="text-[10px] text-muted-foreground uppercase">Payment Method</p>
                  <p className="text-sm text-foreground capitalize">{(customer.defaultPaymentMethod as string).replace("_", " ")}</p>
                </div>
              )}
              {!!customer.billingAddress && (
                <div className="col-span-2">
                  <p className="text-[10px] text-muted-foreground uppercase">Address</p>
                  <p className="text-sm text-foreground">
                    {customer.billingAddress as string}
                    {customer.billingCity ? `, ${customer.billingCity as string}` : ""}
                    {customer.billingState ? `, ${customer.billingState as string}` : ""}
                    {customer.billingZip ? ` ${customer.billingZip as string}` : ""}
                    {customer.billingCountry ? `, ${customer.billingCountry as string}` : ""}
                  </p>
                </div>
              )}
              {!!customer.taxId && (
                <div>
                  <p className="text-[10px] text-muted-foreground uppercase">Tax ID</p>
                  <p className="text-sm text-foreground font-mono">{customer.taxId as string}</p>
                </div>
              )}
            </div>
          </div>
        )}

        {/* Products */}
        {products.length > 0 && (
          <div>
            <p className={labelClass}>Products</p>
            <div className="flex flex-wrap gap-1.5 mt-1.5">
              {products.map((p, i) => (
                <span key={i} className="inline-flex px-2 py-1 rounded-md text-xs font-medium bg-secondary text-foreground">{p.name}</span>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-6 mt-6 border-t border-border">
        <Button variant="outline" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={onDelete}>
          <Trash2 className="w-4 h-4 mr-1.5" />
          Delete
        </Button>
        <Button size="sm" onClick={onEdit}>
          <Pencil className="w-4 h-4 mr-1.5" />
          Edit
        </Button>
      </div>
    </div>
  );
}

/* ---------- customer form ---------- */

function CustomerForm({ customer, onClose, onSuccess }: { customer: Customer | null; onClose: () => void; onSuccess: () => void }) {
  const isEditing = !!customer;
  const [name, setName] = useState((customer?.name as string) ?? "");
  const [industry, setIndustry] = useState((customer?.industry as string) ?? "");
  const [tier, setTier] = useState((customer?.tier as string) ?? "Starter");
  const [location, setLocation] = useState((customer?.location as string) ?? "");
  const [contact, setContact] = useState((customer?.contact as string) ?? "");
  const [email, setEmail] = useState((customer?.email as string) ?? "");
  const [phone, setPhone] = useState((customer?.phone as string) ?? "");
  const [billingEmail, setBillingEmail] = useState((customer?.billingEmail as string) ?? "");
  const [billingAddress, setBillingAddress] = useState((customer?.billingAddress as string) ?? "");
  const [billingCity, setBillingCity] = useState((customer?.billingCity as string) ?? "");
  const [billingState, setBillingState] = useState((customer?.billingState as string) ?? "");
  const [billingZip, setBillingZip] = useState((customer?.billingZip as string) ?? "");
  const [billingCountry, setBillingCountry] = useState((customer?.billingCountry as string) ?? "");
  const [taxId, setTaxId] = useState((customer?.taxId as string) ?? "");
  const [defaultPaymentMethod, setDefaultPaymentMethod] = useState((customer?.defaultPaymentMethod as string) ?? "");
  const [submitting, setSubmitting] = useState(false);

  const inputClass = "w-full h-9 mt-1 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent";

  const handleSubmit = async () => {
    if (!name.trim() || !industry.trim() || !contact.trim()) {
      toast.error("Name, industry, and contact are required");
      return;
    }
    setSubmitting(true);
    const data = {
      name, industry, tier, location, contact, email, phone,
      billingEmail: billingEmail || null,
      billingAddress: billingAddress || null,
      billingCity: billingCity || null,
      billingState: billingState || null,
      billingZip: billingZip || null,
      billingCountry: billingCountry || null,
      taxId: taxId || null,
      defaultPaymentMethod: defaultPaymentMethod || null,
    };
    if (isEditing) {
      const result = await updateCustomer(customer.id as string, data);
      if (result.success) {
        toast.success(`"${name}" updated`);
        onSuccess();
        onClose();
      }
    } else {
      const result = await createCustomer(data);
      if (result.success) {
        toast.success(`"${name}" created`);
        onSuccess();
        onClose();
      }
    }
    setSubmitting(false);
  };

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-2 gap-3">
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Company Name</label><input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} /></div>
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Industry</label><input value={industry} onChange={(e) => setIndustry(e.target.value)} className={inputClass} /></div>
      </div>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Tier</label>
          <div className="flex gap-2 mt-1">
            {(["Enterprise", "Growth", "Starter"] as const).map((t) => (
              <button key={t} onClick={() => setTier(t)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", tier === t ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground")}>
                {t}
              </button>
            ))}
          </div>
        </div>
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Location</label><input value={location} onChange={(e) => setLocation(e.target.value)} className={inputClass} /></div>
      </div>
      <div className="grid grid-cols-3 gap-3">
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Contact</label><input value={contact} onChange={(e) => setContact(e.target.value)} className={inputClass} /></div>
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Email</label><input type="email" value={email} onChange={(e) => setEmail(e.target.value)} className={inputClass} /></div>
        <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Phone</label><input value={phone} onChange={(e) => setPhone(e.target.value)} className={inputClass} /></div>
      </div>

      {/* Auto-computed fields info */}
      <div className="rounded-lg bg-secondary/50 border border-border p-3">
        <p className="text-xs text-muted-foreground">Revenue, health score, trend, and last contact are automatically computed from deals, subscriptions, and invoice data.</p>
      </div>

      {/* Billing Profile */}
      <div className="pt-3 mt-1 border-t border-border">
        <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3 flex items-center gap-1.5"><CreditCard className="w-3.5 h-3.5" /> Billing Profile</p>
        <div className="space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Billing Email</label><input type="email" value={billingEmail} onChange={(e) => setBillingEmail(e.target.value)} className={inputClass} placeholder="billing@company.com" /></div>
            <div>
              <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Payment Method</label>
              <div className="relative">
                <select value={defaultPaymentMethod} onChange={(e) => setDefaultPaymentMethod(e.target.value)} className={cn(inputClass, "appearance-none pr-8")}>
                  <option value="">None</option>
                  <option value="manual">Manual</option>
                  <option value="stripe">Stripe</option>
                  <option value="bank_transfer">Bank Transfer</option>
                  <option value="check">Check</option>
                </select>
                <ChevronDown className="absolute right-2.5 top-1/2 mt-0.5 -translate-y-1/2 w-3.5 h-3.5 text-muted-foreground pointer-events-none" />
              </div>
            </div>
          </div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Address</label><input value={billingAddress} onChange={(e) => setBillingAddress(e.target.value)} className={inputClass} placeholder="Street address" /></div>
          <div className="grid grid-cols-4 gap-3">
            <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">City</label><input value={billingCity} onChange={(e) => setBillingCity(e.target.value)} className={inputClass} /></div>
            <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">State</label><input value={billingState} onChange={(e) => setBillingState(e.target.value)} className={inputClass} /></div>
            <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Zip</label><input value={billingZip} onChange={(e) => setBillingZip(e.target.value)} className={inputClass} /></div>
            <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Country</label><input value={billingCountry} onChange={(e) => setBillingCountry(e.target.value)} className={inputClass} /></div>
          </div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Tax ID</label><input value={taxId} onChange={(e) => setTaxId(e.target.value)} className={inputClass} placeholder="e.g. US12345678" /></div>
        </div>
      </div>

      <DialogFooter className="pt-4 border-t border-border">
        <Button variant="outline" onClick={onClose}>Cancel</Button>
        <Button onClick={handleSubmit} disabled={submitting}>{submitting ? "Saving..." : isEditing ? "Save Changes" : "Create Customer"}</Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- main ---------- */

export function ManageCustomersSection() {
  const { data: allCustomers, isLoading, mutate } = useCustomers();
  const [searchQuery, setSearchQuery] = useState("");
  const [tierFilter, setTierFilter] = useState<string>("all");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedCustomer, setSelectedCustomer] = useState<Customer | null>(null);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [deleteTarget, setDeleteTarget] = useState<Customer | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);

  const customers = (allCustomers ?? []) as Customer[];
  const filtered = customers.filter((c) => {
    const name = (c.name as string).toLowerCase();
    const industry = (c.industry as string).toLowerCase();
    const q = searchQuery.toLowerCase();
    const matchesSearch = name.includes(q) || industry.includes(q);
    const matchesTier = tierFilter === "all" || c.tier === tierFilter;
    return matchesSearch && matchesTier;
  });

  const openDetail = (customer: Customer) => {
    setSelectedCustomer(customer);
    setDialogMode("view");
    setDialogOpen(true);
  };

  const openEdit = (customer: Customer) => {
    setSelectedCustomer(customer);
    setDialogMode("edit");
    setDialogOpen(true);
  };

  const openCreate = () => {
    setSelectedCustomer(null);
    setDialogMode("create");
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setSelectedCustomer(null);
    setDialogMode("view");
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    const result = await deleteCustomer(deleteTarget.id as string);
    if (result.success) {
      mutate();
      toast.success(`"${deleteTarget.name}" deleted`);
      setDeleteTarget(null);
    }
    setDeleteLoading(false);
  };

  const getHealthColor = (score: number) => {
    if (score >= 80) return "text-success";
    if (score >= 50) return "text-warning";
    return "text-destructive";
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
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input type="text" placeholder="Search customers..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all" />
          </div>
          <div className="flex items-center gap-2">
            {[{ key: "all", label: "All" }, { key: "Enterprise", label: "Enterprise" }, { key: "Growth", label: "Growth" }, { key: "Starter", label: "Starter" }].map((tab) => (
              <button key={tab.key} onClick={() => setTierFilter(tab.key)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", tierFilter === tab.key ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground")}>{tab.label}</button>
            ))}
          </div>
        </div>
        <Button onClick={openCreate}>
          <Plus className="w-4 h-4 mr-2" />
          Add Customer
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="bg-secondary/50">
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Name</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Industry</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Tier</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Contact</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Revenue</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Health</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Trend</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Products</TableHead>
              <TableHead className="w-12" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((customer) => {
              const id = customer.id as string;
              const t = customer.trend as "up" | "down" | "stable";
              const tConfig = trendConfig[t];
              const TrendIcon = tConfig.icon;
              const health = customer.healthScore as number;
              const products = (customer.products as { type: string; name: string }[]) ?? [];
              const tierCls = tierColors[(customer.tier as string)] ?? "bg-secondary text-muted-foreground";

              return (
                <TableRow key={id} className="cursor-pointer" onClick={() => openDetail(customer)}>
                  <TableCell className="px-4">
                    <div className="flex items-center gap-2">
                      <div className="w-7 h-7 rounded-md bg-secondary flex items-center justify-center text-xs font-semibold text-muted-foreground">{(customer.name as string).charAt(0)}</div>
                      <div>
                        <p className="font-medium">{customer.name as string}</p>
                        <p className="text-xs text-muted-foreground">{customer.location as string}</p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell className="px-4 text-sm">{customer.industry as string}</TableCell>
                  <TableCell className="px-4">
                    <span className={cn("inline-flex px-2 py-0.5 rounded-md text-xs font-medium", tierCls)}>{customer.tier as string}</span>
                  </TableCell>
                  <TableCell className="px-4">
                    <div><p className="text-sm">{customer.contact as string}</p><p className="text-xs text-muted-foreground">{customer.email as string}</p></div>
                  </TableCell>
                  <TableCell className="px-4 font-semibold">${((customer.totalRevenue as number) / 1000).toFixed(0)}k</TableCell>
                  <TableCell className="px-4">
                    <span className={cn("text-sm font-medium", getHealthColor(health))}>{health}</span>
                  </TableCell>
                  <TableCell className="px-4">
                    <div className={cn("inline-flex items-center gap-1", tConfig.color)}>
                      <TrendIcon className="w-3.5 h-3.5" />
                      <span className="text-xs font-medium">{tConfig.label}</span>
                    </div>
                  </TableCell>
                  <TableCell className="px-4">
                    {products.length > 0 ? (
                      <div className="flex flex-wrap gap-1">
                        {products.map((p, i) => (
                          <span key={i} className="inline-flex px-1.5 py-0.5 rounded text-[10px] font-medium bg-secondary text-muted-foreground">{p.name}</span>
                        ))}
                      </div>
                    ) : (
                      <span className="text-muted-foreground/50">--</span>
                    )}
                  </TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                          <MoreHorizontal className="w-4 h-4" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => openEdit(customer)}>
                          <Pencil className="w-4 h-4" />Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(customer)}>
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
          <span className="text-sm text-muted-foreground">Showing {filtered.length} of {customers.length} customers</span>
        </div>
      </div>

      {/* Detail / Edit / Create Dialog */}
      <Dialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          {dialogMode === "view" && selectedCustomer ? (
            <CustomerDetail
              customer={selectedCustomer}
              onEdit={() => setDialogMode("edit")}
              onDelete={() => { closeDialog(); setDeleteTarget(selectedCustomer); }}
            />
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>{selectedCustomer ? "Edit Customer" : "Add Customer"}</DialogTitle>
                <DialogDescription>{selectedCustomer ? "Update customer details" : "Create a new customer"}</DialogDescription>
              </DialogHeader>
              <CustomerForm key={selectedCustomer?.id as string ?? "new"} customer={selectedCustomer} onClose={closeDialog} onSuccess={() => mutate()} />
            </>
          )}
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <DeleteDialog open={!!deleteTarget} onOpenChange={(open) => { if (!open) setDeleteTarget(null); }} title={`Delete "${deleteTarget?.name ?? ""}"?`} description="This action cannot be undone. This will permanently delete the customer and all associated data." onConfirm={handleDelete} loading={deleteLoading} />
    </div>
  );
}
