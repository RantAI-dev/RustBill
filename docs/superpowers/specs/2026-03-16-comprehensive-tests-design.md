# Comprehensive Test Suite — Design Spec

**Date:** 2026-03-16
**Status:** Approved
**Scope:** Integration tests for Rust backend API endpoints + frontend hook/component tests for Next.js.

## Context

RantAI Billing has a Rust backend (Axum + SQLx) with ~100 API endpoints and a Next.js frontend. Tests should cover API contracts, business logic, and the frontend data layer without requiring paid infrastructure.

## Rust Backend Tests

### Framework
- `#[sqlx::test]` — auto-provisions a temp PostgreSQL database per test, applies migrations, tears down after
- `axum_test::TestServer` or direct service calls against real DB
- No database mocking

### Shared Helpers (`tests/common/mod.rs`)
- `create_test_user(pool)` — insert admin user, return session token
- `create_test_customer(pool)` — insert test customer with billing fields
- `create_test_product(pool, product_type)` — insert product (licensed/saas/api)
- `create_test_plan(pool, product_id)` — insert pricing plan
- `create_test_invoice(pool, customer_id)` — insert draft invoice
- `create_test_subscription(pool, customer_id, plan_id)` — insert active subscription
- `app_with_pool(pool)` — build Axum router with test pool, return test client
- `auth_header(token)` — build cookie header for authenticated requests

### Test Files

#### `tests/products.rs` (~5 tests)
- List products returns computed metrics (revenue, MoM change)
- Create product with valid data succeeds
- Create product with invalid data returns 422
- Update product fields
- Delete product returns 204

#### `tests/customers.rs` (~5 tests)
- List customers returns computed health scores and trends
- Create customer with billing fields
- Get customer by ID
- Update customer
- Delete customer

#### `tests/deals.rs` (~5 tests)
- List deals with type filter
- Create deal for licensed product auto-generates license
- Create deal for non-licensed product does not create license
- Update deal
- Delete deal

#### `tests/licenses.rs` (~8 tests)
- List licenses with status filter
- Create license
- Generate keypair
- Generate keypair with existing keypair returns 409 without confirm
- Sign license with keypair
- Verify signed license file
- Export signed license as .lic
- Delete license

#### `tests/auth.rs` (~5 tests)
- Login with valid credentials returns session cookie
- Login with invalid credentials returns 401
- Login with non-admin user returns 403
- GET /me with valid session returns user
- Logout clears session

#### `tests/subscriptions.rs` (~8 tests)
- List subscriptions
- Create subscription auto-computes period from plan
- Create subscription with trial sets trialing status
- Update subscription with correct version succeeds
- Update subscription with wrong version returns 409
- Delete subscription sets canceled status
- Lifecycle: expired trial converts to active
- Lifecycle: cancel_at_period_end cancels subscription

#### `tests/invoices.rs` (~7 tests)
- List invoices excludes soft-deleted
- Create invoice generates invoice number
- Create invoice with subscription generates line items
- Add line item updates totals
- Update invoice with version lock
- Soft delete invoice
- Generate PDF returns application/pdf content-type

#### `tests/payments.rs` (~4 tests)
- Create payment records successfully
- Create payment updates invoice to paid when fully paid
- Duplicate stripe_payment_intent_id is idempotent
- List payments with invoice filter

#### `tests/coupons.rs` (~5 tests)
- Create coupon
- Apply coupon to subscription
- Apply coupon at max redemptions fails
- List coupons auto-deactivates expired
- Delete coupon

#### `tests/refunds.rs` (~4 tests)
- Create refund within payment amount succeeds
- Create refund exceeding payment amount fails
- Completed refund reverts invoice status to issued
- List refunds with customer isolation

#### `tests/dunning.rs` (~4 tests)
- Mark overdue invoices past due_at
- Dunning escalation logs correct step per threshold
- Suspension step pauses subscription
- Dunning skips already-processed steps

#### `tests/analytics.rs` (~3 tests)
- Overview returns expected metric fields
- Forecasting returns scenarios, risk factors, KPIs
- Reports returns conversion rates and revenue by type

#### `tests/webhooks.rs` (~5 tests)
- Valid Stripe signature passes verification
- Invalid Stripe signature returns 401
- Valid Xendit callback token passes
- Invalid Xendit token returns 401
- LemonSqueezy HMAC verification

#### `tests/v1_api.rs` (~5 tests)
- Request without API key returns 401
- Request with valid API key returns data
- V1 products list
- V1 licenses CRUD
- V1 usage batch POST

#### `tests/checkout.rs` (~2 tests)
- Checkout with unknown provider returns 400
- Checkout with valid provider returns checkout URL (mocked HTTP)

**Total: ~75 Rust tests**

## Next.js Frontend Tests

### Framework
- Vitest (fast, ESM-native, Bun-compatible)
- React Testing Library for component tests
- MSW (Mock Service Worker) for fetch interception in hook tests

### Configuration
- `vitest.config.ts` at project root
- `__tests__/setup.ts` for MSW server setup
- Add to package.json: `"test": "vitest run"`, `"test:watch": "vitest"`

### Test Files

#### `__tests__/hooks/use-api.test.ts` (~10 tests)
- SWR fetcher returns parsed JSON on 200
- SWR fetcher throws error with status on non-200
- Mutation helper returns `{success: true, data}` on 200
- Mutation helper returns `{success: false, error, status}` on error
- Mutation helper shows toast.error on failure
- Mutation helper handles timeout (AbortController)
- createProduct calls correct URL with POST
- deleteProduct calls correct URL with DELETE
- getCheckout returns structured result
- generateKeypair preserves status code on error

#### `__tests__/components/error-boundary.test.tsx` (~3 tests)
- Renders children normally when no error
- Catches render error and shows fallback UI
- Reload button calls window.location.reload

#### `__tests__/components/backend-banner.test.tsx` (~4 tests)
- Banner hidden when backendDown is false
- Banner visible when backendDown is true
- Dismiss hides banner
- Banner reappears after recovery + new failure

#### `__tests__/components/api-error.test.tsx` (~3 tests)
- Renders default error message
- Renders custom message
- Retry button calls onRetry callback

#### `__tests__/middleware.test.ts` (~5 tests)
- Responses include security headers
- API requests return 503 when RUST_BACKEND_URL is set
- /health passes through without auth
- /login redirects to / when session cookie present
- Dev mode preserves existing auth flow

**Total: ~25 Next.js tests**

## Dependencies to Add

### Rust (`Cargo.toml`)
```toml
[dev-dependencies]
axum-test = "16"
sqlx = { features = ["runtime-tokio", "postgres", "migrate"] }
tokio = { features = ["macros", "rt-multi-thread"] }
serde_json = "1"
```

### Next.js (`package.json`)
```json
{
  "devDependencies": {
    "vitest": "^3",
    "@testing-library/react": "^16",
    "@testing-library/jest-dom": "^6",
    "jsdom": "^25",
    "msw": "^2"
  }
}
```

## Out of Scope
- E2E browser tests (Playwright — needs compute)
- Load/stress testing
- Visual regression tests
- Testing shadcn/ui components
- Testing dashboard section render output
