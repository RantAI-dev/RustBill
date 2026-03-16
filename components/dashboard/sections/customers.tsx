"use client";

import { useState } from "react";
import { appConfig } from "@/lib/app-config";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import {
  Building2,
  Search,
  MapPin,
  Mail,
  Phone,
  DollarSign,
  Calendar,
  ExternalLink,
  Star,
  TrendingUp,
  TrendingDown,
  Filter,
  KeyRound,
  Users,
  Zap,
} from "lucide-react";
import { ProductTypeBadge } from "@/components/dashboard/product-type-badge";
import { type ProductType, formatUsageMetric } from "@/lib/product-types";
import { useCustomers } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";

interface CustomerProduct {
  type: ProductType;
  name: string;
  licenseKeys?: string[];
  mau?: number;
  apiCalls?: number;
}

interface Customer {
  id: string;
  name: string;
  industry: string;
  tier: string;
  location: string;
  contact: string;
  email: string;
  phone: string;
  totalRevenue: number;
  healthScore: number;
  trend: "up" | "down" | "stable";
  lastContact: string;
  products: CustomerProduct[];
}

const tierColors: Record<string, string> = {
  Enterprise: "bg-accent/20 text-accent border-accent/30",
  Growth: "bg-chart-1/20 text-chart-1 border-chart-1/30",
  Starter: "bg-muted text-muted-foreground border-border",
};

/* ---------- main section ---------- */

