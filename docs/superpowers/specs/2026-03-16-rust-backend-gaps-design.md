# Rust Backend Gap Implementation — Design Spec

**Date:** 2026-03-16
**Status:** Approved
**Scope:** Implement all missing features in the Rust backend to reach full parity with the Next.js API routes.

## Context

The Rust backend (rustbill) at `rustbill/` is ~70% complete. Core CRUD operations work. The following sub-projects close the remaining gaps.

**Crate structure:**
- `rustbill-core` — business logic, DB queries, models (`crates/rustbill-core/`)
- `rustbill-server` — Axum routes, middleware, extractors (`crates/rustbill-server/`)

## Sub-project 1: Role Isolation Extension

**Gap:** Customer isolation exists on refunds + invoices but not on subscriptions, payments, usage, credit notes.

**Implementation:** In the core service list functions for subscriptions, payments, usage, and credit notes, add optional `role_customer_id: Option<String>` filter. In the route handlers, when the authenticated user has `role == Customer`, pass `auth_user.customer_id` as the filter. Admin users pass `None` (see all).

**Files:**
- `crates/rustbill-core/src/billing/subscriptions.rs` — add customer filter to list query
- `crates/rustbill-core/src/billing/payments.rs` — add customer filter to list query
- `crates/rustbill-core/src/billing/usage.rs` — add customer filter to list query
- `crates/rustbill-core/src/billing/credit_notes.rs` — add customer filter to list query
- `crates/rustbill-server/src/routes/billing/subscriptions.rs` — pass customer_id from auth
- `crates/rustbill-server/src/routes/billing/payments.rs` — pass customer_id from auth
- `crates/rustbill-server/src/routes/billing/usage.rs` — pass customer_id from auth
- `crates/rustbill-server/src/routes/billing/credit_notes.rs` — pass customer_id from auth

## Sub-project 2: License System

**Gap:** License signing returns null stub. No deal→license auto-creation.

### License Signing
Use the `rsa` crate (or `ring`) for RSA-PKCS1v15-SHA256 signing. The Next.js version uses Node crypto with RSA.

**Keypair generation** (`POST /api/licenses/keypair`):
- Generate 2048-bit RSA keypair
- Store PEM-encoded private key and public key in `system_settings` table
- If keypair exists and `confirm` flag not set, return 409

**Sign license** (`POST /api/licenses/{key}/sign`):
- Build payload JSON: `{ key, features, maxActivations, metadata, issuedAt, expiresAt }`
- Sign payload with private key (RSA-PKCS1v15-SHA256)
- Store `signed_payload` and `signature` (base64) on license record

**Verify license** (`POST /api/licenses/verify`):
- Parse license file (base64-decoded JSON with `payload` and `signature` fields)
- Verify signature against public key
- Check expiration from payload

**Export license** (`GET /api/licenses/{key}/export`):
- Return signed license as downloadable `.lic` file (base64-encoded JSON)

### Deal→License Auto-Creation
In the deal creation handler (`POST /api/deals`), after inserting the deal:
- If product.product_type == Licensed, auto-generate a license key
- If keypair exists in system_settings, auto-sign the license
- Store license_key on the deal record

**Files:**
- `crates/rustbill-core/src/licenses/signing.rs` — new file: keypair gen, sign, verify, export
- `crates/rustbill-core/src/licenses/mod.rs` — wire signing module
- `crates/rustbill-server/src/routes/licenses.rs` — implement sign/verify/export/keypair handlers
- `crates/rustbill-core/src/deals/mod.rs` — add auto-license creation logic
- `crates/rustbill-server/src/routes/deals.rs` — call auto-license after deal creation

## Sub-project 3: Auth & Security

### Keycloak OAuth Flow
**Gap:** No `/api/auth/keycloak/login` or `/api/auth/keycloak/callback` endpoints.

**Login** (`GET /api/auth/keycloak/login`):
- Generate random CSRF state
- Store state in a short-lived cookie
- Redirect to Keycloak authorization endpoint with `response_type=code`, `client_id`, `redirect_uri`, `state`

**Callback** (`GET /api/auth/keycloak/callback`):
- Validate CSRF state from cookie matches query param
- Exchange authorization code for tokens via Keycloak token endpoint
- Decode ID token JWT to extract user info (email, name)
- Find or create user in `users` table (with `auth_provider=keycloak`)
- Create local session
- Redirect to `/`

**Files:**
- `crates/rustbill-core/src/auth/keycloak.rs` — token exchange, JWT decode, user provisioning
- `crates/rustbill-server/src/routes/auth.rs` — add keycloak_login and keycloak_callback handlers

### Webhook Signature Verification

**Stripe:** Verify `stripe-signature` header using HMAC-SHA256 with webhook secret. Compare timestamp tolerance (5 min).

**Xendit:** Compare `x-callback-token` header with stored webhook token (timing-safe comparison).

**LemonSqueezy:** Compute HMAC-SHA256 of raw body with webhook secret, compare to `x-signature` header.

**Files:**
- `crates/rustbill-core/src/billing/webhook_verify.rs` — new file: verification functions per provider
- `crates/rustbill-server/src/routes/billing/webhooks_inbound.rs` — call verification before processing

## Sub-project 4: Payment Integration

### Checkout URL Generation
**Gap:** `GET /api/billing/checkout` returns null.

Implement actual API calls to payment providers:
- **Xendit:** Call Xendit Invoice API to create invoice, return `invoice_url`
- **LemonSqueezy:** Call LS Checkout API, return checkout URL

