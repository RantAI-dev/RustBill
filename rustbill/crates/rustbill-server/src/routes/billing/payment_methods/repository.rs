use super::schema::CreatePaymentMethodRequest;
use async_trait::async_trait;
use rustbill_core::billing::payment_methods as core_payment_methods;
use rustbill_core::db::models::SavedPaymentMethod;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[async_trait]
pub trait PaymentMethodRepository: Send + Sync {
    async fn list_for_customer(
        &self,
        customer_id: &str,
    ) -> Result<Vec<SavedPaymentMethod>, BillingError>;
    async fn create(
        &self,
        req: &CreatePaymentMethodRequest,
    ) -> Result<SavedPaymentMethod, BillingError>;
    async fn find_customer_id(&self, method_id: &str) -> Result<Option<String>, BillingError>;
    async fn remove(&self, customer_id: &str, method_id: &str) -> Result<(), BillingError>;
    async fn set_default(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> Result<SavedPaymentMethod, BillingError>;
}

#[derive(Clone)]
pub struct SqlxPaymentMethodRepository {
    pool: PgPool,
}

impl SqlxPaymentMethodRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentMethodRepository for SqlxPaymentMethodRepository {
    async fn list_for_customer(
        &self,
        customer_id: &str,
    ) -> Result<Vec<SavedPaymentMethod>, BillingError> {
        core_payment_methods::list_for_customer(&self.pool, customer_id).await
    }

    async fn create(
        &self,
        req: &CreatePaymentMethodRequest,
    ) -> Result<SavedPaymentMethod, BillingError> {
        core_payment_methods::create(
            &self.pool,
            core_payment_methods::CreatePaymentMethodRequest {
                customer_id: req.customer_id.clone(),
                provider: req.provider.clone(),
                provider_token: req.provider_token.clone(),
                method_type: req.method_type.clone(),
                label: req.label.clone(),
                last_four: req.last_four.clone(),
                expiry_month: req.expiry_month,
                expiry_year: req.expiry_year,
                set_default: req.set_default,
            },
        )
        .await
    }

    async fn find_customer_id(&self, method_id: &str) -> Result<Option<String>, BillingError> {
        sqlx::query_scalar::<_, String>(
            "SELECT customer_id FROM saved_payment_methods WHERE id = $1",
        )
        .bind(method_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn remove(&self, customer_id: &str, method_id: &str) -> Result<(), BillingError> {
        core_payment_methods::remove(&self.pool, customer_id, method_id).await
    }

    async fn set_default(
        &self,
        customer_id: &str,
        method_id: &str,
    ) -> Result<SavedPaymentMethod, BillingError> {
        core_payment_methods::set_default(&self.pool, customer_id, method_id).await
    }
}
