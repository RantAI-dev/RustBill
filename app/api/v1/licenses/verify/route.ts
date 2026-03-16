import { db } from "@/lib/db";
import { licenses, licenseActivations } from "@/lib/db/schema";
import { onlineVerifySchema } from "@/lib/validations/licenses";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq, and, count } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

function buildLicenseResponse(license: typeof licenses.$inferSelect) {
  return {
    key: license.key,
    status: license.status,
    product: license.productName,
    productId: license.productId,
    customer: license.customerName,
    customerId: license.customerId,
    createdAt: license.createdAt,
    expiresAt: license.expiresAt,
    features: license.features,
  };
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  // Parse & validate request body
  const body = await req.json().catch(() => null);
  const parsed = onlineVerifySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { valid: false, error: "invalid_request", message: "Request body must include 'licenseKey' string" },
      { status: 400 },
    );
  }

  const { licenseKey, deviceId, deviceName } = parsed.data;
  const normalizedKey = licenseKey.trim().toUpperCase();

  // Look up the license
  const [license] = await db.select().from(licenses).where(eq(licenses.key, normalizedKey));

  if (!license) {
    return NextResponse.json({ valid: false, error: "not_found", message: "License key not found" });
  }

  // Check status
  if (license.status !== "active") {
    return NextResponse.json({
      valid: false,
      error: "license_inactive",
      status: license.status,
      message: `License is ${license.status}`,
    });
  }

  // Check expiration
  const now = new Date();
  const expiresAt = new Date(license.expiresAt);
  if (expiresAt <= now) {
    return NextResponse.json({
      valid: false,
      error: "license_expired",
      status: "expired",
      expiresAt: license.expiresAt,
      message: "License has expired",
    });
  }

  // Handle activation tracking if deviceId is provided
  if (deviceId) {
    const ipAddress = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim()
      ?? req.headers.get("x-real-ip")
      ?? null;

    // Use upsert to handle concurrent same-device requests safely
    // ON CONFLICT (licenseKey, deviceId) → just update lastSeenAt
    const [upserted] = await db
      .insert(licenseActivations)
      .values({
        licenseKey: normalizedKey,
        deviceId,
        deviceName: deviceName ?? null,
        ipAddress,
      })
      .onConflictDoUpdate({
        target: [licenseActivations.licenseKey, licenseActivations.deviceId],
        set: { lastSeenAt: new Date() },
      })
      .returning({ activatedAt: licenseActivations.activatedAt, lastSeenAt: licenseActivations.lastSeenAt });

    // Check if this was an existing activation (activatedAt !== lastSeenAt means update, not insert)
    const wasExisting = upserted.activatedAt.getTime() !== upserted.lastSeenAt.getTime();

    // Get the current count AFTER the upsert
    const [{ value: activationCount }] = await db
      .select({ value: count() })
      .from(licenseActivations)
      .where(eq(licenseActivations.licenseKey, normalizedKey));

    // If this was a NEW activation, check if we exceeded the limit
    if (!wasExisting && license.maxActivations && activationCount > license.maxActivations) {
      // We exceeded — roll back by deleting the just-inserted activation
      await db
        .delete(licenseActivations)
        .where(
          and(
            eq(licenseActivations.licenseKey, normalizedKey),
            eq(licenseActivations.deviceId, deviceId),
          ),
        );

      return NextResponse.json({
        valid: false,
        error: "activation_limit_reached",
        message: `Maximum ${license.maxActivations} activations reached`,
        activations: activationCount - 1,
        maxActivations: license.maxActivations,
      });
    }

    return NextResponse.json({
      valid: true,
      license: buildLicenseResponse(license),
      activations: activationCount,
      maxActivations: license.maxActivations,
    });
  }

  // No deviceId — backward-compatible simple validation
  const [{ value: activationCount }] = await db
    .select({ value: count() })
    .from(licenseActivations)
    .where(eq(licenseActivations.licenseKey, normalizedKey));

  return NextResponse.json({
    valid: true,
    license: buildLicenseResponse(license),
    activations: activationCount,
    maxActivations: license.maxActivations,
  });
}
