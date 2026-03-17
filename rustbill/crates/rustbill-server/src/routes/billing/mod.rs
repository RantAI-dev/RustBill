pub mod checkout;
pub mod coupons;
pub mod credit_notes;
pub mod credits;
pub mod cron;
pub mod dunning;
pub mod events;
pub mod invoices;
pub mod payments;
pub mod plans;
pub mod refunds;
pub mod subscriptions;
pub mod tax_rules;
pub mod usage;
pub mod webhooks;

use crate::app::SharedState;
use axum::Router;

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
        .nest("/credits", credits::router())
        .nest("/tax-rules", tax_rules::router())
}
