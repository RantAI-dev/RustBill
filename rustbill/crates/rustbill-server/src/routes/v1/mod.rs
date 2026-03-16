pub mod billing;
pub mod customers;
pub mod deals;
pub mod licenses;
pub mod products;

use crate::app::SharedState;
use axum::Router;

pub fn router() -> Router<SharedState> {
    Router::new()
        .nest("/products", products::router())
        .nest("/customers", customers::router())
        .nest("/deals", deals::router())
        .nest("/licenses", licenses::router())
        .nest("/billing", billing::router())
    // API key authentication is applied via middleware in the app router builder
}
