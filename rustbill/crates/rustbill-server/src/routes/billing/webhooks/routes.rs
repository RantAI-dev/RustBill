use super::repository::SqlxWebhooksRepository;
use super::schema::{CreateWebhookRequest, UpdateWebhookRequest};
use super::service::{self, WebhookDispatcher};
use crate::app::SharedState;
use crate::extractors::AdminUser;
use crate::routes::ApiResult;
use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_one).put(update).delete(remove))
        .route("/{id}/test", post(test_webhook))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let rows = service::list(&repo).await?;
    Ok(Json(rows))
}

async fn get_one(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let row = service::get(&repo, &id).await?;
    Ok(Json(row))
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateWebhookRequest>,
) -> ApiResult<(StatusCode, Json<serde_json::Value>)> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let row = service::create(&repo, &body).await?;
    Ok((StatusCode::CREATED, Json(row)))
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateWebhookRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let row = service::update(&repo, &id, &body).await?;
    Ok(Json(row))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let row = service::remove(&repo, &id).await?;
    Ok(Json(row))
}

struct ReqwestWebhookDispatcher {
    client: reqwest::Client,
}

impl ReqwestWebhookDispatcher {
    fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl WebhookDispatcher for ReqwestWebhookDispatcher {
    async fn post_json(&self, url: &str, payload: &serde_json::Value) -> Result<u16, String> {
        let response = self
            .client
            .post(url)
            .json(payload)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        Ok(response.status().as_u16())
    }
}

async fn test_webhook(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let repo = SqlxWebhooksRepository::new(state.db.clone());
    let dispatcher = ReqwestWebhookDispatcher::new(state.http_client.clone());
    let row = service::test_webhook(&repo, &dispatcher, &id).await?;
    Ok(Json(row))
}
