import { db } from "@/lib/db";
import { pricingPlans, products } from "@/lib/db/schema";
import { insertPlanSchema } from "@/lib/validations/billing";
import { desc, eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET() {
  const rows = await db
    .select()
    .from(pricingPlans)
    .leftJoin(products, eq(pricingPlans.productId, products.id))
    .orderBy(desc(pricingPlans.createdAt));

  const mapped = rows.map((r) => ({
    ...r.pricing_plans,
    productName: r.products?.name ?? null,
    productType: r.products?.productType ?? null,
  }));

  return NextResponse.json(mapped);
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const parsed = insertPlanSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const [row] = await db.insert(pricingPlans).values(parsed.data).returning();
  return NextResponse.json(row, { status: 201 });
}
