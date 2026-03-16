import { z } from "zod";

export const insertLicenseSchema = z.object({
  key: z.string().min(1, "Key is required"),
  customerId: z.string().min(1, "Customer is required"),
  customerName: z.string().optional().default(""),
  productId: z.string().min(1, "Product is required"),
  productName: z.string().optional().default(""),
  status: z.enum(["active", "expired", "revoked", "suspended"]).default("active"),
  createdAt: z.string().min(1),
  expiresAt: z.string().min(1),
  licenseType: z.enum(["simple", "signed"]).default("simple"),
  features: z.array(z.string()).optional(),
  maxActivations: z.number().int().positive().optional(),
});

export const updateLicenseSchema = z.object({
  status: z.enum(["active", "expired", "revoked", "suspended"]).optional(),
  expiresAt: z.string().optional(),
  features: z.array(z.string()).optional(),
  maxActivations: z.number().int().positive().nullable().optional(),
});

export const generateSignedLicenseSchema = z.object({
  features: z.array(z.string()).default([]),
  maxActivations: z.number().int().positive().optional(),
  metadata: z.record(z.unknown()).optional(),
});

export const verifyLicenseSchema = z.object({
  licenseFile: z.string().min(1, "License file content is required"),
});

export const onlineVerifySchema = z.object({
  licenseKey: z.string().min(1, "License key is required"),
  deviceId: z.string().max(255).optional(),
  deviceName: z.string().max(255).optional(),
});

export type InsertLicense = z.infer<typeof insertLicenseSchema>;
