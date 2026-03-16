# Comprehensive Test Suite Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add ~100 integration + unit tests covering all Rust API endpoints and Next.js frontend hooks/components.

**Architecture:** Rust tests use `#[sqlx::test]` with auto-provisioned temp PostgreSQL databases. Next.js tests use Vitest + React Testing Library + MSW for fetch mocking. Both projects test against real behavior, not mocks of internal code.

**Tech Stack:** Rust (sqlx, axum-test, tokio), Next.js (vitest, @testing-library/react, msw, jsdom)

**Spec:** `docs/superpowers/specs/2026-03-16-comprehensive-tests-design.md`

---

## Chunk 1: Rust Test Infrastructure

### Task 1: Create SQLx migration from Drizzle schema

Since the Rust backend has no migrations directory (schema is managed by Drizzle/Next.js), we need to create one for `#[sqlx::test]` to auto-provision test databases.

**Files:**
- Create: `rustbill/migrations/20260316000000_init.sql`

- [ ] **Step 1: Generate SQL migration**

Read `/home/shiro/rantai/RantAI-Billing/lib/db/schema.ts` fully and translate ALL tables, enums, indexes, and constraints into a single PostgreSQL migration file. This must include every enum type and every table defined in the Drizzle schema.

The migration file must:
- Create all enum types first (product_type, license_status, trend, etc.)
- Create all tables with correct column types, defaults, and constraints
- Create the `invoice_number_seq` sequence for invoice number generation
- Use `TEXT` for IDs (UUIDs stored as text)
- Use `NUMERIC(12,2)` for monetary fields
- Use `TIMESTAMP` for datetime fields
- Use `JSONB` for JSON fields
- Match the exact column names from the Drizzle schema (snake_case)

- [ ] **Step 2: Verify migration applies**

Run: `cd rustbill && DATABASE_URL="postgresql://rantai_billing:rantai_billing@localhost:5433/rantai_billing_test" sqlx database create && sqlx migrate run`

- [ ] **Step 3: Commit**

```bash
git add rustbill/migrations/
git commit -m "feat: add SQLx migration for test database provisioning"
```

---

### Task 2: Add test dependencies and shared helpers

**Files:**
- Modify: `rustbill/Cargo.toml` — add workspace dev-dependencies
- Modify: `rustbill/crates/rustbill-server/Cargo.toml` — add dev-dependencies
- Create: `rustbill/crates/rustbill-server/tests/common/mod.rs` — shared test helpers

- [ ] **Step 1: Add dev-dependencies to workspace Cargo.toml**

Add under `[workspace.dependencies]`:
```toml
# Testing
axum-test = "16"
```

Add to `rustbill/crates/rustbill-server/Cargo.toml`:
```toml
[dev-dependencies]
axum-test = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
uuid = { workspace = true }
rust_decimal = { workspace = true }
```

- [ ] **Step 2: Create shared test helpers**

Create `rustbill/crates/rustbill-server/tests/common/mod.rs`:

