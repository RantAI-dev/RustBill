# SOTA Billing Gap-Closure Build Plan

**Date:** 2026-03-18
**Status:** Ready for build execution
**Chosen mode:** Build option 1 (complete auto-charge success settlement)

## Objective

Close remaining gaps between implemented billing engine code and the approved SOTA billing design/spec.

## Ordered execution checklist

- [x] **Chunk 1 (P0): Plan-change financial correctness**
  - [x] Add shared core service for plan change with proration + transaction boundary.
  - [x] Implement `net > 0` upgrade path to create proration invoice immediately.
  - [x] Keep `net < 0` downgrade wallet credit behavior.
  - [x] Return existing invoice payload on idempotency replay.
  - [x] Keep optimistic locking and `subscription.plan_changed` event emission.

- [x] **Chunk 2 (P0): Auto-charge success settlement (build option 1)**
  - [x] Ensure success path creates payment record.
  - [x] Ensure success path marks invoice as paid.
  - [x] Emit `payment.received` and `invoice.paid` events after settlement.
  - [x] Keep transient/permanent failure handling and invoice state behavior.

- [x] **Chunk 3 (P1): Credit wallet concurrency hardening**
  - [x] Make `apply_to_invoice` use atomic guarded deduction semantics.
  - [x] Keep balance update + audit log in one transaction.
  - [x] Preserve same-currency-only application.

- [x] **Chunk 4 (P1): v1 API parity**
  - [x] Add v1 payment-method endpoints (list, setup scaffold, delete, set-default).
  - [x] Add v1 credits endpoint.
  - [x] Enforce customer scoping from auth context.

- [x] **Chunk 5 (P1): Next proxy parity + auth forwarding**
  - [x] Add missing dynamic proxy routes for tax-rules/payment-methods ID actions.
  - [x] Forward cookie headers through Next API proxies to Rust backend.
  - [x] Keep existing hook call signatures stable.

- [x] **Chunk 6 (P2): UI spec alignment**
  - [x] Add `Effective From` column to tax rules management table.
  - [x] Validate invoice amount field mapping (`tax`, `creditsApplied`, `amountDue`) in portal UI.

- [x] **Chunk 7 (P0): Verification gate**
  - [x] Add/extend tests for plan-change upgrade invoice + idempotency replay.
  - [x] Add/extend tests for auto-charge success settlement behavior.
  - [x] Add concurrency test for credits application.
  - [x] Run `cargo test -- --test-threads=1` in `rustbill/`.
  - [x] Run `bun lint`.
  - [x] Run `bun run build`.

## Definition of done

- Billing correctness gaps are closed (plan-change + auto-charge + credits).
- API and proxy layers match spec-required surface and current hook usage.
- UI reflects required tax-rule effective-date metadata.
- Test/build/lint gates pass.
