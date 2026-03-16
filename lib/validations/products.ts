import { z } from "zod";

// Only manual/intent fields are required. Revenue, change, unitsSold,
// activeLicenses, totalLicenses are computed from deals & licenses.
const baseProductSchema = z.object({
  name: z.string().min(1, "Name is required"),
  target: z.number().min(0).default(0),
  // Computed fields — accepted on insert/update but ignored in forms
  revenue: z.number().min(0).default(0).optional(),
  change: z.number().default(0).optional(),
});

export const insertLicensedProductSchema = baseProductSchema.extend({
  productType: z.literal("licensed"),
  // Computed from deals/licenses — kept optional for backward compat
  unitsSold: z.number().int().min(0).default(0).optional(),
  activeLicenses: z.number().int().min(0).default(0).optional(),
  totalLicenses: z.number().int().min(0).default(0).optional(),
});

export const insertSaasProductSchema = baseProductSchema.extend({
  productType: z.literal("saas"),
  // External tracking metrics — remain manual
  mau: z.number().int().min(0).default(0),
  dau: z.number().int().min(0).default(0),
  freeUsers: z.number().int().min(0).default(0),
  paidUsers: z.number().int().min(0).default(0),
  churnRate: z.number().min(0).default(0),
});

export const insertApiProductSchema = baseProductSchema.extend({
  productType: z.literal("api"),
  // External tracking metrics — remain manual
  apiCalls: z.number().int().min(0).default(0),
  activeDevelopers: z.number().int().min(0).default(0),
  avgLatency: z.number().min(0).default(0),
});

export const insertProductSchema = z.discriminatedUnion("productType", [
  insertLicensedProductSchema,
  insertSaasProductSchema,
  insertApiProductSchema,
]);

export const updateProductSchema = z.object({
  name: z.string().min(1).optional(),
  target: z.number().min(0).optional(),
  revenue: z.number().min(0).optional(),
  change: z.number().optional(),
  // Licensed
  unitsSold: z.number().int().min(0).optional(),
  activeLicenses: z.number().int().min(0).optional(),
  totalLicenses: z.number().int().min(0).optional(),
  // SaaS
  mau: z.number().int().min(0).optional(),
  dau: z.number().int().min(0).optional(),
  freeUsers: z.number().int().min(0).optional(),
  paidUsers: z.number().int().min(0).optional(),
  churnRate: z.number().min(0).optional(),
  // API
  apiCalls: z.number().int().min(0).optional(),
  activeDevelopers: z.number().int().min(0).optional(),
  avgLatency: z.number().min(0).optional(),
});

export type InsertProduct = z.infer<typeof insertProductSchema>;
export type UpdateProduct = z.infer<typeof updateProductSchema>;
