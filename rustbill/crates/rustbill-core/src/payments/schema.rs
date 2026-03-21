use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct CheckoutParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub stripe_customer_id: Option<String>,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Clone)]
pub struct XenditInvoiceParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub customer_name: Option<String>,
    pub success_url: String,
    pub failure_url: String,
}

#[derive(Debug, Clone)]
pub struct XenditInvoiceResult {
    pub invoice_url: String,
    pub xendit_invoice_id: String,
}

#[derive(Debug, Clone)]
pub struct LsCheckoutParams {
    pub invoice_id: String,
    pub invoice_number: String,
    pub total: Decimal,
    pub currency: String,
    pub customer_email: Option<String>,
    pub customer_name: Option<String>,
    pub success_url: String,
}

#[derive(Debug, Clone)]
pub struct LsCheckoutResult {
    pub checkout_url: String,
    pub checkout_id: String,
}
