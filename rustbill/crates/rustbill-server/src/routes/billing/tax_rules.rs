use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use rustbill_core::billing::tax;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", axum::routing::put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rules = tax::list_tax_rules(&state.db).await?;
    Ok(Json(serde_json::to_value(rules).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaxRuleRequest {
    country: String,
    region: Option<String>,
    tax_name: String,
    rate: Decimal,
    inclusive: bool,
    product_category: Option<String>,
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let rule = tax::create_tax_rule(
        &state.db,
        &body.country,
        body.region.as_deref(),
        &body.tax_name,
        body.rate,
        body.inclusive,
        body.product_category.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::to_value(rule).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTaxRuleRequest {
    tax_name: String,
    rate: Decimal,
    inclusive: bool,
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let rule =
        tax::update_tax_rule(&state.db, &id, &body.tax_name, body.rate, body.inclusive).await?;
    Ok(Json(serde_json::to_value(rule).map_err(|e| {
        rustbill_core::error::BillingError::Internal(e.into())
    })?))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    tax::delete_tax_rule(&state.db, &id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
