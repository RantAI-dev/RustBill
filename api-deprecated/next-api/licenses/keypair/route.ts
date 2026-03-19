import { db } from "@/lib/db";
import { systemSettings } from "@/lib/db/schema";
import { generateKeypair } from "@/lib/license-signing";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET() {
  const [row] = await db
    .select()
    .from(systemSettings)
    .where(eq(systemSettings.key, "license_signing_public_key"));

  return NextResponse.json({
    hasKeypair: !!row,
    publicKey: row?.value ?? null,
  });
}

export async function POST(req: NextRequest) {
  const body = await req.json().catch(() => ({}));

  // Check if keypair already exists
  const [existing] = await db
    .select()
    .from(systemSettings)
    .where(eq(systemSettings.key, "license_signing_public_key"));

  if (existing && !body.confirm) {
    return NextResponse.json(
      { error: "Keypair already exists. Pass { confirm: true } to regenerate. This will invalidate all previously signed licenses." },
      { status: 409 },
    );
  }

  const { publicKey, privateKey } = generateKeypair();

  // Upsert both keys
  await db
    .insert(systemSettings)
    .values({ key: "license_signing_private_key", value: privateKey, sensitive: true })
    .onConflictDoUpdate({
      target: systemSettings.key,
      set: { value: privateKey, updatedAt: new Date() },
    });

  await db
    .insert(systemSettings)
    .values({ key: "license_signing_public_key", value: publicKey, sensitive: false })
    .onConflictDoUpdate({
      target: systemSettings.key,
      set: { value: publicKey, updatedAt: new Date() },
    });

  return NextResponse.json({ publicKey }, { status: 201 });
}
