import { db } from "@/lib/db";
import { deals, customers, products, licenses } from "@/lib/db/schema";
import { updateDealSchema } from "@/lib/validations/deals";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { generateLicenseKey } from "@/lib/license-keys";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.select().from(deals)
    .leftJoin(customers, eq(deals.customerId, customers.id))
    .leftJoin(products, eq(deals.productId, products.id))
    .where(eq(deals.id, id));
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({
    ...row.deals,
    company: row.customers?.name ?? row.deals.company,
    contact: row.customers?.contact ?? row.deals.contact,
    email: row.customers?.email ?? row.deals.email,
    productName: row.products?.name ?? row.deals.productName,
    productType: row.products?.productType ?? row.deals.productType,
  });
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();

  // Fetch current deal
  const [current] = await db.select().from(deals).where(eq(deals.id, id));
  if (!current) return NextResponse.json({ error: "Not found" }, { status: 404 });

  // Auto-populate from customer FK
  let customerName = current.company ?? "";
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
  let productName = current.productName ?? "";
  if (body.productId) {
    const [product] = await db.select().from(products).where(eq(products.id, body.productId));
    if (product) {
      body.productName = product.name;
      body.productType = product.productType;
      productName = product.name;
    }
  }

  const parsed = updateDealSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const data = { ...parsed.data };
  const customExpiresAt = data.licenseExpiresAt;
  delete data.licenseExpiresAt;

  const newProductType = data.productType ?? current.productType;
  const existingKey = data.licenseKey ?? current.licenseKey;

  // Auto-create license for licensed product deals
  if (newProductType === "licensed" && !existingKey) {
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
      customerId: data.customerId ?? current.customerId ?? null,
      customerName,
      productId: data.productId ?? current.productId ?? null,
      productName,
      status: "active",
      createdAt: today.toISOString().split("T")[0],
      expiresAt,
    });
  }

  const setData = Object.fromEntries(
    Object.entries({ ...data, updatedAt: new Date() }).filter(([, v]) => v !== undefined)
  );
  const [row] = await db.update(deals).set(setData as typeof deals.$inferInsert).where(eq(deals.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.delete(deals).where(eq(deals.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({ success: true });
}
