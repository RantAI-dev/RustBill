# SOTA Billing Engine Design

**Date:** 2026-03-17
**Status:** Approved
**Approach:** Full in-house billing engine (Approach A)

## Context

RustBill is a billing, product, and license management platform with a Rust backend (Axum + SQLx) and Next.js frontend. It already has: subscription lifecycle management (trial→active, renewal, cancel-at-period-end), cron scheduler (hourly lifecycle + 6-hourly dunning), dunning automation with cascade, tiered/flat/per_unit/usage_based pricing models, invoice line items, multi-provider payment (Stripe + Xendit + LemonSqueezy), webhook event system with HMAC signing and retry, and credit notes.

The system serves a **hybrid model** — mix of self-serve customers (auto-charge credit cards monthly) and enterprise customers (manual invoicing). All three payment providers are used in production.

## Gaps Addressed

| Feature | Before | After |
|---|---|---|
| Proration | Not implemented | Immediate mid-cycle proration on plan/quantity changes |
| Auto-charge | Checkout-link only | Saved payment methods + automatic charging on invoice |
| Customer credit wallet | Credit notes exist but not applied to invoices | Full wallet with auto-apply during invoicing |
| Tax calculation | No logic | Built-in tax rules engine + optional external fallback |
| Invoice generation | Simple plan amount only | Full pipeline: charges → discounts → tax → credits → auto-charge |

## Design

### 1. Architecture Overview

Five new subsystems built into `rustbill-core`, wired into the existing scheduler and event system:

```
┌──────────────────────────────────────────────────────┐
│                   Billing Engine                      │
│                                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────┐ │
│  │  Proration   │  │   Credit    │  │  Tax Rules   │ │
│  │  Engine      │  │   Wallet    │  │  Engine      │ │
│  └──────┬──────┘  └──────┬──────┘  └──────┬───────┘ │
│         │                │                │          │
│  ┌──────▼────────────────▼────────────────▼───────┐ │
│  │           Invoice Generation Pipeline           │ │
│  │  (collect charges → apply credits → apply tax   │ │
│  │   → create invoice with line items)             │ │
│  └────────────────────┬───────────────────────────┘ │
│                       │                              │
│  ┌────────────────────▼───────────────────────────┐ │
│  │           Auto-Charge Engine                    │ │
│  │  (saved payment methods → charge via provider   │ │
│  │   → on failure → dunning [existing])            │ │
│  └────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

The existing scheduler runs hourly lifecycle + 6-hourly dunning. After renewal invoice generation, the auto-charge engine attempts payment using the customer's saved method. On failure, the existing dunning flow takes over.

### 2. Proration Engine

**Trigger:** API call to change a subscription's `plan_id` or `quantity` mid-cycle.

**Calculation:**

```
days_remaining_ratio = (period_end - now) / (period_end - period_start)

credit_for_old_plan = old_plan_amount × days_remaining_ratio
charge_for_new_plan = new_plan_amount × days_remaining_ratio
net = charge - credit

If net > 0 → create invoice immediately (customer owes money)
If net < 0 → deposit |net| into customer credit wallet
If net = 0 → no financial action
```

**Example:** Customer on $100/mo, upgrades to $200/mo at day 15 of 30-day cycle:

| Line Item | Amount |
|---|---|
| Credit: Pro Plan (Mar 15 – Mar 31) | -$50.00 |
| Charge: Enterprise Plan (Mar 15 – Mar 31) | +$100.00 |
| **Total** | **$50.00** |

**Quantity changes** follow the same logic — prorate the delta seats for remaining days.

**Behavior:** The subscription keeps its current `period_end` — only the plan/price changes, the cycle does not reset.

**New module:** `rustbill-core/src/billing/proration.rs`

```rust
pub struct ProrationResult {
    pub credit_amount: Decimal,
    pub charge_amount: Decimal,
    pub net: Decimal,
    pub line_items: Vec<ProrationLineItem>,
}

