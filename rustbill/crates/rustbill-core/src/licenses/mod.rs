pub mod repository;
pub mod schema;
pub mod service;
pub mod signing;
pub mod validation;

use repository::{LicensesRepository, PgLicensesRepository};
pub use service::*;
use sqlx::PgPool;

pub async fn list_licenses(pool: &PgPool) -> crate::error::Result<Vec<serde_json::Value>> {
    let repo = PgLicensesRepository::new(pool);
    service::list_licenses_with_repo(&repo).await
}

pub async fn get_license(
    pool: &PgPool,
    key: &str,
) -> crate::error::Result<crate::db::models::License> {
    let repo = PgLicensesRepository::new(pool);
    service::get_license_with_repo(&repo, key).await
}

pub async fn create_license(
    pool: &PgPool,
    req: schema::CreateLicenseRequest,
) -> crate::error::Result<crate::db::models::License> {
    let repo = PgLicensesRepository::new(pool);
    service::create_license_with_repo(&repo, req).await
}

pub async fn update_license(
    pool: &PgPool,
    key: &str,
    req: schema::UpdateLicenseRequest,
) -> crate::error::Result<crate::db::models::License> {
    let repo = PgLicensesRepository::new(pool);
    service::update_license_with_repo(&repo, key, req).await
}

pub async fn delete_license(pool: &PgPool, key: &str) -> crate::error::Result<()> {
    let repo = PgLicensesRepository::new(pool);
    service::delete_license_with_repo(&repo, key).await
}

pub async fn list_activations(
    pool: &PgPool,
    license_key: &str,
) -> crate::error::Result<Vec<crate::db::models::LicenseActivation>> {
    let repo = PgLicensesRepository::new(pool);
    service::list_activations_with_repo(&repo, license_key).await
}

pub async fn deactivate_device(
    pool: &PgPool,
    license_key: &str,
    device_id: &str,
) -> crate::error::Result<()> {
    let repo = PgLicensesRepository::new(pool);
    service::deactivate_device_with_repo(&repo, license_key, device_id).await
}

pub async fn get_keypair(pool: &PgPool) -> crate::error::Result<Option<(String, String)>> {
    let repo = PgLicensesRepository::new(pool);
    repo.get_keypair().await
}

pub async fn generate_keypair_and_store(pool: &PgPool) -> crate::error::Result<(String, String)> {
    let repo = PgLicensesRepository::new(pool);
    service::generate_keypair_and_store_with_repo(&repo).await
}

pub async fn verify_license_online(
    pool: &PgPool,
    req: schema::VerifyLicenseRequest,
) -> crate::error::Result<serde_json::Value> {
    let repo = PgLicensesRepository::new(pool);
    service::verify_license_online_with_repo(&repo, req).await
}

pub async fn sign_license_by_key(
    pool: &PgPool,
    key: &str,
) -> crate::error::Result<crate::db::models::License> {
    let repo = PgLicensesRepository::new(pool);
    service::sign_license_by_key_with_repo(&repo, key).await
}

pub async fn verify_license_file(
    pool: &PgPool,
    file_content: &str,
) -> crate::error::Result<serde_json::Value> {
    let repo = PgLicensesRepository::new(pool);
    service::verify_license_file_with_repo(&repo, file_content).await
}

pub async fn export_license_file(pool: &PgPool, key: &str) -> crate::error::Result<String> {
    let repo = PgLicensesRepository::new(pool);
    service::export_license_file_with_repo(&repo, key).await
}
pub use signing::{
    generate_keypair, parse_license_file, sign_license, to_license_file, verify_license,
    LicensePayload, SignedLicense,
};
