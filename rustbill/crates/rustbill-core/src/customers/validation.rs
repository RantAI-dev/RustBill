use crate::db::models::PaymentMethod;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1, max = 255))]
    pub industry: String,
    #[validate(length(min = 1, max = 50))]
    pub tier: String,
    #[validate(length(min = 1, max = 255))]
    pub location: String,
    #[validate(length(min = 1, max = 255))]
    pub contact: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1, max = 50))]
    pub phone: String,
    // Billing profile (optional)
    #[validate(email)]
    pub billing_email: Option<String>,
    pub billing_address: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_zip: Option<String>,
    pub billing_country: Option<String>,
    pub tax_id: Option<String>,
    pub default_payment_method: Option<PaymentMethod>,
    pub stripe_customer_id: Option<String>,
    pub xendit_customer_id: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub industry: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub tier: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub location: Option<String>,
    #[validate(length(min = 1, max = 255))]
    pub contact: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 1, max = 50))]
    pub phone: Option<String>,
    #[validate(email)]
    pub billing_email: Option<String>,
    pub billing_address: Option<String>,
    pub billing_city: Option<String>,
    pub billing_state: Option<String>,
    pub billing_zip: Option<String>,
    pub billing_country: Option<String>,
    pub tax_id: Option<String>,
    pub default_payment_method: Option<PaymentMethod>,
    pub stripe_customer_id: Option<String>,
    pub xendit_customer_id: Option<String>,
}
