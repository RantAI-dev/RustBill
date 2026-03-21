use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct GenerateInvoicePdfRequest {
    #[validate(length(min = 1, message = "invoice_id is required"))]
    pub invoice_id: String,
}
