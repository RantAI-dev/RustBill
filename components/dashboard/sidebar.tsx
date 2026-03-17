"use client";

import React from "react";
import { cn } from "@/lib/utils";
import type { Section } from "@/app/page";
import Image from "next/image";
import { appConfig } from "@/lib/app-config";
import {
  LayoutDashboard,
  FlaskConical,
  Handshake,
  BarChart3,
  PanelLeftClose,
  PanelLeftOpen,
  Package,
  Building2,
  TrendingUp,
  Settings,
  KeyRound,
  BookOpen,
  CreditCard,
  Receipt,
  RefreshCw,
  FileText,
  Tag,
  Globe,
  Scale,
  UserCircle,
} from "lucide-react";

interface SidebarProps {
  activeSection: Section;
  onSectionChange: (section: Section) => void;
  collapsed: boolean;
  onCollapsedChange: (collapsed: boolean) => void;
}

type NavGroup = {
  label: string;
  items: { id: Section; label: string; icon: React.ElementType }[];
};

const navGroups: NavGroup[] = [
  {
    label: "Dashboard",
    items: [
      { id: "overview", label: "Overview", icon: LayoutDashboard },
      { id: "products", label: "Product Performance", icon: Package },
      { id: "trials", label: "Trials", icon: FlaskConical },
      { id: "deals", label: "Deals", icon: Handshake },
      { id: "customers", label: "Customers", icon: Building2 },
      { id: "licenses", label: "Licenses", icon: KeyRound },
      { id: "forecasting", label: "Forecasting", icon: TrendingUp },
      { id: "reports", label: "Reports", icon: BarChart3 },
      { id: "billing", label: "Billing", icon: CreditCard },
    ],
  },
  {
    label: "Management",
    items: [
      { id: "manage-products", label: "Products", icon: Package },
      { id: "manage-deals", label: "Deals", icon: Handshake },
      { id: "manage-customers", label: "Customers", icon: Building2 },
      { id: "manage-licenses", label: "Licenses", icon: KeyRound },
      { id: "manage-plans", label: "Pricing Plans", icon: Receipt },
      { id: "manage-subscriptions", label: "Subscriptions", icon: RefreshCw },
      { id: "manage-invoices", label: "Invoices", icon: FileText },
      { id: "manage-coupons", label: "Coupons", icon: Tag },
      { id: "manage-webhooks", label: "Webhooks", icon: Globe },
      { id: "manage-tax-rules", label: "Tax Rules", icon: Scale },
    ],
  },
  {
    label: "Portal",
    items: [
      { id: "billing-portal", label: "Billing Portal", icon: UserCircle },
    ],
  },
  {
    label: "System",
    items: [
      { id: "settings", label: "Settings", icon: Settings },
      { id: "api-docs", label: "API Docs", icon: BookOpen },
    ],
  },
];

export function Sidebar({
  activeSection,
  onSectionChange,
  collapsed,
  onCollapsedChange,
}: SidebarProps) {
  return (
    <aside
      className={cn(
        "fixed left-0 top-0 z-40 h-screen bg-sidebar border-r border-sidebar-border transition-all duration-300 ease-out flex flex-col",
        collapsed ? "w-[72px]" : "w-[260px]"
      )}
    >
      {/* Logo + toggle */}
      <div className="h-16 flex items-center px-4 border-b border-sidebar-border">
        {collapsed ? (
          <button
            onClick={() => onCollapsedChange(false)}
            className="group relative w-9 h-9 rounded-lg shrink-0 cursor-pointer"
          >
            <Image
              src={appConfig.logo}
              alt={appConfig.shortName}
              width={36}
              height={36}
              className="w-9 h-9 rounded-lg object-contain transition-opacity duration-200 group-hover:opacity-0"
            />
            <div className="absolute inset-0 flex items-center justify-center rounded-lg bg-sidebar-accent opacity-0 group-hover:opacity-100 transition-opacity duration-200">
              <PanelLeftOpen className="w-5 h-5 text-sidebar-foreground" />
            </div>
          </button>
        ) : (
          <div className="flex items-center justify-between w-full">
            <Image
              src={appConfig.logoFull}
              alt={appConfig.name}
              width={160}
              height={40}
              className="h-9 w-auto object-contain"
            />
            <button
              onClick={() => onCollapsedChange(true)}
              className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent/50 transition-colors duration-200"
            >
              <PanelLeftClose className="w-[18px] h-[18px]" />
            </button>
          </div>
        )}
      </div>

      {/* Navigation */}
      <nav className="flex-1 px-3 py-4 space-y-4 overflow-hidden overflow-y-auto sidebar-scroll">
        {navGroups.map((group) => (
          <div key={group.label}>
            <span
              className={cn(
                "px-3 mb-1 block text-[10px] font-semibold uppercase tracking-widest text-muted-foreground/60 transition-all duration-300",
                collapsed ? "opacity-0 h-0 mb-0" : "opacity-100"
              )}
            >
              {group.label}
            </span>
            <div className="space-y-0.5">
              {group.items.map((item) => {
                const Icon = item.icon;
                const isActive = activeSection === item.id;

                return (
                  <button
                    key={item.id}
                    onClick={() => onSectionChange(item.id)}
                    className={cn(
                      "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 group relative",
                      isActive
                        ? "bg-sidebar-accent text-sidebar-foreground"
                        : "text-muted-foreground hover:text-sidebar-foreground hover:bg-sidebar-accent/50"
                    )}
                  >
                    <span
                      className={cn(
                        "absolute left-0 top-1/2 -translate-y-1/2 w-1 h-6 rounded-r-full bg-accent transition-all duration-300",
                        isActive ? "opacity-100" : "opacity-0"
                      )}
                    />
                    <Icon
                      className={cn(
                        "w-5 h-5 shrink-0 transition-transform duration-200",
                        isActive ? "text-accent" : "group-hover:scale-110"
                      )}
                    />
                    <span
                      className={cn(
                        "whitespace-nowrap transition-all duration-300",
                        collapsed ? "opacity-0 w-0 overflow-hidden" : "opacity-100"
                      )}
                    >
                      {item.label}
                    </span>
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

    </aside>
  );
}
