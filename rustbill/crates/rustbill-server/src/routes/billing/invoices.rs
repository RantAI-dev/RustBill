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
        r#"INSERT INTO invoices (id, invoice_number, customer_id, subscription_id, status, currency, subtotal, tax, total, due_at, notes, created_at, updated_at)
           VALUES (gen_random_uuid()::text, 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0'), $1, $2, 'draft', COALESCE($3, 'USD'), $4, $5, $6, $7::timestamp, $8, now(), now())
           RETURNING to_jsonb(invoices.*)"#,
    )
    .bind(body["customerId"].as_str())
    .bind(body["subscriptionId"].as_str())
    .bind(body["currency"].as_str())
    .bind(body["subtotal"].as_f64().unwrap_or(0.0))
    .bind(body["tax"].as_f64().unwrap_or(0.0))
    .bind(body["total"].as_f64().unwrap_or(0.0))
    .bind(body["dueAt"].as_str())
    .bind(body["notes"].as_str())
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
             status = COALESCE($2::invoice_status, status),
             notes = COALESCE($3, notes),
             due_at = COALESCE($4::timestamp, due_at),
             version = version + 1,
             updated_at = now()
           WHERE id = $1 AND deleted_at IS NULL
           RETURNING to_jsonb(invoices.*)"#,
    )
    .bind(&id)
    .bind(body["status"].as_str())
    .bind(body["notes"].as_str())
    .bind(body["dueAt"].as_str())
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
        sqlx::query("UPDATE invoices SET status = 'void', deleted_at = now(), version = version + 1, updated_at = now() WHERE id = $1 AND deleted_at IS NULL")
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
        "SELECT to_jsonb(li) FROM invoice_items li WHERE li.invoice_id = $1 ORDER BY li.id",
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
        r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6::timestamp, $7::timestamp)
           RETURNING to_jsonb(invoice_items.*)"#,
    )
    .bind(&id)
    .bind(body["description"].as_str())
    .bind(body["quantity"].as_f64().unwrap_or(1.0))
    .bind(body["unitPrice"].as_f64().unwrap_or(0.0))
    .bind(body["amount"].as_f64().unwrap_or(0.0))
    .bind(body["periodStart"].as_str())
    .bind(body["periodEnd"].as_str())
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
