import { db } from "@/lib/db";
import { deals, customers, products, licenses } from "@/lib/db/schema";
import { insertDealSchema } from "@/lib/validations/deals";
import { authenticateApiKey } from "@/lib/api-auth";
import { desc, eq, and } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { generateLicenseKey } from "@/lib/license-keys";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { searchParams } = new URL(req.url);
  const type = searchParams.get("type");
  const dealType = searchParams.get("dealType");

  const conditions = [];
  if (type && type !== "all") conditions.push(eq(deals.productType, type as "licensed" | "saas" | "api"));
  if (dealType && dealType !== "all") conditions.push(eq(deals.dealType, dealType as "sale" | "trial" | "partner"));

  const rows = conditions.length > 0
    ? await db.select().from(deals)
        .leftJoin(customers, eq(deals.customerId, customers.id))
        .leftJoin(products, eq(deals.productId, products.id))
        .where(and(...conditions))
        .orderBy(desc(deals.createdAt))
    : await db.select().from(deals)
        .leftJoin(customers, eq(deals.customerId, customers.id))
        .leftJoin(products, eq(deals.productId, products.id))
        .orderBy(desc(deals.createdAt));

  const mapped = rows.map((r) => ({
    ...r.deals,
    company: r.customers?.name ?? r.deals.company,
    contact: r.customers?.contact ?? r.deals.contact,
    email: r.customers?.email ?? r.deals.email,
    productName: r.products?.name ?? r.deals.productName,
    productType: r.products?.productType ?? r.deals.productType,
  }));

  return NextResponse.json(mapped);
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const body = await req.json();

  // Auto-populate from customer FK
  let customerName = body.company ?? "";
  if (body.customerId) {
    const [customer] = await db.select().from(customers).where(eq(customers.id, body.customerId));
    if (customer) {
      body.company = customer.name;
      body.contact = customer.contact;
      body.email = customer.email;
      customerName = customer.name;
    }
  }

  // Auto-populate from product FK
  let productName = body.productName ?? "";
  if (body.productId) {
    const [product] = await db.select().from(products).where(eq(products.id, body.productId));
    if (product) {
      body.productName = product.name;
      body.productType = product.productType;
      productName = product.name;
    }
  }

  const parsed = insertDealSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  // Auto-create license for licensed product deals
  const data = { ...parsed.data };
  const customExpiresAt = data.licenseExpiresAt;
  delete data.licenseExpiresAt;

  if (data.productType === "licensed" && !data.licenseKey) {
    const key = generateLicenseKey();
    data.licenseKey = key;

    const today = new Date();
    let expiresAt: string;
    if (customExpiresAt) {
      expiresAt = customExpiresAt;
    } else {
      const expDate = new Date(today);
      expDate.setFullYear(expDate.getFullYear() + 1);
      expiresAt = expDate.toISOString().split("T")[0];
    }

    await db.insert(licenses).values({
      key,
      customerId: data.customerId ?? null,
      customerName,
      productId: data.productId ?? null,
      productName,
      status: "active",
      createdAt: today.toISOString().split("T")[0],
      expiresAt,
    });
  }

  const [row] = await db.insert(deals).values(data).returning();
  return NextResponse.json(row, { status: 201 });
}
