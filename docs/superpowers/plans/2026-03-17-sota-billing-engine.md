# SOTA Billing Engine Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add proration, customer credit wallet, saved payment methods with auto-charge, tax rules engine, and a unified invoice generation pipeline to RustBill.

**Architecture:** Five subsystems built into `rustbill-core`, wired into the existing Axum scheduler and billing event system. Each subsystem is a separate Rust module with its own tests. A database migration adds new tables and columns. The invoice generation pipeline ties all subsystems together, replacing the existing `renew_single_subscription` function.

**Tech Stack:** Rust (Axum, SQLx, rust_decimal, chrono), PostgreSQL 17, Next.js 16 (TypeScript, SWR, Tailwind CSS v4)

**Spec:** `docs/superpowers/specs/2026-03-17-sota-billing-engine-design.md`

**Test helpers:** Tests use `common::test_server(pool)` → `TestServer` and `common::create_admin_session(&pool)` → `String` (session token). Also available: `create_test_customer`, `create_test_product`, `create_test_plan`, `create_test_subscription`, `create_test_invoice`, `create_test_api_key`. See `rustbill/crates/rustbill-server/tests/common/mod.rs`.

**Important notes for implementers:**
- The `credits::deposit` and `credits::apply_to_invoice` functions must accept a generic SQLx executor (`&PgPool` or `&mut Transaction`) so they can be used both standalone and within the invoice pipeline's transaction.
- Customer `billing_country` must be ISO 2-letter codes (e.g., "US", "ID", "SG") for tax rule matching. The admin UI should enforce this.
- Currency must not be hardcoded — always derive from the subscription/invoice's currency field.

---

## Chunk 1: Database Migration + Models

### Task 1: Database Migration

**Files:**
- Create: `rustbill/migrations/20260317000000_billing_engine.sql`

- [ ] **Step 1: Write the migration file**

```sql
-- New enums
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'credit.deposited';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'credit.applied';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.added';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.removed';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'payment_method.failed';
ALTER TYPE billing_event_type ADD VALUE IF NOT EXISTS 'subscription.plan_changed';

CREATE TYPE credit_reason AS ENUM ('proration', 'credit_note', 'manual', 'overpayment', 'refund');
CREATE TYPE saved_payment_method_status AS ENUM ('active', 'expired', 'failed');
CREATE TYPE saved_payment_method_type AS ENUM ('card', 'bank_account', 'ewallet', 'va');
CREATE TYPE payment_provider AS ENUM ('stripe', 'xendit', 'lemonsqueezy');

-- Customer credit balance (source of truth for current balance)
CREATE TABLE customer_credit_balances (
    customer_id TEXT NOT NULL REFERENCES customers(id),
    currency VARCHAR(3) NOT NULL DEFAULT 'USD',
    balance NUMERIC(12,2) NOT NULL DEFAULT 0 CHECK (balance >= 0),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (customer_id, currency)
);

-- Customer credits audit log
CREATE TABLE customer_credits (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    currency VARCHAR(3) NOT NULL,
    amount NUMERIC(12,2) NOT NULL,
    balance_after NUMERIC(12,2) NOT NULL,
    reason credit_reason NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    invoice_id TEXT REFERENCES invoices(id),
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_customer_credits_customer ON customer_credits (customer_id, currency);

-- Tax rules (immutable — close old, create new)
CREATE TABLE tax_rules (
    id TEXT PRIMARY KEY,
    country VARCHAR(2) NOT NULL,
    region VARCHAR(100),
    tax_name TEXT NOT NULL,
    rate NUMERIC(6,4) NOT NULL,
    inclusive BOOLEAN NOT NULL DEFAULT FALSE,
    product_category TEXT,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    effective_from DATE NOT NULL DEFAULT CURRENT_DATE,
    effective_to DATE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_tax_rules_lookup ON tax_rules (country, region, active, effective_from);

-- Saved payment methods
CREATE TABLE saved_payment_methods (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id),
    provider payment_provider NOT NULL,
    provider_token TEXT NOT NULL,
    method_type saved_payment_method_type NOT NULL,
    label TEXT NOT NULL DEFAULT '',
    last_four VARCHAR(4),
    expiry_month INTEGER,
    expiry_year INTEGER,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    status saved_payment_method_status NOT NULL DEFAULT 'active',
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE UNIQUE INDEX idx_one_default_per_customer
    ON saved_payment_methods (customer_id) WHERE is_default = TRUE;
CREATE INDEX idx_saved_pm_customer ON saved_payment_methods (customer_id);

-- New columns on invoices
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_name VARCHAR(50);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_rate NUMERIC(6,4);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS tax_inclusive BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS credits_applied NUMERIC(12,2) NOT NULL DEFAULT 0;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS amount_due NUMERIC(12,2);
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS auto_charge_attempts INTEGER NOT NULL DEFAULT 0;
ALTER TABLE invoices ADD COLUMN IF NOT EXISTS idempotency_key VARCHAR(255);
CREATE UNIQUE INDEX idx_invoice_idempotency ON invoices (idempotency_key) WHERE idempotency_key IS NOT NULL;

-- Backfill amount_due = total for existing invoices
UPDATE invoices SET amount_due = total WHERE amount_due IS NULL;
ALTER TABLE invoices ALTER COLUMN amount_due SET DEFAULT 0;

-- New columns on subscriptions
ALTER TABLE subscriptions ADD COLUMN IF NOT EXISTS managed_by VARCHAR(20);

-- Seed tax rules
INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, effective_from) VALUES
    (gen_random_uuid()::text, 'ID', NULL, 'PPN', 0.1100, TRUE, '2025-01-01'),
    (gen_random_uuid()::text, 'SG', NULL, 'GST', 0.0900, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'US', 'CA', 'Sales Tax', 0.0725, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'US', 'TX', 'Sales Tax', 0.0625, FALSE, '2025-01-01'),
    (gen_random_uuid()::text, 'DE', NULL, 'VAT', 0.1900, TRUE, '2025-01-01'),
    (gen_random_uuid()::text, 'GB', NULL, 'VAT', 0.2000, TRUE, '2025-01-01');
```

- [ ] **Step 2: Run the migration**

Run: `cd rustbill && cargo sqlx migrate run`
Expected: Migration applied successfully, no errors.

- [ ] **Step 3: Verify tables exist**

Run: `cd rustbill && cargo sqlx prepare -- --all-targets`
Expected: Query data prepared successfully (sqlx offline mode updated).

- [ ] **Step 4: Commit**

```bash
git add rustbill/migrations/20260317000000_billing_engine.sql
git commit -m "feat(billing): add migration for SOTA billing engine tables"
```

---

### Task 2: Add New Enums and Structs to models.rs

**Files:**
- Modify: `rustbill/crates/rustbill-core/src/db/models.rs`

- [ ] **Step 1: Add new BillingEventType variants**

Add after the existing `DunningSuspension` variant (around line 247):

```rust
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
```

- [ ] **Step 2: Add new enums**

Add after the existing enums section:

```rust
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
```

- [ ] **Step 3: Add new structs**

```rust
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
    pub effective_from: chrono::NaiveDate,
    pub effective_to: Option<chrono::NaiveDate>,
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
```

- [ ] **Step 4: Add new fields to existing Invoice struct**

Add after the existing `updated_at` field in the `Invoice` struct:

```rust
    pub tax_name: Option<String>,
    pub tax_rate: Option<Decimal>,
    pub tax_inclusive: bool,
    pub credits_applied: Decimal,
    pub amount_due: Decimal,
    pub auto_charge_attempts: i32,
    pub idempotency_key: Option<String>,
```

- [ ] **Step 5: Add managed_by field to Subscription struct**

Add after `stripe_subscription_id` in the `Subscription` struct:

```rust
    pub managed_by: Option<String>,
```

