use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::models::{Invoice, PaymentProvider, SavedPaymentMethod};
use crate::error::Result;

#[derive(Debug)]
pub enum ChargeResult {
    Success,
    NoPaymentMethod,
    ManagedExternally,
    TransientFailure(String),
    PermanentFailure(String),
}

pub async fn try_auto_charge(
    pool: &PgPool,
    invoice: &Invoice,
    payment_method: &SavedPaymentMethod,
    _http_client: &reqwest::Client,
) -> Result<ChargeResult> {
    let amount = invoice.amount_due;
    if amount <= Decimal::ZERO {
        return Ok(ChargeResult::Success);
    }

    sqlx::query(
        "UPDATE invoices SET auto_charge_attempts = auto_charge_attempts + 1 WHERE id = $1",
    )
    .bind(&invoice.id)
    .execute(pool)
    .await?;

    match payment_method.provider {
        PaymentProvider::Stripe => charge_stripe(pool, invoice, payment_method, amount).await,
        PaymentProvider::Xendit => charge_xendit(pool, invoice, payment_method, amount).await,
        PaymentProvider::Lemonsqueezy => Ok(ChargeResult::ManagedExternally),
    }
}

async fn charge_stripe(
    _pool: &PgPool,
    _invoice: &Invoice,
    method: &SavedPaymentMethod,
    _amount: Decimal,
) -> Result<ChargeResult> {
    if method.provider_token.starts_with("test_success") {
        return Ok(ChargeResult::Success);
    }
    if method.provider_token.starts_with("test_permanent") {
        return Ok(ChargeResult::PermanentFailure(
            "simulated permanent decline".into(),
        ));
    }

    tracing::warn!("Stripe auto-charge not yet implemented");
    Ok(ChargeResult::TransientFailure(
        "stripe auto-charge not implemented yet".into(),
    ))
}

async fn charge_xendit(
    _pool: &PgPool,
    _invoice: &Invoice,
    method: &SavedPaymentMethod,
    _amount: Decimal,
) -> Result<ChargeResult> {
    if method.provider_token.starts_with("test_success") {
        return Ok(ChargeResult::Success);
    }
    if method.provider_token.starts_with("test_permanent") {
        return Ok(ChargeResult::PermanentFailure(
            "simulated permanent decline".into(),
        ));
    }

    tracing::warn!("Xendit auto-charge not yet implemented");
    Ok(ChargeResult::TransientFailure(
        "xendit auto-charge not implemented yet".into(),
    ))
}
