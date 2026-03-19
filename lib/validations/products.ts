import { z } from "zod";

const numberFromInput = z.coerce.number();
const intFromInput = z.coerce.number().int();

// Only manual/intent fields are required. Revenue, change, unitsSold,
// activeLicenses, totalLicenses are computed from deals & licenses.
const baseProductSchema = z.object({
  name: z.string().min(1, "Name is required"),
  target: numberFromInput.min(0).default(0),
  // Computed fields — accepted on insert/update but ignored in forms
  revenue: numberFromInput.min(0).default(0).optional(),
  change: numberFromInput.default(0).optional(),
});

export const insertLicensedProductSchema = baseProductSchema.extend({
  productType: z.literal("licensed"),
  // Computed from deals/licenses — kept optional for backward compat
  unitsSold: intFromInput.min(0).default(0).optional(),
  activeLicenses: intFromInput.min(0).default(0).optional(),
  totalLicenses: intFromInput.min(0).default(0).optional(),
});

export const insertSaasProductSchema = baseProductSchema.extend({
  productType: z.literal("saas"),
  // External tracking metrics — remain manual
  mau: intFromInput.min(0).default(0),
  dau: intFromInput.min(0).default(0),
  freeUsers: intFromInput.min(0).default(0),
  paidUsers: intFromInput.min(0).default(0),
  churnRate: numberFromInput.min(0).default(0),
});

export const insertApiProductSchema = baseProductSchema.extend({
  productType: z.literal("api"),
  // External tracking metrics — remain manual
  apiCalls: intFromInput.min(0).default(0),
  activeDevelopers: intFromInput.min(0).default(0),
  avgLatency: numberFromInput.min(0).default(0),
});

export const insertProductSchema = z.discriminatedUnion("productType", [
  insertLicensedProductSchema,
  insertSaasProductSchema,
  insertApiProductSchema,
]);

export const updateProductSchema = z.object({
  name: z.string().min(1).optional(),
  target: numberFromInput.min(0).optional(),
  revenue: numberFromInput.min(0).optional(),
  change: numberFromInput.optional(),
  // Licensed
  unitsSold: intFromInput.min(0).optional(),
  activeLicenses: intFromInput.min(0).optional(),
  totalLicenses: intFromInput.min(0).optional(),
  // SaaS
  mau: intFromInput.min(0).optional(),
  dau: intFromInput.min(0).optional(),
  freeUsers: intFromInput.min(0).optional(),
  paidUsers: intFromInput.min(0).optional(),
  churnRate: numberFromInput.min(0).optional(),
  // API
  apiCalls: intFromInput.min(0).optional(),
  activeDevelopers: intFromInput.min(0).optional(),
  avgLatency: numberFromInput.min(0).optional(),
});

export type InsertProduct = z.infer<typeof insertProductSchema>;
export type UpdateProduct = z.infer<typeof updateProductSchema>;
