use axum::{extract::{Path, Query, State}, routing::get, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(get_one))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    r#type: Option<String>,
    entity_id: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let limit = params.limit.unwrap_or(50).min(200);
    let offset = params.offset.unwrap_or(0);

    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(e) FROM billing_events e
           WHERE ($1::text IS NULL OR e.event_type = $1)
             AND ($2::text IS NULL OR e.entity_id = $2)
           ORDER BY e.created_at DESC
           LIMIT $3 OFFSET $4"#,
    )
    .bind(&params.r#type)
    .bind(&params.entity_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let total: (i64,) = sqlx::query_as(
        r#"SELECT COUNT(*) FROM billing_events e
           WHERE ($1::text IS NULL OR e.event_type = $1)
             AND ($2::text IS NULL OR e.entity_id = $2)"#,
    )
    .bind(&params.r#type)
    .bind(&params.entity_id)
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "data": rows,
        "total": total.0,
        "limit": limit,
        "offset": offset,
    })))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(e) FROM billing_events e WHERE e.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "event".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}