pub fn calculate_proration(
    old_plan: &PricingPlan,
    new_plan: &PricingPlan,
    old_quantity: i32,
    new_quantity: i32,
    period_start: NaiveDateTime,
    period_end: NaiveDateTime,
    now: NaiveDateTime,
) -> ProrationResult;
```

**Modified:** Subscription update route — when `plan_id` or `quantity` changes, run proration engine, generate invoice or credit, then update subscription.

### 3. Customer Credit Wallet

**Concept:** Every customer has a credit balance. Credits are applied automatically during invoice generation before charging the payment method.

**Credit sources:**

1. Proration — downgrade produces negative net → credit deposited
2. Credit notes — issuing a credit note now deposits into wallet
3. Manual adjustment — admin adds/removes credits via API
4. Overpayment — payment exceeds invoice total → excess to wallet
5. Refund-to-credit — refund to wallet instead of payment method

**Credit consumption:** During invoice generation, `min(balance, invoice_total)` is applied as a negative line item. If credit covers the full amount, the invoice is marked as paid immediately with no charge.

**Database — new table `customer_credits`:**

| Column | Type | Purpose |
|---|---|---|
| id | TEXT PK | UUID |
| customer_id | TEXT FK → customers | |
| amount | DECIMAL(12,2) | positive = deposit, negative = withdrawal |
| balance_after | DECIMAL(12,2) | running balance for audit trail |
| reason | ENUM(proration, credit_note, manual, overpayment, refund) | |
| description | TEXT | human-readable note |
| invoice_id | TEXT FK → invoices, nullable | which invoice consumed/generated this |
| created_at | TIMESTAMP | |

Balance is always computed as `SUM(amount) FROM customer_credits WHERE customer_id = X`. No separate balance column — this gives a full audit trail and avoids race conditions.

**New module:** `rustbill-core/src/billing/credits.rs`

```rust
pub async fn get_balance(pool: &PgPool, customer_id: &str) -> Result<Decimal>;
pub async fn deposit(pool: &PgPool, customer_id: &str, amount: Decimal, reason: CreditReason, description: &str) -> Result<CreditEntry>;
pub async fn apply_to_invoice(pool: &PgPool, customer_id: &str, invoice_id: &str, max_amount: Decimal) -> Result<Decimal>;
```

### 4. Saved Payment Methods & Auto-Charge

**Concept:** Customers have one or more saved payment methods (tokenized via provider). One is marked as default. The auto-charge engine uses the default method when invoices are generated.

**Tokenization per provider:**

| Provider | Token Type | Creation Flow |
|---|---|---|
| Stripe | `pm_xxx` (PaymentMethod ID) | Customer completes SetupIntent → webhook stores token |
| Xendit | Linked account / token ID | Xendit tokenization API / recurring setup |
| LemonSqueezy | LS subscription ID | LS manages charging — we store reference |

RustBill never stores raw card numbers — only provider-issued tokens.

**Database — new table `payment_methods`:**

| Column | Type | Purpose |
|---|---|---|
| id | TEXT PK | UUID |
| customer_id | TEXT FK → customers | |
| provider | ENUM(stripe, xendit, lemonsqueezy) | which provider holds the token |
| provider_token | TEXT | provider's token/ID |
| method_type | ENUM(card, bank_account, ewallet, va) | kind of method |
| label | TEXT | display name ("Visa ****4242") |
| last_four | VARCHAR(4) nullable | last 4 digits for cards |
| expiry_month | INT nullable | card expiry |
| expiry_year | INT nullable | card expiry |
| is_default | BOOLEAN | one default per customer |
| status | ENUM(active, expired, failed) | |
| created_at | TIMESTAMP | |
| updated_at | TIMESTAMP | |

**Auto-charge flow (runs after invoice generation in scheduler):**

```
Invoice created (status: draft)
  → Issue invoice (status: issued)
  → Look up customer's default payment method
  → If no saved method → stop (awaits manual payment / checkout link)
  → If saved method exists:
      → Call provider-specific charge API:
         Stripe:  PaymentIntent.create(amount, payment_method, confirm: true, off_session: true)
         Xendit:  Charge recurring token
         LS:      Skip (LS manages its own charging)
      → On success:
         → Create payment record
         → Mark invoice as paid
         → Emit payment.received + invoice.paid events
      → On failure:
         → Mark payment_method.status = failed (if permanent, e.g., card_declined)
         → Invoice stays as issued
         → Dunning flow kicks in (existing system)
