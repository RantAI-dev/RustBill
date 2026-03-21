use super::schema::{
    ApplyCreditRequest, CreditAdjustmentRequest, CreditBalanceRequest, ListCreditsRequest,
};
use crate::db::models::{CreditReason, CustomerCredit, CustomerCreditBalance};
use crate::error::{BillingError, Result};
use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres, Transaction};

#[async_trait]
pub trait CreditsRepository {
    async fn get_balance(&mut self, req: &CreditBalanceRequest) -> Result<Option<Decimal>>;
    async fn list_credits(&mut self, req: &ListCreditsRequest) -> Result<Vec<CustomerCredit>>;
    async fn adjust_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit>;
    async fn deposit_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit>;
    async fn apply_credit_to_invoice(&mut self, req: &ApplyCreditRequest) -> Result<Decimal>;
}

#[derive(Clone)]
pub struct PgCreditsRepository {
    pool: PgPool,
}

impl PgCreditsRepository {
    pub fn new(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

pub struct PgCreditsTransaction<'tx, 'conn> {
    tx: &'tx mut Transaction<'conn, Postgres>,
}

impl<'tx, 'conn> PgCreditsTransaction<'tx, 'conn> {
    pub fn new(tx: &'tx mut Transaction<'conn, Postgres>) -> Self {
        Self { tx }
    }
}

#[async_trait]
impl CreditsRepository for PgCreditsRepository {
    async fn get_balance(&mut self, req: &CreditBalanceRequest) -> Result<Option<Decimal>> {
        let row: Option<CustomerCreditBalance> = sqlx::query_as(
            "SELECT * FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2",
        )
        .bind(&req.customer_id)
        .bind(&req.currency)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.balance))
    }

    async fn list_credits(&mut self, req: &ListCreditsRequest) -> Result<Vec<CustomerCredit>> {
        let credits = if let Some(currency) = req.currency.as_deref() {
            sqlx::query_as::<_, CustomerCredit>(
                "SELECT * FROM customer_credits WHERE customer_id = $1 AND currency = $2 ORDER BY created_at DESC",
            )
            .bind(&req.customer_id)
            .bind(currency)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, CustomerCredit>(
                "SELECT * FROM customer_credits WHERE customer_id = $1 ORDER BY created_at DESC",
            )
            .bind(&req.customer_id)
            .fetch_all(&self.pool)
            .await?
        };

        Ok(credits)
    }

    async fn adjust_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit> {
        let mut tx = self.pool.begin().await?;
        let credit = adjust_credit_in_tx(
            &mut tx,
            &req.customer_id,
            &req.currency,
            req.amount,
            req.reason.clone(),
            &req.description,
            req.invoice_id.as_deref(),
        )
        .await?;
        tx.commit().await?;
        Ok(credit)
    }

    async fn deposit_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit> {
        let mut tx = self.pool.begin().await?;
        let credit = deposit_credit_in_tx(
            &mut tx,
            &req.customer_id,
            &req.currency,
            req.amount,
            req.reason.clone(),
            &req.description,
            req.invoice_id.as_deref(),
        )
        .await?;
        tx.commit().await?;
        Ok(credit)
    }

    async fn apply_credit_to_invoice(&mut self, req: &ApplyCreditRequest) -> Result<Decimal> {
        let mut tx = self.pool.begin().await?;
        let applied = apply_credit_to_invoice_in_tx(
            &mut tx,
            &req.customer_id,
            &req.invoice_id,
            &req.currency,
            req.max_amount,
        )
        .await?;
        tx.commit().await?;
        Ok(applied)
    }
}

#[async_trait]
impl<'tx, 'conn> CreditsRepository for PgCreditsTransaction<'tx, 'conn> {
    async fn get_balance(&mut self, req: &CreditBalanceRequest) -> Result<Option<Decimal>> {
        let row: Option<CustomerCreditBalance> = sqlx::query_as(
            "SELECT * FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2",
        )
        .bind(&req.customer_id)
        .bind(&req.currency)
        .fetch_optional(&mut **self.tx)
        .await?;

        Ok(row.map(|r| r.balance))
    }

    async fn list_credits(&mut self, req: &ListCreditsRequest) -> Result<Vec<CustomerCredit>> {
        let credits = if let Some(currency) = req.currency.as_deref() {
            sqlx::query_as::<_, CustomerCredit>(
                "SELECT * FROM customer_credits WHERE customer_id = $1 AND currency = $2 ORDER BY created_at DESC",
            )
            .bind(&req.customer_id)
            .bind(currency)
            .fetch_all(&mut **self.tx)
            .await?
        } else {
            sqlx::query_as::<_, CustomerCredit>(
                "SELECT * FROM customer_credits WHERE customer_id = $1 ORDER BY created_at DESC",
            )
            .bind(&req.customer_id)
            .fetch_all(&mut **self.tx)
            .await?
        };

        Ok(credits)
    }

