"use client";

import { useState, useCallback } from "react";
import { Sidebar } from "@/components/dashboard/sidebar";
import { Header } from "@/components/dashboard/header";
import { CommandPalette } from "@/components/global/command-palette";
import { OverviewSection } from "@/components/dashboard/sections/overview";
import { TrialsSection } from "@/components/dashboard/sections/trials";
import { DealsSection } from "@/components/dashboard/sections/deals";
import { CustomersSection } from "@/components/dashboard/sections/customers";
import { ProductPerformanceSection } from "@/components/dashboard/sections/product-performance";
import { LicensesSection } from "@/components/dashboard/sections/licenses";
import { ForecastingSection } from "@/components/dashboard/sections/forecasting";
import { ReportsSection } from "@/components/dashboard/sections/reports";
import { Sales360Section } from "@/components/dashboard/sections/sales-360";
import { SettingsSection } from "@/components/dashboard/sections/settings";
import { ApiDocsSection } from "@/components/dashboard/sections/api-docs";
import { ManageProductsSection } from "@/components/management/products";
import { ManageDealsSection } from "@/components/management/deals";
import { ManageCustomersSection } from "@/components/management/customers";
import { ManageLicensesSection } from "@/components/management/licenses";
import { BillingSection } from "@/components/dashboard/sections/billing";
import { ManagePlansSection } from "@/components/management/plans";
import { ManageSubscriptionsSection } from "@/components/management/subscriptions";
import { ManageInvoicesSection } from "@/components/management/invoices";
import { ManageCouponsSection } from "@/components/management/coupons";
import { ManageWebhooksSection } from "@/components/management/webhooks";
import { TaxRulesManagement } from "@/components/management/tax-rules";
import { BillingPortalSection } from "@/components/dashboard/sections/billing-portal";
import { ApiProvider } from "@/hooks/use-api";
import { BackendBanner } from "@/components/backend-banner";

export type Section =
  | "overview" | "trials" | "deals" | "customers" | "licenses" | "products" | "forecasting" | "reports" | "sales-360" | "settings" | "api-docs" | "billing"
  | "manage-products" | "manage-deals" | "manage-customers" | "manage-licenses" | "manage-plans" | "manage-subscriptions" | "manage-invoices" | "manage-coupons"
  | "manage-webhooks" | "manage-tax-rules" | "billing-portal";

export type PrimarySection = "dashboard" | "management" | "portal" | "system";

type DashboardView =
  | "overview"
  | "products"
  | "trials"
  | "customers"
  | "licenses"
  | "forecasting"
  | "reports"
  | "sales-360"
  | "billing";

const dashboardViews: { id: DashboardView; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "products", label: "Products" },
  { id: "trials", label: "Trials" },
  { id: "customers", label: "Customers" },
  { id: "licenses", label: "Licenses" },
  { id: "forecasting", label: "Forecasting" },
  { id: "reports", label: "Reports" },
  { id: "sales-360", label: "Sales 360" },
  { id: "billing", label: "Billing" },
];

function isDashboardView(section: Section): section is DashboardView {
  return dashboardViews.some((view) => view.id === section);
}

function getPrimarySection(section: Section): PrimarySection {
  if (isDashboardView(section)) return "dashboard";
  if (section.startsWith("manage-")) return "management";
  if (section === "billing-portal") return "portal";
  return "system";
}

export default function Dashboard() {
  const [activeSection, setActiveSection] = useState<Section>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [lastDashboardView, setLastDashboardView] = useState<DashboardView>("overview");

  // Open search palette programmatically (from header button click)
  const openSearch = useCallback(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }));
  }, []);

  const activePrimary = getPrimarySection(activeSection);

  const handleSectionChange = useCallback((section: Section) => {
    setActiveSection(section);
    if (isDashboardView(section)) {
      setLastDashboardView(section);
    }
  }, []);

  const handlePrimaryChange = useCallback((primary: PrimarySection) => {
    if (primary === "dashboard") {
      setActiveSection(lastDashboardView);
      return;
    }

    if (primary === "management") {
      setActiveSection((current) =>
        current.startsWith("manage-") ? current : "manage-products"
      );
      return;
    }

    if (primary === "portal") {
      setActiveSection("billing-portal");
      return;
    }

    setActiveSection((current) =>
      current === "settings" || current === "api-docs" ? current : "settings"
    );
  }, [lastDashboardView]);

  const renderSection = () => {
    switch (activeSection) {
      case "overview":
        return <OverviewSection />;
      case "trials":
        return <TrialsSection />;
      case "deals":
        return <DealsSection />;
      case "customers":
        return <CustomersSection />;
      case "licenses":
        return <LicensesSection />;
      case "products":
        return <ProductPerformanceSection />;
      case "forecasting":
        return <ForecastingSection />;
      case "reports":
        return <ReportsSection />;
      case "sales-360":
        return <Sales360Section />;
      case "settings":
        return <SettingsSection />;
      case "api-docs":
        return <ApiDocsSection />;
      case "manage-products":
        return <ManageProductsSection />;
      case "manage-deals":
        return <ManageDealsSection />;
      case "manage-customers":
        return <ManageCustomersSection />;
      case "manage-licenses":
        return <ManageLicensesSection />;
      case "billing":
        return <BillingSection />;
      case "manage-plans":
        return <ManagePlansSection />;
      case "manage-subscriptions":
        return <ManageSubscriptionsSection />;
      case "manage-invoices":
        return <ManageInvoicesSection />;
      case "manage-coupons":
        return <ManageCouponsSection />;
      case "manage-webhooks":
        return <ManageWebhooksSection />;
      case "manage-tax-rules":
        return <TaxRulesManagement />;
      case "billing-portal":
        return <BillingPortalSection />;
      default:
        return <OverviewSection />;
    }
  };

  return (
    <ApiProvider>
    <div className="flex min-h-screen bg-background">
      <Sidebar
        activeSection={activeSection}
        activePrimary={activePrimary}
        onSectionChange={handleSectionChange}
        onDashboardClick={() => handlePrimaryChange("dashboard")}
        collapsed={sidebarCollapsed}
        onCollapsedChange={setSidebarCollapsed}
      />
      <div
        className={`flex-1 flex flex-col transition-all duration-300 ease-out ${
          sidebarCollapsed ? "ml-[72px]" : "ml-[260px]"
        }`}
      >
        <BackendBanner />
        <Header
          activeSection={activeSection}
          activePrimary={activePrimary}
          dashboardViews={dashboardViews}
          onDashboardViewChange={handleSectionChange}
          onOpenSearch={openSearch}
        />
        <main className="flex-1 p-6 overflow-auto">
          <div
            key={activeSection}
            className="animate-in fade-in slide-in-from-bottom-4 duration-500"
          >
            {renderSection()}
          </div>
        </main>
      </div>
      <CommandPalette onNavigate={handleSectionChange} />
    </div>
    </ApiProvider>
  );
}
