"use client";

import { cn } from "@/lib/utils";
import { type ProductType, productTypeConfig } from "@/lib/product-types";

interface ProductTypeBadgeProps {
  type: ProductType;
  size?: "sm" | "md";
  className?: string;
}

export function ProductTypeBadge({ type, size = "sm", className }: ProductTypeBadgeProps) {
  const config = productTypeConfig[type] ?? productTypeConfig.licensed;
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full font-medium",
        config.bgClass,
        config.textClass,
        size === "sm" ? "px-2 py-0.5 text-[10px]" : "px-2.5 py-0.5 text-xs",
        className
      )}
    >
      <span className={cn("rounded-full shrink-0", config.dotClass, size === "sm" ? "w-1.5 h-1.5" : "w-2 h-2")} />
      {config.label}
    </span>
  );
}
