pub mod stripe;
pub mod xendit;
pub mod lemonsqueezy;

use axum::Router;
use crate::app::SharedState;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/webhooks/stripe", stripe::router())
        .nest("/webhooks/xendit", xendit::router())
        .nest("/webhooks/lemonsqueezy", lemonsqueezy::router())
}
