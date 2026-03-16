export type ProductType = "licensed" | "saas" | "api";

export interface ProductTypeConfig {
  label: string;
  shortLabel: string;
  bgClass: string;
  textClass: string;
  dotClass: string;
}

export const productTypeConfig: Record<ProductType, ProductTypeConfig> = {
  licensed: {
    label: "Licensed",
    shortLabel: "License",
    bgClass: "bg-chart-1/10",
    textClass: "text-chart-1",
    dotClass: "bg-chart-1",
  },
  saas: {
    label: "Platform",
    shortLabel: "SaaS",
    bgClass: "bg-chart-3/10",
    textClass: "text-chart-3",
    dotClass: "bg-chart-3",
  },
  api: {
    label: "API",
    shortLabel: "API",
    bgClass: "bg-chart-5/10",
    textClass: "text-chart-5",
    dotClass: "bg-chart-5",
  },
};

/**
 * Format a usage metric value for display.
 * e.g. 45200 -> "45.2k", 2400000 -> "2.4M"
 */
export function formatUsageMetric(value: number): string {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return value.toString();
}
