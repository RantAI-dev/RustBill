"use client";

import { cn } from "@/lib/utils";
import type { PrimarySection, Section } from "@/app/page";
import { Search, Calendar, LogOut } from "lucide-react";
import { useState, useRef, useEffect } from "react";
import { useCurrentUser, useLogout } from "@/hooks/use-auth";
import { NotificationCenter } from "@/components/global/notification-center";

interface HeaderProps {
  activeSection: Section;
  activePrimary: PrimarySection;
  dashboardViews: { id: Section; label: string }[];
  onDashboardViewChange: (section: Section) => void;
  onOpenSearch: () => void;
}

const sectionTitles: Record<Section, string> = {
  overview: "Overview",
  trials: "Trials",
  deals: "Deals",
  customers: "Customers",
  licenses: "License Management",
  products: "Product Performance",
  forecasting: "Revenue Forecasting",
  reports: "Reports",
  settings: "Settings",
  "api-docs": "API Documentation",
  "manage-products": "Manage Products",
  "manage-deals": "Manage Deals",
  "manage-customers": "Manage Customers",
  "manage-licenses": "Manage Licenses",
  billing: "Billing",
  "manage-plans": "Pricing Plans",
  "manage-subscriptions": "Subscriptions",
  "manage-invoices": "Invoices",
  "manage-coupons": "Coupons & Discounts",
  "manage-webhooks": "Webhooks",
  "billing-portal": "Billing Portal",
  "manage-tax-rules": "Tax Rules",
};

export function Header({
  activeSection,
  activePrimary,
  dashboardViews,
  onDashboardViewChange,
  onOpenSearch,
}: HeaderProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const { data: user } = useCurrentUser();
  const logout = useLogout();

  // Close dropdown on outside click
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, []);

  const initials = user
    ? user.name
        .split(" ")
        .map((w: string) => w[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "??";

  return (
    <header className="border-b border-border bg-background/80 backdrop-blur-sm sticky top-0 z-30">
      <div className="h-16 flex items-center justify-between px-6">
        <div className="flex items-center gap-6">
          <h1 className="text-xl font-semibold text-foreground">
            {activePrimary === "dashboard"
              ? `Dashboard / ${sectionTitles[activeSection]}`
              : sectionTitles[activeSection]}
          </h1>
          <div className="hidden md:flex items-center gap-2 text-sm text-muted-foreground">
            <Calendar className="w-4 h-4" />
            <span>All time</span>
          </div>
        </div>
 
        <div className="flex items-center gap-4">
          {/* Search trigger — opens Cmd+K palette */}
          <button
            onClick={onOpenSearch}
            className={cn(
              "flex items-center gap-2 h-9 px-3 rounded-lg bg-secondary border border-border text-sm text-muted-foreground hover:text-foreground hover:border-accent/50 transition-all duration-200 w-48"
            )}
          >
            <Search className="w-4 h-4" />
            <span className="flex-1 text-left">Search...</span>
            <kbd className="hidden sm:inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded bg-background border border-border text-[10px] font-mono text-muted-foreground">
              <span className="text-xs">&#x2318;</span>K
            </kbd>
          </button>

          {/* Notifications */}
          <NotificationCenter />

          {/* User avatar + dropdown */}
          <div className="relative" ref={menuRef}>
            <button
              onClick={() => setMenuOpen(!menuOpen)}
              className="w-9 h-9 rounded-lg overflow-hidden bg-secondary ring-2 ring-transparent hover:ring-accent/50 transition-all duration-200"
            >
              <div className="w-full h-full bg-gradient-to-br from-accent/80 to-chart-1 flex items-center justify-center text-xs font-semibold text-accent-foreground">
                {initials}
              </div>
            </button>

            {menuOpen && (
              <div className="absolute right-0 mt-2 w-56 rounded-xl bg-card border border-border shadow-lg py-1 animate-in fade-in slide-in-from-top-2 duration-200 z-50">
                {user && (
                  <div className="px-3 py-2 border-b border-border">
                    <p className="text-sm font-medium text-foreground truncate">{user.name}</p>
                    <p className="text-xs text-muted-foreground truncate">{user.email}</p>
                  </div>
                )}
                <button
                  onClick={() => {
                    setMenuOpen(false);
                    logout();
                  }}
                  className="w-full flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"
                >
                  <LogOut className="w-4 h-4" />
                  Sign out
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {activePrimary === "dashboard" && (
        <div className="px-6 pb-3">
          <div className="hidden md:flex items-center gap-1 overflow-x-auto">
            {dashboardViews.map((view) => (
              <button
                key={view.id}
                onClick={() => onDashboardViewChange(view.id)}
                className={cn(
                  "px-3 py-1.5 rounded-lg text-xs font-medium whitespace-nowrap transition-colors",
                  activeSection === view.id
                    ? "bg-accent text-accent-foreground"
                    : "bg-secondary text-muted-foreground hover:text-foreground"
                )}
              >
                {view.label}
              </button>
            ))}
          </div>
          <div className="md:hidden">
            <select
              value={activeSection}
              onChange={(e) => onDashboardViewChange(e.target.value as Section)}
              className="w-full h-9 rounded-lg bg-secondary border border-border px-3 text-sm text-foreground"
            >
              {dashboardViews.map((view) => (
                <option key={view.id} value={view.id}>
                  {view.label}
                </option>
              ))}
            </select>
          </div>
        </div>
      )}
    </header>
  );
}
