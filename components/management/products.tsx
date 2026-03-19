"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2, TrendingUp, TrendingDown, KeyRound, Users, Zap } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useProducts, createProduct, updateProduct, deleteProduct } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Product = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

/* ---------- detail view ---------- */

function ProductDetail({ product, onEdit, onDelete }: { product: Product; onEdit: () => void; onDelete: () => void }) {
  const type = product.productType as ProductType;
  const revenue = product.revenue as number;
  const target = product.target as number;
  const pct = (revenue / target) * 100;
  const change = product.change as number;

  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const valueClass = "text-sm font-medium text-foreground mt-0.5";

  return (
    <div>
      <DialogHeader>
        <div className="flex items-center gap-3">
          <DialogTitle className="text-lg">{product.name as string}</DialogTitle>
          <ProductTypeBadge type={type} />
        </div>
        <DialogDescription>Product details</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-6">
        {/* Revenue section */}
        <div className="grid grid-cols-3 gap-4">
          <div>
            <p className={labelClass}>Revenue</p>
            <p className={valueClass}>${(revenue / 1000).toFixed(0)}k</p>
          </div>
          <div>
            <p className={labelClass}>Target</p>
            <p className={valueClass}>${(target / 1000).toFixed(0)}k</p>
          </div>
          <div>
            <p className={labelClass}>Change</p>
            <p className={cn(valueClass, "flex items-center gap-1", change >= 0 ? "text-success" : "text-destructive")}>
              {change >= 0 ? <TrendingUp className="w-3.5 h-3.5" /> : <TrendingDown className="w-3.5 h-3.5" />}
              {change >= 0 ? "+" : ""}{change}%
            </p>
          </div>
        </div>

        {/* Progress bar */}
        <div>
          <div className="flex items-center justify-between text-xs mb-1.5">
            <span className="text-muted-foreground">Revenue vs Target</span>
            <span className={cn("font-medium", pct >= 100 ? "text-success" : "text-foreground")}>{pct.toFixed(0)}%</span>
          </div>
          <div className="h-2 bg-secondary rounded-full overflow-hidden">
            <div
              className={cn("h-full rounded-full transition-all duration-700", pct >= 100 ? "bg-success" : "bg-accent")}
              style={{ width: `${Math.min(pct, 100)}%` }}
            />
          </div>
        </div>

        {/* Type-specific metrics */}
        {type === "licensed" && (
          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className={labelClass}>Units Sold</p>
              <p className={valueClass}>{(product.unitsSold as number) ?? 0}</p>
            </div>
            <div>
              <p className={labelClass}>Active Licenses</p>
              <p className={cn(valueClass, "flex items-center gap-1.5")}>
                <KeyRound className="w-3.5 h-3.5 text-chart-1" />
                {(product.activeLicenses as number) ?? 0} / {(product.totalLicenses as number) ?? 0}
              </p>
            </div>
            <div>
              <p className={labelClass}>Utilization</p>
              <p className={valueClass}>
                {(product.totalLicenses as number) > 0
                  ? Math.round(((product.activeLicenses as number) / (product.totalLicenses as number)) * 100)
                  : 0}%
              </p>
            </div>
          </div>
        )}
        {type === "saas" && (
          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className={labelClass}>MAU</p>
              <p className={cn(valueClass, "flex items-center gap-1.5")}><Users className="w-3.5 h-3.5 text-chart-3" />{formatUsageMetric((product.mau as number) ?? 0)}</p>
            </div>
            <div>
              <p className={labelClass}>DAU</p>
              <p className={valueClass}>{formatUsageMetric((product.dau as number) ?? 0)}</p>
            </div>
            <div>
              <p className={labelClass}>Churn Rate</p>
              <p className={valueClass}>{(product.churnRate as number) ?? 0}%</p>
            </div>
            <div>
              <p className={labelClass}>Free Users</p>
              <p className={valueClass}>{formatUsageMetric((product.freeUsers as number) ?? 0)}</p>
            </div>
            <div>
              <p className={labelClass}>Paid Users</p>
              <p className={valueClass}>{formatUsageMetric((product.paidUsers as number) ?? 0)}</p>
            </div>
          </div>
        )}
        {type === "api" && (
          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className={labelClass}>API Calls / mo</p>
              <p className={cn(valueClass, "flex items-center gap-1.5")}><Zap className="w-3.5 h-3.5 text-chart-5" />{formatUsageMetric((product.apiCalls as number) ?? 0)}</p>
            </div>
            <div>
              <p className={labelClass}>Active Developers</p>
              <p className={valueClass}>{(product.activeDevelopers as number) ?? 0}</p>
            </div>
            <div>
              <p className={labelClass}>Avg Latency</p>
              <p className={valueClass}>{(product.avgLatency as number) ?? 0}ms</p>
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

/* ---------- product form ---------- */

function ProductForm({ product, onClose, onSuccess }: { product: Product | null; onClose: () => void; onSuccess: () => void }) {
  const isEditing = !!product;
  const [productType, setProductType] = useState<ProductType>((product?.productType as ProductType) ?? "licensed");
  const [name, setName] = useState((product?.name as string) ?? "");
  const [target, setTarget] = useState((product?.target as number) ?? 0);
  // SaaS-specific (external tracking)
  const [mau, setMau] = useState((product?.mau as number) ?? 0);
  const [dau, setDau] = useState((product?.dau as number) ?? 0);
  const [freeUsers, setFreeUsers] = useState((product?.freeUsers as number) ?? 0);
  const [paidUsers, setPaidUsers] = useState((product?.paidUsers as number) ?? 0);
  const [churnRate, setChurnRate] = useState((product?.churnRate as number) ?? 0);
  // API-specific (external tracking)
  const [apiCalls, setApiCalls] = useState((product?.apiCalls as number) ?? 0);
  const [activeDevelopers, setActiveDevelopers] = useState((product?.activeDevelopers as number) ?? 0);
  const [avgLatency, setAvgLatency] = useState((product?.avgLatency as number) ?? 0);
  const [submitting, setSubmitting] = useState(false);

  const toNumberOrZero = (value: string) => {
    const n = Number(value);
    return Number.isFinite(n) ? n : 0;
  };

  const inputClass = "w-full h-9 mt-1 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent";

  const handleSubmit = async () => {
    if (!name.trim()) { toast.error("Name is required"); return; }
    setSubmitting(true);
    const base = { name, productType, target };
    const data = productType === "licensed"
      ? { ...base }
      : productType === "saas"
        ? { ...base, mau, dau, freeUsers, paidUsers, churnRate }
        : { ...base, apiCalls, activeDevelopers, avgLatency };

    if (isEditing) {
      const result = await updateProduct(product.id as string, data);
      if (result.success) {
        toast.success(`"${name}" updated`);
        onSuccess();
        onClose();
      }
    } else {
      const result = await createProduct(data);
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
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Name</label>
        <input value={name} onChange={(e) => setName(e.target.value)} className={inputClass} />
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Product Type</label>
        <div className="flex gap-2 mt-1">
          {(["licensed", "saas", "api"] as const).map((t) => (
            <button key={t} onClick={() => !isEditing && setProductType(t)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", productType === t ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground", isEditing && "cursor-not-allowed opacity-70")}>
              {t === "saas" ? "Platform" : t === "api" ? "API" : "Licensed"}
            </button>
          ))}
        </div>
      </div>
      <div>
        <label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Target ($)</label>
        <input type="number" value={target} onChange={(e) => setTarget(toNumberOrZero(e.target.value))} className={inputClass} />
      </div>
      {productType === "licensed" && (
        <div className="rounded-lg bg-secondary/50 border border-border p-3">
          <p className="text-xs text-muted-foreground">Revenue, units sold, and license counts are automatically computed from deals and licenses data.</p>
        </div>
      )}
      {productType === "saas" && (
        <div className="grid grid-cols-3 gap-3">
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">MAU</label><input type="number" value={mau} onChange={(e) => setMau(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">DAU</label><input type="number" value={dau} onChange={(e) => setDau(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Churn %</label><input type="number" step="0.1" value={churnRate} onChange={(e) => setChurnRate(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Free Users</label><input type="number" value={freeUsers} onChange={(e) => setFreeUsers(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Paid Users</label><input type="number" value={paidUsers} onChange={(e) => setPaidUsers(toNumberOrZero(e.target.value))} className={inputClass} /></div>
        </div>
      )}
      {productType === "api" && (
        <div className="grid grid-cols-3 gap-3">
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">API Calls</label><input type="number" value={apiCalls} onChange={(e) => setApiCalls(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Developers</label><input type="number" value={activeDevelopers} onChange={(e) => setActiveDevelopers(toNumberOrZero(e.target.value))} className={inputClass} /></div>
          <div><label className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Latency (ms)</label><input type="number" value={avgLatency} onChange={(e) => setAvgLatency(toNumberOrZero(e.target.value))} className={inputClass} /></div>
        </div>
      )}
      <DialogFooter className="pt-4 border-t border-border">
        <Button variant="outline" onClick={onClose}>Cancel</Button>
        <Button onClick={handleSubmit} disabled={submitting}>
          {submitting ? "Saving..." : isEditing ? "Save Changes" : "Create Product"}
        </Button>
      </DialogFooter>
    </div>
  );
}

/* ---------- main ---------- */

export function ManageProductsSection() {
  const { data: products, isLoading, mutate } = useProducts();
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<string>("all");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedProduct, setSelectedProduct] = useState<Product | null>(null);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [deleteTarget, setDeleteTarget] = useState<Product | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);

  const allProducts = (products ?? []) as Product[];
  const filtered = allProducts.filter((p) => {
    const name = (p.name as string).toLowerCase();
    const matchesSearch = name.includes(searchQuery.toLowerCase());
    const matchesType = typeFilter === "all" || p.productType === typeFilter;
    return matchesSearch && matchesType;
  });

  const openDetail = (product: Product) => {
    setSelectedProduct(product);
    setDialogMode("view");
    setDialogOpen(true);
  };

  const openEdit = (product: Product) => {
    setSelectedProduct(product);
    setDialogMode("edit");
    setDialogOpen(true);
  };

  const openCreate = () => {
    setSelectedProduct(null);
    setDialogMode("create");
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setSelectedProduct(null);
    setDialogMode("view");
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    const result = await deleteProduct(deleteTarget.id as string);
    if (result.success) {
      mutate();
      toast.success(`"${deleteTarget.name}" deleted`);
      setDeleteTarget(null);
    }
    setDeleteLoading(false);
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
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search products..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all"
            />
          </div>
          <div className="flex items-center gap-2">
            {[{ key: "all", label: "All" }, { key: "licensed", label: "Licensed" }, { key: "saas", label: "Platform" }, { key: "api", label: "API" }].map((tab) => (
              <button key={tab.key} onClick={() => setTypeFilter(tab.key)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", typeFilter === tab.key ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground")}>
                {tab.label}
              </button>
            ))}
          </div>
        </div>
        <Button onClick={openCreate}>
          <Plus className="w-4 h-4 mr-2" />
          Add Product
        </Button>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="bg-secondary/50">
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Name</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Type</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Revenue</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Target</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">% to Target</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Metric</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Change</TableHead>
              <TableHead className="w-12" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((product) => {
              const pct = ((product.revenue as number) / (product.target as number) * 100);
              const type = product.productType as ProductType;
              const metricStr = type === "licensed"
                ? `${product.activeLicenses ?? 0} / ${product.totalLicenses ?? 0} licenses`
                : type === "saas"
                  ? `${formatUsageMetric((product.mau as number) ?? 0)} MAU`
                  : `${formatUsageMetric((product.apiCalls as number) ?? 0)} calls/mo`;

              return (
                <TableRow key={product.id as string} className="cursor-pointer" onClick={() => openDetail(product)}>
                  <TableCell className="px-4 font-medium">{product.name as string}</TableCell>
                  <TableCell className="px-4"><ProductTypeBadge type={type} /></TableCell>
                  <TableCell className="px-4">${((product.revenue as number) / 1000).toFixed(0)}k</TableCell>
                  <TableCell className="px-4">${((product.target as number) / 1000).toFixed(0)}k</TableCell>
                  <TableCell className="px-4">
                    <span className={cn("text-sm font-medium", pct >= 100 ? "text-success" : "text-foreground")}>{pct.toFixed(0)}%</span>
                  </TableCell>
                  <TableCell className="px-4 text-sm text-muted-foreground">{metricStr}</TableCell>
                  <TableCell className="px-4">
                    <span className={cn("text-sm font-medium", (product.change as number) >= 0 ? "text-success" : "text-destructive")}>
                      {(product.change as number) >= 0 ? "+" : ""}{product.change as number}%
                    </span>
                  </TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                          <MoreHorizontal className="w-4 h-4" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => openEdit(product)}>
                          <Pencil className="w-4 h-4" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(product)}>
                          <Trash2 className="w-4 h-4" />
                          Delete
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
          <span className="text-sm text-muted-foreground">
            Showing {filtered.length} of {allProducts.length} products
          </span>
        </div>
      </div>

      {/* Detail / Edit / Create Dialog */}
      <Dialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          {dialogMode === "view" && selectedProduct ? (
            <ProductDetail
              product={selectedProduct}
              onEdit={() => setDialogMode("edit")}
              onDelete={() => { closeDialog(); setDeleteTarget(selectedProduct); }}
            />
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>{selectedProduct ? "Edit Product" : "Add Product"}</DialogTitle>
                <DialogDescription>{selectedProduct ? "Update product details" : "Create a new product"}</DialogDescription>
              </DialogHeader>
              <ProductForm
                key={selectedProduct?.id as string ?? "new"}
                product={selectedProduct}
                onClose={closeDialog}
                onSuccess={() => mutate()}
              />
            </>
          )}
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation */}
      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title={`Delete "${deleteTarget?.name ?? ""}"?`}
        description="This action cannot be undone. This will permanently delete the product and all associated data."
        onConfirm={handleDelete}
        loading={deleteLoading}
      />
    </div>
  );
}
