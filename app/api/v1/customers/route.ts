import { db } from "@/lib/db";
import { customers, customerProducts, products } from "@/lib/db/schema";
import { insertCustomerSchema } from "@/lib/validations/customers";
import { authenticateApiKey } from "@/lib/api-auth";
import { desc, eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const rows = await db.select().from(customers).orderBy(desc(customers.totalRevenue));

  const result = await Promise.all(
    rows.map(async (customer) => {
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

      return {
        ...customer,
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

  return NextResponse.json(result);
}

export async function POST(req: NextRequest) {
  const auth = await authenticateApiKey(req);
  if (!auth.success) return auth.response;

  const body = await req.json();
  const parsed = insertCustomerSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }
  const values = {
    ...parsed.data,
    totalRevenue: parsed.data.totalRevenue ?? 0,
    healthScore: parsed.data.healthScore ?? 50,
    trend: parsed.data.trend ?? "stable",
    lastContact: parsed.data.lastContact ?? "Today",
  };
  const [row] = await db.insert(customers).values(values).returning();
  return NextResponse.json({ ...row, products: [] }, { status: 201 });
}
