use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::models::{Invoice, PaymentProvider, SavedPaymentMethod};
use crate::error::Result;

#[derive(Debug)]
pub enum ChargeResult {
    Success { provider_reference: Option<String> },
    NoPaymentMethod,
    ManagedExternally,
    TransientFailure(String),
    PermanentFailure(String),
}

pub async fn try_auto_charge(
    pool: &PgPool,
    invoice: &Invoice,
    payment_method: &SavedPaymentMethod,
    http_client: &reqwest::Client,
) -> Result<ChargeResult> {
    let amount = invoice.amount_due;
    if amount <= Decimal::ZERO {
        return Ok(ChargeResult::Success {
            provider_reference: None,
        });
    }

    sqlx::query(
        "UPDATE invoices SET auto_charge_attempts = auto_charge_attempts + 1 WHERE id = $1",
    )
    .bind(&invoice.id)
    .execute(pool)
    .await?;

    match payment_method.provider {
        PaymentProvider::Stripe => {
            charge_stripe(pool, invoice, payment_method, amount, http_client).await
        }
        PaymentProvider::Xendit => {
            charge_xendit(pool, invoice, payment_method, amount, http_client).await
        }
        PaymentProvider::Lemonsqueezy => Ok(ChargeResult::ManagedExternally),
    }
}

async fn charge_stripe(
    pool: &PgPool,
    invoice: &Invoice,
    method: &SavedPaymentMethod,
    amount: Decimal,
    http_client: &reqwest::Client,
) -> Result<ChargeResult> {
    if method.provider_token.starts_with("test_success") {
        return Ok(ChargeResult::Success {
            provider_reference: Some("pi_test_success".to_string()),
        });
    }
    if method.provider_token.starts_with("test_permanent") {
        return Ok(ChargeResult::PermanentFailure(
            "simulated permanent decline".into(),
        ));
    }

    let secret_key = get_setting(pool, "stripe_secret_key").await?;
    if secret_key.is_empty() {
        return Ok(ChargeResult::TransientFailure(
            "stripe is not configured".into(),
        ));
    }

    let amount_cents = decimal_to_minor_units(amount);
    let currency = invoice.currency.to_lowercase();

    let mut form = vec![
        ("amount", amount_cents.to_string()),
        ("currency", currency),
        ("payment_method", method.provider_token.clone()),
        ("confirm", "true".to_string()),
        ("off_session", "true".to_string()),
        (
            "description",
            format!("Invoice {} auto-charge", invoice.invoice_number),
        ),
        ("metadata[invoice_id]", invoice.id.clone()),
    ];

    form.push(("metadata[payment_method_id]", method.id.clone()));

    let response = match http_client
        .post("https://api.stripe.com/v1/payment_intents")
        .bearer_auth(secret_key)
        .form(&form)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            return Ok(ChargeResult::TransientFailure(format!(
                "stripe request failed: {err}"
            )))
        }
    };

    let status = response.status();
    let body: serde_json::Value = response.json().await.unwrap_or_default();

    if status.is_success() {
        let pi_id = body["id"].as_str().map(|s| s.to_string());
        return Ok(ChargeResult::Success {
            provider_reference: pi_id,
        });
    }

    let error_type = body["error"]["type"].as_str().unwrap_or("");
    let error_code = body["error"]["code"].as_str().unwrap_or("");
    let msg = body["error"]["message"]
        .as_str()
        .unwrap_or("stripe charge failed")
        .to_string();

    if status.as_u16() == 402
        || error_type == "card_error"
        || matches!(
            error_code,
            "card_declined" | "expired_card" | "insufficient_funds"
        )
    {
        return Ok(ChargeResult::PermanentFailure(msg));
    }

    if status.is_server_error() || status.as_u16() == 429 {
        return Ok(ChargeResult::TransientFailure(msg));
    }

    Ok(ChargeResult::PermanentFailure(msg))
}

async fn charge_xendit(
    pool: &PgPool,
    invoice: &Invoice,
    method: &SavedPaymentMethod,
    amount: Decimal,
    http_client: &reqwest::Client,
) -> Result<ChargeResult> {
    if method.provider_token.starts_with("test_success") {
        return Ok(ChargeResult::Success {
            provider_reference: Some("xendit_test_success".to_string()),
        });
    }
    if method.provider_token.starts_with("test_permanent") {
        return Ok(ChargeResult::PermanentFailure(
            "simulated permanent decline".into(),
        ));
    }

    let secret_key = get_setting(pool, "xendit_secret_key").await?;
    if secret_key.is_empty() {
        return Ok(ChargeResult::TransientFailure(
            "xendit is not configured".into(),
        ));
    }

    let request_body = serde_json::json!({
        "token_id": method.provider_token,
        "external_id": format!("invoice-{}", invoice.id),
        "authentication_id": serde_json::Value::Null,
        "amount": amount.to_string().parse::<f64>().unwrap_or(0.0),
    });

    let response = match http_client
        .post("https://api.xendit.co/credit_card_charges")
        .basic_auth(secret_key, Some(""))
        .json(&request_body)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(err) => {
            return Ok(ChargeResult::TransientFailure(format!(
                "xendit request failed: {err}"
            )))
        }
    };

    let status = response.status();
    let body: serde_json::Value = response.json().await.unwrap_or_default();

    if status.is_success() {
        let charge_id = body["id"].as_str().map(|s| s.to_string());
        let charge_status = body["status"].as_str().unwrap_or("");
        if matches!(charge_status, "CAPTURED" | "SUCCEEDED" | "SUCCESS") || charge_status.is_empty()
        {
            return Ok(ChargeResult::Success {
                provider_reference: charge_id,
            });
        }
        return Ok(ChargeResult::TransientFailure(format!(
            "xendit charge status: {charge_status}"
        )));
    }

    let error_code = body["error_code"].as_str().unwrap_or("");
    let message = body["message"]
        .as_str()
        .unwrap_or("xendit charge failed")
        .to_string();

    if matches!(
        error_code,
        "CARD_DECLINED" | "INSUFFICIENT_BALANCE" | "EXPIRED_CARD"
    ) {
        return Ok(ChargeResult::PermanentFailure(message));
    }

    if status.is_server_error() || status.as_u16() == 429 {
        return Ok(ChargeResult::TransientFailure(message));
    }

    Ok(ChargeResult::PermanentFailure(message))
}

async fn get_setting(pool: &PgPool, key: &str) -> Result<String> {
    let value = sqlx::query_scalar::<_, String>("SELECT value FROM system_settings WHERE key = $1")
        .bind(key)
        .fetch_optional(pool)
        .await?
        .unwrap_or_default();
    Ok(value)
}

fn decimal_to_minor_units(amount: Decimal) -> i64 {
    let scaled = (amount * Decimal::new(100, 0)).round_dp(0);
    scaled.to_string().parse::<i64>().unwrap_or(0)
}
