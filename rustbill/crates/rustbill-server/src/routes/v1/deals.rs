use crate::app::SharedState;
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
        .route("/{id}", get(get_one).put(update).delete(remove))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    product_type: Option<String>,
    deal_type: Option<String>,
}

async fn list(
    State(state): State<SharedState>,
    Query(params): Query<ListParams>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(d) FROM deals d
           WHERE ($1::text IS NULL OR d.product_type::text = $1)
             AND ($2::text IS NULL OR d.deal_type::text = $2)
           ORDER BY d.created_at DESC"#,
    )
    .bind(&params.product_type)
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
        r#"INSERT INTO deals (id, customer_id, company, contact, email, value, product_id, product_name, product_type, deal_type, date, license_key, notes, usage_metric_label, usage_metric_value, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, COALESCE($8::product_type, 'licensed'), COALESCE($9::deal_type, 'sale'), COALESCE($10, to_char(now(), 'YYYY-MM-DD')), $11, $12, $13, $14, now(), now())
           RETURNING to_jsonb(deals.*)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["company"].as_str().unwrap_or(""))
    .bind(body["contact"].as_str().unwrap_or(""))
    .bind(body["email"].as_str().unwrap_or(""))
    .bind(body["value"].as_f64().unwrap_or(0.0))
    .bind(body["productId"].as_str())
    .bind(body["productName"].as_str().unwrap_or(""))
    .bind(body["productType"].as_str())
    .bind(body["dealType"].as_str())
    .bind(body["date"].as_str())
    .bind(body["licenseKey"].as_str())
    .bind(body["notes"].as_str())
    .bind(body["usageMetricLabel"].as_str())
    .bind(body["usageMetricValue"].as_i64().map(|v| v as i32))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE deals SET
             customer_id = COALESCE($2, customer_id),
             company = COALESCE($3, company),
             contact = COALESCE($4, contact),
             email = COALESCE($5, email),
             value = COALESCE($6, value),
             product_id = COALESCE($7, product_id),
             product_name = COALESCE($8, product_name),
             product_type = COALESCE($9::product_type, product_type),
             deal_type = COALESCE($10::deal_type, deal_type),
             notes = COALESCE($11, notes),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(deals.*)"#,
    )
    .bind(&id)
    .bind(body["customerId"].as_str())
    .bind(body["company"].as_str())
    .bind(body["contact"].as_str())
    .bind(body["email"].as_str())
    .bind(body["value"].as_f64())
    .bind(body["productId"].as_str())
    .bind(body["productName"].as_str())
    .bind(body["productType"].as_str())
    .bind(body["dealType"].as_str())
    .bind(body["notes"].as_str())
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
