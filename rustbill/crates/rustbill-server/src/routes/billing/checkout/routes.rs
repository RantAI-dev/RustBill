use super::repository::SqlxCheckoutRepository;
use super::schema::{CheckoutQuery, CheckoutResponse};
use super::service;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new().route("/", get(get_checkout_url))
}

fn public_origin(state: &SharedState) -> String {
    std::env::var("PUBLIC_URL").unwrap_or_else(|_| {
        format!(
            "http://{}:{}",
            state.config.server.host, state.config.server.port
        )
    })
}

async fn get_checkout_url(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(query): Query<CheckoutQuery>,
) -> ApiResult<Json<CheckoutResponse>> {
    let repo = SqlxCheckoutRepository::new(state.clone());
    let result = service::get_checkout(&repo, &query, &public_origin(&state)).await?;

    Ok(Json(CheckoutResponse {
        invoice_id: result.invoice_id,
        provider: result.provider,
        checkout_url: result.checkout_url,
    }))
}
