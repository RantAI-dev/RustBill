import { db } from "@/lib/db";
import { customers, customerProducts, products, deals, subscriptions, invoices } from "@/lib/db/schema";
import { insertCustomerSchema } from "@/lib/validations/customers";
import { desc, eq, sql, and } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

// Compute health score from payment recency, subscription status, and deal frequency
function computeHealthScore(metrics: {
  daysSinceLastDeal: number | null;
  hasActiveSubscription: boolean;
  paidInvoiceRatio: number;
}): number {
  let score = 50; // baseline

  // Recency factor (40%): more recent = higher score
  if (metrics.daysSinceLastDeal !== null) {
    if (metrics.daysSinceLastDeal <= 30) score += 20;
    else if (metrics.daysSinceLastDeal <= 90) score += 10;
    else if (metrics.daysSinceLastDeal <= 180) score += 0;
    else score -= 10;
  } else {
    score -= 15; // no deals at all
  }

  // Subscription status (30%)
  if (metrics.hasActiveSubscription) score += 15;

  // Payment reliability (30%)
  score += Math.round(metrics.paidInvoiceRatio * 15);

  return Math.max(0, Math.min(100, score));
}

// Compute trend by comparing last 3 months revenue vs prior 3 months
function computeTrend(recent: number, prior: number): "up" | "down" | "stable" {
  if (prior === 0 && recent === 0) return "stable";
  if (prior === 0 && recent > 0) return "up";
  const changePercent = ((recent - prior) / prior) * 100;
  if (changePercent > 5) return "up";
  if (changePercent < -5) return "down";
  return "stable";
}

export async function GET() {
  const rows = await db.select().from(customers).orderBy(desc(customers.createdAt));

  const result = await Promise.all(
    rows.map(async (customer) => {
      // Customer products
      const cp = await db
        .select({
          id: customerProducts.id,
          productId: customerProducts.productId,
          licenseKeys: customerProducts.licenseKeys,
          mau: customerProducts.mau,
          apiCalls: customerProducts.apiCalls,
          productName: products.name,
          productType: products.productType,
        })
        .from(customerProducts)
        .innerJoin(products, eq(customerProducts.productId, products.id))
        .where(eq(customerProducts.customerId, customer.id));

      // Total revenue from deals
      const [revenueResult] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(eq(deals.customerId, customer.id));
      const computedRevenue = Number(revenueResult?.total ?? 0);

      // Last deal date for lastContact
      const [lastDealResult] = await db
        .select({ lastDate: sql<string>`MAX(${deals.date})` })
        .from(deals)
        .where(eq(deals.customerId, customer.id));
      const lastDealDate = lastDealResult?.lastDate ?? null;

      // Format lastContact as relative time
      let computedLastContact = "No deals";
      let daysSinceLastDeal: number | null = null;
      if (lastDealDate) {
        const dealDate = new Date(lastDealDate);
        const now = new Date();
        daysSinceLastDeal = Math.floor((now.getTime() - dealDate.getTime()) / (1000 * 60 * 60 * 24));
        if (daysSinceLastDeal === 0) computedLastContact = "Today";
        else if (daysSinceLastDeal === 1) computedLastContact = "Yesterday";
        else if (daysSinceLastDeal < 30) computedLastContact = `${daysSinceLastDeal} days ago`;
        else if (daysSinceLastDeal < 365) computedLastContact = `${Math.floor(daysSinceLastDeal / 30)} months ago`;
        else computedLastContact = `${Math.floor(daysSinceLastDeal / 365)} years ago`;
      }

      // Trend: compare last 3 months vs prior 3 months
      const now = new Date();
      const threeMonthsAgo = new Date(now.getFullYear(), now.getMonth() - 3, 1);
      const sixMonthsAgo = new Date(now.getFullYear(), now.getMonth() - 6, 1);

      const [recentRev] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(and(
          eq(deals.customerId, customer.id),
          sql`${deals.date} >= ${threeMonthsAgo.toISOString().split("T")[0]}`
        ));
      const [priorRev] = await db
        .select({ total: sql<string>`COALESCE(SUM(CAST(${deals.value} AS numeric)), 0)` })
        .from(deals)
        .where(and(
          eq(deals.customerId, customer.id),
          sql`${deals.date} >= ${sixMonthsAgo.toISOString().split("T")[0]}`,
          sql`${deals.date} < ${threeMonthsAgo.toISOString().split("T")[0]}`
        ));
      const computedTrend = computeTrend(Number(recentRev?.total ?? 0), Number(priorRev?.total ?? 0));

      // Health score components
      const [activeSubCount] = await db
        .select({ count: sql<string>`COUNT(*)` })
        .from(subscriptions)
        .where(and(eq(subscriptions.customerId, customer.id), eq(subscriptions.status, "active")));
      const hasActiveSubscription = Number(activeSubCount?.count ?? 0) > 0;

      const [totalInv] = await db
        .select({ count: sql<string>`COUNT(*)` })
        .from(invoices)
        .where(eq(invoices.customerId, customer.id));
      const [paidInv] = await db
        .select({ count: sql<string>`COUNT(*)` })
        .from(invoices)
        .where(and(eq(invoices.customerId, customer.id), eq(invoices.status, "paid")));
      const paidInvoiceRatio = Number(totalInv?.count ?? 0) > 0
        ? Number(paidInv?.count ?? 0) / Number(totalInv?.count ?? 0)
        : 0.5; // neutral if no invoices

      const computedHealthScore = computeHealthScore({
        daysSinceLastDeal,
        hasActiveSubscription,
        paidInvoiceRatio,
      });

      return {
        ...customer,
        totalRevenue: computedRevenue,
        healthScore: computedHealthScore,
        trend: computedTrend,
        lastContact: computedLastContact,
        products: cp.map((p) => ({
          type: p.productType,
          name: p.productName,
          licenseKeys: p.licenseKeys ?? undefined,
          mau: p.mau ?? undefined,
          apiCalls: p.apiCalls ?? undefined,
        })),
      };
    })
  );

  // Sort by computed revenue descending
  result.sort((a, b) => (b.totalRevenue as number) - (a.totalRevenue as number));

  return NextResponse.json(result);
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const parsed = insertCustomerSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const values = {
    ...parsed.data,
    totalRevenue: parsed.data.totalRevenue ?? 0,
    healthScore: parsed.data.healthScore ?? 50,
    trend: parsed.data.trend ?? ("stable" as const),
    lastContact: parsed.data.lastContact ?? "Today",
  };
  const [row] = await db.insert(customers).values(values as typeof customers.$inferInsert).returning();
  return NextResponse.json({ ...row, products: [] }, { status: 201 });
}
