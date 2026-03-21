use serde::de::Deserializer;
use serde::Deserialize;
use serde_json::Value;

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(std::borrow::ToOwned::to_owned)
}

fn json_f64(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

#[derive(Debug, Clone)]
pub struct CreateRefundRequest {
    pub payment_id: Option<String>,
    pub invoice_id: Option<String>,
    pub amount: Option<f64>,
    pub reason: Option<String>,
    pub stripe_refund_id: Option<String>,
}

impl<'de> Deserialize<'de> for CreateRefundRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self {
            payment_id: json_string(&value, "paymentId"),
            invoice_id: json_string(&value, "invoiceId"),
            amount: json_f64(&value, "amount"),
            reason: json_string(&value, "reason"),
            stripe_refund_id: json_string(&value, "stripeRefundId"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateRefundRequest {
    pub status: Option<String>,
}

impl<'de> Deserialize<'de> for UpdateRefundRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self {
            status: json_string(&value, "status"),
        })
    }
}
