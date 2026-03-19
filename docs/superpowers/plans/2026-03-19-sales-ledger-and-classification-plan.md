# Sales Ledger and Classification Plan

**Date:** 2026-03-19  
**Status:** Approved for implementation

## Objective

Create a unified "Sales 360" analytics layer without collapsing operational billing/sales models into one write table.

Keep operational truth in domain tables (`deals`, `subscriptions`, `invoices`, `payments`, `credit_notes`, `refunds`) and add an append-only ledger for consistent reporting.

## Why this approach

- Avoids mixing sales pipeline data with accounting-grade billing records.
- Prevents metric confusion (`bookings` vs `billings` vs `cash collected`).
- Enables one unified dashboard while preserving financial correctness and auditability.

## Classification model (proposed standard)

### 1) Bookings (Sales)

Commercial commitments from pipeline/contract motion.

- Typical source: `deals` (won/converted signals)
- Example events: `deal_won`, `deal_trial_started`, `deal_partner_started`

### 2) Billings (Invoicing)

Amounts invoiced to customers.

- Typical source: `invoices`, `invoice_items`
- Example events: `invoice_issued`, `invoice_voided`, `invoice_overdue`

### 3) Collections (Cash)

Actual money movement received.

- Typical source: `payments`
- Example events: `payment_collected`, `payment_failed`

### 4) Adjustments

Post-billing reductions/returns/credits.

- Typical source: `credit_notes`, `refunds`, `credits`
- Example events: `credit_note_issued`, `refund_issued`, `wallet_credit_applied`

### 5) Recurring health (Subscription performance)

MRR/ARR lifecycle and changes.

- Typical source: `subscriptions` + billing outputs
- Example events: `subscription_started`, `subscription_renewed`, `subscription_canceled`, `mrr_expanded`, `mrr_contracted`, `mrr_churned`

## Ledger design

### New table: `sales_events`

Append-only event/fact table for reporting.

Suggested columns:

- `id` (uuid/text, PK)
- `occurred_at` (timestamp)
- `event_type` (text)
- `classification` (enum/text: `bookings|billings|collections|adjustments|recurring`)
- `amount` (numeric)
- `currency` (text)
- `customer_id` (nullable)
- `subscription_id` (nullable)
- `product_id` (nullable)
- `invoice_id` (nullable)
- `payment_id` (nullable)
- `source_table` (text)
- `source_id` (text)
- `metadata` (jsonb)
- `created_at` (timestamp default now)

Indexes:

- `(occurred_at)`
- `(classification, occurred_at)`
- `(customer_id, occurred_at)`
- `(event_type, occurred_at)`
- unique guard on `(source_table, source_id, event_type)` for idempotency

## Event production strategy

### Write-time emitters (preferred)

Emit ledger rows inside existing Rust write paths, same transaction where possible.

Phase-1 emitters:

- Deal create/update terminal sales state -> bookings events
- Invoice create/issue/void/overdue -> billings events
- Payment success/failure -> collections events
- Credit note/refund/credits application -> adjustments events
- Subscription create/renew/cancel/plan-change -> recurring events

### Backfill (one-time)

Migration job to backfill historical rows from existing tables, then switch to write-time emitters for new data.

## Reporting layer

### New API endpoints (Rust)

- `GET /api/analytics/sales-360/summary`
  - totals by classification for date range
- `GET /api/analytics/sales-360/timeseries`
  - daily/weekly aggregates by classification
- `GET /api/analytics/sales-360/breakdown`
  - by product/customer/plan

Optional:

- `GET /api/analytics/sales-360/events`
  - paginated raw ledger events for audits

## UI plan

### Sales 360 section

Single section with clearly labeled blocks:

- Bookings
- Billings
- Collections
- Adjustments
- Recurring (MRR/ARR deltas)

Each block includes:

- current period value
- period-over-period delta
- sparkline or timeseries

## Rollout phases

### Phase 0: Definitions freeze

- Agree event types and metric formulas.

### Phase 1: Ledger schema + emitters

- Add `sales_events` migration.
- Emit events in core Rust flows.

### Phase 2: Backfill + verification

- Backfill historical events.
- Reconcile totals against source tables for sample windows.

### Phase 3: API + dashboard

- Add Sales 360 APIs.
- Build frontend Sales 360 section.

### Phase 4: Test/CI

- Add integration tests for emitters and idempotency.
- Add analytics contract tests.

## Guardrails

- Ledger is append-only; no hard updates to historical facts.
- Always include `source_table/source_id` for traceability.
- Metric definitions versioned in docs to prevent dashboard drift.

## Approved decisions

1. **Bookings scope**: include all deal types (`sale`, `trial`, `partner`) with sub-buckets by `deal_type`.
2. **Recurring metrics**: emit explicit recurring events (`mrr_expanded`, `mrr_contracted`, `mrr_churned`, etc.) instead of deriving only at query time.
3. **Currency strategy**: ship per-currency reporting in V1; defer FX normalization/base-currency conversion to V2.

## Critical guardrails (added)

### A) Reversal strategy (append-only correction model)

Because ledger rows are immutable, corrections must be represented as new events:

- Option 1: emit full reversal + replacement
  - `invoice_issued` `+100`
  - `invoice_reversal` `-100`
  - `invoice_issued` `+80`
- Option 2: emit net adjustment
  - `invoice_adjustment` `-20`

V1 standard:

- use **full reversal + replacement** for deterministic audit trails.
- add optional `reversal_of_event_id` and `superseded_by_event_id` in `metadata`.

### B) Gross vs Net amount semantics

To avoid overstating revenue with tax-inclusive values, ledger rows must separate components.

Recommended columns:

- `amount_subtotal` (net amount before tax)
- `amount_tax` (tax component)
- `amount_total` (gross amount)

Metric rules:

- revenue/bookings analytics default to **net** (`amount_subtotal`)
- tax reporting uses `amount_tax`
- cash collection views can expose `amount_total`

### C) Partitioning strategy for scale

`sales_events` will grow quickly and should be partitioned early.

V1 recommendation:

- PostgreSQL native time partitioning by month on `occurred_at`
  - e.g. `sales_events_2026_03`, `sales_events_2026_04`
- maintain per-partition indexes for `(classification, occurred_at)` and `(customer_id, occurred_at)`

### D) Timezone policy

- Store `occurred_at` and `created_at` in **UTC** only.
- Apply timezone conversion only at read/query time using org-level timezone settings.
- Day/week/month grouping in APIs must accept timezone context and default to org timezone.

## Implementation note

Before Phase 1 migration, define and freeze:

- full event-type catalog
- reversal event naming convention
- amount semantics (`subtotal/tax/total`) per event type
- timezone behavior in analytics endpoints
