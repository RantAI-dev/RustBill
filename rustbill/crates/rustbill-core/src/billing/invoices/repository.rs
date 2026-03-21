use super::schema::{
    AddInvoiceItemRequest, CreateInvoiceDraft, InvoiceItemDraft, InvoiceView, ListInvoicesFilter,
    UpdateInvoiceDraft,
};
use crate::db::models::{Coupon, Invoice, InvoiceItem, PricingPlan, Subscription};
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait InvoiceRepository: Send + Sync {
    async fn list_invoices(&self, filter: &ListInvoicesFilter) -> Result<Vec<InvoiceView>>;
    async fn get_invoice(&self, id: &str) -> Result<Option<Invoice>>;
    async fn next_invoice_number(&self) -> Result<String>;
    async fn find_subscription(&self, id: &str) -> Result<Option<Subscription>>;
    async fn find_plan(&self, id: &str) -> Result<Option<PricingPlan>>;
    async fn find_coupon(&self, code: &str) -> Result<Option<Coupon>>;
    async fn create_invoice(&self, draft: &CreateInvoiceDraft) -> Result<Invoice>;
    async fn update_invoice(&self, id: &str, draft: &UpdateInvoiceDraft)
        -> Result<Option<Invoice>>;
    async fn delete_invoice(&self, id: &str) -> Result<u64>;
    async fn add_invoice_item(
        &self,
        invoice_id: &str,
        req: &AddInvoiceItemRequest,
    ) -> Result<InvoiceItem>;
    async fn list_invoice_items(&self, invoice_id: &str) -> Result<Vec<InvoiceItem>>;
}

#[derive(Clone)]
pub struct PgInvoiceRepository {
    pool: PgPool,
}

