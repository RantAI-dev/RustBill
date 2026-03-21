use super::repository::ProviderSettingsRepository;
use super::schema::{
    CheckoutParams, LsCheckoutParams, LsCheckoutResult, XenditInvoiceParams, XenditInvoiceResult,
};
use crate::error::Result;
use hmac::{Hmac, Mac};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use subtle::ConstantTimeEq;

type HmacSha256 = Hmac<Sha256>;

const LS_API_BASE: &str = "https://api.lemonsqueezy.com/v1";

#[cfg(feature = "stripe")]
pub use stripe::Client as StripeClient;

#[cfg(feature = "stripe")]
pub fn create_stripe_client(secret_key: &str) -> StripeClient {
    StripeClient::new(secret_key)
}

#[cfg(feature = "stripe")]
pub async fn create_stripe_checkout_session(
    client: &StripeClient,
    params: CheckoutParams,
) -> Result<Option<String>> {
    let amount = params
        .total
        .to_string()
        .parse::<f64>()
        .map(|value| (value * 100.0) as i64)
        .map_err(|error| anyhow::anyhow!("invalid amount: {error}"))?;

    let mut create = stripe::CreateCheckoutSession::new();
    create.mode = Some(stripe::CheckoutSessionMode::Payment);
    create.success_url = Some(&params.success_url);
    create.cancel_url = Some(&params.cancel_url);

    if let Some(ref email) = params.customer_email {
        create.customer_email = Some(email);
    }

    create.line_items = Some(vec![stripe::CreateCheckoutSessionLineItems {
        price_data: Some(stripe::CreateCheckoutSessionLineItemsPriceData {
            currency: params
                .currency
                .to_lowercase()
                .parse()
                .unwrap_or(stripe::Currency::USD),
            unit_amount: Some(amount),
            product_data: Some(stripe::CreateCheckoutSessionLineItemsPriceDataProductData {
                name: format!("Invoice {}", params.invoice_number),
                ..Default::default()
            }),
            ..Default::default()
        }),
        quantity: Some(1),
        ..Default::default()
    }]);

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("invoiceId".to_string(), params.invoice_id.clone());
    create.metadata = Some(metadata);

    let session: stripe::CheckoutSession = stripe::CheckoutSession::create(client, create)
        .await
        .map_err(|error| anyhow::anyhow!("stripe checkout failed: {error}"))?;

    Ok(session.url)
}

#[cfg(not(feature = "stripe"))]
pub async fn create_stripe_checkout_session(_params: CheckoutParams) -> Result<Option<String>> {
    Err(crate::error::BillingError::ProviderNotConfigured(
        "stripe".to_string(),
    ))
}

#[cfg(feature = "stripe")]
pub fn verify_stripe_webhook(
    payload: &str,
    sig_header: &str,
    secret: &str,
) -> Result<serde_json::Value> {
    let event = stripe::Webhook::construct_event(payload, sig_header, secret)
        .map_err(|error| anyhow::anyhow!("stripe signature verification failed: {error}"))?;

    serde_json::to_value(&event)
        .map_err(|error| anyhow::anyhow!("stripe event serialization failed: {error}").into())
}

#[cfg(not(feature = "stripe"))]
pub fn verify_stripe_webhook(
    _payload: &str,
    _sig_header: &str,
    _secret: &str,
) -> Result<serde_json::Value> {
    Err(crate::error::BillingError::ProviderNotConfigured(
        "stripe".to_string(),
    ))
}

#[cfg(feature = "stripe")]
pub async fn create_stripe_refund(
    client: &StripeClient,
    payment_intent_id: &str,
    amount_cents: i64,
) -> Result<String> {
    let mut params = stripe::CreateRefund::new();
    params.payment_intent = Some(
        payment_intent_id
            .parse()
            .map_err(|error| anyhow::anyhow!("invalid payment intent id: {error}"))?,
    );
    params.amount = Some(amount_cents);

    let refund: stripe::Refund = stripe::Refund::create(client, params)
        .await
        .map_err(|error| anyhow::anyhow!("stripe refund failed: {error}"))?;

    Ok(refund.id.to_string())
}

