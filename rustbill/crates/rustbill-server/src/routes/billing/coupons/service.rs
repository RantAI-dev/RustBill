use super::repository::CouponsRepository;
use super::schema::{CreateCouponRequest, UpdateCouponRequest};
use rustbill_core::error::BillingError;

pub async fn list<R: CouponsRepository>(repo: &R) -> Result<Vec<serde_json::Value>, BillingError> {
    repo.list().await
}

pub async fn get<R: CouponsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    repo.get(id).await
}

pub async fn create<R: CouponsRepository>(
    repo: &R,
    body: &CreateCouponRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.create(body).await
}

pub async fn update<R: CouponsRepository>(
    repo: &R,
    id: &str,
    body: &UpdateCouponRequest,
) -> Result<serde_json::Value, BillingError> {
    repo.update(id, body).await
}

pub async fn delete<R: CouponsRepository>(
    repo: &R,
    id: &str,
) -> Result<serde_json::Value, BillingError> {
    let affected = repo.delete(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("coupon", id));
    }

    Ok(serde_json::json!({ "success": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::billing::coupons::repository::CouponsRepository;
    use async_trait::async_trait;

    struct MockRepo {
        delete_rows: u64,
    }

    impl MockRepo {
        fn new(delete_rows: u64) -> Self {
            Self { delete_rows }
        }
    }

    #[async_trait]
    impl CouponsRepository for MockRepo {
        async fn list(&self) -> Result<Vec<serde_json::Value>, BillingError> {
            Ok(vec![serde_json::json!({ "id": "coupon-1" })])
        }

        async fn get(&self, _id: &str) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "coupon-1" }))
        }

        async fn create(
            &self,
            _body: &CreateCouponRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "coupon-1" }))
        }

        async fn update(
            &self,
            _id: &str,
            _body: &UpdateCouponRequest,
        ) -> Result<serde_json::Value, BillingError> {
            Ok(serde_json::json!({ "id": "coupon-1" }))
        }

        async fn delete(&self, _id: &str) -> Result<u64, BillingError> {
            Ok(self.delete_rows)
        }
    }

    #[tokio::test]
    async fn delete_maps_zero_rows_to_not_found() {
        let repo = MockRepo::new(0);
        let result = delete(&repo, "coupon-1").await;

        assert!(
            matches!(result, Err(BillingError::NotFound { entity: "coupon", id }) if id == "coupon-1")
        );
    }
}
