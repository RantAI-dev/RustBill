use rustbill_core::db::models::PaymentMethod;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerMutationRequest {
    pub name: Option<String>,
    pub industry: Option<String>,
    pub tier: Option<String>,
    pub location: Option<String>,
    pub contact: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    #[serde(alias = "billing_email")]
    pub billing_email: Option<String>,
    #[serde(alias = "billing_address")]
    pub billing_address: Option<String>,
    #[serde(alias = "billing_city")]
    pub billing_city: Option<String>,
    #[serde(alias = "billing_state")]
    pub billing_state: Option<String>,
    #[serde(alias = "billing_zip")]
    pub billing_zip: Option<String>,
    #[serde(alias = "billing_country")]
    pub billing_country: Option<String>,
    #[serde(alias = "tax_id")]
    pub tax_id: Option<String>,
    #[serde(alias = "default_payment_method")]
    pub default_payment_method: Option<PaymentMethod>,
    #[serde(alias = "stripe_customer_id")]
    pub stripe_customer_id: Option<String>,
    #[serde(alias = "xendit_customer_id")]
    pub xendit_customer_id: Option<String>,
}

pub type CreateCustomerRequest = CustomerMutationRequest;
pub type UpdateCustomerRequest = CustomerMutationRequest;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DeleteCustomerResponse {
    pub success: bool,
}
