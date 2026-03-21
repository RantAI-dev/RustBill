use crate::db::models::{Invoice, SavedPaymentMethod};

#[derive(Debug)]
pub enum ChargeResult {
    Success { provider_reference: Option<String> },
    NoPaymentMethod,
    ManagedExternally,
    TransientFailure(String),
    PermanentFailure(String),
}

#[derive(Debug, Clone)]
pub struct AutoChargeContext {
    pub invoice: Invoice,
    pub payment_method: SavedPaymentMethod,
}
