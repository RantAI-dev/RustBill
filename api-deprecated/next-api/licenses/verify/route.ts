import { db } from "@/lib/db";
import { systemSettings } from "@/lib/db/schema";
import { parseLicenseFile, verifyLicense } from "@/lib/license-signing";
import { verifyLicenseSchema } from "@/lib/validations/licenses";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function POST(req: NextRequest) {
  const body = await req.json();

  const parsed = verifyLicenseSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  // Fetch public key
  const [publicKeySetting] = await db
    .select()
    .from(systemSettings)
    .where(eq(systemSettings.key, "license_signing_public_key"));

  if (!publicKeySetting) {
    return NextResponse.json(
      { error: "No signing keypair configured. Generate one in Settings first." },
      { status: 400 },
    );
  }

  try {
    const signed = parseLicenseFile(parsed.data.licenseFile);
    const valid = verifyLicense(signed, publicKeySetting.value);

    const now = new Date().toISOString().split("T")[0];
    const expired = signed.payload.expiresAt < now;

    return NextResponse.json({
      valid,
      expired,
      payload: valid ? signed.payload : null,
    });
  } catch {
    return NextResponse.json({
      valid: false,
      expired: false,
      payload: null,
      error: "Failed to parse license file",
    });
  }
}
