use super::repository::PaymentMethodRepository;
use super::schema::{CreatePaymentMethodDraft, CreatePaymentMethodRequest};
use crate::db::models::SavedPaymentMethod;
use crate::error::{BillingError, Result};

pub async fn list_for_customer<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    customer_id: &str,
) -> Result<Vec<SavedPaymentMethod>> {
    repo.list_for_customer(customer_id).await
}

pub async fn get_default<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    customer_id: &str,
) -> Result<Option<SavedPaymentMethod>> {
    repo.get_default(customer_id).await
}

pub async fn create<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    req: CreatePaymentMethodRequest,
) -> Result<SavedPaymentMethod> {
    let active_count = repo.count_active_for_customer(&req.customer_id).await?;
    let is_default = req.set_default || active_count == 0;
    let draft = CreatePaymentMethodDraft {
        customer_id: req.customer_id,
        provider: req.provider,
        provider_token: req.provider_token,
        method_type: req.method_type,
        label: req.label,
        last_four: req.last_four,
        expiry_month: req.expiry_month,
        expiry_year: req.expiry_year,
        is_default,
        clear_existing_default: req.set_default,
    };

    repo.create(&draft).await
}

pub async fn set_default<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    customer_id: &str,
    method_id: &str,
) -> Result<SavedPaymentMethod> {
    repo.set_default(customer_id, method_id)
        .await?
        .ok_or_else(|| BillingError::not_found("payment_method", method_id))
}

pub async fn remove<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    customer_id: &str,
    method_id: &str,
) -> Result<()> {
    let result = repo.remove(customer_id, method_id).await?;
    if result == 0 {
        return Err(BillingError::not_found("payment_method", method_id));
    }
    Ok(())
}

pub async fn mark_failed<R: PaymentMethodRepository + ?Sized>(
    repo: &R,
    method_id: &str,
) -> Result<()> {
    repo.mark_failed(method_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        PaymentProvider, SavedPaymentMethod, SavedPaymentMethodStatus, SavedPaymentMethodType,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        list_rows: Vec<SavedPaymentMethod>,
        active_count: i64,
        created_draft: Option<CreatePaymentMethodDraft>,
        created_method: Option<SavedPaymentMethod>,
        set_default_result: Option<SavedPaymentMethod>,
        removed_rows: u64,
        marked_failed: Option<String>,
        default_method: Option<SavedPaymentMethod>,
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

    fn sample_method(id: &str, is_default: bool) -> SavedPaymentMethod {
        SavedPaymentMethod {
            id: id.to_string(),
            customer_id: "cust-1".to_string(),
            provider: PaymentProvider::Stripe,
            provider_token: "token-1".to_string(),
            method_type: SavedPaymentMethodType::Card,
            label: "Card".to_string(),
            last_four: Some("4242".to_string()),
            expiry_month: Some(12),
            expiry_year: Some(2030),
            is_default,
            status: SavedPaymentMethodStatus::Active,
            created_at: chrono::NaiveDateTime::MIN,
            updated_at: chrono::NaiveDateTime::MIN,
        }
    }

    #[async_trait]
    impl PaymentMethodRepository for StubRepo {
        async fn list_for_customer(&self, _customer_id: &str) -> Result<Vec<SavedPaymentMethod>> {
            Ok(self.state.lock().expect("mutex").list_rows.clone())
        }

        async fn get_default(&self, _customer_id: &str) -> Result<Option<SavedPaymentMethod>> {
            Ok(self.state.lock().expect("mutex").default_method.clone())
        }

        async fn count_active_for_customer(&self, _customer_id: &str) -> Result<i64> {
            Ok(self.state.lock().expect("mutex").active_count)
        }

        async fn create(&self, draft: &CreatePaymentMethodDraft) -> Result<SavedPaymentMethod> {
            let mut state = self.state.lock().expect("mutex");
            state.created_draft = Some(draft.clone());
            Ok(state
                .created_method
                .clone()
                .unwrap_or_else(|| sample_method("pm-created", draft.is_default)))
        }

        async fn set_default(
            &self,
            _customer_id: &str,
            _method_id: &str,
        ) -> Result<Option<SavedPaymentMethod>> {
            Ok(self.state.lock().expect("mutex").set_default_result.clone())
        }

        async fn remove(&self, _customer_id: &str, _method_id: &str) -> Result<u64> {
            Ok(self.state.lock().expect("mutex").removed_rows)
        }

        async fn mark_failed(&self, method_id: &str) -> Result<()> {
            self.state.lock().expect("mutex").marked_failed = Some(method_id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn create_defaults_when_no_active_methods_exist() {
        let repo = StubRepo::with_state(StubState {
            active_count: 0,
            created_method: Some(sample_method("pm-1", true)),
            ..StubState::default()
        });

        let result = create(
            &repo,
            CreatePaymentMethodRequest {
                customer_id: "cust-1".to_string(),
                provider: PaymentProvider::Stripe,
                provider_token: "token-1".to_string(),
                method_type: SavedPaymentMethodType::Card,
                label: "Card".to_string(),
                last_four: Some("4242".to_string()),
                expiry_month: Some(12),
                expiry_year: Some(2030),
                set_default: false,
            },
        )
        .await
        .expect("create");

        let state = repo.state.lock().expect("mutex");
        assert_eq!(result.id, "pm-1");
        assert!(state.created_draft.as_ref().unwrap().is_default);
        assert!(!state.created_draft.as_ref().unwrap().clear_existing_default);
    }

    #[tokio::test]
    async fn create_clears_existing_default_when_requested() {
        let repo = StubRepo::with_state(StubState {
            active_count: 2,
            created_method: Some(sample_method("pm-2", true)),
            ..StubState::default()
        });

        let _ = create(
            &repo,
            CreatePaymentMethodRequest {
                customer_id: "cust-1".to_string(),
                provider: PaymentProvider::Stripe,
                provider_token: "token-1".to_string(),
                method_type: SavedPaymentMethodType::Card,
                label: "Card".to_string(),
                last_four: Some("4242".to_string()),
                expiry_month: Some(12),
                expiry_year: Some(2030),
                set_default: true,
            },
        )
        .await
        .expect("create");

        let state = repo.state.lock().expect("mutex");
        assert!(state.created_draft.as_ref().unwrap().is_default);
        assert!(state.created_draft.as_ref().unwrap().clear_existing_default);
    }

    #[tokio::test]
    async fn set_default_maps_missing_to_not_found() {
        let repo = StubRepo::with_state(StubState {
            set_default_result: None,
            ..StubState::default()
        });

        let err = set_default(&repo, "cust-1", "pm-1")
            .await
            .expect_err("should fail");
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "payment_method",
                id
            } if id == "pm-1"
        ));
    }

    #[tokio::test]
    async fn remove_maps_zero_rows_to_not_found() {
        let repo = StubRepo::with_state(StubState {
            removed_rows: 0,
            ..StubState::default()
        });

        let err = remove(&repo, "cust-1", "pm-1")
            .await
            .expect_err("should fail");
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "payment_method",
                id
            } if id == "pm-1"
        ));
    }

    #[tokio::test]
    async fn mark_failed_forwards_to_repository() {
        let repo = StubRepo::default();

        mark_failed(&repo, "pm-1").await.expect("mark_failed");
        let state = repo.state.lock().expect("mutex");
        assert_eq!(state.marked_failed.as_deref(), Some("pm-1"));
    }
}
