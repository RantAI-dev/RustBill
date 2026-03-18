use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres};

use crate::db::models::{CreditReason, CustomerCredit, CustomerCreditBalance};
use crate::error::{BillingError, Result};

pub async fn get_balance(pool: &PgPool, customer_id: &str, currency: &str) -> Result<Decimal> {
    let row: Option<CustomerCreditBalance> = sqlx::query_as(
        "SELECT * FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.balance).unwrap_or(Decimal::ZERO))
}

pub async fn deposit(
    pool: &PgPool,
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

    let mut tx = pool.begin().await?;
    let credit = deposit_in_tx(
        &mut tx,
        customer_id,
        currency,
        amount,
        reason,
        description,
        invoice_id,
    )
    .await?;
    tx.commit().await?;
    Ok(credit)
}

pub async fn deposit_in_tx(
    tx: &mut sqlx::Transaction<'_, Postgres>,
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

pub async fn apply_to_invoice(
    tx: &mut sqlx::Transaction<'_, Postgres>,
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

pub async fn list_credits(
    pool: &PgPool,
    customer_id: &str,
    currency: Option<&str>,
) -> Result<Vec<CustomerCredit>> {
    let credits = if let Some(curr) = currency {
        sqlx::query_as::<_, CustomerCredit>(
            "SELECT * FROM customer_credits WHERE customer_id = $1 AND currency = $2 ORDER BY created_at DESC",
        )
        .bind(customer_id)
        .bind(curr)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CustomerCredit>(
            "SELECT * FROM customer_credits WHERE customer_id = $1 ORDER BY created_at DESC",
        )
        .bind(customer_id)
        .fetch_all(pool)
        .await?
    };
    Ok(credits)
}
