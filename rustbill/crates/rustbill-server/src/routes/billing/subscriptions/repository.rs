use super::schema::{
    ChangePlanRequest, CreateSubscriptionRequest, CreateSubscriptionV1Request,
    UpdateSubscriptionRequest, UpdateSubscriptionV1Request,
};
use async_trait::async_trait;
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use rustbill_core::db::models::{BillingEventType, PricingTier, Subscription};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait SubscriptionsRepository: Send + Sync {
    async fn list_admin(
        &self,
        role_customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn get_by_id(&self, id: &str) -> Result<serde_json::Value, BillingError>;
    async fn create_admin(
        &self,
        req: &CreateSubscriptionRequest,
        metadata: serde_json::Value,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update_admin(
        &self,
        id: &str,
        req: &UpdateSubscriptionRequest,
        metadata: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BillingError>;

    async fn find_active_subscription(&self, id: &str) -> Result<Subscription, BillingError>;
    async fn cancel_subscription(&self, id: &str) -> Result<u64, BillingError>;
    async fn lifecycle_update_status(
        &self,
        subscription_id: &str,
        new_status: &str,
    ) -> Result<serde_json::Value, BillingError>;

    async fn list_v1(
        &self,
        status: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
    async fn create_v1(
        &self,
        req: &CreateSubscriptionV1Request,
    ) -> Result<serde_json::Value, BillingError>;
    async fn update_v1(
        &self,
        id: &str,
        req: &UpdateSubscriptionV1Request,
    ) -> Result<serde_json::Value, BillingError>;

    async fn compute_subscription_mrr(
        &self,
        plan_id: &str,
        quantity: i32,
    ) -> Result<Decimal, BillingError>;

    async fn emit_subscription_created_event(
        &self,
        created: &Subscription,
        mrr: Decimal,
    ) -> Result<(), BillingError>;

    async fn emit_mrr_delta_event(
        &self,
        after: &Subscription,
        before: &Subscription,
        trigger: &str,
        event_type: &'static str,
        amount: Decimal,
    ) -> Result<(), BillingError>;

    async fn change_plan_with_proration(
        &self,
        subscription_id: &str,
        req: &ChangePlanRequest,
    ) -> Result<rustbill_core::billing::plan_change::ChangePlanOutput, BillingError>;

    async fn emit_subscription_plan_changed_notification(
        &self,
        http_client: &reqwest::Client,
        subscription_id: &str,
        customer_id: &str,
        old_plan_name: &str,
        new_plan_name: &str,
        proration_net: &str,
    ) -> Result<(), BillingError>;
}

#[derive(Clone)]
pub struct SqlxSubscriptionsRepository {
    pool: PgPool,
}

impl SqlxSubscriptionsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SubscriptionsRepository for SqlxSubscriptionsRepository {
    async fn list_admin(
        &self,
        role_customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(s) FROM subscriptions s
               WHERE ($1::text IS NULL OR s.customer_id = $1)
               ORDER BY s.created_at DESC"#,
        )
        .bind(role_customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn get_by_id(&self, id: &str) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT to_jsonb(s) FROM subscriptions s WHERE s.id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("subscription", id))
    }

    async fn create_admin(
        &self,
        req: &CreateSubscriptionRequest,
        metadata: serde_json::Value,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, quantity, metadata, cancel_at_period_end, version, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, 'active', now(), now() + interval '1 month', COALESCE($3, 1), $4, false, 1, now(), now())
               RETURNING to_jsonb(subscriptions.*)"#,
        )
        .bind(&req.customer_id)
        .bind(&req.plan_id)
        .bind(req.quantity_i32())
        .bind(metadata)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update_admin(
        &self,
        id: &str,
        req: &UpdateSubscriptionRequest,
        metadata: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE subscriptions SET
                 plan_id = COALESCE($2, plan_id),
                 status = COALESCE($3::subscription_status, status),
                 metadata = COALESCE($4, metadata),
                 updated_at = now()
               WHERE id = $1
               RETURNING to_jsonb(subscriptions.*)"#,
        )
        .bind(id)
        .bind(req.plan_id.as_deref())
        .bind(req.status.as_deref())
        .bind(metadata)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("subscription", id))
    }

    async fn find_active_subscription(&self, id: &str) -> Result<Subscription, BillingError> {
        sqlx::query_as::<_, Subscription>(
            "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("subscription", id))
    }

    async fn cancel_subscription(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query(
            "UPDATE subscriptions SET status = 'canceled', canceled_at = now(), updated_at = now(), version = version + 1 WHERE id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(BillingError::from)?;
        Ok(result.rows_affected())
    }

    async fn lifecycle_update_status(
        &self,
        subscription_id: &str,
        new_status: &str,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE subscriptions SET status = $2::subscription_status, updated_at = now(), version = version + 1
               WHERE id = $1
               RETURNING to_jsonb(subscriptions.*)"#,
        )
        .bind(subscription_id)
        .bind(new_status)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("subscription", subscription_id))
    }

    async fn list_v1(
        &self,
        status: Option<&str>,
        customer_id: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT to_jsonb(s) FROM subscriptions s
               WHERE ($1::text IS NULL OR s.status::text = $1)
                 AND ($2::text IS NULL OR s.customer_id = $2)
               ORDER BY s.created_at DESC"#,
        )
        .bind(status)
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn create_v1(
        &self,
        req: &CreateSubscriptionV1Request,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, quantity, metadata, cancel_at_period_end, version, created_at, updated_at)
               VALUES (gen_random_uuid()::text, $1, $2, 'active', now(), now() + interval '1 month', COALESCE($3, 1), $4, false, 1, now(), now())
               RETURNING to_jsonb(subscriptions.*)"#,
        )
        .bind(req.customer_id.as_deref())
        .bind(req.plan_id.as_deref())
        .bind(req.quantity_i32())
        .bind(req.metadata_or_default())
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update_v1(
        &self,
        id: &str,
        req: &UpdateSubscriptionV1Request,
    ) -> Result<serde_json::Value, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"UPDATE subscriptions SET
                 plan_id = COALESCE($2, plan_id),
                 status = COALESCE($3::subscription_status, status),
                 metadata = COALESCE($4, metadata),
                 updated_at = now()
               WHERE id = $1
               RETURNING to_jsonb(subscriptions.*)"#,
        )
        .bind(id)
        .bind(req.plan_id.as_deref())
        .bind(req.status.as_deref())
        .bind(req.metadata.clone())
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("subscription", id))
    }

    async fn compute_subscription_mrr(
        &self,
        plan_id: &str,
        quantity: i32,
    ) -> Result<Decimal, BillingError> {
        let plan = rustbill_core::billing::plans::get_plan(&self.pool, plan_id).await?;
        let tiers = plan
            .tiers
            .as_ref()
            .and_then(|value| serde_json::from_value::<Vec<PricingTier>>(value.clone()).ok());

        Ok(rustbill_core::billing::tiered_pricing::calculate_amount(
            &plan.pricing_model,
            plan.base_price,
            plan.unit_price,
            tiers.as_deref(),
            quantity,
        ))
    }

    async fn emit_subscription_created_event(
        &self,
        created: &Subscription,
        mrr: Decimal,
    ) -> Result<(), BillingError> {
        emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "mrr_expanded",
                classification: SalesClassification::Recurring,
                amount_subtotal: mrr,
                amount_tax: Decimal::ZERO,
                amount_total: mrr,
                currency: "USD",
                customer_id: Some(&created.customer_id),
                subscription_id: Some(&created.id),
                product_id: None,
                invoice_id: None,
                payment_id: None,
                source_table: "subscriptions",
                source_id: &created.id,
                metadata: Some(serde_json::json!({
                    "trigger": "subscription_create",
                    "plan_id": created.plan_id,
                    "quantity": created.quantity,
                })),
            },
        )
        .await
    }

    async fn emit_mrr_delta_event(
        &self,
        after: &Subscription,
        before: &Subscription,
        trigger: &str,
        event_type: &'static str,
        amount: Decimal,
    ) -> Result<(), BillingError> {
        let source_id = format!("{}:v{}", after.id, after.version);

        emit_sales_event(
            &self.pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type,
                classification: SalesClassification::Recurring,
                amount_subtotal: amount,
                amount_tax: Decimal::ZERO,
                amount_total: amount,
                currency: "USD",
                customer_id: Some(&after.customer_id),
                subscription_id: Some(&after.id),
                product_id: None,
                invoice_id: None,
                payment_id: None,
                source_table: "subscription_revisions",
                source_id: &source_id,
                metadata: Some(serde_json::json!({
                    "trigger": trigger,
                    "from_status": before.status,
                    "to_status": after.status,
                    "from_plan_id": before.plan_id,
                    "to_plan_id": after.plan_id,
                    "from_quantity": before.quantity,
                    "to_quantity": after.quantity,
                })),
            },
        )
        .await
    }

    async fn change_plan_with_proration(
        &self,
        subscription_id: &str,
        req: &ChangePlanRequest,
    ) -> Result<rustbill_core::billing::plan_change::ChangePlanOutput, BillingError> {
        rustbill_core::billing::plan_change::change_plan_with_proration(
            &self.pool,
            rustbill_core::billing::plan_change::ChangePlanInput {
                subscription_id,
                new_plan_id: &req.plan_id,
                new_quantity: req.quantity,
                idempotency_key: req.idempotency_key.as_deref(),
                now: chrono::Utc::now().naive_utc(),
            },
        )
        .await
    }

    async fn emit_subscription_plan_changed_notification(
        &self,
        http_client: &reqwest::Client,
        subscription_id: &str,
        customer_id: &str,
        old_plan_name: &str,
        new_plan_name: &str,
        proration_net: &str,
    ) -> Result<(), BillingError> {
        rustbill_core::notifications::events::emit_billing_event(
            &self.pool,
            http_client,
            BillingEventType::SubscriptionPlanChanged,
            "subscription",
            subscription_id,
            Some(customer_id),
            Some(serde_json::json!({
                "old_plan": old_plan_name,
                "new_plan": new_plan_name,
                "proration_net": proration_net,
            })),
        )
        .await?;
        Ok(())
    }
}
