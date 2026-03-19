"use client";

import { useMemo, useState } from "react";
import {
  useSales360Breakdown,
  runSales360Backfill,
  useSales360Reconcile,
  useSales360Summary,
  useSales360Timeseries,
} from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { RefreshCw } from "lucide-react";
import { toast } from "sonner";
import {
  ResponsiveContainer,
  LineChart,
  Line,
  CartesianGrid,
  XAxis,
  YAxis,
  Tooltip,
} from "recharts";

const CLASSIFICATIONS = [
  { key: "bookings", label: "Bookings" },
  { key: "billings", label: "Billings" },
  { key: "collections", label: "Collections" },
  { key: "adjustments", label: "Adjustments" },
  { key: "recurring", label: "Recurring" },
] as const;

type RangePreset = "7d" | "30d" | "90d" | "custom";

function formatDateISO(d: Date): string {
  return d.toISOString().slice(0, 10);
}

function computeRange(preset: Exclude<RangePreset, "custom">): { from: string; to: string } {
  const to = new Date();
  const from = new Date(to);
  if (preset === "7d") from.setDate(from.getDate() - 6);
  if (preset === "30d") from.setDate(from.getDate() - 29);
  if (preset === "90d") from.setDate(from.getDate() - 89);
  return { from: formatDateISO(from), to: formatDateISO(to) };
}

function formatCurrency(v: number): string {
  return `$${Number(v || 0).toLocaleString()}`;
}

function buildExportUrl(from?: string, to?: string, timezone?: string, currency?: string): string {
  const params = new URLSearchParams();
  if (from) params.set("from", from);
  if (to) params.set("to", to);
  if (timezone) params.set("timezone", timezone);
  if (currency) params.set("currency", currency);
  const suffix = params.toString() ? `?${params.toString()}` : "";
  return `/api/analytics/sales-360/export${suffix}`;
}

