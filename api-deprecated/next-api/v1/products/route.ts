import { db } from "@/lib/db";
import { products } from "@/lib/db/schema";
import { authenticateApiKey } from "@/lib/api-auth";
import { desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const rows = await db.select().from(products).orderBy(desc(products.revenue));
  return NextResponse.json(rows);
}
