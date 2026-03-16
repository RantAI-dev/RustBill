use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/items", get(list_items).post(add_item))
        .route("/{id}/pdf", get(get_pdf))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(i) FROM invoices i ORDER BY i.created_at DESC",
    )
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
        "SELECT to_jsonb(i) FROM invoices i WHERE i.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "invoice".into(),
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
        r#"INSERT INTO invoices (id, customer_id, subscription_id, status, currency, subtotal, tax, total, due_date, metadata, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, 'draft', $3, $4, $5, $6, $7, $8, now(), now())
           RETURNING to_jsonb(invoices)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["subscriptionId"].as_str())
    .bind(body["currency"].as_str().unwrap_or("USD"))
    .bind(body["subtotal"].as_i64().unwrap_or(0))
    .bind(body["tax"].as_i64().unwrap_or(0))
    .bind(body["total"].as_i64().unwrap_or(0))
    .bind(body["dueDate"].as_str())
    .bind(body.get("metadata").unwrap_or(&serde_json::json!({})))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE invoices SET
             status = COALESCE($2, status),
             metadata = COALESCE($3, metadata),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(invoices)"#,
    )
    .bind(&id)
    .bind(body["status"].as_str())
    .bind(body.get("metadata"))
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "invoice".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result =
        sqlx::query("UPDATE invoices SET status = 'void', updated_at = now() WHERE id = $1")
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "invoice".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn list_items(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(li) FROM invoice_items li WHERE li.invoice_id = $1 ORDER BY li.created_at",
    )
    .bind(&id)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows))
}

async fn add_item(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, now())
           RETURNING to_jsonb(invoice_items)"#,
    )
    .bind(&id)
    .bind(body["description"].as_str())
    .bind(body["quantity"].as_i64().unwrap_or(1) as i32)
    .bind(body["unitPrice"].as_i64().unwrap_or(0))
    .bind(body["amount"].as_i64().unwrap_or(0))
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok((StatusCode::CREATED, Json(row)))
}

async fn get_pdf(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let pdf_bytes =
        rustbill_core::billing::invoice_pdf::generate_invoice_pdf(&state.db, &id).await?;

    // Fetch invoice number for the filename
    let invoice_number: Option<String> =
        sqlx::query_scalar("SELECT invoice_number FROM invoices WHERE id = $1")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    let filename = invoice_number
        .map(|n| format!("invoice-{n}.pdf"))
        .unwrap_or_else(|| format!("invoice-{id}.pdf"));

    let headers = [
        (header::CONTENT_TYPE, "application/pdf".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"{filename}\""),
        ),
    ];

    Ok((headers, pdf_bytes))
}
