use super::email::EmailSender;
use super::repository::NotificationsRepository;
use super::schema::{EmitBillingEventRequest, WebhookDispatchPayload, WebhookEndpoint};
use super::templates;
use crate::db::models::BillingEventType;
use crate::error::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::Duration;

type HmacSha256 = Hmac<Sha256>;

pub async fn emit_billing_event<R: NotificationsRepository + Clone + Send + Sync + 'static>(
    repo: &R,
    http: &reqwest::Client,
    req: EmitBillingEventRequest,
) -> Result<String> {
    let event_id = repo.insert_billing_event(&req).await?;
    let endpoints = repo.list_active_webhook_endpoints().await?;

    let event_type_str = serde_json::to_value(&req.event_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    for endpoint in endpoints {
        let subscribed = endpoint.events.as_array().is_some_and(|events| {
            events
                .iter()
                .any(|event| event.as_str() == Some(&event_type_str) || event.as_str() == Some("*"))
        });

        if !subscribed {
            continue;
        }

        let payload = WebhookDispatchPayload {
            event: event_type_str.clone(),
            data: req.data.clone(),
            resource_type: req.resource_type.clone(),
            resource_id: req.resource_id.clone(),
            customer_id: req.customer_id.clone(),
        };

        let repo = repo.clone();
        let http = http.clone();
        let event_id = event_id.clone();
        tokio::spawn(async move {
            dispatch_webhook(&repo, &http, &endpoint, &event_id, &payload.as_json()).await;
        });
    }

    Ok(event_id)
}

async fn dispatch_webhook<R: NotificationsRepository + ?Sized>(
    repo: &R,
    http: &reqwest::Client,
    endpoint: &WebhookEndpoint,
    event_id: &str,
    payload: &serde_json::Value,
) {
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    let signature = match HmacSha256::new_from_slice(endpoint.secret.as_bytes()) {
        Ok(mut mac) => {
            mac.update(payload_str.as_bytes());
            hex::encode(mac.finalize().into_bytes())
        }
        Err(err) => {
            tracing::warn!(
                endpoint = %endpoint.url,
                error = %err,
                "Skipping webhook dispatch because HMAC key initialization failed"
            );
            return;
        }
    };

    let delivery_id = repo
        .insert_webhook_delivery(&endpoint.id, event_id, payload)
        .await
        .ok()
        .flatten();

    let backoff = [1, 4, 16];
    for (attempt, delay_secs) in backoff.iter().enumerate() {
        let result = http
            .post(&endpoint.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &signature)
            .header("X-Webhook-Event", payload["event"].as_str().unwrap_or(""))
            .header("X-Webhook-Id", event_id)
            .timeout(Duration::from_secs(10))
            .body(payload_str.clone())
            .send()
            .await;

        match result {
            Ok(resp) => {
                let status = resp.status().as_u16() as i32;
                let body = resp.text().await.unwrap_or_default();

                if let Some(ref delivery_id) = delivery_id {
                    if let Err(err) = repo
                        .update_webhook_delivery(
                            delivery_id,
                            status,
                            &body[..body.len().min(1000)],
                            (attempt + 1) as i32,
                        )
                        .await
                    {
                        tracing::warn!(
                            endpoint = %endpoint.url,
                            error = %err,
                            "Failed to update webhook delivery row"
                        );
                    }
                }

                if (200..300).contains(&(status as u16))
                    || ((400..500).contains(&(status as u16)) && status != 429)
                {
                    return;
                }
            }
            Err(error) => {
                tracing::warn!(
                    endpoint = %endpoint.url,
                    attempt = attempt + 1,
                    error = %error,
                    "Webhook delivery failed"
                );
            }
        }

        if attempt < backoff.len() - 1 {
            tokio::time::sleep(Duration::from_secs(*delay_secs)).await;
        }
    }
}

pub async fn notify_invoice_created<R: NotificationsRepository + ?Sized>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) {
    notify_with_customer_email(repo, email_sender, customer_id, |name| {
        templates::invoice_created(name, invoice_number, total, currency)
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn notify_invoice_issued<R: NotificationsRepository + ?Sized>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    due_date: &str,
) {
    notify_with_customer_email(repo, email_sender, customer_id, |name| {
        templates::invoice_issued(name, invoice_number, total, currency, due_date)
    })
    .await;
}

pub async fn notify_payment_received<R: NotificationsRepository + ?Sized>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    amount: &str,
    method: &str,
) {
    notify_with_customer_email(repo, email_sender, customer_id, |name| {
        templates::payment_received(name, amount, method)
    })
    .await;
}

pub async fn notify_invoice_paid<R: NotificationsRepository + ?Sized>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
) {
    notify_with_customer_email(repo, email_sender, customer_id, |name| {
        templates::invoice_paid(name, invoice_number, total, currency)
    })
    .await;
}

#[allow(clippy::too_many_arguments)]
pub async fn notify_subscription_renewed<R: NotificationsRepository + ?Sized>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    plan_name: &str,
    invoice_number: &str,
    total: &str,
    currency: &str,
    next_period_end: &str,
) {
    notify_with_customer_email(repo, email_sender, customer_id, |name| {
        templates::subscription_renewed(
            name,
            plan_name,
            invoice_number,
            total,
            currency,
            next_period_end,
        )
    })
    .await;
}

async fn notify_with_customer_email<R, F>(
    repo: &R,
    email_sender: Option<&EmailSender>,
    customer_id: &str,
    template: F,
) where
    R: NotificationsRepository + ?Sized,
    F: FnOnce(&str) -> (String, String),
{
    let Some(sender) = email_sender else {
        tracing::warn!("Email not configured, skipping notification");
        return;
    };

    let contact = match repo.find_customer_contact(customer_id).await {
        Ok(contact) => contact,
        Err(error) => {
            tracing::warn!(
                customer_id,
                error = %error,
                "Failed to look up customer contact for notification"
            );
            None
        }
    };

    let Some(contact) = contact else {
        tracing::warn!(
            customer_id,
            "Could not find customer email for billing notification"
        );
        return;
    };

    let (subject, html) = template(&contact.name);
    if !sender.send(&contact.email, &subject, &html).await {
        tracing::warn!(customer_id, "Failed to send billing notification email");
    }
}

pub fn build_emit_request(
    event_type: BillingEventType,
    resource_type: &str,
    resource_id: &str,
    customer_id: Option<&str>,
    data: Option<serde_json::Value>,
) -> EmitBillingEventRequest {
    EmitBillingEventRequest {
        event_type,
        resource_type: resource_type.to_string(),
        resource_id: resource_id.to_string(),
        customer_id: customer_id.map(str::to_string),
        data,
    }
}
