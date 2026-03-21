use super::repository::BillingRepository;
use super::schema::{
    CreatePaymentMethodRequestV1, CreditsQueryV1, DeletePaymentMethodResponse,
    PaymentMethodSetupRequestV1,
};
use rustbill_core::auth::api_key::ApiKeyInfo;
use rustbill_core::db::models::SavedPaymentMethod;
use rustbill_core::error::BillingError;

pub async fn list_payment_methods<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    query_customer_id: Option<&str>,
) -> std::result::Result<Vec<SavedPaymentMethod>, BillingError> {
    let customer_id = scoped_customer_id(api_key, query_customer_id)?;
    repo.list_payment_methods(&customer_id).await
}

pub async fn create_payment_method<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    body: &CreatePaymentMethodRequestV1,
) -> std::result::Result<SavedPaymentMethod, BillingError> {
    let customer_id = scoped_customer_id(api_key, Some(&body.customer_id))?;
    let request = CreatePaymentMethodRequestV1 {
        customer_id,
        provider: body.provider.clone(),
        provider_token: body.provider_token.clone(),
        method_type: body.method_type.clone(),
        label: body.label.clone(),
        last_four: body.last_four.clone(),
        expiry_month: body.expiry_month,
        expiry_year: body.expiry_year,
        set_default: body.set_default,
    };
    repo.create_payment_method(&request).await
}

pub async fn create_payment_method_setup<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    body: &PaymentMethodSetupRequestV1,
) -> std::result::Result<serde_json::Value, BillingError> {
    let customer_id = scoped_customer_id(api_key, Some(&body.customer_id))?;

    match body.provider.clone() {
        rustbill_core::db::models::PaymentProvider::Stripe => {
            let success_url = body
                .success_url
                .as_deref()
                .unwrap_or(super::schema::DEFAULT_STRIPE_SETUP_SUCCESS_URL);
            let cancel_url = body
                .cancel_url
                .as_deref()
                .unwrap_or(super::schema::DEFAULT_STRIPE_SETUP_CANCEL_URL);
            repo.create_stripe_setup_session(&customer_id, success_url, cancel_url)
                .await
        }
        rustbill_core::db::models::PaymentProvider::Xendit => {
            repo.create_xendit_setup_session(&customer_id).await
        }
        rustbill_core::db::models::PaymentProvider::Lemonsqueezy => Err(BillingError::bad_request(
            "lemonsqueezy setup sessions are not supported; use LS-managed subscription checkout",
        )),
    }
}

pub async fn delete_payment_method<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> std::result::Result<DeletePaymentMethodResponse, BillingError> {
    let customer_id =
        resolve_payment_method_customer_id(repo, api_key, method_id, provided_customer_id).await?;
    repo.remove_payment_method(&customer_id, method_id).await?;
    Ok(DeletePaymentMethodResponse { deleted: true })
}

pub async fn set_default_payment_method<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> std::result::Result<SavedPaymentMethod, BillingError> {
    let customer_id =
        resolve_payment_method_customer_id(repo, api_key, method_id, provided_customer_id).await?;
    repo.set_default_payment_method(&customer_id, method_id)
        .await
}

pub async fn get_credits<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    query: &CreditsQueryV1,
) -> std::result::Result<serde_json::Value, BillingError> {
    let customer_id = scoped_customer_id(api_key, query.customer_id.as_deref())?;
    let currency = query.currency.as_deref().unwrap_or("USD");
    let balance = repo.get_credit_balance(&customer_id, currency).await?;
    let history = repo
        .list_credits(&customer_id, query.currency.as_deref())
        .await?;

    Ok(serde_json::json!({
        "balance": balance,
        "currency": currency,
        "history": history,
    }))
}

fn scoped_customer_id(
    api_key: &ApiKeyInfo,
    requested: Option<&str>,
) -> std::result::Result<String, BillingError> {
    let scoped = api_key
        .customer_id
        .as_deref()
        .ok_or(BillingError::Forbidden)?;

    if let Some(requested) = requested {
        if requested != scoped {
            return Err(BillingError::Forbidden);
        }
    }

    Ok(scoped.to_string())
}

