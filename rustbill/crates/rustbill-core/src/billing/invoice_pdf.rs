use crate::db::models::*;
use crate::error::{BillingError, Result};
use printpdf::*;
use rust_decimal::Decimal;
use sqlx::PgPool;

/// Generate a PDF for the given invoice, returning raw PDF bytes.
pub async fn generate_invoice_pdf(pool: &PgPool, invoice_id: &str) -> Result<Vec<u8>> {
    // Fetch invoice
    let invoice = crate::billing::invoices::get_invoice(pool, invoice_id).await?;

    // Fetch customer
    let customer = sqlx::query_as::<_, Customer>(
        "SELECT * FROM customers WHERE id = $1",
    )
    .bind(&invoice.customer_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| BillingError::not_found("customer", &invoice.customer_id))?;

    // Fetch line items
    let items = crate::billing::invoices::list_invoice_items(pool, invoice_id).await?;

    // Build PDF
    let (doc, page1, layer1) = PdfDocument::new(
        &format!("Invoice {}", invoice.invoice_number),
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );

    let font = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold).unwrap();
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let mut y = 270.0;

    // ---- Header ----
    current_layer.use_text("INVOICE", 20.0, Mm(20.0), Mm(y), &font_bold);
    y -= 10.0;
    current_layer.use_text(
        &invoice.invoice_number,
        12.0,
        Mm(20.0),
        Mm(y),
        &font,
    );

    // Status on the right
    current_layer.use_text(
        &format!("Status: {:?}", invoice.status),
        10.0,
        Mm(140.0),
        Mm(y),
        &font,
    );

    y -= 15.0;

    // ---- Bill To ----
    current_layer.use_text("Bill To:", 10.0, Mm(20.0), Mm(y), &font_bold);
    y -= 6.0;
    current_layer.use_text(&customer.name, 10.0, Mm(20.0), Mm(y), &font);
    y -= 5.0;
    current_layer.use_text(&customer.email, 9.0, Mm(20.0), Mm(y), &font);
    y -= 5.0;

    if let Some(ref addr) = customer.billing_address {
        current_layer.use_text(addr, 9.0, Mm(20.0), Mm(y), &font);
        y -= 5.0;
    }

    let city_state = [
        customer.billing_city.as_deref().unwrap_or(""),
        customer.billing_state.as_deref().unwrap_or(""),
        customer.billing_zip.as_deref().unwrap_or(""),
    ]
    .iter()
    .filter(|s| !s.is_empty())
    .copied()
    .collect::<Vec<_>>()
    .join(", ");

    if !city_state.is_empty() {
        current_layer.use_text(&city_state, 9.0, Mm(20.0), Mm(y), &font);
        y -= 5.0;
    }

    y -= 5.0;

    // ---- Dates ----
    current_layer.use_text("Dates:", 10.0, Mm(120.0), Mm(y + 20.0), &font_bold);
    if let Some(issued) = invoice.issued_at {
        current_layer.use_text(
            &format!("Issued: {}", issued.format("%Y-%m-%d")),
            9.0,
            Mm(120.0),
            Mm(y + 14.0),
            &font,
        );
    }
    if let Some(due) = invoice.due_at {
        current_layer.use_text(
            &format!("Due: {}", due.format("%Y-%m-%d")),
            9.0,
            Mm(120.0),
            Mm(y + 8.0),
            &font,
        );
    }

    y -= 10.0;

    // ---- Line Items Table Header ----
    current_layer.use_text("Description", 9.0, Mm(20.0), Mm(y), &font_bold);
    current_layer.use_text("Qty", 9.0, Mm(110.0), Mm(y), &font_bold);
    current_layer.use_text("Unit Price", 9.0, Mm(130.0), Mm(y), &font_bold);
    current_layer.use_text("Amount", 9.0, Mm(165.0), Mm(y), &font_bold);

    y -= 3.0;

    // Separator line
    let line = Line {
        points: vec![
            (Point::new(Mm(20.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line);

    y -= 6.0;

    // ---- Line Items ----
    for item in &items {
        // Truncate description if too long
        let desc = if item.description.len() > 50 {
            format!("{}...", &item.description[..47])
        } else {
            item.description.clone()
        };

        current_layer.use_text(&desc, 8.0, Mm(20.0), Mm(y), &font);
        current_layer.use_text(&item.quantity.to_string(), 8.0, Mm(110.0), Mm(y), &font);
        current_layer.use_text(
            &format_currency(item.unit_price, &invoice.currency),
            8.0,
            Mm(130.0),
            Mm(y),
            &font,
        );
        current_layer.use_text(
            &format_currency(item.amount, &invoice.currency),
            8.0,
            Mm(165.0),
            Mm(y),
            &font,
        );

        y -= 5.0;

        // Handle page overflow (unlikely for most invoices but safe)
        if y < 40.0 {
            break;
        }
    }

    y -= 5.0;

    // Separator line
    let line = Line {
        points: vec![
            (Point::new(Mm(110.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    };
    current_layer.add_line(line);

    y -= 6.0;

    // ---- Totals ----
    current_layer.use_text("Subtotal:", 9.0, Mm(130.0), Mm(y), &font);
    current_layer.use_text(
        &format_currency(invoice.subtotal, &invoice.currency),
        9.0,
        Mm(165.0),
        Mm(y),
        &font,
    );
    y -= 5.0;

    current_layer.use_text("Tax:", 9.0, Mm(130.0), Mm(y), &font);
    current_layer.use_text(
        &format_currency(invoice.tax, &invoice.currency),
        9.0,
        Mm(165.0),
        Mm(y),
        &font,
    );
    y -= 6.0;

    current_layer.use_text("Total:", 10.0, Mm(130.0), Mm(y), &font_bold);
    current_layer.use_text(
        &format_currency(invoice.total, &invoice.currency),
        10.0,
        Mm(165.0),
        Mm(y),
        &font_bold,
    );

    y -= 15.0;

    // ---- Footer ----
    if let Some(ref notes) = invoice.notes {
        current_layer.use_text("Notes:", 9.0, Mm(20.0), Mm(y), &font_bold);
        y -= 5.0;
        current_layer.use_text(notes, 8.0, Mm(20.0), Mm(y), &font);
        let _ = y;
    }

    // Footer line
    current_layer.use_text(
        "Thank you for your business.",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &font,
    );

    // Save to bytes
    let bytes = doc.save_to_bytes().map_err(|e| {
        BillingError::Internal(anyhow::anyhow!("PDF generation failed: {e}"))
    })?;

    Ok(bytes)
}

fn format_currency(amount: Decimal, currency: &str) -> String {
    format!("{} {}", currency, amount)
}
