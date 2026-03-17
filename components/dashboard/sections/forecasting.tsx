"use client";

import { useState } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  TrendingUp,
  TrendingDown,
  Target,
  AlertTriangle,
  CheckCircle2,
  ArrowRight,
  RefreshCw,
} from "lucide-react";
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  BarChart,
  Bar,
  Legend,
} from "recharts";
import { useForecastAnalytics } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

export function ForecastingSection() {
  const [timeframe, setTimeframe] = useState("quarterly");
  const { data, isLoading, mutate } = useForecastAnalytics();

  const forecastData = data?.forecastData ?? [];
  const quarterlyForecast = data?.quarterlyForecast ?? [];
  const riskFactors = data?.riskFactors ?? [];
  const scenarios = data?.scenarios ?? [];
  const kpis = data?.kpis ?? { currentQuarterForecast: 0, quarterTarget: 0, forecastAccuracy: 0, dealCoverage: 0, atRiskRevenue: 0 };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="flex justify-between">
          <Skeleton className="h-8 w-64" />
          <Skeleton className="h-9 w-48" />
        </div>
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-[100px] rounded-xl" />)}
        </div>
        <Skeleton className="h-[350px] rounded-xl" />
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <Skeleton className="h-[300px] rounded-xl" />
          <Skeleton className="h-[300px] rounded-xl" />
        </div>
      </div>
    );
  }

  const formatCurrency = (n: number) => {
    if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `$${Math.round(n / 1_000)}K`;
    return `$${n}`;
  };

  return (
    <div className="space-y-6">
      {/* Header Controls */}
      <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-foreground">Revenue Forecasting</h2>
          <p className="text-sm text-muted-foreground mt-1">
            Predictions across Licensed, Platform, and API products
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Select value={timeframe} onValueChange={setTimeframe}>
            <SelectTrigger className="w-[140px] bg-secondary border-border">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="monthly">Monthly</SelectItem>
              <SelectItem value="quarterly">Quarterly</SelectItem>
              <SelectItem value="annual">Annual</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={() => mutate()}>
            <RefreshCw className="w-4 h-4 mr-2" />
            Refresh
          </Button>
        </div>
      </div>

      {/* KPI Summary */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {[
          {
            label: "Q Forecast",
            value: formatCurrency(kpis.currentQuarterForecast),
            subtext: `Target: ${formatCurrency(kpis.quarterTarget)}`,
            icon: Target,
            trend: kpis.currentQuarterForecast > kpis.quarterTarget ? `+${Math.round(((kpis.currentQuarterForecast - kpis.quarterTarget) / (kpis.quarterTarget || 1)) * 100)}%` : "On track",
            trendUp: kpis.currentQuarterForecast >= kpis.quarterTarget,
          },
          {
            label: "Forecast Accuracy",
            value: `${kpis.forecastAccuracy}%`,
            subtext: "Based on historical data",
            icon: CheckCircle2,
            trend: kpis.forecastAccuracy > 0 ? `${kpis.forecastAccuracy}%` : "N/A",
            trendUp: kpis.forecastAccuracy >= 80,
          },
          {
            label: "Deal Coverage",
            value: `${kpis.dealCoverage}x`,
            subtext: "vs quota",
            icon: TrendingUp,
            trend: kpis.dealCoverage >= 1 ? "Healthy" : "Low",
            trendUp: kpis.dealCoverage >= 1,
          },
          {
            label: "At-Risk Revenue",
            value: formatCurrency(kpis.atRiskRevenue),
            subtext: `${riskFactors.length} factor(s) flagged`,
            icon: AlertTriangle,
            trend: riskFactors.length > 0 ? `${riskFactors.length} risks` : "None",
            trendUp: riskFactors.length === 0,
          },
        ].map((stat, index) => (
          <Card
            key={stat.label}
            className="border-border bg-card transition-all duration-500 animate-in fade-in slide-in-from-bottom-2"
            style={{ animationDelay: `${index * 100}ms` }}
          >
            <CardContent className="p-4">
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm text-muted-foreground">{stat.label}</p>
                  <p className="text-2xl font-semibold text-foreground mt-1">{stat.value}</p>
                  <p className="text-xs text-muted-foreground mt-0.5">{stat.subtext}</p>
                </div>
                <div className="flex flex-col items-end gap-2">
                  <stat.icon
                    className={`w-5 h-5 ${
                      stat.label === "At-Risk Revenue" ? "text-chart-3" : "text-accent"
                    }`}
                  />
                  <Badge
                    variant="outline"
                    className={`text-xs ${
                      stat.trendUp
                        ? "text-accent border-accent/30"
                        : "text-destructive border-destructive/30"
                    }`}
                  >
                    {stat.trendUp ? (
                      <TrendingUp className="w-3 h-3 mr-1" />
                    ) : (
                      <TrendingDown className="w-3 h-3 mr-1" />
                    )}
                    {stat.trend}
                  </Badge>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Main Chart */}
      <Card className="border-border bg-card">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-base font-medium">Revenue Forecast vs Actual</CardTitle>
            <div className="flex items-center gap-4 text-xs">
              <div className="flex items-center gap-1.5">
                <div className="w-3 h-3 rounded-full bg-accent" />
                <span className="text-muted-foreground">Actual</span>
              </div>
              <div className="flex items-center gap-1.5">
                <div className="w-3 h-3 rounded-full bg-chart-1" />
                <span className="text-muted-foreground">Forecast</span>
              </div>
              <div className="flex items-center gap-1.5">
                <div className="w-3 h-3 rounded-full bg-muted-foreground/30" />
                <span className="text-muted-foreground">Target</span>
              </div>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="h-[300px]">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={forecastData}>
                <defs>
                  <linearGradient id="actualGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="oklch(0.75 0.130 243)" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="oklch(0.75 0.130 243)" stopOpacity={0} />
                  </linearGradient>
                  <linearGradient id="forecastGradient" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="5%" stopColor="oklch(0.7 0.18 220)" stopOpacity={0.3} />
                    <stop offset="95%" stopColor="oklch(0.7 0.18 220)" stopOpacity={0} />
                  </linearGradient>
                </defs>
                <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.22 0.005 260)" />
                <XAxis dataKey="month" stroke="oklch(0.65 0 0)" fontSize={12} />
                <YAxis
                  stroke="oklch(0.65 0 0)"
                  fontSize={12}
                  tickFormatter={(value) => `$${value / 1000}K`}
                />
                <Tooltip
                  contentStyle={{
                    backgroundColor: "oklch(0.12 0.005 260)",
                    border: "1px solid oklch(0.22 0.005 260)",
                    borderRadius: "8px",
                    color: "oklch(0.95 0 0)",
                  }}
                  formatter={(value) => [typeof value === "number" ? `$${value.toLocaleString()}` : "—", ""]}
                />
                <Area
                  type="monotone"
                  dataKey="target"
                  stroke="oklch(0.65 0 0)"
                  strokeDasharray="5 5"
                  fill="none"
                  strokeWidth={1}
                />
                <Area
                  type="monotone"
                  dataKey="forecast"
                  stroke="oklch(0.7 0.18 220)"
                  fill="url(#forecastGradient)"
                  strokeWidth={2}
                />
                <Area
                  type="monotone"
                  dataKey="actual"
                  stroke="oklch(0.75 0.130 243)"
                  fill="url(#actualGradient)"
                  strokeWidth={2}
                  connectNulls={false}
                />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Quarterly Forecast Breakdown */}
        <Card className="border-border bg-card">
          <CardHeader className="pb-2">
            <CardTitle className="text-base font-medium">Quarterly Forecast Breakdown</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-[250px]">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={quarterlyForecast} barGap={4}>
                  <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.22 0.005 260)" />
                  <XAxis dataKey="quarter" stroke="oklch(0.65 0 0)" fontSize={12} />
                  <YAxis
                    stroke="oklch(0.65 0 0)"
                    fontSize={12}
                    tickFormatter={(value) => value >= 1000000 ? `$${value / 1000000}M` : `$${value / 1000}K`}
                  />
                  <Tooltip
                    contentStyle={{
                      backgroundColor: "oklch(0.12 0.005 260)",
                      border: "1px solid oklch(0.22 0.005 260)",
                      borderRadius: "8px",
                      color: "oklch(0.95 0 0)",
                    }}
                    formatter={(value: number) => [`$${value.toLocaleString()}`, ""]}
                  />
                  <Legend
                    wrapperStyle={{ fontSize: "12px" }}
                    formatter={(value) => (
                      <span style={{ color: "oklch(0.65 0 0)" }}>{value}</span>
                    )}
                  />
                  <Bar dataKey="committed" name="Committed" fill="oklch(0.75 0.130 243)" radius={[4, 4, 0, 0]} />
                  <Bar dataKey="bestCase" name="Best Case" fill="oklch(0.7 0.18 220)" radius={[4, 4, 0, 0]} />
                  <Bar dataKey="projected" name="Projected" fill="oklch(0.22 0.005 260)" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Scenario Analysis */}
        <Card className="border-border bg-card">
          <CardHeader className="pb-2">
            <CardTitle className="text-base font-medium">Scenario Analysis</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            {scenarios.map((scenario: { name: string; probability: number; revenue: number; color: string }, index: number) => (
              <div
                key={scenario.name}
                className="p-4 rounded-lg bg-secondary/50 border border-border hover:border-muted-foreground/30 transition-all duration-300 animate-in fade-in slide-in-from-right-2"
                style={{ animationDelay: `${index * 100}ms` }}
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div
                      className="w-2 h-8 rounded-full"
                      style={{
                        backgroundColor:
                          scenario.color === "accent"
                            ? "oklch(0.75 0.130 243)"
                            : scenario.color === "chart-1"
                            ? "oklch(0.7 0.18 220)"
                            : "oklch(0.65 0.2 25)",
                      }}
                    />
                    <div>
                      <p className="font-medium text-foreground">{scenario.name}</p>
                      <p className="text-xs text-muted-foreground">
                        {scenario.probability}% probability
                      </p>
                    </div>
                  </div>
                  <p className="text-xl font-semibold text-foreground">
                    ${(scenario.revenue / 1000000).toFixed(1)}M
                  </p>
                </div>
                <div className="w-full h-2 bg-secondary rounded-full overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all duration-1000 ease-out"
                    style={{
                      width: `${scenario.probability}%`,
                      backgroundColor:
                        scenario.color === "accent"
                          ? "oklch(0.75 0.130 243)"
                          : scenario.color === "chart-1"
                          ? "oklch(0.7 0.18 220)"
                          : "oklch(0.65 0.2 25)",
                    }}
                  />
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      {/* Risk Factors */}
      <Card className="border-border bg-card">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-base font-medium">Risk Factors</CardTitle>
            <Badge variant="outline" className="text-chart-3 border-chart-3/30">
              <AlertTriangle className="w-3 h-3 mr-1" />
              {riskFactors.length} identified
            </Badge>
          </div>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {riskFactors.map((risk: { id: string; title: string; description: string; impact: string; severity: string; deals: string[] }, index: number) => (
              <div
                key={risk.id}
                className="p-4 rounded-lg bg-secondary/50 border border-border hover:border-chart-3/30 transition-all duration-300 group animate-in fade-in slide-in-from-bottom-2"
                style={{ animationDelay: `${index * 75}ms` }}
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-start gap-3">
                    <div
                      className={`w-2 h-2 rounded-full mt-2 ${
                        risk.severity === "high" ? "bg-destructive" : "bg-chart-3"
                      }`}
                    />
                    <div>
                      <p className="font-medium text-foreground">{risk.title}</p>
                      <p className="text-sm text-muted-foreground">{risk.description}</p>
                    </div>
                  </div>
                  <Badge
                    className={
                      risk.severity === "high"
                        ? "bg-destructive/20 text-destructive border-destructive/30"
                        : "bg-chart-3/20 text-chart-3 border-chart-3/30"
                    }
                  >
                    {risk.impact}
                  </Badge>
                </div>
                {risk.deals.length > 0 && (
                  <div className="ml-5 flex items-center gap-2 flex-wrap">
                    {risk.deals.map((deal: string) => (
                      <Badge
                        key={deal}
                        variant="outline"
                        className="text-xs text-muted-foreground border-border"
                      >
                        {deal}
                      </Badge>
                    ))}
                  </div>
                )}
                <div className="ml-5 mt-3">
                  <Button
                    variant="ghost"
                    size="sm"
                    className="text-xs text-muted-foreground hover:text-foreground p-0 h-auto"
                  >
                    View details
                    <ArrowRight className="w-3 h-3 ml-1" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
