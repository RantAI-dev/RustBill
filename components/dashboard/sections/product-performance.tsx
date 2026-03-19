"use client";

import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import { Package, DollarSign, TrendingUp, TrendingDown, KeyRound, Users, Zap } from "lucide-react";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useProducts } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";

/* ---------- data types ---------- */

interface BaseProduct {
  id: string;
  name: string;
  productType: ProductType;
  revenue: number;
  target: number;
  change: number;
}

interface LicensedProduct extends BaseProduct {
  productType: "licensed";
  unitsSold: number;
  activeLicenses: number;
  totalLicenses: number;
}

interface SaasProduct extends BaseProduct {
  productType: "saas";
  mau: number;
  dau: number;
  freeUsers: number;
  paidUsers: number;
  churnRate: number;
}

interface ApiProduct extends BaseProduct {
  productType: "api";
  apiCalls: number;
  activeDevelopers: number;
  avgLatency: number;
}

type Product = LicensedProduct | SaasProduct | ApiProduct;

/* ---------- product card ---------- */

function ProductCard({ product, index }: { product: Product; index: number }) {
  const targetPercentage = (product.revenue / product.target) * 100;
  const isAboveTarget = targetPercentage >= 100;

  return (
    <div
      className="group bg-card border border-border rounded-xl p-5 hover:border-accent/50 transition-all duration-300 animate-in fade-in slide-in-from-bottom-4"
      style={{ animationDelay: `${index * 100}ms`, animationFillMode: "both" }}
    >
      <div className="flex items-start justify-between mb-4">
        <div className="flex items-center gap-3">
          <div className="w-12 h-12 rounded-lg bg-secondary flex items-center justify-center group-hover:bg-accent/10 transition-colors">
            <Package className="w-5 h-5 text-muted-foreground group-hover:text-accent transition-colors" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <h4 className="text-sm font-semibold text-foreground">{product.name}</h4>
              <ProductTypeBadge type={product.productType} />
            </div>
            <p className="text-xs text-muted-foreground mt-0.5">
              {product.productType === "licensed" && `${product.unitsSold} units sold`}
              {product.productType === "saas" && `${formatUsageMetric(product.mau)} MAU`}
              {product.productType === "api" && `${formatUsageMetric(product.apiCalls)} calls/mo`}
            </p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <p className="text-xs text-muted-foreground mb-1">Revenue</p>
          <p className="text-lg font-bold text-foreground">${(product.revenue / 1000).toFixed(0)}k</p>
        </div>
        {product.productType === "licensed" && (
          <div>
            <p className="text-xs text-muted-foreground mb-1">Active Licenses</p>
            <div className="flex items-center gap-1.5">
              <p className="text-lg font-bold text-foreground">{product.activeLicenses}</p>
              <span className="text-xs text-muted-foreground">/ {product.totalLicenses}</span>
            </div>
          </div>
        )}
        {product.productType === "saas" && (
          <div>
            <p className="text-xs text-muted-foreground mb-1">Paid Users</p>
            <div className="flex items-center gap-1.5">
              <p className="text-lg font-bold text-foreground">{formatUsageMetric(product.paidUsers)}</p>
              <span className="text-xs text-muted-foreground">/ {formatUsageMetric(product.mau)}</span>
            </div>
          </div>
        )}
        {product.productType === "api" && (
          <div>
            <p className="text-xs text-muted-foreground mb-1">Active Developers</p>
            <p className="text-lg font-bold text-foreground">{product.activeDevelopers}</p>
          </div>
        )}
      </div>

      <div className="mb-4">
        <div className="flex items-center justify-between text-xs mb-1.5">
          <span className="text-muted-foreground">Revenue vs Target</span>
          <span className={cn("font-medium", isAboveTarget ? "text-success" : "text-foreground")}>{targetPercentage.toFixed(0)}%</span>
        </div>
        <div className="h-2 bg-secondary rounded-full overflow-hidden">
          <div className={cn("h-full rounded-full transition-all duration-700", isAboveTarget ? "bg-success" : "bg-accent")} style={{ width: `${Math.min(targetPercentage, 100)}%` }} />
        </div>
      </div>

      <div className="flex items-center justify-between pt-4 border-t border-border">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          {product.productType === "licensed" && (<><KeyRound className="w-3.5 h-3.5" /><span>{Math.round((product.activeLicenses / product.totalLicenses) * 100)}% utilization</span></>)}
          {product.productType === "saas" && (<><Users className="w-3.5 h-3.5" /><span>{product.churnRate}% churn rate</span></>)}
          {product.productType === "api" && (<><Zap className="w-3.5 h-3.5" /><span>{product.avgLatency}ms avg latency</span></>)}
        </div>
        <div className={cn("flex items-center gap-1 text-sm font-medium", product.change >= 0 ? "text-success" : "text-destructive")}>
          {product.change >= 0 ? <TrendingUp className="w-4 h-4" /> : <TrendingDown className="w-4 h-4" />}
          {product.change >= 0 ? "+" : ""}{product.change}%
        </div>
      </div>
    </div>
  );
}

/* ---------- main section ---------- */

