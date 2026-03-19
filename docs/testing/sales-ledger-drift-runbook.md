# Sales Ledger Drift Runbook

## Purpose

Provide a deterministic way to triage CI failures related to Sales 360 ledger integrity:

- backfill idempotency drift,
- reconciliation mismatch,
- reversal linkage regressions.

## CI Gates Covered

The CI workflow enforces these checks in `.github/workflows/ci.yml`:

- `analytics_sales_360_backfill_is_idempotent`
- `analytics_sales_360_reconcile_returns_drift_shape`
- `analytics_sales_360_summary_supports_currency_breakdown`
- `deleting_invoice_emits_reversal_with_metadata`
- `updating_deal_emits_reversal_and_replacement_events`
- `deleting_completed_refund_emits_reversal_metadata`
- `deleting_credit_note_emits_reversal_with_metadata`

## Triage Flow

1. Re-run the failing test locally in `rustbill/`:

```bash
cargo test --test analytics analytics_sales_360_reconcile_returns_drift_shape -- --test-threads=1
```

2. If mismatch is about duplicate totals, verify idempotency gate first:

```bash
cargo test --test analytics analytics_sales_360_backfill_is_idempotent -- --test-threads=1
```

3. If mismatch is about correction behavior, run the domain-specific reversal test:

```bash
cargo test --test invoices deleting_invoice_emits_reversal_with_metadata -- --test-threads=1
cargo test --test deals updating_deal_emits_reversal_and_replacement_events -- --test-threads=1
cargo test --test refunds deleting_completed_refund_emits_reversal_metadata -- --test-threads=1
cargo test --test credit_notes deleting_credit_note_emits_reversal_with_metadata -- --test-threads=1
```

4. Validate runtime reconcile output shape via API:

```bash
curl -s "http://127.0.0.1:8787/api/analytics/sales-360/reconcile?timezone=UTC" \
  -H "Cookie: session=<admin-session-cookie>"
```

Check fields per classification:

- `ledgerTotal`
- `sourceTotal`
- `delta`
- `eventCount`
- `missingSources`
- `status`

## Partition Maintenance and Pruning Checks

Ensure future partitions exist:

```bash
psql "$DATABASE_URL" -c "SELECT ensure_sales_events_partitions(date_trunc('month', now())::date, 8, 1);"
```

Inspect partitioned query plan for date-bounded scans:

```bash
psql "$DATABASE_URL" -c "EXPLAIN SELECT COALESCE(SUM(amount_total), 0) FROM sales_events WHERE occurred_at >= now() - interval '1 day' AND occurred_at < now() + interval '1 day';"
```

Expected diagnostic outcome:

- planner scans current relevant month partition(s),
- out-of-window future partitions are pruned.

## Root Cause Map

- **`delta != 0` with `missingSources = 0`**
  - likely event emission amount mismatch (wrong subtotal/tax/total sign or value)
  - likely incorrect correction sequence (missing reversal or replacement)

- **`missingSources > 0`**
  - source record was deleted/voided path without corresponding reversal
  - source table/source id mismatch in emitted event metadata

- **idempotency failure**
  - duplicate key guard not applied in new emitter path
  - unstable `source_id` for revision-style events

## Invariants (must always hold)

- `sales_events` is append-only from application behavior.
- Every correction is represented as event-based reversal (and replacement when applicable).
- Reversal metadata includes `reversal_of_event_id` and `reversal_of_event_type`.
- Replacement-linked reversals include `superseded_by_event_id` when available.
- Reconcile checks are evaluated per selected date window and currency context.

## Recovery Actions

- Fix emitter logic; do not mutate old ledger rows.
- Add/adjust reversal event emission in the same write path.
- Add integration test that reproduces the failing sequence.
- Re-run:

```bash
cargo test --test analytics -- --test-threads=1
```

and affected domain tests before pushing.
