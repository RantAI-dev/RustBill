use super::repository::{CheckoutProviderResult, CheckoutRepository};
use super::schema::{CheckoutContext, CheckoutRequest, CheckoutResult};
use crate::db::models::InvoiceStatus;
use crate::error::{BillingError, Result};

pub async fn create_checkout<R: CheckoutRepository + ?Sized>(
    repo: &R,
    request: CheckoutRequest,
) -> Result<CheckoutResult> {
    let invoice = repo.find_invoice(&request.invoice_id).await?;

    if invoice.status == InvoiceStatus::Paid {
        return Err(BillingError::bad_request("invoice is already paid"));
    }
    if invoice.status == InvoiceStatus::Void {
        return Err(BillingError::bad_request("invoice has been voided"));
    }

    let customer = repo.find_customer(&invoice.customer_id).await?;
    let context = CheckoutContext {
        invoice,
        customer,
        success_url: format!(
            "{}/checkout/success?invoice_id={}",
            request.origin, request.invoice_id
        ),
        cancel_url: format!(
            "{}/checkout/cancel?invoice_id={}",
            request.origin, request.invoice_id
        ),
    };

    match request.provider.as_str() {
        "stripe" => create_stripe_checkout(&context),
        "xendit" => create_xendit_checkout(repo, &context).await,
        "lemonsqueezy" => create_lemonsqueezy_checkout(repo, &context).await,
        _ => Err(BillingError::ProviderNotConfigured(request.provider)),
    }
}

fn create_stripe_checkout(context: &CheckoutContext) -> Result<CheckoutResult> {
    let stripe_customer_id = context
        .customer
        .stripe_customer_id
        .as_ref()
        .ok_or_else(|| {
            BillingError::bad_request("customer does not have a Stripe customer ID configured")
        })?;

    let _ = stripe_customer_id;

    let checkout_url = format!(
        "https://checkout.stripe.com/pay/placeholder?invoice={}&amount={}&currency={}&success_url={}&cancel_url={}",
        context.invoice.id,
        context.invoice.total,
        context.invoice.currency,
        urlencoding::encode(&context.success_url),
        urlencoding::encode(&context.cancel_url),
    );

    Ok(CheckoutResult {
        checkout_url,
        provider: "stripe".to_string(),
    })
}

async fn create_xendit_checkout<R: CheckoutRepository + ?Sized>(
    repo: &R,
    context: &CheckoutContext,
) -> Result<CheckoutResult> {
    let _xendit_customer_id = context
        .customer
        .xendit_customer_id
        .as_ref()
        .ok_or_else(|| {
            BillingError::bad_request("customer does not have a Xendit customer ID configured")
        })?;

    let provider_result = repo.create_xendit_checkout(context).await?;
    repo.save_xendit_invoice_id(&context.invoice.id, &provider_result.provider_reference)
        .await?;

    Ok(to_checkout_result(provider_result, "xendit"))
}

async fn create_lemonsqueezy_checkout<R: CheckoutRepository + ?Sized>(
    repo: &R,
    context: &CheckoutContext,
) -> Result<CheckoutResult> {
    let provider_result = repo.create_lemonsqueezy_checkout(context).await?;
    repo.save_lemonsqueezy_checkout_id(&context.invoice.id, &provider_result.provider_reference)
        .await?;

    Ok(to_checkout_result(provider_result, "lemonsqueezy"))
}

