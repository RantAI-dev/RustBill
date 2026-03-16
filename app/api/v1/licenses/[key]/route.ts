import { db } from "@/lib/db";
import { licenses, customers, products } from "@/lib/db/schema";
import { updateLicenseSchema } from "@/lib/validations/licenses";
import { authenticateApiKey } from "@/lib/api-auth";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function PUT(req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { key } = await params;
  const body = await req.json();
  const parsed = updateLicenseSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const [row] = await db.update(licenses).set(parsed.data).where(eq(licenses.key, key)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

  // JOIN to return fresh names
  const [customer] = row.customerId ? await db.select().from(customers).where(eq(customers.id, row.customerId)) : [null];
  const [product] = row.productId ? await db.select().from(products).where(eq(products.id, row.productId)) : [null];

  return NextResponse.json({
    key: row.key,
    customer: customer?.name ?? row.customerName,
    customerId: row.customerId,
    product: product?.name ?? row.productName,
    productId: row.productId,
    status: row.status,
    createdAt: row.createdAt,
    expiresAt: row.expiresAt,
  });
}

export async function DELETE(req: NextRequest, { params }: { params: Promise<{ key: string }> }) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const { key } = await params;
  const [row] = await db.delete(licenses).where(eq(licenses.key, key)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({ success: true });
}
