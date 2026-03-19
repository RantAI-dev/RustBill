import { db } from "@/lib/db";
import { products, deals, licenses } from "@/lib/db/schema";
import { updateProductSchema } from "@/lib/validations/products";
import { eq, sql, and } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [product] = await db.select().from(products).where(eq(products.id, id));
  if (!product) return NextResponse.json({ error: "Not found" }, { status: 404 });

  // Compute revenue from deals
  const [revenueResult] = await db
    .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
    .from(deals)
    .where(eq(deals.productId, id));
  const computedRevenue = Number(revenueResult?.total ?? 0);

  // Month-over-month change
  const now = new Date();
  const startOfThisMonth = new Date(now.getFullYear(), now.getMonth(), 1);
  const startOfLastMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1);
  const [thisMonthRev] = await db
    .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
    .from(deals)
    .where(and(eq(deals.productId, id), sql`${deals.date} >= ${startOfThisMonth.toISOString().split("T")[0]}`));
  const [lastMonthRev] = await db
    .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
    .from(deals)
    .where(and(eq(deals.productId, id), sql`${deals.date} >= ${startOfLastMonth.toISOString().split("T")[0]}`, sql`${deals.date} < ${startOfThisMonth.toISOString().split("T")[0]}`));
  const thisMonth = Number(thisMonthRev?.total ?? 0);
  const lastMonth = Number(lastMonthRev?.total ?? 0);
  const computedChange = lastMonth > 0 ? Math.round(((thisMonth - lastMonth) / lastMonth) * 100 * 100) / 100 : thisMonth > 0 ? 100 : 0;

  // Licensed-specific metrics
  let computedUnitsSold = product.unitsSold;
  let computedActiveLicenses = product.activeLicenses;
  let computedTotalLicenses = product.totalLicenses;
  if (product.productType === "licensed") {
    const [dealCount] = await db.select({ count: sql<string>`COUNT(*)` }).from(deals).where(eq(deals.productId, id));
    computedUnitsSold = Number(dealCount?.count ?? 0);
    const [activeCount] = await db.select({ count: sql<string>`COUNT(*)` }).from(licenses).where(and(eq(licenses.productId, id), eq(licenses.status, "active")));
    computedActiveLicenses = Number(activeCount?.count ?? 0);
    const [totalCount] = await db.select({ count: sql<string>`COUNT(*)` }).from(licenses).where(eq(licenses.productId, id));
    computedTotalLicenses = Number(totalCount?.count ?? 0);
  }

  return NextResponse.json({
    ...product,
    revenue: computedRevenue,
    change: computedChange,
    unitsSold: computedUnitsSold,
    activeLicenses: computedActiveLicenses,
    totalLicenses: computedTotalLicenses,
  });
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();
  const parsed = updateProductSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const { ...fields } = parsed.data;
  const setData = Object.fromEntries(
    Object.entries({ ...fields, updatedAt: new Date() }).filter(([, v]) => v !== undefined)
  );
  const [row] = await db.update(products).set(setData as typeof products.$inferInsert).where(eq(products.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.delete(products).where(eq(products.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({ success: true });
}
