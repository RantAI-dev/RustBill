pub mod plans;
pub mod subscriptions;
pub mod invoices;
pub mod payments;
pub mod checkout;
pub mod credit_notes;
pub mod coupons;
pub mod refunds;
pub mod usage;
pub mod dunning;
pub mod events;
pub mod webhooks;
pub mod cron;

use axum::Router;
use crate::app::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/plans", plans::router())
        .nest("/subscriptions", subscriptions::router())
        .nest("/invoices", invoices::router())
        .nest("/payments", payments::router())
        .nest("/checkout", checkout::router())
        .nest("/credit-notes", credit_notes::router())
        .nest("/coupons", coupons::router())
        .nest("/refunds", refunds::router())
        .nest("/usage", usage::router())
        .nest("/dunning", dunning::router())
        .nest("/events", events::router())
        .nest("/webhooks", webhooks::router())
        .nest("/cron", cron::router())
}
