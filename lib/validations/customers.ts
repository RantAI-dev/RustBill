import { z } from "zod";

export const insertCustomerSchema = z.object({
  name: z.string().min(1, "Name is required"),
  industry: z.string().min(1, "Industry is required"),
  tier: z.enum(["Enterprise", "Growth", "Starter"]),
  location: z.string().min(1, "Location is required"),
  contact: z.string().min(1, "Contact is required"),
  email: z.string().email("Invalid email"),
  phone: z.string().min(1, "Phone is required"),
  // Computed fields — accepted but ignored in forms
  totalRevenue: z.number().min(0).default(0).optional(),
  healthScore: z.number().int().min(0).max(100).default(50).optional(),
  trend: z.enum(["up", "down", "stable"]).default("stable").optional(),
  lastContact: z.string().min(1).default("Today").optional(),
  // Billing profile
  billingEmail: z.string().email().nullable().optional(),
  billingAddress: z.string().nullable().optional(),
  billingCity: z.string().nullable().optional(),
  billingState: z.string().nullable().optional(),
  billingZip: z.string().nullable().optional(),
  billingCountry: z.string().nullable().optional(),
  taxId: z.string().nullable().optional(),
  defaultPaymentMethod: z.enum(["manual", "stripe", "bank_transfer", "check"]).nullable().optional(),
  stripeCustomerId: z.string().nullable().optional(),
});

export const updateCustomerSchema = insertCustomerSchema.partial();

export type InsertCustomer = z.infer<typeof insertCustomerSchema>;
