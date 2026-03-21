use rustbill_core::db::models::UserRole;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeycloakCallbackQuery {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionUserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub user: PublicUserResponse,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeResponse {
    pub user: SessionUserResponse,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LogoutResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoginResult {
    pub user: PublicUserResponse,
    pub session_token: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeResult {
    pub user: SessionUserResponse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogoutResult {
    pub redirect_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeycloakLoginResult {
    pub auth_url: String,
    pub state_cookie: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeycloakCallbackResult {
    pub session_token: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeycloakTokens {
    pub access_token: String,
    pub id_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoginUserRecord {
    pub id: String,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub role: UserRole,
}
