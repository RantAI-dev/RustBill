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
    metric: Option<String>,
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
        r#"SELECT to_jsonb(u) FROM usage_records u
           JOIN subscriptions s ON s.id = u.subscription_id
           WHERE ($1::text IS NULL OR u.subscription_id = $1)
             AND ($2::text IS NULL OR u.metric = $2)
             AND ($3::text IS NULL OR s.customer_id = $3)
           ORDER BY u.recorded_at DESC"#,
    )
    .bind(&params.subscription_id)
    .bind(&params.metric)
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
        r#"INSERT INTO usage_records (id, subscription_id, metric, quantity, recorded_at, metadata)
           VALUES (gen_random_uuid()::text, $1, $2, $3, COALESCE($4::timestamptz, now()), $5)
           RETURNING to_jsonb(usage_records)"#,
    )
    .bind(body["subscriptionId"].as_str())
    .bind(body["metric"].as_str())
    .bind(body["quantity"].as_i64().unwrap_or(1))
    .bind(body["recordedAt"].as_str())
    .bind(body.get("metadata").unwrap_or(&serde_json::json!({})))
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
             'metric', u.metric,
             'totalQuantity', SUM(u.quantity),
             'recordCount', COUNT(*)
           )
           FROM usage_records u
           WHERE u.subscription_id = $1
           GROUP BY u.metric"#,
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
