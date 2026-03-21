pub use crate::routes::licenses::service::{
    create_v1 as create, get_one, list, list_activations, remove_license as remove,
    update_v1 as update,
};

use super::repository::LicensesRepository;
use super::schema::VerifyLicenseRequest;
use rustbill_core::error::BillingError;

pub async fn verify<R: LicensesRepository>(
    repo: &R,
    body: &VerifyLicenseRequest,
) -> Result<serde_json::Value, BillingError> {
    crate::routes::licenses::service::verify_v1(repo, body).await
}
