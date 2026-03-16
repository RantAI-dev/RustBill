import { z } from "zod";

export const insertDealSchema = z.object({
  customerId: z.string().nullable().optional(),
  company: z.string().min(1, "Company is required"),
  contact: z.string().min(1, "Contact is required"),
  email: z.string().email("Invalid email"),
  value: z.number().min(0, "Value must be positive"),
  productId: z.string().nullable().optional(),
  productName: z.string().min(1, "Product is required"),
  productType: z.enum(["licensed", "saas", "api"]),
  dealType: z.enum(["sale", "trial", "partner"]).default("sale"),
  date: z.string().min(1, "Date is required"),
  licenseKey: z.string().nullable().optional(),
  notes: z.string().nullable().optional(),
  licenseExpiresAt: z.string().nullable().optional(),
  usageMetricLabel: z.string().nullable().optional(),
  usageMetricValue: z.number().int().nullable().optional(),
});

export const updateDealSchema = insertDealSchema.partial();

export type InsertDeal = z.infer<typeof insertDealSchema>;
