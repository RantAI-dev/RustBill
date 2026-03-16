import { db } from "@/lib/db";
import { products } from "@/lib/db/schema";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { id } = await params;
  const [row] = await db.select().from(products).where(eq(products.id, id));
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}
