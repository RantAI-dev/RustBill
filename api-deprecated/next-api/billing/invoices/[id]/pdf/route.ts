import { db } from "@/lib/db";
import { invoices, invoiceItems, customers, payments } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";
import { PDFDocument, rgb, StandardFonts } from "pdf-lib";
import { withAuth } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";
import { appConfig } from "@/lib/app-config";

export async function GET(_req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  try {
    const auth = await withAuth();
    if (!auth.success) return auth.response;

    const { id } = await params;

    // Fetch invoice with customer and items
    const [invoice] = await db
      .select()
      .from(invoices)
      .leftJoin(customers, eq(invoices.customerId, customers.id))
      .where(eq(invoices.id, id));

    if (!invoice) return NextResponse.json({ error: "Not found" }, { status: 404 });

    // Check customer owns invoice (non-admin users)
    if (auth.user.role !== "admin" && invoice.invoices.customerId !== auth.user.customerId) {
      return NextResponse.json({ error: "Forbidden" }, { status: 403 });
    }

    const items = await db.select().from(invoiceItems).where(eq(invoiceItems.invoiceId, id));
    const paymentRows = await db.select().from(payments).where(eq(payments.invoiceId, id));

    const inv = invoice.invoices;
    const customer = invoice.customers;

    // Create PDF
    const pdfDoc = await PDFDocument.create();
    const page = pdfDoc.addPage([595.28, 841.89]); // A4
    const { height } = page.getSize();

    const helvetica = await pdfDoc.embedFont(StandardFonts.Helvetica);
    const helveticaBold = await pdfDoc.embedFont(StandardFonts.HelveticaBold);

    const darkColor = rgb(0.15, 0.15, 0.15);
    const mutedColor = rgb(0.45, 0.45, 0.45);
    const accentColor = rgb(0.2, 0.65, 0.45);

    let y = height - 50;
    const leftMargin = 50;
    const rightEdge = 545;

    // ---- Header ----
    page.drawText("INVOICE", {
      x: leftMargin, y,
      size: 28, font: helveticaBold, color: accentColor,
    });

    page.drawText(inv.invoiceNumber, {
      x: rightEdge - helveticaBold.widthOfTextAtSize(inv.invoiceNumber, 14),
      y, size: 14, font: helveticaBold, color: darkColor,
    });

    y -= 20;
    const statusText = inv.status.toUpperCase();
    page.drawText(statusText, {
      x: rightEdge - helvetica.widthOfTextAtSize(statusText, 10),
      y, size: 10, font: helvetica, color: mutedColor,
    });

    // ---- Company info ----
    y -= 35;
    page.drawText(appConfig.name, {
      x: leftMargin, y, size: 12, font: helveticaBold, color: darkColor,
    });
    y -= 16;
    page.drawText(appConfig.billingEmail, {
      x: leftMargin, y, size: 9, font: helvetica, color: mutedColor,
    });

    // ---- Bill To ----
    y -= 35;
    page.drawText("BILL TO", {
      x: leftMargin, y, size: 8, font: helveticaBold, color: mutedColor,
    });
    y -= 16;
    page.drawText(customer?.name ?? "Unknown", {
      x: leftMargin, y, size: 11, font: helveticaBold, color: darkColor,
    });
    y -= 14;
    if (customer?.billingEmail) {
      page.drawText(customer.billingEmail, {
        x: leftMargin, y, size: 9, font: helvetica, color: mutedColor,
      });
      y -= 12;
    }
    if (customer?.billingAddress) {
      const addressParts = [customer.billingAddress];
      if (customer.billingCity) addressParts.push(customer.billingCity);
      if (customer.billingState) addressParts.push(customer.billingState);
      if (customer.billingZip) addressParts.push(customer.billingZip);
      page.drawText(addressParts.join(", "), {
        x: leftMargin, y, size: 9, font: helvetica, color: mutedColor,
      });
      y -= 12;
    }
    if (customer?.taxId) {
      page.drawText(`Tax ID: ${customer.taxId}`, {
        x: leftMargin, y, size: 9, font: helvetica, color: mutedColor,
      });
      y -= 12;
    }

    // ---- Dates ----
    const dateY = height - 155;
    const dateX = 350;
    const drawDateRow = (label: string, value: string, yPos: number) => {
      page.drawText(label, { x: dateX, y: yPos, size: 9, font: helvetica, color: mutedColor });
      page.drawText(value, { x: dateX + 80, y: yPos, size: 9, font: helveticaBold, color: darkColor });
    };

    drawDateRow("Issued:", inv.issuedAt ? new Date(inv.issuedAt).toLocaleDateString() : "—", dateY);
    drawDateRow("Due:", inv.dueAt ? new Date(inv.dueAt).toLocaleDateString() : "—", dateY - 14);
    if (inv.paidAt) {
      drawDateRow("Paid:", new Date(inv.paidAt).toLocaleDateString(), dateY - 28);
    }

    // ---- Line Items Table ----
    y -= 25;

    // Table header
    const colX = { desc: leftMargin, qty: 330, price: 400, amount: 480 };

    page.drawRectangle({
      x: leftMargin - 5, y: y - 4, width: rightEdge - leftMargin + 10, height: 20,
      color: rgb(0.95, 0.95, 0.95),
    });

    page.drawText("Description", { x: colX.desc, y, size: 8, font: helveticaBold, color: mutedColor });
    page.drawText("Qty", { x: colX.qty, y, size: 8, font: helveticaBold, color: mutedColor });
    page.drawText("Unit Price", { x: colX.price, y, size: 8, font: helveticaBold, color: mutedColor });
    page.drawText("Amount", { x: colX.amount, y, size: 8, font: helveticaBold, color: mutedColor });

    y -= 22;

    // Table rows
    for (const item of items) {
      const desc = item.description.length > 45 ? item.description.slice(0, 42) + "..." : item.description;
      page.drawText(desc, { x: colX.desc, y, size: 9, font: helvetica, color: darkColor });
      page.drawText(formatQty(Number(item.quantity)), { x: colX.qty, y, size: 9, font: helvetica, color: mutedColor });
      page.drawText(`$${Number(item.unitPrice).toLocaleString()}`, { x: colX.price, y, size: 9, font: helvetica, color: mutedColor });
      page.drawText(`$${Number(item.amount).toLocaleString()}`, { x: colX.amount, y, size: 9, font: helveticaBold, color: darkColor });
      y -= 18;
    }

    // Divider
    y -= 5;
    page.drawLine({
      start: { x: 350, y }, end: { x: rightEdge, y },
      thickness: 0.5, color: rgb(0.85, 0.85, 0.85),
    });
    y -= 15;

    // Totals
    const drawTotal = (label: string, value: string, bold = false) => {
      const font = bold ? helveticaBold : helvetica;
      const size = bold ? 12 : 9;
      page.drawText(label, { x: 370, y, size, font, color: mutedColor });
      page.drawText(value, { x: colX.amount, y, size, font: bold ? helveticaBold : helvetica, color: darkColor });
      y -= bold ? 22 : 16;
    };

    drawTotal("Subtotal", `$${Number(inv.subtotal).toLocaleString()}`);
    drawTotal("Tax", `$${Number(inv.tax).toLocaleString()}`);
    drawTotal("Total", `$${Number(inv.total).toLocaleString()}`, true);

    // ---- Payments section ----
    if (paymentRows.length > 0) {
      y -= 10;
      page.drawText("PAYMENTS", { x: leftMargin, y, size: 8, font: helveticaBold, color: mutedColor });
      y -= 16;
      for (const p of paymentRows) {
        const line = `$${Number(p.amount).toLocaleString()} via ${p.method.replace("_", " ")} on ${new Date(p.paidAt).toLocaleDateString()}`;
        page.drawText(line, { x: leftMargin, y, size: 9, font: helvetica, color: darkColor });
        if (p.reference) {
          page.drawText(`Ref: ${p.reference}`, { x: leftMargin + 300, y, size: 8, font: helvetica, color: mutedColor });
        }
        y -= 14;
      }
    }

    // ---- Notes ----
    if (inv.notes) {
      y -= 15;
      page.drawText("Notes", { x: leftMargin, y, size: 8, font: helveticaBold, color: mutedColor });
      y -= 14;
      page.drawText(inv.notes, { x: leftMargin, y, size: 9, font: helvetica, color: mutedColor });
    }

    // ---- Footer ----
    page.drawText(`Currency: ${inv.currency}`, {
      x: leftMargin, y: 40, size: 8, font: helvetica, color: mutedColor,
    });
    const footerText = `Generated by ${appConfig.name}`;
    page.drawText(footerText, {
      x: rightEdge - helvetica.widthOfTextAtSize(footerText, 8),
      y: 40, size: 8, font: helvetica, color: mutedColor,
    });

    // Serialize
    const pdfBytes = await pdfDoc.save();

    return new NextResponse(Buffer.from(pdfBytes), {
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `inline; filename="${inv.invoiceNumber}.pdf"`,
      },
    });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/invoices/[id]/pdf");
  }
}

function formatQty(qty: number): string {
  if (qty >= 1000) return qty.toLocaleString();
  if (Number.isInteger(qty)) return String(qty);
  return qty.toFixed(2);
}
