use super::repository::SubscriptionsRepository;
use super::schema::{
    ChangePlanRequest, CreateSubscriptionRequest, CreateSubscriptionV1Request, LifecycleRequest,
    UpdateSubscriptionRequest, UpdateSubscriptionV1Request,
};
use rust_decimal::Decimal;
use rustbill_core::db::models::{Subscription, SubscriptionStatus};
use rustbill_core::error::BillingError;

pub async fn list_admin<R: SubscriptionsRepository>(
    repo: &R,
    role_customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_admin(role_customer_id).await
}

pub async fn get_admin<R: SubscriptionsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_by_id(id).await
}

pub async fn create_admin<R: SubscriptionsRepository>(
    repo: &R,
    req: &CreateSubscriptionRequest,
) -> Result<serde_json::Value, BillingError> {
    if req.customer_id.trim().is_empty() {
        return Err(BillingError::bad_request("customerId is required"));
    }
    if req.plan_id.trim().is_empty() {
        return Err(BillingError::bad_request("planId is required"));
    }

    let metadata = req.merged_metadata()?;
    let row = repo.create_admin(req, metadata).await?;

    let created: Subscription =
        serde_json::from_value(row.clone()).map_err(|e| BillingError::Internal(e.into()))?;
    let mrr = repo
        .compute_subscription_mrr(&created.plan_id, created.quantity)
        .await?;

    if contributes_to_mrr(&created.status) {
        if let Err(err) = repo.emit_subscription_created_event(&created, mrr).await {
            tracing::warn!(error = %err, subscription_id = %created.id, "failed to emit mrr_expanded on create");
        }
    }

    Ok(row)
}

pub async fn update_admin<R: SubscriptionsRepository>(
    repo: &R,
    id: &str,
    req: &UpdateSubscriptionRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.find_active_subscription(id).await?;
    let metadata = req.merged_metadata_optional()?;

    let row = repo.update_admin(id, req, metadata).await?;
    let after = repo.find_active_subscription(id).await?;

    emit_mrr_change_events(repo, &before, &after, "subscription_update").await;

    Ok(row)
}

pub async fn delete_admin<R: SubscriptionsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.find_active_subscription(id).await?;

    let affected = repo.cancel_subscription(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("subscription", id));
    }

    let after = repo.find_active_subscription(id).await?;
    emit_mrr_change_events(repo, &before, &after, "subscription_delete").await;

    Ok(serde_json::json!({ "success": true }))
}

pub async fn lifecycle_admin<R: SubscriptionsRepository>(
    repo: &R,
    req: &LifecycleRequest,
) -> Result<serde_json::Value, BillingError> {
    let subscription_id = req.subscription_id.trim();
    if subscription_id.is_empty() {
        return Err(BillingError::bad_request("subscriptionId is required"));
    }

    let new_status = match req.action.as_str() {
        "pause" => "paused",
        "resume" => "active",
        "cancel" => "canceled",
        "renew" => "active",
        _ => {
            return Err(BillingError::bad_request(format!(
                "Unknown lifecycle action: {}",
                req.action
            )));
        }
    };

    let before = repo.find_active_subscription(subscription_id).await?;
    let row = repo
        .lifecycle_update_status(subscription_id, new_status)
        .await?;
    let after = repo.find_active_subscription(subscription_id).await?;

    emit_mrr_change_events(repo, &before, &after, "subscription_lifecycle").await;

    Ok(row)
}

pub async fn change_plan_admin<R: SubscriptionsRepository>(
    repo: &R,
    http_client: &reqwest::Client,
    id: &str,
    req: &ChangePlanRequest,
) -> Result<serde_json::Value, BillingError> {
    let before = repo.find_active_subscription(id).await?;

    let result = repo.change_plan_with_proration(id, req).await?;

    if !result.already_processed {
        emit_mrr_change_events(
            repo,
            &before,
            &result.subscription,
            "subscription_change_plan",
        )
        .await;

        let proration_net = result.proration_net.to_string();
        let _ = repo
            .emit_subscription_plan_changed_notification(
                http_client,
                id,
                &result.customer_id,
                &result.old_plan_name,
                &result.new_plan_name,
                &proration_net,
            )
            .await;
    }

    if result.already_processed {
        serde_json::to_value(result.invoice).map_err(|e| BillingError::Internal(e.into()))
    } else {
        serde_json::to_value(result.subscription).map_err(|e| BillingError::Internal(e.into()))
    }
}

