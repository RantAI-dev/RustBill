pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::{Invoice, SavedPaymentMethod};
use crate::error::Result;
use repository::PgAutoChargeRepository;
use reqwest::Client;
use schema::AutoChargeContext;
pub use schema::ChargeResult;
use sqlx::PgPool;

pub async fn try_auto_charge(
    pool: &PgPool,
    invoice: &Invoice,
    payment_method: &SavedPaymentMethod,
    http_client: &Client,
) -> Result<ChargeResult> {
    let repo = PgAutoChargeRepository::new(pool, http_client);
    let context = AutoChargeContext {
        invoice: invoice.clone(),
        payment_method: payment_method.clone(),
    };

    service::try_auto_charge(&repo, &context).await
}
