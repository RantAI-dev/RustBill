use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rust_decimal::Decimal;
use rustbill_core::analytics::sales_ledger::{
    emit_sales_event, NewSalesEvent, SalesClassification,
};
use std::str::FromStr;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(r) FROM refunds r WHERE r.deleted_at IS NULL ORDER BY r.created_at DESC",
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
        "SELECT to_jsonb(r) FROM refunds r WHERE r.id = $1 AND r.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "refund".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let amount = body["amount"].as_f64().unwrap_or(0.0);

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO refunds (id, payment_id, invoice_id, amount, reason, status, stripe_refund_id, created_at)
           VALUES (gen_random_uuid()::text, $1, COALESCE($2, (SELECT invoice_id FROM payments WHERE id = $1)), $3, $4, 'pending', $5, now())
           RETURNING to_jsonb(refunds.*)"#,
    )
    .bind(body["paymentId"].as_str())
    .bind(body["invoiceId"].as_str())
    .bind(amount)
    .bind(body["reason"].as_str())
    .bind(body["stripeRefundId"].as_str())
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let refund_id = row["id"].as_str().unwrap_or_default();
    if let (Some(invoice_id), Some(payment_id), Some(reason)) = (
        row["invoice_id"].as_str(),
        row["payment_id"].as_str(),
        row["reason"].as_str(),
    ) {
        let amount_dec = Decimal::from_str(&amount.to_string()).unwrap_or(Decimal::ZERO);
        if let Err(err) = emit_sales_event(
            &state.db,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.created",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount_dec,
                amount_tax: Decimal::ZERO,
                amount_total: amount_dec,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id: Some(invoice_id),
                payment_id: Some(payment_id),
                source_table: "refunds",
                source_id: refund_id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                    "status": "pending",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id, "failed to emit refund.created");
        }
    }

    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let before = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(r) FROM refunds r WHERE r.id = $1 AND r.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "refund".into(),
        id: id.clone(),
    })?;

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE refunds SET
              status = COALESCE($2::refund_status, status),
              processed_at = CASE WHEN $2::refund_status = 'completed' THEN now() ELSE processed_at END
           WHERE id = $1 AND deleted_at IS NULL
           RETURNING to_jsonb(refunds.*)"#,
    )
    .bind(&id)
    .bind(body["status"].as_str())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "refund".into(),
        id: id.clone(),
    })?;

    let before_status = before["status"].as_str().unwrap_or("pending");
    let after_status = row["status"].as_str().unwrap_or("pending");
    if before_status != "completed" && after_status == "completed" {
        let amount_dec = parse_decimal_value(&row["amount"]);
        let reason = row["reason"].as_str().unwrap_or_default();
        let invoice_id = row["invoice_id"].as_str();
        let payment_id = row["payment_id"].as_str();

        if let Err(err) = emit_sales_event(
            &state.db,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.completed",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount_dec,
                amount_tax: Decimal::ZERO,
                amount_total: amount_dec,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id,
                payment_id,
                source_table: "refunds",
                source_id: &id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.completed");
        }
    }

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let before = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(r) FROM refunds r WHERE r.id = $1 AND r.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "refund".into(),
        id: id.clone(),
    })?;

    let result =
        sqlx::query("UPDATE refunds SET deleted_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
            .bind(&id)
            .execute(&state.db)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "refund".into(),
            id,
        }
        .into());
    }

    if matches!(before["status"].as_str(), Some("completed")) {
        let amount_dec = parse_decimal_value(&before["amount"]);

        let completed_event: Option<(String, String)> = sqlx::query_as(
            r#"SELECT id, event_type
               FROM sales_events
               WHERE source_table = 'refunds'
                 AND source_id = $1
                 AND event_type = 'refund.completed'
               ORDER BY created_at DESC
               LIMIT 1"#,
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

        let mut metadata = serde_json::json!({
            "trigger": "refund_delete",
            "reason": "refund_removed",
        });
        if let Some((event_id, event_type)) = completed_event {
            metadata["reversal_of_event_id"] = serde_json::json!(event_id);
            metadata["reversal_of_event_type"] = serde_json::json!(event_type);
        }

        if let Err(err) = emit_sales_event(
            &state.db,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "refund.reversal",
                classification: SalesClassification::Adjustments,
                amount_subtotal: -amount_dec,
                amount_tax: Decimal::ZERO,
                amount_total: -amount_dec,
                currency: "USD",
                customer_id: None,
                subscription_id: None,
                product_id: None,
                invoice_id: before["invoice_id"].as_str(),
                payment_id: before["payment_id"].as_str(),
                source_table: "refund_revisions",
                source_id: &id,
                metadata: Some(metadata),
            },
        )
        .await
        {
            tracing::warn!(error = %err, refund_id = %id, "failed to emit refund.reversal");
        }
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

fn parse_decimal_value(val: &serde_json::Value) -> Decimal {
    match val {
        serde_json::Value::String(s) => Decimal::from_str(s).unwrap_or(Decimal::ZERO),
        serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).unwrap_or(Decimal::ZERO),
        _ => Decimal::ZERO,
    }
}
