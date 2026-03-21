//! Xendit payment compatibility layer.

use crate::error::Result;
use crate::settings::provider_settings::ProviderSettings;
use reqwest::Client;
use rust_decimal::Decimal;

pub use super::schema::{XenditInvoiceParams, XenditInvoiceResult};

pub async fn create_invoice(
    http: &Client,
    settings: &ProviderSettings,
    params: XenditInvoiceParams,
) -> Result<XenditInvoiceResult> {
    super::service::create_xendit_invoice(http, settings, params).await
}

pub async fn create_refund(
    http: &Client,
    settings: &ProviderSettings,
    payment_id: &str,
    amount: Decimal,
    currency: &str,
    reason: Option<&str>,
) -> Result<String> {
    super::service::create_xendit_refund(http, settings, payment_id, amount, currency, reason).await
}

pub fn verify_webhook(callback_token: Option<&str>, expected_token: &str) -> bool {
    super::service::verify_xendit_webhook(callback_token, expected_token)
}
