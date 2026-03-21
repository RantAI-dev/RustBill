use super::repository::AutoChargeRepository;
use super::schema::{AutoChargeContext, ChargeResult};
use crate::db::models::PaymentProvider;
use crate::error::Result;
use rust_decimal::Decimal;

pub async fn try_auto_charge<R: AutoChargeRepository + ?Sized>(
    repo: &R,
    context: &AutoChargeContext,
) -> Result<ChargeResult> {
    let amount = context.invoice.amount_due;
    if amount <= Decimal::ZERO {
        return Ok(ChargeResult::Success {
            provider_reference: None,
        });
    }

    repo.increment_attempts(&context.invoice.id).await?;

    match context.payment_method.provider {
        PaymentProvider::Stripe => {
            if context
                .payment_method
                .provider_token
                .starts_with("test_success")
            {
                return Ok(ChargeResult::Success {
                    provider_reference: Some("pi_test_success".to_string()),
                });
            }
            if context
                .payment_method
                .provider_token
                .starts_with("test_permanent")
            {
                return Ok(ChargeResult::PermanentFailure(
                    "simulated permanent decline".to_string(),
                ));
            }

            repo.stripe_charge(context, amount).await
        }
        PaymentProvider::Xendit => {
            if context
                .payment_method
                .provider_token
                .starts_with("test_success")
            {
                return Ok(ChargeResult::Success {
                    provider_reference: Some("xendit_test_success".to_string()),
                });
            }
            if context
                .payment_method
                .provider_token
                .starts_with("test_permanent")
            {
                return Ok(ChargeResult::PermanentFailure(
                    "simulated permanent decline".to_string(),
                ));
            }

            repo.xendit_charge(context, amount).await
        }
        PaymentProvider::Lemonsqueezy => Ok(ChargeResult::ManagedExternally),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        Invoice, InvoiceStatus, PaymentProvider, SavedPaymentMethod, SavedPaymentMethodStatus,
        SavedPaymentMethodType,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct StubRepo {
        state: Arc<Mutex<StubState>>,
    }

    #[derive(Default)]
    struct StubState {
        attempts: usize,
        stripe_calls: usize,
        xendit_calls: usize,
    }

    impl StubRepo {
        fn new() -> Self {
            Self {
                state: Arc::new(Mutex::new(StubState::default())),
            }
        }
    }

    #[async_trait]
    impl AutoChargeRepository for StubRepo {
        async fn increment_attempts(&self, _invoice_id: &str) -> Result<()> {
            let mut state = self.state.lock().expect("mutex");
            state.attempts += 1;
            Ok(())
        }

        async fn get_setting(&self, _key: &str) -> Result<String> {
            Ok(String::new())
        }

        async fn stripe_charge(
            &self,
            _context: &AutoChargeContext,
            _amount: Decimal,
        ) -> Result<ChargeResult> {
            let mut state = self.state.lock().expect("mutex");
            state.stripe_calls += 1;
            Ok(ChargeResult::TransientFailure(
                "stripe transient".to_string(),
            ))
        }

        async fn xendit_charge(
            &self,
            _context: &AutoChargeContext,
            _amount: Decimal,
        ) -> Result<ChargeResult> {
            let mut state = self.state.lock().expect("mutex");
            state.xendit_calls += 1;
            Ok(ChargeResult::TransientFailure(
                "xendit transient".to_string(),
            ))
        }
    }

    fn sample_invoice(amount_due: Decimal) -> Invoice {
        let now = Utc::now().naive_utc();
        Invoice {
            id: "inv_1".to_string(),
            invoice_number: "INV-00000001".to_string(),
            customer_id: "cust_1".to_string(),
            subscription_id: None,
            status: InvoiceStatus::Issued,
            issued_at: None,
            due_at: None,
            paid_at: None,
            subtotal: amount_due,
            tax: Decimal::ZERO,
            total: amount_due,
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
            amount_due,
            auto_charge_attempts: 0,
            idempotency_key: None,
        }
    }

    fn sample_method(provider: PaymentProvider, token: &str) -> SavedPaymentMethod {
        let now = Utc::now().naive_utc();
        SavedPaymentMethod {
            id: "pm_1".to_string(),
            customer_id: "cust_1".to_string(),
            provider,
            provider_token: token.to_string(),
            method_type: SavedPaymentMethodType::Card,
            label: "card".to_string(),
            last_four: Some("4242".to_string()),
            expiry_month: Some(12),
            expiry_year: Some(2030),
            is_default: true,
            status: SavedPaymentMethodStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn zero_due_returns_success_without_attempts() {
        let repo = StubRepo::new();
        let context = AutoChargeContext {
            invoice: sample_invoice(Decimal::ZERO),
            payment_method: sample_method(PaymentProvider::Stripe, "pm_real"),
        };

        let result = try_auto_charge(&repo, &context).await.expect("auto-charge");
        let state = repo.state.lock().expect("mutex");

        assert!(matches!(
            result,
            ChargeResult::Success {
                provider_reference: None
            }
        ));
        assert_eq!(state.attempts, 0);
    }

    #[tokio::test]
    async fn stripe_test_success_short_circuits() {
        let repo = StubRepo::new();
        let context = AutoChargeContext {
            invoice: sample_invoice(Decimal::from(100)),
            payment_method: sample_method(PaymentProvider::Stripe, "test_success_card"),
        };

        let result = try_auto_charge(&repo, &context).await.expect("auto-charge");
        let state = repo.state.lock().expect("mutex");

        assert!(matches!(
            result,
            ChargeResult::Success {
                provider_reference: Some(_)
            }
        ));
        assert_eq!(state.attempts, 1);
        assert_eq!(state.stripe_calls, 0);
    }

    #[tokio::test]
    async fn stripe_non_test_delegates_to_repository() {
        let repo = StubRepo::new();
        let context = AutoChargeContext {
            invoice: sample_invoice(Decimal::from(100)),
            payment_method: sample_method(PaymentProvider::Stripe, "pm_live"),
        };

        let result = try_auto_charge(&repo, &context).await.expect("auto-charge");
        let state = repo.state.lock().expect("mutex");

        assert!(matches!(result, ChargeResult::TransientFailure(_)));
        assert_eq!(state.attempts, 1);
        assert_eq!(state.stripe_calls, 1);
    }

    #[tokio::test]
    async fn lemonsqueezy_returns_managed_externally() {
        let repo = StubRepo::new();
        let context = AutoChargeContext {
            invoice: sample_invoice(Decimal::from(100)),
            payment_method: sample_method(PaymentProvider::Lemonsqueezy, "ls_123"),
        };

        let result = try_auto_charge(&repo, &context).await.expect("auto-charge");
        let state = repo.state.lock().expect("mutex");

        assert!(matches!(result, ChargeResult::ManagedExternally));
        assert_eq!(state.attempts, 1);
        assert_eq!(state.stripe_calls, 0);
        assert_eq!(state.xendit_calls, 0);
    }
}
