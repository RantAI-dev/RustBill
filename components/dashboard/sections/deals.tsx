"use client";

import React, { useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import {
  Search,
  ArrowUpDown,
  KeyRound,
  Users,
  Zap,
  DollarSign,
  FlaskConical,
  Handshake,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useDeals } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

type FilterTab = "all" | "licensed" | "saas" | "api";
type SortField = "company" | "value" | null;

const dealTypeBadgeStyles: Record<string, { label: string; className: string; icon: React.ElementType }> = {
  sale: { label: "Sale", className: "bg-accent/10 text-accent border-accent/20", icon: DollarSign },
  trial: { label: "Trial", className: "bg-chart-3/10 text-chart-3 border-chart-3/20", icon: FlaskConical },
  partner: { label: "Partner", className: "bg-chart-1/10 text-chart-1 border-chart-1/20", icon: Handshake },
};

const PER_PAGE = 15;

/* ---------- main section ---------- */

export function DealsSection() {
  const { data: allDeals, isLoading } = useDeals();
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<FilterTab>("all");
  const [sortField, setSortField] = useState<SortField>(null);
  const [sortDir, setSortDir] = useState<"asc" | "desc">("desc");
  const [page, setPage] = useState(1);
  const deals = (allDeals ?? []) as Record<string, unknown>[];

  // Reset page when filters change
  useEffect(() => { setPage(1); }, [searchQuery, typeFilter, sortField, sortDir]);

  const filteredDeals = deals.filter((deal) => {
    const company = deal.company as string;
    const contact = deal.contact as string;
    const dealType = deal.productType as string;

    const matchesSearch =
      company.toLowerCase().includes(searchQuery.toLowerCase()) ||
      contact.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesType = typeFilter === "all" || dealType === typeFilter;
    return matchesSearch && matchesType;
  });

  // Sort
  const sortedDeals = [...filteredDeals].sort((a, b) => {
    if (!sortField) return 0;
    if (sortField === "company") {
      const aVal = (a.company as string).toLowerCase();
      const bVal = (b.company as string).toLowerCase();
      return sortDir === "asc" ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
    }
    if (sortField === "value") {
      const aVal = a.value as number;
      const bVal = b.value as number;
      return sortDir === "asc" ? aVal - bVal : bVal - aVal;
    }
    return 0;
  });

  // Paginate
  const totalPages = Math.max(1, Math.ceil(sortedDeals.length / PER_PAGE));
  const paginatedDeals = sortedDeals.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  const toggleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDir((d) => (d === "asc" ? "desc" : "asc"));
    } else {
      setSortField(field);
      setSortDir("asc");
    }
  };

  const maskKey = (key: string) => {
    const lastSegment = key.slice(-4);
    return `\u2022\u2022\u2022\u2022-\u2022\u2022\u2022\u2022-\u2022\u2022\u2022\u2022-${lastSegment}`;
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-5 w-64" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[400px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <p className="text-sm text-muted-foreground">View deals across all product types</p>
      </div>

      {/* Product type filter tabs */}
      <div className="flex items-center gap-2 border-b border-border pb-3">
        {([
          { key: "all", label: "All Products" },
          { key: "licensed", label: "Licensed" },
          { key: "saas", label: "Platform" },
          { key: "api", label: "API" },
        ] as { key: FilterTab; label: string }[]).map((tab) => (
          <button
            key={tab.key}
            onClick={() => setTypeFilter(tab.key)}
            className={cn(
              "px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200",
              typeFilter === tab.key
                ? "bg-accent text-accent-foreground"
                : "bg-secondary text-muted-foreground hover:text-foreground"
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Search */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search deals, contacts, or keys..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all duration-200"
            />
          </div>
        </div>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-500">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border bg-secondary/50">
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                  <button onClick={() => toggleSort("company")} className="flex items-center gap-1 hover:text-foreground transition-colors">
                    Company
                    <ArrowUpDown className={cn("w-3 h-3", sortField === "company" && "text-accent")} />
                  </button>
                </th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Type</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Product</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                  <button onClick={() => toggleSort("value")} className="flex items-center gap-1 hover:text-foreground transition-colors">
                    Value
                    <ArrowUpDown className={cn("w-3 h-3", sortField === "value" && "text-accent")} />
                  </button>
                </th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Key / Usage</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Date</th>
              </tr>
            </thead>
            <tbody>
              {paginatedDeals.map((deal, index) => {
                const dealId = deal.id as string;
                const company = deal.company as string;
                const dealValue = deal.value as number;
                const productName = deal.productName as string;
                const productType = deal.productType as ProductType;
                const dealDate = deal.date as string;
                const licenseKey = deal.licenseKey as string | null;
                const usageMetricLabel = deal.usageMetricLabel as string | null;
                const usageMetricValue = deal.usageMetricValue as number | null;

                return (
                  <tr
                    key={dealId}
                    className="border-b border-border last:border-0 hover:bg-secondary/30 transition-colors duration-150 animate-in fade-in slide-in-from-left-2"
                    style={{ animationDelay: `${index * 50}ms`, animationFillMode: "both" }}
                  >
                    <td className="py-4 px-4">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-md bg-secondary flex items-center justify-center text-xs font-semibold text-muted-foreground">
                          {company.charAt(0)}
                        </div>
                        <span className="text-sm font-medium text-foreground">{company}</span>
                      </div>
                    </td>
                    <td className="py-4 px-4">
                      {(() => {
                        const dt = (deal.dealType as string) ?? "sale";
                        const cfg = dealTypeBadgeStyles[dt];
                        if (!cfg) return null;
                        const Icon = cfg.icon;
                        return (
                          <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border", cfg.className)}>
                            <Icon className="w-3 h-3" />
                            {cfg.label}
                          </span>
                        );
                      })()}
                    </td>
                    <td className="py-4 px-4">
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-foreground">{productName}</span>
                        <ProductTypeBadge type={productType} />
                      </div>
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm font-semibold text-foreground">
                        ${dealValue.toLocaleString()}
                      </span>
                    </td>
                    <td className="py-4 px-4">
                      {productType === "licensed" && licenseKey ? (
                        <span className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary text-xs font-mono text-muted-foreground">
                          <KeyRound className="w-3 h-3 shrink-0" />
                          <span>{maskKey(licenseKey)}</span>
                        </span>
                      ) : productType === "saas" && usageMetricLabel ? (
                        <span className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-chart-3/5 text-xs text-chart-3 font-medium">
                          <Users className="w-3 h-3" />
                          {formatUsageMetric(usageMetricValue ?? 0)} {usageMetricLabel}
                        </span>
                      ) : productType === "api" && usageMetricLabel ? (
                        <span className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-chart-5/5 text-xs text-chart-5 font-medium">
                          <Zap className="w-3 h-3" />
                          {formatUsageMetric(usageMetricValue ?? 0)} {usageMetricLabel}
                        </span>
                      ) : (
                        <span className="text-sm text-muted-foreground/50">--</span>
                      )}
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm text-muted-foreground">{dealDate}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-secondary/30">
          <span className="text-sm text-muted-foreground">
            Showing {sortedDeals.length > 0 ? (page - 1) * PER_PAGE + 1 : 0}–{Math.min(page * PER_PAGE, sortedDeals.length)} of {sortedDeals.length} deals
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page <= 1}
              className="px-2 py-1.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors duration-200 disabled:opacity-40 disabled:pointer-events-none"
            >
              <ChevronLeft className="w-4 h-4" />
            </button>
            {Array.from({ length: totalPages }, (_, i) => i + 1)
              .slice(Math.max(0, page - 3), page + 2)
              .map((p) => (
                <button
                  key={p}
                  onClick={() => setPage(p)}
                  className={cn(
                    "px-3 py-1.5 rounded-lg text-sm font-medium transition-colors duration-200",
                    p === page
                      ? "bg-accent text-accent-foreground"
                      : "text-muted-foreground hover:text-foreground hover:bg-secondary"
                  )}
                >
                  {p}
                </button>
              ))}
            <button
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages}
              className="px-2 py-1.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors duration-200 disabled:opacity-40 disabled:pointer-events-none"
            >
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