```rust
use axum::Router;
use axum_test::TestServer;
use sqlx::PgPool;
use serde_json::{json, Value};
use uuid::Uuid;

/// Build the full Axum app with the given test pool
pub async fn test_server(pool: PgPool) -> TestServer {
    let config = rustbill_core::config::AppConfig::test_defaults();
    let state = std::sync::Arc::new(rustbill_server::app::AppState {
        db: pool,
        config: std::sync::Arc::new(config),
        http_client: reqwest::Client::new(),
        email_sender: None,
        provider_cache: std::sync::Arc::new(
            rustbill_core::settings::ProviderSettingsCache::empty(),
        ),
    });
    let app = rustbill_server::app::build_router(state);
    TestServer::new(app).unwrap()
}

/// Create an admin user and return a session token
pub async fn create_admin_session(pool: &PgPool) -> String {
    let user_id = Uuid::new_v4().to_string();
    let password_hash = rustbill_core::auth::hash_password("test_password_123").unwrap();

    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, auth_provider, created_at, updated_at)
         VALUES ($1, 'admin@test.com', 'Test Admin', $2, 'admin', 'default', NOW(), NOW())"
    )
    .bind(&user_id)
    .bind(&password_hash)
    .execute(pool)
    .await
    .unwrap();

    let session_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO sessions (id, user_id, expires_at, created_at)
         VALUES ($1, $2, NOW() + INTERVAL '24 hours', NOW())"
    )
    .bind(&session_id)
    .bind(&user_id)
    .execute(pool)
    .await
    .unwrap();

    session_id
}

/// Create a test customer, return its ID
pub async fn create_test_customer(pool: &PgPool) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO customers (id, name, industry, tier, location, contact, email, phone, total_revenue, health_score, trend, last_contact, created_at, updated_at)
         VALUES ($1, 'Test Corp', 'Tech', 'enterprise', 'US', 'John', 'john@test.com', '+1234', 0, 50, 'stable', '2026-01-01', NOW(), NOW())"
    )
    .bind(&id)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Create a test product (licensed type), return its ID
pub async fn create_test_product(pool: &PgPool, product_type: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO products (id, name, product_type, revenue, target, change, created_at, updated_at)
         VALUES ($1, 'Test Product', $2::product_type, 0, 10000, 0, NOW(), NOW())"
    )
    .bind(&id)
    .bind(product_type)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Create a pricing plan, return its ID
pub async fn create_test_plan(pool: &PgPool, product_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO pricing_plans (id, product_id, name, slug, interval, interval_count, amount, currency, trial_days, pricing_model, created_at, updated_at)
         VALUES ($1, $2, 'Basic Plan', 'basic', 'monthly', 1, 29.99, 'USD', 0, 'flat', NOW(), NOW())"
    )
    .bind(&id)
    .bind(product_id)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Create a test subscription, return its ID
pub async fn create_test_subscription(pool: &PgPool, customer_id: &str, plan_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO subscriptions (id, customer_id, plan_id, status, current_period_start, current_period_end, cancel_at_period_end, quantity, version, created_at, updated_at)
         VALUES ($1, $2, $3, 'active', NOW(), NOW() + INTERVAL '1 month', false, 1, 1, NOW(), NOW())"
    )
    .bind(&id)
    .bind(customer_id)
    .bind(plan_id)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Create a test invoice, return its ID
pub async fn create_test_invoice(pool: &PgPool, customer_id: &str) -> String {
    let id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO invoices (id, invoice_number, customer_id, status, subtotal, tax, total, currency, version, created_at, updated_at)
         VALUES ($1, 'INV-TEST-001', $2, 'draft', 100.00, 10.00, 110.00, 'USD', 1, NOW(), NOW())"
    )
    .bind(&id)
    .bind(customer_id)
    .execute(pool)
    .await
    .unwrap();
    id
}

/// Create an API key, return (id, plaintext_key)
pub async fn create_test_api_key(pool: &PgPool) -> (String, String) {
    let id = Uuid::new_v4().to_string();
    let key = format!("rnt_test_{}", Uuid::new_v4().to_string().replace("-", ""));
    let prefix = &key[..12];
    let hashed = rustbill_core::auth::api_key::hash_api_key(&key);

    sqlx::query(
        "INSERT INTO api_keys (id, name, prefix, hashed_key, scopes, created_at)
         VALUES ($1, 'Test Key', $2, $3, '[]'::jsonb, NOW())"
    )
    .bind(&id)
    .bind(prefix)
    .bind(&hashed)
    .execute(pool)
    .await
    .unwrap();
    (id, key)
}
```

Note: This file will need adjustment based on the actual function signatures in `rustbill_core::auth` and `rustbill_server::app`. The implementer should read those modules and adapt.

- [ ] **Step 3: Verify test infrastructure compiles**

Run: `cd rustbill && cargo check --tests`

- [ ] **Step 4: Commit**

```bash
git add rustbill/Cargo.toml rustbill/crates/rustbill-server/Cargo.toml rustbill/crates/rustbill-server/tests/
git commit -m "feat: add test dependencies and shared test helpers"
```

---

## Chunk 2: Rust API Tests (Part A — Core CRUD)

### Task 3: Products tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/products.rs`

