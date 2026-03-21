use super::repository::{CustomerCreateParams, CustomerRepository, CustomerUpdateParams};
use super::schema::{CreateCustomerRequest, DeleteCustomerResponse, UpdateCustomerRequest};
use rustbill_core::db::models::Customer;
use rustbill_core::error::BillingError;

pub async fn list<R: CustomerRepository>(repo: &R) -> Result<Vec<Customer>, BillingError> {
    repo.list().await
}

pub async fn get<R: CustomerRepository>(repo: &R, id: &str) -> Result<Customer, BillingError> {
    repo.get(id).await
}

pub async fn create<R: CustomerRepository>(
    repo: &R,
    body: &CreateCustomerRequest,
) -> Result<Customer, BillingError> {
    let params = CustomerCreateParams {
        name: body.name.clone().unwrap_or_default(),
        industry: body.industry.clone().unwrap_or_default(),
        tier: body.tier.clone().unwrap_or_else(|| "standard".to_string()),
        location: body.location.clone().unwrap_or_default(),
        contact: body.contact.clone().unwrap_or_default(),
        email: body.email.clone().unwrap_or_default(),
        phone: body.phone.clone().unwrap_or_default(),
        billing_email: body.billing_email.clone(),
        billing_address: body.billing_address.clone(),
        billing_city: body.billing_city.clone(),
        billing_state: body.billing_state.clone(),
        billing_zip: body.billing_zip.clone(),
        billing_country: body.billing_country.clone(),
        tax_id: body.tax_id.clone(),
        default_payment_method: body.default_payment_method.clone(),
        stripe_customer_id: body.stripe_customer_id.clone(),
        xendit_customer_id: body.xendit_customer_id.clone(),
    };

    repo.create(&params).await
}

pub async fn update<R: CustomerRepository>(
    repo: &R,
    id: &str,
    body: &UpdateCustomerRequest,
) -> Result<Customer, BillingError> {
    let params = CustomerUpdateParams {
        name: body.name.clone(),
        industry: body.industry.clone(),
        tier: body.tier.clone(),
        location: body.location.clone(),
        contact: body.contact.clone(),
        email: body.email.clone(),
        phone: body.phone.clone(),
        billing_email: body.billing_email.clone(),
        billing_address: body.billing_address.clone(),
        billing_city: body.billing_city.clone(),
        billing_state: body.billing_state.clone(),
        billing_zip: body.billing_zip.clone(),
        billing_country: body.billing_country.clone(),
        tax_id: body.tax_id.clone(),
        default_payment_method: body.default_payment_method.clone(),
        stripe_customer_id: body.stripe_customer_id.clone(),
        xendit_customer_id: body.xendit_customer_id.clone(),
    };

    repo.update(id, &params).await
}

pub async fn delete<R: CustomerRepository>(
    repo: &R,
    id: &str,
) -> Result<DeleteCustomerResponse, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("customer", id));
    }

    Ok(DeleteCustomerResponse { success: true })
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::db::models::{Customer, PaymentMethod, Trend};
    use rustbill_core::error::BillingError;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct MockCustomerRepository {
        create_params: Mutex<Option<CustomerCreateParams>>,
        update_params: Mutex<Option<CustomerUpdateParams>>,
        delete_rows: Mutex<u64>,
        delete_called: AtomicBool,
    }

    impl MockCustomerRepository {
        fn new(delete_rows: u64) -> Self {
            Self {
                create_params: Mutex::new(None),
                update_params: Mutex::new(None),
                delete_rows: Mutex::new(delete_rows),
                delete_called: AtomicBool::new(false),
            }
        }

        fn sample_customer(id: &str) -> Customer {
            Customer {
                id: id.to_string(),
                name: "Sample".to_string(),
                industry: "Tech".to_string(),
                tier: "standard".to_string(),
                location: "US".to_string(),
                contact: "Ops".to_string(),
                email: "ops@example.com".to_string(),
                phone: "+1".to_string(),
                total_revenue: rust_decimal::Decimal::ZERO,
                health_score: 50,
                trend: Trend::Stable,
                last_contact: String::new(),
                billing_email: None,
                billing_address: None,
                billing_city: None,
                billing_state: None,
                billing_zip: None,
                billing_country: None,
                tax_id: None,
                default_payment_method: Some(PaymentMethod::Stripe),
                stripe_customer_id: None,
                xendit_customer_id: None,
                created_at: chrono::NaiveDateTime::MIN,
                updated_at: chrono::NaiveDateTime::MIN,
            }
        }
    }

    #[async_trait]
    impl CustomerRepository for MockCustomerRepository {
        async fn list(&self) -> Result<Vec<Customer>, BillingError> {
            Ok(vec![Self::sample_customer("cust-1")])
        }

        async fn get(&self, id: &str) -> Result<Customer, BillingError> {
            Ok(Self::sample_customer(id))
        }

        async fn create(&self, body: &CustomerCreateParams) -> Result<Customer, BillingError> {
            if let Ok(mut guard) = self.create_params.lock() {
                *guard = Some(body.clone());
            }
            Ok(Self::sample_customer("cust-created"))
        }

        async fn update(
            &self,
            _id: &str,
            body: &CustomerUpdateParams,
        ) -> Result<Customer, BillingError> {
            if let Ok(mut guard) = self.update_params.lock() {
                *guard = Some(body.clone());
            }
            Ok(Self::sample_customer("cust-updated"))
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            self.delete_called.store(true, Ordering::SeqCst);
            if let Ok(guard) = self.delete_rows.lock() {
                Ok(*guard)
            } else {
                Ok(0)
            }
        }
    }

    #[tokio::test]
    async fn create_applies_legacy_defaults() {
        let repo = MockCustomerRepository::new(1);
        let created = create(&repo, &CreateCustomerRequest::default())
            .await
            .expect("customer should be created");

        assert_eq!(created.id, "cust-created");
        let captured = repo.create_params.lock().expect("capture");
        let params = captured.as_ref().expect("create params");
        assert_eq!(params.name, "");
        assert_eq!(params.industry, "");
        assert_eq!(params.tier, "standard");
        assert_eq!(params.location, "");
        assert_eq!(params.contact, "");
        assert_eq!(params.email, "");
        assert_eq!(params.phone, "");
        assert!(params.billing_email.is_none());
    }

    #[tokio::test]
    async fn update_passes_partial_fields_through() {
        let repo = MockCustomerRepository::new(1);
        let body = UpdateCustomerRequest {
            name: Some("Updated".to_string()),
            billing_email: Some("billing@example.com".to_string()),
            ..Default::default()
        };

        let updated = update(&repo, "cust-1", &body)
            .await
            .expect("customer update");
        assert_eq!(updated.id, "cust-updated");

        let captured = repo.update_params.lock().expect("capture");
        let params = captured.as_ref().expect("update params");
        assert_eq!(params.name.as_deref(), Some("Updated"));
        assert_eq!(params.billing_email.as_deref(), Some("billing@example.com"));
    }

    #[tokio::test]
    async fn delete_returns_not_found_when_no_rows_are_affected() {
        let repo = MockCustomerRepository::new(0);
        let err = delete(&repo, "cust-missing").await.unwrap_err();

        match err {
            BillingError::NotFound { entity, id } => {
                assert_eq!(entity, "customer");
                assert_eq!(id, "cust-missing");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
