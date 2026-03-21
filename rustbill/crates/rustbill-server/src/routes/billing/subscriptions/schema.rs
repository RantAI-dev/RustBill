use rustbill_core::error::BillingError;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionListParams {
    pub status: Option<String>,
    pub customer_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionRequest {
    pub customer_id: String,
    pub plan_id: String,
    pub quantity: Option<i64>,
    pub metadata: Option<serde_json::Value>,
    pub pre_renewal_invoice_days: Option<i64>,
}

impl CreateSubscriptionRequest {
    pub fn quantity_i32(&self) -> Option<i32> {
        self.quantity.map(|v| v as i32)
    }

    pub fn merged_metadata(&self) -> Result<serde_json::Value, BillingError> {
        let base = self
            .metadata
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        merge_pre_renewal_days(base, self.pre_renewal_invoice_days)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionRequest {
    pub plan_id: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub pre_renewal_invoice_days: Option<i64>,
}

impl UpdateSubscriptionRequest {
    pub fn merged_metadata_optional(&self) -> Result<Option<serde_json::Value>, BillingError> {
        if self.metadata.is_none() && self.pre_renewal_invoice_days.is_none() {
            return Ok(None);
        }

        let base = self
            .metadata
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));
        Ok(Some(merge_pre_renewal_days(
            base,
            self.pre_renewal_invoice_days,
        )?))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleRequest {
    pub subscription_id: String,
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePlanRequest {
    pub plan_id: String,
    #[serde(default)]
    pub quantity: Option<i32>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionV1Request {
    pub customer_id: Option<String>,
    pub plan_id: Option<String>,
    pub quantity: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

impl CreateSubscriptionV1Request {
    pub fn quantity_i32(&self) -> Option<i32> {
        self.quantity.map(|v| v as i32)
    }

    pub fn metadata_or_default(&self) -> serde_json::Value {
        self.metadata
            .clone()
            .unwrap_or_else(|| serde_json::json!({}))
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionV1Request {
    pub plan_id: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

fn merge_pre_renewal_days(
    mut metadata: serde_json::Value,
    pre_renewal_invoice_days: Option<i64>,
) -> Result<serde_json::Value, BillingError> {
    let Some(days) = pre_renewal_invoice_days else {
        return Ok(metadata);
    };

    if !(0..=90).contains(&days) {
        return Err(BillingError::bad_request(
            "preRenewalInvoiceDays must be between 0 and 90",
        ));
    }

    if !metadata.is_object() {
        metadata = serde_json::json!({});
    }

    if let Some(obj) = metadata.as_object_mut() {
        obj.insert("preRenewalInvoiceDays".to_string(), serde_json::json!(days));
    }

    Ok(metadata)
}
