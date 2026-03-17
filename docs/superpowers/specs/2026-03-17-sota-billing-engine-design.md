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

**Usage-based subscriptions:** Mid-cycle plan changes are disallowed for `usage_based` pricing model. The API returns an error directing the caller to schedule the change for the next billing cycle. This avoids the ambiguity of settling partially-accrued usage mid-period.

**Tax on proration invoices:** Proration invoices go through the same full pipeline (steps 1-6 in Section 6), including tax calculation. The proration net amount is the taxable subtotal.

**Cross-currency plan changes:** Disallowed. Both old and new plan must use the same currency. The API returns an error if currencies differ.

**Idempotency:** The plan-change endpoint accepts an `idempotency_key` parameter. Before generating a proration invoice, the system checks whether a proration invoice already exists for this subscription with the same idempotency key. If found, the existing invoice is returned without creating a duplicate.

**Transaction boundary:** The proration calculation, invoice creation, credit deposit (if net < 0), and subscription plan update are all wrapped in a single database transaction. If any step fails, the entire operation rolls back.

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

**Modified:** Subscription update route must be refactored to use the core service layer with optimistic locking (the current route handler at `rustbill-server/src/routes/billing/subscriptions.rs` bypasses versioning). When `plan_id` or `quantity` changes, the service layer runs the proration engine, generates invoice or credit, then updates the subscription — all within a single transaction.

### 3. Customer Credit Wallet

**Concept:** Every customer has a credit balance per currency. Credits are applied automatically during invoice generation before charging the payment method.

**Credit sources:**

1. Proration — downgrade produces negative net → credit deposited
2. Credit notes — issuing a credit note now deposits into wallet
3. Manual adjustment — admin adds/removes credits via API
4. Overpayment — payment exceeds invoice total → excess to wallet
5. Refund-to-credit — refund to wallet instead of payment method

**Overpayment handling:** The existing `create_payment_inner` function in `payments.rs` must be extended: after marking an invoice as paid, if `net_paid > invoice.total`, automatically call `credits::deposit(pool, customer_id, excess, CreditReason::Overpayment, ...)`.

