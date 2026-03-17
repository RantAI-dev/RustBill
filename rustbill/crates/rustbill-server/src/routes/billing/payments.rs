use crate::app::SharedState;
use crate::extractors::{AdminUser, SessionUser};
use crate::routes::ApiResult;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use rustbill_core::db::models::UserRole;

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
        r#"SELECT to_jsonb(p) FROM payments p
           JOIN invoices i ON i.id = p.invoice_id
           WHERE ($1::text IS NULL OR i.customer_id = $1)
           ORDER BY p.created_at DESC"#,
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
        "SELECT to_jsonb(p) FROM payments p WHERE p.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "payment".into(),
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
        r#"INSERT INTO payments (id, invoice_id, amount, method, reference, paid_at, notes, stripe_payment_intent_id, xendit_payment_id, lemonsqueezy_order_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, COALESCE($3::payment_method, 'manual'), $4, COALESCE($5::timestamp, now()), $6, $7, $8, $9, now())
           RETURNING to_jsonb(payments.*)"#,
    )
    .bind(body["invoiceId"].as_str())
    .bind(body["amount"].as_f64().unwrap_or(0.0))
    .bind(body["method"].as_str())
    .bind(body["reference"].as_str())
    .bind(body["paidAt"].as_str())
    .bind(body["notes"].as_str())
    .bind(body["stripePaymentIntentId"].as_str())
    .bind(body["xenditPaymentId"].as_str())
    .bind(body["lemonsqueezyOrderId"].as_str())
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
        r#"UPDATE payments SET
             reference = COALESCE($2, reference),
             notes = COALESCE($3, notes)
           WHERE id = $1
           RETURNING to_jsonb(payments.*)"#,
    )
    .bind(&id)
    .bind(body["reference"].as_str())
    .bind(body["notes"].as_str())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "payment".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM payments WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "payment".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
