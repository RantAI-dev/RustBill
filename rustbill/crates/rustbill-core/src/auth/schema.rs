use crate::db::models::UserRole;
use serde::Serialize;

#[derive(Debug, Clone)]
pub struct ApiKeyInfo {
    pub id: String,
    pub name: String,
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: UserRole,
    pub customer_id: Option<String>,
}
