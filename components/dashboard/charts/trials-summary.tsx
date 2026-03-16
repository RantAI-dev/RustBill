"use client";

import { FlaskConical, Handshake, KeyRound, Clock } from "lucide-react";
import { useDeals } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

export function TrialsSummary() {
  const { data: allDeals, isLoading } = useDeals();

  if (isLoading) {
    return (
      <div className="bg-card border border-border rounded-xl p-5">
        <Skeleton className="h-6 w-40 mb-4" />
        <div className="space-y-3">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-14 rounded-lg" />)}
        </div>
      </div>
    );
  }

  const deals = (allDeals ?? []) as Record<string, unknown>[];
  const trials = deals.filter((d) => d.dealType === "trial");
  const partners = deals.filter((d) => d.dealType === "partner");
  const activeTrials = trials.length;
  const activePartners = partners.length;
  const totalTrialPartner = activeTrials + activePartners;
  const withLicense = deals.filter((d) => (d.dealType === "trial" || d.dealType === "partner") && d.licenseKey).length;

  const stats = [
    { label: "Active Trials", value: activeTrials, icon: FlaskConical, color: "text-chart-3" },
    { label: "Partner Grants", value: activePartners, icon: Handshake, color: "text-chart-1" },
    { label: "Total Active", value: totalTrialPartner, icon: Clock, color: "text-accent" },
    { label: "With License", value: withLicense, icon: KeyRound, color: "text-chart-5" },
  ];

  return (
    <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-100">
      <h3 className="text-base font-semibold text-foreground mb-4">Trials & Partners</h3>
      <div className="space-y-3">
        {stats.map((stat, index) => (
          <div
            key={stat.label}
            className="flex items-center justify-between p-3 rounded-lg bg-secondary/50 animate-in fade-in slide-in-from-left-2"
            style={{ animationDelay: `${(index + 1) * 100}ms`, animationFillMode: "both" }}
          >
            <div className="flex items-center gap-3">
              <div className="w-9 h-9 rounded-lg bg-secondary flex items-center justify-center">
                <stat.icon className={`w-4 h-4 ${stat.color}`} />
              </div>
              <span className="text-sm text-muted-foreground">{stat.label}</span>
            </div>
            <span className="text-lg font-semibold text-foreground">{stat.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