```

**Setup flow for customers (self-serve):**

1. Customer clicks "Add payment method" in billing portal
2. Frontend creates a Stripe SetupIntent or Xendit tokenization session
3. Customer enters card details on provider's hosted form (PCI compliant — card data never touches RustBill)
4. Provider webhook confirms setup → token stored in `payment_methods`

**New modules:**

- `rustbill-core/src/billing/payment_methods.rs` — CRUD for saved methods
- `rustbill-core/src/billing/auto_charge.rs` — provider-specific charge logic

**New routes:**

- `GET/POST/DELETE /api/billing/payment-methods` (admin)
- `GET/POST/DELETE /api/v1/billing/payment-methods` (public API)

### 5. Tax Rules Engine

**Two layers — built-in rules + optional external fallback.**

**Layer 1: Built-in tax rules**

A configurable rules table covering known markets. Evaluated at invoice generation time based on customer's `billing_country` and `billing_state`.

**Database — new table `tax_rules`:**

| Column | Type | Purpose |
|---|---|---|
| id | TEXT PK | UUID |
| country | VARCHAR(2) | ISO country code |
| region | VARCHAR(100) nullable | state/province (for US) |
| tax_name | TEXT | display name ("PPN", "GST", "VAT") |
| rate | DECIMAL(6,4) | e.g., 0.1100 = 11% |
| inclusive | BOOLEAN | tax-inclusive (EU/SEA) vs exclusive (US) |
| product_category | TEXT nullable | different rates per product type |
| active | BOOLEAN | |
| effective_from | DATE | when this rate starts |
| effective_to | DATE nullable | null = current |
| created_at | TIMESTAMP | |

**Seed data:**

| Country | Region | Tax Name | Rate | Inclusive |
|---|---|---|---|---|
| ID | null | PPN | 11% | true |
| SG | null | GST | 9% | false |
| US | CA | Sales Tax | 7.25% | false |
| US | TX | Sales Tax | 6.25% | false |
| DE | null | VAT | 19% | true |
| GB | null | VAT | 20% | true |

**Layer 2: External fallback**

```
1. Look up tax_rules for customer.billing_country + billing_state
2. If found → use it
3. If not found AND external_tax_provider configured:
   → Call Stripe Tax API / TaxJar with (amount, customer_address, product_type)
   → Cache the result as a new tax_rule for future use
4. If not found AND no external provider → apply 0% tax, flag invoice for review
```

**Inclusive vs exclusive tax:**

- Inclusive (EU/SEA): tax is extracted from subtotal → `tax = subtotal × rate / (1 + rate)`
- Exclusive (US): tax is added on top → `tax = subtotal × rate`

**New module:** `rustbill-core/src/billing/tax.rs`

```rust
pub struct TaxResult {
    pub rate: Decimal,
    pub amount: Decimal,
    pub name: String,
    pub inclusive: bool,
}

pub async fn resolve_tax(
    pool: &PgPool,
    country: &str,
    region: Option<&str>,
    product_category: Option<&str>,
    subtotal: Decimal,
) -> Result<TaxResult>;
```

**Admin routes:** `GET/POST/PUT/DELETE /api/billing/tax-rules`

### 6. Invoice Generation Pipeline

**Current flow:** Scheduler → renew subscription → create invoice with plan amount → done.

**New flow:**

```
Scheduler triggers billing run
  │
  ├─ 1. COLLECT CHARGES
  │    ├─ Base plan amount (flat/per_unit/tiered)
  │    ├─ Usage charges: aggregate usage_events for period
  │    │   → calculate via pricing model
  │    ├─ Proration charges (if plan changed mid-cycle)
  │    └─ Each becomes an invoice line item
  │
  ├─ 2. APPLY DISCOUNTS (existing coupon system)
  │    └─ Negative line items for active coupons
  │
  ├─ 3. CALCULATE TAX
  │    ├─ Resolve tax rule for customer's country/region
  │    ├─ If inclusive → extract tax from subtotal
  │    ├─ If exclusive → add tax on top
  │    └─ Add tax line item
  │
  ├─ 4. APPLY CREDITS
  │    ├─ Check customer credit wallet balance
  │    ├─ Apply min(balance, total) as negative line item
  │    └─ Deduct from wallet
  │
  ├─ 5. FINALIZE INVOICE
  │    ├─ Sum line items → subtotal, tax, total, amount_due
  │    ├─ If amount_due = 0 → mark as paid immediately
  │    ├─ If amount_due > 0 → status = issued
  │    └─ Emit invoice.created + invoice.issued events
  │
  └─ 6. AUTO-CHARGE
       ├─ Look up default payment method
       ├─ If none → stop (awaits manual payment / checkout link)
       ├─ If exists → charge via provider
       ├─ On success → record payment, mark paid, emit events
       └─ On failure → stays issued, dunning picks it up
