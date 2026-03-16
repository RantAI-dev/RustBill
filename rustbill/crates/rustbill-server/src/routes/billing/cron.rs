use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use axum::{extract::State, routing::post, Json, Router};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/run", post(run_all))
        .route("/renew-subscriptions", post(renew_subscriptions))
        .route("/generate-invoices", post(generate_invoices))
        .route("/process-dunning", post(process_dunning))
        .route("/expire-licenses", post(expire_licenses))
}

async fn run_all(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!("Running all cron jobs");

    // 1. Full subscription lifecycle (trials, cancellations, renewals + invoices)
    let lifecycle_result = rustbill_core::billing::lifecycle::run_full_lifecycle(
        &state.db,
        state.email_sender.as_ref(),
        &state.http_client,
    )
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    // 2. Process dunning
    let dunning_config = rustbill_core::billing::dunning::DunningConfig::default();
    let dunning_processed =
        rustbill_core::billing::dunning::run_dunning(&state.db, &dunning_config)
            .await
            .map_err(rustbill_core::error::BillingError::from)?;

    // 3. Expire licenses
    let licenses_expired = sqlx::query_scalar::<_, i64>(
        r#"WITH expired AS (
             UPDATE licenses
             SET status = 'expired', updated_at = now()
             WHERE status = 'active'
               AND expires_at IS NOT NULL
               AND expires_at <= now()
             RETURNING id
           )
           SELECT COUNT(*) FROM expired"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "jobs": ["lifecycle", "dunning", "expire_licenses"],
        "lifecycle": {
            "trials_converted": lifecycle_result.trials_converted,
            "canceled": lifecycle_result.canceled,
            "renewed": lifecycle_result.renewed,
            "invoices_generated": lifecycle_result.invoices_generated,
            "errors": lifecycle_result.errors,
        },
        "dunning": {
            "processed": dunning_processed,
        },
        "licenses": {
            "expired": licenses_expired,
        },
    })))
}

async fn renew_subscriptions(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let result = rustbill_core::billing::lifecycle::run_full_lifecycle(
        &state.db,
        state.email_sender.as_ref(),
        &state.http_client,
    )
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "trials_converted": result.trials_converted,
        "canceled": result.canceled,
        "renewed": result.renewed,
        "invoices_generated": result.invoices_generated,
        "errors": result.errors,
    })))
}

async fn generate_invoices(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let generated = rustbill_core::billing::lifecycle::generate_pending_invoices(
        &state.db,
        state.email_sender.as_ref(),
        &state.http_client,
    )
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "generated": generated,
    })))
}

async fn process_dunning(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let config = rustbill_core::billing::dunning::DunningConfig::default();
    let processed = rustbill_core::billing::dunning::run_dunning(&state.db, &config)
        .await
        .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "processed": processed,
        "config": {
            "reminder_days": config.reminder_days,
            "warning_days": config.warning_days,
            "final_notice_days": config.final_notice_days,
            "suspension_days": config.suspension_days,
        },
    })))
}

async fn expire_licenses(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let result = sqlx::query_scalar::<_, i64>(
        r#"WITH expired AS (
             UPDATE licenses
             SET status = 'expired', updated_at = now()
             WHERE status = 'active'
               AND expires_at IS NOT NULL
               AND expires_at <= now()
             RETURNING id
           )
           SELECT COUNT(*) FROM expired"#,
    )
    .fetch_one(&state.db)
    .await
    .map_err(rustbill_core::error::BillingError::from)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "expired": result,
    })))
}
