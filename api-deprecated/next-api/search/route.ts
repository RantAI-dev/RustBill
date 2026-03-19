import { db } from "@/lib/db";
import { products, customers, deals, licenses, invoices, subscriptions, pricingPlans } from "@/lib/db/schema";
import { sql, eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(request: NextRequest) {
  const q = request.nextUrl.searchParams.get("q");
  if (!q || q.length < 2) {
    return NextResponse.json({ products: [], customers: [], deals: [], licenses: [], invoices: [], subscriptions: [] });
  }

  const pattern = `%${q}%`;

  const [productResults, customerResults, dealResults, licenseResults, invoiceResults, subscriptionResults] =
    await Promise.all([
      db
        .select({ id: products.id, name: products.name, productType: products.productType, revenue: products.revenue })
        .from(products)
        .where(sql`${products.name} ILIKE ${pattern}`)
        .limit(5),

      db
        .select({ id: customers.id, name: customers.name, email: customers.email, tier: customers.tier })
        .from(customers)
        .where(sql`${customers.name} ILIKE ${pattern} OR ${customers.email} ILIKE ${pattern} OR ${customers.contact} ILIKE ${pattern}`)
        .limit(5),

      db
        .select({ id: deals.id, company: deals.company, contact: deals.contact, value: deals.value, productName: deals.productName })
        .from(deals)
        .where(sql`${deals.company} ILIKE ${pattern} OR ${deals.contact} ILIKE ${pattern}`)
        .limit(5),

      db
        .select({ key: licenses.key, customerName: licenses.customerName, productName: licenses.productName, status: licenses.status })
        .from(licenses)
        .where(sql`${licenses.key} ILIKE ${pattern} OR ${licenses.customerName} ILIKE ${pattern} OR ${licenses.productName} ILIKE ${pattern}`)
        .limit(5),

      db
        .select({ id: invoices.id, invoiceNumber: invoices.invoiceNumber, total: invoices.total, status: invoices.status, customerName: customers.name })
        .from(invoices)
        .leftJoin(customers, eq(invoices.customerId, customers.id))
        .where(sql`${invoices.invoiceNumber} ILIKE ${pattern} OR ${customers.name} ILIKE ${pattern}`)
        .limit(5),

      db
        .select({ id: subscriptions.id, status: subscriptions.status, customerName: customers.name, planName: pricingPlans.name })
        .from(subscriptions)
        .leftJoin(customers, eq(subscriptions.customerId, customers.id))
        .leftJoin(pricingPlans, eq(subscriptions.planId, pricingPlans.id))
        .where(sql`${customers.name} ILIKE ${pattern} OR ${pricingPlans.name} ILIKE ${pattern}`)
        .limit(5),
    ]);

  return NextResponse.json({
    products: productResults,
    customers: customerResults,
    deals: dealResults,
    licenses: licenseResults,
    invoices: invoiceResults,
    subscriptions: subscriptionResults,
  });
}
