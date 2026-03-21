use rustbill_core::billing::{dunning::DunningConfig, lifecycle::LifecycleResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunAllResponse {
    pub success: bool,
    pub jobs: Vec<String>,
    pub lifecycle: LifecycleResult,
    pub dunning: RunAllDunningResponse,
    pub licenses: RunAllLicensesResponse,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunAllDunningResponse {
    pub processed: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunAllLicensesResponse {
    pub expired: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleResponse {
    pub success: bool,
    #[serde(flatten)]
    pub lifecycle: LifecycleResult,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenerateInvoicesResponse {
    pub success: bool,
    pub generated: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DunningResponse {
    pub success: bool,
    pub processed: u64,
    pub config: DunningConfig,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpireLicensesResponse {
    pub success: bool,
    pub expired: i64,
}
