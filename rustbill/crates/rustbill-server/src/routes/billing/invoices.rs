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
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use rustbill_core::db::models::{Invoice, InvoiceStatus};
use rustbill_core::error::BillingError;
use sqlx::PgPool;
use std::str::FromStr;

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
    let customer_id = body["customerId"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| BillingError::BadRequest("customerId is required".to_string()))?;

    let status = body["status"].as_str().unwrap_or("draft");
    let currency = body["currency"].as_str().unwrap_or("USD");
    let tax = body["tax"].as_f64().unwrap_or(0.0);

    let mut subtotal = body["subtotal"].as_f64().unwrap_or(0.0);
    if subtotal <= 0.0 {
        let items_total = body["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| {
                        let qty = item["quantity"].as_f64().unwrap_or(1.0);
                        let unit_price = item["unitPrice"]
                            .as_f64()
                            .or_else(|| item["unit_price"].as_f64())
                            .unwrap_or(0.0);
                        qty * unit_price
                    })
                    .sum::<f64>()
            })
            .unwrap_or(0.0);
        subtotal = items_total;
    }

    let total = body["total"].as_f64().unwrap_or(subtotal + tax);

    let invoice_number = generate_invoice_number(&state.db).await?;

    let mut tx = state
        .db
        .begin()
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO invoices (id, invoice_number, customer_id, subscription_id, status, currency, subtotal, tax, total, due_at, issued_at, notes, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4::invoice_status, $5, $6, $7, $8, $9::timestamp, $10::timestamp, $11, now(), now())
           RETURNING to_jsonb(invoices.*)"#,
    )
    .bind(invoice_number)
    .bind(customer_id)
    .bind(body["subscriptionId"].as_str())
    .bind(status)
    .bind(currency)
    .bind(subtotal)
    .bind(tax)
    .bind(total)
    .bind(body["dueAt"].as_str())
    .bind(body["issuedAt"].as_str())
    .bind(body["notes"].as_str())
    .fetch_one(&mut *tx)
    .await
    .map_err(|err| {
        tracing::error!(error = ?err, customer_id = %customer_id, "Invoice create insert failed");
        rustbill_core::error::BillingError::from(err)
    })?;

    let invoice_id = row["id"].as_str().unwrap_or_default().to_string();

    if let Some(items) = body["items"].as_array() {
        for item in items {
            let description = item["description"]
                .as_str()
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(description) = description else {
                continue;
            };

            let quantity = item["quantity"].as_f64().unwrap_or(1.0);
            let unit_price = item["unitPrice"]
                .as_f64()
                .or_else(|| item["unit_price"].as_f64())
                .unwrap_or(0.0);
            let amount = item["amount"].as_f64().unwrap_or(quantity * unit_price);

            sqlx::query(
                r#"INSERT INTO invoice_items (id, invoice_id, description, quantity, unit_price, amount, period_start, period_end)
                   VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, NULL, NULL)"#,
            )
            .bind(&invoice_id)
            .bind(description)
            .bind(quantity)
            .bind(unit_price)
            .bind(amount)
            .execute(&mut *tx)
            .await
            .map_err(|err| {
                tracing::error!(error = ?err, invoice_id = %invoice_id, "Invoice item insert failed");
                rustbill_core::error::BillingError::from(err)
            })?;
        }
    }

    tx.commit().await.map_err(|err| {
        tracing::error!(error = ?err, "Invoice create commit failed");
        rustbill_core::error::BillingError::from(err)
    })?;

    let subtotal_dec = Decimal::from_str(&subtotal.to_string()).unwrap_or(Decimal::ZERO);
    let tax_dec = Decimal::from_str(&tax.to_string()).unwrap_or(Decimal::ZERO);
    let total_dec = Decimal::from_str(&total.to_string()).unwrap_or(Decimal::ZERO);

    if let Err(err) = emit_sales_event(
        &state.db,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "invoice.created",
            classification: SalesClassification::Billings,
            amount_subtotal: subtotal_dec,
            amount_tax: tax_dec,
            amount_total: total_dec,
            currency,
            customer_id: Some(customer_id),
            subscription_id: body["subscriptionId"].as_str(),
            product_id: None,
            invoice_id: Some(&invoice_id),
            payment_id: None,
            source_table: "invoices",
            source_id: &invoice_id,
            metadata: Some(serde_json::json!({
                "status": status,
                "origin": "manual",
            })),
        },
    )
    .await
    {
        tracing::warn!(error = %err, invoice_id = %invoice_id, "failed to emit sales event invoice.created");
    }

    Ok((StatusCode::CREATED, Json(row)))
}

