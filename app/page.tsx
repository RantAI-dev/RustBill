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
import { BillingPortalSection } from "@/components/dashboard/sections/billing-portal";
import { ApiProvider } from "@/hooks/use-api";
import { BackendBanner } from "@/components/backend-banner";

export type Section =
  | "overview" | "trials" | "deals" | "customers" | "licenses" | "products" | "forecasting" | "reports" | "settings" | "api-docs" | "billing"
  | "manage-products" | "manage-deals" | "manage-customers" | "manage-licenses" | "manage-plans" | "manage-subscriptions" | "manage-invoices" | "manage-coupons"
  | "manage-webhooks" | "billing-portal";

export default function Dashboard() {
  const [activeSection, setActiveSection] = useState<Section>("overview");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);

  // Open search palette programmatically (from header button click)
  const openSearch = useCallback(() => {
    document.dispatchEvent(new KeyboardEvent("keydown", { key: "k", metaKey: true, bubbles: true }));
  }, []);

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
        onSectionChange={setActiveSection}
        collapsed={sidebarCollapsed}
        onCollapsedChange={setSidebarCollapsed}
      />
      <div
        className={`flex-1 flex flex-col transition-all duration-300 ease-out ${
          sidebarCollapsed ? "ml-[72px]" : "ml-[260px]"
        }`}
      >
        <BackendBanner />
        <Header activeSection={activeSection} onOpenSearch={openSearch} />
        <main className="flex-1 p-6 overflow-auto">
          <div
            key={activeSection}
            className="animate-in fade-in slide-in-from-bottom-4 duration-500"
          >
            {renderSection()}
          </div>
        </main>
      </div>
      <CommandPalette onNavigate={setActiveSection} />
    </div>
    </ApiProvider>
  );
}