export function Sales360Section() {
  const [preset, setPreset] = useState<RangePreset>("30d");
  const [timezone, setTimezone] = useState("UTC");
  const [currency, setCurrency] = useState("ALL");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");

  const selectedCurrency = currency === "ALL" ? undefined : currency;

  const range = useMemo(() => {
    if (preset === "custom") {
      return { from: customFrom || undefined, to: customTo || undefined };
    }
    return computeRange(preset);
  }, [preset, customFrom, customTo]);

  const { data: summary, isLoading: summaryLoading, mutate: mutateSummary } = useSales360Summary(range.from, range.to, timezone, selectedCurrency);
  const { data: timeseries, isLoading: seriesLoading, mutate: mutateTimeseries } = useSales360Timeseries(range.from, range.to, timezone, selectedCurrency);
  const { data: breakdown, isLoading: breakdownLoading, mutate: mutateBreakdown } = useSales360Breakdown(range.from, range.to, timezone, selectedCurrency);
  const { data: reconcile, isLoading: reconcileLoading, mutate: mutateReconcile } = useSales360Reconcile(range.from, range.to, timezone, selectedCurrency);

  const availableCurrencies = ((summary?.availableCurrencies ?? []) as string[]).filter(Boolean);

  const summaryMap = (summary?.summary ?? {}) as Record<
    string,
    { subtotal?: number; tax?: number; total?: number }
  >;

  const chartData = ((timeseries?.data ?? []) as Record<string, unknown>[]).map((row) => ({
    day: (row.day as string) ?? "",
    bookings: Number(row.bookings ?? 0),
    billings: Number(row.billings ?? 0),
    collections: Number(row.collections ?? 0),
    adjustments: Number(row.adjustments ?? 0),
    recurring: Number(row.recurring ?? 0),
  }));

  const byEventType = (breakdown?.byEventType ?? []) as Array<{
    eventType: string;
    total: number;
  }>;

  const reconcileRows = (reconcile?.rows ?? {}) as Record<
    string,
    {
      ledgerTotal?: number;
      sourceTotal?: number;
      delta?: number;
      eventCount?: number;
      missingSources?: number;
      status?: "ok" | "drift";
    }
  >;

  if (summaryLoading || seriesLoading || breakdownLoading || reconcileLoading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
          {[...Array(5)].map((_, i) => (
            <Skeleton key={i} className="h-[90px] rounded-xl" />
          ))}
        </div>
        <Skeleton className="h-[320px] rounded-xl" />
        <Skeleton className="h-[260px] rounded-xl" />
        <Skeleton className="h-[260px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold text-foreground">Sales 360</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Unified view of bookings, billings, collections, adjustments, and recurring signals.
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <select
            className="h-9 rounded-md border border-border bg-background px-2 text-sm"
            value={preset}
            onChange={(e) => setPreset(e.target.value as RangePreset)}
          >
            <option value="7d">Last 7 days</option>
            <option value="30d">Last 30 days</option>
            <option value="90d">Last 90 days</option>
            <option value="custom">Custom</option>
          </select>

          {preset === "custom" && (
            <>
              <input
                type="date"
                className="h-9 rounded-md border border-border bg-background px-2 text-sm"
                value={customFrom}
                onChange={(e) => setCustomFrom(e.target.value)}
              />
              <input
                type="date"
                className="h-9 rounded-md border border-border bg-background px-2 text-sm"
                value={customTo}
                onChange={(e) => setCustomTo(e.target.value)}
              />
            </>
          )}

          <select
            className="h-9 rounded-md border border-border bg-background px-2 text-sm"
            value={timezone}
            onChange={(e) => setTimezone(e.target.value)}
          >
            <option value="UTC">UTC</option>
            <option value="Asia/Jakarta">Asia/Jakarta</option>
            <option value="America/Los_Angeles">America/Los_Angeles</option>
          </select>

          <select
            className="h-9 rounded-md border border-border bg-background px-2 text-sm"
            value={currency}
            onChange={(e) => setCurrency(e.target.value)}
          >
            <option value="ALL">All currencies</option>
            {availableCurrencies.map((item) => (
              <option key={item} value={item}>
                {item}
              </option>
            ))}
          </select>

          <Button
            variant="outline"
            size="sm"
            onClick={async () => {
              try {
                await runSales360Backfill();
                await Promise.all([mutateSummary(), mutateTimeseries(), mutateBreakdown(), mutateReconcile()]);
                toast.success("Sales ledger backfill completed");
              } catch (err) {
                toast.error(err instanceof Error ? err.message : "Backfill failed");
              }
            }}
          >
            <RefreshCw className="w-4 h-4 mr-2" /> Backfill Ledger
          </Button>

          <a
            href={buildExportUrl(range.from, range.to, timezone, selectedCurrency)}
            className="inline-flex items-center h-9 rounded-md border border-border bg-background px-3 text-sm hover:bg-secondary/50"
          >
            Export CSV
          </a>
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
        {CLASSIFICATIONS.map((item) => {
          const metric = summaryMap[item.key] ?? {};
          return (
            <div key={item.key} className="bg-card border border-border rounded-xl p-4">
              <p className="text-xs uppercase tracking-wide text-muted-foreground">{item.label}</p>
              <p className="text-lg font-semibold text-foreground mt-1">
                {formatCurrency(Number(metric.total ?? 0))}
              </p>
              <p className="text-[11px] text-muted-foreground mt-1">
                Net {formatCurrency(Number(metric.subtotal ?? 0))} · Tax {formatCurrency(Number(metric.tax ?? 0))}
              </p>
            </div>
          );
        })}
      </div>

      <div className="bg-card border border-border rounded-xl p-5">
        <div className="mb-4">
          <h3 className="text-base font-semibold text-foreground">Daily Sales Event Totals</h3>
          <p className="text-sm text-muted-foreground mt-0.5">
            Last 30 days grouped by classification.
          </p>
        </div>
        <div className="h-[280px]">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.22 0.005 260)" />
              <XAxis dataKey="day" tick={{ fill: "oklch(0.65 0 0)", fontSize: 12 }} />
              <YAxis tick={{ fill: "oklch(0.65 0 0)", fontSize: 12 }} />
              <Tooltip
                contentStyle={{
                  backgroundColor: "oklch(0.12 0.005 260)",
                  border: "1px solid oklch(0.22 0.005 260)",
                  borderRadius: "8px",
                  fontSize: "12px",
                }}
                formatter={(value: number) => formatCurrency(value)}
              />
              <Line type="monotone" dataKey="bookings" stroke="oklch(0.72 0.17 34)" strokeWidth={2} dot={false} />
              <Line type="monotone" dataKey="billings" stroke="oklch(0.75 0.13 243)" strokeWidth={2} dot={false} />
              <Line type="monotone" dataKey="collections" stroke="oklch(0.78 0.14 145)" strokeWidth={2} dot={false} />
              <Line type="monotone" dataKey="adjustments" stroke="oklch(0.67 0.20 25)" strokeWidth={2} dot={false} />
              <Line type="monotone" dataKey="recurring" stroke="oklch(0.70 0.16 290)" strokeWidth={2} dot={false} />
            </LineChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div className="bg-card border border-border rounded-xl p-5">
        <h3 className="text-base font-semibold text-foreground">Top Ledger Event Types</h3>
        <div className="mt-4 space-y-2">
          {byEventType.length === 0 ? (
            <p className="text-sm text-muted-foreground">No sales events available yet.</p>
          ) : (
            byEventType.slice(0, 12).map((row) => (
              <div key={row.eventType} className="flex items-center justify-between text-sm">
                <span className="text-muted-foreground">{row.eventType}</span>
                <span className="font-medium text-foreground">{formatCurrency(row.total)}</span>
              </div>
            ))
          )}
        </div>
      </div>

      <div className="bg-card border border-border rounded-xl p-5">
        <h3 className="text-base font-semibold text-foreground">Ledger Reconciliation</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Compares event totals to source records referenced by ledger entries.
        </p>
        <div className="mt-4 space-y-2">
          {CLASSIFICATIONS.map((item) => {
            const row = reconcileRows[item.key] ?? {};
            const delta = Number(row.delta ?? 0);
            const hasDrift = delta !== 0 || Number(row.missingSources ?? 0) > 0;
            return (
              <div key={item.key} className="grid grid-cols-1 md:grid-cols-6 gap-2 text-sm border border-border rounded-lg p-3">
                <div className="font-medium text-foreground">{item.label}</div>
                <div className="text-muted-foreground">Ledger {formatCurrency(Number(row.ledgerTotal ?? 0))}</div>
                <div className="text-muted-foreground">Source {formatCurrency(Number(row.sourceTotal ?? 0))}</div>
                <div className={hasDrift ? "text-amber-400" : "text-emerald-400"}>
                  Delta {formatCurrency(delta)}
                </div>
                <div className="text-muted-foreground">Events {Number(row.eventCount ?? 0)}</div>
                <div className={Number(row.missingSources ?? 0) > 0 ? "text-amber-400" : "text-muted-foreground"}>
                  Missing {Number(row.missingSources ?? 0)}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
