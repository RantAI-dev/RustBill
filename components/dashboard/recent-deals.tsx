"use client";

import { ArrowUpRight, Copy, Check, Users, Zap, DollarSign, FlaskConical, Handshake } from "lucide-react";
import React, { useState } from "react";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useDeals } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utils";

const dealTypeStyles: Record<string, { label: string; className: string; icon: React.ElementType }> = {
  sale: { label: "Sale", className: "bg-accent/10 text-accent border-accent/20", icon: DollarSign },
  trial: { label: "Trial", className: "bg-chart-3/10 text-chart-3 border-chart-3/20", icon: FlaskConical },
  partner: { label: "Partner", className: "bg-chart-1/10 text-chart-1 border-chart-1/20", icon: Handshake },
};

export function RecentDeals() {
  const { data: allDeals, isLoading } = useDeals();
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

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

  // Take the 5 most recent deals
  const deals = (allDeals ?? []).slice(0, 5);

  return (
    <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-200">
      <div className="flex items-center justify-between mb-5">
        <div>
          <h3 className="text-base font-semibold text-foreground">Recent Deals</h3>
          <p className="text-sm text-muted-foreground mt-0.5">Latest activity</p>
        </div>
        <button className="flex items-center gap-1 text-sm text-accent hover:text-accent/80 font-medium transition-colors group">
          View all
          <ArrowUpRight className="w-4 h-4 transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
        </button>
      </div>

      <div className="space-y-3">
        {deals.map((deal: Record<string, unknown>, index: number) => {
          const productType = deal.productType as ProductType;
          const productName = (deal.productName ?? deal.product) as string;
          const licenseKey = deal.licenseKey as string | null;
          const company = deal.company as string;
          const value = deal.value as number;

          return (
            <div
              key={deal.id as string}
              className="group flex items-center justify-between p-3 rounded-lg hover:bg-secondary/50 transition-all duration-200 cursor-pointer animate-in fade-in slide-in-from-left-2"
              style={{ animationDelay: `${(index + 3) * 100}ms`, animationFillMode: "both" }}
            >
              <div className="flex items-center gap-3 min-w-0">
                <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center text-sm font-semibold text-muted-foreground group-hover:bg-accent/10 group-hover:text-accent transition-all duration-200 shrink-0">
                  {company.charAt(0)}
                </div>
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <p className="text-sm font-medium text-foreground truncate">{company}</p>
                    <ProductTypeBadge type={productType} />
                    {(() => {
                      const dt = (deal.dealType as string) ?? "sale";
                      const cfg = dealTypeStyles[dt];
                      if (!cfg || dt === "sale") return null;
                      const Icon = cfg.icon;
                      return (
                        <span className={cn("inline-flex items-center gap-1 px-1.5 py-0.5 rounded-full text-[10px] font-medium border", cfg.className)}>
                          <Icon className="w-2.5 h-2.5" />
                          {cfg.label}
                        </span>
                      );
                    })()}
                  </div>
                  <p className="text-xs text-muted-foreground">{productName} &middot; {deal.date as string}</p>
                </div>
              </div>

              <div className="flex items-center gap-3 shrink-0">
                {productType === "licensed" && licenseKey && (
                  <button
                    onClick={(e) => { e.stopPropagation(); copyKey(licenseKey); }}
                    className="hidden sm:flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary text-xs font-mono text-muted-foreground hover:text-foreground transition-colors"
                    title="Copy license key"
                  >
                    <span className="truncate max-w-[120px]">{licenseKey}</span>
                    {copiedKey === licenseKey ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
                  </button>
                )}
                {productType === "saas" && !!deal.usageMetricLabel && (
                  <span className="hidden sm:flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary text-xs text-muted-foreground">
                    <Users className="w-3 h-3 shrink-0" />
                    {formatUsageMetric(deal.usageMetricValue as number)} {deal.usageMetricLabel as string}
                  </span>
                )}
                {productType === "api" && !!deal.usageMetricLabel && (
                  <span className="hidden sm:flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary text-xs text-muted-foreground">
                    <Zap className="w-3 h-3 shrink-0" />
                    {formatUsageMetric(deal.usageMetricValue as number)} {deal.usageMetricLabel as string}
                  </span>
                )}
                <span className="text-sm font-semibold text-foreground">${value.toLocaleString()}</span>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
