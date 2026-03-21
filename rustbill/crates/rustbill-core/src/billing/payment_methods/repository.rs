use super::schema::CreatePaymentMethodDraft;
use crate::db::models::SavedPaymentMethod;
use crate::error::Result;
use async_trait::async_trait;
use sqlx::PgPool;

#[async_trait]
pub trait PaymentMethodRepository: Send + Sync {
    async fn list_for_customer(&self, customer_id: &str) -> Result<Vec<SavedPaymentMethod>>;
    async fn get_default(&self, customer_id: &str) -> Result<Option<SavedPaymentMethod>>;
    async fn count_active_for_customer(&self, customer_id: &str) -> Result<i64>;
    async fn create(&self, draft: &CreatePaymentMethodDraft) -> Result<SavedPaymentMethod>;
    async fn set_default(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> Result<Option<SavedPaymentMethod>>;
    async fn remove(&self, customer_id: &str, method_id: &str) -> Result<u64>;
    async fn mark_failed(&self, method_id: &str) -> Result<()>;
}

#[derive(Clone)]
pub struct PgPaymentMethodRepository {
    pool: PgPool,
}

impl PgPaymentMethodRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PaymentMethodRepository for PgPaymentMethodRepository {
    async fn list_for_customer(&self, customer_id: &str) -> Result<Vec<SavedPaymentMethod>> {
        let methods = sqlx::query_as::<_, SavedPaymentMethod>(
            "SELECT * FROM saved_payment_methods WHERE customer_id = $1 ORDER BY is_default DESC, created_at DESC",
        )
        .bind(customer_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(methods)
    }

    async fn get_default(&self, customer_id: &str) -> Result<Option<SavedPaymentMethod>> {
        let method = sqlx::query_as::<_, SavedPaymentMethod>(
            "SELECT * FROM saved_payment_methods WHERE customer_id = $1 AND is_default = TRUE AND status = 'active'",
        )
        .bind(customer_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(method)
    }

    async fn count_active_for_customer(&self, customer_id: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM saved_payment_methods WHERE customer_id = $1 AND status = 'active'",
        )
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(count)
    }

    async fn create(&self, draft: &CreatePaymentMethodDraft) -> Result<SavedPaymentMethod> {
        let mut tx = self.pool.begin().await?;

        if draft.clear_existing_default {
            sqlx::query(
                "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
            )
            .bind(&draft.customer_id)
            .execute(&mut *tx)
            .await?;
        }

        let method = sqlx::query_as::<_, SavedPaymentMethod>(
            r#"INSERT INTO saved_payment_methods
               (id, customer_id, provider, provider_token, method_type, label, last_four, expiry_month, expiry_year, is_default, status)
               VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
               RETURNING *"#,
        )
        .bind(&draft.customer_id)
        .bind(&draft.provider)
        .bind(&draft.provider_token)
        .bind(&draft.method_type)
        .bind(&draft.label)
        .bind(&draft.last_four)
        .bind(draft.expiry_month)
        .bind(draft.expiry_year)
        .bind(draft.is_default)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(method)
    }

    async fn set_default(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> Result<Option<SavedPaymentMethod>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
        )
        .bind(customer_id)
        .execute(&mut *tx)
        .await?;

        let method = sqlx::query_as::<_, SavedPaymentMethod>(
            "UPDATE saved_payment_methods SET is_default = TRUE, updated_at = NOW() WHERE id = $1 AND customer_id = $2 RETURNING *",
        )
        .bind(method_id)
        .bind(customer_id)
        .fetch_optional(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(method)
    }

    async fn remove(&self, customer_id: &str, method_id: &str) -> Result<u64> {
        let result =
            sqlx::query("DELETE FROM saved_payment_methods WHERE id = $1 AND customer_id = $2")
                .bind(method_id)
                .bind(customer_id)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    async fn mark_failed(&self, method_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE saved_payment_methods SET status = 'failed', is_default = FALSE, updated_at = NOW() WHERE id = $1",
        )
        .bind(method_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
