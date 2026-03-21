use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SearchResult {
    #[serde(rename = "type")]
    pub result_type: String,
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}
