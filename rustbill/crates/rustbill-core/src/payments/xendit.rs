//! Xendit payment integration: invoices, refunds, webhook verification.

use crate::error::Result;
use crate::settings::provider_settings::ProviderSettings;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;

#[derive(Debug, Serialize)]
struct CreateInvoiceRequest {
    external_id: String,
    amount: f64,
    currency: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payer_email: Option<String>,
    success_redirect_url: String,
    failure_redirect_url: String,
}

#[derive(Debug, Deserialize)]
struct CreateInvoiceResponse {
    id: String,
    invoice_url: String,
}

/// Create a Xendit invoice (checkout equivalent).
pub async fn create_invoice(
    http: &Client,
    settings: &ProviderSettings,
    params: XenditInvoiceParams,
) -> Result<XenditInvoiceResult> {
    let secret_key = settings
        .get("xendit_secret_key")
        .ok_or_else(|| crate::error::BillingError::ProviderNotConfigured("xendit".to_string()))?;

    let body = CreateInvoiceRequest {
        external_id: params.invoice_id.clone(),
        amount: params.total.to_string().parse::<f64>().unwrap_or(0.0),
        currency: params.currency.to_uppercase(),
        description: format!("Payment for invoice {}", params.invoice_number),
        payer_email: params.customer_email,
        success_redirect_url: params.success_url,
        failure_redirect_url: params.failure_url,
    };

    let resp = http
        .post("https://api.xendit.co/v2/invoices")
        .basic_auth(&secret_key, None::<&str>)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("xendit request failed: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("xendit invoice creation failed: {text}").into());
    }

    let result: CreateInvoiceResponse = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("xendit response parse failed: {e}"))?;

    Ok(XenditInvoiceResult {
        invoice_url: result.invoice_url,
        xendit_invoice_id: result.id,
    })
}

/// Create a Xendit refund.
pub async fn create_refund(
    http: &Client,
    settings: &ProviderSettings,
    payment_id: &str,
    amount: Decimal,
    currency: &str,
    reason: Option<&str>,
) -> Result<String> {
    let secret_key = settings
        .get("xendit_secret_key")
        .ok_or_else(|| crate::error::BillingError::ProviderNotConfigured("xendit".to_string()))?;

    let body = serde_json::json!({
        "payment_request_id": payment_id,
        "amount": amount.to_string().parse::<f64>().unwrap_or(0.0),
        "currency": currency.to_uppercase(),
        "reason": reason.unwrap_or("REQUESTED_BY_CUSTOMER"),
    });

    let resp = http
        .post("https://api.xendit.co/refunds")
        .basic_auth(&secret_key, None::<&str>)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("xendit refund request failed: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("xendit refund failed: {text}").into());
    }

    let result: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("xendit refund response parse failed: {e}"))?;

    Ok(result["id"].as_str().unwrap_or_default().to_string())
}

/// Verify Xendit webhook callback token (constant-time comparison).
pub fn verify_webhook(callback_token: Option<&str>, expected_token: &str) -> bool {
    match callback_token {
        Some(token) => {
            let a = token.as_bytes();
            let b = expected_token.as_bytes();
            if a.len() != b.len() {
                return false;
            }
            a.ct_eq(b).into()
        }
        None => false,
    }
}

#[derive(Debug)]
pub struct XenditInvoiceParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub customer_name: Option<String>,
    pub success_url: String,
    pub failure_url: String,
}

#[derive(Debug)]
pub struct XenditInvoiceResult {
    pub invoice_url: String,
    pub xendit_invoice_id: String,
}
