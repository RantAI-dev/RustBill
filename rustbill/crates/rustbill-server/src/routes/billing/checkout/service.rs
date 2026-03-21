use super::repository::CheckoutRepository;
use super::schema::{CheckoutQuery, CheckoutResult};
use rustbill_core::error::BillingError;

pub async fn get_checkout<R: CheckoutRepository>(
    repo: &R,
    query: &CheckoutQuery,
    origin: &str,
) -> Result<CheckoutResult, BillingError> {
    let provider = query.provider.as_deref().unwrap_or("stripe");
    repo.create_checkout(&query.invoice_id, provider, origin)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use rustbill_core::error::BillingError;
    use std::sync::Mutex;

    struct MockCheckoutRepository {
        captured: Mutex<Option<(String, String, String)>>,
    }

    impl MockCheckoutRepository {
        fn new() -> Self {
            Self {
                captured: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl CheckoutRepository for MockCheckoutRepository {
        async fn create_checkout(
            &self,
            invoice_id: &str,
            provider: &str,
            origin: &str,
        ) -> Result<CheckoutResult, BillingError> {
            if let Ok(mut guard) = self.captured.lock() {
                *guard = Some((
                    invoice_id.to_string(),
                    provider.to_string(),
                    origin.to_string(),
                ));
            }
            Ok(CheckoutResult {
                invoice_id: invoice_id.to_string(),
                provider: provider.to_string(),
                checkout_url: "https://checkout.example".into(),
            })
        }
    }

    #[tokio::test]
    async fn defaults_provider_to_stripe() {
        let repo = MockCheckoutRepository::new();
        let result = get_checkout(
            &repo,
            &CheckoutQuery {
                invoice_id: "inv-1".into(),
                provider: None,
            },
            "https://app.example",
        )
        .await
        .expect("checkout should succeed");

        assert_eq!(result.provider, "stripe");
        let captured = repo.captured.lock().expect("capture");
        let captured = captured.as_ref().expect("captured checkout");
        assert_eq!(captured.1, "stripe");
    }
}
