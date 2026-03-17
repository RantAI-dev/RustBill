"use client";

import { MetricCard } from "@/components/dashboard/metric-card";
import { useSubscriptions, useInvoices, usePayments } from "@/hooks/use-api";
import { DollarSign, RefreshCw, FileText, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";
import { Skeleton } from "@/components/ui/skeleton";
import {
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
} from "recharts";

const invoiceStatusColors: Record<string, string> = {
  draft: "bg-muted-foreground/20 text-muted-foreground",
  issued: "bg-blue-500/20 text-blue-400",
  paid: "bg-sky-500/20 text-sky-400",
  overdue: "bg-red-500/20 text-red-400",
  void: "bg-zinc-500/20 text-zinc-400",
};

const pieColors = ["hsl(var(--chart-1))", "hsl(var(--chart-2))", "hsl(var(--chart-3))", "hsl(var(--chart-4))", "hsl(var(--chart-5))"];

function formatCurrency(value: number) {
  if (value >= 1_000_000) return `$${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `$${(value / 1_000).toFixed(1)}k`;
  return `$${value.toFixed(0)}`;
}

export function BillingSection() {
  const { data: subs, isLoading: subsLoading } = useSubscriptions();
  const { data: invs, isLoading: invsLoading } = useInvoices();
  const { data: pmts, isLoading: pmtsLoading } = usePayments();

  const isLoading = subsLoading || invsLoading || pmtsLoading;

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {[0, 1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-28 rounded-xl" />
          ))}
        </div>
        <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
          <Skeleton className="lg:col-span-2 h-72 rounded-xl" />
          <Skeleton className="h-72 rounded-xl" />
        </div>
        <Skeleton className="h-64 rounded-xl" />
      </div>
    );
  }

  const subscriptions = subs || [];
  const invoiceList = invs || [];
  const paymentList = pmts || [];

  // Compute metrics
  const activeSubs = subscriptions.filter((s: { status: string }) => s.status === "active");

  // MRR = sum of all active subscription plan base prices (normalized to monthly)
  const mrr = activeSubs.reduce((sum: number, s: { planBasePrice: number | null; planBillingCycle: string | null; quantity: number }) => {
    const base = s.planBasePrice ?? 0;
    const qty = s.quantity ?? 1;
    const cycle = s.planBillingCycle;
    let monthly = base * qty;
    if (cycle === "quarterly") monthly = monthly / 3;
    if (cycle === "yearly") monthly = monthly / 12;
    return sum + monthly;
  }, 0);

  const totalCollected = paymentList.reduce((sum: number, p: { amount: number }) => sum + p.amount, 0);
  const outstandingInvoices = invoiceList.filter((i: { status: string }) => i.status === "issued" || i.status === "overdue");
  const overdueInvoices = invoiceList.filter((i: { status: string }) => i.status === "overdue");

  // Invoice status breakdown for pie chart
  const statusCounts = invoiceList.reduce((acc: Record<string, number>, i: { status: string }) => {
    acc[i.status] = (acc[i.status] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);

  const pieData = Object.entries(statusCounts).map(([name, value]) => ({ name, value }));

  // Revenue by month (from payments)
  const monthlyRevenue = paymentList.reduce((acc: Record<string, number>, p: { paidAt: string; amount: number }) => {
    const d = new Date(p.paidAt);
    const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}`;
    acc[key] = (acc[key] || 0) + p.amount;
    return acc;
  }, {} as Record<string, number>);

  const barData = Object.entries(monthlyRevenue)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([month, revenue]) => ({ month, revenue }));

  // Recent invoices (last 10)
  const recentInvoices = invoiceList.slice(0, 10);

  return (
    <div className="space-y-6">
      {/* Metric cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          title="Monthly Recurring Revenue"
          value={formatCurrency(mrr)}
          change={`${activeSubs.length} active subs`}
          changeType="neutral"
          icon={DollarSign}
          delay={0}
        />
        <MetricCard
          title="Active Subscriptions"
          value={String(activeSubs.length)}
          change={`${subscriptions.length} total`}
          changeType="neutral"
          icon={RefreshCw}
          delay={1}
        />
        <MetricCard
          title="Outstanding Invoices"
          value={String(outstandingInvoices.length)}
          change={formatCurrency(outstandingInvoices.reduce((s: number, i: { total: number }) => s + i.total, 0))}
          changeType={overdueInvoices.length > 0 ? "negative" : "neutral"}
          icon={FileText}
          delay={2}
        />
        <MetricCard
          title="Revenue Collected"
          value={formatCurrency(totalCollected)}
          change={`${paymentList.length} payments`}
          changeType="positive"
          icon={DollarSign}
          delay={3}
        />
      </div>

      {/* Charts row */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Revenue bar chart */}
        <div className="lg:col-span-2 bg-card border border-border rounded-xl p-5">
          <h3 className="text-sm font-semibold text-foreground mb-4">Revenue by Month</h3>
          {barData.length > 0 ? (
            <ResponsiveContainer width="100%" height={240}>
              <BarChart data={barData}>
                <CartesianGrid strokeDasharray="3 3" stroke="hsl(var(--border))" />
                <XAxis dataKey="month" tick={{ fontSize: 12, fill: "hsl(var(--muted-foreground))" }} />
                <YAxis tick={{ fontSize: 12, fill: "hsl(var(--muted-foreground))" }} tickFormatter={(v) => `$${(v / 1000).toFixed(0)}k`} />
                <Tooltip
                  contentStyle={{ backgroundColor: "hsl(var(--card))", border: "1px solid hsl(var(--border))", borderRadius: 8 }}
                  labelStyle={{ color: "hsl(var(--foreground))" }}
                  formatter={(value: number) => [`$${value.toLocaleString()}`, "Revenue"]}
                />
                <Bar dataKey="revenue" fill="hsl(var(--accent))" radius={[4, 4, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-60 flex items-center justify-center text-muted-foreground text-sm">No payment data yet</div>
          )}
        </div>

        {/* Invoice status pie chart */}
        <div className="bg-card border border-border rounded-xl p-5">
          <h3 className="text-sm font-semibold text-foreground mb-4">Invoice Status</h3>
          {pieData.length > 0 ? (
            <ResponsiveContainer width="100%" height={240}>
              <PieChart>
                <Pie data={pieData} cx="50%" cy="50%" innerRadius={50} outerRadius={80} paddingAngle={4} dataKey="value" nameKey="name">
                  {pieData.map((_, i) => (
                    <Cell key={i} fill={pieColors[i % pieColors.length]} />
                  ))}
                </Pie>
                <Tooltip
                  contentStyle={{ backgroundColor: "hsl(var(--card))", border: "1px solid hsl(var(--border))", borderRadius: 8 }}
                  labelStyle={{ color: "hsl(var(--foreground))" }}
                />
              </PieChart>
            </ResponsiveContainer>
          ) : (
            <div className="h-60 flex items-center justify-center text-muted-foreground text-sm">No invoices yet</div>
          )}
          {/* Legend */}
          <div className="flex flex-wrap gap-3 mt-2">
            {pieData.map((d, i) => (
              <div key={d.name} className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: pieColors[i % pieColors.length] }} />
                <span className="capitalize">{d.name}</span>
                <span className="font-medium text-foreground">{d.value as number}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Overdue alerts */}
      {overdueInvoices.length > 0 && (
        <div className="bg-red-500/10 border border-red-500/30 rounded-xl p-4 flex items-start gap-3">
          <AlertTriangle className="w-5 h-5 text-red-400 mt-0.5 shrink-0" />
          <div>
            <p className="text-sm font-medium text-foreground">
              {overdueInvoices.length} overdue invoice{overdueInvoices.length > 1 ? "s" : ""}
            </p>
            <p className="text-xs text-muted-foreground mt-0.5">
              Total outstanding: {formatCurrency(overdueInvoices.reduce((s: number, i: { total: number }) => s + i.total, 0))}
            </p>
          </div>
        </div>
      )}

      {/* Recent invoices table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <div className="px-5 py-4 border-b border-border">
          <h3 className="text-sm font-semibold text-foreground">Recent Invoices</h3>
        </div>
        {recentInvoices.length > 0 ? (
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-left">
                  <th className="px-5 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">Invoice</th>
                  <th className="px-5 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">Customer</th>
                  <th className="px-5 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">Status</th>
                  <th className="px-5 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider text-right">Total</th>
                  <th className="px-5 py-3 text-xs font-medium text-muted-foreground uppercase tracking-wider">Due Date</th>
                </tr>
              </thead>
              <tbody>
                {recentInvoices.map((inv: { id: string; invoiceNumber: string; customerName: string | null; status: string; total: number; dueAt: string | null }) => (
                  <tr key={inv.id} className="border-b border-border/50 hover:bg-secondary/30 transition-colors">
                    <td className="px-5 py-3 font-mono text-xs text-foreground">{inv.invoiceNumber}</td>
                    <td className="px-5 py-3 text-foreground">{inv.customerName ?? "—"}</td>
                    <td className="px-5 py-3">
                      <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize", invoiceStatusColors[inv.status] ?? "bg-secondary text-muted-foreground")}>
                        {inv.status}
                      </span>
                    </td>
                    <td className="px-5 py-3 text-right font-medium text-foreground">${inv.total.toLocaleString()}</td>
                    <td className="px-5 py-3 text-muted-foreground">{inv.dueAt ? new Date(inv.dueAt).toLocaleDateString() : "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <div className="px-5 py-10 text-center text-muted-foreground text-sm">No invoices yet</div>
        )}
      </div>
    </div>
  );
}
