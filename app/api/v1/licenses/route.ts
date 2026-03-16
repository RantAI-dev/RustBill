import { db } from "@/lib/db";
import { licenses, customers, products } from "@/lib/db/schema";
import { insertLicenseSchema } from "@/lib/validations/licenses";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq, desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { searchParams } = new URL(req.url);
  const status = searchParams.get("status");

  const rows = status && status !== "all"
    ? await db.select().from(licenses)
        .leftJoin(customers, eq(licenses.customerId, customers.id))
        .leftJoin(products, eq(licenses.productId, products.id))
        .where(eq(licenses.status, status as "active" | "expired" | "revoked" | "suspended"))
        .orderBy(desc(licenses.createdAt))
    : await db.select().from(licenses)
        .leftJoin(customers, eq(licenses.customerId, customers.id))
        .leftJoin(products, eq(licenses.productId, products.id))
        .orderBy(desc(licenses.createdAt));

  const mapped = rows.map((r) => ({
    key: r.licenses.key,
    customer: r.customers?.name ?? r.licenses.customerName,
    customerId: r.licenses.customerId,
    product: r.products?.name ?? r.licenses.productName,
    productId: r.licenses.productId,
    status: r.licenses.status,
    createdAt: r.licenses.createdAt,
    expiresAt: r.licenses.expiresAt,
  }));

  return NextResponse.json(mapped);
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const body = await req.json();

  // Auto-populate names from FKs
  if (body.customerId) {
    const [customer] = await db.select().from(customers).where(eq(customers.id, body.customerId));
    if (customer) body.customerName = customer.name;
  }
  if (body.productId) {
    const [product] = await db.select().from(products).where(eq(products.id, body.productId));
    if (product) body.productName = product.name;
  }

  const parsed = insertLicenseSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const [row] = await db.insert(licenses).values(parsed.data).returning();
  return NextResponse.json({
    key: row.key,
    customer: row.customerName,
    customerId: row.customerId,
    product: row.productName,
    productId: row.productId,
    status: row.status,
    createdAt: row.createdAt,
    expiresAt: row.expiresAt,
  }, { status: 201 });
}
