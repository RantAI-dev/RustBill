"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Search, Copy, Check, KeyRound, FlaskConical, Handshake, Clock } from "lucide-react";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType } from "@/lib/product-types";
import { useDeals } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

type DealTypeFilter = "all" | "trial" | "partner";

const dealTypeBadgeConfig: Record<string, { label: string; className: string; icon: React.ElementType }> = {
  trial: { label: "Trial", className: "bg-chart-3/10 text-chart-3 border-chart-3/20", icon: FlaskConical },
  partner: { label: "Partner", className: "bg-chart-1/10 text-chart-1 border-chart-1/20", icon: Handshake },
};

export function TrialsSection() {
  const { data: allDeals, isLoading } = useDeals();
  const [searchQuery, setSearchQuery] = useState("");
  const [typeFilter, setTypeFilter] = useState<DealTypeFilter>("all");
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const deals = (allDeals ?? []) as Record<string, unknown>[];

  // Only show trial and partner deals
  const trialDeals = deals.filter((d) => d.dealType === "trial" || d.dealType === "partner");

  const filtered = trialDeals.filter((d) => {
    const company = (d.company as string).toLowerCase();
    const contact = (d.contact as string).toLowerCase();
    const q = searchQuery.toLowerCase();
    const matchesSearch = company.includes(q) || contact.includes(q);
    const matchesType = typeFilter === "all" || d.dealType === typeFilter;
    return matchesSearch && matchesType;
  });

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-5 w-64" />
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[400px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <p className="text-sm text-muted-foreground">Active trials and partner grants</p>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {[
          { label: "Active Trials", value: trialDeals.filter((d) => d.dealType === "trial").length, icon: FlaskConical, color: "text-chart-3" },
          { label: "Partner Grants", value: trialDeals.filter((d) => d.dealType === "partner").length, icon: Handshake, color: "text-chart-1" },
          { label: "Total", value: trialDeals.length, icon: Clock, color: "text-accent" },
        ].map((stat, i) => (
          <div
            key={stat.label}
            className="bg-card border border-border rounded-xl p-4 flex items-center gap-3 animate-in fade-in slide-in-from-bottom-4 duration-500"
            style={{ animationDelay: `${i * 100}ms`, animationFillMode: "both" }}
          >
            <div className="w-10 h-10 rounded-lg bg-secondary flex items-center justify-center">
              <stat.icon className={`w-5 h-5 ${stat.color}`} />
            </div>
            <div>
              <p className="text-2xl font-semibold text-foreground">{stat.value}</p>
              <p className="text-xs text-muted-foreground">{stat.label}</p>
            </div>
          </div>
        ))}
      </div>

      {/* Filter tabs */}
      <div className="flex items-center gap-2 border-b border-border pb-3">
        {([
          { key: "all", label: "All" },
          { key: "trial", label: "Trials" },
          { key: "partner", label: "Partners" },
        ] as { key: DealTypeFilter; label: string }[]).map((tab) => (
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
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
        <input
          type="text"
          placeholder="Search trials..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all duration-200"
        />
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-500">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border bg-secondary/50">
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Company</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Type</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Product</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">License Key</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Date</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Notes</th>
              </tr>
            </thead>
            <tbody>
              {filtered.map((deal, index) => {
                const dealType = deal.dealType as string;
                const badge = dealTypeBadgeConfig[dealType];
                const licenseKey = deal.licenseKey as string | null;
                const BadgeIcon = badge?.icon;

                return (
                  <tr
                    key={deal.id as string}
                    className="border-b border-border last:border-0 hover:bg-secondary/30 transition-colors duration-150 animate-in fade-in slide-in-from-left-2"
                    style={{ animationDelay: `${index * 50}ms`, animationFillMode: "both" }}
                  >
                    <td className="py-4 px-4">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-md bg-secondary flex items-center justify-center text-xs font-semibold text-muted-foreground">
                          {(deal.company as string).charAt(0)}
                        </div>
                        <div>
                          <span className="text-sm font-medium text-foreground">{deal.company as string}</span>
                          <p className="text-xs text-muted-foreground">{deal.contact as string}</p>
                        </div>
                      </div>
                    </td>
                    <td className="py-4 px-4">
                      {badge && (
                        <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border", badge.className)}>
                          {BadgeIcon && <BadgeIcon className="w-3 h-3" />}
                          {badge.label}
                        </span>
                      )}
                    </td>
                    <td className="py-4 px-4">
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-foreground">{deal.productName as string}</span>
                        <ProductTypeBadge type={deal.productType as ProductType} />
                      </div>
                    </td>
                    <td className="py-4 px-4">
                      {licenseKey ? (
                        <button
                          onClick={() => copyKey(licenseKey)}
                          className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary hover:bg-secondary/80 text-xs font-mono text-muted-foreground hover:text-foreground transition-colors"
                        >
                          <KeyRound className="w-3 h-3 shrink-0" />
                          <span className="truncate max-w-[120px]">{licenseKey}</span>
                          {copiedKey === licenseKey ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
                        </button>
                      ) : (
                        <span className="text-sm text-muted-foreground/50">--</span>
                      )}
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm text-muted-foreground">{deal.date as string}</span>
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm text-muted-foreground truncate max-w-[200px] block">
                        {(deal.notes as string) || "--"}
                      </span>
                    </td>
                  </tr>
                );
              })}
              {filtered.length === 0 && (
                <tr>
                  <td colSpan={6} className="py-12 text-center text-muted-foreground">
                    <FlaskConical className="w-8 h-8 mx-auto mb-3 opacity-50" />
                    <p className="text-sm">No trials or partner grants found</p>
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-secondary/30">
          <span className="text-sm text-muted-foreground">
            Showing {filtered.length} of {trialDeals.length} trials & partners
          </span>
        </div>
      </div>
    </div>
  );
}
