use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::UserRole;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(record))
        .route("/{subscription_id}/summary", get(summary))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    subscription_id: Option<String>,
    metric_name: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.clone()
    } else {
        None
    };

    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(u) FROM usage_events u
           JOIN subscriptions s ON s.id = u.subscription_id
           WHERE ($1::text IS NULL OR u.subscription_id = $1)
             AND ($2::text IS NULL OR u.metric_name = $2)
             AND ($3::text IS NULL OR s.customer_id = $3)
           ORDER BY u.timestamp DESC"#,
    )
    .bind(&params.subscription_id)
    .bind(&params.metric_name)
    .bind(&role_customer_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn record(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO usage_events (id, subscription_id, metric_name, value, timestamp, idempotency_key, properties)
           VALUES (gen_random_uuid()::text, $1, $2, $3, COALESCE($4::timestamp, now()), $5, $6)
           RETURNING to_jsonb(usage_events)"#,
    )
    .bind(body["subscriptionId"].as_str())
    .bind(body["metricName"].as_str())
    .bind(body["value"].as_f64().unwrap_or(1.0))
    .bind(body["timestamp"].as_str())
    .bind(body["idempotencyKey"].as_str())
    .bind(body.get("properties").unwrap_or(&serde_json::json!({})))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn summary(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(subscription_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT jsonb_build_object(
             'metricName', u.metric_name,
             'totalValue', SUM(u.value),
             'recordCount', COUNT(*)
           )
           FROM usage_events u
           WHERE u.subscription_id = $1
           GROUP BY u.metric_name"#,
    )
    .bind(&subscription_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "subscriptionId": subscription_id,
        "metrics": rows,
    })))
}
