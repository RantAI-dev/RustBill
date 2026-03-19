import { db } from "@/lib/db";
import { pricingPlans, products } from "@/lib/db/schema";
import { updatePlanSchema } from "@/lib/validations/billing";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db
    .select()
    .from(pricingPlans)
    .leftJoin(products, eq(pricingPlans.productId, products.id))
    .where(eq(pricingPlans.id, id));

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({
    ...row.pricing_plans,
    productName: row.products?.name ?? null,
    productType: row.products?.productType ?? null,
  });
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();
  const parsed = updatePlanSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const { ...fields } = parsed.data;
  const setData = Object.fromEntries(
    Object.entries({ ...fields, updatedAt: new Date() }).filter(([, v]) => v !== undefined)
  );

  const [row] = await db
    .update(pricingPlans)
    .set(setData as typeof pricingPlans.$inferInsert)
    .where(eq(pricingPlans.id, id))
    .returning();

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.delete(pricingPlans).where(eq(pricingPlans.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({ success: true });
}
