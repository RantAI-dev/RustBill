use super::schema::{AutoChargeContext, ChargeResult};
use crate::error::Result;
use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;

#[async_trait]
pub trait AutoChargeRepository {
    async fn increment_attempts(&self, invoice_id: &str) -> Result<()>;
    async fn get_setting(&self, key: &str) -> Result<String>;
    async fn stripe_charge(
        &self,
        context: &AutoChargeContext,
        amount: Decimal,
    ) -> Result<ChargeResult>;
    async fn xendit_charge(
        &self,
        context: &AutoChargeContext,
        amount: Decimal,
    ) -> Result<ChargeResult>;
}

pub struct PgAutoChargeRepository<'a> {
    pool: &'a PgPool,
    http_client: &'a Client,
}

impl<'a> PgAutoChargeRepository<'a> {
    pub fn new(pool: &'a PgPool, http_client: &'a Client) -> Self {
        Self { pool, http_client }
    }
}

#[async_trait]
impl AutoChargeRepository for PgAutoChargeRepository<'_> {
    async fn increment_attempts(&self, invoice_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE invoices SET auto_charge_attempts = auto_charge_attempts + 1 WHERE id = $1",
        )
        .bind(invoice_id)
        .execute(self.pool)
        .await?;

        Ok(())
    }

    async fn get_setting(&self, key: &str) -> Result<String> {
        let value =
            sqlx::query_scalar::<_, String>("SELECT value FROM system_settings WHERE key = $1")
                .bind(key)
                .fetch_optional(self.pool)
                .await?
                .unwrap_or_default();
        Ok(value)
    }

    async fn stripe_charge(
        &self,
        context: &AutoChargeContext,
        amount: Decimal,
    ) -> Result<ChargeResult> {
        let secret_key = self.get_setting("stripe_secret_key").await?;
        if secret_key.is_empty() {
            return Ok(ChargeResult::TransientFailure(
                "stripe is not configured".to_string(),
            ));
        }

        let amount_cents = decimal_to_minor_units(amount);
        let currency = context.invoice.currency.to_lowercase();

        let mut form = vec![
            ("amount", amount_cents.to_string()),
            ("currency", currency),
            (
                "payment_method",
                context.payment_method.provider_token.clone(),
            ),
            ("confirm", "true".to_string()),
            ("off_session", "true".to_string()),
            (
                "description",
                format!("Invoice {} auto-charge", context.invoice.invoice_number),
            ),
            ("metadata[invoice_id]", context.invoice.id.clone()),
        ];
        form.push((
            "metadata[payment_method_id]",
            context.payment_method.id.clone(),
        ));

        let response = match self
            .http_client
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
        let body: serde_json::Value = match response.json().await {
            Ok(value) => value,
            Err(_) => serde_json::Value::Null,
        };

        if status.is_success() {
            let pi_id = body["id"].as_str().map(ToString::to_string);
            return Ok(ChargeResult::Success {
                provider_reference: pi_id,
            });
        }

        let error_type = body["error"]["type"].as_str().unwrap_or("");
        let error_code = body["error"]["code"].as_str().unwrap_or("");
        let message = body["error"]["message"]
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
            return Ok(ChargeResult::PermanentFailure(message));
        }

        if status.is_server_error() || status.as_u16() == 429 {
            return Ok(ChargeResult::TransientFailure(message));
        }

        Ok(ChargeResult::PermanentFailure(message))
    }

    async fn xendit_charge(
        &self,
        context: &AutoChargeContext,
        amount: Decimal,
    ) -> Result<ChargeResult> {
        let secret_key = self.get_setting("xendit_secret_key").await?;
        if secret_key.is_empty() {
            return Ok(ChargeResult::TransientFailure(
                "xendit is not configured".to_string(),
            ));
        }

        let amount_f64 = amount.to_f64().unwrap_or(0.0);
        let request_body = json!({
            "token_id": context.payment_method.provider_token,
            "external_id": format!("invoice-{}", context.invoice.id),
            "authentication_id": serde_json::Value::Null,
            "amount": amount_f64,
        });

        let response = match self
            .http_client
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
        let body: serde_json::Value = match response.json().await {
            Ok(value) => value,
            Err(_) => serde_json::Value::Null,
        };

        if status.is_success() {
            let charge_id = body["id"].as_str().map(ToString::to_string);
            let charge_status = body["status"].as_str().unwrap_or("");
            if matches!(charge_status, "CAPTURED" | "SUCCEEDED" | "SUCCESS")
                || charge_status.is_empty()
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
}

fn decimal_to_minor_units(amount: Decimal) -> i64 {
    let scaled = (amount * Decimal::new(100, 0)).round_dp(0);
    scaled.to_i64().unwrap_or(0)
}
