use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsListParams {
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub resource_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl EventsListParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).min(200)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0)
    }
}
