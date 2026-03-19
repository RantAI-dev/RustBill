import { db } from "@/lib/db";
import { deals, products, invoices, customers } from "@/lib/db/schema";
import { sql, sum, count, desc } from "drizzle-orm";
import { NextResponse } from "next/server";

export async function GET() {
  // 1. Conversion data — monthly deal count
  const monthlyDeals = await db
    .select({
      month: sql<string>`to_char(to_date(${deals.date}, 'Mon DD, YYYY'), 'Mon')`,
      monthNum: sql<number>`extract(month from to_date(${deals.date}, 'Mon DD, YYYY'))`,
      dealCount: count(),
    })
    .from(deals)
    .groupBy(
      sql`to_char(to_date(${deals.date}, 'Mon DD, YYYY'), 'Mon')`,
      sql`extract(month from to_date(${deals.date}, 'Mon DD, YYYY'))`,
    )
    .orderBy(sql`extract(month from to_date(${deals.date}, 'Mon DD, YYYY'))`);

  // Monthly customer count (total customers up to each month)
  const totalCustomers = await db.select({ total: count() }).from(customers);
  const customerCount = Number(totalCustomers[0]?.total ?? 1);

  // Conversion rate = deals / customers ratio scaled
  const conversionData = monthlyDeals.map((d) => ({
    month: d.month,
    rate: Math.round((Number(d.dealCount) / customerCount) * 100),
  }));

  // 2. Revenue by product type
  const productTypeRevenue = await db
    .select({
      type: products.productType,
      revenue: sum(products.revenue),
    })
    .from(products)
    .groupBy(products.productType);

  const totalRevenue = productTypeRevenue.reduce((sum, r) => sum + Number(r.revenue ?? 0), 0);
  const typeLabels: Record<string, string> = {
    licensed: "Licensed Products",
    saas: "AI Chat Platform",
    api: "AI Chat API",
  };
  const typeColors: Record<string, string> = {
    licensed: "oklch(0.7 0.18 220)",
    saas: "oklch(0.75 0.18 55)",
    api: "oklch(0.65 0.2 25)",
  };

  const sourceData = productTypeRevenue.map((r) => ({
    name: typeLabels[r.type] ?? r.type,
    value: totalRevenue > 0 ? Math.round((Number(r.revenue ?? 0) / totalRevenue) * 100) : 0,
    color: typeColors[r.type] ?? "oklch(0.5 0 0)",
  }));

  // 3. Recent reports — use recent invoices as generated reports
  const recentInvoices = await db
    .select({
      id: invoices.id,
      invoiceNumber: invoices.invoiceNumber,
      status: invoices.status,
      total: invoices.total,
      createdAt: invoices.createdAt,
      customerName: customers.name,
    })
    .from(invoices)
    .leftJoin(customers, sql`${invoices.customerId} = ${customers.id}`)
    .orderBy(desc(invoices.createdAt))
    .limit(5);

  const reports = recentInvoices.map((inv) => ({
    id: inv.id,
    name: `Invoice ${inv.invoiceNumber} — ${inv.customerName}`,
    type: "Invoice",
    date: inv.createdAt.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" }),
    status: inv.status === "draft" ? "generating" : "ready",
  }));

  return NextResponse.json({
    conversionData,
    sourceData,
    reports: reports.slice(0, 5),
    yoyChange: conversionData.length >= 2
      ? `+${Math.abs(conversionData[conversionData.length - 1].rate - conversionData[0].rate)}%`
      : "+0%",
  });
}
