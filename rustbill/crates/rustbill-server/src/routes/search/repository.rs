use async_trait::async_trait;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait SearchRepository: Send + Sync {
    async fn search_customers(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError>;

    async fn search_products(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError>;

    async fn search_licenses(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError>;
}

#[derive(Clone)]
pub struct SqlxSearchRepository {
    pool: PgPool,
}

impl SqlxSearchRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SearchRepository for SqlxSearchRepository {
    async fn search_customers(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT jsonb_build_object('type', 'customer', 'data', to_jsonb(c))
               FROM customers c
               WHERE c.name ILIKE $1 OR c.email ILIKE $1
               LIMIT $2"#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn search_products(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT jsonb_build_object('type', 'product', 'data', to_jsonb(p))
               FROM products p
               WHERE p.name ILIKE $1
               LIMIT $2"#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn search_licenses(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<serde_json::Value>, BillingError> {
        sqlx::query_scalar::<_, serde_json::Value>(
            r#"SELECT jsonb_build_object('type', 'license', 'data', to_jsonb(l))
               FROM licenses l
               WHERE l.key ILIKE $1
               LIMIT $2"#,
        )
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(BillingError::from)
    }
}
