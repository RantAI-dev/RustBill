"use client";

import { useState, useEffect, useCallback } from "react";
import type { Section } from "@/app/page";
import { useSearch } from "@/hooks/use-api";
import {
  CommandDialog,
  CommandInput,
  CommandList,
  CommandEmpty,
  CommandGroup,
  CommandItem,
  CommandSeparator,
} from "@/components/ui/command";
import {
  Package,
  Building2,
  KeyRound,
  FileText,
  RefreshCw,
  LayoutDashboard,
  BarChart3,
  TrendingUp,
  CreditCard,
  Settings,
  BookOpen,
  FlaskConical,
  Receipt,
  Tag,
  Globe,
  UserCircle,
  FileBadge,
  Activity,
  Coins,
} from "lucide-react";

interface CommandPaletteProps {
  onNavigate: (section: Section) => void;
}

const sectionNav: { id: Section; label: string; icon: React.ElementType; group: string }[] = [
  { id: "overview", label: "Overview", icon: LayoutDashboard, group: "Dashboard" },
  { id: "products", label: "Product Performance", icon: Package, group: "Dashboard" },
  { id: "trials", label: "Trials", icon: FlaskConical, group: "Dashboard" },
  { id: "customers", label: "Customers", icon: Building2, group: "Dashboard" },
  { id: "licenses", label: "Licenses", icon: KeyRound, group: "Dashboard" },
  { id: "forecasting", label: "Forecasting", icon: TrendingUp, group: "Dashboard" },
  { id: "reports", label: "Reports", icon: BarChart3, group: "Dashboard" },
  { id: "sales-360", label: "Sales 360", icon: BarChart3, group: "Dashboard" },
  { id: "billing", label: "Billing", icon: CreditCard, group: "Dashboard" },
  { id: "sales-one-time", label: "One-Time", icon: FileBadge, group: "Sales" },
  { id: "sales-subscriptions", label: "Subscriptions", icon: RefreshCw, group: "Sales" },
  { id: "sales-usage", label: "Usage Events", icon: Activity, group: "Sales" },
  { id: "sales-credits", label: "Token Credits", icon: Coins, group: "Sales" },
  { id: "sales-licenses", label: "Licenses", icon: KeyRound, group: "Sales" },
  { id: "manage-products", label: "Manage Products", icon: Package, group: "Management" },
  { id: "manage-customers", label: "Manage Customers", icon: Building2, group: "Management" },
  { id: "manage-licenses", label: "Manage Licenses", icon: KeyRound, group: "Management" },
  { id: "manage-plans", label: "Pricing Plans", icon: Receipt, group: "Management" },
  { id: "manage-invoices", label: "Invoices", icon: FileText, group: "Management" },
  { id: "manage-coupons", label: "Coupons", icon: Tag, group: "Management" },
  { id: "manage-webhooks", label: "Webhooks", icon: Globe, group: "Management" },
  { id: "billing-portal", label: "Billing Portal", icon: UserCircle, group: "Portal" },
  { id: "settings", label: "Settings", icon: Settings, group: "System" },
  { id: "api-docs", label: "API Docs", icon: BookOpen, group: "System" },
];

export function CommandPalette({ onNavigate }: CommandPaletteProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const { data: results } = useSearch(query);

  const handleOpenChange = useCallback((v: boolean) => {
    setOpen(v);
    if (!v) setQuery("");
  }, []);

  // Keyboard shortcut
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setOpen((prev) => !prev);
      }
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, []);

  const navigate = (section: Section) => {
    onNavigate(section);
    handleOpenChange(false);
  };

  const hasResults = results && (
    results.products?.length > 0 ||
    results.customers?.length > 0 ||
    results.licenses?.length > 0 ||
    results.invoices?.length > 0 ||
    results.subscriptions?.length > 0
  );

  return (
    <CommandDialog
      open={open}
      onOpenChange={handleOpenChange}
      title="Search"
      description="Search across all entities or navigate to a section"
    >
      <CommandInput
        placeholder="Search products, customers, invoices..."
        value={query}
        onValueChange={setQuery}
      />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {/* Search results */}
        {query.length >= 2 && hasResults && (
          <>
            {results.products?.length > 0 && (
              <CommandGroup heading="Products">
                {results.products.map((p: { id: string; name: string; productType: string; revenue: number }) => (
                  <CommandItem key={p.id} onSelect={() => navigate("manage-products")}>
                    <Package className="w-4 h-4 mr-2 text-muted-foreground" />
                    <span>{p.name}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{p.productType}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {results.customers?.length > 0 && (
              <CommandGroup heading="Customers">
                {results.customers.map((c: { id: string; name: string; email: string; tier: string }) => (
                  <CommandItem key={c.id} onSelect={() => navigate("manage-customers")}>
                    <Building2 className="w-4 h-4 mr-2 text-muted-foreground" />
                    <span>{c.name}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{c.tier}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {results.licenses?.length > 0 && (
              <CommandGroup heading="Licenses">
                {results.licenses.map((l: { key: string; customerName: string; productName: string; status: string }) => (
                  <CommandItem key={l.key} onSelect={() => navigate("sales-licenses")}>
                    <KeyRound className="w-4 h-4 mr-2 text-muted-foreground" />
                    <span>{l.customerName} — {l.productName}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{l.status}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {results.invoices?.length > 0 && (
              <CommandGroup heading="Invoices">
                {results.invoices.map((i: { id: string; invoiceNumber: string; customerName: string; total: number; status: string }) => (
                  <CommandItem key={i.id} onSelect={() => navigate("manage-invoices")}>
                    <FileText className="w-4 h-4 mr-2 text-muted-foreground" />
                    <span>{i.invoiceNumber} — {i.customerName}</span>
                    <span className="ml-auto text-xs text-muted-foreground">${i.total?.toLocaleString()}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            {results.subscriptions?.length > 0 && (
              <CommandGroup heading="Subscriptions">
                {results.subscriptions.map((s: { id: string; customerName: string; planName: string; status: string }) => (
                  <CommandItem key={s.id} onSelect={() => navigate("sales-subscriptions")}>
                    <RefreshCw className="w-4 h-4 mr-2 text-muted-foreground" />
                    <span>{s.customerName} — {s.planName}</span>
                    <span className="ml-auto text-xs text-muted-foreground">{s.status}</span>
                  </CommandItem>
                ))}
              </CommandGroup>
            )}

            <CommandSeparator />
          </>
        )}

        {/* Quick navigation (always shown) */}
        {query.length < 2 && (
          <>
            <CommandGroup heading="Navigate">
              {sectionNav.map((item) => (
                <CommandItem key={item.id} onSelect={() => navigate(item.id)}>
                  <item.icon className="w-4 h-4 mr-2 text-muted-foreground" />
                  <span>{item.label}</span>
                  <span className="ml-auto text-xs text-muted-foreground">{item.group}</span>
                </CommandItem>
              ))}
            </CommandGroup>
          </>
        )}
      </CommandList>
    </CommandDialog>
  );
}