Store provider-specific IDs on the invoice record for webhook reconciliation.

**Files:**
- `crates/rustbill-core/src/billing/checkout.rs` — new file: provider-specific checkout logic
- `crates/rustbill-server/src/routes/billing/checkout.rs` — call checkout service

### Webhook Event Dispatch
**Gap:** Inbound webhooks record events but don't dispatch actions.

After recording a billing event, dispatch based on event type:
- `invoice.paid` → update invoice status to paid, record payment
- `payment.received` → create payment record
- `subscription.canceled` → update subscription status
- `charge.refunded` → create refund record

Also dispatch to outbound webhook endpoints (the user-configured URLs).

**Files:**
- `crates/rustbill-core/src/billing/event_dispatch.rs` — new file: event→action routing
- `crates/rustbill-core/src/billing/webhook_delivery.rs` — deliver to outbound webhook endpoints
- `crates/rustbill-server/src/routes/billing/webhooks_inbound.rs` — call dispatch after recording

### Invoice PDF Generation
**Gap:** `GET /api/billing/invoices/{id}/pdf` returns null.

Use `printpdf` or `genpdf` crate to generate A4 PDF with: company header, bill-to customer, line items table, subtotal/tax/total, payment status, notes.

**Files:**
- `crates/rustbill-core/src/billing/pdf.rs` — new file: PDF generation
- `crates/rustbill-server/src/routes/billing/invoices.rs` — implement pdf handler

## Sub-project 5: Billing Automation

### Full Subscription Lifecycle
**Gap:** Current lifecycle only handles pause/resume/cancel/renew. Missing: trial→active conversion, auto-renewal with invoice generation, coupon application, tiered/usage-based pricing.

Implement cron-triggered lifecycle that:
1. Converts expired trials → active
2. Cancels subscriptions with `cancel_at_period_end` flag past period end
3. Renews active subscriptions past period end:
   - Generate invoice with line items from plan
   - Apply active coupon discounts
   - Calculate tiered/usage-based amounts
   - Send email notification

**Files:**
- `crates/rustbill-core/src/billing/lifecycle.rs` — new/expand: full lifecycle processing
- `crates/rustbill-server/src/routes/billing/cron.rs` — trigger lifecycle from cron

### Email Notifications
**Gap:** EmailSender initialized but never called.

Send emails via Resend for:
- Payment received confirmation
- Invoice paid notification
- Invoice issued notification
- Subscription renewal notification

**Files:**
- `crates/rustbill-core/src/billing/notifications.rs` — new file: email templates and sending
- Wire into: invoice creation, payment creation, lifecycle processing

### Dunning Process
**Gap:** `POST /api/billing/cron/process-dunning` is a stub.

Implement grace period logic:
- 3 days overdue → reminder
- 7 days → warning
- 14 days → final notice
- 30 days → suspend subscription

Log each step in `dunning_log` table.

**Files:**
- `crates/rustbill-core/src/billing/dunning.rs` — implement dunning logic
- `crates/rustbill-server/src/routes/billing/cron.rs` — wire dunning into cron

## Sub-project 6: Analytics Enhancement

### Forecasting
**Gap:** Naive linear projection vs Next.js's scenarios/risk factors/KPIs.

Implement:
- Quarterly breakdown (committed/best case/projected)
- Risk factors (overdue invoices, at-risk subscriptions, low-health customers)
- Scenarios (conservative/base/optimistic with growth multipliers)
- KPIs (quarter forecast, forecast accuracy, deal coverage, at-risk revenue)

**Files:**
- `crates/rustbill-core/src/analytics/forecasting.rs` — expand forecasting logic

### Reports
**Gap:** Minimal reports vs conversion rates, revenue by type, YoY change.

Implement:
- Conversion rates from deals
- Revenue grouped by product type
- YoY change calculation
- Recent invoices as reports

**Files:**
- `crates/rustbill-core/src/analytics/reports.rs` — expand reports logic

## Sub-project 7: V1 Public API

**Gap:** V1 API only has licenses/verify and licenses/{key}/activations. Missing all other endpoints.

Add v1 routes (all with API key auth):
- Products: GET, GET/:id
- Customers: CRUD
- Deals: CRUD
- Licenses: CRUD + activations
- Billing/invoices: GET, GET/:id
- Billing/subscriptions: CRUD
- Billing/usage: GET, POST (batch)

Follow the same patterns as admin routes but with API key middleware and customer scoping where appropriate.

**Files:**
- `crates/rustbill-server/src/routes/v1/products.rs` — new
- `crates/rustbill-server/src/routes/v1/customers.rs` — new
- `crates/rustbill-server/src/routes/v1/deals.rs` — new
- `crates/rustbill-server/src/routes/v1/licenses.rs` — expand
- `crates/rustbill-server/src/routes/v1/billing.rs` — new (invoices, subscriptions, usage)
- `crates/rustbill-server/src/routes/v1/mod.rs` — wire all v1 routes
- `crates/rustbill-server/src/app.rs` — mount v1 router

## Dependency Order

```
Wave 1 (parallel): SP1, SP2, SP3, SP6
Wave 2 (parallel): SP4, SP7
Wave 3: SP5
```

SP4 depends on SP3 (webhook signatures). SP5 depends on SP4 (payment dispatch for lifecycle). SP7 depends on SP1 (role isolation).