pub async fn list_v1<R: SubscriptionsRepository>(
    repo: &R,
    status: Option<&str>,
    customer_id: Option<&str>,
) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list_v1(status, customer_id).await
}

pub async fn get_v1<R: SubscriptionsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get_by_id(id).await
}

pub async fn create_v1<R: SubscriptionsRepository>(
    repo: &R,
    req: &CreateSubscriptionV1Request,
) -> Result<serde_json::Value, BillingError> {
    repo.create_v1(req).await
}

pub async fn update_v1<R: SubscriptionsRepository>(
    repo: &R,
    id: &str,
    req: &UpdateSubscriptionV1Request,
) -> Result<serde_json::Value, BillingError> {
    repo.update_v1(id, req).await
}

pub async fn change_plan_v1<R: SubscriptionsRepository>(
    repo: &R,
    http_client: &reqwest::Client,
    id: &str,
    req: &ChangePlanRequest,
) -> Result<serde_json::Value, BillingError> {
    let result = repo.change_plan_with_proration(id, req).await?;

    if !result.already_processed {
        let proration_net = result.proration_net.to_string();
        let _ = repo
            .emit_subscription_plan_changed_notification(
                http_client,
                id,
                &result.customer_id,
                &result.old_plan_name,
                &result.new_plan_name,
                &proration_net,
            )
            .await;
    }

    if result.already_processed {
        serde_json::to_value(result.invoice).map_err(|e| BillingError::Internal(e.into()))
    } else {
        serde_json::to_value(result.subscription).map_err(|e| BillingError::Internal(e.into()))
    }
}

fn contributes_to_mrr(status: &SubscriptionStatus) -> bool {
    matches!(
        status,
        SubscriptionStatus::Active | SubscriptionStatus::PastDue
    )
}

async fn emit_mrr_change_events<R: SubscriptionsRepository>(
    repo: &R,
    before: &Subscription,
    after: &Subscription,
    trigger: &str,
) {
    let old_mrr = match repo
        .compute_subscription_mrr(&before.plan_id, before.quantity)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(error = %err, subscription_id = %before.id, "failed to compute old subscription MRR");
            return;
        }
    };

    let new_mrr = match repo
        .compute_subscription_mrr(&after.plan_id, after.quantity)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            tracing::warn!(error = %err, subscription_id = %after.id, "failed to compute new subscription MRR");
            return;
        }
    };

    let old_effective = if contributes_to_mrr(&before.status) {
        old_mrr
    } else {
        Decimal::ZERO
    };
    let new_effective = if contributes_to_mrr(&after.status) {
        new_mrr
    } else {
        Decimal::ZERO
    };

    let delta = new_effective - old_effective;
    if delta > Decimal::ZERO {
        if let Err(err) = repo
            .emit_mrr_delta_event(after, before, trigger, "mrr_expanded", delta)
            .await
        {
            tracing::warn!(error = %err, subscription_id = %after.id, "failed to emit mrr_expanded");
        }
    } else if delta < Decimal::ZERO {
        let magnitude = delta.abs();
        let event_type = if new_effective == Decimal::ZERO
            && old_effective > Decimal::ZERO
            && matches!(after.status, SubscriptionStatus::Canceled)
        {
            "mrr_churned"
        } else {
            "mrr_contracted"
        };

        if let Err(err) = repo
            .emit_mrr_delta_event(after, before, trigger, event_type, magnitude)
            .await
        {
            tracing::warn!(error = %err, subscription_id = %after.id, "failed to emit {event_type}");
        }
    }
}
