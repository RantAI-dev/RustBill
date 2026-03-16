use crate::db::models::*;
use crate::error::{BillingError, Result};
use crate::payments::{lemonsqueezy, xendit};
use crate::settings::provider_settings::ProviderSettings;
use reqwest::Client;
use serde::Serialize;
use sqlx::PgPool;

/// Result of creating a checkout session.
#[derive(Debug, Clone, Serialize)]
pub struct CheckoutResult {
    pub checkout_url: String,
    pub provider: String,
}

/// Create a checkout session for a given invoice using the specified payment provider.
///
/// Supported providers: `"stripe"`, `"xendit"`, `"lemonsqueezy"`.
/// The `origin` parameter is the base URL for redirect URLs (e.g. `https://billing.example.com`).
pub async fn create_checkout(
    pool: &PgPool,
    http: &Client,
    settings: &ProviderSettings,
    invoice_id: &str,
    provider: &str,
    origin: &str,
) -> Result<CheckoutResult> {
    // Fetch the invoice
    let invoice = crate::billing::invoices::get_invoice(pool, invoice_id).await?;

    if invoice.status == InvoiceStatus::Paid {
        return Err(BillingError::bad_request("invoice is already paid"));
    }
    if invoice.status == InvoiceStatus::Void {
        return Err(BillingError::bad_request("invoice has been voided"));
    }

    // Fetch customer for provider-specific customer IDs
    let customer = sqlx::query_as::<_, Customer>("SELECT * FROM customers WHERE id = $1")
        .bind(&invoice.customer_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| BillingError::not_found("customer", &invoice.customer_id))?;

    let success_url = format!("{origin}/checkout/success?invoice_id={invoice_id}");
    let cancel_url = format!("{origin}/checkout/cancel?invoice_id={invoice_id}");

    match provider {
        "stripe" => {
            create_stripe_checkout(pool, &invoice, &customer, &success_url, &cancel_url).await
        }
        "xendit" => {
            create_xendit_checkout(
                pool,
                http,
                settings,
                &invoice,
                &customer,
                &success_url,
                &cancel_url,
            )
            .await
        }
        "lemonsqueezy" => {
            create_lemonsqueezy_checkout(pool, http, settings, &invoice, &customer, &success_url)
                .await
        }
        _ => Err(BillingError::ProviderNotConfigured(provider.to_string())),
    }
}

// ---- Provider-specific checkout creation ----

async fn create_stripe_checkout(
    _pool: &PgPool,
    invoice: &Invoice,
    customer: &Customer,
    success_url: &str,
    cancel_url: &str,
) -> Result<CheckoutResult> {
    // Stripe checkout still uses placeholder URL -- Stripe SDK integration is out of scope
    // for the reqwest-based approach (Stripe requires form-encoded requests with their SDK).
    let _stripe_customer_id = customer.stripe_customer_id.as_ref().ok_or_else(|| {
        BillingError::bad_request("customer does not have a Stripe customer ID configured")
    })?;

    let checkout_url = format!(
        "https://checkout.stripe.com/pay/placeholder?invoice={}&amount={}&currency={}&success_url={}&cancel_url={}",
        invoice.id,
        invoice.total,
        invoice.currency,
        urlencoding::encode(success_url),
        urlencoding::encode(cancel_url),
    );

    Ok(CheckoutResult {
        checkout_url,
        provider: "stripe".to_string(),
    })
}

async fn create_xendit_checkout(
    pool: &PgPool,
    http: &Client,
    settings: &ProviderSettings,
    invoice: &Invoice,
    customer: &Customer,
    success_url: &str,
    cancel_url: &str,
) -> Result<CheckoutResult> {
    let _xendit_customer_id = customer.xendit_customer_id.as_ref().ok_or_else(|| {
        BillingError::bad_request("customer does not have a Xendit customer ID configured")
    })?;

    let params = xendit::XenditInvoiceParams {
        invoice_id: invoice.id.clone(),
        invoice_number: invoice.invoice_number.clone(),
        total: invoice.total,
        currency: invoice.currency.clone(),
        customer_email: Some(customer.email.clone()),
        customer_name: Some(customer.name.clone()),
        success_url: success_url.to_string(),
        failure_url: cancel_url.to_string(),
    };

    let result = xendit::create_invoice(http, settings, params).await?;

    // Store the Xendit invoice ID on the invoice record
    sqlx::query("UPDATE invoices SET xendit_invoice_id = $2, updated_at = NOW() WHERE id = $1")
        .bind(&invoice.id)
        .bind(&result.xendit_invoice_id)
        .execute(pool)
        .await?;

    Ok(CheckoutResult {
        checkout_url: result.invoice_url,
        provider: "xendit".to_string(),
    })
}

async fn create_lemonsqueezy_checkout(
    pool: &PgPool,
    http: &Client,
    settings: &ProviderSettings,
    invoice: &Invoice,
    customer: &Customer,
    success_url: &str,
) -> Result<CheckoutResult> {
    let params = lemonsqueezy::LsCheckoutParams {
        invoice_id: invoice.id.clone(),
        invoice_number: invoice.invoice_number.clone(),
        total: invoice.total,
        currency: invoice.currency.clone(),
        customer_email: Some(customer.email.clone()),
        customer_name: Some(customer.name.clone()),
        success_url: success_url.to_string(),
    };

    let result = lemonsqueezy::create_checkout(http, settings, params).await?;

    // Store the LemonSqueezy checkout/order ID on the invoice record
    sqlx::query("UPDATE invoices SET lemonsqueezy_order_id = $2, updated_at = NOW() WHERE id = $1")
        .bind(&invoice.id)
        .bind(&result.checkout_id)
        .execute(pool)
        .await?;

    Ok(CheckoutResult {
        checkout_url: result.checkout_url,
        provider: "lemonsqueezy".to_string(),
    })
}
