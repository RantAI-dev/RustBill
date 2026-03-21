mod common;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use common::{
    create_admin_session, create_test_customer, create_test_plan, create_test_product,
    create_test_subscription, test_server,
};
use cookie::Cookie;
use serde_json::{json, Value};
use sqlx::PgPool;
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn setup(pool: PgPool) -> (axum_test::TestServer, String) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    (server, token)
}

async fn insert_billing_event(pool: &PgPool, resource_id: &str) -> String {
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().naive_utc();

    sqlx::query(
        r#"INSERT INTO billing_events
           (id, event_type, resource_type, resource_id, customer_id, data, created_at)
           VALUES ($1, 'invoice.paid'::billing_event_type, 'invoice', $2, $3, $4, $5)"#,
    )
    .bind(&id)
    .bind(resource_id)
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(json!({ "source": "integration-test" }))
    .bind(now)
    .execute(pool)
    .await
    .expect("failed to insert billing event");

    id
}

async fn spawn_webhook_capture_server() -> (String, mpsc::Receiver<Value>) {
    let (tx, rx) = mpsc::channel(1);

    async fn capture(State(tx): State<mpsc::Sender<Value>>, Json(body): Json<Value>) -> StatusCode {
        tx.send(body)
            .await
            .expect("failed to capture webhook payload");
        StatusCode::NO_CONTENT
    }

    let app = Router::new().route("/", post(capture)).with_state(tx);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind capture server");
    let addr = listener.local_addr().expect("failed to read local addr");

    let _handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .expect("capture server exited unexpectedly");
    });

    (format!("http://{addr}/"), rx)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrations = "../../migrations")]
