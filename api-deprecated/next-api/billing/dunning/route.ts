import { db } from "@/lib/db";
import { invoices, dunningLog, subscriptions } from "@/lib/db/schema";
import { eq, and, lte, desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { verifyCronSecret } from "@/lib/billing/cron-auth";
import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

// GET: list dunning log entries
export async function GET(req: NextRequest) {
  try {
    // Cron auth: allow cron secret OR admin
    const cronCheck = verifyCronSecret(req);
    if (cronCheck) {
      const auth = await withAdmin();
      if (!auth.success) return cronCheck;
    }

    const { searchParams } = new URL(req.url);
    const invoiceId = searchParams.get("invoiceId");

    const query = invoiceId
      ? db.select().from(dunningLog).where(eq(dunningLog.invoiceId, invoiceId)).orderBy(desc(dunningLog.createdAt))
      : db.select().from(dunningLog).orderBy(desc(dunningLog.createdAt));

    const rows = await query;
    return NextResponse.json(rows);
  } catch (error) {
    return handleApiError(error, "GET /api/billing/dunning");
  }
}

// POST: run dunning process
// Configurable grace periods (days after due date):
//   reminder: 3, warning: 7, final_notice: 14, suspension: 30
export async function POST(req: NextRequest) {
  try {
    // Cron auth: allow cron secret OR admin
    const cronCheck = verifyCronSecret(req);
    if (cronCheck) {
      const auth = await withAdmin();
      if (!auth.success) return cronCheck;
    }

    const body = await req.json().catch(() => ({}));
    const gracePeriods = {
      reminder: body.reminderDays ?? 3,
      warning: body.warningDays ?? 7,
      final_notice: body.finalNoticeDays ?? 14,
      suspension: body.suspensionDays ?? 30,
    };

    const now = new Date();
    const results = { reminded: 0, warned: 0, finalNoticed: 0, suspended: 0, overdueMarked: 0 };

    // 1. Find issued invoices past due date -> mark as overdue
    const pastDueInvoices = await db
      .select()
      .from(invoices)
      .where(and(
        eq(invoices.status, "issued"),
        lte(invoices.dueAt, now),
      ));

    for (const inv of pastDueInvoices) {
      await db.transaction(async (tx) => {
        await tx.update(invoices).set({ status: "overdue", updatedAt: now }).where(eq(invoices.id, inv.id));
      });
      results.overdueMarked++;
    }

    // 2. Process overdue invoices through dunning steps
    const overdueInvoices = await db
      .select()
      .from(invoices)
      .where(eq(invoices.status, "overdue"));

    for (const inv of overdueInvoices) {
      if (!inv.dueAt) continue;
      const daysPastDue = Math.floor((now.getTime() - new Date(inv.dueAt).getTime()) / 86400000);

      await db.transaction(async (tx) => {
        // Get latest dunning step for this invoice
        const [latestStep] = await tx
          .select()
          .from(dunningLog)
          .where(eq(dunningLog.invoiceId, inv.id))
          .orderBy(desc(dunningLog.createdAt))
          .limit(1);

        const lastStep = latestStep?.step;

        // Determine which step to execute
        let nextStep: "reminder" | "warning" | "final_notice" | "suspension" | null = null;

        if (!lastStep && daysPastDue >= gracePeriods.reminder) {
          nextStep = "reminder";
        } else if (lastStep === "reminder" && daysPastDue >= gracePeriods.warning) {
          nextStep = "warning";
        } else if (lastStep === "warning" && daysPastDue >= gracePeriods.final_notice) {
          nextStep = "final_notice";
        } else if (lastStep === "final_notice" && daysPastDue >= gracePeriods.suspension) {
          nextStep = "suspension";
        }

        if (!nextStep) return;

        // Log the dunning step
        await tx.insert(dunningLog).values({
          invoiceId: inv.id,
          subscriptionId: inv.subscriptionId,
          step: nextStep,
          scheduledAt: now,
          executedAt: now,
          notes: `Auto-dunning: ${nextStep} at ${daysPastDue} days past due`,
        });

        // If suspension step, pause the linked subscription
        if (nextStep === "suspension" && inv.subscriptionId) {
          await tx
            .update(subscriptions)
            .set({ status: "paused", updatedAt: now })
            .where(eq(subscriptions.id, inv.subscriptionId));
        }

        results[nextStep === "reminder" ? "reminded" : nextStep === "warning" ? "warned" : nextStep === "final_notice" ? "finalNoticed" : "suspended"]++;
      });
    }

    return NextResponse.json(results);
  } catch (error) {
    return handleApiError(error, "POST /api/billing/dunning");
  }
}
