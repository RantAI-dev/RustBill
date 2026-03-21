use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ForecastParams {
    pub months: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ReportParams {
    pub report_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sales360Params {
    pub from: Option<String>,
    pub to: Option<String>,
    pub timezone: Option<String>,
    pub currency: Option<String>,
}
