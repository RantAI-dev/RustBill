import { db } from "@/lib/db";
import { creditNotes, creditNoteItems, customers, invoices } from "@/lib/db/schema";
import { updateCreditNoteSchema } from "@/lib/validations/billing";
import { eq, and, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;

    const [row] = await db
      .select()
      .from(creditNotes)
      .leftJoin(customers, eq(creditNotes.customerId, customers.id))
      .leftJoin(invoices, eq(creditNotes.invoiceId, invoices.id))
      .where(and(eq(creditNotes.id, id), isNull(creditNotes.deletedAt)));

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });

    // Non-admin users can only see their own credit notes
    if (auth.user.role !== "admin" && row.credit_notes.customerId !== auth.user.customerId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }

    const items = await db
      .select()
      .from(creditNoteItems)
      .where(eq(creditNoteItems.creditNoteId, id));

    return NextResponse.json({
      ...row.credit_notes,
      customerName: row.customers?.name ?? null,
      invoiceNumber: row.invoices?.invoiceNumber ?? null,
      items,
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/credit-notes/[id]");
  }
}

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;
    const body = await req.json();
    const parsed = updateCreditNoteSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const data: Record<string, unknown> = { ...parsed.data, updatedAt: new Date() };
    if (parsed.data.issuedAt) data.issuedAt = new Date(parsed.data.issuedAt);
    if (parsed.data.status === "issued" && !parsed.data.issuedAt) data.issuedAt = new Date();

    const [row] = await db
      .update(creditNotes)
      .set(data)
      .where(and(eq(creditNotes.id, id), isNull(creditNotes.deletedAt)))
      .returning();

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
    return NextResponse.json(row);
  } catch (error) {
    return handleApiError(error, "PUT /api/billing/credit-notes/[id]");
  }
}

export async function DELETE(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;

    // Soft delete
    const [row] = await db
      .update(creditNotes)
      .set({ deletedAt: new Date(), updatedAt: new Date() })
      .where(and(eq(creditNotes.id, id), isNull(creditNotes.deletedAt)))
      .returning();

    if (!row) return NextResponse.json({ error: "Not found" }, { status: 404 });
    return NextResponse.json({ success: true });
  } catch (error) {
    return handleApiError(error, "DELETE /api/billing/credit-notes/[id]");
  }
}
