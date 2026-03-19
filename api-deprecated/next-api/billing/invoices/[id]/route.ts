import { db } from "@/lib/db";
import { invoices, invoiceItems, payments, customers } from "@/lib/db/schema";
import { updateInvoiceSchema } from "@/lib/validations/billing";
import { withAuth, withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { eq, and, sql, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const [row] = await db
      .select()
      .from(invoices)
      .leftJoin(customers, eq(invoices.customerId, customers.id))
      .where(and(eq(invoices.id, id), isNull(invoices.deletedAt)));

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

    // Customer isolation: customers can only see their own invoices
    if (auth.user.role === "customer" && auth.user.customerId) {
      if (row.invoices.customerId !== auth.user.customerId) {
        return NextResponse.json({ error: "Not found" }, { status: 404 });
      }
    }

    const items = await db.select().from(invoiceItems).where(eq(invoiceItems.invoiceId, id));
    const pmts = await db.select().from(payments).where(eq(payments.invoiceId, id));

    return NextResponse.json({
      ...row.invoices,
      customerName: row.customers?.name ?? null,
      items,
      payments: pmts,
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/invoices/[id]");
  }
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const body = await req.json();
    const parsed = updateInvoiceSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    // Extract version for optimistic locking
    const { version, ...updateFields } = parsed.data as Record<string, unknown> & { version?: number };
    if (version === undefined || version === null) {
      return NextResponse.json(
        { error: "version field is required for updates" },
        { status: 400 },
      );
    }

    const data: Record<string, unknown> = { ...updateFields, updatedAt: new Date() };
    if (updateFields.dueAt) data.dueAt = new Date(updateFields.dueAt as string);
    if (updateFields.paidAt) data.paidAt = new Date(updateFields.paidAt as string);

    // Optimistic locking: only update if version matches, then increment
    const [row] = await db
      .update(invoices)
      .set({ ...data, version: sql`${invoices.version} + 1` })
      .where(
        and(
          eq(invoices.id, id),
          eq(invoices.version, version),
          isNull(invoices.deletedAt),
        ),
      )
      .returning();

    if (!row) {
      // Check if the invoice exists at all to distinguish 404 vs 409
      const [existing] = await db
        .select({ id: invoices.id, version: invoices.version })
        .from(invoices)
        .where(and(eq(invoices.id, id), isNull(invoices.deletedAt)));

      if (!existing) {
        return NextResponse.json({ error: "Not found" }, { status: 404 });
      }

      return NextResponse.json(
        { error: "Conflict: invoice was modified by another request", currentVersion: existing.version },
        { status: 409 },
      );
    }

    return NextResponse.json(row);
  } catch (error) {
    return handleApiError(error, "PUT /api/billing/invoices/[id]");
  }
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAdmin();
    if (!auth.success) return auth.response;

    const { id } = await params;

    // Fetch the invoice first to check status
    const [existing] = await db
      .select()
      .from(invoices)
      .where(and(eq(invoices.id, id), isNull(invoices.deletedAt)));

    if (!existing) {
      return NextResponse.json({ error: "Not found" }, { status: 404 });
    }

    // Block deletion of issued or paid invoices
    if (existing.status === "issued" || existing.status === "paid") {
      return NextResponse.json(
        { error: `Cannot delete an invoice with status "${existing.status}". Void it instead.` },
        { status: 400 },
      );
    }

    // Soft delete
    const [row] = await db
      .update(invoices)
      .set({ deletedAt: new Date(), updatedAt: new Date() })
      .where(eq(invoices.id, id))
      .returning();

    return NextResponse.json({ success: true, id: row.id });
  } catch (error) {
    return handleApiError(error, "DELETE /api/billing/invoices/[id]");
  }
}