fn to_checkout_result(provider_result: CheckoutProviderResult, provider: &str) -> CheckoutResult {
    CheckoutResult {
        checkout_url: provider_result.checkout_url,
        provider: provider.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Customer, Invoice, Trend};
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        invoice: Option<Invoice>,
        customer: Option<Customer>,
        xendit_called: bool,
        lemonsqueezy_called: bool,
        xendit_saved_id: Option<String>,
        lemonsqueezy_saved_id: Option<String>,
    }

    #[derive(Clone, Default)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    impl StubRepo {
        fn with_state(state: StubState) -> Self {
            Self {
                state: Arc::new(Mutex::new(state)),
            }
        }
    }

    #[async_trait]
    impl CheckoutRepository for StubRepo {
        async fn find_invoice(&self, _invoice_id: &str) -> Result<Invoice> {
            let state = self.state.lock().expect("mutex");
            state
                .invoice
                .clone()
                .ok_or_else(|| BillingError::not_found("invoice", "inv_1"))
        }

        async fn find_customer(&self, _customer_id: &str) -> Result<Customer> {
            let state = self.state.lock().expect("mutex");
            state
                .customer
                .clone()
                .ok_or_else(|| BillingError::not_found("customer", "cust_1"))
        }

        async fn create_xendit_checkout(
            &self,
            _ctx: &CheckoutContext,
        ) -> Result<CheckoutProviderResult> {
            let mut state = self.state.lock().expect("mutex");
            state.xendit_called = true;
            Ok(CheckoutProviderResult {
                checkout_url: "https://xendit.example/checkout".to_string(),
                provider_reference: "xendit_inv_1".to_string(),
            })
        }

        async fn save_xendit_invoice_id(&self, _invoice_id: &str, provider_id: &str) -> Result<()> {
            let mut state = self.state.lock().expect("mutex");
            state.xendit_saved_id = Some(provider_id.to_string());
            Ok(())
        }

        async fn create_lemonsqueezy_checkout(
            &self,
            _ctx: &CheckoutContext,
        ) -> Result<CheckoutProviderResult> {
            let mut state = self.state.lock().expect("mutex");
            state.lemonsqueezy_called = true;
            Ok(CheckoutProviderResult {
                checkout_url: "https://lemonsqueezy.example/checkout".to_string(),
                provider_reference: "ls_1".to_string(),
            })
        }

        async fn save_lemonsqueezy_checkout_id(
            &self,
            _invoice_id: &str,
            provider_id: &str,
        ) -> Result<()> {
            let mut state = self.state.lock().expect("mutex");
            state.lemonsqueezy_saved_id = Some(provider_id.to_string());
            Ok(())
        }
    }

    fn sample_invoice(status: InvoiceStatus) -> Invoice {
        let now = Utc::now().naive_utc();
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cust_1".to_string(),
            subscription_id: None,
            status,
            issued_at: None,
            due_at: None,
            paid_at: None,
            subtotal: Decimal::from(100),
            tax: Decimal::ZERO,
            total: Decimal::from(100),
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: now,
            updated_at: now,
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::from(100),
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    fn sample_customer() -> Customer {
        let now = Utc::now().naive_utc();
        Customer {
            id: "cust_1".to_string(),
            name: "Acme".to_string(),
            industry: "Software".to_string(),
            tier: "Enterprise".to_string(),
            location: "US".to_string(),
            contact: "Ops".to_string(),
            email: "ops@acme.test".to_string(),
            phone: "555-0000".to_string(),
            total_revenue: Decimal::from(1000),
            health_score: 90,
            trend: Trend::Stable,
            last_contact: "2026-01-01".to_string(),
            billing_email: None,
            billing_address: None,
            billing_city: None,
            billing_state: None,
            billing_zip: None,
            billing_country: None,
            tax_id: None,
            default_payment_method: None,
            stripe_customer_id: Some("cus_123".to_string()),
            xendit_customer_id: Some("xendit_cus_123".to_string()),
            created_at: now,
            updated_at: now,
        }
    }

    fn sample_request(provider: &str) -> CheckoutRequest {
        CheckoutRequest {
            invoice_id: "inv_1".to_string(),
            provider: provider.to_string(),
            origin: "https://billing.example.com".to_string(),
        }
    }

    #[tokio::test]
    async fn stripe_checkout_returns_placeholder_url() {
        let repo = StubRepo::with_state(StubState {
            invoice: Some(sample_invoice(InvoiceStatus::Issued)),
            customer: Some(sample_customer()),
            ..StubState::default()
        });

        let result = create_checkout(&repo, sample_request("stripe"))
            .await
            .expect("stripe checkout");

        assert_eq!(result.provider, "stripe");
        assert!(result.checkout_url.contains("checkout.stripe.com"));
    }

    #[tokio::test]
    async fn paid_invoice_is_rejected() {
        let repo = StubRepo::with_state(StubState {
            invoice: Some(sample_invoice(InvoiceStatus::Paid)),
            customer: Some(sample_customer()),
            ..StubState::default()
        });

        let err = create_checkout(&repo, sample_request("stripe"))
            .await
            .expect_err("paid invoice should fail");
        assert!(err.to_string().contains("already paid"));
    }

    #[tokio::test]
    async fn xendit_path_persists_provider_id() {
        let repo = StubRepo::with_state(StubState {
            invoice: Some(sample_invoice(InvoiceStatus::Issued)),
            customer: Some(sample_customer()),
            ..StubState::default()
        });

        let result = create_checkout(&repo, sample_request("xendit"))
            .await
            .expect("xendit checkout");
        let state = repo.state.lock().expect("mutex");

        assert_eq!(result.provider, "xendit");
        assert!(state.xendit_called);
        assert_eq!(state.xendit_saved_id.as_deref(), Some("xendit_inv_1"));
    }

    #[tokio::test]
    async fn unknown_provider_returns_configuration_error() {
        let repo = StubRepo::with_state(StubState {
            invoice: Some(sample_invoice(InvoiceStatus::Issued)),
            customer: Some(sample_customer()),
            ..StubState::default()
        });

        let err = create_checkout(&repo, sample_request("unknown"))
            .await
            .expect_err("unknown provider should fail");
        assert!(err.to_string().contains("provider not configured"));
    }
}
