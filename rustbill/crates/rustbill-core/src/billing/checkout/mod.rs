pub mod repository;
pub mod schema;
pub mod service;

use crate::error::Result;
use crate::settings::provider_settings::ProviderSettings;
use repository::PgCheckoutRepository;
use reqwest::Client;
use schema::{CheckoutRequest, CheckoutResult};
use sqlx::PgPool;

pub async fn create_checkout(
    pool: &PgPool,
    http: &Client,
    settings: &ProviderSettings,
    invoice_id: &str,
    provider: &str,
    origin: &str,
) -> Result<CheckoutResult> {
    let repo = PgCheckoutRepository::new(pool, http, settings);

    service::create_checkout(
        &repo,
        CheckoutRequest {
            invoice_id: invoice_id.to_string(),
            provider: provider.to_string(),
            origin: origin.to_string(),
        },
    )
    .await
}
