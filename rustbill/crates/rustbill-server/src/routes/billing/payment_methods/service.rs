use super::repository::PaymentMethodRepository;
use super::schema::{
    CreatePaymentMethodRequest, DeletePaymentMethodResponse, SetupPaymentMethodRequest,
    SetupPaymentMethodResponse, DEFAULT_STRIPE_SETUP_CANCEL_URL, DEFAULT_STRIPE_SETUP_SUCCESS_URL,
};
use async_trait::async_trait;
use rustbill_core::db::models::{PaymentProvider, SavedPaymentMethod};
use rustbill_core::error::BillingError;

#[async_trait]
pub trait PaymentMethodSetupGateway: Send + Sync {
    async fn create_stripe_setup_session(
        &self,
        customer_id: &str,
        success_url: &str,
        cancel_url: &str,
    ) -> Result<SetupPaymentMethodResponse, BillingError>;

    async fn create_xendit_setup_session(
        &self,
        customer_id: &str,
    ) -> Result<SetupPaymentMethodResponse, BillingError>;
}

pub async fn list_for_customer<R: PaymentMethodRepository>(
    repo: &R,
    customer_id: &str,
) -> Result<Vec<SavedPaymentMethod>, BillingError> {
    repo.list_for_customer(customer_id).await
}

pub async fn create<R: PaymentMethodRepository>(
    repo: &R,
    req: &CreatePaymentMethodRequest,
) -> Result<SavedPaymentMethod, BillingError> {
    repo.create(req).await
}

