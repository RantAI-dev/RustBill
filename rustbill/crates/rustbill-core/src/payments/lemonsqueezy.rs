//! LemonSqueezy payment integration: checkouts, webhook verification.

use crate::error::Result;
use crate::settings::provider_settings::ProviderSettings;
use hmac::{Hmac, Mac};
use reqwest::Client;
use rust_decimal::Decimal;
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

const API_BASE: &str = "https://api.lemonsqueezy.com/v1";

/// Create a LemonSqueezy checkout.
pub async fn create_checkout(
    http: &Client,
    settings: &ProviderSettings,
    params: LsCheckoutParams,
) -> Result<LsCheckoutResult> {
    let api_key = settings.get("lemonsqueezy_api_key")
        .ok_or_else(|| crate::error::BillingError::ProviderNotConfigured("lemonsqueezy".to_string()))?;
    let store_id = settings.get("lemonsqueezy_store_id")
        .ok_or_else(|| crate::error::BillingError::ProviderNotConfigured("lemonsqueezy".to_string()))?;

    let amount_cents = (params.total * Decimal::from(100))
        .to_string()
        .parse::<i64>()
        .unwrap_or(0);

    let body = serde_json::json!({
        "data": {
            "type": "checkouts",
            "attributes": {
                "custom_price": amount_cents,
                "product_options": {
                    "name": format!("Invoice {}", params.invoice_number),
                    "description": format!("Payment for invoice {}", params.invoice_number),
                    "enabled_variants": [],
                },
                "checkout_options": { "embed": false },
                "checkout_data": {
                    "email": params.customer_email,
                    "name": params.customer_name,
                    "custom": { "invoiceId": params.invoice_id },
                },
                "success_url": params.success_url,
            },
            "relationships": {
                "store": {
                    "data": { "type": "stores", "id": store_id }
                }
            }
        }
    });

    let resp = http
        .post(format!("{API_BASE}/checkouts"))
        .header("Accept", "application/vnd.api+json")
        .header("Content-Type", "application/vnd.api+json")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("lemonsqueezy request failed: {e}"))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("lemonsqueezy checkout failed: {text}").into());
    }

    let json: serde_json::Value = resp.json().await
        .map_err(|e| anyhow::anyhow!("lemonsqueezy response parse failed: {e}"))?;

    let checkout_url = json["data"]["attributes"]["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let checkout_id = json["data"]["id"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    Ok(LsCheckoutResult { checkout_url, checkout_id })
}

/// Verify LemonSqueezy webhook signature (HMAC-SHA256, constant-time).
pub fn verify_webhook(raw_body: &str, signature: Option<&str>, secret: &str) -> bool {
    let sig = match signature {
        Some(s) => s,
        None => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(raw_body.as_bytes());
    let digest = hex::encode(mac.finalize().into_bytes());

    let a = digest.as_bytes();
    let b = sig.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

#[derive(Debug)]
pub struct LsCheckoutParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub customer_name: Option<String>,
    pub success_url: String,
}

#[derive(Debug)]
pub struct LsCheckoutResult {
    pub checkout_url: String,
    pub checkout_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_webhook_valid() {
        let secret = "test_secret";
        let body = r#"{"event":"order_created"}"#;

        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());

        assert!(verify_webhook(body, Some(&sig), secret));
    }

    #[test]
    fn test_verify_webhook_invalid() {
        assert!(!verify_webhook("body", Some("wrong"), "secret"));
        assert!(!verify_webhook("body", None, "secret"));
    }
}