    async fn adjust_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit> {
        adjust_credit_in_tx(
            self.tx,
            &req.customer_id,
            &req.currency,
            req.amount,
            req.reason.clone(),
            &req.description,
            req.invoice_id.as_deref(),
        )
        .await
    }

    async fn deposit_credit(&mut self, req: &CreditAdjustmentRequest) -> Result<CustomerCredit> {
        deposit_credit_in_tx(
            self.tx,
            &req.customer_id,
            &req.currency,
            req.amount,
            req.reason.clone(),
            &req.description,
            req.invoice_id.as_deref(),
        )
        .await
    }

    async fn apply_credit_to_invoice(&mut self, req: &ApplyCreditRequest) -> Result<Decimal> {
        apply_credit_to_invoice_in_tx(
            self.tx,
            &req.customer_id,
            &req.invoice_id,
            &req.currency,
            req.max_amount,
        )
        .await
    }
}

async fn adjust_credit_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    if amount == Decimal::ZERO {
        return Err(BillingError::bad_request("adjust amount must be non-zero"));
    }

    let current_balance: Option<Decimal> = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2 FOR UPDATE",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_optional(&mut **tx)
    .await?;

    let next_balance = current_balance.unwrap_or(Decimal::ZERO) + amount;
    if next_balance < Decimal::ZERO {
        return Err(BillingError::bad_request(
            "adjustment would result in negative credit balance",
        ));
    }

    let balance_row: CustomerCreditBalance = sqlx::query_as(
        r#"INSERT INTO customer_credit_balances (customer_id, currency, balance, updated_at)
           VALUES ($1, $2, $3, NOW())
           ON CONFLICT (customer_id, currency)
           DO UPDATE SET balance = $3, updated_at = NOW()
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(next_balance)
    .fetch_one(&mut **tx)
    .await?;

    let credit = sqlx::query_as::<_, CustomerCredit>(
        r#"INSERT INTO customer_credits (id, customer_id, currency, amount, balance_after, reason, description, invoice_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, NOW())
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(amount)
    .bind(balance_row.balance)
    .bind(reason)
    .bind(description)
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(credit)
}

async fn deposit_credit_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    if amount <= Decimal::ZERO {
        return Err(BillingError::bad_request("deposit amount must be positive"));
    }

    let balance_row: CustomerCreditBalance = sqlx::query_as(
        r#"INSERT INTO customer_credit_balances (customer_id, currency, balance, updated_at)
           VALUES ($1, $2, $3, NOW())
           ON CONFLICT (customer_id, currency)
           DO UPDATE SET balance = customer_credit_balances.balance + $3, updated_at = NOW()
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(amount)
    .fetch_one(&mut **tx)
    .await?;

    let credit = sqlx::query_as::<_, CustomerCredit>(
        r#"INSERT INTO customer_credits (id, customer_id, currency, amount, balance_after, reason, description, invoice_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, NOW())
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(amount)
    .bind(balance_row.balance)
    .bind(reason)
    .bind(description)
    .bind(invoice_id)
    .fetch_one(&mut **tx)
    .await?;

    Ok(credit)
}

async fn apply_credit_to_invoice_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    customer_id: &str,
    invoice_id: &str,
    currency: &str,
    max_amount: Decimal,
) -> Result<Decimal> {
    if max_amount <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let current_balance: Option<Decimal> = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2 FOR UPDATE",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_optional(&mut **tx)
    .await?;

    let balance = current_balance.unwrap_or(Decimal::ZERO);
    if balance <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let apply_amount = max_amount.min(balance);

    let new_balance: Option<Decimal> = sqlx::query_scalar(
        "UPDATE customer_credit_balances
         SET balance = balance - $3, updated_at = NOW()
         WHERE customer_id = $1 AND currency = $2 AND balance >= $3
         RETURNING balance",
    )
    .bind(customer_id)
    .bind(currency)
    .bind(apply_amount)
    .fetch_optional(&mut **tx)
    .await?;

    let Some(new_balance) = new_balance else {
        return Ok(Decimal::ZERO);
    };

    sqlx::query(
        r#"INSERT INTO customer_credits (id, customer_id, currency, amount, balance_after, reason, description, invoice_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, 'Credit applied to invoice', $6, NOW())"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(-apply_amount)
    .bind(new_balance)
    .bind(CreditReason::Manual)
    .bind(invoice_id)
    .execute(&mut **tx)
    .await?;

    Ok(apply_amount)
}