- [ ] **Step 1: Write tests**

Tests to implement:
1. `test_list_products` — GET /api/products returns 200 with array
2. `test_create_product` — POST /api/products with valid JSON returns 201
3. `test_create_product_invalid` — POST with missing required fields returns 422
4. `test_get_product` — GET /api/products/{id} returns the product
5. `test_update_product` — PUT /api/products/{id} updates fields
6. `test_delete_product` — DELETE /api/products/{id} returns success

Each test uses `#[sqlx::test(migrations = "migrations")]` to get a fresh DB. Uses `test_server()` and `create_admin_session()` from common helpers. Sends requests with session cookie.

- [ ] **Step 2: Run tests**

Run: `cd rustbill && cargo test --test products -- --test-threads=1`

- [ ] **Step 3: Commit**

```bash
git add rustbill/crates/rustbill-server/tests/products.rs
git commit -m "test: add products API integration tests"
```

---

### Task 4: Customers tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/customers.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_list_customers` — returns computed health scores and trends
2. `test_create_customer` — POST with valid data
3. `test_get_customer` — GET by ID
4. `test_update_customer` — PUT updates fields
5. `test_delete_customer` — DELETE removes customer

- [ ] **Step 2: Run and commit** (same pattern as Task 3)

---

### Task 5: Deals tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/deals.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_list_deals` — returns deals with type filter
2. `test_create_deal_licensed_auto_license` — creating a deal for a licensed product auto-generates a license
3. `test_create_deal_saas_no_license` — SaaS product deal does NOT create a license
4. `test_update_deal` — PUT updates fields
5. `test_delete_deal` — DELETE removes deal

- [ ] **Step 2: Run and commit**

---

### Task 6: Licenses tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/licenses.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_list_licenses` — with status filter
2. `test_create_license` — POST creates license
3. `test_generate_keypair` — POST /api/licenses/keypair creates keypair
4. `test_generate_keypair_conflict` — POST without confirm when keypair exists returns 409
5. `test_sign_license` — POST /api/licenses/{key}/sign signs with keypair
6. `test_verify_license` — POST /api/licenses/verify verifies signed file
7. `test_export_license` — GET /api/licenses/{key}/export returns .lic file
8. `test_delete_license` — DELETE removes license

- [ ] **Step 2: Run and commit**

---

### Task 7: Auth tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/auth.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_login_valid` — POST /api/auth/login with correct credentials returns session cookie
2. `test_login_invalid_password` — wrong password returns 401
3. `test_login_non_admin` — customer role returns 403
4. `test_me_with_session` — GET /api/auth/me returns user info
5. `test_logout` — POST /api/auth/logout clears session

- [ ] **Step 2: Run and commit**

---

### Task 8: API Keys tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/api_keys.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_create_api_key` — returns plaintext key once
2. `test_list_api_keys` — returns keys with masked values
3. `test_revoke_api_key` — DELETE sets revoked_at

- [ ] **Step 2: Run and commit**

---

## Chunk 3: Rust API Tests (Part B — Billing)

### Task 9: Subscriptions tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/subscriptions.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_list_subscriptions` — returns subscriptions
2. `test_create_subscription` — auto-computes period from plan
3. `test_create_subscription_with_trial` — sets trialing status
4. `test_update_subscription_version_ok` — correct version succeeds
5. `test_update_subscription_version_conflict` — wrong version returns 409
6. `test_delete_subscription` — sets canceled status
7. `test_lifecycle_trial_to_active` — expired trial converts to active
8. `test_lifecycle_cancel_at_period_end` — cancels when period ends

- [ ] **Step 2: Run and commit**

---

### Task 10: Invoices tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/invoices.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_list_invoices` — excludes soft-deleted
2. `test_create_invoice` — generates invoice number
3. `test_create_invoice_with_subscription` — generates line items from plan
4. `test_add_line_item` — updates invoice totals
5. `test_update_invoice_version_lock` — correct version succeeds
6. `test_soft_delete_invoice` — sets deleted_at, not hard delete
7. `test_get_pdf` — returns application/pdf content-type