async fn billing_events_admin_list_and_get(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let resource_id = uuid::Uuid::new_v4().to_string();
    let event_id = insert_billing_event(&pool, &resource_id).await;

    let list_resp = server
        .get("/api/billing/events")
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    list_resp.assert_status_ok();
    let list_body: Value = list_resp.json();

    assert_eq!(list_body["total"], json!(1));
    assert_eq!(list_body["limit"], json!(50));
    assert_eq!(list_body["offset"], json!(0));

    let rows = list_body["data"]
        .as_array()
        .expect("data should be an array");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["id"], json!(event_id));
    assert_eq!(rows[0]["event_type"], json!("invoice.paid"));
    assert_eq!(rows[0]["resource_id"], json!(resource_id));

    let get_resp = server
        .get(&format!("/api/billing/events/{event_id}"))
        .add_cookie(Cookie::new("session", token))
        .await;

    get_resp.assert_status_ok();
    let get_body: Value = get_resp.json();
    assert_eq!(get_body["id"], json!(event_id));
    assert_eq!(get_body["event_type"], json!("invoice.paid"));
    assert_eq!(get_body["resource_id"], json!(resource_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_plans_admin_crud_cycle(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let product_id = create_test_product(&pool, "saas").await;

    let create_resp = server
        .post("/api/billing/plans")
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "productId": product_id,
            "name": "Pro Plan",
            "pricingModel": "flat",
            "billingCycle": "monthly",
            "basePrice": 49.99,
            "trialDays": 14,
            "active": true
        }))
        .await;

    create_resp.assert_status(StatusCode::CREATED);
    let created: Value = create_resp.json();
    let plan_id = created["id"]
        .as_str()
        .expect("plan id should exist")
        .to_string();

    assert_eq!(created["product_id"], json!(product_id));
    assert_eq!(created["name"], json!("Pro Plan"));
    assert_eq!(created["pricing_model"], json!("flat"));
    assert_eq!(created["billing_cycle"], json!("monthly"));
    assert_eq!(created["active"], json!(true));

    let list_resp = server
        .get("/api/billing/plans")
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    list_resp.assert_status_ok();
    let list_body: Vec<Value> = list_resp.json();
    assert_eq!(list_body.len(), 1);
    assert_eq!(list_body[0]["id"], json!(plan_id));

    let get_resp = server
        .get(&format!("/api/billing/plans/{plan_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    get_resp.assert_status_ok();
    let fetched: Value = get_resp.json();
    assert_eq!(fetched["id"], json!(plan_id));
    assert_eq!(fetched["name"], json!("Pro Plan"));

    let update_resp = server
        .put(&format!("/api/billing/plans/{plan_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "name": "Pro Plan v2",
            "basePrice": 59.99,
            "active": false
        }))
        .await;

    update_resp.assert_status_ok();
    let updated: Value = update_resp.json();
    assert_eq!(updated["id"], json!(plan_id));
    assert_eq!(updated["name"], json!("Pro Plan v2"));
    assert_eq!(updated["active"], json!(false));

    let delete_resp = server
        .delete(&format!("/api/billing/plans/{plan_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    delete_resp.assert_status_ok();
    let deleted: Value = delete_resp.json();
    assert_eq!(deleted["success"], json!(true));

    let missing_resp = server
        .get(&format!("/api/billing/plans/{plan_id}"))
        .add_cookie(Cookie::new("session", token))
        .await;

    missing_resp.assert_status(StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_dunning_admin_list_create_and_get(pool: PgPool) {
    let (server, token) = setup(pool.clone()).await;
    let customer_id = create_test_customer(&pool).await;
    let product_id = create_test_product(&pool, "saas").await;
    let plan_id = create_test_plan(&pool, &product_id).await;
    let subscription_id = create_test_subscription(&pool, &customer_id, &plan_id).await;
    let invoice_id = common::create_test_invoice(&pool, &customer_id).await;
    let scheduled_at = chrono::Utc::now().naive_utc().to_string();

    let create_resp = server
        .post("/api/billing/dunning")
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "invoiceId": invoice_id,
            "subscriptionId": subscription_id,
            "step": "warning",
            "scheduledAt": scheduled_at,
            "notes": "integration-test"
        }))
        .await;

    create_resp.assert_status(StatusCode::CREATED);
    let created: Value = create_resp.json();
    let dunning_id = created["id"]
        .as_str()
        .expect("dunning id should exist")
        .to_string();

    assert_eq!(created["invoice_id"], json!(invoice_id));
    assert_eq!(created["subscription_id"], json!(subscription_id));
    assert_eq!(created["step"], json!("warning"));
    assert_eq!(created["notes"], json!("integration-test"));

    let list_resp = server
        .get(&format!("/api/billing/dunning?invoice_id={invoice_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    list_resp.assert_status_ok();
    let listed: Vec<Value> = list_resp.json();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], json!(dunning_id));
    assert_eq!(listed[0]["invoice_id"], json!(invoice_id));

    let get_resp = server
        .get(&format!("/api/billing/dunning/{dunning_id}"))
        .add_cookie(Cookie::new("session", token))
        .await;

    get_resp.assert_status_ok();
    let fetched: Value = get_resp.json();
    assert_eq!(fetched["id"], json!(dunning_id));
    assert_eq!(fetched["step"], json!("warning"));
    assert_eq!(fetched["invoice_id"], json!(invoice_id));
}

#[sqlx::test(migrations = "../../migrations")]
async fn billing_webhooks_admin_crud_and_test(pool: PgPool) {
    let (server, token) = setup(pool).await;
    let (webhook_url, mut rx) = spawn_webhook_capture_server().await;

    let create_resp = server
        .post("/api/billing/webhooks")
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "url": webhook_url,
            "description": "Primary billing webhook",
            "events": ["invoice.paid"],
            "secret": "whsec_integration_test"
        }))
        .await;

    create_resp.assert_status(StatusCode::CREATED);
    let created: Value = create_resp.json();
    let webhook_id = created["id"]
        .as_str()
        .expect("webhook id should exist")
        .to_string();

    assert_eq!(created["url"], json!(webhook_url));
    assert_eq!(created["description"], json!("Primary billing webhook"));
    assert_eq!(created["events"], json!(["invoice.paid"]));
    assert_eq!(created["status"], json!("active"));

    let list_resp = server
        .get("/api/billing/webhooks")
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    list_resp.assert_status_ok();
    let listed: Vec<Value> = list_resp.json();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["id"], json!(webhook_id));

    let get_resp = server
        .get(&format!("/api/billing/webhooks/{webhook_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    get_resp.assert_status_ok();
    let fetched: Value = get_resp.json();
    assert_eq!(fetched["id"], json!(webhook_id));
    assert_eq!(fetched["url"], json!(webhook_url));
    assert_eq!(fetched["status"], json!("active"));

    let test_resp = server
        .post(&format!("/api/billing/webhooks/{webhook_id}/test"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    test_resp.assert_status_ok();
    let test_body: Value = test_resp.json();
    assert_eq!(test_body["success"], json!(true));
    assert_eq!(test_body["statusCode"], json!(204));

    let captured = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv())
        .await
        .expect("timed out waiting for webhook test payload")
        .expect("webhook test payload should be captured");
    assert_eq!(captured["type"], json!("test.webhook"));
    assert_eq!(
        captured["data"]["message"],
        json!("This is a test webhook delivery")
    );

    let update_resp = server
        .put(&format!("/api/billing/webhooks/{webhook_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .json(&json!({
            "description": "Updated billing webhook",
            "events": ["invoice.paid", "subscription.canceled"],
            "status": "inactive"
        }))
        .await;

    update_resp.assert_status_ok();
    let updated: Value = update_resp.json();
    assert_eq!(updated["id"], json!(webhook_id));
    assert_eq!(updated["description"], json!("Updated billing webhook"));
    assert_eq!(
        updated["events"],
        json!(["invoice.paid", "subscription.canceled"])
    );
    assert_eq!(updated["status"], json!("inactive"));

    let delete_resp = server
        .delete(&format!("/api/billing/webhooks/{webhook_id}"))
        .add_cookie(Cookie::new("session", token.clone()))
        .await;

    delete_resp.assert_status_ok();
    let deleted: Value = delete_resp.json();
    assert_eq!(deleted["success"], json!(true));

    let missing_resp = server
        .get(&format!("/api/billing/webhooks/{webhook_id}"))
        .add_cookie(Cookie::new("session", token))
        .await;

    missing_resp.assert_status(StatusCode::NOT_FOUND);
}
