//! Stripe payment integration: checkout sessions, refunds, webhook verification.
//!
//! Uses the `stripe` crate (async-stripe) when the "stripe" feature is enabled.
//! Falls back to stub implementations that return ProviderNotConfigured errors.

use crate::error::Result;
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct CheckoutParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub success_url: String,
    pub cancel_url: String,
}

// ---- Feature-gated Stripe SDK implementations ----

#[cfg(feature = "stripe")]
pub use stripe_impl::*;

#[cfg(feature = "stripe")]
mod stripe_impl {
    use super::*;
    pub use stripe::Client as StripeClient;

    /// Create a Stripe client from secret key.
    pub fn create_client(secret_key: &str) -> StripeClient {
        StripeClient::new(secret_key)
    }

    /// Create a Stripe checkout session for an invoice.
    pub async fn create_checkout_session(
        client: &StripeClient,
        params: CheckoutParams,
    ) -> Result<Option<String>> {
        let amount = params
            .total
            .to_string()
            .parse::<f64>()
            .map(|v| (v * 100.0) as i64)
            .map_err(|e| anyhow::anyhow!("invalid amount: {e}"))?;

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
            .map_err(|e| anyhow::anyhow!("stripe checkout failed: {e}"))?;

        Ok(session.url)
    }

    /// Verify a Stripe webhook signature.
    pub fn verify_webhook(
        payload: &str,
        sig_header: &str,
        secret: &str,
    ) -> Result<serde_json::Value> {
        let event = stripe::Webhook::construct_event(payload, sig_header, secret)
            .map_err(|e| anyhow::anyhow!("stripe signature verification failed: {e}"))?;

        serde_json::to_value(&event)
            .map_err(|e| anyhow::anyhow!("stripe event serialization failed: {e}").into())
    }

    /// Create a Stripe refund.
    pub async fn create_refund(
        client: &StripeClient,
        payment_intent_id: &str,
        amount_cents: i64,
    ) -> Result<String> {
        let mut params = stripe::CreateRefund::new();
        params.payment_intent = Some(payment_intent_id.parse().unwrap());
        params.amount = Some(amount_cents);

        let refund: stripe::Refund = stripe::Refund::create(client, params)
            .await
            .map_err(|e| anyhow::anyhow!("stripe refund failed: {e}"))?;

        Ok(refund.id.to_string())
    }
}

// ---- Stub implementations when stripe feature is disabled ----

#[cfg(not(feature = "stripe"))]
pub async fn create_checkout_session(_params: CheckoutParams) -> Result<Option<String>> {
    Err(crate::error::BillingError::ProviderNotConfigured(
        "stripe".to_string(),
    ))
}

#[cfg(not(feature = "stripe"))]
pub fn verify_webhook(
    _payload: &str,
    _sig_header: &str,
    _secret: &str,
) -> Result<serde_json::Value> {
    Err(crate::error::BillingError::ProviderNotConfigured(
        "stripe".to_string(),
    ))
}
