use super::schema::{CustomerContact, EmitBillingEventRequest, WebhookEndpoint};
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait NotificationsRepository: Send + Sync {
    async fn insert_billing_event(&self, req: &EmitBillingEventRequest) -> Result<String>;
    async fn list_active_webhook_endpoints(&self) -> Result<Vec<WebhookEndpoint>>;
    async fn insert_webhook_delivery(
        &self,
        endpoint_id: &str,
        event_id: &str,
        payload: &serde_json::Value,
    ) -> Result<Option<String>>;
    async fn update_webhook_delivery(
        &self,
        delivery_id: &str,
        status_code: i32,
        body: &str,
        attempts: i32,
    ) -> Result<()>;
    async fn find_customer_contact(&self, customer_id: &str) -> Result<Option<CustomerContact>>;
}

#[derive(Clone)]
pub struct PgNotificationsRepository {
    pool: PgPool,
}

impl PgNotificationsRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct WebhookRow {
    id: String,
    url: String,
    secret: String,
    events: serde_json::Value,
}

#[derive(Debug, sqlx::FromRow)]
struct CustomerContactRow {
    email: String,
    name: String,
}

#[async_trait]
impl NotificationsRepository for PgNotificationsRepository {
    async fn insert_billing_event(&self, req: &EmitBillingEventRequest) -> Result<String> {
        let event_id: String = sqlx::query_scalar(
            r#"
            INSERT INTO billing_events (id, event_type, resource_type, resource_id, customer_id, data)
            VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5)
            RETURNING id
            "#,
        )
        .bind(&req.event_type)
        .bind(&req.resource_type)
        .bind(&req.resource_id)
        .bind(&req.customer_id)
        .bind(&req.data)
        .fetch_one(&self.pool)
        .await?;

        Ok(event_id)
    }

    async fn list_active_webhook_endpoints(&self) -> Result<Vec<WebhookEndpoint>> {
        let rows: Vec<WebhookRow> = sqlx::query_as(
            "SELECT id, url, secret, events FROM webhook_endpoints WHERE status = 'active'",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| WebhookEndpoint {
                id: row.id,
                url: row.url,
                secret: row.secret,
                events: row.events,
            })
            .collect())
    }

    async fn insert_webhook_delivery(
        &self,
        endpoint_id: &str,
        event_id: &str,
        payload: &serde_json::Value,
    ) -> Result<Option<String>> {
        let delivery_id: Option<String> = sqlx::query_scalar(
            r#"
            INSERT INTO webhook_deliveries (id, endpoint_id, event_id, payload, attempts)
            VALUES (gen_random_uuid()::text, $1, $2, $3, 0)
            RETURNING id
            "#,
        )
        .bind(endpoint_id)
        .bind(event_id)
        .bind(payload)
        .fetch_optional(&self.pool)
        .await?;

        Ok(delivery_id)
    }

    async fn update_webhook_delivery(
        &self,
        delivery_id: &str,
        status_code: i32,
        body: &str,
        attempts: i32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE webhook_deliveries SET response_code = $2, response_body = $3, attempts = $4, delivered_at = NOW() WHERE id = $1",
        )
        .bind(delivery_id)
        .bind(status_code)
        .bind(body)
        .bind(attempts)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn find_customer_contact(&self, customer_id: &str) -> Result<Option<CustomerContact>> {
        let row: Option<CustomerContactRow> = sqlx::query_as(
            "SELECT COALESCE(billing_email, email) AS email, name FROM customers WHERE id = $1",
        )
        .bind(customer_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| CustomerContact {
            email: r.email,
            name: r.name,
        }))
    }
}
