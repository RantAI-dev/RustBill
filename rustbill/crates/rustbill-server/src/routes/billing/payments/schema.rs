use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentRequest {
    pub invoice_id: Option<Value>,
    pub amount: Option<Value>,
    pub method: Option<Value>,
    pub reference: Option<Value>,
    pub paid_at: Option<Value>,
    pub notes: Option<Value>,
    pub stripe_payment_intent_id: Option<Value>,
    pub xendit_payment_id: Option<Value>,
    pub lemonsqueezy_order_id: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePaymentRequest {
    pub reference: Option<Value>,
    pub notes: Option<Value>,
}