```

**Modified file:** `rustbill-core/src/billing/lifecycle.rs` — the existing `process_renewal` function is replaced with this pipeline. Each step is a separate function call for independent testability.

**New fields on `invoices` table:**

| Column | Type | Purpose |
|---|---|---|
| tax_name | VARCHAR(50) nullable | "PPN", "VAT", "Sales Tax" |
| tax_rate | DECIMAL(6,4) nullable | the rate applied |
| tax_inclusive | BOOLEAN DEFAULT false | whether tax is inclusive |
| credits_applied | DECIMAL(12,2) DEFAULT 0 | how much credit was used |
| amount_due | DECIMAL(12,2) | total minus credits (what needs to be charged) |

### 7. API Surface

**New v1 public API endpoints:**

| Method | Path | Purpose |
|---|---|---|
| GET | `/api/v1/billing/payment-methods` | List customer's saved methods |
| POST | `/api/v1/billing/payment-methods/setup` | Create setup session (returns provider URL) |
| DELETE | `/api/v1/billing/payment-methods/:id` | Remove a saved method |
| POST | `/api/v1/billing/payment-methods/:id/default` | Set as default |
| GET | `/api/v1/billing/credits` | Get customer credit balance + history |
| POST | `/api/v1/billing/subscriptions/:id/change-plan` | Change plan with immediate proration |

**New admin API endpoints:**

| Method | Path | Purpose |
|---|---|---|
| CRUD | `/api/billing/tax-rules` | Manage tax rules |
| POST | `/api/billing/credits/adjust` | Manual credit adjustment |
| GET | `/api/billing/credits/:customerId` | View customer credit history |

### 8. Dashboard UI Changes

- **Billing Portal:** Add "Payment Methods" tab — list saved methods, add new, set default
- **Billing Portal:** Show credit balance in header area
- **Settings:** Add "Tax Rules" management page
- **Invoices:** Show tax breakdown, credits applied, amount due columns
- **Customer Detail:** Show credit balance and payment methods

### 9. Files Modified vs Created

**New files (Rust):**

- `rustbill-core/src/billing/proration.rs`
- `rustbill-core/src/billing/credits.rs`
- `rustbill-core/src/billing/tax.rs`
- `rustbill-core/src/billing/payment_methods.rs`
- `rustbill-core/src/billing/auto_charge.rs`
- `rustbill-server/src/routes/billing/tax_rules.rs`
- `rustbill-server/src/routes/billing/payment_methods.rs`
- `rustbill-server/src/routes/billing/credits.rs`
- New migration file for schema changes

**Modified files (Rust):**

- `rustbill-core/src/billing/lifecycle.rs` — replace `process_renewal` with full pipeline
- `rustbill-core/src/billing/mod.rs` — register new modules
- `rustbill-core/src/db/models.rs` — new enums and structs
- `rustbill-server/src/routes/billing/mod.rs` — register new routes
- `rustbill-server/src/routes/billing/subscriptions.rs` — add proration to plan change
- `rustbill-server/src/routes/v1/billing.rs` — add new v1 endpoints
- `rustbill-server/src/app.rs` — wire new routes

**New/modified files (Next.js):**

- `components/dashboard/sections/billing-portal.tsx` — payment methods tab, credit balance
- `components/management/tax-rules.tsx` — new management page
- `components/management/invoices.tsx` — tax/credit columns
- `hooks/use-api.ts` — new SWR hooks
- `app/api/billing/tax-rules/route.ts` — proxy to Rust
- `app/api/billing/payment-methods/route.ts` — proxy to Rust
- `app/api/billing/credits/route.ts` — proxy to Rust

### 10. Testing Strategy

Each subsystem is independently testable:

- **Proration:** Unit tests with known dates/amounts, verify line item generation
- **Credits:** Integration tests — deposit, apply to invoice, verify balance
- **Tax:** Unit tests per country/region, inclusive vs exclusive math
- **Auto-charge:** Integration tests with mock provider responses (success, decline, network error)
- **Pipeline:** End-to-end test — create customer + plan + subscription + usage → run billing → verify invoice with all line items, tax, credits, payment
