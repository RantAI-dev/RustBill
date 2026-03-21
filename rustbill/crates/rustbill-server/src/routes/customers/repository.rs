use async_trait::async_trait;
use rustbill_core::db::models::{Customer, PaymentMethod};
use rustbill_core::error::BillingError;
use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct CustomerCreateParams {
    pub name: String,
    pub industry: String,
    pub tier: String,
    pub location: String,
    pub contact: String,
    pub email: String,
    pub phone: String,
    pub billing_email: Option<String>,
    pub billing_address: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_zip: Option<String>,
    pub billing_country: Option<String>,
    pub tax_id: Option<String>,
    pub default_payment_method: Option<PaymentMethod>,
    pub stripe_customer_id: Option<String>,
    pub xendit_customer_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CustomerUpdateParams {
    pub name: Option<String>,
    pub industry: Option<String>,
    pub tier: Option<String>,
    pub location: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub billing_email: Option<String>,
    pub billing_address: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_zip: Option<String>,
    pub billing_country: Option<String>,
    pub tax_id: Option<String>,
    pub default_payment_method: Option<PaymentMethod>,
    pub stripe_customer_id: Option<String>,
    pub xendit_customer_id: Option<String>,
}

#[async_trait]
pub trait CustomerRepository: Send + Sync {
    async fn list(&self) -> Result<Vec<Customer>, BillingError>;
    async fn get(&self, id: &str) -> Result<Customer, BillingError>;
    async fn create(&self, body: &CustomerCreateParams) -> Result<Customer, BillingError>;
    async fn update(&self, id: &str, body: &CustomerUpdateParams)
        -> Result<Customer, BillingError>;
    async fn delete(&self, id: &str) -> Result<u64, BillingError>;
}

#[derive(Clone)]
pub struct SqlxCustomerRepository {
    pool: PgPool,
}

impl SqlxCustomerRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CustomerRepository for SqlxCustomerRepository {
    async fn list(&self) -> Result<Vec<Customer>, BillingError> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(BillingError::from)
    }

    async fn get(&self, id: &str) -> Result<Customer, BillingError> {
        sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingError::from)?
            .ok_or_else(|| BillingError::not_found("customer", id))
    }

    async fn create(&self, body: &CustomerCreateParams) -> Result<Customer, BillingError> {
        sqlx::query_as::<_, Customer>(
            r#"INSERT INTO customers (
                 id, name, industry, tier, location, contact, email, phone,
                 total_revenue, health_score, trend, last_contact,
                 billing_email, billing_address, billing_city, billing_state,
                 billing_zip, billing_country, tax_id, default_payment_method,
                 stripe_customer_id, xendit_customer_id, created_at, updated_at
               )
               VALUES (
                 gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7,
                 0, 50, 'stable', '',
                 $8, $9, $10, $11, $12, $13, $14, $15::payment_method,
                 $16, $17, now(), now()
               )
               RETURNING *"#,
        )
        .bind(&body.name)
        .bind(&body.industry)
        .bind(&body.tier)
        .bind(&body.location)
        .bind(&body.contact)
        .bind(&body.email)
        .bind(&body.phone)
        .bind(&body.billing_email)
        .bind(&body.billing_address)
        .bind(&body.billing_city)
        .bind(&body.billing_state)
        .bind(&body.billing_zip)
        .bind(&body.billing_country)
        .bind(&body.tax_id)
        .bind(&body.default_payment_method)
        .bind(&body.stripe_customer_id)
        .bind(&body.xendit_customer_id)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingError::from)
    }

    async fn update(
        &self,
        id: &str,
        body: &CustomerUpdateParams,
    ) -> Result<Customer, BillingError> {
        sqlx::query_as::<_, Customer>(
            r#"UPDATE customers SET
                 name = COALESCE($2, name),
                 industry = COALESCE($3, industry),
                 tier = COALESCE($4, tier),
                 location = COALESCE($5, location),
                 contact = COALESCE($6, contact),
                 email = COALESCE($7, email),
                 phone = COALESCE($8, phone),
                 billing_email = COALESCE($9, billing_email),
                 billing_address = COALESCE($10, billing_address),
                 billing_city = COALESCE($11, billing_city),
                 billing_state = COALESCE($12, billing_state),
                 billing_zip = COALESCE($13, billing_zip),
                 billing_country = COALESCE($14, billing_country),
                 tax_id = COALESCE($15, tax_id),
                 default_payment_method = COALESCE($16::payment_method, default_payment_method),
                 stripe_customer_id = COALESCE($17, stripe_customer_id),
                 xendit_customer_id = COALESCE($18, xendit_customer_id),
                 updated_at = now()
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(id)
        .bind(&body.name)
        .bind(&body.industry)
        .bind(&body.tier)
        .bind(&body.location)
        .bind(&body.contact)
        .bind(&body.email)
        .bind(&body.phone)
        .bind(&body.billing_email)
        .bind(&body.billing_address)
        .bind(&body.billing_city)
        .bind(&body.billing_state)
        .bind(&body.billing_zip)
        .bind(&body.billing_country)
        .bind(&body.tax_id)
        .bind(&body.default_payment_method)
        .bind(&body.stripe_customer_id)
        .bind(&body.xendit_customer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(BillingError::from)?
        .ok_or_else(|| BillingError::not_found("customer", id))
    }

    async fn delete(&self, id: &str) -> Result<u64, BillingError> {
        let result = sqlx::query("DELETE FROM customers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(BillingError::from)?;

        Ok(result.rows_affected())
    }
}
