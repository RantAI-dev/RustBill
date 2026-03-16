import { db } from "@/lib/db";
import { licenseActivations, licenses } from "@/lib/db/schema";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq, desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

// Public API: list activations for a license (API key auth)
export async function GET(req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { key } = await params;

  const [license] = await db.select().from(licenses).where(eq(licenses.key, key));
  if (!license) {
    return NextResponse.json({ error: "License not found" }, { status: 404 });
  }

  const rows = await db
    .select()
    .from(licenseActivations)
    .where(eq(licenseActivations.licenseKey, key))
    .orderBy(desc(licenseActivations.activatedAt));

  return NextResponse.json({
    activations: rows.map((r) => ({
      deviceId: r.deviceId,
      deviceName: r.deviceName,
      activatedAt: r.activatedAt,
      lastSeenAt: r.lastSeenAt,
    })),
    count: rows.length,
    maxActivations: license.maxActivations,
  });
}
