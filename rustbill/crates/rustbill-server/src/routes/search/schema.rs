use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<i64>,
}
