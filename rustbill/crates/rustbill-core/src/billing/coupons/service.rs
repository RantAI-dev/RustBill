use super::repository::CouponsRepository;
use super::schema::{ApplyCouponRequest, CouponView, CreateCouponRequest, UpdateCouponRequest};
use crate::db::models::{Coupon, SubscriptionDiscount};
use crate::error::{BillingError, Result};
use validator::Validate;

pub async fn list_coupons<R: CouponsRepository + ?Sized>(repo: &R) -> Result<Vec<CouponView>> {
    repo.list_coupons().await
}

pub async fn get_coupon<R: CouponsRepository + ?Sized>(repo: &R, id: &str) -> Result<Coupon> {
    repo.get_coupon(id)
        .await?
        .ok_or_else(|| BillingError::not_found("coupon", id))
}

pub async fn create_coupon<R: CouponsRepository + ?Sized>(
    repo: &R,
    req: CreateCouponRequest,
) -> Result<Coupon> {
    req.validate().map_err(BillingError::from_validation)?;

    if repo.find_coupon_by_code(&req.code).await?.is_some() {
        return Err(BillingError::conflict(format!(
            "coupon code '{}' already exists",
            req.code
        )));
    }

    repo.create_coupon(&req).await
}

pub async fn update_coupon<R: CouponsRepository + ?Sized>(
    repo: &R,
    id: &str,
    req: UpdateCouponRequest,
) -> Result<Coupon> {
    req.validate().map_err(BillingError::from_validation)?;

    let _existing = get_coupon(repo, id).await?;
    repo.update_coupon(id, &req).await
}

pub async fn delete_coupon<R: CouponsRepository + ?Sized>(repo: &R, id: &str) -> Result<()> {
    let affected = repo.delete_coupon(id).await?;
    if affected == 0 {
        return Err(BillingError::not_found("coupon", id));
    }
    Ok(())
}

