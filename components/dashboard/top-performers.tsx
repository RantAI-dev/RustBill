"use client";

import { Package, TrendingUp, KeyRound, Users, Zap } from "lucide-react";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useProducts } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

const rankColors = [
  "bg-amber-500/10 text-amber-500",
  "bg-slate-400/10 text-slate-400",
  "bg-orange-600/10 text-orange-600",
];

function getMetricIcon(type: ProductType) {
  return type === "saas" ? Users : type === "api" ? Zap : KeyRound;
}

function getMetric(product: Record<string, unknown>): { label: string; value: number } {
  const type = product.productType as ProductType;
  if (type === "licensed") return { label: "active", value: (product.activeLicenses as number) ?? 0 };
  if (type === "saas") return { label: "MAU", value: (product.mau as number) ?? 0 };
  return { label: "calls/mo", value: (product.apiCalls as number) ?? 0 };
}

export function TopPerformers() {
  const { data: allProducts, isLoading } = useProducts();

  if (isLoading) {
    return (
      <div className="bg-card border border-border rounded-xl p-5">
        <Skeleton className="h-6 w-32 mb-5" />
        <div className="space-y-3">
          {[...Array(5)].map((_, i) => <Skeleton key={i} className="h-16 rounded-lg" />)}
        </div>
      </div>
    );
  }

  const products = (allProducts ?? []).slice(0, 5);

  return (
    <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-300">
      <div className="flex items-center justify-between mb-5">
        <div>
          <h3 className="text-base font-semibold text-foreground">Top Products</h3>
          <p className="text-sm text-muted-foreground mt-0.5">Best performers this month</p>
        </div>
        <div className="flex items-center gap-1 text-accent">
          <Package className="w-5 h-5" />
        </div>
      </div>

      <div className="space-y-3">
        {products.map((product: Record<string, unknown>, index: number) => {
          const productType = product.productType as ProductType;
          const MetricIcon = getMetricIcon(productType);
          const metric = getMetric(product);
          const rank = index + 1;

          return (
            <div
              key={product.id as string}
              className="group flex items-center justify-between p-3 rounded-lg hover:bg-secondary/50 transition-all duration-200 cursor-pointer animate-in fade-in slide-in-from-right-2"
              style={{ animationDelay: `${(index + 4) * 100}ms`, animationFillMode: "both" }}
            >
              <div className="flex items-center gap-3">
                <div className="relative">
                  <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center group-hover:bg-accent/10 transition-colors duration-200">
                    <Package className="w-4 h-4 text-muted-foreground group-hover:text-accent transition-colors" />
                  </div>
                  {rank <= 3 && (
                    <div className={`absolute -top-1 -right-1 w-5 h-5 rounded-full text-[10px] font-bold flex items-center justify-center ${rankColors[rank - 1]}`}>
                      {rank}
                    </div>
                  )}
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-medium text-foreground">{product.name as string}</p>
                    <ProductTypeBadge type={productType} />
                  </div>
                  <div className="flex items-center gap-1 text-xs text-muted-foreground">
                    <MetricIcon className="w-3 h-3" />
                    <span>{formatUsageMetric(metric.value)} {metric.label}</span>
                  </div>
                </div>
              </div>

              <div className="text-right">
                <p className="text-sm font-semibold text-foreground">${((product.revenue as number) / 1000).toFixed(0)}k</p>
                <div className="flex items-center justify-end gap-1 text-xs text-success">
                  <TrendingUp className="w-3 h-3" />
                  +{product.change as number}%
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
