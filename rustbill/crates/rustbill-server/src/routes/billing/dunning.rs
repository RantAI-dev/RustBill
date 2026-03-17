use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    invoice_id: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(d) FROM dunning_log d
           WHERE ($1::text IS NULL OR d.invoice_id = $1)
           ORDER BY d.created_at DESC"#,
    )
    .bind(&params.invoice_id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(d) FROM dunning_log d WHERE d.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "dunning_log".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO dunning_log (id, invoice_id, subscription_id, step, scheduled_at, executed_at, notes, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, COALESCE($4::timestamp, now()), $5::timestamp, $6, now())
           RETURNING to_jsonb(dunning_log)"#,
    )
    .bind(body["invoiceId"].as_str())
    .bind(body["subscriptionId"].as_str())
    .bind(body["step"].as_str().unwrap_or("reminder"))
    .bind(body["scheduledAt"].as_str())
    .bind(body["executedAt"].as_str())
    .bind(body["notes"].as_str())
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}
