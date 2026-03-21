use super::schema::{
    CreateSubscriptionDraft, ListSubscriptionsFilter, SubscriptionView, UpdateSubscriptionRequest,
};
use crate::analytics::sales_ledger::{emit_sales_event, NewSalesEvent, SalesClassification};
use crate::db::models::{PricingPlan, Subscription};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn list_subscriptions(
        &self,
        filter: &ListSubscriptionsFilter,
    ) -> Result<Vec<SubscriptionView>>;
    async fn get_subscription(&self, id: &str) -> Result<Option<Subscription>>;
    async fn find_plan(&self, id: &str) -> Result<Option<PricingPlan>>;
    async fn create_subscription(&self, draft: &CreateSubscriptionDraft) -> Result<Subscription>;
    async fn update_subscription(
        &self,
        id: &str,
        req: &UpdateSubscriptionRequest,
    ) -> Result<Option<Subscription>>;
    async fn delete_subscription(&self, id: &str) -> Result<u64>;
    async fn run_lifecycle(&self) -> Result<u64>;
}

#[derive(Clone)]
pub struct PgSubscriptionRepository {
    pool: PgPool,
}

impl PgSubscriptionRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl SubscriptionRepository for PgSubscriptionRepository {
    async fn list_subscriptions(
        &self,
        filter: &ListSubscriptionsFilter,
    ) -> Result<Vec<SubscriptionView>> {
        let rows = sqlx::query_as::<_, SubscriptionView>(
            r#"
            SELECT
                s.*,
                c.name AS customer_name,
                pp.name AS plan_name
            FROM subscriptions s
            LEFT JOIN customers c ON c.id = s.customer_id
            LEFT JOIN pricing_plans pp ON pp.id = s.plan_id
            WHERE s.deleted_at IS NULL
              AND ($1::subscription_status IS NULL OR s.status = $1)
              AND ($2::text IS NULL OR s.customer_id = $2)
              AND ($3::text IS NULL OR s.customer_id = $3)
            ORDER BY s.created_at DESC
            "#,
        )
        .bind(&filter.status)
        .bind(&filter.customer_id)
        .bind(&filter.role_customer_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_subscription(&self, id: &str) -> Result<Option<Subscription>> {
        let row = sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_plan(&self, id: &str) -> Result<Option<PricingPlan>> {
        let plan = sqlx::query_as::<_, PricingPlan>("SELECT * FROM pricing_plans WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(plan)
    }

    async fn create_subscription(&self, draft: &CreateSubscriptionDraft) -> Result<Subscription> {
        let mut tx = self.pool.begin().await?;

        let row = sqlx::query_as::<_, Subscription>(
            r#"
            INSERT INTO subscriptions
                (id, customer_id, plan_id, status, current_period_start, current_period_end,
                 trial_end, quantity, metadata, stripe_subscription_id, version,
                 cancel_at_period_end)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, 1, false)
            RETURNING *
            "#,
        )
        .bind(&draft.customer_id)
        .bind(&draft.plan_id)
        .bind(&draft.status)
        .bind(draft.current_period_start)
        .bind(draft.current_period_end)
        .bind(draft.trial_end)
        .bind(draft.quantity)
        .bind(&draft.metadata)
        .bind(&draft.stripe_subscription_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Err(err) = emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "subscription.created",
                classification: SalesClassification::Recurring,
                amount_subtotal: rust_decimal::Decimal::ZERO,
                amount_tax: rust_decimal::Decimal::ZERO,
                amount_total: rust_decimal::Decimal::ZERO,
                currency: "USD",
                customer_id: Some(&row.customer_id),
                subscription_id: Some(&row.id),
                product_id: None,
                invoice_id: None,
                payment_id: None,
                source_table: "subscriptions",
                source_id: &row.id,
                metadata: Some(serde_json::json!({
                    "status": row.status,
                    "plan_id": row.plan_id,
                    "quantity": row.quantity,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, subscription_id = %row.id, "failed to emit sales event subscription.created");
        }

        Ok(row)
    }

    async fn update_subscription(
        &self,
        id: &str,
        req: &UpdateSubscriptionRequest,
    ) -> Result<Option<Subscription>> {
        let row = sqlx::query_as::<_, Subscription>(
            r#"
            UPDATE subscriptions SET
                status               = COALESCE($2, status),
                quantity             = COALESCE($3, quantity),
                cancel_at_period_end = COALESCE($4, cancel_at_period_end),
                canceled_at          = COALESCE($5, canceled_at),
                metadata             = COALESCE($6, metadata),
                stripe_subscription_id = COALESCE($7, stripe_subscription_id),
                version              = version + 1,
                updated_at           = NOW()
            WHERE id = $1
              AND deleted_at IS NULL
              AND version = $8
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&req.status)
        .bind(req.quantity)
        .bind(req.cancel_at_period_end)
        .bind(req.canceled_at)
        .bind(&req.metadata)
        .bind(&req.stripe_subscription_id)
        .bind(req.version)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_subscription(&self, id: &str) -> Result<u64> {
        let result = sqlx::query(
            "UPDATE subscriptions SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    async fn run_lifecycle(&self) -> Result<u64> {
        let now = chrono::Utc::now().naive_utc();
        let mut processed: u64 = 0;

        let r = sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = 'active',
                current_period_end = current_period_end + (
                    SELECT CASE pp.billing_cycle
                        WHEN 'monthly'   THEN INTERVAL '1 month'
                        WHEN 'quarterly' THEN INTERVAL '3 months'
                        WHEN 'yearly'    THEN INTERVAL '1 year'
                    END
                    FROM pricing_plans pp WHERE pp.id = subscriptions.plan_id
                ),
                version = version + 1,
                updated_at = NOW()
            WHERE status = 'trialing'
              AND deleted_at IS NULL
              AND trial_end IS NOT NULL
              AND trial_end <= $1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        processed += r.rows_affected();

        let r = sqlx::query(
            r#"
            UPDATE subscriptions
            SET status = 'canceled',
                canceled_at = NOW(),
                version = version + 1,
                updated_at = NOW()
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = true
              AND current_period_end <= $1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        processed += r.rows_affected();

        let r = sqlx::query(
            r#"
            UPDATE subscriptions
            SET current_period_start = current_period_end,
                current_period_end = current_period_end + (
                    SELECT CASE pp.billing_cycle
                        WHEN 'monthly'   THEN INTERVAL '1 month'
                        WHEN 'quarterly' THEN INTERVAL '3 months'
                        WHEN 'yearly'    THEN INTERVAL '1 year'
                    END
                    FROM pricing_plans pp WHERE pp.id = subscriptions.plan_id
                ),
                version = version + 1,
                updated_at = NOW()
            WHERE status = 'active'
              AND deleted_at IS NULL
              AND cancel_at_period_end = false
              AND current_period_end <= $1
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await?;
        processed += r.rows_affected();

        tracing::info!(processed, "subscription lifecycle completed");
        Ok(processed)
    }
}
