//! Stripe payment compatibility layer.

use crate::error::Result;

pub use super::schema::CheckoutParams;

#[cfg(feature = "stripe")]
pub use super::service::StripeClient;

#[cfg(feature = "stripe")]
pub fn create_client(secret_key: &str) -> StripeClient {
    super::service::create_stripe_client(secret_key)
}

#[cfg(feature = "stripe")]
pub async fn create_checkout_session(
    client: &StripeClient,
    params: CheckoutParams,
) -> Result<Option<String>> {
    super::service::create_stripe_checkout_session(client, params).await
}

#[cfg(not(feature = "stripe"))]
pub async fn create_checkout_session(params: CheckoutParams) -> Result<Option<String>> {
    super::service::create_stripe_checkout_session(params).await
}

#[cfg(feature = "stripe")]
pub fn verify_webhook(payload: &str, sig_header: &str, secret: &str) -> Result<serde_json::Value> {
    super::service::verify_stripe_webhook(payload, sig_header, secret)
}

#[cfg(not(feature = "stripe"))]
pub fn verify_webhook(payload: &str, sig_header: &str, secret: &str) -> Result<serde_json::Value> {
    super::service::verify_stripe_webhook(payload, sig_header, secret)
}

#[cfg(feature = "stripe")]
pub async fn create_refund(
    client: &StripeClient,
    payment_intent_id: &str,
    amount_cents: i64,
) -> Result<String> {
    super::service::create_stripe_refund(client, payment_intent_id, amount_cents).await
}
