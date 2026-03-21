mod common;

use axum::http::{HeaderName, HeaderValue};
use common::{
    create_admin_session, create_test_api_key_for_customer, create_test_customer, test_server,
};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn admin_one_time_sales_crud_cycle(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;

    let create_resp = server
        .post("/api/billing/one-time-sales")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "customerId": customer_id,
            "currency": "USD",
            "subtotal": 1000,
            "tax": 100,
            "total": 1100,
            "status": "issued"
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let id = created["id"].as_str().unwrap().to_string();

    let list_resp = server
        .get("/api/billing/one-time-sales")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;
    list_resp.assert_status_ok();
    let listed: Vec<serde_json::Value> = list_resp.json();
    assert!(listed
        .iter()
        .any(|row| row["id"].as_str() == Some(id.as_str())));

    let get_resp = server
        .get(&format!("/api/billing/one-time-sales/{id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;
    get_resp.assert_status_ok();
    let fetched: serde_json::Value = get_resp.json();
    assert_eq!(fetched["id"].as_str(), Some(id.as_str()));

    let update_resp = server
        .put(&format!("/api/billing/one-time-sales/{id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({ "notes": "Updated note" }))
        .await;
    update_resp.assert_status_ok();
    let updated: serde_json::Value = update_resp.json();
    assert_eq!(updated["notes"].as_str(), Some("Updated note"));

    let delete_resp = server
        .delete(&format!("/api/billing/one-time-sales/{id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    delete_resp.assert_status_ok();
    let deleted: serde_json::Value = delete_resp.json();
    assert_eq!(deleted["success"], json!(true));
}

#[sqlx::test(migrations = "../../migrations")]
async fn v1_one_time_sales_scoped_crud_cycle(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let (_api_key_id, key) = create_test_api_key_for_customer(&pool, Some(&customer_id)).await;
    let auth_name = HeaderName::from_static("authorization");
    let auth_value = HeaderValue::from_str(&format!("Bearer {key}")).unwrap();

    let create_resp = server
        .post("/api/v1/billing/one-time-sales")
        .add_header(auth_name.clone(), auth_value.clone())
        .json(&json!({
            "customerId": customer_id,
            "currency": "USD",
            "subtotal": 500,
            "tax": 50,
            "total": 550
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let id = created["id"].as_str().unwrap().to_string();

    let list_resp = server
        .get("/api/v1/billing/one-time-sales")
        .add_header(auth_name.clone(), auth_value.clone())
        .await;
    list_resp.assert_status_ok();
    let listed: Vec<serde_json::Value> = list_resp.json();
    assert!(listed
        .iter()
        .any(|row| row["id"].as_str() == Some(id.as_str())));

    let update_resp = server
        .put(&format!("/api/v1/billing/one-time-sales/{id}"))
        .add_header(auth_name.clone(), auth_value.clone())
        .json(&json!({ "notes": "Scoped update" }))
        .await;
    update_resp.assert_status_ok();
    let updated: serde_json::Value = update_resp.json();
    assert_eq!(updated["notes"].as_str(), Some("Scoped update"));

    let delete_resp = server
        .delete(&format!("/api/v1/billing/one-time-sales/{id}"))
        .add_header(auth_name, auth_value)
        .await;
    delete_resp.assert_status_ok();
    let deleted: serde_json::Value = delete_resp.json();
    assert_eq!(deleted["success"], json!(true));
}

#[sqlx::test(migrations = "../../migrations")]
async fn v1_one_time_sales_rejects_cross_customer_create(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let scoped_customer_id = create_test_customer(&pool).await;
    let other_customer_id = create_test_customer(&pool).await;
    let (_api_key_id, key) =
        create_test_api_key_for_customer(&pool, Some(&scoped_customer_id)).await;
    let auth_name = HeaderName::from_static("authorization");
    let auth_value = HeaderValue::from_str(&format!("Bearer {key}")).unwrap();

    let create_resp = server
        .post("/api/v1/billing/one-time-sales")
        .add_header(auth_name, auth_value)
        .json(&json!({
            "customerId": other_customer_id,
            "currency": "USD",
            "subtotal": 500
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::FORBIDDEN);
}
