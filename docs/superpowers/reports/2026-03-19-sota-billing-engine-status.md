# SOTA Billing Engine Status Report

**Date:** 2026-03-19  
**Source plan:** `docs/superpowers/plans/2026-03-17-sota-billing-engine.md`  
**Source spec:** `docs/superpowers/specs/2026-03-17-sota-billing-engine-design.md`

Legend: ✅ Implemented, 🟡 Partial, ❌ Missing, ⚪ Not verifiable from code-only

## Chunk-by-chunk status

### Chunk 1: Database Migration + Models

| Task | Status | Notes |
|---|---|---|
| Task 1: Billing engine migration | ✅ | Present in `rustbill/migrations/20260317000000_billing_engine.sql` |
| Task 2: Models/enums updates | ✅ | Present in `rustbill/crates/rustbill-core/src/db/models.rs` |

### Chunk 2: Tax Rules Engine

| Task | Status | Notes |
|---|---|---|
| Task 3: Tax core module | ✅ | `rustbill/crates/rustbill-core/src/billing/tax.rs` |
| Task 4: Tax routes | ✅ | `rustbill/crates/rustbill-server/src/routes/billing/tax_rules.rs` + route registration |
| Task 5: Tax tests | ✅ | `rustbill/crates/rustbill-server/tests/tax_rules.rs` |

### Chunk 3: Customer Credit Wallet

| Task | Status | Notes |
|---|---|---|
| Task 6: Credits core module | 🟡 | Implemented in `rustbill/crates/rustbill-core/src/billing/credits.rs`; `apply_to_invoice` remains tx-based instead of generic executor signature from plan note |
| Task 7: Credits routes | ✅ | `rustbill/crates/rustbill-server/src/routes/billing/credits.rs` |
| Task 8: Credits tests | ✅ | `rustbill/crates/rustbill-server/tests/credits.rs` including concurrency test |

### Chunk 4: Proration Engine

| Task | Status | Notes |
|---|---|---|
| Task 9: Proration module | ✅ | `rustbill/crates/rustbill-core/src/billing/proration.rs` |
| Task 10: Centralize `advance_period` | 🟡 | Calendar-month semantics implemented, but lifecycle still keeps a local wrapper instead of removing duplicate helper entirely |

### Chunk 5: Saved Payment Methods & Auto-Charge

| Task | Status | Notes |
|---|---|---|
| Task 11: Payment methods core | ✅ | `rustbill/crates/rustbill-core/src/billing/payment_methods.rs` |
| Task 12: Auto-charge engine | ✅ | Integrated with provider-side Stripe/Xendit request handling and transient/permanent failure classification |
| Task 13: Payment methods routes | ✅ | `rustbill/crates/rustbill-server/src/routes/billing/payment_methods.rs` |
| Task 14: Unified invoice pipeline | ✅ | Implemented in lifecycle with lock/tax/credits/amount_due/auto-charge integration |
| Task 15: Payments use `amount_due` + overpayment credits | ✅ | Implemented in `rustbill/crates/rustbill-core/src/billing/payments.rs` |
| Task 16: Plan change + proration API | ✅ | Shared service path + idempotent replay behavior implemented |
| Task 17: Pipeline integration tests | ✅ | `rustbill/crates/rustbill-server/tests/billing_pipeline.rs` |

### Chunk 6: Next.js API + UI

| Task | Status | Notes |
|---|---|---|
| Task 18: Proxies + hooks | ✅ | Billing proxies, dynamic routes, cookie forwarding, and hook mappings updated |
| Task 19: Tax rules management UI | ✅ | Implemented and wired in app nav |
| Task 20: Billing portal UI updates | ✅ | Payment methods, credits, tax/amount_due columns implemented |

### Chunk 7: Verification

| Task | Status | Notes |
|---|---|---|
| Task 21: Test/build/lint gate | ✅ | Full Rust tests, lint, and build pass in latest run |

## Additional implemented items beyond original chunk list

- ✅ Customer-scoped API keys for v1 billing routes:
  - Migration: `rustbill/migrations/20260318010000_api_keys_customer_scope.sql`
  - Auth/model/route updates in:
    - `rustbill/crates/rustbill-core/src/auth/api_key.rs`
    - `rustbill/crates/rustbill-core/src/db/models.rs`
    - `rustbill/crates/rustbill-server/src/routes/api_keys.rs`
    - `rustbill/crates/rustbill-server/src/routes/v1/billing.rs`

## Remaining high-impact gaps

No remaining high-impact implementation gaps are currently identified from the original plan/spec scope.

## Operational follow-ups (non-blocking)

1. Run end-to-end provider sandbox verification for Stripe/Xendit setup and off-session charge flows using real credentials.
2. Configure and validate optional external tax fallback (`stripe` or `taxjar`) in a non-dev environment.
