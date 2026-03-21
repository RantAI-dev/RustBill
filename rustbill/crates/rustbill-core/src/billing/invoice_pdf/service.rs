use super::repository::InvoicePdfRepository;
use super::schema::GenerateInvoicePdfRequest;
use crate::db::models::{Customer, Invoice, InvoiceItem};
use crate::error::{BillingError, Result};
use printpdf::*;
use validator::Validate;

pub async fn generate_invoice_pdf<R: InvoicePdfRepository + ?Sized>(
    repo: &R,
    req: GenerateInvoicePdfRequest,
) -> Result<Vec<u8>> {
    req.validate().map_err(BillingError::from_validation)?;

    let invoice = repo.get_invoice(&req.invoice_id).await?;
    let customer = repo
        .get_customer(&invoice.customer_id)
        .await?
        .ok_or_else(|| BillingError::not_found("customer", &invoice.customer_id))?;
    let items = repo.list_invoice_items(&req.invoice_id).await?;

    render_invoice_pdf(&invoice, &customer, &items)
}

fn render_invoice_pdf(
    invoice: &Invoice,
    customer: &Customer,
    items: &[InvoiceItem],
) -> Result<Vec<u8>> {
    let (doc, page1, layer1) = PdfDocument::new(
        format!("Invoice {}", invoice.invoice_number),
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );

    let font = doc
        .add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| anyhow::anyhow!("PDF font setup failed: {e}"))?;
    let font_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| anyhow::anyhow!("PDF font setup failed: {e}"))?;
    let current_layer = doc.get_page(page1).get_layer(layer1);

    let mut y = 270.0;

    current_layer.use_text("INVOICE", 20.0, Mm(20.0), Mm(y), &font_bold);
    y -= 10.0;
    current_layer.use_text(&invoice.invoice_number, 12.0, Mm(20.0), Mm(y), &font);
    current_layer.use_text(
        format!("Status: {:?}", invoice.status),
        10.0,
        Mm(140.0),
        Mm(y),
        &font,
    );

    y -= 15.0;

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

    current_layer.use_text("Dates:", 10.0, Mm(120.0), Mm(y + 20.0), &font_bold);
    if let Some(issued) = invoice.issued_at {
        current_layer.use_text(
            format!("Issued: {}", issued.format("%Y-%m-%d")),
            9.0,
            Mm(120.0),
            Mm(y + 14.0),
            &font,
        );
    }
    if let Some(due) = invoice.due_at {
        current_layer.use_text(
            format!("Due: {}", due.format("%Y-%m-%d")),
            9.0,
            Mm(120.0),
            Mm(y + 8.0),
            &font,
        );
    }

    y -= 10.0;

    current_layer.use_text("Description", 9.0, Mm(20.0), Mm(y), &font_bold);
    current_layer.use_text("Qty", 9.0, Mm(110.0), Mm(y), &font_bold);
    current_layer.use_text("Unit Price", 9.0, Mm(130.0), Mm(y), &font_bold);
    current_layer.use_text("Amount", 9.0, Mm(165.0), Mm(y), &font_bold);

    y -= 3.0;
    current_layer.add_line(Line {
        points: vec![
            (Point::new(Mm(20.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    });

    y -= 6.0;

    for item in items {
        let desc = truncate_description(&item.description, 50);

        current_layer.use_text(&desc, 8.0, Mm(20.0), Mm(y), &font);
        current_layer.use_text(item.quantity.to_string(), 8.0, Mm(110.0), Mm(y), &font);
        current_layer.use_text(
            format_currency(item.unit_price, &invoice.currency),
            8.0,
            Mm(130.0),
            Mm(y),
            &font,
        );
        current_layer.use_text(
            format_currency(item.amount, &invoice.currency),
            8.0,
            Mm(165.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        if y < 40.0 {
            break;
        }
    }

    y -= 5.0;
    current_layer.add_line(Line {
        points: vec![
            (Point::new(Mm(110.0), Mm(y)), false),
            (Point::new(Mm(190.0), Mm(y)), false),
        ],
        is_closed: false,
    });

    y -= 6.0;

    current_layer.use_text("Subtotal:", 9.0, Mm(130.0), Mm(y), &font);
    current_layer.use_text(
        format_currency(invoice.subtotal, &invoice.currency),
        9.0,
        Mm(165.0),
        Mm(y),
        &font,
    );
    y -= 5.0;

    current_layer.use_text("Tax:", 9.0, Mm(130.0), Mm(y), &font);
    current_layer.use_text(
        format_currency(invoice.tax, &invoice.currency),
        9.0,
        Mm(165.0),
        Mm(y),
        &font,
    );
    y -= 6.0;

    current_layer.use_text("Total:", 10.0, Mm(130.0), Mm(y), &font_bold);
    current_layer.use_text(
        format_currency(invoice.total, &invoice.currency),
        10.0,
        Mm(165.0),
        Mm(y),
        &font_bold,
    );

    y -= 15.0;

    if let Some(ref notes) = invoice.notes {
        current_layer.use_text("Notes:", 9.0, Mm(20.0), Mm(y), &font_bold);
        y -= 5.0;
        current_layer.use_text(notes, 8.0, Mm(20.0), Mm(y), &font);
    }

    current_layer.use_text(
        "Thank you for your business.",
        8.0,
        Mm(20.0),
        Mm(20.0),
        &font,
    );

    let bytes = doc
        .save_to_bytes()
        .map_err(|e| anyhow::anyhow!("PDF generation failed: {e}"))?;

    Ok(bytes)
}

fn truncate_description(description: &str, max_chars: usize) -> String {
    let total_chars = description.chars().count();
    if total_chars <= max_chars {
        return description.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    let mut out: String = description.chars().take(keep).collect();
    out.push_str("...");
    out
}

fn format_currency(amount: rust_decimal::Decimal, currency: &str) -> String {
    format!("{} {}", currency, amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Customer, Invoice, InvoiceItem, InvoiceStatus, Trend};
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;

    #[derive(Clone)]
    struct StubRepo {
        invoice: Option<Invoice>,
        customer: Option<Customer>,
        items: Vec<InvoiceItem>,
    }

    impl StubRepo {
        fn new(
            invoice: Option<Invoice>,
            customer: Option<Customer>,
            items: Vec<InvoiceItem>,
        ) -> Self {
            Self {
                invoice,
                customer,
                items,
            }
        }
    }

    #[async_trait]
    impl InvoicePdfRepository for StubRepo {
        async fn get_invoice(&self, _invoice_id: &str) -> Result<Invoice> {
            self.invoice
                .clone()
                .ok_or_else(|| BillingError::not_found("invoice", "inv_1"))
        }

        async fn get_customer(&self, _customer_id: &str) -> Result<Option<Customer>> {
            Ok(self.customer.clone())
        }

        async fn list_invoice_items(&self, _invoice_id: &str) -> Result<Vec<InvoiceItem>> {
            Ok(self.items.clone())
        }
    }

    fn sample_invoice() -> Invoice {
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cus_1".to_string(),
            subscription_id: None,
            status: InvoiceStatus::Issued,
            issued_at: Some(Utc::now().naive_utc()),
            due_at: Some(Utc::now().naive_utc()),
            paid_at: None,
            subtotal: Decimal::from(100),
            tax: Decimal::from(10),
            total: Decimal::from(110),
            currency: "USD".to_string(),
            notes: Some("Thanks".to_string()),
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::from(110),
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    fn sample_customer() -> Customer {
        Customer {
            id: "cus_1".to_string(),
            name: "Acme".to_string(),
            industry: "Software".to_string(),
            tier: "Pro".to_string(),
            location: "Remote".to_string(),
            contact: "Jane".to_string(),
            email: "jane@acme.test".to_string(),
            phone: "123456".to_string(),
            total_revenue: Decimal::ZERO,
            health_score: 50,
            trend: Trend::Stable,
            last_contact: "".to_string(),
            billing_email: Some("billing@acme.test".to_string()),
            billing_address: Some("123 Road".to_string()),
            billing_city: Some("Austin".to_string()),
            billing_state: Some("TX".to_string()),
            billing_zip: Some("78701".to_string()),
            billing_country: Some("US".to_string()),
            tax_id: None,
            default_payment_method: None,
            stripe_customer_id: None,
            xendit_customer_id: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_item(description: &str) -> InvoiceItem {
        InvoiceItem {
            id: "item_1".to_string(),
            invoice_id: "inv_1".to_string(),
            description: description.to_string(),
            quantity: Decimal::from(1),
            unit_price: Decimal::from(100),
            amount: Decimal::from(100),
            period_start: None,
            period_end: None,
        }
    }

    #[tokio::test]
    async fn generate_invoice_pdf_returns_pdf_bytes() {
        let repo = StubRepo::new(
            Some(sample_invoice()),
            Some(sample_customer()),
            vec![sample_item("Design work")],
        );

        let bytes = generate_invoice_pdf(
            &repo,
            GenerateInvoicePdfRequest {
                invoice_id: "inv_1".to_string(),
            },
        )
        .await
        .expect("generate_invoice_pdf");

        assert!(bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn generate_invoice_pdf_handles_non_ascii_long_descriptions() {
        let repo = StubRepo::new(
            Some(sample_invoice()),
            Some(sample_customer()),
            vec![sample_item(
                "日本語の説明がとても長いので安全に切り詰める必要があります",
            )],
        );

        let bytes = generate_invoice_pdf(
            &repo,
            GenerateInvoicePdfRequest {
                invoice_id: "inv_1".to_string(),
            },
        )
        .await
        .expect("generate_invoice_pdf");

        assert!(bytes.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn generate_invoice_pdf_returns_not_found_when_customer_missing() {
        let repo = StubRepo::new(Some(sample_invoice()), None, vec![]);

        let err = generate_invoice_pdf(
            &repo,
            GenerateInvoicePdfRequest {
                invoice_id: "inv_1".to_string(),
            },
        )
        .await
        .expect_err("should fail");

        assert!(err.to_string().contains("not found"));
    }
}
