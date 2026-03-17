use super::ApiResult;
use crate::app::SharedState;
use crate::extractors::AdminUser;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let rows = sqlx::query_as::<_, (serde_json::Value,)>(
        "SELECT to_jsonb(c) FROM customers c ORDER BY c.created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(rows.into_iter().map(|r| r.0).collect()))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT to_jsonb(c) FROM customers c WHERE c.id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "customer".into(),
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
        r#"INSERT INTO customers (id, name, industry, tier, location, contact, email, phone, total_revenue, health_score, trend, last_contact, billing_email, billing_address, billing_city, billing_state, billing_zip, billing_country, tax_id, default_payment_method, stripe_customer_id, xendit_customer_id, created_at, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, 0, 50, 'stable', '', $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, now(), now())
           RETURNING to_jsonb(customers)"#,
    )
    .bind(body["name"].as_str().unwrap_or(""))
    .bind(body["industry"].as_str().unwrap_or(""))
    .bind(body["tier"].as_str().unwrap_or("standard"))
    .bind(body["location"].as_str().unwrap_or(""))
    .bind(body["contact"].as_str().unwrap_or(""))
    .bind(body["email"].as_str().unwrap_or(""))
    .bind(body["phone"].as_str().unwrap_or(""))
    .bind(body["billingEmail"].as_str())
    .bind(body["billingAddress"].as_str())
    .bind(body["billingCity"].as_str())
    .bind(body["billingState"].as_str())
    .bind(body["billingZip"].as_str())
    .bind(body["billingCountry"].as_str())
    .bind(body["taxId"].as_str())
    .bind(body["defaultPaymentMethod"].as_str())
    .bind(body["stripeCustomerId"].as_str())
    .bind(body["xenditCustomerId"].as_str())
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
        r#"UPDATE customers SET
             name = COALESCE($2, name),
             industry = COALESCE($3, industry),
             tier = COALESCE($4, tier),
             location = COALESCE($5, location),
             contact = COALESCE($6, contact),
             email = COALESCE($7, email),
             phone = COALESCE($8, phone),
             billing_email = COALESCE($9, billing_email),
             billing_address = COALESCE($10, billing_address),
             billing_city = COALESCE($11, billing_city),
             billing_state = COALESCE($12, billing_state),
             billing_zip = COALESCE($13, billing_zip),
             billing_country = COALESCE($14, billing_country),
             tax_id = COALESCE($15, tax_id),
             default_payment_method = COALESCE($16, default_payment_method),
             stripe_customer_id = COALESCE($17, stripe_customer_id),
             xendit_customer_id = COALESCE($18, xendit_customer_id),
             updated_at = now()
           WHERE id = $1
           RETURNING to_jsonb(customers)"#,
    )
    .bind(&id)
    .bind(body["name"].as_str())
    .bind(body["industry"].as_str())
    .bind(body["tier"].as_str())
    .bind(body["location"].as_str())
    .bind(body["contact"].as_str())
    .bind(body["email"].as_str())
    .bind(body["phone"].as_str())
    .bind(body["billingEmail"].as_str())
    .bind(body["billingAddress"].as_str())
    .bind(body["billingCity"].as_str())
    .bind(body["billingState"].as_str())
    .bind(body["billingZip"].as_str())
    .bind(body["billingCountry"].as_str())
    .bind(body["taxId"].as_str())
    .bind(body["defaultPaymentMethod"].as_str())
    .bind(body["stripeCustomerId"].as_str())
    .bind(body["xenditCustomerId"].as_str())
    .fetch_optional(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?
    .ok_or_else(|| rustbill_core::error::BillingError::NotFound {
        entity: "customer".into(),
        id: id.clone(),
    })?;

    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM customers WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    if result.rows_affected() == 0 {
        return Err(rustbill_core::error::BillingError::NotFound {
            entity: "customer".into(),
            id,
        }
        .into());
    }

    Ok(Json(serde_json::json!({ "success": true })))
}