**Credit consumption:** During invoice generation, `min(balance, invoice_total)` is applied as a negative line item. Credits are applied to the **post-tax total** and do not affect the taxable amount (consistent with Stripe's behavior). If credit covers the full amount, the invoice is marked as paid immediately with no charge.

**Database — new table `customer_credit_balances`:**

| Column | Type | Purpose |
|---|---|---|
| customer_id | TEXT PK FK → customers | one row per customer per currency |
| currency | VARCHAR(3) PK | e.g., "USD", "IDR" |
| balance | DECIMAL(12,2) NOT NULL DEFAULT 0 | current balance, CHECK(balance >= 0) |
| updated_at | TIMESTAMP | |

**Database — new table `customer_credits` (audit log):**

| Column | Type | Purpose |
|---|---|---|
| id | TEXT PK | UUID |
| customer_id | TEXT FK → customers | |
| currency | VARCHAR(3) NOT NULL | must match invoice currency |
| amount | DECIMAL(12,2) | positive = deposit, negative = withdrawal |
| balance_after | DECIMAL(12,2) | snapshot for audit |
| reason | ENUM(proration, credit_note, manual, overpayment, refund) | |
| description | TEXT | human-readable note |
| invoice_id | TEXT FK → invoices, nullable | which invoice consumed/generated this |
| created_at | TIMESTAMP | |

**Concurrency safety:** The `customer_credit_balances` table acts as the source of truth for the current balance. Credit application uses `UPDATE customer_credit_balances SET balance = balance - $amount WHERE customer_id = $1 AND currency = $2 AND balance >= $amount RETURNING balance`. If the UPDATE affects 0 rows (insufficient balance), the credit application is skipped or partial. This database-level constraint prevents double-spending under concurrent invoice generation. The `customer_credits` table is the audit log, written after the balance update within the same transaction.

**Currency matching:** `apply_to_invoice` only applies credits in the same currency as the invoice. A customer with USD credits cannot apply them to an IDR invoice.

**Existing credit notes migration:** Existing issued credit notes are not retroactively migrated to the wallet. Going forward, new credit notes deposit into the wallet upon issuance. This avoids data migration complexity.

**New module:** `rustbill-core/src/billing/credits.rs`

```rust
pub async fn get_balance(pool: &PgPool, customer_id: &str, currency: &str) -> Result<Decimal>;
pub async fn deposit(tx: &mut PgTransaction, customer_id: &str, currency: &str, amount: Decimal, reason: CreditReason, description: &str) -> Result<CreditEntry>;
pub async fn apply_to_invoice(tx: &mut PgTransaction, customer_id: &str, invoice_id: &str, currency: &str, max_amount: Decimal) -> Result<Decimal>;
```

### 4. Saved Payment Methods & Auto-Charge

**Concept:** Customers have one or more saved payment methods (tokenized via provider). One is marked as default. The auto-charge engine uses the default method when invoices are generated.

**Tokenization per provider:**

| Provider | Token Type | Creation Flow | Auto-charge Support |
|---|---|---|---|
| Stripe | `pm_xxx` (PaymentMethod ID) | Customer completes SetupIntent → webhook stores token | Full (cards, SEPA) |
| Xendit | Card token ID | Xendit card tokenization API | Cards only (initial scope) |
| LemonSqueezy | LS subscription ID | LS manages subscriptions | LS charges independently (see below) |

**Xendit scope:** Initial implementation supports card-based recurring only. E-wallet and VA recurring have different Xendit APIs and limitations — these are deferred to a future iteration.

**LemonSqueezy integration:** LS-managed subscriptions are excluded from the invoice generation pipeline. LS handles its own charging and subscription lifecycle. RustBill listens for LS webhooks (`order.completed`, `subscription_payment.success`) and creates mirror invoice + payment records for dashboard visibility. The subscription in RustBill is flagged with `managed_by = 'lemonsqueezy'` to skip auto-charge.

RustBill never stores raw card numbers — only provider-issued tokens.

**Database — new table `saved_payment_methods`:**

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

**Table named `saved_payment_methods`** (not `payment_methods`) to avoid naming collision with the existing `PaymentMethod` enum in `models.rs` which represents payment method *types*.

**Default uniqueness:** Enforced at the database level with a partial unique index: `CREATE UNIQUE INDEX idx_one_default_per_customer ON saved_payment_methods (customer_id) WHERE is_default = true`. Setting a new default first clears the old one within the same transaction.

**Auto-charge flow (runs after invoice generation in scheduler):**

```
Invoice created (status: draft)
  → Issue invoice (status: issued)
  → Look up customer's default saved payment method
  → If no saved method → stop (awaits manual payment / checkout link)
  → If subscription managed_by = 'lemonsqueezy' → skip (LS charges independently)
  → If saved method exists:
      → Call provider-specific charge API:
         Stripe:  PaymentIntent.create(amount, payment_method, confirm: true, off_session: true)
         Xendit:  Charge card token
      → On success:
         → Create payment record
         → Mark invoice as paid
         → Emit payment.received + invoice.paid events
      → On failure (transient: timeout, 5xx, rate_limited):
         → Schedule retry: 1 hour, 4 hours, 24 hours
         → After 3 retries exhausted → invoice stays issued, dunning flow takes over
      → On failure (permanent: card_declined, insufficient_funds, expired_card):
         → Mark saved_payment_method.status = failed
         → Invoice stays issued
         → Dunning flow kicks in immediately
```

**Short-term retry vs dunning:** Auto-charge has its own retry schedule (1h, 4h, 24h) for transient failures before handing off to dunning. This prevents a temporary network issue from triggering a 3-day dunning reminder. A new field `auto_charge_attempts` on invoices tracks retry count.

**Setup flow for customers (self-serve):**

1. Customer clicks "Add payment method" in billing portal
2. Frontend creates a Stripe SetupIntent or Xendit tokenization session via `/api/billing/payment-methods/setup`
3. Customer enters card details on provider's hosted form (PCI compliant — card data never touches RustBill)
4. Provider webhook confirms setup → token stored in `saved_payment_methods`

**New modules:**

- `rustbill-core/src/billing/payment_methods.rs` — CRUD for saved methods (struct: `SavedPaymentMethod`)
- `rustbill-core/src/billing/auto_charge.rs` — provider-specific charge logic + retry

**New routes:**

- `GET/POST/DELETE /api/billing/payment-methods` (admin)
- `GET/POST/DELETE /api/v1/billing/payment-methods` (public API)

### 5. Tax Rules Engine

**Two layers — built-in rules + optional external fallback.**

**Layer 1: Built-in tax rules**

A configurable rules table covering known markets. Evaluated at invoice generation time based on customer's `billing_country` and `billing_state`.

**Tax rules are immutable:** Updates create new rows with `effective_to` set on the old row and `effective_from` set on the new row. This provides a full audit trail for which rate was in effect when any historical invoice was generated. Admin "update" API creates a new rule and closes the old one in a single transaction.

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
| effective_to | DATE nullable | null = currently active |
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
   WHERE active = true AND effective_from <= today AND (effective_to IS NULL OR effective_to > today)
2. If found → use it
3. If not found AND external_tax_provider configured:
   → Call Stripe Tax API / TaxJar with (amount, customer_address, product_type)
   → Cache the result as a new tax_rule with 90-day effective_to (TTL)
4. If not found AND no external provider → apply 0% tax, flag invoice for manual review
```

**Cached external results** expire after 90 days (`effective_to = today + 90`). On expiry, the next invoice triggers a fresh lookup. This balances API cost against rate staleness.

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
  ├─ 0. ACQUIRE LOCK (SELECT ... FOR UPDATE SKIP LOCKED)
  │    └─ Prevents duplicate processing if scheduler runs overlap
  │
  ├─ 1. COLLECT CHARGES
  │    ├─ Base plan amount (flat/per_unit/tiered)
  │    ├─ Usage charges: aggregate usage_events for period
  │    │   → calculate via pricing model
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
  ├─ 4. APPLY CREDITS (post-tax)
  │    ├─ Check customer credit wallet balance (same currency)
  │    ├─ Apply min(balance, total) as negative line item
  │    └─ Deduct from wallet (atomic UPDATE with CHECK constraint)
  │
  ├─ 5. FINALIZE INVOICE
  │    ├─ Sum line items → subtotal, tax, total, amount_due
  │    ├─ If amount_due = 0 → mark as paid immediately
  │    ├─ If amount_due > 0 → status = issued
  │    └─ Emit invoice.created + invoice.issued events
  │
  │    ── TRANSACTION BOUNDARY (steps 0-5 are one DB transaction) ──
  │
  └─ 6. AUTO-CHARGE (outside transaction — external API call)
       ├─ Look up default saved payment method
       ├─ If none → stop (awaits manual payment / checkout link)
       ├─ If managed_by = 'lemonsqueezy' → skip
       ├─ If exists → charge via provider
       ├─ On success → record payment, mark paid, emit events
       └─ On failure → retry schedule or dunning
```

**Transaction boundary:** Steps 0-5 execute within a single database transaction. This ensures that credit deductions, invoice creation, and line items are atomic. Step 6 (auto-charge) runs outside the transaction to avoid holding long-lived transactions during external API calls. If auto-charge fails, the invoice remains in "issued" state — correct behavior.

**Scheduler idempotency:** Step 0 uses `SELECT ... FOR UPDATE SKIP LOCKED` on the subscription row. This prevents duplicate processing when scheduler runs overlap. Additionally, the subscription's `current_period_end` is advanced as part of the transaction — subsequent runs will not pick up the same subscription.

**Calendar-month periods:** The existing `advance_period` function (duplicated in `lifecycle.rs` and `subscriptions.rs`) must be centralized and changed to use `chrono::NaiveDate::checked_add_months` instead of `Duration::days(30)`. "Monthly" means calendar month (Jan 15 → Feb 15 → Mar 15), not 30 days.

**Modified file:** `rustbill-core/src/billing/lifecycle.rs` — the existing `process_renewal` function is replaced with this pipeline. Each step is a separate function call for independent testability.

**Proration invoices** use the same pipeline. When a mid-cycle plan change triggers proration, the proration charges feed into step 1, then steps 2-6 proceed normally (tax is applied, credits are checked, auto-charge runs).

**New fields on `invoices` table:**

| Column | Type | Purpose |
|---|---|---|
| tax_name | VARCHAR(50) nullable | "PPN", "VAT", "Sales Tax" |
| tax_rate | DECIMAL(6,4) nullable | the rate applied |
| tax_inclusive | BOOLEAN DEFAULT false | whether tax is inclusive |
| credits_applied | DECIMAL(12,2) DEFAULT 0 | how much credit was used |
| amount_due | DECIMAL(12,2) | total minus credits (what needs to be charged) |
| auto_charge_attempts | INT DEFAULT 0 | retry count for auto-charge |

**`amount_due` is the charge target.** The existing payment completeness check (`SUM(payments) >= invoice.total`) must be updated to use `amount_due` instead of `total`. `total` remains the full invoice amount (before credits), while `amount_due` is what actually needs to be collected from the payment method.

**New fields on `subscriptions` table:**

| Column | Type | Purpose |
|---|---|---|
| managed_by | VARCHAR(20) nullable | 'lemonsqueezy' if LS manages charging, null otherwise |

### 7. New Webhook Event Types

The following event types are added to the `BillingEventType` enum:

| Event | Trigger |
|---|---|
| `credit.deposited` | Credit added to wallet (any source) |
| `credit.applied` | Credit consumed by invoice |
| `payment_method.added` | New saved payment method created |
| `payment_method.removed` | Saved payment method deleted |
| `payment_method.failed` | Saved payment method marked as failed |
| `subscription.plan_changed` | Plan or quantity changed with proration |

### 8. API Surface

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

### 9. Dashboard UI Changes

- **Billing Portal:** Add "Payment Methods" tab — list saved methods, add new, set default
- **Billing Portal:** Show credit balance in header area
- **Settings:** Add "Tax Rules" management page
- **Invoices:** Show tax breakdown, credits applied, amount due columns
- **Customer Detail:** Show credit balance and payment methods

### 10. Files Modified vs Created

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
- `rustbill-core/src/billing/subscriptions.rs` — centralize `advance_period` with calendar-month semantics
- `rustbill-core/src/billing/payments.rs` — add overpayment → credit wallet logic, use `amount_due` for completeness check
- `rustbill-core/src/billing/mod.rs` — register new modules
- `rustbill-core/src/db/models.rs` — new enums (`CreditReason`, `SavedPaymentMethodStatus`), new structs, new `BillingEventType` variants
- `rustbill-core/src/notifications/events.rs` — new event types
- `rustbill-server/src/routes/billing/mod.rs` — register new routes
- `rustbill-server/src/routes/billing/subscriptions.rs` — refactor to use core service layer with optimistic locking, add proration on plan change
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

### 11. Testing Strategy

Each subsystem is independently testable:

- **Proration:** Unit tests with known dates/amounts, verify line item generation. Test idempotency (duplicate requests). Test rejection of usage-based and cross-currency changes.
- **Credits:** Integration tests — deposit, apply to invoice, verify balance. Concurrency test: two concurrent `apply_to_invoice` calls must not overdraw (CHECK constraint). Test currency matching.
- **Tax:** Unit tests per country/region, inclusive vs exclusive math. Test effective_from/to date filtering. Test external fallback with mock HTTP.
- **Auto-charge:** Integration tests with mock provider responses (success, decline, network error). Test retry schedule (1h, 4h, 24h). Test handoff to dunning after retries exhausted.
- **Pipeline:** End-to-end test — create customer + plan + subscription + usage → run billing → verify invoice with all line items, tax, credits, payment. Test scheduler idempotency (run twice, only one invoice). Test transaction rollback on partial failure.
- **Payment methods:** Test partial unique index enforcement on `is_default`. Test webhook token storage from provider callbacks.
