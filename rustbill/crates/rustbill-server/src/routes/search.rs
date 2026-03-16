use axum::{extract::{Query, State}, routing::get, Json, Router};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(search))
}

#[derive(serde::Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<i64>,
}

async fn search(
    State(state): State<SharedState>,
    _user: AdminUser,
    Query(params): Query<SearchParams>,
) -> ApiResult<Json<serde_json::Value>> {
    let query = format!("%{}%", params.q);
    let limit = params.limit.unwrap_or(20).min(100);

    let customers = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT jsonb_build_object('type', 'customer', 'data', to_jsonb(c))
           FROM customers c
           WHERE c.name ILIKE $1 OR c.email ILIKE $1
           LIMIT $2"#,
    )
    .bind(&query)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let products = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT jsonb_build_object('type', 'product', 'data', to_jsonb(p))
           FROM products p
           WHERE p.name ILIKE $1
           LIMIT $2"#,
    )
    .bind(&query)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let licenses = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT jsonb_build_object('type', 'license', 'data', to_jsonb(l))
           FROM licenses l
           WHERE l.key ILIKE $1
           LIMIT $2"#,
    )
    .bind(&query)
    .bind(limit)
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    let mut results = Vec::new();
    results.extend(customers);
    results.extend(products);
    results.extend(licenses);

    Ok(Json(serde_json::json!({
        "query": params.q,
        "results": results,
        "total": results.len(),
    })))
}
