mod common;

use common::{
    create_admin_session, create_test_customer, create_test_plan, create_test_product,
    create_test_subscription, test_server,
};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn usage_admin_crud_and_summary(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "api").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let sub_id = create_test_subscription(&pool, &customer_id, &plan_id).await;

    let create_resp = server
        .post("/api/billing/usage")
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "subscriptionId": sub_id,
            "metricName": "api_calls",
            "value": 12
        }))
        .await;
    create_resp.assert_status(axum::http::StatusCode::CREATED);
    let created: serde_json::Value = create_resp.json();
    let usage_id = created["id"].as_str().unwrap().to_string();

    let list_resp = server
        .get(&format!("/api/billing/usage?subscriptionId={sub_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;
    list_resp.assert_status_ok();
    let list_body: Vec<serde_json::Value> = list_resp.json();
    assert!(list_body
        .iter()
        .any(|row| row["id"].as_str() == Some(usage_id.as_str())));

    let update_resp = server
        .put(&format!("/api/billing/usage/{usage_id}"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .json(&json!({
            "value": 25
        }))
        .await;
    update_resp.assert_status_ok();
    let updated: serde_json::Value = update_resp.json();
    let updated_value = updated["value"].as_f64().unwrap_or_default();
    assert!((updated_value - 25.0).abs() < f64::EPSILON);

    let summary_resp = server
        .get(&format!("/api/billing/usage/{sub_id}/summary"))
        .add_cookie(cookie::Cookie::new("session", token.clone()))
        .await;
    summary_resp.assert_status_ok();
    let summary: serde_json::Value = summary_resp.json();
    assert_eq!(summary["subscriptionId"].as_str(), Some(sub_id.as_str()));
    assert!(summary["metrics"].is_array());

    let delete_resp = server
        .delete(&format!("/api/billing/usage/{usage_id}"))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    delete_resp.assert_status_ok();
    let deleted: serde_json::Value = delete_resp.json();
    assert_eq!(deleted["success"], json!(true));
}
