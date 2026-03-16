import { db } from "@/lib/db";
import { licenseActivations, licenses } from "@/lib/db/schema";
import { eq, and, desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

// Dashboard: list all activations for a license
export async function GET(_req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const { key } = await params;

  // Verify license exists
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
      id: r.id,
      deviceId: r.deviceId,
      deviceName: r.deviceName,
      ipAddress: r.ipAddress,
      activatedAt: r.activatedAt,
      lastSeenAt: r.lastSeenAt,
    })),
    maxActivations: license.maxActivations,
  });
}

// Dashboard: deactivate a specific device (admin only)
export async function DELETE(req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const { key } = await params;
  const { searchParams } = new URL(req.url);
  const deviceId = searchParams.get("deviceId");

  if (!deviceId) {
    return NextResponse.json({ error: "deviceId query parameter required" }, { status: 400 });
  }

  const [deleted] = await db
    .delete(licenseActivations)
    .where(
      and(
        eq(licenseActivations.licenseKey, key),
        eq(licenseActivations.deviceId, deviceId),
      ),
    )
    .returning();

  if (!deleted) {
    return NextResponse.json({ error: "Activation not found" }, { status: 404 });
  }

  return NextResponse.json({ success: true });
}