impl PgInvoiceRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl InvoiceRepository for PgInvoiceRepository {
    async fn list_invoices(&self, filter: &ListInvoicesFilter) -> Result<Vec<InvoiceView>> {
        let rows = sqlx::query_as::<_, InvoiceView>(
            r#"
            SELECT
                i.*,
                c.name AS customer_name
            FROM invoices i
            LEFT JOIN customers c ON c.id = i.customer_id
            WHERE i.deleted_at IS NULL
              AND ($1::invoice_status IS NULL OR i.status = $1)
              AND ($2::text IS NULL OR i.customer_id = $2)
              AND ($3::text IS NULL OR i.customer_id = $3)
            ORDER BY i.created_at DESC
            "#,
        )
        .bind(&filter.status)
        .bind(&filter.customer_id)
        .bind(&filter.role_customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_invoice(&self, id: &str) -> Result<Option<Invoice>> {
        let invoice = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(invoice)
    }

    async fn next_invoice_number(&self) -> Result<String> {
        let invoice_number: String = sqlx::query_scalar(
            "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(invoice_number)
    }

    async fn find_subscription(&self, id: &str) -> Result<Option<Subscription>> {
        let sub = sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(sub)
    }

    async fn find_plan(&self, id: &str) -> Result<Option<PricingPlan>> {
        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(plan)
    }

    async fn find_coupon(&self, code: &str) -> Result<Option<Coupon>> {
        let coupon = sqlx::query_as::<_, Coupon>(
            "SELECT * FROM coupons WHERE code = $1 AND active = true AND deleted_at IS NULL",
        )
        .bind(code)
        .fetch_optional(&self.pool)
        .await?;
        Ok(coupon)
    }

    async fn create_invoice(&self, draft: &CreateInvoiceDraft) -> Result<Invoice> {
        let mut tx = self.pool.begin().await?;

        let invoice = sqlx::query_as::<_, Invoice>(
            r#"
            INSERT INTO invoices
                (id, invoice_number, customer_id, subscription_id, status,
                 due_at, subtotal, tax, total, currency, notes)
            VALUES (gen_random_uuid()::text, $1, $2, $3, 'draft', $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(&draft.invoice_number)
        .bind(&draft.customer_id)
        .bind(&draft.subscription_id)
        .bind(draft.due_at)
        .bind(draft.subtotal)
        .bind(draft.tax)
        .bind(draft.total)
        .bind(&draft.currency)
        .bind(&draft.notes)
        .fetch_one(&mut *tx)
        .await?;

        for item in &draft.line_items {
            insert_invoice_item_row(&mut tx, &invoice.id, item).await?;
        }

        if let Some(coupon_id) = &draft.coupon_id_to_increment {
            sqlx::query("UPDATE coupons SET times_redeemed = times_redeemed + 1 WHERE id = $1")
                .bind(coupon_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(invoice)
    }

    async fn update_invoice(
        &self,
        id: &str,
        draft: &UpdateInvoiceDraft,
    ) -> Result<Option<Invoice>> {
        let row = sqlx::query_as::<_, Invoice>(
            r#"
            UPDATE invoices SET
                status                = COALESCE($2, status),
                due_at                = COALESCE($3, due_at),
                notes                 = COALESCE($4, notes),
                stripe_invoice_id     = COALESCE($5, stripe_invoice_id),
                xendit_invoice_id     = COALESCE($6, xendit_invoice_id),
                lemonsqueezy_order_id = COALESCE($7, lemonsqueezy_order_id),
                issued_at             = COALESCE($8, issued_at),
                version               = version + 1,
                updated_at            = NOW()
            WHERE id = $1
              AND deleted_at IS NULL
              AND version = $9
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&draft.status)
        .bind(draft.due_at)
        .bind(&draft.notes)
        .bind(&draft.stripe_invoice_id)
        .bind(&draft.xendit_invoice_id)
        .bind(&draft.lemonsqueezy_order_id)
        .bind(draft.issued_at)
        .bind(draft.version)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn delete_invoice(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE invoices SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    async fn add_invoice_item(
        &self,
        invoice_id: &str,
        req: &AddInvoiceItemRequest,
    ) -> Result<InvoiceItem> {
        let amount = (req.quantity * req.unit_price).round_dp(2);
        let mut tx = self.pool.begin().await?;

        let _inv = sqlx::query_as::<_, Invoice>(
            "SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(invoice_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| BillingError::not_found("invoice", invoice_id))?;

        let item = insert_invoice_item_row(
            &mut tx,
            invoice_id,
            &InvoiceItemDraft {
                description: req.description.clone(),
                quantity: req.quantity,
                unit_price: req.unit_price,
                amount,
                period_start: req.period_start,
                period_end: req.period_end,
            },
        )
        .await?;

        recompute_invoice_totals(&mut tx, invoice_id).await?;

        tx.commit().await?;
        Ok(item)
    }

    async fn list_invoice_items(&self, invoice_id: &str) -> Result<Vec<InvoiceItem>> {
        let rows = sqlx::query_as::<_, InvoiceItem>(
            "SELECT * FROM invoice_items WHERE invoice_id = $1 ORDER BY id",
        )
        .bind(invoice_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

async fn insert_invoice_item_row(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice_id: &str,
    item: &InvoiceItemDraft,
) -> Result<InvoiceItem> {
    let row = sqlx::query_as::<_, InvoiceItem>(
        r#"
        INSERT INTO invoice_items
            (id, invoice_id, description, quantity, unit_price, amount,
             period_start, period_end)
        VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(invoice_id)
    .bind(&item.description)
    .bind(item.quantity)
    .bind(item.unit_price)
    .bind(item.amount)
    .bind(item.period_start)
    .bind(item.period_end)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row)
}

async fn recompute_invoice_totals(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    invoice_id: &str,
) -> Result<()> {
    let subtotal: Option<rust_decimal::Decimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0) FROM invoice_items WHERE invoice_id = $1",
    )
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await?;

    let subtotal = subtotal.unwrap_or_default();

    let (old_subtotal, old_tax): (rust_decimal::Decimal, rust_decimal::Decimal) =
        sqlx::query_as("SELECT subtotal, tax FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_one(&mut **tx)
            .await?;

    let tax = if old_subtotal > rust_decimal::Decimal::ZERO {
        (subtotal * old_tax / old_subtotal).round_dp(2)
    } else {
        rust_decimal::Decimal::ZERO
    };
    let total = subtotal + tax;

    sqlx::query(
        "UPDATE invoices SET subtotal = $2, tax = $3, total = $4, updated_at = NOW() WHERE id = $1",
    )
    .bind(invoice_id)
    .bind(subtotal)
    .bind(tax)
    .bind(total)
    .execute(&mut **tx)
    .await?;

    Ok(())
}