- [ ] **Step 6: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles with no errors. May have warnings about unused structs (expected — we'll use them in later tasks).

- [ ] **Step 7: Commit**

```bash
git add rustbill/crates/rustbill-core/src/db/models.rs
git commit -m "feat(billing): add new enums and structs for billing engine"
```

---

## Chunk 2: Tax Rules Engine

### Task 3: Tax Rules Core Module

**Files:**
- Create: `rustbill/crates/rustbill-core/src/billing/tax.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/mod.rs`

- [ ] **Step 1: Register the module**

In `rustbill/crates/rustbill-core/src/billing/mod.rs`, add:

```rust
pub mod tax;
```

- [ ] **Step 2: Write the tax module**

Create `rustbill/crates/rustbill-core/src/billing/tax.rs`:

```rust
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use sqlx::PgPool;

use crate::db::models::TaxRule;
use crate::error::Result;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaxResult {
    pub rate: Decimal,
    pub amount: Decimal,
    pub name: String,
    pub inclusive: bool,
}

impl TaxResult {
    pub fn zero() -> Self {
        Self {
            rate: Decimal::ZERO,
            amount: Decimal::ZERO,
            name: String::new(),
            inclusive: false,
        }
    }
}

/// Calculate tax for a subtotal given the customer's country and region.
/// Returns TaxResult with rate=0 if no matching rule found.
pub async fn resolve_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    _product_category: Option<&str>,
    subtotal: Decimal,
) -> Result<TaxResult> {
    let rule = find_tax_rule(pool, country, region).await?;

    match rule {
        Some(r) => Ok(calculate_tax(subtotal, &r)),
        None => Ok(TaxResult::zero()),
    }
}

/// Find the most specific active tax rule for a country + region.
/// Matches region-specific rules first, then country-only rules.
async fn find_tax_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
) -> Result<Option<TaxRule>> {
    let today = chrono::Utc::now().date_naive();

    // Try region-specific first
    if let Some(region) = region {
        let rule: Option<TaxRule> = sqlx::query_as(
            r#"SELECT * FROM tax_rules
               WHERE country = $1 AND region = $2 AND active = TRUE
               AND effective_from <= $3 AND (effective_to IS NULL OR effective_to > $3)
               ORDER BY effective_from DESC LIMIT 1"#,
        )
        .bind(country)
        .bind(region)
        .bind(today)
        .fetch_optional(pool)
        .await?;

        if rule.is_some() {
            return Ok(rule);
        }
    }

    // Fallback to country-only
    let rule: Option<TaxRule> = sqlx::query_as(
        r#"SELECT * FROM tax_rules
           WHERE country = $1 AND region IS NULL AND active = TRUE
           AND effective_from <= $2 AND (effective_to IS NULL OR effective_to > $2)
           ORDER BY effective_from DESC LIMIT 1"#,
    )
    .bind(country)
    .bind(today)
    .fetch_optional(pool)
    .await?;

    Ok(rule)
}

/// Calculate tax amount from a subtotal and a tax rule.
pub fn calculate_tax(subtotal: Decimal, rule: &TaxRule) -> TaxResult {
    let amount = if rule.inclusive {
        // Inclusive: tax is already in the subtotal
        // tax = subtotal * rate / (1 + rate)
        let divisor = Decimal::ONE + rule.rate;
        (subtotal * rule.rate / divisor).round_dp(2)
    } else {
        // Exclusive: tax is added on top
        (subtotal * rule.rate).round_dp(2)
    };

    TaxResult {
        rate: rule.rate,
        amount,
        name: rule.tax_name.clone(),
        inclusive: rule.inclusive,
    }
}

// ---- CRUD for admin ----

pub async fn list_tax_rules(pool: &PgPool) -> Result<Vec<TaxRule>> {
    let rules = sqlx::query_as::<_, TaxRule>(
        "SELECT * FROM tax_rules WHERE active = TRUE ORDER BY country, region, effective_from DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(rules)
}

pub async fn create_tax_rule(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    tax_name: &str,
    rate: Decimal,
    inclusive: bool,
    product_category: Option<&str>,
) -> Result<TaxRule> {
    let rule = sqlx::query_as::<_, TaxRule>(
        r#"INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, CURRENT_DATE)
           RETURNING *"#,
    )
    .bind(country)
    .bind(region)
    .bind(tax_name)
    .bind(rate)
    .bind(inclusive)
    .bind(product_category)
    .fetch_one(pool)
    .await?;
    Ok(rule)
}

/// "Update" a tax rule by closing the old one and creating a new one.
pub async fn update_tax_rule(
    pool: &PgPool,
    id: &str,
    tax_name: &str,
    rate: Decimal,
    inclusive: bool,
) -> Result<TaxRule> {
    let mut tx = pool.begin().await?;

    // Close old rule
    let old: TaxRule = sqlx::query_as(
        "UPDATE tax_rules SET effective_to = CURRENT_DATE, active = FALSE WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    // Create new rule
    let new_rule = sqlx::query_as::<_, TaxRule>(
        r#"INSERT INTO tax_rules (id, country, region, tax_name, rate, inclusive, product_category, active, effective_from)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, TRUE, CURRENT_DATE)
           RETURNING *"#,
    )
    .bind(&old.country)
    .bind(&old.region)
    .bind(tax_name)
    .bind(rate)
    .bind(inclusive)
    .bind(&old.product_category)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(new_rule)
}

pub async fn delete_tax_rule(pool: &PgPool, id: &str) -> Result<()> {
    sqlx::query("UPDATE tax_rules SET effective_to = CURRENT_DATE, active = FALSE WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/tax.rs rustbill/crates/rustbill-core/src/billing/mod.rs
git commit -m "feat(billing): add tax rules engine core module"
```

---

### Task 4: Tax Rules Route Handler

**Files:**
- Create: `rustbill/crates/rustbill-server/src/routes/billing/tax_rules.rs`
- Modify: `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`

- [ ] **Step 1: Create tax rules route handler**

Create `rustbill/crates/rustbill-server/src/routes/billing/tax_rules.rs`:

```rust
use axum::{extract::State, extract::Path, Json, routing::{get, post, put, delete}};
use axum::Router;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::middleware::session_auth::AdminUser;
use crate::routes::ApiResult;
use crate::app::SharedState;
use rustbill_core::billing::tax;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", put(update).delete(remove))
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
) -> ApiResult<Json<serde_json::Value>> {
    let rules = tax::list_tax_rules(&state.db).await?;
    Ok(Json(serde_json::to_value(rules)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTaxRuleRequest {
    country: String,
    region: Option<String>,
    tax_name: String,
    rate: Decimal,
    inclusive: bool,
    product_category: Option<String>,
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let rule = tax::create_tax_rule(
        &state.db,
        &body.country,
        body.region.as_deref(),
        &body.tax_name,
        body.rate,
        body.inclusive,
        body.product_category.as_deref(),
    )
    .await?;
    Ok(Json(serde_json::to_value(rule)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTaxRuleRequest {
    tax_name: String,
    rate: Decimal,
    inclusive: bool,
}

async fn update(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateTaxRuleRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let rule = tax::update_tax_rule(&state.db, &id, &body.tax_name, body.rate, body.inclusive).await?;
    Ok(Json(serde_json::to_value(rule)?))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    tax::delete_tax_rule(&state.db, &id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}
```

- [ ] **Step 2: Register the route in billing mod.rs**

In `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`, add the module and nest the route:

Add to modules: `pub mod tax_rules;`

Add to `router()` function: `.nest("/tax-rules", tax_rules::router())`

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-server/src/routes/billing/tax_rules.rs rustbill/crates/rustbill-server/src/routes/billing/mod.rs
git commit -m "feat(billing): add tax rules CRUD route handler"
```

---

### Task 5: Tax Rules Tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/tax_rules.rs`

- [ ] **Step 1: Write tests**

```rust
mod common;

use common::{test_server, create_admin_session};
use sqlx::PgPool;

#[sqlx::test(migrations = "../../migrations")]
async fn test_calculate_tax_exclusive(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    // Create a US/CA exclusive tax rule
    let resp = server
        .post("/api/billing/tax-rules")
        .json(&serde_json::json!({
            "country": "US",
            "region": "NY",
            "taxName": "Sales Tax",
            "rate": "0.0800",
            "inclusive": false
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let rule: serde_json::Value = resp.json();
    assert_eq!(rule["country"], "US");
    assert_eq!(rule["region"], "NY");
    assert_eq!(rule["inclusive"], false);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_list_tax_rules(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    // Seed data should already have rules
    let resp = server
        .get("/api/billing/tax-rules")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let rules: Vec<serde_json::Value> = resp.json();
    // At least the 6 seed rules
    assert!(rules.len() >= 6);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_update_tax_rule_creates_new_version(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;

    // Create a rule
    let resp = server
        .post("/api/billing/tax-rules")
        .json(&serde_json::json!({
            "country": "JP",
            "taxName": "Consumption Tax",
            "rate": "0.1000",
            "inclusive": true
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let old_rule: serde_json::Value = resp.json();
    let old_id = old_rule["id"].as_str().unwrap();

    // Update it (should create new rule, close old)
    let resp = server
        .put(&format!("/api/billing/tax-rules/{old_id}"))
        .json(&serde_json::json!({
            "taxName": "Consumption Tax",
            "rate": "0.0800",
            "inclusive": true
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let new_rule: serde_json::Value = resp.json();
    assert_ne!(new_rule["id"], old_rule["id"]);
    assert_eq!(new_rule["rate"], "0.0800");
}
```

- [ ] **Step 2: Run the tests**

Run: `cd rustbill && cargo test --test tax_rules -- --test-threads=1`
Expected: All 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add rustbill/crates/rustbill-server/tests/tax_rules.rs
git commit -m "test(billing): add tax rules engine tests"
```

---

## Chunk 3: Customer Credit Wallet

### Task 6: Credits Core Module

**Files:**
- Create: `rustbill/crates/rustbill-core/src/billing/credits.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/mod.rs`

- [ ] **Step 1: Register the module**

In `rustbill/crates/rustbill-core/src/billing/mod.rs`, add:

```rust
pub mod credits;
```

- [ ] **Step 2: Write the credits module**

Create `rustbill/crates/rustbill-core/src/billing/credits.rs`:

```rust
use rust_decimal::Decimal;
use sqlx::{PgPool, Postgres};

use crate::db::models::{CreditReason, CustomerCredit, CustomerCreditBalance};
use crate::error::{BillingError, Result};

/// Get the current credit balance for a customer in a specific currency.
pub async fn get_balance(pool: &PgPool, customer_id: &str, currency: &str) -> Result<Decimal> {
    let row: Option<CustomerCreditBalance> = sqlx::query_as(
        "SELECT * FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.balance).unwrap_or(Decimal::ZERO))
}

/// Deposit credits into a customer's wallet. Creates the balance row if it doesn't exist.
/// Accepts either &PgPool (standalone) or &mut Transaction (within pipeline).
pub async fn deposit<'e, E>(
    executor: E,
    customer_id: &str,
    currency: &str,
    amount: Decimal,
    reason: CreditReason,
    description: &str,
    invoice_id: Option<&str>,
) -> Result<CustomerCredit>
where
    E: sqlx::Acquire<'e, Database = Postgres>,
{
    if amount <= Decimal::ZERO {
        return Err(BillingError::bad_request("deposit amount must be positive"));
    }

    let mut tx = executor.acquire().await?;
    // Note: when called with &mut Transaction, acquire() returns the same connection.
    // When called with &PgPool, it acquires a new connection.
    // For true transactional use within the pipeline, pass &mut *tx from the caller.

    // Upsert balance row
    let balance_row: CustomerCreditBalance = sqlx::query_as(
        r#"INSERT INTO customer_credit_balances (customer_id, currency, balance, updated_at)
           VALUES ($1, $2, $3, NOW())
           ON CONFLICT (customer_id, currency)
           DO UPDATE SET balance = customer_credit_balances.balance + $3, updated_at = NOW()
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(amount)
    .fetch_one(&mut *tx)
    .await?;

    // Insert audit log
    let credit = sqlx::query_as::<_, CustomerCredit>(
        r#"INSERT INTO customer_credits (id, customer_id, currency, amount, balance_after, reason, description, invoice_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, NOW())
           RETURNING *"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(amount)
    .bind(balance_row.balance)
    .bind(reason)
    .bind(description)
    .bind(invoice_id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(credit)
}

/// Apply credits to an invoice. Returns the amount actually applied (may be less than max_amount
/// if balance is insufficient). Uses FOR UPDATE + CHECK constraint to prevent overdraw.
/// Should be called within the pipeline's transaction for atomicity.
pub async fn apply_to_invoice(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    customer_id: &str,
    invoice_id: &str,
    currency: &str,
    max_amount: Decimal,
) -> Result<Decimal> {
    if max_amount <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    // Get current balance (lock the row)
    let current_balance: Option<Decimal> = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2 FOR UPDATE",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_optional(&mut **tx)
    .await?;

    let balance = current_balance.unwrap_or(Decimal::ZERO);
    if balance <= Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let apply_amount = max_amount.min(balance);

    // Deduct from balance (CHECK constraint prevents going below 0)
    sqlx::query(
        "UPDATE customer_credit_balances SET balance = balance - $3, updated_at = NOW() WHERE customer_id = $1 AND currency = $2",
    )
    .bind(customer_id)
    .bind(currency)
    .bind(apply_amount)
    .execute(&mut **tx)
    .await?;

    // Get new balance for audit
    let new_balance: Decimal = sqlx::query_scalar(
        "SELECT balance FROM customer_credit_balances WHERE customer_id = $1 AND currency = $2",
    )
    .bind(customer_id)
    .bind(currency)
    .fetch_one(&mut **tx)
    .await?;

    // Insert audit log (negative amount = withdrawal)
    sqlx::query(
        r#"INSERT INTO customer_credits (id, customer_id, currency, amount, balance_after, reason, description, invoice_id, created_at)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, 'Credit applied to invoice', $6, NOW())"#,
    )
    .bind(customer_id)
    .bind(currency)
    .bind(-apply_amount)
    .bind(new_balance)
    .bind(CreditReason::Manual) // Bound as enum, not raw string
    .bind(invoice_id)
    .execute(&mut **tx)
    .await?;

    Ok(apply_amount)
}

/// List credit history for a customer.
pub async fn list_credits(
    pool: &PgPool,
    customer_id: &str,
    currency: Option<&str>,
) -> Result<Vec<CustomerCredit>> {
    let credits = if let Some(curr) = currency {
        sqlx::query_as::<_, CustomerCredit>(
            "SELECT * FROM customer_credits WHERE customer_id = $1 AND currency = $2 ORDER BY created_at DESC",
        )
        .bind(customer_id)
        .bind(curr)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, CustomerCredit>(
            "SELECT * FROM customer_credits WHERE customer_id = $1 ORDER BY created_at DESC",
        )
        .bind(customer_id)
        .fetch_all(pool)
        .await?
    };
    Ok(credits)
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/credits.rs rustbill/crates/rustbill-core/src/billing/mod.rs
git commit -m "feat(billing): add customer credit wallet core module"
```

---

### Task 7: Credits Route Handler

**Files:**
- Create: `rustbill/crates/rustbill-server/src/routes/billing/credits.rs`
- Modify: `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`

- [ ] **Step 1: Create credits route handler**

Create `rustbill/crates/rustbill-server/src/routes/billing/credits.rs`:

```rust
use axum::{extract::State, extract::Path, extract::Query, Json, routing::{get, post}};
use axum::Router;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::middleware::session_auth::AdminUser;
use crate::routes::ApiResult;
use crate::app::SharedState;
use rustbill_core::billing::credits;
use rustbill_core::db::models::CreditReason;

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/adjust", post(adjust))
        .route("/{customer_id}", get(get_customer_credits))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdjustRequest {
    customer_id: String,
    currency: String,
    amount: Decimal,
    description: String,
}

async fn adjust(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<AdjustRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let credit = credits::deposit(
        &state.db,
        &body.customer_id,
        &body.currency,
        body.amount,
        CreditReason::Manual,
        &body.description,
        None,
    )
    .await?;
    Ok(Json(serde_json::to_value(credit)?))
}

#[derive(Deserialize)]
struct CreditQuery {
    currency: Option<String>,
}

async fn get_customer_credits(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(customer_id): Path<String>,
    Query(query): Query<CreditQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let balance = credits::get_balance(&state.db, &customer_id, query.currency.as_deref().unwrap_or("USD")).await?;
    let history = credits::list_credits(&state.db, &customer_id, query.currency.as_deref()).await?;

    Ok(Json(serde_json::json!({
        "balance": balance,
        "currency": query.currency.as_deref().unwrap_or("USD"),
        "history": history
    })))
}
```

- [ ] **Step 2: Register the route**

In `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`:

Add module: `pub mod credits;`

Add to `router()`: `.nest("/credits", credits::router())`

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-server/src/routes/billing/credits.rs rustbill/crates/rustbill-server/src/routes/billing/mod.rs
git commit -m "feat(billing): add credits admin route handler"
```

---

### Task 8: Credits Tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/credits.rs`

- [ ] **Step 1: Write tests**

```rust
mod common;

use common::{test_server, create_admin_session};
use sqlx::PgPool;

async fn create_customer(server: &axum_test::TestServer, token: &str) -> String {
    let resp = server
        .post("/api/customers")
        .json(&serde_json::json!({
            "name": "Credit Test Co",
            "industry": "Tech",
            "tier": "Enterprise",
            "location": "US",
            "contact": "Jane",
            "email": "jane@test.com",
            "phone": "+1-555-0100"
        }))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    body["id"].as_str().unwrap().to_string()
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_deposit_and_get_balance(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_customer(&server, &token).await;

    // Deposit $50
    let resp = server
        .post("/api/billing/credits/adjust")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "50.00",
            "description": "Manual credit"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Check balance
    let resp = server
        .get(&format!("/api/billing/credits/{customer_id}?currency=USD"))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["balance"], "50.00");
    assert_eq!(body["history"].as_array().unwrap().len(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_deposit_rejects_negative(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let customer_id = create_customer(&server, &token).await;

    let resp = server
        .post("/api/billing/credits/adjust")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "-10.00",
            "description": "Should fail"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status(axum::http::StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run the tests**

Run: `cd rustbill && cargo test --test credits -- --test-threads=1`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add rustbill/crates/rustbill-server/tests/credits.rs
git commit -m "test(billing): add customer credit wallet tests"
```

---

## Chunk 4: Proration Engine

### Task 9: Proration Core Module

**Files:**
- Create: `rustbill/crates/rustbill-core/src/billing/proration.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/mod.rs`

- [ ] **Step 1: Register the module**

In `rustbill/crates/rustbill-core/src/billing/mod.rs`, add:

```rust
pub mod proration;
```

- [ ] **Step 2: Write the proration module**

Create `rustbill/crates/rustbill-core/src/billing/proration.rs`:

```rust
use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

use crate::db::models::{PricingModel, PricingPlan};
use crate::error::{BillingError, Result};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProrationLineItem {
    pub description: String,
    pub amount: Decimal,
    pub period_start: NaiveDateTime,
    pub period_end: NaiveDateTime,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProrationResult {
    pub credit_amount: Decimal,
    pub charge_amount: Decimal,
    pub net: Decimal,
    pub line_items: Vec<ProrationLineItem>,
}

/// Calculate proration for a mid-cycle plan or quantity change.
/// Returns error for usage-based plans (must wait for next cycle).
pub fn calculate_proration(
    old_plan: &PricingPlan,
    new_plan: &PricingPlan,
    old_quantity: i32,
    new_quantity: i32,
    period_start: NaiveDateTime,
    period_end: NaiveDateTime,
    now: NaiveDateTime,
) -> Result<ProrationResult> {
    // Disallow for usage-based plans
    if old_plan.pricing_model == PricingModel::UsageBased
        || new_plan.pricing_model == PricingModel::UsageBased
    {
        return Err(BillingError::bad_request(
            "mid-cycle plan changes are not supported for usage-based plans; schedule the change for the next billing cycle",
        ));
    }

    // Validate currencies match
    // (Plans don't have a currency field currently — this is enforced by
    //  the fact that all plans use the subscription's currency. Left as
    //  a guard for future multi-currency plan support.)

    let total_seconds = (period_end - period_start).num_seconds() as f64;
    if total_seconds <= 0.0 {
        return Err(BillingError::bad_request("invalid period: end must be after start"));
    }
    let remaining_seconds = (period_end - now).num_seconds().max(0) as f64;
    let ratio = Decimal::from_f64(remaining_seconds / total_seconds)
        .unwrap_or(Decimal::ZERO);

    let old_amount = plan_amount(old_plan, old_quantity);
    let new_amount = plan_amount(new_plan, new_quantity);

    let credit = (old_amount * ratio).round_dp(2);
    let charge = (new_amount * ratio).round_dp(2);
    let net = charge - credit;

    let mut line_items = Vec::new();

    if credit > Decimal::ZERO {
        line_items.push(ProrationLineItem {
            description: format!(
                "Credit: {} (unused portion)",
                old_plan.name
            ),
            amount: -credit,
            period_start: now,
            period_end,
        });
    }

    if charge > Decimal::ZERO {
        line_items.push(ProrationLineItem {
            description: format!(
                "Charge: {} (remaining portion)",
                new_plan.name
            ),
            amount: charge,
            period_start: now,
            period_end,
        });
    }

    Ok(ProrationResult {
        credit_amount: credit,
        charge_amount: charge,
        net,
        line_items,
    })
}

/// Get the per-period amount for a plan given a quantity.
fn plan_amount(plan: &PricingPlan, quantity: i32) -> Decimal {
    match plan.pricing_model {
        PricingModel::Flat => plan.base_price,
        PricingModel::PerUnit => {
            plan.unit_price.unwrap_or(plan.base_price) * Decimal::from(quantity)
        }
        PricingModel::Tiered => {
            // Use tiered_pricing::calculate_amount for consistency
            crate::billing::tiered_pricing::calculate_amount(
                &plan.pricing_model,
                plan.base_price,
                plan.unit_price,
                plan.tiers.as_ref().and_then(|t| {
                    serde_json::from_value::<Vec<crate::db::models::PricingTier>>(t.clone()).ok()
                }).as_deref(),
                quantity,
            )
        }
        PricingModel::UsageBased => Decimal::ZERO, // Should not reach here
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::BillingCycle;
    use chrono::NaiveDate;

    fn make_plan(name: &str, model: PricingModel, base: f64, unit: Option<f64>) -> PricingPlan {
        PricingPlan {
            id: "plan-1".to_string(),
            product_id: None,
            name: name.to_string(),
            pricing_model: model,
            billing_cycle: BillingCycle::Monthly,
            base_price: Decimal::from_f64(base).unwrap(),
            unit_price: unit.map(|u| Decimal::from_f64(u).unwrap()),
            tiers: None,
            usage_metric_name: None,
            trial_days: 0,
            active: true,
            created_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
            updated_at: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap().and_hms_opt(0, 0, 0).unwrap(),
        }
    }

    #[test]
    fn test_upgrade_mid_cycle() {
        let old = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let new = make_plan("Enterprise", PricingModel::Flat, 200.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now).unwrap();

        assert!(result.net > Decimal::ZERO); // Upgrade = positive net
        assert_eq!(result.line_items.len(), 2);
        assert!(result.line_items[0].amount < Decimal::ZERO); // Credit
        assert!(result.line_items[1].amount > Decimal::ZERO); // Charge
    }

    #[test]
    fn test_downgrade_produces_credit() {
        let old = make_plan("Enterprise", PricingModel::Flat, 200.0, None);
        let new = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now).unwrap();

        assert!(result.net < Decimal::ZERO); // Downgrade = negative net (credit)
    }

    #[test]
    fn test_usage_based_rejected() {
        let old = make_plan("Usage", PricingModel::UsageBased, 0.0, Some(0.01));
        let new = make_plan("Pro", PricingModel::Flat, 100.0, None);
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&old, &new, 1, 1, start, end, now);
        assert!(result.is_err());
    }

    #[test]
    fn test_quantity_change() {
        let plan = make_plan("Per Seat", PricingModel::PerUnit, 10.0, Some(10.0));
        let start = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap().and_hms_opt(0, 0, 0).unwrap();
        let now = NaiveDate::from_ymd_opt(2026, 3, 16).unwrap().and_hms_opt(0, 0, 0).unwrap();

        let result = calculate_proration(&plan, &plan, 5, 10, start, end, now).unwrap();

        assert!(result.net > Decimal::ZERO); // Adding seats = positive net
    }
}
```

- [ ] **Step 3: Run unit tests**

Run: `cd rustbill && cargo test --lib proration -- --test-threads=1`
Expected: All 4 unit tests pass.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/proration.rs rustbill/crates/rustbill-core/src/billing/mod.rs
git commit -m "feat(billing): add proration engine with unit tests"
```

---

### Task 10: Centralize advance_period with Calendar-Month Semantics

**Files:**
- Modify: `rustbill/crates/rustbill-core/src/billing/subscriptions.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/lifecycle.rs`

- [ ] **Step 1: Update advance_period in subscriptions.rs to use calendar months**

Replace the `advance_period` function in `rustbill/crates/rustbill-core/src/billing/subscriptions.rs` (lines 304-310):

```rust
/// Advance a period by one billing cycle using calendar-month semantics.
/// Monthly: Jan 15 → Feb 15 → Mar 15 (not +30 days).
pub fn advance_period(from: NaiveDateTime, cycle: &BillingCycle) -> NaiveDateTime {
    let date = from.date();
    let time = from.time();
    let months = match cycle {
        BillingCycle::Monthly => 1,
        BillingCycle::Quarterly => 3,
        BillingCycle::Yearly => 12,
    };
    let new_date = date
        .checked_add_months(chrono::Months::new(months))
        .unwrap_or_else(|| date + chrono::Duration::days(months as i64 * 30));
    new_date.and_time(time)
}
```

Make it `pub` so lifecycle.rs can use it.

- [ ] **Step 2: Remove duplicate advance_period from lifecycle.rs**

In `rustbill/crates/rustbill-core/src/billing/lifecycle.rs`, remove the local `advance_period` function (lines 446-452) and replace all calls to use `crate::billing::subscriptions::advance_period`.

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cd rustbill && cargo test -- --test-threads=1`
Expected: All existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/subscriptions.rs rustbill/crates/rustbill-core/src/billing/lifecycle.rs
git commit -m "refactor(billing): centralize advance_period with calendar-month semantics"
```

---

## Chunk 5: Saved Payment Methods & Auto-Charge

### Task 11: Payment Methods Core Module

**Files:**
- Create: `rustbill/crates/rustbill-core/src/billing/payment_methods.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/mod.rs`

- [ ] **Step 1: Register the module**

In `rustbill/crates/rustbill-core/src/billing/mod.rs`, add:

```rust
pub mod payment_methods;
```

- [ ] **Step 2: Write the payment_methods module**

Create `rustbill/crates/rustbill-core/src/billing/payment_methods.rs`:

```rust
use sqlx::PgPool;

use crate::db::models::{PaymentProvider, SavedPaymentMethod, SavedPaymentMethodStatus, SavedPaymentMethodType};
use crate::error::{BillingError, Result};

pub async fn list_for_customer(pool: &PgPool, customer_id: &str) -> Result<Vec<SavedPaymentMethod>> {
    let methods = sqlx::query_as::<_, SavedPaymentMethod>(
        "SELECT * FROM saved_payment_methods WHERE customer_id = $1 ORDER BY is_default DESC, created_at DESC",
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await?;
    Ok(methods)
}

pub async fn get_default(pool: &PgPool, customer_id: &str) -> Result<Option<SavedPaymentMethod>> {
    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        "SELECT * FROM saved_payment_methods WHERE customer_id = $1 AND is_default = TRUE AND status = 'active'",
    )
    .bind(customer_id)
    .fetch_optional(pool)
    .await?;
    Ok(method)
}

pub struct CreatePaymentMethodRequest {
    pub customer_id: String,
    pub provider: PaymentProvider,
    pub provider_token: String,
    pub method_type: SavedPaymentMethodType,
    pub label: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i32>,
    pub expiry_year: Option<i32>,
    pub set_default: bool,
}

pub async fn create(pool: &PgPool, req: CreatePaymentMethodRequest) -> Result<SavedPaymentMethod> {
    let mut tx = pool.begin().await?;

    // If setting as default, clear existing default first
    if req.set_default {
        sqlx::query(
            "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
        )
        .bind(&req.customer_id)
        .execute(&mut *tx)
        .await?;
    }

    // Check if this is the first method for the customer (auto-set as default)
    let existing_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM saved_payment_methods WHERE customer_id = $1 AND status = 'active'",
    )
    .bind(&req.customer_id)
    .fetch_one(&mut *tx)
    .await?;
    let is_default = req.set_default || existing_count == 0;

    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        r#"INSERT INTO saved_payment_methods
           (id, customer_id, provider, provider_token, method_type, label, last_four, expiry_month, expiry_year, is_default, status)
           VALUES (gen_random_uuid()::text, $1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')
           RETURNING *"#,
    )
    .bind(&req.customer_id)
    .bind(&req.provider)
    .bind(&req.provider_token)
    .bind(&req.method_type)
    .bind(&req.label)
    .bind(&req.last_four)
    .bind(req.expiry_month)
    .bind(req.expiry_year)
    .bind(is_default)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(method)
}

pub async fn set_default(pool: &PgPool, customer_id: &str, method_id: &str) -> Result<SavedPaymentMethod> {
    let mut tx = pool.begin().await?;

    // Clear existing default
    sqlx::query(
        "UPDATE saved_payment_methods SET is_default = FALSE, updated_at = NOW() WHERE customer_id = $1 AND is_default = TRUE",
    )
    .bind(customer_id)
    .execute(&mut *tx)
    .await?;

    // Set new default
    let method = sqlx::query_as::<_, SavedPaymentMethod>(
        "UPDATE saved_payment_methods SET is_default = TRUE, updated_at = NOW() WHERE id = $1 AND customer_id = $2 RETURNING *",
    )
    .bind(method_id)
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| BillingError::not_found("payment_method", method_id))?;

    tx.commit().await?;
    Ok(method)
}

pub async fn remove(pool: &PgPool, customer_id: &str, method_id: &str) -> Result<()> {
    let result = sqlx::query(
        "DELETE FROM saved_payment_methods WHERE id = $1 AND customer_id = $2",
    )
    .bind(method_id)
    .bind(customer_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(BillingError::not_found("payment_method", method_id));
    }
    Ok(())
}

pub async fn mark_failed(pool: &PgPool, method_id: &str) -> Result<()> {
    sqlx::query(
        "UPDATE saved_payment_methods SET status = 'failed', is_default = FALSE, updated_at = NOW() WHERE id = $1",
    )
    .bind(method_id)
    .execute(pool)
    .await?;
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/payment_methods.rs rustbill/crates/rustbill-core/src/billing/mod.rs
git commit -m "feat(billing): add saved payment methods core module"
```

---

### Task 12: Auto-Charge Engine

**Files:**
- Create: `rustbill/crates/rustbill-core/src/billing/auto_charge.rs`
- Modify: `rustbill/crates/rustbill-core/src/billing/mod.rs`

- [ ] **Step 1: Register the module**

In `rustbill/crates/rustbill-core/src/billing/mod.rs`, add:

```rust
pub mod auto_charge;
```

- [ ] **Step 2: Write the auto_charge module**

Create `rustbill/crates/rustbill-core/src/billing/auto_charge.rs`:

```rust
use rust_decimal::Decimal;
use sqlx::PgPool;

use crate::db::models::{Invoice, PaymentProvider, SavedPaymentMethod};
use crate::error::Result;

/// Result of an auto-charge attempt.
#[derive(Debug)]
pub enum ChargeResult {
    /// Payment succeeded — payment record created, invoice marked paid.
    Success,
    /// No saved payment method — invoice awaits manual payment.
    NoPaymentMethod,
    /// Subscription is managed by external provider (e.g., LemonSqueezy).
    ManagedExternally,
    /// Transient failure — should retry later.
    TransientFailure(String),
    /// Permanent failure — card declined, expired, etc.
    PermanentFailure(String),
}

/// Attempt to auto-charge an invoice using the customer's default payment method.
/// This should be called OUTSIDE the invoice creation transaction (it makes external API calls).
pub async fn try_auto_charge(
    pool: &PgPool,
    invoice: &Invoice,
    payment_method: &SavedPaymentMethod,
    _http_client: &reqwest::Client,
) -> Result<ChargeResult> {
    let amount = invoice.amount_due;
    if amount <= Decimal::ZERO {
        return Ok(ChargeResult::Success);
    }

    // Increment attempt counter
    sqlx::query("UPDATE invoices SET auto_charge_attempts = auto_charge_attempts + 1 WHERE id = $1")
        .bind(&invoice.id)
        .execute(pool)
        .await?;

    match payment_method.provider {
        PaymentProvider::Stripe => {
            charge_stripe(pool, invoice, payment_method, amount).await
        }
        PaymentProvider::Xendit => {
            charge_xendit(pool, invoice, payment_method, amount).await
        }
        PaymentProvider::Lemonsqueezy => {
            // LS manages its own charging
            Ok(ChargeResult::ManagedExternally)
        }
    }
}

async fn charge_stripe(
    _pool: &PgPool,
    _invoice: &Invoice,
    _method: &SavedPaymentMethod,
    _amount: Decimal,
) -> Result<ChargeResult> {
    // TODO: Implement Stripe PaymentIntent.create with off_session=true
    // For now, return transient failure so dunning handles it
    tracing::warn!("Stripe auto-charge not yet implemented");
    Ok(ChargeResult::TransientFailure("stripe auto-charge not implemented yet".into()))
}

async fn charge_xendit(
    _pool: &PgPool,
    _invoice: &Invoice,
    _method: &SavedPaymentMethod,
    _amount: Decimal,
) -> Result<ChargeResult> {
    // TODO: Implement Xendit recurring card charge
    tracing::warn!("Xendit auto-charge not yet implemented");
    Ok(ChargeResult::TransientFailure("xendit auto-charge not implemented yet".into()))
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/auto_charge.rs rustbill/crates/rustbill-core/src/billing/mod.rs
git commit -m "feat(billing): add auto-charge engine scaffold"
```

---

### Task 13: Payment Methods Route Handler

**Files:**
- Create: `rustbill/crates/rustbill-server/src/routes/billing/payment_methods.rs`
- Modify: `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`

- [ ] **Step 1: Create payment methods route handler**

Create `rustbill/crates/rustbill-server/src/routes/billing/payment_methods.rs`:

```rust
use axum::{extract::State, extract::Path, Json, routing::{get, post, delete}};
use axum::Router;
use serde::Deserialize;

use crate::middleware::session_auth::AdminUser;
use crate::routes::ApiResult;
use crate::app::SharedState;
use rustbill_core::billing::payment_methods;
use rustbill_core::db::models::{PaymentProvider, SavedPaymentMethodType};

pub fn router() -> Router<SharedState> {
    Router::new()
        .route("/", get(list))
        .route("/", post(create))
        .route("/{id}", delete(remove))
        .route("/{id}/default", post(set_default))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    customer_id: String,
}

async fn list(
    State(state): State<SharedState>,
    _user: AdminUser,
    axum::extract::Query(query): axum::extract::Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let methods = payment_methods::list_for_customer(&state.db, &query.customer_id).await?;
    Ok(Json(serde_json::to_value(methods)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRequest {
    customer_id: String,
    provider: PaymentProvider,
    provider_token: String,
    method_type: SavedPaymentMethodType,
    label: String,
    last_four: Option<String>,
    expiry_month: Option<i32>,
    expiry_year: Option<i32>,
    #[serde(default)]
    set_default: bool,
}

async fn create(
    State(state): State<SharedState>,
    _user: AdminUser,
    Json(body): Json<CreateRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let method = payment_methods::create(
        &state.db,
        payment_methods::CreatePaymentMethodRequest {
            customer_id: body.customer_id,
            provider: body.provider,
            provider_token: body.provider_token,
            method_type: body.method_type,
            label: body.label,
            last_four: body.last_four,
            expiry_month: body.expiry_month,
            expiry_year: body.expiry_year,
            set_default: body.set_default,
        },
    )
    .await?;
    Ok(Json(serde_json::to_value(method)?))
}

async fn remove(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    payment_methods::remove(&state.db, &query.customer_id, &id).await?;
    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn set_default(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ListQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let method = payment_methods::set_default(&state.db, &query.customer_id, &id).await?;
    Ok(Json(serde_json::to_value(method)?))
}
```

- [ ] **Step 2: Register the route**

In `rustbill/crates/rustbill-server/src/routes/billing/mod.rs`:

Add module: `pub mod payment_methods;`

Add to `router()`: `.nest("/payment-methods", payment_methods::router())`

- [ ] **Step 3: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-server/src/routes/billing/payment_methods.rs rustbill/crates/rustbill-server/src/routes/billing/mod.rs
git commit -m "feat(billing): add payment methods route handler"
```

---

## Chunk 6: Invoice Generation Pipeline

### Task 14: Refactor lifecycle.rs — Invoice Generation Pipeline

This is the core task. Replace the existing `renew_single_subscription` with the full 6-step pipeline.

**Files:**
- Modify: `rustbill/crates/rustbill-core/src/billing/lifecycle.rs`

- [ ] **Step 1: Read the current lifecycle.rs carefully**

Run: Read `rustbill/crates/rustbill-core/src/billing/lifecycle.rs` fully to understand the existing `renew_single_subscription` function (lines 164-376) and `renew_active_subscriptions` (lines 118-163).

- [ ] **Step 2: Update renew_active_subscriptions to use FOR UPDATE SKIP LOCKED**

In the `renew_active_subscriptions` function, change the subscription query to use locking:

Replace the existing subscription query with:

```rust
    let subs: Vec<Subscription> = sqlx::query_as(
        r#"SELECT * FROM subscriptions
           WHERE status = 'active'
           AND current_period_end <= $1
           AND deleted_at IS NULL
           AND (managed_by IS NULL OR managed_by = '')
           FOR UPDATE SKIP LOCKED"#,
    )
    .bind(now)
    .fetch_all(pool)
    .await?;
```

- [ ] **Step 3: Rewrite renew_single_subscription with the full pipeline**

Replace the existing `renew_single_subscription` function with the new pipeline. The key changes:

1. **Step 0:** Already handled by FOR UPDATE SKIP LOCKED in the caller
2. **Step 1:** Collect charges (existing logic for plan amount + usage)
3. **Step 2:** Apply discounts (existing coupon logic)
4. **Step 3:** Calculate tax (NEW — call `tax::resolve_tax`)
5. **Step 4:** Apply credits (NEW — call `credits::apply_to_invoice`)
6. **Step 5:** Finalize invoice (NEW — set `amount_due`, handle zero-due)
7. **Step 6:** Auto-charge (NEW — call `auto_charge::try_auto_charge`)

The function should:

- Use a transaction for steps 1-5
- Fetch the customer's `billing_country` and `billing_state` for tax lookup
- Add `tax_name`, `tax_rate`, `tax_inclusive`, `credits_applied`, `amount_due` to the invoice INSERT
- After the transaction commits, attempt auto-charge (step 6)
- On auto-charge success, create payment record and mark invoice as paid
- On auto-charge failure, log and let dunning handle it

This is a large refactor. The key SQL changes:

Invoice INSERT gains new columns:
```sql
INSERT INTO invoices (id, invoice_number, customer_id, subscription_id, status,
    subtotal, tax, total, currency, issued_at, due_at,
    tax_name, tax_rate, tax_inclusive, credits_applied, amount_due)
VALUES (...)
```

- [ ] **Step 4: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 5: Run existing subscription/lifecycle tests**

Run: `cd rustbill && cargo test --test subscriptions -- --test-threads=1`
Expected: All existing tests still pass (the pipeline is backward-compatible — when no tax rules exist and no credits, behavior is the same).

- [ ] **Step 6: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/lifecycle.rs
git commit -m "feat(billing): replace renew_single_subscription with full invoice pipeline"
```

---

### Task 15: Update Payment Logic for amount_due

**Files:**
- Modify: `rustbill/crates/rustbill-core/src/billing/payments.rs`

- [ ] **Step 1: Update the payment completeness check**

In `create_payment_inner` (around line 180), change:

```rust
// OLD: if net_paid >= invoice.total
// NEW: use amount_due (what actually needs to be collected after credits)
if net_paid >= invoice.amount_due {
```

- [ ] **Step 2: Add overpayment → credit wallet logic**

After marking the invoice as paid, add:

```rust
// Deposit overpayment into credit wallet
if net_paid > invoice.amount_due {
    let excess = net_paid - invoice.amount_due;
    if let Err(e) = crate::billing::credits::deposit(
        pool,
        &invoice.customer_id,
        &invoice.currency,
        excess,
        crate::db::models::CreditReason::Overpayment,
        &format!("Overpayment on invoice {}", invoice.invoice_number),
        Some(&invoice.id),
    ).await {
        tracing::warn!("Failed to deposit overpayment credit: {e}");
    }
}
```

- [ ] **Step 3: Verify it compiles and tests pass**

Run: `cd rustbill && cargo test --test payments -- --test-threads=1`
Expected: All existing payment tests pass.

- [ ] **Step 4: Commit**

```bash
git add rustbill/crates/rustbill-core/src/billing/payments.rs
git commit -m "feat(billing): use amount_due for payment completeness, deposit overpayments to wallet"
```

---

### Task 16: Plan Change API with Proration

**Files:**
- Modify: `rustbill/crates/rustbill-server/src/routes/billing/subscriptions.rs`

- [ ] **Step 1: Add change-plan route**

Add a new route to the subscriptions router:

```rust
.route("/{id}/change-plan", post(change_plan))
```

- [ ] **Step 2: Implement the change_plan handler**

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePlanRequest {
    plan_id: String,
    #[serde(default)]
    quantity: Option<i32>,
    idempotency_key: Option<String>,
}

async fn change_plan(
    State(state): State<SharedState>,
    _user: AdminUser,
    Path(id): Path<String>,
    Json(body): Json<ChangePlanRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now().naive_utc();
    let mut tx = state.db.begin().await?;

    // Lock subscription
    let sub: Subscription = sqlx::query_as(
        "SELECT * FROM subscriptions WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| BillingError::not_found("subscription", &id))?;

    // Idempotency check
    if let Some(ref key) = body.idempotency_key {
        let existing: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM invoices WHERE idempotency_key = $1",
        )
        .bind(key)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some((inv_id,)) = existing {
            let invoice: serde_json::Value = sqlx::query_scalar(
                "SELECT to_jsonb(i) FROM invoices i WHERE i.id = $1",
            )
            .bind(&inv_id)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
            return Ok(Json(invoice));
        }
    }

    let old_plan: PricingPlan = sqlx::query_as(
        "SELECT * FROM pricing_plans WHERE id = $1",
    )
    .bind(&sub.plan_id)
    .fetch_one(&mut *tx)
    .await?;

    let new_plan: PricingPlan = sqlx::query_as(
        "SELECT * FROM pricing_plans WHERE id = $1",
    )
    .bind(&body.plan_id)
    .fetch_one(&mut *tx)
    .await?;

    let new_quantity = body.quantity.unwrap_or(sub.quantity);

    // Calculate proration
    let proration = rustbill_core::billing::proration::calculate_proration(
        &old_plan, &new_plan, sub.quantity, new_quantity,
        sub.current_period_start, sub.current_period_end, now,
    )?;

    // Handle the financial result
    if proration.net > Decimal::ZERO {
        // Upgrade: create invoice with proration line items
        // (generate invoice number, create invoice + line items within this tx)
        // ... invoice creation SQL similar to lifecycle.rs
    } else if proration.net < Decimal::ZERO {
        // Downgrade: deposit credit (use invoice currency from the subscription's last invoice, or default)
        let currency: String = sqlx::query_scalar(
            "SELECT currency FROM invoices WHERE subscription_id = $1 ORDER BY created_at DESC LIMIT 1",
        )
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await?
        .unwrap_or_else(|| "USD".to_string());

        rustbill_core::billing::credits::deposit(
            &mut *tx, &sub.customer_id, &currency,
            proration.net.abs(),
            rustbill_core::db::models::CreditReason::Proration,
            &format!("Proration credit: {} → {}", old_plan.name, new_plan.name),
            None,
        ).await?;
    }

    // Update subscription plan
    sqlx::query(
        r#"UPDATE subscriptions
           SET plan_id = $2, quantity = $3, version = version + 1, updated_at = NOW()
           WHERE id = $1 AND version = $4"#,
    )
    .bind(&id)
    .bind(&body.plan_id)
    .bind(new_quantity)
    .bind(sub.version)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Emit event
    rustbill_core::notifications::events::emit_billing_event(
        &state.db, &state.http_client,
        BillingEventType::SubscriptionPlanChanged,
        "subscription", &id,
        Some(&sub.customer_id),
        Some(serde_json::json!({
            "old_plan": old_plan.name,
            "new_plan": new_plan.name,
            "proration_net": proration.net.to_string(),
        })),
    ).await?;

    let updated: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(s) FROM subscriptions s WHERE s.id = $1",
    )
    .bind(&id)
    .fetch_one(&state.db)
    .await?;

    Ok(Json(updated))
}
```

Note: The invoice creation SQL inside the `proration.net > 0` branch follows the same pattern as `renew_single_subscription` in lifecycle.rs — generate invoice number via sequence, INSERT invoice, INSERT invoice_items for each proration line item.

- [ ] **Step 3: Add the same route to v1 API**

In `rustbill/crates/rustbill-server/src/routes/v1/billing.rs`, add:

```rust
.route("/subscriptions/{id}/change-plan", post(change_plan_v1))
```

The v1 handler delegates to the same core logic but uses API key auth instead of session auth.

- [ ] **Step 4: Verify it compiles**

Run: `cd rustbill && cargo check`
Expected: Compiles successfully.

- [ ] **Step 5: Commit**

```bash
git add rustbill/crates/rustbill-server/src/routes/billing/subscriptions.rs rustbill/crates/rustbill-server/src/routes/v1/billing.rs
git commit -m "feat(billing): add plan change API with immediate proration"
```

---

### Task 17: Integration Tests for Pipeline

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/billing_pipeline.rs`

- [ ] **Step 1: Write integration tests**

```rust
mod common;

use common::{test_server, create_admin_session};
use sqlx::PgPool;

/// Helper: create customer, plan, and subscription
async fn setup_billing(server: &axum_test::TestServer, token: &str) -> (String, String, String) {
    // Create customer with billing_country for tax
    let resp = server
        .post("/api/customers")
        .json(&serde_json::json!({
            "name": "Pipeline Test Co",
            "industry": "Tech",
            "tier": "Enterprise",
            "location": "US",
            "contact": "Jane",
            "email": "jane@test.com",
            "phone": "+1-555-0100",
            "billingCountry": "US",
            "billingState": "CA"
        }))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    resp.assert_status_ok();
    let customer: serde_json::Value = resp.json();
    let customer_id = customer["id"].as_str().unwrap().to_string();

    // Create plan
    let resp = server
        .post("/api/billing/plans")
        .json(&serde_json::json!({
            "name": "Pro Plan",
            "pricingModel": "flat",
            "billingCycle": "monthly",
            "basePrice": "100.00"
        }))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    resp.assert_status_ok();
    let plan: serde_json::Value = resp.json();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    // Create subscription
    let resp = server
        .post("/api/billing/subscriptions")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "planId": plan_id,
            "quantity": 1
        }))
        .add_cookie(cookie::Cookie::new("session", token))
        .await;
    resp.assert_status_ok();
    let sub: serde_json::Value = resp.json();
    let sub_id = sub["id"].as_str().unwrap().to_string();

    (customer_id, plan_id, sub_id)
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_lifecycle_generates_invoice_with_tax(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let (_customer_id, _plan_id, _sub_id) = setup_billing(&server, &token).await;

    // Advance subscription period_end to past
    sqlx::query("UPDATE subscriptions SET current_period_end = NOW() - INTERVAL '1 hour'")
        .execute(&pool)
        .await
        .unwrap();

    // Trigger lifecycle
    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();
    let result: serde_json::Value = resp.json();
    assert_eq!(result["renewed"], 1);
    assert_eq!(result["invoicesGenerated"], 1);

    // Check invoice has tax fields (US/CA = 7.25% exclusive from seed data)
    let invoices: Vec<serde_json::Value> = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM invoices i ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    let inv = &invoices[0];
    assert_eq!(inv["taxName"], "Sales Tax");
}

#[sqlx::test(migrations = "../../migrations")]
async fn test_credits_applied_to_invoice(pool: PgPool) {
    let server = test_server(pool.clone()).await;
    let token = create_admin_session(&pool).await;
    let (customer_id, _plan_id, _sub_id) = setup_billing(&server, &token).await;

    // Add $25 credit
    server
        .post("/api/billing/credits/adjust")
        .json(&serde_json::json!({
            "customerId": customer_id,
            "currency": "USD",
            "amount": "25.00",
            "description": "Test credit"
        }))
        .add_cookie(cookie::Cookie::new("session", &token))
        .await
        .assert_status_ok();

    // Advance subscription and trigger lifecycle
    sqlx::query("UPDATE subscriptions SET current_period_end = NOW() - INTERVAL '1 hour'")
        .execute(&pool)
        .await
        .unwrap();

    let resp = server
        .post("/api/billing/subscriptions/lifecycle")
        .add_cookie(cookie::Cookie::new("session", &token))
        .await;
    resp.assert_status_ok();

    // Check invoice has credits_applied
    let inv: serde_json::Value = sqlx::query_scalar(
        "SELECT to_jsonb(i) FROM invoices i ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // credits_applied should be 25.00, amount_due should be total - 25
    assert_eq!(inv["creditsApplied"], "25.00");
}
```

- [ ] **Step 2: Run the tests**

Run: `cd rustbill && cargo test --test billing_pipeline -- --test-threads=1`
Expected: All tests pass.

- [ ] **Step 3: Commit**

```bash
git add rustbill/crates/rustbill-server/tests/billing_pipeline.rs
git commit -m "test(billing): add invoice generation pipeline integration tests"
```

---

## Chunk 7: Frontend UI Updates

### Task 18: Next.js API Proxies

**Files:**
- Create: `app/api/billing/tax-rules/route.ts`
- Create: `app/api/billing/payment-methods/route.ts`
- Create: `app/api/billing/credits/route.ts`

- [ ] **Step 1: Create tax-rules proxy**

These are simple proxy routes that forward to the Rust backend. Follow the pattern used by existing proxy routes (e.g., `app/api/billing/plans/route.ts`).

Each file follows this pattern:

```typescript
import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL;

export async function GET(req: NextRequest) {
  const res = await fetch(`${BACKEND}/api/billing/tax-rules`, {
    headers: { cookie: req.headers.get("cookie") ?? "" },
  });
  return NextResponse.json(await res.json(), { status: res.status });
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const res = await fetch(`${BACKEND}/api/billing/tax-rules`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      cookie: req.headers.get("cookie") ?? "",
    },
    body: JSON.stringify(body),
  });
  return NextResponse.json(await res.json(), { status: res.status });
}
```

Create similar files for `/api/billing/payment-methods/route.ts` and `/api/billing/credits/route.ts`.

- [ ] **Step 2: Add SWR hooks in use-api.ts**

In `hooks/use-api.ts`, add:

```typescript
// ---- Tax Rules ----
export function useTaxRules() {
  return useSWR("/api/billing/tax-rules", fetcher);
}

// ---- Credits ----
export function useCustomerCredits(customerId: string | undefined) {
  return useSWR(
    customerId ? `/api/billing/credits/${customerId}` : null,
    fetcher,
  );
}

// ---- Saved Payment Methods ----
export function useSavedPaymentMethods(customerId: string | undefined) {
  return useSWR(
    customerId ? `/api/billing/payment-methods?customerId=${customerId}` : null,
    fetcher,
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add app/api/billing/tax-rules/route.ts app/api/billing/payment-methods/route.ts app/api/billing/credits/route.ts hooks/use-api.ts
git commit -m "feat(frontend): add API proxies and SWR hooks for billing engine"
```

---

### Task 19: Tax Rules Management UI

**Files:**
- Create: `components/management/tax-rules.tsx`
- Modify: `app/page.tsx` (add section type and route)
- Modify: `components/dashboard/sidebar.tsx` (add nav item)
- Modify: `components/dashboard/header.tsx` (add section title)

- [ ] **Step 1: Create the tax rules management component**

Follow the existing management component pattern (e.g., `components/management/coupons.tsx`) — table with CRUD dialog, using the `useTaxRules` SWR hook. Display columns: Country, Region, Tax Name, Rate, Inclusive, Effective From.

- [ ] **Step 2: Register the section**

Add `"manage-tax-rules"` to the `Section` type in `app/page.tsx`, add the switch case, add to sidebar under Management, add to header titles.

- [ ] **Step 3: Verify it builds**

Run: `bun run build`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add components/management/tax-rules.tsx app/page.tsx components/dashboard/sidebar.tsx components/dashboard/header.tsx
git commit -m "feat(frontend): add tax rules management UI"
```

---

### Task 20: Billing Portal — Payment Methods Tab + Credit Balance

**Files:**
- Modify: `components/dashboard/sections/billing-portal.tsx`

- [ ] **Step 1: Add credit balance display**

In the billing portal header area, show the customer's credit balance using `useCustomerCredits(effectiveCustomerId)`.

- [ ] **Step 2: Add "Payment Methods" tab**

Add a fourth tab `payment_methods` alongside invoices/subscriptions/activity. List saved payment methods using `useSavedPaymentMethods(effectiveCustomerId)`. Show: label, last four, provider, status, default badge.

- [ ] **Step 3: Update invoices table columns**

Add columns for: Tax, Credits Applied, Amount Due (in addition to existing Total column).

- [ ] **Step 4: Verify it builds**

Run: `bun run build`
Expected: Build succeeds.

- [ ] **Step 5: Commit**

```bash
git add components/dashboard/sections/billing-portal.tsx
git commit -m "feat(frontend): add payment methods tab and credit balance to billing portal"
```

---

### Task 21: Final Verification

- [ ] **Step 1: Run all Rust tests**

Run: `cd rustbill && cargo test -- --test-threads=1`
Expected: All tests pass.

- [ ] **Step 2: Run Next.js build**

Run: `bun run build`
Expected: Build succeeds.

- [ ] **Step 3: Run linter**

Run: `bun lint`
Expected: No errors.
