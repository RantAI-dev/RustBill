"use client";

import { MetricCard } from "@/components/dashboard/metric-card";
import { RevenueChart } from "@/components/dashboard/charts/revenue-chart";
import { TrialsSummary } from "@/components/dashboard/charts/trials-summary";
import { RecentDeals } from "@/components/dashboard/recent-deals";
import { TopPerformers } from "@/components/dashboard/top-performers";
import { DollarSign, Users, KeyRound, UserPlus } from "lucide-react";
import { useOverviewAnalytics } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

export function OverviewSection() {
  const { data, isLoading } = useOverviewAnalytics();

  return (
    <div className="space-y-6">
      {/* Metric cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {isLoading ? (
          <>
            {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-[106px] rounded-xl" />)}
          </>
        ) : (
          <>
            <MetricCard
              title="Total Revenue"
              value={data?.totalRevenue ?? "$0"}
              change={data?.revenueChange ?? "+0%"}
              changeType="positive"
              icon={DollarSign}
              delay={0}
            />
            <MetricCard
              title="Platform Users"
              value={data?.platformUsers ?? "0"}
              change={data?.platformUsersChange ?? "+0%"}
              changeType="positive"
              icon={Users}
              delay={1}
            />
            <MetricCard
              title="Active Licenses"
              value={data?.activeLicenses ?? "0"}
              change={data?.licensesChange ?? "+0"}
              changeType="positive"
              icon={KeyRound}
              delay={2}
            />
            <MetricCard
              title="Customers"
              value={data?.customerCount ?? "0"}
              change={data?.newCustomers ?? "0 total"}
              changeType="positive"
              icon={UserPlus}
              delay={3}
            />
          </>
        )}
      </div>

      {/* Charts row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2">
          <RevenueChart data={data?.revenueChart} isLoading={isLoading} />
        </div>
        <TrialsSummary />
      </div>

      {/* Bottom row */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <RecentDeals />
        <TopPerformers />
      </div>
    </div>
  );
}
