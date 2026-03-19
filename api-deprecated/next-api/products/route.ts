import { db } from "@/lib/db";
import { products, deals, licenses } from "@/lib/db/schema";
import { insertProductSchema } from "@/lib/validations/products";
import { desc, eq, sql, and } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET() {
  const rows = await db.select().from(products).orderBy(desc(products.createdAt));

  // Compute aggregated metrics for each product from deals & licenses
  const enriched = await Promise.all(
    rows.map(async (product) => {
      // Revenue: sum of deal values for this product
      const [revenueResult] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(eq(deals.productId, product.id));
      const computedRevenue = Number(revenueResult?.total ?? 0);

      // Revenue from previous month for change % calculation
      const now = new Date();
      const startOfThisMonth = new Date(now.getFullYear(), now.getMonth(), 1);
      const startOfLastMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1);

      const [thisMonthRev] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(and(
          eq(deals.productId, product.id),
          sql`${deals.date} >= ${startOfThisMonth.toISOString().split("T")[0]}`
        ));
      const [lastMonthRev] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(and(
          eq(deals.productId, product.id),
          sql`${deals.date} >= ${startOfLastMonth.toISOString().split("T")[0]}`,
          sql`${deals.date} < ${startOfThisMonth.toISOString().split("T")[0]}`
        ));

      const thisMonth = Number(thisMonthRev?.total ?? 0);
      const lastMonth = Number(lastMonthRev?.total ?? 0);
      const computedChange = lastMonth > 0
        ? Math.round(((thisMonth - lastMonth) / lastMonth) * 100 * 100) / 100
        : thisMonth > 0 ? 100 : 0;

      // Licensed-specific: units sold (deal count), active/total licenses
      let computedUnitsSold = product.unitsSold;
      let computedActiveLicenses = product.activeLicenses;
      let computedTotalLicenses = product.totalLicenses;

      if (product.productType === "licensed") {
        const [dealCount] = await db
          .select({ count: sql<string>`COUNT(*)` })
          .from(deals)
          .where(eq(deals.productId, product.id));
        computedUnitsSold = Number(dealCount?.count ?? 0);

        const [activeCount] = await db
          .select({ count: sql<string>`COUNT(*)` })
          .from(licenses)
          .where(and(eq(licenses.productId, product.id), eq(licenses.status, "active")));
        computedActiveLicenses = Number(activeCount?.count ?? 0);

        const [totalCount] = await db
          .select({ count: sql<string>`COUNT(*)` })
          .from(licenses)
          .where(eq(licenses.productId, product.id));
        computedTotalLicenses = Number(totalCount?.count ?? 0);
      }

      return {
        ...product,
        revenue: computedRevenue,
        change: computedChange,
        unitsSold: computedUnitsSold,
        activeLicenses: computedActiveLicenses,
        totalLicenses: computedTotalLicenses,
      };
    })
  );

  // Sort by computed revenue descending
  enriched.sort((a, b) => (b.revenue as number) - (a.revenue as number));

  return NextResponse.json(enriched);
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const parsed = insertProductSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const [row] = await db.insert(products).values(parsed.data).returning();
  return NextResponse.json(row, { status: 201 });
}
