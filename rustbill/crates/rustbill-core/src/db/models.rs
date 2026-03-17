//! Database models matching the existing PostgreSQL schema exactly.
//! All types use sqlx::FromRow for direct mapping.

use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

// ---- Enum types (matching PostgreSQL enums) ----

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "product_type", rename_all = "lowercase")]
pub enum ProductType {
    #[serde(rename = "licensed")]
    Licensed,
    #[serde(rename = "saas")]
    Saas,
    #[serde(rename = "api")]
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "license_status", rename_all = "lowercase")]
pub enum LicenseStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "revoked")]
    Revoked,
    #[serde(rename = "suspended")]
    Suspended,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "trend", rename_all = "lowercase")]
pub enum Trend {
    #[serde(rename = "up")]
    Up,
    #[serde(rename = "down")]
    Down,
    #[serde(rename = "stable")]
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "deal_type", rename_all = "lowercase")]
pub enum DealType {
    #[serde(rename = "sale")]
    Sale,
    #[serde(rename = "trial")]
    Trial,
    #[serde(rename = "partner")]
    Partner,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "api_key_status", rename_all = "lowercase")]
pub enum ApiKeyStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "revoked")]
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "customer")]
    Customer,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "billing_cycle", rename_all = "lowercase")]
pub enum BillingCycle {
    #[serde(rename = "monthly")]
    Monthly,
    #[serde(rename = "quarterly")]
    Quarterly,
    #[serde(rename = "yearly")]
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "pricing_model", rename_all = "snake_case")]
pub enum PricingModel {
    #[serde(rename = "flat")]
    Flat,
    #[serde(rename = "per_unit")]
    PerUnit,
    #[serde(rename = "tiered")]
    Tiered,
    #[serde(rename = "usage_based")]
    UsageBased,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "subscription_status", rename_all = "snake_case")]
pub enum SubscriptionStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "canceled")]
    Canceled,
    #[serde(rename = "past_due")]
    PastDue,
    #[serde(rename = "trialing")]
    Trialing,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "invoice_status", rename_all = "lowercase")]
pub enum InvoiceStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "issued")]
    Issued,
    #[serde(rename = "paid")]
    Paid,
    #[serde(rename = "overdue")]
    Overdue,
    #[serde(rename = "void")]
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "stripe")]
    Stripe,
    #[serde(rename = "xendit")]
    Xendit,
    #[serde(rename = "lemonsqueezy")]
    Lemonsqueezy,
    #[serde(rename = "bank_transfer")]
    BankTransfer,
    #[serde(rename = "check")]
    Check,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "credit_note_status", rename_all = "lowercase")]
pub enum CreditNoteStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "issued")]
    Issued,
    #[serde(rename = "void")]
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "discount_type", rename_all = "snake_case")]
pub enum DiscountType {
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "fixed_amount")]
    FixedAmount,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "refund_status", rename_all = "lowercase")]
pub enum RefundStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "dunning_step", rename_all = "snake_case")]
pub enum DunningStep {
    #[serde(rename = "reminder")]
    Reminder,
    #[serde(rename = "warning")]
    Warning,
    #[serde(rename = "final_notice")]
    FinalNotice,
    #[serde(rename = "suspension")]
    Suspension,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "webhook_status", rename_all = "lowercase")]
pub enum WebhookStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "inactive")]
    Inactive,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "billing_event_type")]
pub enum BillingEventType {
    #[serde(rename = "invoice.created")]
    #[sqlx(rename = "invoice.created")]
    InvoiceCreated,
    #[serde(rename = "invoice.issued")]
    #[sqlx(rename = "invoice.issued")]
    InvoiceIssued,
    #[serde(rename = "invoice.paid")]
    #[sqlx(rename = "invoice.paid")]
    InvoicePaid,
    #[serde(rename = "invoice.overdue")]
    #[sqlx(rename = "invoice.overdue")]
    InvoiceOverdue,
    #[serde(rename = "invoice.voided")]
    #[sqlx(rename = "invoice.voided")]
    InvoiceVoided,
    #[serde(rename = "payment.received")]
    #[sqlx(rename = "payment.received")]
    PaymentReceived,
    #[serde(rename = "payment.refunded")]
    #[sqlx(rename = "payment.refunded")]
    PaymentRefunded,
    #[serde(rename = "subscription.created")]
    #[sqlx(rename = "subscription.created")]
    SubscriptionCreated,
    #[serde(rename = "subscription.renewed")]
    #[sqlx(rename = "subscription.renewed")]
    SubscriptionRenewed,
    #[serde(rename = "subscription.canceled")]
    #[sqlx(rename = "subscription.canceled")]
    SubscriptionCanceled,
    #[serde(rename = "subscription.paused")]
    #[sqlx(rename = "subscription.paused")]
    SubscriptionPaused,
    #[serde(rename = "dunning.reminder")]
    #[sqlx(rename = "dunning.reminder")]
    DunningReminder,
    #[serde(rename = "dunning.warning")]
    #[sqlx(rename = "dunning.warning")]
    DunningWarning,
    #[serde(rename = "dunning.final_notice")]
    #[sqlx(rename = "dunning.final_notice")]
    DunningFinalNotice,
    #[serde(rename = "dunning.suspension")]
    #[sqlx(rename = "dunning.suspension")]
    DunningSuspension,
    #[serde(rename = "credit.deposited")]
    #[sqlx(rename = "credit.deposited")]
    CreditDeposited,
    #[serde(rename = "credit.applied")]
    #[sqlx(rename = "credit.applied")]
    CreditApplied,
    #[serde(rename = "payment_method.added")]
    #[sqlx(rename = "payment_method.added")]
    PaymentMethodAdded,
    #[serde(rename = "payment_method.removed")]
    #[sqlx(rename = "payment_method.removed")]
    PaymentMethodRemoved,
    #[serde(rename = "payment_method.failed")]
    #[sqlx(rename = "payment_method.failed")]
    PaymentMethodFailed,
    #[serde(rename = "subscription.plan_changed")]
    #[sqlx(rename = "subscription.plan_changed")]
    SubscriptionPlanChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "credit_reason", rename_all = "snake_case")]