async fn generate_invoice_number(db: &PgPool) -> Result<String, BillingError> {
    let from_sequence = sqlx::query_scalar::<_, String>(
        "SELECT 'INV-' || LPAD(nextval('invoice_number_seq')::text, 8, '0')",
    )
    .fetch_one(db)
    .await;

    match from_sequence {
        Ok(value) => Ok(value),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("42P01") => {
            let next: i64 = sqlx::query_scalar(
                r#"
                SELECT COALESCE(MAX(NULLIF(regexp_replace(invoice_number, '[^0-9]', '', 'g'), '')::bigint), 0) + 1
                FROM invoices
                "#,
            )
            .fetch_one(db)
            .await
            .map_err(BillingError::from)?;

            Ok(format!("INV-{next:08}"))
        }
        Err(err) => Err(BillingError::from(err)),
    }
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let before =
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?
            .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
                entity: "invoice".into(),
                id: id.clone(),
            })?;

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

    let after =
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(&id)
            .fetch_one(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    if before.status != after.status && matches!(after.status, InvoiceStatus::Void) {
        emit_invoice_void_reversal(&state.db, &after, "invoice_update").await;
    }

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let invoice =
        sqlx::query_as::<_, Invoice>("SELECT * FROM invoices WHERE id = $1 AND deleted_at IS NULL")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?
            .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
                entity: "invoice".into(),
                id: id.clone(),
            })?;

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

    emit_invoice_void_reversal(&state.db, &invoice, "invoice_delete").await;

    Ok(Json(serde_json::json!({ "success": true })))
}

async fn emit_invoice_void_reversal(pool: &PgPool, invoice: &Invoice, trigger: &str) {
    let reversal_target: Option<(String, String, Decimal, Decimal, Decimal)> = sqlx::query_as(
        r#"
        SELECT id, event_type, amount_subtotal, amount_tax, amount_total
        FROM sales_events
        WHERE source_table = 'invoices'
          AND source_id = $1
          AND classification = 'billings'
          AND amount_total > 0
          AND event_type IN ('invoice.created', 'invoice.created_from_deal', 'invoice.issued')
        ORDER BY occurred_at DESC, created_at DESC
        LIMIT 1
        "#,
    )
    .bind(&invoice.id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((event_id, event_type, subtotal, tax, total)) = reversal_target {
        if let Err(err) = emit_sales_event(
            pool,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "invoice.reversal",
                classification: SalesClassification::Billings,
                amount_subtotal: -subtotal,
                amount_tax: -tax,
                amount_total: -total,
                currency: &invoice.currency,
                customer_id: Some(&invoice.customer_id),
                subscription_id: invoice.subscription_id.as_deref(),
                product_id: None,
                invoice_id: Some(&invoice.id),
                payment_id: None,
                source_table: "invoices",
                source_id: &invoice.id,
                metadata: Some(serde_json::json!({
                    "trigger": trigger,
                    "reason": "invoice_voided",
                    "reversal_of_event_id": event_id,
                    "reversal_of_event_type": event_type,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit invoice.reversal");
        }
    }

    if let Err(err) = emit_sales_event(
        pool,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "invoice.voided",
            classification: SalesClassification::Billings,
            amount_subtotal: Decimal::ZERO,
            amount_tax: Decimal::ZERO,
            amount_total: Decimal::ZERO,
            currency: &invoice.currency,
            customer_id: Some(&invoice.customer_id),
            subscription_id: invoice.subscription_id.as_deref(),
            product_id: None,
            invoice_id: Some(&invoice.id),
            payment_id: None,
            source_table: "invoices",
            source_id: &invoice.id,
            metadata: Some(serde_json::json!({
                "trigger": trigger,
            })),
        },
    )
    .await
    {
        tracing::warn!(error = %err, invoice_id = %invoice.id, "failed to emit invoice.voided");
    }
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
