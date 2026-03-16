use axum::{extract::{Path, Query, State}, http::StatusCode, routing::get, Json, Router};
use crate::app::SharedState;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    r#type: Option<String>,
    deal_type: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(d) FROM deals d
           WHERE ($1::text IS NULL OR d.type = $1)
             AND ($2::text IS NULL OR d.deal_type = $2)
           ORDER BY d.created_at DESC"#,
    )
    .bind(&params.r#type)
    .bind(&params.deal_type)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(d) FROM deals d WHERE d.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "deal".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO deals (id, name, type, deal_type, product_id, customer_id, metadata, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, now(), now())
           RETURNING to_jsonb(deals)"#,
    )
    .bind(body["name"].as_str())
    .bind(body["type"].as_str())
    .bind(body["dealType"].as_str())
    .bind(body["productId"].as_str())
    .bind(body["customerId"].as_str())
    .bind(body.get("metadata").unwrap_or(&serde_json::json!({})))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Auto-generate license for licensed product deals
    if body["type"].as_str() == Some("licensed") || body["dealType"].as_str() == Some("licensed") {
        if let (Some(product_id), Some(customer_id)) =
            (body["productId"].as_str(), body["customerId"].as_str())
        {
            let deal_id = row["id"].as_str().unwrap_or_default();
            let key = format!(
                "LIC-{}-{}",
                &deal_id[..8.min(deal_id.len())],
                uuid::Uuid::new_v4().to_string()[..8].to_uppercase()
            );

            let _ = sqlx::query(
                r#"INSERT INTO licenses (id, key, product_id, customer_id, deal_id, status, created_at, updated_at)
                   VALUES (gen_random_uuid()::text, $1, $2, $3, $4, 'active', now(), now())"#,
            )
            .bind(&key)
            .bind(product_id)
            .bind(customer_id)
            .bind(deal_id)
            .execute(&state.db)
            .await;
        }
    }

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE deals SET
             name = COALESCE($2, name),
             type = COALESCE($3, type),
             deal_type = COALESCE($4, deal_type),
             metadata = COALESCE($5, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(deals)"#,
    )
    .bind(&id)
    .bind(body["name"].as_str())
    .bind(body["type"].as_str())
    .bind(body["dealType"].as_str())
    .bind(body.get("metadata"))
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "deal".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM deals WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "deal".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
