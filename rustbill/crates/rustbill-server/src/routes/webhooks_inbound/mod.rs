pub mod lemonsqueezy;
pub mod stripe;
pub mod xendit;

use crate::app::SharedState;
use axum::Router;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/webhooks/stripe", stripe::router())
        .nest("/webhooks/xendit", xendit::router())
        .nest("/webhooks/lemonsqueezy", lemonsqueezy::router())
}
