use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
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
use rustbill_core::db::models::UserRole;
use std::str::FromStr;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    user: SessionUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let role_customer_id = if user.0.role == UserRole::Customer {
        user.0.customer_id.clone()
    } else {
        None
    };

    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT to_jsonb(cn) FROM credit_notes cn
           WHERE cn.deleted_at IS NULL
             AND ($1::text IS NULL OR cn.customer_id = $1)
           ORDER BY cn.created_at DESC"#,
    )
    .bind(&role_customer_id)
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
        "SELECT to_jsonb(cn) FROM credit_notes cn WHERE cn.id = $1 AND cn.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "credit_note".into(),
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
        r#"INSERT INTO credit_notes (id, credit_note_number, invoice_id, customer_id, amount, reason, status, created_at, updated_at)
           VALUES (gen_random_uuid()::text, 'CN-' || LPAD((extract(epoch from now()) * 1000)::bigint::text, 14, '0'), $1, $2, $3, $4, 'draft', now(), now())
           RETURNING to_jsonb(credit_notes.*)"#,
    )
    .bind(body["invoiceId"].as_str())
    .bind(body["customerId"].as_str())
    .bind(amount)
    .bind(body["reason"].as_str())
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let credit_note_id = row["id"].as_str().unwrap_or_default();
    if let (Some(invoice_id), Some(customer_id), Some(reason)) = (
        row["invoice_id"].as_str(),
        row["customer_id"].as_str(),
        row["reason"].as_str(),
    ) {
        let amount_dec = Decimal::from_str(&amount.to_string()).unwrap_or(Decimal::ZERO);
        if let Err(err) = emit_sales_event(
            &state.db,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.created",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount_dec,
                amount_tax: Decimal::ZERO,
                amount_total: amount_dec,
                currency: "USD",
                customer_id: Some(customer_id),
                subscription_id: None,
                product_id: None,
                invoice_id: Some(invoice_id),
                payment_id: None,
                source_table: "credit_notes",
                source_id: credit_note_id,
                metadata: Some(serde_json::json!({
                    "status": "draft",
                    "reason": reason,
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id, "failed to emit credit_note.created");
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
        "SELECT to_jsonb(cn) FROM credit_notes cn WHERE cn.id = $1 AND cn.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "credit_note".into(),
        id: id.clone(),
    })?;

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"UPDATE credit_notes SET
             status = COALESCE($2::credit_note_status, status),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(credit_notes.*)"#,
    )
    .bind(&id)
    .bind(body["status"].as_str())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "credit_note".into(),
        id: id.clone(),
    })?;

    let before_status = before["status"].as_str().unwrap_or("draft");
    let after_status = row["status"].as_str().unwrap_or("draft");
    if before_status != "issued" && after_status == "issued" {
        let amount_dec = parse_decimal_value(&row["amount"]);
        let reason = row["reason"].as_str().unwrap_or_default();
        if let Err(err) = emit_sales_event(
            &state.db,
            NewSalesEvent {
                occurred_at: chrono::Utc::now(),
                event_type: "credit_note.issued",
                classification: SalesClassification::Adjustments,
                amount_subtotal: amount_dec,
                amount_tax: Decimal::ZERO,
                amount_total: amount_dec,
                currency: "USD",
                customer_id: row["customer_id"].as_str(),
                subscription_id: None,
                product_id: None,
                invoice_id: row["invoice_id"].as_str(),
                payment_id: None,
                source_table: "credit_notes",
                source_id: &id,
                metadata: Some(serde_json::json!({
                    "reason": reason,
                    "status": "issued",
                })),
            },
        )
        .await
        {
            tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.issued");
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
        "SELECT to_jsonb(cn) FROM credit_notes cn WHERE cn.id = $1 AND cn.deleted_at IS NULL",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "credit_note".into(),
        id: id.clone(),
    })?;

    let result = sqlx::query("UPDATE credit_notes SET deleted_at = NOW(), updated_at = NOW() WHERE id = $1 AND deleted_at IS NULL")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "credit_note".into(),
            id,
        }
        .into());
    }

    let prior_event: Option<(String, String)> = sqlx::query_as(
        r#"SELECT id, event_type
           FROM sales_events
           WHERE source_table = 'credit_notes'
             AND source_id = $1
             AND amount_total > 0
             AND event_type IN ('credit_note.created', 'credit_note.issued')
           ORDER BY occurred_at DESC, created_at DESC
           LIMIT 1"#,
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let mut metadata = serde_json::json!({
        "trigger": "credit_note_delete",
        "reason": "credit_note_removed",
    });
    if let Some((event_id, event_type)) = prior_event {
        metadata["reversal_of_event_id"] = serde_json::json!(event_id);
        metadata["reversal_of_event_type"] = serde_json::json!(event_type);
    }

    let amount_dec = parse_decimal_value(&before["amount"]);
    if let Err(err) = emit_sales_event(
        &state.db,
        NewSalesEvent {
            occurred_at: chrono::Utc::now(),
            event_type: "credit_note.reversal",
            classification: SalesClassification::Adjustments,
            amount_subtotal: -amount_dec,
            amount_tax: Decimal::ZERO,
            amount_total: -amount_dec,
            currency: "USD",
            customer_id: before["customer_id"].as_str(),
            subscription_id: None,
            product_id: None,
            invoice_id: before["invoice_id"].as_str(),
            payment_id: None,
            source_table: "credit_note_revisions",
            source_id: &id,
            metadata: Some(metadata),
        },
    )
    .await
    {
        tracing::warn!(error = %err, credit_note_id = %id, "failed to emit credit_note.reversal");
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
