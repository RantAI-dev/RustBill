use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use rust_decimal::Decimal;
use rustbill_core::billing::credits;
use rustbill_core::db::models::CreditReason;
use serde::Deserialize;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/adjust", post(adjust))
        .route(
            "/adjust/{id}",
            put(update_adjustment).delete(delete_adjustment),
        )
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustUpdateRequest {
    amount: Decimal,
    description: Option<String>,
}

async fn adjust(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<AdjustRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let credit = credits::adjust(
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

async fn update_adjustment(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<AdjustUpdateRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    if body.amount <= Decimal::ZERO {
        return Err(
            rustbill_core::error::BillingError::bad_request("amount must be positive").into(),
        );
    }

    let existing: rustbill_core::db::models::CustomerCredit =
        sqlx::query_as("SELECT * FROM customer_credits WHERE id = $1")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?
            .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
                entity: "credit_adjustment".into(),
                id: id.clone(),
            })?;

    if existing.reason != CreditReason::Manual || existing.invoice_id.is_some() {
        return Err(rustbill_core::error::BillingError::bad_request(
            "only manual adjustments can be edited",
        )
        .into());
    }

    let delta = body.amount - existing.amount;
    if delta == Decimal::ZERO {
        return Ok(Json(serde_json::to_value(existing).map_err(|e| {
            rustbill_core::error::BillingError::Internal(e.into())
        })?));
    }

    let description = body
        .description
        .unwrap_or_else(|| format!("Adjusted entry {}", existing.id));

    let adjustment = credits::adjust(
        &state.db,
        &existing.customer_id,
        &existing.currency,
        delta,
        CreditReason::Manual,
        &description,
        existing.invoice_id.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::to_value(adjustment).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn delete_adjustment(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let existing: rustbill_core::db::models::CustomerCredit =
        sqlx::query_as("SELECT * FROM customer_credits WHERE id = $1")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?
            .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
                entity: "credit_adjustment".into(),
                id: id.clone(),
            })?;

    if existing.reason != CreditReason::Manual || existing.invoice_id.is_some() {
        return Err(rustbill_core::error::BillingError::bad_request(
            "only manual adjustments can be deleted",
        )
        .into());
    }

    credits::adjust(
        &state.db,
        &existing.customer_id,
        &existing.currency,
        -existing.amount,
        CreditReason::Manual,
        &format!("Reversal of entry {}", existing.id),
        existing.invoice_id.as_deref(),
    )
    .await?;

    Ok(Json(serde_json::json!({ "success": true })))
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
