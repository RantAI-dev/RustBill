# Sales Ledger Hardening Implementation Plan

**Date:** 2026-03-19  
**Status:** Completed  
**Owner:** Billing/Analytics

## Execution Progress

- Phase 1 reversal coverage is implemented across invoices, deals, refunds, and credit notes with metadata linkage.
- Phase 2 partition migration is implemented for `sales_events` with partition-aware idempotency via `sales_event_idempotency_keys`.
- Phase 3 CI reconciliation gates are implemented in `.github/workflows/ci.yml`.
- Phase 4 runbook/docs updates are implemented:
  - `docs/testing/sales-ledger-drift-runbook.md`
  - `README.md` documentation link.
- Optional hardening completed in the same run:
  - partition maintenance function migration (`ensure_sales_events_partitions`)
  - partition-pruning diagnostics tests included in CI gate.

Notes:

- Optional future hardening can be tracked in separate follow-up plans.

## Objective

Close the remaining compliance gaps for the approved Sales Ledger architecture by:

1. standardizing append-only reversal coverage across all correction flows,
2. introducing partition-ready `sales_events` storage for scale,
3. preserving current API/UI contracts while improving audit guarantees.

## Current State (What is already done)

- `sales_events` ledger exists and is append-only by behavior.
- Idempotency guard exists (`source_table`, `source_id`, `event_type`).
- Sales 360 endpoints and dashboard are live.
- Currency-aware reporting (V1) is live.
- Timezone-aware query grouping is live.
- Gross/net/tax fields are present (`amount_subtotal`, `amount_tax`, `amount_total`).
- Reversal metadata pattern exists in some domains (invoice + deal corrections).

## Remaining Gaps

1. Reversal conventions are not yet uniformly applied across all correction-capable flows.
2. `sales_events` is not partitioned yet.
3. Reconciliation and drift checks are not enforced in CI as a blocking quality gate.

## Design Principles

- **Append-only facts**: no updates/deletes of historical ledger facts.
- **Traceability**: every correction event must point to prior facts.
- **Idempotency-first**: retries must be safe.
- **Compatibility-first rollout**: no breaking API changes for current frontend.

## Event Correction Standard (to enforce)

For any correction flow (void, cancel, overwrite, replacement):

1. Emit a `*.reversal` event with negative amounts for the superseded fact.
2. Emit replacement event (if applicable) with corrected positive amounts.
3. Include metadata:
   - `reversal_of_event_id`
   - `reversal_of_event_type`
   - `superseded_by_event_id` (on reversal when replacement exists)
   - `trigger`
   - `reason`

Amount rules:

- reversal amount must exactly negate prior `amount_subtotal/tax/total`.
- replacement event must represent corrected canonical values.

## Partitioning Strategy (V1)

Adopt PostgreSQL native range partitioning on `occurred_at` by month.

### Target shape

- Parent table: `sales_events` partitioned by range (`occurred_at`).
- Child partitions: monthly (`sales_events_YYYY_MM`).
- Per-partition indexes:
  - `(classification, occurred_at)`
  - `(customer_id, occurred_at)`
  - `(event_type, occurred_at)`

### Idempotency in partitioned model

Because global unique constraints can be restrictive with partitioning, we will preserve correctness via:

1. same uniqueness key semantics (`source_table`, `source_id`, `event_type`) on each partition, and
2. a guard routine in emit path for cross-partition retry safety when needed.

If PostgreSQL constraint strategy requires `occurred_at` in unique keys, adjust key to include it while preserving logical idempotency checks in code.

## Implementation Phases

## Phase 1 - Reversal Coverage Completion

Scope:

- Identify all correction-capable flows in:
  - invoices
  - subscriptions (status/plan corrections)
  - refunds/credit notes (if amendment paths exist)
  - deals (already partially covered; finalize metadata linking)

Deliverables:

- Unified helper(s) for reversal+replacement emission.
- Metadata link completeness across all correction paths.
- Integration tests per domain ensuring reversal linkage fields exist.

Acceptance criteria:

- Every tested correction emits either:
  - reversal-only, or
  - reversal+replacement
  with valid linkage metadata.

## Phase 2 - Partition Migration

Scope:

- Add migration(s) to convert or recreate `sales_events` as partitioned table.
- Create current + near-future partitions.
- Add maintenance routine for monthly partition creation.

Deliverables:

- New migration SQL files.
- Backfill compatibility on partitioned table.
- Read/query behavior unchanged for Sales 360 endpoints.

Acceptance criteria:

- All existing analytics tests pass unchanged.
- Backfill and emitters write/read correctly from partitions.
- Query plans show partition pruning on date-bounded queries.

## Phase 3 - CI Reconciliation Gate

Scope:

- Add CI job steps to validate:
  - analytics tests,
  - backfill idempotency,
  - reconciliation endpoint consistency for fixture windows.

Deliverables:

- Workflow updates in `.github/workflows/ci.yml`.
- Deterministic reconciliation test fixture(s).

Acceptance criteria:

- PRs fail when reconciliation invariants drift.
- Nightly run includes reconciliation checks on broader fixture set.

## Phase 4 - Operational Readiness

Scope:

- Add runbook notes for ledger reconciliation and correction handling.
- Add dashboard-level drift visibility notes.

Deliverables:

- Documentation update in `docs/testing/` and/or `README.md`.

Acceptance criteria:

- Team can diagnose drift using documented steps only.

## Test Plan

- Unit/integration tests for each correction flow:
  - assert reversal event emitted
  - assert linkage metadata present and valid
  - assert net totals reconcile to corrected state
- Partition tests:
  - writes land in correct partition
  - date range queries prune partitions
- Existing suites must remain green:
  - `analytics`
  - `invoices`
  - `deals`
  - `subscriptions`
  - `billing_pipeline`

## Rollout and Safety

- Rollout order:
  1. merge reversal coverage,
  2. deploy partition migration in maintenance window,
  3. run backfill idempotency check,
  4. enable CI reconciliation gate.
- No destructive rewrite of historical ledger rows.
- Fallback: keep read APIs pointed to parent `sales_events` interface so implementation details stay transparent.

## Risks and Mitigations

- **Risk:** partition uniqueness semantics differ from single-table behavior.  
  **Mitigation:** enforce logical idempotency in code + partition-local constraints.
- **Risk:** correction events double-emitted under retries.  
  **Mitigation:** stable source keys for revisions + ON CONFLICT guards.
- **Risk:** reconciliation false positives on mixed currencies.  
  **Mitigation:** reconcile per-currency windows by default.

## Definition of Done

This hardening track is complete when:

1. correction flows all emit linked reversal events,
2. `sales_events` is partitioned and performance-safe,
3. reconciliation checks are CI-enforced,
4. docs/runbooks reflect the finalized operating model.