- [ ] **Step 2: Run and commit**

---

### Task 11: Payments tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/payments.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_create_payment` — records payment
2. `test_payment_marks_invoice_paid` — fully paid invoice gets status=paid
3. `test_payment_idempotency` — duplicate stripe_payment_intent_id doesn't create second payment
4. `test_list_payments_with_filter` — invoice filter works

- [ ] **Step 2: Run and commit**

---

### Task 12: Coupons tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/coupons.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_create_coupon` — creates with valid data
2. `test_apply_coupon` — links coupon to subscription, increments times_redeemed
3. `test_apply_coupon_max_redemptions` — fails when maxed out
4. `test_list_auto_deactivates_expired` — expired coupons deactivated on list
5. `test_delete_coupon` — soft deletes

- [ ] **Step 2: Run and commit**

---

### Task 13: Refunds tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/refunds.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_create_refund` — within payment amount succeeds
2. `test_refund_exceeds_payment` — returns 400
3. `test_completed_refund_reverts_invoice` — invoice status goes back to issued
4. `test_list_refunds_customer_isolation` — customer only sees their refunds

- [ ] **Step 2: Run and commit**

---

### Task 14: Dunning tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/dunning.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_mark_overdue` — invoices past due_at become overdue
2. `test_dunning_escalation` — logs correct step per threshold
3. `test_suspension_pauses_subscription` — 30-day dunning pauses sub
4. `test_dunning_skips_processed` — doesn't re-log already-processed steps

- [ ] **Step 2: Run and commit**

---

### Task 15: Analytics tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/analytics.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_overview` — returns expected metric fields (totalRevenue, platformUsers, etc.)
2. `test_forecasting` — returns scenarios, riskFactors, kpis, forecastData
3. `test_reports` — returns conversionData, sourceData, yoyChange

- [ ] **Step 2: Run and commit**

---

### Task 16: Webhooks tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/webhooks.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_stripe_valid_signature` — valid HMAC passes, event recorded
2. `test_stripe_invalid_signature` — bad signature returns 401
3. `test_xendit_valid_token` — correct callback token passes
4. `test_xendit_invalid_token` — wrong token returns 401
5. `test_lemonsqueezy_valid_hmac` — correct HMAC passes

For signature tests: compute the real HMAC using the test webhook secret, then send with correct/incorrect headers.

- [ ] **Step 2: Run and commit**

---

### Task 17: V1 API tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/v1_api.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_v1_no_api_key_returns_401` — request without Authorization header
2. `test_v1_valid_api_key` — returns data
3. `test_v1_products_list` — GET /api/v1/products returns products
4. `test_v1_licenses_crud` — create, list, update, delete license via v1
5. `test_v1_usage_batch` — POST /api/v1/billing/usage with array of events

- [ ] **Step 2: Run and commit**

---

### Task 18: Checkout tests

**Files:**
- Create: `rustbill/crates/rustbill-server/tests/checkout.rs`

- [ ] **Step 1: Write tests**

Tests:
1. `test_checkout_unknown_provider` — returns 400
2. `test_checkout_missing_invoice` — returns 404

Note: We can't test real provider API calls without credentials. Test validation logic only.

- [ ] **Step 2: Run and commit**

---

## Chunk 4: Next.js Frontend Tests

### Task 19: Setup Vitest + MSW

**Files:**
- Modify: `package.json` — add test dependencies and scripts
- Create: `vitest.config.ts` — Vitest configuration
- Create: `__tests__/setup.ts` — MSW server setup

- [ ] **Step 1: Install dependencies**

Run: `bun add -D vitest @testing-library/react @testing-library/jest-dom @testing-library/user-event jsdom msw`

- [ ] **Step 2: Create vitest.config.ts**

```ts
import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  test: {
    environment: "jsdom",
    setupFiles: ["./__tests__/setup.ts"],
    globals: true,
    css: false,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "."),
    },
  },
});
```

- [ ] **Step 3: Create test setup file**

