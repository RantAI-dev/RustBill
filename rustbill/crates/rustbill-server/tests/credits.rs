mod common;

use common::{create_admin_session, create_test_customer, test_server};
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
