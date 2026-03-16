use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{extract::State, routing::get, Json, Router};

pub fn router() -> Router<SharedState> {
    Router::new().route(
        "/payment-providers",
        get(get_providers).put(update_providers),
    )
}

async fn get_providers(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rows = sqlx::query_scalar::<_, serde_json::Value>(
        r#"SELECT jsonb_build_object(
             'provider', ps.provider,
             'enabled', ps.enabled,
             'mode', ps.mode,
             'updatedAt', ps.updated_at
           )
           FROM payment_provider_settings ps
           ORDER BY ps.provider"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "providers": rows,
    })))
}

async fn update_providers(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let provider = body["provider"].as_str().unwrap_or_default();
    let enabled = body["enabled"].as_bool();
    let mode = body["mode"].as_str();
    let credentials = body.get("credentials");

    let row = sqlx::query_scalar::<_, serde_json::Value>(
        r#"INSERT INTO payment_provider_settings (id, provider, enabled, mode, credentials_enc, updated_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, now())
           ON CONFLICT (provider) DO UPDATE SET
             enabled = COALESCE($2, payment_provider_settings.enabled),
             mode = COALESCE($3, payment_provider_settings.mode),
             credentials_enc = COALESCE($4, payment_provider_settings.credentials_enc),
             updated_at = now()
           RETURNING jsonb_build_object(
             'provider', payment_provider_settings.provider,
             'enabled', payment_provider_settings.enabled,
             'mode', payment_provider_settings.mode,
             'updatedAt', payment_provider_settings.updated_at
           )"#,
    )
    .bind(provider)
    .bind(enabled)
    .bind(mode)
    .bind(credentials)
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // Invalidate provider settings cache
    state.provider_cache.clear_cache().await;

    Ok(Json(row))
}