pub enum CreditReason {
    #[serde(rename = "proration")]
    Proration,
    #[serde(rename = "credit_note")]
    CreditNote,
    #[serde(rename = "manual")]
    Manual,
    #[serde(rename = "overpayment")]
    Overpayment,
    #[serde(rename = "refund")]
    Refund,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "saved_payment_method_status", rename_all = "snake_case")]
pub enum SavedPaymentMethodStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "saved_payment_method_type", rename_all = "snake_case")]
pub enum SavedPaymentMethodType {
    #[serde(rename = "card")]
    Card,
    #[serde(rename = "bank_account")]
    BankAccount,
    #[serde(rename = "ewallet")]
    Ewallet,
    #[serde(rename = "va")]
    Va,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "payment_provider", rename_all = "snake_case")]
pub enum PaymentProvider {
    #[serde(rename = "stripe")]
    Stripe,
    #[serde(rename = "xendit")]
    Xendit,
    #[serde(rename = "lemonsqueezy")]
    Lemonsqueezy,
}

// ---- Helper types for JSONB columns ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    #[serde(rename = "upTo")]
    pub up_to: Option<i64>,
    pub price: f64,
}

// ---- Table models ----

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub product_type: ProductType,
    pub revenue: Decimal,
    pub target: Decimal,
    pub change: Decimal,
    // Licensed-specific
    pub units_sold: Option<i32>,
    pub active_licenses: Option<i32>,
    pub total_licenses: Option<i32>,
    // SaaS-specific
    pub mau: Option<i32>,
    pub dau: Option<i32>,
    pub free_users: Option<i32>,
    pub paid_users: Option<i32>,
    pub churn_rate: Option<Decimal>,
    // API-specific
    pub api_calls: Option<i32>,
    pub active_developers: Option<i32>,
    pub avg_latency: Option<Decimal>,
    // Timestamps
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Customer {
    pub id: String,
    pub name: String,
    pub industry: String,
    pub tier: String,
    pub location: String,
    pub contact: String,
    pub email: String,
    pub phone: String,
    pub total_revenue: Decimal,
    pub health_score: i32,
    pub trend: Trend,
    pub last_contact: String,
    // Billing profile
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
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomerProduct {
    pub id: String,
    pub customer_id: String,
    pub product_id: String,
    pub license_keys: Option<serde_json::Value>,
    pub mau: Option<i32>,
    pub api_calls: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Deal {
    pub id: String,
    pub customer_id: Option<String>,
    pub company: String,
    pub contact: String,
    pub email: String,
    pub value: Decimal,
    pub product_id: Option<String>,
    pub product_name: String,
    pub product_type: ProductType,
    pub deal_type: DealType,
    pub date: String,
    pub license_key: Option<String>,
    pub notes: Option<String>,
    pub usage_metric_label: Option<String>,
    pub usage_metric_value: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct License {
    pub key: String,
    pub customer_id: Option<String>,
    pub customer_name: String,
    pub product_id: Option<String>,
    pub product_name: String,
    pub status: LicenseStatus,
    pub created_at: String,
    pub expires_at: String,
    pub license_type: String,
    pub signed_payload: Option<String>,
    pub signature: Option<String>,
    pub features: Option<serde_json::Value>,
    pub max_activations: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LicenseActivation {
    pub id: String,
    pub license_key: String,
    pub device_id: String,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub activated_at: NaiveDateTime,
    pub last_seen_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub status: ApiKeyStatus,
    pub last_used_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub password_hash: Option<String>,
    pub role: UserRole,
    pub auth_provider: String,
    pub customer_id: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub expires_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PricingPlan {
    pub id: String,
    pub product_id: Option<String>,
    pub name: String,
    pub pricing_model: PricingModel,
    pub billing_cycle: BillingCycle,
    pub base_price: Decimal,
    pub unit_price: Option<Decimal>,
    pub tiers: Option<serde_json::Value>,
    pub usage_metric_name: Option<String>,
    pub trial_days: i32,
    pub active: bool,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Subscription {
    pub id: String,
    pub customer_id: String,
    pub plan_id: String,
    pub status: SubscriptionStatus,
    pub current_period_start: NaiveDateTime,
    pub current_period_end: NaiveDateTime,
    pub canceled_at: Option<NaiveDateTime>,
    pub cancel_at_period_end: bool,
    pub trial_end: Option<NaiveDateTime>,
    pub quantity: i32,
    pub metadata: Option<serde_json::Value>,
    pub stripe_subscription_id: Option<String>,
    pub managed_by: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invoice {
    pub id: String,
    pub invoice_number: String,
    pub customer_id: String,
    pub subscription_id: Option<String>,
    pub status: InvoiceStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub due_at: Option<NaiveDateTime>,
    pub paid_at: Option<NaiveDateTime>,
    pub subtotal: Decimal,
    pub tax: Decimal,
    pub total: Decimal,
    pub currency: String,
    pub notes: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub xendit_invoice_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub version: i32,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub tax_name: Option<String>,
    pub tax_rate: Option<Decimal>,
    pub tax_inclusive: bool,
    pub credits_applied: Decimal,
    pub amount_due: Decimal,
    pub auto_charge_attempts: i32,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceItem {
    pub id: String,
    pub invoice_id: String,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
    pub period_start: Option<NaiveDateTime>,
    pub period_end: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub id: String,
    pub invoice_id: String,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub paid_at: NaiveDateTime,
    pub notes: Option<String>,
    pub stripe_payment_intent_id: Option<String>,
    pub xendit_payment_id: Option<String>,
    pub lemonsqueezy_order_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UsageEvent {
    pub id: String,
    pub subscription_id: String,
    pub metric_name: String,
    pub value: Decimal,
    pub timestamp: NaiveDateTime,
    pub idempotency_key: Option<String>,
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CreditNote {
    pub id: String,
    pub credit_note_number: String,
    pub invoice_id: String,
    pub customer_id: String,
    pub reason: String,
    pub amount: Decimal,
    pub status: CreditNoteStatus,
    pub issued_at: Option<NaiveDateTime>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CreditNoteItem {
    pub id: String,
    pub credit_note_id: String,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Coupon {
    pub id: String,
    pub code: String,
    pub name: String,
    pub discount_type: DiscountType,
    pub discount_value: Decimal,
    pub currency: String,
    pub max_redemptions: Option<i32>,
    pub times_redeemed: i32,
    pub valid_from: NaiveDateTime,
    pub valid_until: Option<NaiveDateTime>,
    pub active: bool,
    pub applies_to: Option<serde_json::Value>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SubscriptionDiscount {
    pub id: String,
    pub subscription_id: String,
    pub coupon_id: String,
    pub applied_at: NaiveDateTime,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Refund {
    pub id: String,
    pub payment_id: String,
    pub invoice_id: String,
    pub amount: Decimal,
    pub reason: String,
    pub status: RefundStatus,
    pub stripe_refund_id: Option<String>,
    pub processed_at: Option<NaiveDateTime>,
    pub deleted_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DunningLogEntry {
    pub id: String,
    pub invoice_id: String,
    pub subscription_id: Option<String>,
    pub step: DunningStep,
    pub scheduled_at: NaiveDateTime,
    pub executed_at: Option<NaiveDateTime>,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BillingEvent {
    pub id: String,
    pub event_type: BillingEventType,
    pub resource_type: String,
    pub resource_id: String,
    pub customer_id: Option<String>,
    pub data: Option<serde_json::Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookEndpoint {
    pub id: String,
    pub url: String,
    pub description: Option<String>,
    pub secret: String,
    pub events: serde_json::Value,
    pub status: WebhookStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WebhookDelivery {
    pub id: String,
    pub endpoint_id: String,
    pub event_id: String,
    pub payload: serde_json::Value,
    pub response_code: Option<i32>,
    pub response_body: Option<String>,
    pub attempts: i32,
    pub delivered_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemSetting {
    pub key: String,
    pub value: String,
    pub sensitive: bool,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomerCreditBalance {
    pub customer_id: String,
    pub currency: String,
    pub balance: Decimal,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomerCredit {
    pub id: String,
    pub customer_id: String,
    pub currency: String,
    pub amount: Decimal,
    pub balance_after: Decimal,
    pub reason: CreditReason,
    pub description: String,
    pub invoice_id: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaxRule {
    pub id: String,
    pub country: String,
    pub region: Option<String>,
    pub tax_name: String,
    pub rate: Decimal,
    pub inclusive: bool,
    pub product_category: Option<String>,
    pub active: bool,
    pub effective_from: NaiveDate,
    pub effective_to: Option<NaiveDate>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SavedPaymentMethod {
    pub id: String,
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub provider_token: String,
    pub method_type: SavedPaymentMethodType,
    pub label: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub is_default: bool,
    pub status: SavedPaymentMethodStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
