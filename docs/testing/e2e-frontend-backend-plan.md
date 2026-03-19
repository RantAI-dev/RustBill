# Frontend-to-Backend E2E Plan

## Goal

Catch integration regressions where UI payloads or API contracts drift from Rust backend behavior.

## Test Tiers

- `@smoke` (PR): login/session + core navigation sanity
- `@core` (PR): deterministic frontend CRUD and critical billing/settings flows
- `@provider` (nightly): provider sandbox flows requiring external credentials

## Coverage Matrix

| Area | Frontend Entry | Backend API | Tier | Status |
|---|---|---|---|---|
| Auth login/logout | `/login`, header avatar | `/api/auth/login`, `/api/auth/logout`, `/api/auth/me` | smoke/core | Implemented |
| Command palette search | Cmd+K | `/api/search` | core | Implemented |
| Products CRUD | Management -> Products | `/api/products` | core | Implemented |
| Customers CRUD | Management -> Customers | `/api/customers` | core | Implemented |
| Plans CRUD | Management -> Pricing Plans | `/api/billing/plans` | core | Implemented |
| Subscriptions CRUD + pre-renewal days | Management -> Subscriptions | `/api/billing/subscriptions` | core | Implemented |
| Manual invoice create/update + payment | Management -> Invoices | `/api/billing/invoices`, `/api/billing/invoices/:id`, `/api/billing/payments` | core | Implemented |
| API key create/revoke | Settings -> API Keys | `/api/api-keys`, `/api/api-keys/:id` | core | Implemented |
| License keypair generate | Settings -> License Signing | `/api/licenses/keypair` | core | Implemented |
| Billing portal load | Portal -> Billing Portal | `/api/billing/*`, `/api/customers` | core | Implemented |
| Coupons CRUD | Management -> Coupons | `/api/billing/coupons` | core | Planned |
| Tax rules CRUD | Management -> Tax Rules | `/api/billing/tax-rules` | core | Planned |
| Webhooks CRUD | Management -> Webhooks | `/api/billing/webhooks` | core | Planned |
| Deals CRUD | Management -> Deals | `/api/deals` | core | Planned |
| Licenses actions | Management -> Licenses | `/api/licenses/*` | core | Planned |
| Credit notes/refunds/payments | Management -> Invoices | `/api/billing/credit-notes`, `/api/billing/refunds`, `/api/billing/payments` | core | Planned |
| Stripe setup sandbox | Billing Portal / API | `/api/billing/payment-methods/setup` | provider | Implemented |
| Xendit setup sandbox | Billing Portal / API | `/api/billing/payment-methods/setup` | provider | Planned |

## CI Rollout

1. PR workflow runs `bun run test:e2e:smoke` and `bun run test:e2e:full`.
2. Nightly workflow runs `bun run test:e2e:providers` with sandbox secrets.
3. Upload Playwright HTML report and trace artifacts for failed tests.

## Local Run

```bash
bun run db:up
bun run db:push
bun run db:seed
bunx playwright install chromium
bun run test:e2e:smoke
bun run test:e2e:full
```

## Required Env

- `DATABASE_URL` for Rust server and Drizzle
- `RUST_BACKEND_URL=http://127.0.0.1:8787` for Next proxy
- `E2E_ADMIN_EMAIL` (default: `evan@rantai.com`)
- `E2E_ADMIN_PASSWORD` (default: `admin123`)

Provider nightly:

- `STRIPE_SECRET_KEY` (required for current provider test)
- `STRIPE_WEBHOOK_SECRET` (optional)
