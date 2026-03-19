import { db } from "@/lib/db";
import { licenses } from "@/lib/db/schema";
import { toLicenseFile } from "@/lib/license-signing";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const { key } = await params;

  const [license] = await db.select().from(licenses).where(eq(licenses.key, key));
  if (!license) {
    return NextResponse.json({ error: "License not found" }, { status: 404 });
  }

  if (license.licenseType !== "signed" || !license.signedPayload || !license.signature) {
    return NextResponse.json(
      { error: "License has not been signed. Sign it first before exporting." },
      { status: 400 },
    );
  }

  const signed = {
    payload: JSON.parse(license.signedPayload),
    signature: license.signature,
  };

  const fileContent = toLicenseFile(signed);

  return new NextResponse(fileContent, {
    status: 200,
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Content-Disposition": `attachment; filename="${key}.lic"`,
    },
  });
}
