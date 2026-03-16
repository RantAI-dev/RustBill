import { db } from "@/lib/db";
import { creditNotes, creditNoteItems, customers, invoices } from "@/lib/db/schema";
import { insertCreditNoteSchema } from "@/lib/validations/billing";
import { desc, eq, and, sql, isNull } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

async function generateCreditNoteNumber(): Promise<string> {
  const year = new Date().getFullYear();
  const prefix = `CN-${year}-`;

  const [result] = await db
    .select({ maxNum: sql<string>`MAX(${creditNotes.creditNoteNumber})` })
    .from(creditNotes)
    .where(sql`${creditNotes.creditNoteNumber} LIKE ${prefix + "%"}`);

  const lastNum = result?.maxNum ? parseInt(result.maxNum.split("-").pop()!) : 0;
  return `${prefix}${String(lastNum + 1).padStart(4, "0")}`;
}

export async function GET(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");
    const customerId = searchParams.get("customerId");

    const conditions = [isNull(creditNotes.deletedAt)];
    if (invoiceId) conditions.push(eq(creditNotes.invoiceId, invoiceId));

    // Non-admin users can only see their own credit notes
    if (auth.user.role !== "admin") {
      if (auth.user.customerId) {
        conditions.push(eq(creditNotes.customerId, auth.user.customerId));
      } else {
        return NextResponse.json([]);
      }
    } else if (customerId) {
      conditions.push(eq(creditNotes.customerId, customerId));
    }

    const rows = await db
      .select()
      .from(creditNotes)
      .leftJoin(customers, eq(creditNotes.customerId, customers.id))
      .leftJoin(invoices, eq(creditNotes.invoiceId, invoices.id))
      .where(and(...conditions))
      .orderBy(desc(creditNotes.createdAt));

    const mapped = rows.map((r) => ({
      ...r.credit_notes,
      customerName: r.customers?.name ?? null,
      invoiceNumber: r.invoices?.invoiceNumber ?? null,
    }));

    return NextResponse.json(mapped);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/credit-notes");
  }
}

export async function POST(req: NextRequest) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const body = await req.json();
    const parsed = insertCreditNoteSchema.safeParse(body);
    if (!parsed.success) {
      return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
    }

    const { items, ...noteData } = parsed.data;

    const result = await db.transaction(async (tx) => {
      const creditNoteNumber = await generateCreditNoteNumber();
      const amount = items.reduce((sum, item) => sum + item.quantity * item.unitPrice, 0);

      const [note] = await tx
        .insert(creditNotes)
        .values({
          ...noteData,
          creditNoteNumber,
          amount,
          issuedAt: noteData.issuedAt ? new Date(noteData.issuedAt) : null,
        })
        .returning();

      if (items.length > 0) {
        await tx.insert(creditNoteItems).values(
          items.map((item) => ({
            creditNoteId: note.id,
            description: item.description,
            quantity: item.quantity,
            unitPrice: item.unitPrice,
            amount: item.quantity * item.unitPrice,
          }))
        );
      }

      return { ...note, items };
    });

    return NextResponse.json(result, { status: 201 });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/credit-notes");
  }
}
