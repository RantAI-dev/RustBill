pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::{CreditReason, CustomerCredit};
use crate::error::Result;
use repository::{PgCreditsRepository, PgCreditsTransaction};
use rust_decimal::Decimal;
use sqlx::{Postgres, Transaction};

pub use schema::{
    ApplyCreditRequest, CreditAdjustmentRequest, CreditBalanceRequest, ListCreditsRequest,
};

pub async fn get_balance(
    pool: &sqlx::PgPool,
    customer_id: &str,
    currency: &str,
) -> Result<Decimal> {
    let mut repo = PgCreditsRepository::new(pool);
    service::get_balance(
        &mut repo,
        CreditBalanceRequest {
            customer_id: customer_id.to_string(),
            currency: currency.to_string(),
        },
    )
    .await
}

pub async fn deposit(
    pool: &sqlx::PgPool,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    let mut repo = PgCreditsRepository::new(pool);
    service::deposit(
        &mut repo,
        CreditAdjustmentRequest {
            customer_id: customer_id.to_string(),
            currency: currency.to_string(),
            amount,
            reason,
            description: description.to_string(),
            invoice_id: invoice_id.map(str::to_string),
        },
    )
    .await
}

pub async fn adjust(
    pool: &sqlx::PgPool,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    let mut repo = PgCreditsRepository::new(pool);
    service::adjust(
        &mut repo,
        CreditAdjustmentRequest {
            customer_id: customer_id.to_string(),
            currency: currency.to_string(),
            amount,
            reason,
            description: description.to_string(),
            invoice_id: invoice_id.map(str::to_string),
        },
    )
    .await
}

pub async fn adjust_in_tx<'tx, 'conn>(
    tx: &'tx mut Transaction<'conn, Postgres>,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    let mut repo = PgCreditsTransaction::new(tx);
    service::adjust_in_tx(
        &mut repo,
        CreditAdjustmentRequest {
            customer_id: customer_id.to_string(),
            currency: currency.to_string(),
            amount,
            reason,
            description: description.to_string(),
            invoice_id: invoice_id.map(str::to_string),
        },
    )
    .await
}

pub async fn deposit_in_tx<'tx, 'conn>(
    tx: &'tx mut Transaction<'conn, Postgres>,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit> {
    let mut repo = PgCreditsTransaction::new(tx);
    service::deposit_in_tx(
        &mut repo,
        CreditAdjustmentRequest {
            customer_id: customer_id.to_string(),
            currency: currency.to_string(),
            amount,
            reason,
            description: description.to_string(),
            invoice_id: invoice_id.map(str::to_string),
        },
    )
    .await
}

pub async fn apply_to_invoice<'tx, 'conn>(
    tx: &'tx mut Transaction<'conn, Postgres>,
    customer_id: &str,
    invoice_id: &str,
    currency: &str,
    max_amount: Decimal,
) -> Result<Decimal> {
    let mut repo = PgCreditsTransaction::new(tx);
    service::apply_to_invoice(
        &mut repo,
        ApplyCreditRequest {
            customer_id: customer_id.to_string(),
            invoice_id: invoice_id.to_string(),
            currency: currency.to_string(),
            max_amount,
        },
    )
    .await
}

pub async fn list_credits(
    pool: &sqlx::PgPool,
    customer_id: &str,
    currency: Option<&str>,
) -> Result<Vec<CustomerCredit>> {
    let mut repo = PgCreditsRepository::new(pool);
    service::list_credits(
        &mut repo,
        ListCreditsRequest {
            customer_id: customer_id.to_string(),
            currency: currency.map(str::to_string),
        },
    )
    .await
}
