use super::repository::{PaymentCreateOutcome, PaymentsRepository};
use super::schema::{CreatePaymentRequest, ListPaymentsFilter, PaymentView};
use crate::db::models::Payment;
use crate::error::{BillingError, Result};
use validator::Validate;

pub async fn list_payments<R: PaymentsRepository + ?Sized>(
    repo: &R,
    filter: &ListPaymentsFilter,
) -> Result<Vec<PaymentView>> {
    repo.list_payments(filter).await
}

pub async fn create_payment<R: PaymentsRepository + ?Sized>(
    repo: &R,
    req: CreatePaymentRequest,
) -> Result<Payment> {
    Ok(create_payment_details(repo, req).await?.payment)
}

pub(crate) async fn create_payment_details<R: PaymentsRepository + ?Sized>(
    repo: &R,
    req: CreatePaymentRequest,
) -> Result<PaymentCreateOutcome> {
    req.validate().map_err(BillingError::from_validation)?;
    repo.create_payment(&req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{Invoice, InvoiceStatus, Payment as PaymentModel, PaymentMethod};
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        list_rows: Vec<PaymentView>,
        create_req: Option<CreatePaymentRequest>,
        outcome: Option<PaymentCreateOutcome>,
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
    impl PaymentsRepository for StubRepo {
        async fn list_payments(&self, _filter: &ListPaymentsFilter) -> Result<Vec<PaymentView>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn create_payment(&self, req: &CreatePaymentRequest) -> Result<PaymentCreateOutcome> {
            let mut state = self.state.lock().expect("mutex");
            state.create_req = Some(req.clone());
            Ok(state.outcome.clone().expect("outcome"))
        }
    }

    fn sample_payment() -> PaymentModel {
        PaymentModel {
            id: "pay_1".to_string(),
            invoice_id: "inv_1".to_string(),
            amount: Decimal::from(100),
            method: PaymentMethod::Stripe,
            reference: Some("ref".to_string()),
            paid_at: Utc::now().naive_utc(),
            notes: Some("note".to_string()),
            stripe_payment_intent_id: Some("pi_1".to_string()),
            xendit_payment_id: None,
            lemonsqueezy_order_id: None,
            created_at: Utc::now().naive_utc(),
        }
    }

    fn sample_invoice() -> Invoice {
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cus_1".to_string(),
            subscription_id: None,
            status: InvoiceStatus::Draft,
            issued_at: None,
            due_at: None,
            paid_at: None,
            subtotal: Decimal::ZERO,
            tax: Decimal::ZERO,
            total: Decimal::from(100),
            currency: "USD".to_string(),
            notes: None,
            stripe_invoice_id: None,
            xendit_invoice_id: None,
            lemonsqueezy_order_id: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            tax_name: None,
            tax_rate: None,
            tax_inclusive: false,
            credits_applied: Decimal::ZERO,
            amount_due: Decimal::from(100),
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    fn sample_outcome() -> PaymentCreateOutcome {
        PaymentCreateOutcome {
            payment: sample_payment(),
            invoice: sample_invoice(),
            invoice_became_paid: true,
        }
    }

    #[tokio::test]
    async fn list_payments_forwards_repository_rows() {
        let repo = StubRepo::with_state(StubState {
            list_rows: vec![PaymentView {
                id: "pay_1".to_string(),
                invoice_id: "inv_1".to_string(),
                amount: Decimal::from(100),
                method: PaymentMethod::Stripe,
                reference: Some("ref".to_string()),
                paid_at: Utc::now().naive_utc(),
                notes: Some("note".to_string()),
                stripe_payment_intent_id: Some("pi_1".to_string()),
                xendit_payment_id: None,
                lemonsqueezy_order_id: None,
                created_at: Utc::now().naive_utc(),
            }],
            ..StubState::default()
        });

        let rows = list_payments(
            &repo,
            &ListPaymentsFilter {
                invoice_id: Some("inv_1".to_string()),
                role_customer_id: None,
            },
        )
        .await
        .expect("list_payments");

        assert_eq!(rows.len(), 1);
    }

    #[tokio::test]
    async fn create_payment_validates_and_forwards() {
        let repo = StubRepo::with_state(StubState {
            outcome: Some(sample_outcome()),
            ..StubState::default()
        });

        let payment = create_payment(
            &repo,
            CreatePaymentRequest {
                invoice_id: "inv_1".to_string(),
                amount: Decimal::from(100),
                method: PaymentMethod::Stripe,
                reference: Some("ref".to_string()),
                paid_at: None,
                notes: Some("note".to_string()),
                stripe_payment_intent_id: Some("pi_1".to_string()),
                xendit_payment_id: None,
                lemonsqueezy_order_id: None,
            },
        )
        .await
        .expect("create_payment");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(payment.id, "pay_1");
        assert!(state.create_req.is_some());
    }

    #[tokio::test]
    async fn create_payment_rejects_empty_invoice_id() {
        let repo = StubRepo::with_state(StubState::default());

        let err = create_payment(
            &repo,
            CreatePaymentRequest {
                invoice_id: String::new(),
                amount: Decimal::from(100),
                method: PaymentMethod::Stripe,
                reference: None,
                paid_at: None,
                notes: None,
                stripe_payment_intent_id: None,
                xendit_payment_id: None,
                lemonsqueezy_order_id: None,
            },
        )
        .await
        .expect_err("should fail");

        assert!(matches!(err, BillingError::Validation(_)));
    }
}
