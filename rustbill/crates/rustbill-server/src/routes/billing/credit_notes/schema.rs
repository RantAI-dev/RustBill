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
pub struct CreateCreditNoteRequest {
    pub invoice_id: Option<String>,
    pub customer_id: Option<String>,
    pub amount: Option<f64>,
    pub reason: Option<String>,
}

impl<'de> Deserialize<'de> for CreateCreditNoteRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        Ok(Self {
            invoice_id: json_string(&value, "invoiceId"),
            customer_id: json_string(&value, "customerId"),
            amount: json_f64(&value, "amount"),
            reason: json_string(&value, "reason"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateCreditNoteRequest {
    pub status: Option<String>,
}

impl<'de> Deserialize<'de> for UpdateCreditNoteRequest {
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
