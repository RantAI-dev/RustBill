# Production Readiness Design — RantAI Billing Frontend

**Date:** 2026-03-16
**Status:** Approved
**Scope:** Make the Next.js frontend stable and production-ready as a pure frontend shell proxying all API calls to the Rust backend.

## Context

RantAI Billing uses Next.js 16 as a frontend dashboard. All API logic, auth, database, and billing is handled by a Rust backend (Axum + SQLx). Next.js proxies `/api/*` requests to the Rust backend via `next.config.mjs` rewrites.

**Deployment:** Docker Compose multi-container on a single server.
**Payment providers:** Xendit + LemonSqueezy (handled by Rust).
**Next.js API routes:** Kept as reference but disabled in production.

## Section 1: Next.js Config Fixes

### `next.config.mjs`
- Set `typescript.ignoreBuildErrors` to `false` — enforce TypeScript correctness at build time.
- Add validation: if `RUST_BACKEND_URL` is not set in production (`NODE_ENV=production`), fail loudly at startup instead of silently serving dead Next.js routes.
- Keep `images.unoptimized: true` (acceptable for internal dashboard).

### Environment Validation
- Create `.env.example` documenting all required variables.
- Add a startup check in `instrumentation.ts` (Next.js instrumentation hook) that validates critical env vars (`RUST_BACKEND_URL` in production) and throws a clear error on boot if missing.

## Section 2: Disable Next.js API Routes

### Strategy
Add a middleware gate that blocks all `/api/*` requests when `RUST_BACKEND_URL` is configured.

### Behavior
- When `RUST_BACKEND_URL` is set and a request reaches Next.js `/api/*` directly (i.e., the rewrite didn't proxy it), return `503 {"error": "API served by backend service"}`.
- Acts as a safety net — the rewrite should handle requests first, but if it doesn't, stale Next.js logic is never served.
- In local dev without Rust backend, API routes remain usable by leaving `RUST_BACKEND_URL` unset.

### Middleware Placement
When `RUST_BACKEND_URL` is set, the 503 gate replaces the entire existing middleware auth flow (session checks, CORS handling, v1 API logic). The existing middleware branches are only relevant when Next.js serves its own API routes — which doesn't happen in production. The middleware simplifies to:
1. Early return for static/public paths (`/health`, `/_next`, `/login`)
2. If `/api/*` request and `RUST_BACKEND_URL` is set → 503 (safety net)
3. Apply security headers to all responses
4. Pass through for all other page routes

## Section 3: Security Headers in Middleware

Add standard security headers to all responses via `middleware.ts`:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Frame-Options` | `DENY` | Prevent clickjacking |
| `X-Content-Type-Options` | `nosniff` | Prevent MIME sniffing |
| `X-XSS-Protection` | `0` | Disable legacy XSS filter |
| `Referrer-Policy` | `strict-origin-when-cross-origin` | Limit referrer leakage |
| `Permissions-Policy` | `camera=(), microphone=(), geolocation=()` | Disable unused browser APIs |
| `Content-Security-Policy` | `default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; font-src 'self' https://fonts.gstatic.com` | Restrict resource sources |

**Note:** `connect-src` is `'self'` only — the browser never connects directly to the Rust backend. All `/api/*` calls go to the same origin, and Next.js proxies them server-side via rewrites. The internal Docker hostname is not resolvable from the browser.

**Not included:** HSTS (set at reverse proxy level), CSP nonces (overkill for current stage).

## Section 4: Frontend Resilience

### Error Boundary
- Create `components/error-boundary.tsx` as a `"use client"` class component.
- Import and wrap `{children}` in `app/layout.tsx` (which remains a Server Component — do not convert it to a client component, as it exports `metadata`).
- Catches render crashes and shows a recovery UI with a "Reload" button instead of a white screen.

### API Hook Hardening (`hooks/use-api.ts`)
- Add SWR global config: `onErrorRetry` with exponential backoff, max 3 retries.
- Add fetch timeout (10s default) so UI doesn't hang if Rust backend is unresponsive.
- Mutation helpers (`createProduct`, `updateDeal`, etc.) — wrap in try/catch, return structured `{success, error}` result.
- Show user-friendly toast on network/server errors.

### Error States
- Dashboard sections currently check `isLoading` but ignore `error` from SWR hooks.
- Add a reusable `<ApiError>` component showing "Something went wrong" with a retry button.
- Sections that use SWR hooks must render this component when `error` is present.

## Section 5: Docker Compose Production

### Service Changes (`docker-compose.prod.yml`)
- Add Rust backend service container (placeholder image, configurable).
- Fix health check: replace `curl -f http://localhost:3000/login` with `/health` endpoint.
- Add resource limits (`mem_limit`, `cpus`) to prevent runaway containers.
- Remove hardcoded default password — require via environment. **Breaking change** for existing deployments relying on the default; document in `.env.example`.
- Dependency ordering: postgres → rust-backend → nextjs-frontend.
- All services: `restart: unless-stopped`.

### Health Endpoint
- Add `/health` page route in Next.js (not an API route) that returns 200.
- **Must be excluded from middleware auth** — add `/health` to the public paths early-return list so Docker health checks work without a session cookie.
- Optionally checks if Rust backend is reachable and reports status.

### `.env.example`
- Document all required variables for both dev and Docker Compose production.

## Section 6: Proxy Reliability

### Rewrite Config (`next.config.mjs`)
- Validate `RUST_BACKEND_URL` is a proper URL (not just truthy) at build/startup.
- Change `rewrites()` return shape from array to object: `{ beforeFiles: [...] }` to ensure proxy takes priority over file routes.

### Backend-Down Handling
- SWR retry logic from Section 4 handles transient failures.
- After retries exhausted, show a global "Backend unavailable" banner at the top of the dashboard.
- Health page reports backend connectivity status.

### Banner Mechanism
- Create a `BackendStatusProvider` React context in `components/backend-banner.tsx` (`"use client"`).
- SWR global `onError` handler sets a context flag `backendDown = true` when consecutive errors exceed retry threshold.
- Any successful SWR response clears the flag (`backendDown = false`).
- `<BackendBanner>` component consumes the context, renders a dismissible warning bar at the top of `app/page.tsx` (inside the main dashboard layout, above the content area).
- Mount the provider in `app/page.tsx` wrapping the dashboard content (not in `layout.tsx`, since it only applies to the authenticated dashboard).

## Out of Scope

- Rust backend hardening (separate project)
- Heavy test coverage (per user request — lightweight stability, not TDD)
- Stripe integration (not used)
- Redis/distributed rate limiting (single server, not needed)
- HSTS/SSL termination (handled by reverse proxy)
- CSP nonces, CSRF tokens (overkill for pure frontend proxy)
- Removing Next.js API route files (kept as reference)

## Files to Create/Modify

| File | Action | Section |
|------|--------|---------|
| `next.config.mjs` | Modify | 1, 6 |
| `instrumentation.ts` | Create | 1 |
| `.env.example` | Create | 1, 5 |
| `middleware.ts` | Modify | 2, 3 |
| `app/layout.tsx` | Modify | 4 |
| `components/error-boundary.tsx` | Create | 4 |
| `components/api-error.tsx` | Create | 4 |
| `components/backend-banner.tsx` | Create | 6 |
| `hooks/use-api.ts` | Modify | 4 |
| `docker-compose.prod.yml` | Modify | 5 |
| `app/health/page.tsx` | Create | 5 |
