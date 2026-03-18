mod common;

use common::{create_admin_session, create_test_customer, test_server};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_deposit_and_get_balance(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    let resp = server
        .post("/api/billing/credits/adjust")
        .json(&json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "50.00",
            "description": "Manual credit"
        }))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;
    resp.assert_status_ok();

    let resp = server
        .get(&format!("/api/billing/credits/{customer_id}?currency=USD"))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["balance"], "50.00");
    assert_eq!(body["history"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_deposit_rejects_negative(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    let resp = server
        .post("/api/billing/credits/adjust")
        .json(&json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "-10.00",
            "description": "Should fail"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_apply_credits_concurrent_no_overdraw(pool: PgPool) {
    let customer_id = create_test_customer(&pool).await;
    let invoice_id = common::create_test_invoice(&pool, &customer_id).await;

    rustbill_core::billing::credits::deposit(
        &pool,
        &customer_id,
        "USD",
        Decimal::new(1000, 2),
        rustbill_core::db::models::CreditReason::Manual,
        "Seed credits",
        None,
    )
    .await
    .unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));

    let pool1 = pool.clone();
    let c1 = customer_id.clone();
    let i1 = invoice_id.clone();
    let b1 = barrier.clone();
    let t1 = tokio::spawn(async move {
        let mut tx = pool1.begin().await.unwrap();
        b1.wait().await;
        let applied = rustbill_core::billing::credits::apply_to_invoice(
            &mut tx,
            &c1,
            &i1,
            "USD",
            Decimal::new(1000, 2),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        applied
    });

    let pool2 = pool.clone();
    let c2 = customer_id.clone();
    let i2 = invoice_id.clone();
    let b2 = barrier.clone();
    let t2 = tokio::spawn(async move {
        let mut tx = pool2.begin().await.unwrap();
        b2.wait().await;
        let applied = rustbill_core::billing::credits::apply_to_invoice(
            &mut tx,
            &c2,
            &i2,
            "USD",
            Decimal::new(1000, 2),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        applied
    });

    let a1 = t1.await.unwrap();
    let a2 = t2.await.unwrap();

    assert_eq!(a1 + a2, Decimal::new(1000, 2));

    let balance: Decimal = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = 'USD'",
    )
    .bind(&customer_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(balance, Decimal::ZERO);
}
