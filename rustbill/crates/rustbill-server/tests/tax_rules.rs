mod common;

use common::{create_admin_session, test_server};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_calculate_tax_exclusive(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let resp = server
        .post("/api/billing/tax-rules")
        .json(&serde_json::json!({
            "country": "US",
            "region": "NY",
            "taxName": "Sales Tax",
            "rate": "0.0800",
            "inclusive": false
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let rule: serde_json::Value = resp.json();
    assert_eq!(rule["country"], "US");
    assert_eq!(rule["region"], "NY");
    assert_eq!(rule["inclusive"], false);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_tax_rules(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let resp = server
        .get("/api/billing/tax-rules")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let rules: Vec<serde_json::Value> = resp.json();
    assert!(rules.len() >= 6);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_tax_rule_creates_new_version(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    let resp = server
        .post("/api/billing/tax-rules")
        .json(&serde_json::json!({
            "country": "JP",
            "taxName": "Consumption Tax",
            "rate": "0.1000",
            "inclusive": true
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let old_rule: serde_json::Value = resp.json();
    let old_id = old_rule["id"].as_str().unwrap();

    let resp = server
        .put(&format!("/api/billing/tax-rules/{old_id}"))
        .json(&serde_json::json!({
            "taxName": "Consumption Tax",
            "rate": "0.0800",
            "inclusive": true
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let new_rule: serde_json::Value = resp.json();
    assert_ne!(new_rule["id"], old_rule["id"]);
    assert_eq!(new_rule["rate"], "0.0800");
}
