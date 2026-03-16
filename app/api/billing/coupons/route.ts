import { db } from "@/lib/db";
import { coupons, subscriptionDiscounts } from "@/lib/db/schema";
import { insertCouponSchema } from "@/lib/validations/billing";
import { desc, eq, sql } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET() {
  const rows = await db.select().from(coupons).orderBy(desc(coupons.createdAt));

  // Compute timesRedeemed from subscriptionDiscounts and auto-deactivate
  const enriched = await Promise.all(
    rows.map(async (coupon) => {
      // Count actual redemptions from subscriptionDiscounts table
      const [countResult] = await db
        .select({ count: sql<string>`COUNT(*)` })
        .from(subscriptionDiscounts)
        .where(eq(subscriptionDiscounts.couponId, coupon.id));
      const timesRedeemed = Number(countResult?.count ?? 0);

      // Auto-deactivate if maxRedemptions reached or expired
      let active = coupon.active;
      if (active) {
        const maxReached = coupon.maxRedemptions !== null && timesRedeemed >= coupon.maxRedemptions;
        const expired = coupon.validUntil !== null && new Date(coupon.validUntil) < new Date();
        if (maxReached || expired) {
          active = false;
          // Persist the deactivation
          await db
            .update(coupons)
            .set({ active: false, updatedAt: new Date() })
            .where(eq(coupons.id, coupon.id));
        }
      }

      return {
        ...coupon,
        timesRedeemed,
        active,
      };
    })
  );

  return NextResponse.json(enriched);
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const parsed = insertCouponSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const data = {
    ...parsed.data,
    validFrom: parsed.data.validFrom ? new Date(parsed.data.validFrom) : new Date(),
    validUntil: parsed.data.validUntil ? new Date(parsed.data.validUntil) : null,
  };

  const [row] = await db.insert(coupons).values(data).returning();
  return NextResponse.json(row, { status: 201 });
}
