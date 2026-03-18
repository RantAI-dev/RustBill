use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use rust_decimal::Decimal;
use rustbill_core::billing::credits;
use rustbill_core::db::models::CreditReason;
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/adjust", post(adjust))
        .route("/{customer_id}", get(get_customer_credits))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustRequest {
    customer_id: String,
    currency: String,
    amount: Decimal,
    description: String,
}

async fn adjust(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<AdjustRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let credit = credits::deposit(
        &state.db,
        &body.customer_id,
        &body.currency,
        body.amount,
        CreditReason::Manual,
        &body.description,
        None,
    )
    .await?;
    Ok(Json(serde_json::to_value(credit).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

#[derive(Deserialize)]
struct CreditQuery {
    currency: Option<String>,
}

async fn get_customer_credits(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(customer_id): Path<String>,
    Query(query): Query<CreditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let balance = credits::get_balance(
        &state.db,
        &customer_id,
        query.currency.as_deref().unwrap_or("USD"),
    )
    .await?;
    let history = credits::list_credits(&state.db, &customer_id, query.currency.as_deref()).await?;

    Ok(Json(serde_json::json!({
        "balance": balance,
        "currency": query.currency.as_deref().unwrap_or("USD"),
        "history": history
    })))
}
