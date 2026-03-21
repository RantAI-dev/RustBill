use serde::Deserialize;

pub use crate::routes::billing::payment_methods::schema::{
    CreatePaymentMethodRequest as CreatePaymentMethodRequestV1, DeletePaymentMethodResponse,
    SetupPaymentMethodRequest as PaymentMethodSetupRequestV1, DEFAULT_STRIPE_SETUP_CANCEL_URL,
    DEFAULT_STRIPE_SETUP_SUCCESS_URL,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentMethodCustomerQuery {
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreditsQueryV1 {
    pub customer_id: Option<String>,
    pub currency: Option<String>,
}
