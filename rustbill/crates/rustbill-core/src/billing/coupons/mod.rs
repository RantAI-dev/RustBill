pub mod repository;
pub mod schema;
pub mod service;

use crate::db::models::{Coupon, SubscriptionDiscount};
use crate::error::Result;
use repository::PgCouponsRepository;
use sqlx::PgPool;

pub use schema::{ApplyCouponRequest, CouponView, CreateCouponRequest, UpdateCouponRequest};

pub async fn list_coupons(pool: &PgPool) -> Result<Vec<CouponView>> {
    let repo = PgCouponsRepository::new(pool);
    service::list_coupons(&repo).await
}

pub async fn get_coupon(pool: &PgPool, id: &str) -> Result<Coupon> {
    let repo = PgCouponsRepository::new(pool);
    service::get_coupon(&repo, id).await
}

pub async fn create_coupon(pool: &PgPool, req: CreateCouponRequest) -> Result<Coupon> {
    let repo = PgCouponsRepository::new(pool);
    service::create_coupon(&repo, req).await
}

pub async fn update_coupon(pool: &PgPool, id: &str, req: UpdateCouponRequest) -> Result<Coupon> {
    let repo = PgCouponsRepository::new(pool);
    service::update_coupon(&repo, id, req).await
}

pub async fn delete_coupon(pool: &PgPool, id: &str) -> Result<()> {
    let repo = PgCouponsRepository::new(pool);
    service::delete_coupon(&repo, id).await
}

pub async fn apply_coupon(
    pool: &PgPool,
    subscription_id: &str,
    coupon_id: &str,
    expires_at: Option<chrono::NaiveDateTime>,
) -> Result<SubscriptionDiscount> {
    let repo = PgCouponsRepository::new(pool);
    service::apply_coupon(
        &repo,
        ApplyCouponRequest {
            subscription_id: subscription_id.to_string(),
            coupon_id: coupon_id.to_string(),
            expires_at,
        },
    )
    .await
}
