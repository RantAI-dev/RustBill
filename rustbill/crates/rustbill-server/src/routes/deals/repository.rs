use async_trait::async_trait;
use rustbill_core::db::models::{Deal, DealType, ProductType};
use rustbill_core::deals as core_deals;
use rustbill_core::deals::validation as core_validation;
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct CreateDealParams {
    pub customer_id: Option<String>,
    pub company: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub value: rust_decimal::Decimal,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
    pub deal_type: DealType,
    pub date: Option<String>,
    pub license_key: Option<String>,
    pub notes: Option<String>,
    pub usage_metric_label: Option<String>,
    pub usage_metric_value: Option<i32>,
    pub auto_create_invoice: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateDealParams {
    pub customer_id: Option<String>,
    pub company: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub value: Option<rust_decimal::Decimal>,
    pub product_id: Option<String>,
    pub product_name: Option<String>,
    pub product_type: Option<ProductType>,
    pub deal_type: Option<DealType>,
    pub date: Option<String>,
    pub license_key: Option<String>,
    pub notes: Option<String>,
    pub usage_metric_label: Option<String>,
    pub usage_metric_value: Option<i32>,
    pub auto_create_invoice: Option<bool>,
}

#[async_trait]
pub trait DealsRepository: Send + Sync {
    async fn list(
        &self,
        product_type: Option<&str>,
        deal_type: Option<&str>,
    ) -> Result<Vec<Deal>, BillingError>;
    async fn get(&self, id: &str) -> Result<Deal, BillingError>;
    async fn create(&self, body: &CreateDealParams) -> Result<Deal, BillingError>;
    async fn update(&self, id: &str, body: &UpdateDealParams) -> Result<Deal, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxDealsRepository {
    pool: PgPool,
}

impl SqlxDealsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DealsRepository for SqlxDealsRepository {
    async fn list(
        &self,
        product_type: Option<&str>,
        deal_type: Option<&str>,
    ) -> Result<Vec<Deal>, BillingError> {
        core_deals::list_deals(&self.pool, product_type, deal_type).await
    }

    async fn get(&self, id: &str) -> Result<Deal, BillingError> {
        core_deals::get_deal(&self.pool, id).await
    }

    async fn create(&self, body: &CreateDealParams) -> Result<Deal, BillingError> {
        core_deals::create_deal(
            &self.pool,
            core_validation::CreateDealRequest {
                customer_id: body.customer_id.clone(),
                company: body.company.clone(),
                contact: body.contact.clone(),
                email: body.email.clone(),
                value: body.value,
                product_id: body.product_id.clone(),
                product_name: body.product_name.clone(),
                product_type: body.product_type.clone(),
                deal_type: body.deal_type.clone(),
                date: body.date.clone(),
                license_key: body.license_key.clone(),
                notes: body.notes.clone(),
                usage_metric_label: body.usage_metric_label.clone(),
                usage_metric_value: body.usage_metric_value,
                auto_create_invoice: body.auto_create_invoice,
            },
        )
        .await
    }

    async fn update(&self, id: &str, body: &UpdateDealParams) -> Result<Deal, BillingError> {
        core_deals::update_deal(
            &self.pool,
            id,
            core_validation::UpdateDealRequest {
                customer_id: body.customer_id.clone(),
                company: body.company.clone(),
                contact: body.contact.clone(),
                email: body.email.clone(),
                value: body.value,
                product_id: body.product_id.clone(),
                product_name: body.product_name.clone(),
                product_type: body.product_type.clone(),
                deal_type: body.deal_type.clone(),
                date: body.date.clone(),
                license_key: body.license_key.clone(),
                notes: body.notes.clone(),
                usage_metric_label: body.usage_metric_label.clone(),
                usage_metric_value: body.usage_metric_value,
                auto_create_invoice: body.auto_create_invoice,
            },
        )
        .await
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        core_deals::delete_deal(&self.pool, id).await?;
        Ok(1)
    }
}