Create `__tests__/setup.ts`:
```ts
import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const handlers = [
  http.get("/api/products", () => {
    return HttpResponse.json([
      { id: "1", name: "Test Product", productType: "licensed" },
    ]);
  }),
  http.get("/api/analytics/overview", () => {
    return HttpResponse.json({
      totalRevenue: "$10,000",
      platformUsers: "500",
      activeLicenses: "100",
      customerCount: "50",
    });
  }),
];

export const server = setupServer(...handlers);

beforeAll(() => server.listen({ onUnhandledRequest: "bypass" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());
```

- [ ] **Step 4: Add test scripts to package.json**

Add to scripts:
```json
"test": "vitest run",
"test:watch": "vitest"
```

- [ ] **Step 5: Verify setup**

Run: `bunx vitest run --passWithNoTests`

- [ ] **Step 6: Commit**

```bash
git add vitest.config.ts __tests__/setup.ts package.json bun.lock
git commit -m "feat: setup Vitest + MSW test infrastructure"
```

---

### Task 20: Hook tests (use-api.ts)

**Files:**
- Create: `__tests__/hooks/use-api.test.ts`

- [ ] **Step 1: Write tests**

Tests using MSW to mock fetch responses:
1. `fetcher returns parsed JSON on 200` — mock 200, verify data returned
2. `fetcher throws error with status on non-200` — mock 500, verify error thrown
3. `mutation returns success result on 200` — call createProduct, check `{success: true, data}`
4. `mutation returns error result on failure` — mock 400, check `{success: false, error, status}`
5. `mutation shows toast on error` — spy on toast.error, verify called
6. `mutation handles timeout` — mock slow response, verify AbortError handling
7. `createProduct calls correct URL` — verify POST /api/products
8. `deleteProduct calls correct URL` — verify DELETE /api/products/{id}
9. `getCheckout returns structured result` — mock checkout response
10. `generateKeypair preserves status code` — mock 409, check result.status

Note: For hook tests that use SWR, use `renderHook` from React Testing Library. For mutation tests (plain async functions), call directly.

- [ ] **Step 2: Run tests**

Run: `bunx vitest run __tests__/hooks/`

- [ ] **Step 3: Commit**

```bash
git add __tests__/hooks/
git commit -m "test: add use-api.ts hook and mutation tests"
```

---

### Task 21: Error boundary tests

**Files:**
- Create: `__tests__/components/error-boundary.test.tsx`

- [ ] **Step 1: Write tests**

Tests:
1. `renders children normally` — render `<ErrorBoundary><div>Hello</div></ErrorBoundary>`, check "Hello" is visible
2. `catches render error and shows fallback` — render a child that throws, verify "Something went wrong" appears
3. `reload button calls window.location.reload` — spy on `window.location.reload`, click button, verify called

- [ ] **Step 2: Run and commit**

---

### Task 22: Backend banner tests

**Files:**
- Create: `__tests__/components/backend-banner.test.tsx`

- [ ] **Step 1: Write tests**

Tests:
1. `banner hidden when backendDown is false` — render with context `backendDown=false`, verify no banner
2. `banner visible when backendDown is true` — render with context `backendDown=true`, verify banner text
3. `dismiss hides banner` — click X button, verify banner disappears
4. `banner reappears after recovery and new failure` — toggle backendDown false→true, verify banner shows again

- [ ] **Step 2: Run and commit**

---

### Task 23: API error component tests

**Files:**
- Create: `__tests__/components/api-error.test.tsx`

- [ ] **Step 1: Write tests**

Tests:
1. `renders default message` — render `<ApiError />`, check "Something went wrong"
2. `renders custom message` — render with `message="Custom"`, check text
3. `retry button calls onRetry` — render with `onRetry` spy, click, verify called

- [ ] **Step 2: Run and commit**

---

### Task 24: Final verification

- [ ] **Step 1: Run all Rust tests**

Run: `cd rustbill && cargo test --test-threads=1 2>&1`
Expected: All tests pass.

- [ ] **Step 2: Run all Next.js tests**

Run: `bunx vitest run`
Expected: All tests pass.

- [ ] **Step 3: Final commit**

```bash
git add -A
git commit -m "test: comprehensive test suite — Rust API integration + Next.js frontend"
```