pub async fn remove<R: PaymentMethodRepository>(
    repo: &R,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> Result<DeletePaymentMethodResponse, BillingError> {
    let customer_id = resolve_customer_id(repo, method_id, provided_customer_id).await?;
    repo.remove(&customer_id, method_id).await?;
    Ok(DeletePaymentMethodResponse { deleted: true })
}

pub async fn set_default<R: PaymentMethodRepository>(
    repo: &R,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> Result<SavedPaymentMethod, BillingError> {
    let customer_id = resolve_customer_id(repo, method_id, provided_customer_id).await?;
    repo.set_default(&customer_id, method_id).await
}

pub async fn create_setup_session<G: PaymentMethodSetupGateway>(
    gateway: &G,
    req: &SetupPaymentMethodRequest,
) -> Result<SetupPaymentMethodResponse, BillingError> {
    match req.provider {
        PaymentProvider::Stripe => {
            let success_url = match req.success_url.as_deref() {
                Some(url) => url,
                None => DEFAULT_STRIPE_SETUP_SUCCESS_URL,
            };
            let cancel_url = match req.cancel_url.as_deref() {
                Some(url) => url,
                None => DEFAULT_STRIPE_SETUP_CANCEL_URL,
            };
            gateway
                .create_stripe_setup_session(&req.customer_id, success_url, cancel_url)
                .await
        }
        PaymentProvider::Xendit => gateway.create_xendit_setup_session(&req.customer_id).await,
        PaymentProvider::Lemonsqueezy => Err(BillingError::bad_request(
            "lemonsqueezy setup sessions are not supported; use LS-managed subscription checkout",
        )),
    }
}

async fn resolve_customer_id<R: PaymentMethodRepository>(
    repo: &R,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> Result<String, BillingError> {
    match provided_customer_id {
        Some(customer_id) => Ok(customer_id.to_string()),
        None => repo
            .find_customer_id(method_id)
            .await?
            .ok_or_else(|| BillingError::not_found("payment_method", method_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::db::models::{
        PaymentProvider, SavedPaymentMethod, SavedPaymentMethodStatus, SavedPaymentMethodType,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct MockPaymentMethodRepository {
        customer_id: Option<String>,
        created: Mutex<Option<CreatePaymentMethodRequest>>,
        removed: Mutex<Vec<(String, String)>>,
        set_default: Mutex<Vec<(String, String)>>,
        list_called: AtomicBool,
    }

    impl MockPaymentMethodRepository {
        fn new(customer_id: Option<String>) -> Self {
            Self {
                customer_id,
                created: Mutex::new(None),
                removed: Mutex::new(Vec::new()),
                set_default: Mutex::new(Vec::new()),
                list_called: AtomicBool::new(false),
            }
        }

        fn sample_method(id: &str) -> SavedPaymentMethod {
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
                is_default: true,
                status: SavedPaymentMethodStatus::Active,
                created_at: chrono::NaiveDateTime::MIN,
                updated_at: chrono::NaiveDateTime::MIN,
            }
        }
    }

    #[async_trait]
    impl PaymentMethodRepository for MockPaymentMethodRepository {
        async fn list_for_customer(
            &self,
            _customer_id: &str,
        ) -> Result<Vec<SavedPaymentMethod>, BillingError> {
            self.list_called.store(true, Ordering::SeqCst);
            Ok(vec![Self::sample_method("pm-1")])
        }

        async fn create(
            &self,
            req: &CreatePaymentMethodRequest,
        ) -> Result<SavedPaymentMethod, BillingError> {
            if let Ok(mut guard) = self.created.lock() {
                *guard = Some(req.clone());
            }
            Ok(Self::sample_method("pm-created"))
        }

        async fn find_customer_id(&self, _method_id: &str) -> Result<Option<String>, BillingError> {
            Ok(self.customer_id.clone())
        }

        async fn remove(&self, customer_id: &str, method_id: &str) -> Result<(), BillingError> {
            if let Ok(mut guard) = self.removed.lock() {
                guard.push((customer_id.to_string(), method_id.to_string()));
            }
            Ok(())
        }

        async fn set_default(
            &self,
            customer_id: &str,
            method_id: &str,
        ) -> Result<SavedPaymentMethod, BillingError> {
            if let Ok(mut guard) = self.set_default.lock() {
                guard.push((customer_id.to_string(), method_id.to_string()));
            }
            Ok(Self::sample_method(method_id))
        }
    }

    struct MockGateway {
        stripe_args: Mutex<Vec<(String, String, String)>>,
        xendit_args: Mutex<Vec<String>>,
    }

    impl MockGateway {
        fn new() -> Self {
            Self {
                stripe_args: Mutex::new(Vec::new()),
                xendit_args: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl PaymentMethodSetupGateway for MockGateway {
        async fn create_stripe_setup_session(
            &self,
            customer_id: &str,
            success_url: &str,
            cancel_url: &str,
        ) -> Result<SetupPaymentMethodResponse, BillingError> {
            if let Ok(mut guard) = self.stripe_args.lock() {
                guard.push((
                    customer_id.to_string(),
                    success_url.to_string(),
                    cancel_url.to_string(),
                ));
            }
            Ok(SetupPaymentMethodResponse {
                provider: PaymentProvider::Stripe,
                customer_id: customer_id.to_string(),
                setup_url: Some("https://example.test/stripe".to_string()),
                session_id: Some("cs_test_1".to_string()),
                setup_id: None,
                actions: None,
            })
        }

        async fn create_xendit_setup_session(
            &self,
            customer_id: &str,
        ) -> Result<SetupPaymentMethodResponse, BillingError> {
            if let Ok(mut guard) = self.xendit_args.lock() {
                guard.push(customer_id.to_string());
            }
            Ok(SetupPaymentMethodResponse {
                provider: PaymentProvider::Xendit,
                customer_id: customer_id.to_string(),
                setup_url: None,
                session_id: None,
                setup_id: Some("pm-xendit-1".to_string()),
                actions: Some(serde_json::json!([{ "type": "redirect" }])),
            })
        }
    }

    #[tokio::test]
    async fn create_setup_session_uses_default_stripe_urls() {
        let gateway = MockGateway::new();
        let request = SetupPaymentMethodRequest {
            customer_id: "cust-1".to_string(),
            provider: PaymentProvider::Stripe,
            success_url: None,
            cancel_url: None,
        };

        let response = create_setup_session(&gateway, &request).await.unwrap();
        assert_eq!(response.provider, PaymentProvider::Stripe);
        let args = gateway.stripe_args.lock().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].0, "cust-1");
        assert_eq!(args[0].1, DEFAULT_STRIPE_SETUP_SUCCESS_URL);
        assert_eq!(args[0].2, DEFAULT_STRIPE_SETUP_CANCEL_URL);
    }

    #[tokio::test]
    async fn remove_resolves_customer_id_before_deleting() {
        let repo = MockPaymentMethodRepository::new(Some("cust-1".to_string()));

        let result = remove(&repo, "pm-1", None).await.unwrap();
        assert!(result.deleted);
        let removed = repo.removed.lock().unwrap();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "cust-1");
        assert_eq!(removed[0].1, "pm-1");
    }

    #[tokio::test]
    async fn set_default_uses_provided_customer_id() {
        let repo = MockPaymentMethodRepository::new(Some("cust-2".to_string()));

        let result = set_default(&repo, "pm-2", Some("cust-explicit"))
            .await
            .unwrap();
        assert_eq!(result.id, "pm-2");
        let calls = repo.set_default.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "cust-explicit");
        assert_eq!(calls[0].1, "pm-2");
    }
}
