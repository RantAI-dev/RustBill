import { db } from "@/lib/db";
import { coupons, subscriptionDiscounts } from "@/lib/db/schema";
import { updateCouponSchema, insertSubscriptionDiscountSchema } from "@/lib/validations/billing";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.select().from(coupons).where(eq(coupons.id, id));
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

  // Include subscription discounts using this coupon
  const discounts = await db
    .select()
    .from(subscriptionDiscounts)
    .where(eq(subscriptionDiscounts.couponId, id));

  return NextResponse.json({ ...row, subscriptionDiscounts: discounts });
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();

  // Handle applying coupon to subscription
  if (body.action === "apply") {
    const parsed = insertSubscriptionDiscountSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    // Increment redemption count
    const [coupon] = await db.select().from(coupons).where(eq(coupons.id, id));
    if (!coupon) return NextResponse.json({ error: "Not found" }, { status: 404 });
    if (coupon.maxRedemptions && coupon.timesRedeemed >= coupon.maxRedemptions) {
      return NextResponse.json({ error: "Coupon has reached maximum redemptions" }, { status: 400 });
    }

    const [discount] = await db.insert(subscriptionDiscounts).values({
      subscriptionId: parsed.data.subscriptionId,
      couponId: id,
      expiresAt: parsed.data.expiresAt ? new Date(parsed.data.expiresAt) : null,
    }).returning();

    await db.update(coupons).set({ timesRedeemed: coupon.timesRedeemed + 1, updatedAt: new Date() }).where(eq(coupons.id, id));

    return NextResponse.json(discount, { status: 201 });
  }

  // Regular update
  const parsed = updateCouponSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const data: Record<string, unknown> = { ...parsed.data, updatedAt: new Date() };
  if (parsed.data.validFrom) data.validFrom = new Date(parsed.data.validFrom);
  if (parsed.data.validUntil) data.validUntil = new Date(parsed.data.validUntil);

  const [row] = await db
    .update(coupons)
    .set(data)
    .where(eq(coupons.id, id))
    .returning();

  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json(row);
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const [row] = await db.delete(coupons).where(eq(coupons.id, id)).returning();
  if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
  return NextResponse.json({ success: true });
}
