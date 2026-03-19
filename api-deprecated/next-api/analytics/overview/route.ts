import { db } from "@/lib/db";
import { products, customers, licenses, deals, invoices } from "@/lib/db/schema";
import { sql, eq, sum, count } from "drizzle-orm";
import { NextResponse } from "next/server";

export async function GET() {
  // Total revenue from products
  const [revRow] = await db
    .select({ total: sum(products.revenue), avgChange: sql<number>`avg(${products.change})` })
    .from(products);

  const totalRevenue = Number(revRow?.total ?? 0);
  const revenueChange = Number(revRow?.avgChange ?? 0);

  // Platform users (SaaS MAU)
  const [mauRow] = await db
    .select({ total: sum(products.mau) })
    .from(products)
    .where(eq(products.productType, "saas"));

  const platformUsers = Number(mauRow?.total ?? 0);

  // Active licenses
  const [licRow] = await db
    .select({ total: count() })
    .from(licenses)
    .where(eq(licenses.status, "active"));

  const activeLicenses = Number(licRow?.total ?? 0);

  // Total licenses for change display
  const [totalLicRow] = await db.select({ total: count() }).from(licenses);
  const totalLicenseCount = Number(totalLicRow?.total ?? 0);

  // Customer count + recent (last 30 days)
  const [custRow] = await db.select({ total: count() }).from(customers);
  const customerCount = Number(custRow?.total ?? 0);

  const thirtyDaysAgo = new Date();
  thirtyDaysAgo.setDate(thirtyDaysAgo.getDate() - 30);
  const [recentCustRow] = await db
    .select({ total: count() })
    .from(customers)
    .where(sql`${customers.createdAt} >= ${thirtyDaysAgo.toISOString()}`);
  const newCustomers = Number(recentCustRow?.total ?? 0);

  // Revenue chart — monthly deal values
  // deals.date is ISO varchar like "2024-01-15"
  const monthlyDeals = await db
    .select({
      month: sql<string>`to_char(${deals.date}::date, 'Mon')`,
      monthNum: sql<number>`extract(month from ${deals.date}::date)`,
      revenue: sum(deals.value),
    })
    .from(deals)
    .groupBy(
      sql`to_char(${deals.date}::date, 'Mon')`,
      sql`extract(month from ${deals.date}::date)`,
    )
    .orderBy(sql`extract(month from ${deals.date}::date)`);

  // Monthly target = total product target / 12
  const [targetRow] = await db.select({ total: sum(products.target) }).from(products);
  const monthlyTarget = Math.round(Number(targetRow?.total ?? 0) / 12);

  // MRR from active subscriptions (for billing)
  const [mrrRow] = await db
    .select({ total: sum(invoices.total) })
    .from(invoices)
    .where(eq(invoices.status, "paid"));
  const totalPaidInvoices = Number(mrrRow?.total ?? 0);

  const revenueChart = monthlyDeals.map((d) => ({
    month: d.month,
    revenue: Math.round(Number(d.revenue ?? 0)),
    target: monthlyTarget,
  }));

  // Format values for display
  const formatValue = (n: number) => {
    if (n >= 1_000_000) return `$${(n / 1_000_000).toFixed(1)}M`;
    if (n >= 1_000) return `$${(n / 1_000).toFixed(0)}k`;
    return `$${n}`;
  };

  const formatCount = (n: number) => {
    if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
    return String(n);
  };

  return NextResponse.json({
    totalRevenue: formatValue(totalRevenue),
    revenueChange: `+${revenueChange.toFixed(1)}%`,
    platformUsers: formatCount(platformUsers),
    platformUsersChange: platformUsers > 0
      ? `${formatCount(platformUsers)} total`
      : "No data",
    activeLicenses: String(activeLicenses),
    licensesChange: `+${totalLicenseCount - activeLicenses}`,
    customerCount: String(customerCount),
    newCustomers: newCustomers > 0 ? `+${newCustomers} this month` : `${customerCount} total`,
    totalPaidInvoices: formatValue(totalPaidInvoices),
    revenueChart,
  });
}
