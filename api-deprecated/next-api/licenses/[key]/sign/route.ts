import { db } from "@/lib/db";
import { licenses, systemSettings } from "@/lib/db/schema";
import { generateSignedLicenseSchema } from "@/lib/validations/licenses";
import { signLicense, type LicensePayload } from "@/lib/license-signing";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function POST(req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const { key } = await params;
  const body = await req.json();

  const parsed = generateSignedLicenseSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  // Fetch the license
  const [license] = await db.select().from(licenses).where(eq(licenses.key, key));
  if (!license) {
    return NextResponse.json({ error: "License not found" }, { status: 404 });
  }

  // Fetch private key
  const [privateKeySetting] = await db
    .select()
    .from(systemSettings)
    .where(eq(systemSettings.key, "license_signing_private_key"));

  if (!privateKeySetting) {
    return NextResponse.json(
      { error: "No signing keypair configured. Generate one in Settings first." },
      { status: 400 },
    );
  }

  const payload: LicensePayload = {
    licenseId: license.key,
    customerId: license.customerId ?? "",
    customerName: license.customerName,
    productId: license.productId ?? "",
    productName: license.productName,
    features: parsed.data.features,
    maxActivations: parsed.data.maxActivations,
    issuedAt: license.createdAt,
    expiresAt: license.expiresAt,
    metadata: parsed.data.metadata,
  };

  const signed = signLicense(payload, privateKeySetting.value);

  // Store the signed data
  await db
    .update(licenses)
    .set({
      licenseType: "signed",
      signedPayload: JSON.stringify(signed.payload),
      signature: signed.signature,
      features: parsed.data.features,
      maxActivations: parsed.data.maxActivations ?? null,
    })
    .where(eq(licenses.key, key));

  return NextResponse.json({
    payload: signed.payload,
    signature: signed.signature,
  });
}