export function CustomersSection() {
  const { data: allCustomers, isLoading } = useCustomers();
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedTier, setSelectedTier] = useState<string | null>(null);

  const customers = (allCustomers ?? []) as Customer[];

  const filteredCustomers = customers.filter((customer) => {
    const matchesSearch =
      customer.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      customer.contact.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesTier = !selectedTier || customer.tier === selectedTier;
    return matchesSearch && matchesTier;
  });

  const totalRevenue = customers.reduce((acc, c) => acc + c.totalRevenue, 0);
  const avgHealthScore = customers.length > 0
    ? Math.round(customers.reduce((acc, c) => acc + c.healthScore, 0) / customers.length)
    : 0;
  const totalLicenses = customers.reduce(
    (acc, c) => acc + (c.products ?? []).reduce((a: number, p: CustomerProduct) => a + (p.licenseKeys?.length ?? 0), 0), 0
  );

  if (isLoading) {
    return (
      <div className="space-y-6">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-24 rounded-xl" />)}
        </div>
        <Skeleton className="h-10 w-full" />
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-64 rounded-xl" />)}
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {[
          { label: "Total Customers", value: customers.length.toString(), icon: Building2, color: "text-foreground" },
          { label: "Total Revenue", value: `$${(totalRevenue / 1000000).toFixed(2)}M`, icon: DollarSign, color: "text-accent" },
          { label: "Avg Health Score", value: `${avgHealthScore}%`, icon: Star, color: "text-chart-3" },
          { label: "Active Licenses", value: totalLicenses.toString(), icon: KeyRound, color: "text-chart-1" },
        ].map((stat, index) => (
          <Card
            key={stat.label}
            className="border-border bg-card hover:border-muted-foreground/30 transition-all duration-300"
            style={{ animationDelay: `${index * 50}ms` }}
          >
            <CardContent className="p-4">
              <div className="flex items-center justify-between">
                <div>
                  <p className="text-sm text-muted-foreground">{stat.label}</p>
                  <p className={`text-2xl font-semibold mt-1 ${stat.color}`}>{stat.value}</p>
                </div>
                <stat.icon className={`w-8 h-8 ${stat.color} opacity-50`} />
              </div>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Filters and Search */}
      <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center justify-between">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <Input
              placeholder="Search customers..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10 w-[280px] bg-secondary border-border focus:border-accent"
            />
          </div>
          <div className="flex items-center gap-2">
            <Filter className="w-4 h-4 text-muted-foreground" />
            {["Enterprise", "Growth", "Starter"].map((tier) => (
              <Button
                key={tier}
                variant={selectedTier === tier ? "default" : "outline"}
                size="sm"
                onClick={() => setSelectedTier(selectedTier === tier ? null : tier)}
                className={selectedTier === tier ? "bg-accent text-accent-foreground" : ""}
              >
                {tier}
              </Button>
            ))}
          </div>
        </div>
      </div>

      {/* Customer Cards */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        {filteredCustomers.map((customer, index) => {
          const productTypes = [...new Set((customer.products ?? []).map((p: CustomerProduct) => p.type))];
          const licenseKeys = (customer.products ?? []).flatMap((p: CustomerProduct) => p.licenseKeys ?? []);
          const saasProduct = (customer.products ?? []).find((p: CustomerProduct) => p.type === "saas");
          const apiProduct = (customer.products ?? []).find((p: CustomerProduct) => p.type === "api");

          return (
            <Card
              key={customer.id}
              className="border-border bg-card hover:border-accent/50 transition-all duration-300 group animate-in fade-in slide-in-from-bottom-2"
              style={{ animationDelay: `${index * 75}ms` }}
            >
              <CardContent className="p-5">
                <div className="flex items-start justify-between mb-4">
                  <div className="flex items-center gap-3">
                    <Avatar className="w-12 h-12 bg-secondary">
                      <AvatarFallback className="bg-secondary text-foreground font-semibold">
                        {customer.name.split(" ").map((n: string) => n[0]).join("").slice(0, 2)}
                      </AvatarFallback>
                    </Avatar>
                    <div>
                      <h3 className="font-semibold text-foreground group-hover:text-accent transition-colors">
                        {customer.name}
                      </h3>
                      <p className="text-sm text-muted-foreground">{customer.industry}</p>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    {productTypes.map((type: ProductType) => (
                      <ProductTypeBadge key={type} type={type} />
                    ))}
                    <Badge className={`${tierColors[customer.tier] ?? tierColors.Starter} border`}>
                      {customer.tier}
                    </Badge>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4 mb-4">
                  <div className="space-y-2">
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <MapPin className="w-3.5 h-3.5" />
                      {customer.location}
                    </div>
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Mail className="w-3.5 h-3.5" />
                      {customer.email}
                    </div>
                    <div className="flex items-center gap-2 text-sm text-muted-foreground">
                      <Phone className="w-3.5 h-3.5" />
                      {customer.phone}
                    </div>
                  </div>
                  <div className="space-y-2">
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-muted-foreground">Revenue</span>
                      <span className="font-medium text-foreground">
                        ${customer.totalRevenue.toLocaleString()}
                      </span>
                    </div>
                    {licenseKeys.length > 0 && (
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">Active Licenses</span>
                        <span className="font-medium text-foreground">{licenseKeys.length}</span>
                      </div>
                    )}
                    {saasProduct && (
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">Platform Users</span>
                        <span className="font-medium text-foreground">{formatUsageMetric(saasProduct.mau!)}</span>
                      </div>
                    )}
                    {apiProduct && (
                      <div className="flex items-center justify-between text-sm">
                        <span className="text-muted-foreground">API Calls/mo</span>
                        <span className="font-medium text-foreground">{formatUsageMetric(apiProduct.apiCalls!)}</span>
                      </div>
                    )}
                    <div className="flex items-center justify-between text-sm">
                      <span className="text-muted-foreground">Last Contact</span>
                      <span className="font-medium text-foreground">{customer.lastContact}</span>
                    </div>
                  </div>
                </div>

                {/* Health Score */}
                <div className="flex items-center justify-between pt-4 border-t border-border">
                  <div className="flex items-center gap-2">
                    <span className="text-sm text-muted-foreground">Health Score</span>
                    {customer.trend === "up" && <TrendingUp className="w-3.5 h-3.5 text-accent" />}
                    {customer.trend === "down" && <TrendingDown className="w-3.5 h-3.5 text-destructive" />}
                  </div>
                  <div className="flex items-center gap-3">
                    <div className="w-24 h-2 bg-secondary rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all duration-1000 ease-out"
                        style={{
                          width: `${customer.healthScore}%`,
                          backgroundColor:
                            customer.healthScore >= 80 ? "oklch(0.7 0.18 145)" : customer.healthScore >= 60 ? "oklch(0.75 0.18 55)" : "oklch(0.65 0.2 25)",
                        }}
                      />
                    </div>
                    <span className={`text-sm font-semibold ${customer.healthScore >= 80 ? "text-accent" : customer.healthScore >= 60 ? "text-chart-3" : "text-destructive"}`}>
                      {customer.healthScore}%
                    </span>
                  </div>
                </div>

                {/* Product Details Section */}
                <div className="mt-4 pt-4 border-t border-border space-y-3">
                  {licenseKeys.length > 0 && (
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <KeyRound className="w-3.5 h-3.5 text-chart-1" />
                      <span className="font-medium text-chart-1">{licenseKeys.length} license{licenseKeys.length !== 1 ? "s" : ""}</span>
                    </div>
                  )}
                  {saasProduct && (
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <Users className="w-3.5 h-3.5 text-chart-3" />
                      <span className="font-medium text-chart-3">{saasProduct.name}</span>
                      <span>&middot;</span>
                      <span>{formatUsageMetric(saasProduct.mau!)} MAU</span>
                    </div>
                  )}
                  {apiProduct && (
                    <div className="flex items-center gap-2 text-xs text-muted-foreground">
                      <Zap className="w-3.5 h-3.5 text-chart-5" />
                      <span className="font-medium text-chart-5">{apiProduct.name}</span>
                      <span>&middot;</span>
                      <span>{formatUsageMetric(apiProduct.apiCalls!)} calls/mo</span>
                    </div>
                  )}
                </div>

                {/* Quick Actions */}
                <div className="flex items-center gap-2 mt-4 pt-4 border-t border-border">
                  <Button
                    variant="outline"
                    size="sm"
                    className="flex-1 bg-transparent"
                    onClick={() => {
                      window.open(`mailto:${customer.email}?subject=Meeting%20Request&body=Hi%20${encodeURIComponent(customer.contact)},%0A%0AI'd%20like%20to%20schedule%20a%20meeting.`);
                      toast.success(`Opening email to schedule with ${customer.name}`);
                    }}
                  >
                    <Calendar className="w-3.5 h-3.5 mr-1.5" />
                    Schedule
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    className="flex-1 bg-transparent"
                    onClick={() => {
                      window.open(`mailto:${customer.email}?subject=Follow-up%20from%20${appConfig.shortName}`);
                      toast.success(`Opening email to ${customer.email}`);
                    }}
                  >
                    <Mail className="w-3.5 h-3.5 mr-1.5" />
                    Email
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => toast.info(`Customer profile: ${customer.name}`)}
                  >
                    <ExternalLink className="w-4 h-4" />
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