export function ProductPerformanceSection() {
  const { data: products, error, isLoading } = useProducts();
  const [chartLoaded, setChartLoaded] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => setChartLoaded(true), 400);
    return () => clearTimeout(timer);
  }, []);

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-28 rounded-xl" />)}
        </div>
        <Skeleton className="h-[340px] rounded-xl" />
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {[...Array(6)].map((_, i) => <Skeleton key={i} className="h-64 rounded-xl" />)}
        </div>
      </div>
    );
  }

  if (error) return <p className="text-destructive">Failed to load products.</p>;
  if (!products) return null;

  const typedProducts = products as unknown as Product[];
  const chartData = typedProducts.map((p) => ({
    name: p.name.replace(" Plan", "").replace("AI Chat ", ""),
    revenue: Math.round(p.revenue / 1000),
    target: Math.round(p.target / 1000),
  }));

  const totalRevenue = typedProducts.reduce((acc, p) => acc + p.revenue, 0);
  const licensedProducts = typedProducts.filter((p): p is LicensedProduct => p.productType === "licensed");
  const totalActiveLicenses = licensedProducts.reduce((acc, p) => acc + (p.activeLicenses ?? 0), 0);
  const saasProduct = typedProducts.find((p): p is SaasProduct => p.productType === "saas");
  const apiProduct = typedProducts.find((p): p is ApiProduct => p.productType === "api");

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500">
          <div className="flex items-center gap-3 mb-2"><div className="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center"><DollarSign className="w-5 h-5 text-accent" /></div><span className="text-sm text-muted-foreground">Total Revenue</span></div>
          <p className="text-2xl font-bold text-foreground">${(totalRevenue / 1000000).toFixed(2)}M</p>
        </div>
        <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-100">
          <div className="flex items-center gap-3 mb-2"><div className="w-10 h-10 rounded-lg bg-chart-1/10 flex items-center justify-center"><KeyRound className="w-5 h-5 text-chart-1" /></div><span className="text-sm text-muted-foreground">Active Licenses</span></div>
          <p className="text-2xl font-bold text-foreground">{totalActiveLicenses}</p>
        </div>
        <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-200">
          <div className="flex items-center gap-3 mb-2"><div className="w-10 h-10 rounded-lg bg-chart-3/10 flex items-center justify-center"><Users className="w-5 h-5 text-chart-3" /></div><span className="text-sm text-muted-foreground">Platform MAU</span></div>
          <p className="text-2xl font-bold text-foreground">{saasProduct ? formatUsageMetric(saasProduct.mau) : "--"}</p>
        </div>
        <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-300">
          <div className="flex items-center gap-3 mb-2"><div className="w-10 h-10 rounded-lg bg-chart-5/10 flex items-center justify-center"><Zap className="w-5 h-5 text-chart-5" /></div><span className="text-sm text-muted-foreground">API Calls / mo</span></div>
          <p className="text-2xl font-bold text-foreground">{apiProduct ? formatUsageMetric(apiProduct.apiCalls) : "--"}</p>
        </div>
      </div>

      <div className="bg-card border border-border rounded-xl p-5 animate-in fade-in slide-in-from-bottom-4 duration-500 delay-150">
        <div className="flex items-center justify-between mb-6">
          <div><h3 className="text-base font-semibold text-foreground">Revenue by Product</h3><p className="text-sm text-muted-foreground mt-0.5">All product types &middot; revenue vs target</p></div>
          <div className="flex items-center gap-4 text-xs">
            <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-chart-1" /><span className="text-muted-foreground">Revenue (k)</span></div>
            <div className="flex items-center gap-1.5"><div className="w-2.5 h-2.5 rounded-full bg-muted-foreground/30" /><span className="text-muted-foreground">Target (k)</span></div>
          </div>
        </div>
        <div className={`h-[280px] transition-opacity duration-700 ${chartLoaded ? "opacity-100" : "opacity-0"}`}>
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chartData} margin={{ top: 10, right: 10, left: 0, bottom: 0 }}>
              <CartesianGrid strokeDasharray="3 3" stroke="oklch(0.22 0.005 260)" vertical={false} />
              <XAxis dataKey="name" axisLine={false} tickLine={false} tick={{ fill: "oklch(0.65 0 0)", fontSize: 12 }} dy={10} />
              <YAxis axisLine={false} tickLine={false} tick={{ fill: "oklch(0.65 0 0)", fontSize: 12 }} tickFormatter={(value) => `$${value}k`} dx={-10} />
              <Tooltip contentStyle={{ backgroundColor: "oklch(0.12 0.005 260)", border: "1px solid oklch(0.22 0.005 260)", borderRadius: "8px", fontSize: "12px" }} labelStyle={{ color: "oklch(0.95 0 0)", fontWeight: 600 }} itemStyle={{ color: "oklch(0.65 0 0)" }} formatter={(value: number) => [`$${value}k`, ""]} />
              <Bar dataKey="target" fill="oklch(0.65 0 0 / 0.2)" radius={[4, 4, 0, 0]} />
              <Bar dataKey="revenue" fill="oklch(0.7 0.18 220)" radius={[4, 4, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>

      <div>
        <h3 className="text-base font-semibold text-foreground mb-4">All Products</h3>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {typedProducts.map((product, index) => (
            <ProductCard key={product.id} product={product} index={index} />
          ))}
        </div>
      </div>
    </div>
  );
}