pub async fn create_xendit_invoice<S: ProviderSettingsRepository + ?Sized>(
    http: &Client,
    settings: &S,
    params: XenditInvoiceParams,
) -> Result<XenditInvoiceResult> {
    let secret_key = settings
        .get_setting("xendit_secret_key")
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

    let response = http
        .post("https://api.xendit.co/v2/invoices")
        .basic_auth(&secret_key, None::<&str>)
        .json(&body)
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("xendit request failed: {error}"))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("xendit invoice creation failed: {text}").into());
    }

    let result: CreateInvoiceResponse = response
        .json()
        .await
        .map_err(|error| anyhow::anyhow!("xendit response parse failed: {error}"))?;

    Ok(XenditInvoiceResult {
        invoice_url: result.invoice_url,
        xendit_invoice_id: result.id,
    })
}

pub async fn create_xendit_refund<S: ProviderSettingsRepository + ?Sized>(
    http: &Client,
    settings: &S,
    payment_id: &str,
    amount: Decimal,
    currency: &str,
    reason: Option<&str>,
) -> Result<String> {
    let secret_key = settings
        .get_setting("xendit_secret_key")
        .ok_or_else(|| crate::error::BillingError::ProviderNotConfigured("xendit".to_string()))?;

    let body = serde_json::json!({
        "payment_request_id": payment_id,
        "amount": amount.to_string().parse::<f64>().unwrap_or(0.0),
        "currency": currency.to_uppercase(),
        "reason": reason.unwrap_or("REQUESTED_BY_CUSTOMER"),
    });

    let response = http
        .post("https://api.xendit.co/refunds")
        .basic_auth(&secret_key, None::<&str>)
        .json(&body)
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("xendit refund request failed: {error}"))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("xendit refund failed: {text}").into());
    }

    let result: serde_json::Value = response
        .json()
        .await
        .map_err(|error| anyhow::anyhow!("xendit refund response parse failed: {error}"))?;

    Ok(result["id"].as_str().unwrap_or_default().to_string())
}

pub fn verify_xendit_webhook(callback_token: Option<&str>, expected_token: &str) -> bool {
    match callback_token {
        Some(token) => {
            let provided = token.as_bytes();
            let expected = expected_token.as_bytes();
            if provided.len() != expected.len() {
                return false;
            }
            provided.ct_eq(expected).into()
        }
        None => false,
    }
}

pub async fn create_ls_checkout<S: ProviderSettingsRepository + ?Sized>(
    http: &Client,
    settings: &S,
    params: LsCheckoutParams,
) -> Result<LsCheckoutResult> {
    let api_key = settings
        .get_setting("lemonsqueezy_api_key")
        .ok_or_else(|| {
            crate::error::BillingError::ProviderNotConfigured("lemonsqueezy".to_string())
        })?;
    let store_id = settings
        .get_setting("lemonsqueezy_store_id")
        .ok_or_else(|| {
            crate::error::BillingError::ProviderNotConfigured("lemonsqueezy".to_string())
        })?;

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

    let response = http
        .post(format!("{LS_API_BASE}/checkouts"))
        .header("Accept", "application/vnd.api+json")
        .header("Content-Type", "application/vnd.api+json")
        .bearer_auth(&api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| anyhow::anyhow!("lemonsqueezy request failed: {error}"))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("lemonsqueezy checkout failed: {text}").into());
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|error| anyhow::anyhow!("lemonsqueezy response parse failed: {error}"))?;

    let checkout_url = json["data"]["attributes"]["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let checkout_id = json["data"]["id"].as_str().unwrap_or_default().to_string();

    Ok(LsCheckoutResult {
        checkout_url,
        checkout_id,
    })
}

pub fn verify_ls_webhook(raw_body: &str, signature: Option<&str>, secret: &str) -> bool {
    let signature = match signature {
        Some(value) => value,
        None => return false,
    };

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(value) => value,
        Err(_) => return false,
    };
    mac.update(raw_body.as_bytes());
    let digest = hex::encode(mac.finalize().into_bytes());

    let digest_bytes = digest.as_bytes();
    let provided_bytes = signature.as_bytes();
    if digest_bytes.len() != provided_bytes.len() {
        return false;
    }
    digest_bytes.ct_eq(provided_bytes).into()
}

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