pub async fn apply_coupon<R: CouponsRepository + ?Sized>(
    repo: &R,
    req: ApplyCouponRequest,
) -> Result<SubscriptionDiscount> {
    let subscription = repo
        .find_subscription(&req.subscription_id)
        .await?
        .ok_or_else(|| BillingError::not_found("subscription", &req.subscription_id))?;
    let coupon = get_coupon(repo, &req.coupon_id).await?;

    if !coupon.active {
        return Err(BillingError::bad_request("coupon is not active"));
    }

    if let Some(max) = coupon.max_redemptions {
        if coupon.times_redeemed >= max {
            return Err(BillingError::bad_request(
                "coupon has reached max redemptions",
            ));
        }
    }

    if repo
        .subscription_has_coupon(&req.subscription_id, &req.coupon_id)
        .await?
    {
        return Err(BillingError::conflict(
            "coupon already applied to this subscription",
        ));
    }

    repo.apply_coupon(&subscription, &coupon, req.expires_at)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        Coupon, DiscountType, Subscription, SubscriptionDiscount, SubscriptionStatus,
    };
    use async_trait::async_trait;
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;
    use std::sync::{Arc, Mutex};

    #[derive(Default, Clone)]
    struct StubState {
        coupon_by_id: Option<Coupon>,
        coupon_by_code: Option<Coupon>,
        subscription: Option<Subscription>,
        applied_exists: bool,
        list_rows: Vec<CouponView>,
        created: Option<CreateCouponRequest>,
        updated: Option<(String, UpdateCouponRequest)>,
        deleted_rows: u64,
        applied: Option<(String, String)>,
        discount: Option<SubscriptionDiscount>,
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

        fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, StubState>> {
            self.state
                .lock()
                .map_err(|_| BillingError::Internal(anyhow::anyhow!("mutex poisoned")))
        }
    }

    fn dt(y: i32, m: u32, d: u32) -> chrono::NaiveDateTime {
        let date = match NaiveDate::from_ymd_opt(y, m, d) {
            Some(date) => date,
            None => panic!("invalid test date"),
        };
        match date.and_hms_opt(0, 0, 0) {
            Some(dt) => dt,
            None => panic!("invalid test time"),
        }
    }

    fn sample_coupon(active: bool, times_redeemed: i32) -> Coupon {
        Coupon {
            id: "coupon_1".to_string(),
            code: "SAVE10".to_string(),
            name: "Save 10".to_string(),
            discount_type: DiscountType::Percentage,
            discount_value: Decimal::from(10),
            currency: "USD".to_string(),
            max_redemptions: Some(5),
            times_redeemed,
            valid_from: dt(2026, 1, 1),
            valid_until: None,
            active,
            applies_to: None,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_subscription() -> Subscription {
        Subscription {
            id: "sub_1".to_string(),
            customer_id: "cus_1".to_string(),
            plan_id: "plan_1".to_string(),
            status: SubscriptionStatus::Active,
            current_period_start: Utc::now().naive_utc(),
            current_period_end: Utc::now().naive_utc(),
            canceled_at: None,
            cancel_at_period_end: false,
            trial_end: None,
            quantity: 1,
            metadata: None,
            stripe_subscription_id: None,
            managed_by: None,
            version: 1,
            deleted_at: None,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    fn sample_discount() -> SubscriptionDiscount {
        SubscriptionDiscount {
            id: "sd_1".to_string(),
            subscription_id: "sub_1".to_string(),
            coupon_id: "coupon_1".to_string(),
            applied_at: Utc::now().naive_utc(),
            expires_at: None,
        }
    }

    #[async_trait]
    impl CouponsRepository for StubRepo {
        async fn list_coupons(&self) -> Result<Vec<CouponView>> {
            Ok(self.lock_state()?.list_rows.clone())
        }

        async fn get_coupon(&self, id: &str) -> Result<Option<Coupon>> {
            let state = self.lock_state()?;
            Ok(state.coupon_by_id.clone().filter(|coupon| coupon.id == id))
        }

        async fn find_coupon_by_code(&self, code: &str) -> Result<Option<Coupon>> {
            let state = self.lock_state()?;
            Ok(state
                .coupon_by_code
                .clone()
                .filter(|coupon| coupon.code == code))
        }

        async fn create_coupon(&self, req: &CreateCouponRequest) -> Result<Coupon> {
            self.lock_state()?.created = Some(req.clone());
            Ok(sample_coupon(true, 0))
        }

        async fn update_coupon(&self, id: &str, req: &UpdateCouponRequest) -> Result<Coupon> {
            let mut state = self.lock_state()?;
            state.updated = Some((id.to_string(), req.clone()));
            Ok(sample_coupon(true, 1))
        }

        async fn delete_coupon(&self, _id: &str) -> Result<u64> {
            Ok(self.lock_state()?.deleted_rows)
        }

        async fn find_subscription(&self, _id: &str) -> Result<Option<Subscription>> {
            Ok(self.lock_state()?.subscription.clone())
        }

        async fn subscription_has_coupon(
            &self,
            _subscription_id: &str,
            _coupon_id: &str,
        ) -> Result<bool> {
            Ok(self.lock_state()?.applied_exists)
        }

        async fn apply_coupon(
            &self,
            _subscription: &Subscription,
            _coupon: &Coupon,
            _expires_at: Option<chrono::NaiveDateTime>,
        ) -> Result<SubscriptionDiscount> {
            let mut state = self.lock_state()?;
            state.applied = Some(("sub_1".to_string(), "coupon_1".to_string()));
            Ok(match state.discount.clone() {
                Some(discount) => discount,
                None => sample_discount(),
            })
        }
    }

    #[tokio::test]
    async fn create_coupon_rejects_duplicates() {
        let repo = StubRepo::with_state(StubState {
            coupon_by_code: Some(sample_coupon(true, 0)),
            ..StubState::default()
        });

        let err = create_coupon(
            &repo,
            CreateCouponRequest {
                code: "SAVE10".to_string(),
                name: "Save 10".to_string(),
                discount_type: DiscountType::Percentage,
                discount_value: Decimal::from(10),
                currency: None,
                max_redemptions: Some(5),
                valid_from: dt(2026, 1, 1),
                valid_until: None,
                active: true,
                applies_to: None,
            },
        )
        .await
        .err();

        let err = match err {
            Some(err) => err,
            None => panic!("expected conflict"),
        };
        assert!(matches!(err, BillingError::Conflict(_)));
    }

    #[tokio::test]
    async fn update_coupon_maps_missing_to_not_found() {
        let repo = StubRepo::default();

        let err = update_coupon(
            &repo,
            "coupon_1",
            UpdateCouponRequest {
                name: Some("Updated".to_string()),
                discount_type: None,
                discount_value: None,
                currency: None,
                max_redemptions: None,
                valid_until: None,
                active: None,
                applies_to: None,
            },
        )
        .await
        .err();

        let err = match err {
            Some(err) => err,
            None => panic!("expected not found"),
        };
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "coupon",
                id
            } if id == "coupon_1"
        ));
    }

    #[tokio::test]
    async fn delete_coupon_maps_zero_rows_to_not_found() {
        let repo = StubRepo::with_state(StubState {
            deleted_rows: 0,
            ..StubState::default()
        });

        let err = delete_coupon(&repo, "coupon_1").await.err();

        let err = match err {
            Some(err) => err,
            None => panic!("expected not found"),
        };
        assert!(matches!(
            err,
            BillingError::NotFound {
                entity: "coupon",
                id
            } if id == "coupon_1"
        ));
    }

    #[tokio::test]
    async fn apply_coupon_returns_conflict_when_already_applied() {
        let repo = StubRepo::with_state(StubState {
            coupon_by_id: Some(sample_coupon(true, 0)),
            subscription: Some(sample_subscription()),
            applied_exists: true,
            ..StubState::default()
        });

        let err = apply_coupon(
            &repo,
            ApplyCouponRequest {
                subscription_id: "sub_1".to_string(),
                coupon_id: "coupon_1".to_string(),
                expires_at: None,
            },
        )
        .await
        .err();

        let err = match err {
            Some(err) => err,
            None => panic!("expected conflict"),
        };
        assert!(matches!(err, BillingError::Conflict(_)));
    }
}