async fn resolve_payment_method_customer_id<R: BillingRepository>(
    repo: &R,
    api_key: &ApiKeyInfo,
    method_id: &str,
    provided_customer_id: Option<&str>,
) -> std::result::Result<String, BillingError> {
    let scoped_customer_id = scoped_customer_id(api_key, provided_customer_id)?;

    if let Some(customer_id) = provided_customer_id {
        return Ok(customer_id.to_string());
    }

    let customer_id = repo
        .find_payment_method_customer_id(method_id)
        .await?
        .ok_or_else(|| BillingError::not_found("payment_method", method_id))?;

    if customer_id != scoped_customer_id {
        return Err(BillingError::Forbidden);
    }

    Ok(customer_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rust_decimal::Decimal;
    use rustbill_core::db::models::{
        CustomerCredit, PaymentProvider, SavedPaymentMethod, SavedPaymentMethodStatus,
        SavedPaymentMethodType,
    };
    use std::sync::Mutex;

    struct MockRepo {
        customer_id: Option<String>,
        list_customer_id: Mutex<Option<String>>,
        created_payment_method: Mutex<Option<CreatePaymentMethodRequestV1>>,
        removed: Mutex<Option<(String, String)>>,
        set_default: Mutex<Option<(String, String)>>,
        stripe_args: Mutex<Option<(String, String, String)>>,
        xendit_args: Mutex<Option<String>>,
        balance: Decimal,
        history: Vec<CustomerCredit>,
    }

    impl MockRepo {
        fn new(customer_id: Option<String>) -> Self {
            Self {
                customer_id,
                list_customer_id: Mutex::new(None),
                created_payment_method: Mutex::new(None),
                removed: Mutex::new(None),
                set_default: Mutex::new(None),
                stripe_args: Mutex::new(None),
                xendit_args: Mutex::new(None),
                balance: Decimal::new(150, 0),
                history: vec![],
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
    impl BillingRepository for MockRepo {
        async fn list_payment_methods(
            &self,
            customer_id: &str,
        ) -> std::result::Result<Vec<SavedPaymentMethod>, BillingError> {
            *self.list_customer_id.lock().expect("lock poisoned") = Some(customer_id.to_string());
            Ok(vec![Self::sample_method("pm-1")])
        }

        async fn create_payment_method(
            &self,
            req: &CreatePaymentMethodRequestV1,
        ) -> std::result::Result<SavedPaymentMethod, BillingError> {
            *self.created_payment_method.lock().expect("lock poisoned") = Some(req.clone());
            Ok(Self::sample_method("pm-created"))
        }

        async fn find_payment_method_customer_id(
            &self,
            _method_id: &str,
        ) -> std::result::Result<Option<String>, BillingError> {
            Ok(self.customer_id.clone())
        }

        async fn remove_payment_method(
            &self,
            customer_id: &str,
            method_id: &str,
        ) -> std::result::Result<(), BillingError> {
            *self.removed.lock().expect("lock poisoned") =
                Some((customer_id.to_string(), method_id.to_string()));
            Ok(())
        }

        async fn set_default_payment_method(
            &self,
            customer_id: &str,
            method_id: &str,
        ) -> std::result::Result<SavedPaymentMethod, BillingError> {
            *self.set_default.lock().expect("lock poisoned") =
                Some((customer_id.to_string(), method_id.to_string()));
            Ok(Self::sample_method(method_id))
        }

        async fn create_stripe_setup_session(
            &self,
            customer_id: &str,
            success_url: &str,
            cancel_url: &str,
        ) -> std::result::Result<serde_json::Value, BillingError> {
            *self.stripe_args.lock().expect("lock poisoned") = Some((
                customer_id.to_string(),
                success_url.to_string(),
                cancel_url.to_string(),
            ));
            Ok(serde_json::json!({
                "provider": "stripe",
                "customerId": customer_id,
                "setupUrl": "https://example.test/stripe",
                "sessionId": "sess_123",
            }))
        }

        async fn create_xendit_setup_session(
            &self,
            customer_id: &str,
        ) -> std::result::Result<serde_json::Value, BillingError> {
            *self.xendit_args.lock().expect("lock poisoned") = Some(customer_id.to_string());
            Ok(serde_json::json!({
                "provider": "xendit",
                "customerId": customer_id,
                "setupId": "pm_123",
                "actions": ["redirect"],
            }))
        }

        async fn get_credit_balance(
            &self,
            _customer_id: &str,
            _currency: &str,
        ) -> std::result::Result<Decimal, BillingError> {
            Ok(self.balance)
        }

        async fn list_credits(
            &self,
            _customer_id: &str,
            _currency: Option<&str>,
        ) -> std::result::Result<Vec<CustomerCredit>, BillingError> {
            Ok(self.history.clone())
        }
    }

    #[tokio::test]
    async fn list_payment_methods_uses_scoped_customer() {
        let repo = MockRepo::new(Some("cust-1".to_string()));
        let api_key = ApiKeyInfo {
            id: "key-1".to_string(),
            name: "key".to_string(),
            customer_id: Some("cust-1".to_string()),
        };

        let rows = list_payment_methods(&repo, &api_key, Some("cust-1"))
            .await
            .expect("list should succeed");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            repo.list_customer_id
                .lock()
                .expect("lock poisoned")
                .as_deref(),
            Some("cust-1")
        );
    }

    #[tokio::test]
    async fn delete_payment_method_maps_zero_scope_to_not_found() {
        let repo = MockRepo::new(None);
        let api_key = ApiKeyInfo {
            id: "key-1".to_string(),
            name: "key".to_string(),
            customer_id: Some("cust-1".to_string()),
        };

        let result = delete_payment_method(&repo, &api_key, "pm-1", None).await;
        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "payment_method", id }) if id == "pm-1")
        );
    }

    #[tokio::test]
    async fn create_payment_method_setup_uses_default_urls() {
        let repo = MockRepo::new(Some("cust-1".to_string()));
        let api_key = ApiKeyInfo {
            id: "key-1".to_string(),
            name: "key".to_string(),
            customer_id: Some("cust-1".to_string()),
        };
        let body = PaymentMethodSetupRequestV1 {
            customer_id: "cust-1".to_string(),
            provider: PaymentProvider::Stripe,
            success_url: None,
            cancel_url: None,
        };

        let response = create_payment_method_setup(&repo, &api_key, &body)
            .await
            .expect("setup should succeed");
        assert_eq!(response["provider"], serde_json::json!("stripe"));
        let captured = repo.stripe_args.lock().expect("lock poisoned").clone();
        assert_eq!(
            captured,
            Some((
                "cust-1".to_string(),
                crate::routes::v1::billing::schema::DEFAULT_STRIPE_SETUP_SUCCESS_URL.to_string(),
                crate::routes::v1::billing::schema::DEFAULT_STRIPE_SETUP_CANCEL_URL.to_string(),
            ))
        );
    }

    #[tokio::test]
    async fn get_credits_wraps_balance_and_history() {
        let repo = MockRepo::new(Some("cust-1".to_string()));
        let api_key = ApiKeyInfo {
            id: "key-1".to_string(),
            name: "key".to_string(),
            customer_id: Some("cust-1".to_string()),
        };
        let query = CreditsQueryV1 {
            customer_id: Some("cust-1".to_string()),
            currency: None,
        };

        let response = get_credits(&repo, &api_key, &query)
            .await
            .expect("credits should succeed");
        assert_eq!(response["currency"], serde_json::json!("USD"));
        assert_eq!(response["balance"], serde_json::json!("150"));
        assert!(response["history"].is_array());
    }
}
