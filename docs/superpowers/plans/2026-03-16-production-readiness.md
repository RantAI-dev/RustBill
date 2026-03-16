# Production Readiness Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Next.js frontend stable and production-ready as a pure proxy to the Rust backend.

**Architecture:** Next.js serves only the frontend UI. All `/api/*` calls are proxied to a Rust backend via `next.config.mjs` rewrites. Middleware gates stale Next.js API routes, adds security headers, and handles auth redirects.

**Tech Stack:** Next.js 16, TypeScript, SWR, Tailwind CSS v4, Docker Compose, Bun

**Spec:** `docs/superpowers/specs/2026-03-16-production-readiness-design.md`

---

## Chunk 1: Configuration & Infrastructure

### Task 1: Fix next.config.mjs

**Files:**
- Modify: `next.config.mjs`

- [ ] **Step 1: Set ignoreBuildErrors to false and restructure rewrites**

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  typescript: {
    ignoreBuildErrors: false,
  },
  images: {
    unoptimized: true,
  },
  async rewrites() {
    const rustBackend = process.env.RUST_BACKEND_URL;
    if (!rustBackend) return { beforeFiles: [] };
    try {
      new URL(rustBackend);
    } catch {
      throw new Error(`Invalid RUST_BACKEND_URL: "${rustBackend}" — must be a valid URL (e.g., http://rust-backend:8080)`);
    }
    return {
      beforeFiles: [
        {
          source: "/api/:path*",
          destination: `${rustBackend}/api/:path*`,
        },
      ],
    };
  },
};

export default nextConfig;
```

- [ ] **Step 2: Verify config loads**

Run: `bun run build 2>&1 | head -30`
Expected: Build starts (may have TS errors now that `ignoreBuildErrors` is false — that's expected and will be fixed later).

- [ ] **Step 3: Commit**

```bash
git add next.config.mjs
git commit -m "fix: enforce TypeScript errors at build, validate RUST_BACKEND_URL, use beforeFiles rewrite"
```

---

### Task 2: Create instrumentation.ts for env validation

**Files:**
- Create: `instrumentation.ts` (project root, Next.js auto-loads this)

- [ ] **Step 1: Create instrumentation file**

```ts
export async function register() {
  if (process.env.NODE_ENV === "production" && !process.env.RUST_BACKEND_URL) {
    throw new Error(
      "RUST_BACKEND_URL is required in production. " +
      "Set it to the Rust backend URL (e.g., http://rust-backend:8080)."
    );
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add instrumentation.ts
git commit -m "feat: add startup env validation for production"
```

---

### Task 3: Create .env.example

**Files:**
- Create: `.env.example`

- [ ] **Step 1: Create env example file**

```env
# === Required in Production ===

# Rust backend URL — all /api/* calls are proxied here
RUST_BACKEND_URL=http://rust-backend:8080

# === Docker Compose Production ===

# PostgreSQL password (no default — must be set)
POSTGRES_PASSWORD=

# App port mapping (default: 13300)
APP_PORT=13300

# === Optional ===

# Auth provider: "default" (local password) or "keycloak"
AUTH_PROVIDER=default

# Keycloak (required if AUTH_PROVIDER=keycloak)
# KEYCLOAK_URL=
# KEYCLOAK_REALM=
# KEYCLOAK_CLIENT_ID=
# KEYCLOAK_CLIENT_SECRET=

# Session secret for cookie signing
# SESSION_SECRET=

# Cron secret for scheduled job auth
# CRON_SECRET=

# Xendit API key
# XENDIT_SECRET_KEY=
# XENDIT_WEBHOOK_TOKEN=

# LemonSqueezy API key
# LEMONSQUEEZY_API_KEY=
# LEMONSQUEEZY_WEBHOOK_SECRET=

# Email (Resend)
# RESEND_API_KEY=
# EMAIL_FROM=
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "docs: add .env.example with all configuration variables"
```

---

### Task 4: Rewrite middleware.ts

**Files:**
- Modify: `middleware.ts`

This is a full rewrite. When `RUST_BACKEND_URL` is set, the middleware simplifies to: public path passthrough → API 503 safety net → security headers. When it's not set (local dev), the existing auth flow is preserved.

- [ ] **Step 1: Rewrite middleware**

```ts
import { NextRequest, NextResponse } from "next/server";

const RUST_BACKEND = process.env.RUST_BACKEND_URL;
const SESSION_COOKIE = "session";

const SECURITY_HEADERS: Record<string, string> = {
  "X-Frame-Options": "DENY",
  "X-Content-Type-Options": "nosniff",
  "X-XSS-Protection": "0",
  "Referrer-Policy": "strict-origin-when-cross-origin",
  "Permissions-Policy": "camera=(), microphone=(), geolocation=()",
  "Content-Security-Policy": [
    "default-src 'self'",
    "script-src 'self' 'unsafe-inline' 'unsafe-eval'",
    "style-src 'self' 'unsafe-inline'",
    "connect-src 'self'",
    "img-src 'self' data:",
    "font-src 'self' https://fonts.gstatic.com",
  ].join("; "),
};

function withSecurityHeaders(response: NextResponse): NextResponse {
  for (const [key, value] of Object.entries(SECURITY_HEADERS)) {
    response.headers.set(key, value);
  }
  return response;
}

export function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;

  // ---- 1. Health endpoint — always public ----
  if (pathname === "/health") {
    return withSecurityHeaders(NextResponse.next());
  }

  // ---- 2. Production mode: Rust backend handles all API logic ----
  if (RUST_BACKEND) {
    // Safety net: if a request reaches Next.js /api/* directly
    // (rewrite didn't proxy it), block it.
    if (pathname.startsWith("/api/")) {
      const resp = new NextResponse(
        JSON.stringify({ error: "API served by backend service" }),
        {
          status: 503,
          headers: { "Content-Type": "application/json" },
        }
      );
      return withSecurityHeaders(resp);
    }

    // Login page — redirect to / if already has session cookie
    if (pathname === "/login") {
      if (req.cookies.has(SESSION_COOKIE)) {
        return withSecurityHeaders(NextResponse.redirect(new URL("/", req.url)));
      }
      return withSecurityHeaders(NextResponse.next());
    }

    // All other page routes — pass through with security headers
    return withSecurityHeaders(NextResponse.next());
  }

  // ---- 3. Dev mode (no RUST_BACKEND_URL): existing auth flow ----

  // Public API (v1) — CORS only, no auth
  if (pathname.startsWith("/api/v1/")) {
    if (req.method === "OPTIONS") {
      return new NextResponse(null, {
        status: 204,
        headers: {
          "Access-Control-Allow-Origin": "*",
          "Access-Control-Allow-Methods": "GET, POST, PUT, DELETE, OPTIONS",
          "Access-Control-Allow-Headers": "Content-Type, Authorization",
          "Access-Control-Max-Age": "86400",
        },
      });
    }
    const response = NextResponse.next();
    response.headers.set("Access-Control-Allow-Origin", "*");
    response.headers.set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
    response.headers.set("Access-Control-Allow-Headers", "Content-Type, Authorization");
    return withSecurityHeaders(response);
  }

  // Auth routes — always public
  if (pathname.startsWith("/api/auth/")) {
    return withSecurityHeaders(NextResponse.next());
  }

  // Login page — redirect to / if already has session cookie
  if (pathname === "/login") {
    if (req.cookies.has(SESSION_COOKIE)) {
      return withSecurityHeaders(NextResponse.redirect(new URL("/", req.url)));
    }
    return withSecurityHeaders(NextResponse.next());
  }

  // All other routes — require session cookie
  const hasSession = req.cookies.has(SESSION_COOKIE);
  if (!hasSession) {
    if (pathname.startsWith("/api/")) {
      return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
    }
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("from", pathname);
    return NextResponse.redirect(loginUrl);
  }

  return withSecurityHeaders(NextResponse.next());
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico|icon.*|apple-icon.*|manifest|logo/).*)",
  ],
};
```

- [ ] **Step 2: Commit**

```bash
git add middleware.ts
git commit -m "feat: add security headers, API route gate for production, health endpoint passthrough"
```

---

## Chunk 2: Frontend Resilience

### Task 5: Create error boundary component

**Files:**
- Create: `components/error-boundary.tsx`

- [ ] **Step 1: Create error boundary**

```tsx
"use client";

import React from "react";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

interface Props {
  children: React.ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error("ErrorBoundary caught:", error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex min-h-screen items-center justify-center bg-background">
          <div className="text-center space-y-4 max-w-md px-6">
            <AlertTriangle className="h-12 w-12 text-destructive mx-auto" />
            <h2 className="text-xl font-semibold text-foreground">
              Something went wrong
            </h2>
            <p className="text-sm text-muted-foreground">
              An unexpected error occurred. Try reloading the page.
            </p>
            <Button
              onClick={() => {
                this.setState({ hasError: false, error: null });
                window.location.reload();
              }}
            >
              Reload Page
            </Button>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}
```

- [ ] **Step 2: Wrap children in layout.tsx**

In `app/layout.tsx`, import and wrap `{children}` with the error boundary. Do NOT add `"use client"` to layout.tsx — it stays a Server Component.

```tsx
import React from "react"
import type { Metadata } from 'next'
import { DM_Sans, JetBrains_Mono } from 'next/font/google'
import { Analytics } from '@vercel/analytics/next'
import { Toaster } from 'sonner'
import { appConfig } from '@/lib/app-config'
import { ErrorBoundary } from '@/components/error-boundary'
import './globals.css'

const _dmSans = DM_Sans({ subsets: ["latin"], weight: ["400", "500", "600", "700"] });
const _jetbrainsMono = JetBrains_Mono({ subsets: ["latin"] });

export const metadata: Metadata = {
  title: appConfig.name,
  description: appConfig.description,
  icons: {
    icon: [
      { url: appConfig.favicon32, sizes: '32x32', type: 'image/png' },
      { url: appConfig.favicon16, sizes: '16x16', type: 'image/png' },
    ],
    apple: appConfig.appleTouchIcon,
  },
  manifest: appConfig.manifest,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body className={`font-sans antialiased`}>
        <ErrorBoundary>
          {children}
        </ErrorBoundary>
        <Toaster theme="dark" richColors />
        <Analytics />
      </body>
    </html>
  )
}
```

- [ ] **Step 3: Commit**

```bash
git add components/error-boundary.tsx app/layout.tsx
git commit -m "feat: add React error boundary to catch render crashes"
```

---

### Task 6: Create ApiError component

**Files:**
- Create: `components/api-error.tsx`

- [ ] **Step 1: Create api error component**

```tsx
"use client";

import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ApiErrorProps {
  message?: string;
  onRetry?: () => void;
}

export function ApiError({ message = "Something went wrong", onRetry }: ApiErrorProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center space-y-3">
      <AlertTriangle className="h-8 w-8 text-destructive" />
      <p className="text-sm text-muted-foreground">{message}</p>
      {onRetry && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          Try Again
        </Button>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add components/api-error.tsx
git commit -m "feat: add reusable ApiError component with retry button"
```

---

### Task 7: Create BackendBanner with context provider

**Files:**
- Create: `components/backend-banner.tsx`

- [ ] **Step 1: Create backend status provider and banner**

```tsx
"use client";

import React, { createContext, useContext, useState, useCallback } from "react";
import { WifiOff, X } from "lucide-react";

interface BackendStatusContextType {
  backendDown: boolean;
  setBackendDown: (down: boolean) => void;
  clearBackendDown: () => void;
}

const BackendStatusContext = createContext<BackendStatusContextType>({
  backendDown: false,
  setBackendDown: () => {},
  clearBackendDown: () => {},
});

export function useBackendStatus() {
  return useContext(BackendStatusContext);
}

export function BackendStatusProvider({ children }: { children: React.ReactNode }) {
  const [backendDown, setBackendDown] = useState(false);
  const clearBackendDown = useCallback(() => setBackendDown(false), []);

  return (
    <BackendStatusContext.Provider value={{ backendDown, setBackendDown, clearBackendDown }}>
      {children}
    </BackendStatusContext.Provider>
  );
}

export function BackendBanner() {
  const { backendDown, clearBackendDown } = useBackendStatus();
  const [dismissed, setDismissed] = React.useState(false);

  // Reset dismissed state when backend goes down again after recovery
  React.useEffect(() => {
    if (backendDown) setDismissed(false);
  }, [backendDown]);

  if (!backendDown || dismissed) return null;

  return (
    <div className="bg-destructive/10 border-b border-destructive/20 px-4 py-2 flex items-center justify-between">
      <div className="flex items-center gap-2 text-sm text-destructive">
        <WifiOff className="h-4 w-4" />
        <span>Backend service is unavailable. Some features may not work.</span>
      </div>
      <button
        onClick={() => setDismissed(true)}
        className="text-destructive/60 hover:text-destructive"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add components/backend-banner.tsx
git commit -m "feat: add BackendStatusProvider and BackendBanner for backend-down detection"
```

---

### Task 8: Harden hooks/use-api.ts with SWR config and backend status

**Files:**
- Modify: `hooks/use-api.ts`

- [ ] **Step 1: Rewrite use-api.ts with SWR config, timeout, structured mutations**

The full file is large. Key changes:

1. Add `SWR_CONFIG` with `onErrorRetry` (exponential backoff, max 3 retries) and `onSuccess` that clears backend-down state.
2. Replace raw `fetch` in `fetcher` with a timeout-wrapped version (10s).
3. Wrap all mutation helpers to return `{ success: boolean; data?: T; error?: string }` instead of throwing.
4. Integrate `BackendStatusContext` via an exported `ApiProvider` that wraps `SWRConfig` + `BackendStatusProvider`.

Replace the entire file with:

```ts
import useSWR, { SWRConfig, SWRConfiguration } from "swr";
import React from "react";
import { toast } from "sonner";
import { BackendStatusProvider, useBackendStatus } from "@/components/backend-banner";

// ---- Fetch with timeout ----
async function fetchWithTimeout(url: string, init?: RequestInit, timeoutMs = 10000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } finally {
    clearTimeout(timer);
  }
}

// ---- SWR fetcher ----
const fetcher = async (url: string) => {
  const res = await fetchWithTimeout(url);
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: "Request failed" }));
    const error = new Error(body.error ?? `Request failed with status ${res.status}`);
    (error as unknown as Record<string, unknown>).status = res.status;
    throw error;
  }
  return res.json();
};

// ---- Mutation result type ----
type MutationResult<T = unknown> =
  | { success: true; data: T }
  | { success: false; error: string; status?: number };

async function mutate<T = unknown>(
  url: string,
  options: RequestInit,
  errorMessage: string,
): Promise<MutationResult<T>> {
  try {
    const res = await fetchWithTimeout(url, {
      headers: { "Content-Type": "application/json" },
      ...options,
    });
    if (!res.ok) {
      const body = await res.json().catch(() => ({ error: errorMessage }));
      const msg = body.error ?? errorMessage;
      toast.error(msg);
      return { success: false, error: msg, status: res.status };
    }
    const data = await res.json();
    return { success: true, data };
  } catch (err) {
    const msg = err instanceof Error && err.name === "AbortError"
      ? "Request timed out"
      : errorMessage;
    toast.error(msg);
    return { success: false, error: msg };
  }
}

// ---- SWR global config ----
function useAppSWRConfig(): SWRConfiguration {
  const { setBackendDown, clearBackendDown } = useBackendStatus();

  return {
    fetcher,
    onErrorRetry(error, _key, _config, revalidate, { retryCount }) {
      if ((error as Record<string, unknown>).status === 401) return;
      if ((error as Record<string, unknown>).status === 404) return;
      if (retryCount >= 3) {
        setBackendDown(true);
        return;
      }
      const delay = Math.min(1000 * 2 ** retryCount, 10000);
      setTimeout(() => revalidate({ retryCount }), delay);
    },
    onSuccess() {
      clearBackendDown();
    },
  };
}

// ---- API Provider (wrap your app with this) ----
export function ApiProvider({ children }: { children: React.ReactNode }) {
  return React.createElement(
    BackendStatusProvider,
    null,
    React.createElement(ApiProviderInner, null, children),
  );
}

function ApiProviderInner({ children }: { children: React.ReactNode }) {
  const config = useAppSWRConfig();
  return React.createElement(SWRConfig, { value: config }, children);
}

// ---- Products ----
export function useProducts() {
  return useSWR("/api/products", fetcher);
}
export async function createProduct(data: Record<string, unknown>) {
  return mutate("/api/products", { method: "POST", body: JSON.stringify(data) }, "Failed to create product");
}
export async function updateProduct(id: string, data: Record<string, unknown>) {
  return mutate(`/api/products/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update product");
}
export async function deleteProduct(id: string) {
  return mutate(`/api/products/${id}`, { method: "DELETE" }, "Failed to delete product");
}

// ---- Deals ----
export function useDeals() {
  return useSWR("/api/deals", fetcher);
}
export async function createDeal(data: Record<string, unknown>) {
  return mutate("/api/deals", { method: "POST", body: JSON.stringify(data) }, "Failed to create deal");
}
export async function updateDeal(id: string, data: Record<string, unknown>) {
  return mutate(`/api/deals/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update deal");
}
export async function deleteDeal(id: string) {
  return mutate(`/api/deals/${id}`, { method: "DELETE" }, "Failed to delete deal");
}

// ---- Customers ----
export function useCustomers() {
  return useSWR("/api/customers", fetcher);
}
export async function createCustomer(data: Record<string, unknown>) {
  return mutate("/api/customers", { method: "POST", body: JSON.stringify(data) }, "Failed to create customer");
}
export async function updateCustomer(id: string, data: Record<string, unknown>) {
  return mutate(`/api/customers/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update customer");
}
export async function deleteCustomer(id: string) {
  return mutate(`/api/customers/${id}`, { method: "DELETE" }, "Failed to delete customer");
}

// ---- Licenses ----
export function useLicenses() {
  return useSWR("/api/licenses", fetcher);
}
export async function createLicense(data: Record<string, unknown>) {
  return mutate("/api/licenses", { method: "POST", body: JSON.stringify(data) }, "Failed to create license");
}
export async function updateLicense(key: string, data: Record<string, unknown>) {
  return mutate(`/api/licenses/${key}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update license");
}
export async function deleteLicense(key: string) {
  return mutate(`/api/licenses/${key}`, { method: "DELETE" }, "Failed to delete license");
}

// ---- License Activations ----
export function useLicenseActivations(key: string | null) {
  return useSWR(key ? `/api/licenses/${key}/activations` : null, fetcher);
}
export async function deactivateDevice(key: string, deviceId: string) {
  return mutate(`/api/licenses/${key}/activations?deviceId=${encodeURIComponent(deviceId)}`, { method: "DELETE" }, "Failed to deactivate device");
}

// ---- License Signing ----
export function useKeypair() {
  return useSWR("/api/licenses/keypair", fetcher);
}
export async function generateKeypair(confirm?: boolean) {
  return mutate("/api/licenses/keypair", { method: "POST", body: JSON.stringify({ confirm }) }, "Failed to generate keypair");
}
export async function signLicenseKey(
  key: string,
  data: { features?: string[]; maxActivations?: number; metadata?: Record<string, unknown> },
) {
  return mutate(`/api/licenses/${key}/sign`, { method: "POST", body: JSON.stringify(data) }, "Failed to sign license");
}
export function getLicenseExportUrl(key: string) {
  return `/api/licenses/${key}/export`;
}
export async function verifyLicenseFile(licenseFile: string) {
  return mutate("/api/licenses/verify", { method: "POST", body: JSON.stringify({ licenseFile }) }, "Failed to verify license");
}

// ---- API Keys ----
export function useApiKeys() {
  return useSWR("/api/api-keys", fetcher);
}
export async function createApiKey(data: { name: string }) {
  return mutate("/api/api-keys", { method: "POST", body: JSON.stringify(data) }, "Failed to create API key");
}
export async function revokeApiKey(id: string) {
  return mutate(`/api/api-keys/${id}`, { method: "DELETE" }, "Failed to revoke API key");
}

// ---- Pricing Plans ----
export function usePlans() {
  return useSWR("/api/billing/plans", fetcher);
}
export async function createPlan(data: Record<string, unknown>) {
  return mutate("/api/billing/plans", { method: "POST", body: JSON.stringify(data) }, "Failed to create plan");
}
export async function updatePlan(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/plans/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update plan");
}
export async function deletePlan(id: string) {
  return mutate(`/api/billing/plans/${id}`, { method: "DELETE" }, "Failed to delete plan");
}

// ---- Subscriptions ----
export function useSubscriptions() {
  return useSWR("/api/billing/subscriptions", fetcher);
}
export async function createSubscription(data: Record<string, unknown>) {
  return mutate("/api/billing/subscriptions", { method: "POST", body: JSON.stringify(data) }, "Failed to create subscription");
}
export async function updateSubscription(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/subscriptions/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update subscription");
}
export async function deleteSubscription(id: string) {
  return mutate(`/api/billing/subscriptions/${id}`, { method: "DELETE" }, "Failed to delete subscription");
}

// ---- Invoices ----
export function useInvoices() {
  return useSWR("/api/billing/invoices", fetcher);
}
export async function createInvoice(data: Record<string, unknown>) {
  return mutate("/api/billing/invoices", { method: "POST", body: JSON.stringify(data) }, "Failed to create invoice");
}
export async function updateInvoice(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/invoices/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update invoice");
}
export async function deleteInvoice(id: string) {
  return mutate(`/api/billing/invoices/${id}`, { method: "DELETE" }, "Failed to delete invoice");
}
export async function addInvoiceItem(invoiceId: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/invoices/${invoiceId}/items`, { method: "POST", body: JSON.stringify(data) }, "Failed to add invoice item");
}

// ---- Payments ----
export function usePayments(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/payments?invoiceId=${invoiceId}` : "/api/billing/payments";
  return useSWR(url, fetcher);
}
export async function createPayment(data: Record<string, unknown>) {
  return mutate("/api/billing/payments", { method: "POST", body: JSON.stringify(data) }, "Failed to record payment");
}

// ---- Usage Events ----
export function useUsageEvents(subscriptionId: string) {
  return useSWR(subscriptionId ? `/api/billing/usage?subscriptionId=${subscriptionId}` : null, fetcher);
}

// ---- Credit Notes ----
export function useCreditNotes(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/credit-notes?invoiceId=${invoiceId}` : "/api/billing/credit-notes";
  return useSWR(url, fetcher);
}
export async function createCreditNote(data: Record<string, unknown>) {
  return mutate("/api/billing/credit-notes", { method: "POST", body: JSON.stringify(data) }, "Failed to create credit note");
}
export async function updateCreditNote(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/credit-notes/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update credit note");
}
export async function deleteCreditNote(id: string) {
  return mutate(`/api/billing/credit-notes/${id}`, { method: "DELETE" }, "Failed to delete credit note");
}

// ---- Coupons ----
export function useCoupons() {
  return useSWR("/api/billing/coupons", fetcher);
}
export async function createCoupon(data: Record<string, unknown>) {
  return mutate("/api/billing/coupons", { method: "POST", body: JSON.stringify(data) }, "Failed to create coupon");
}
export async function updateCoupon(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/coupons/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update coupon");
}
export async function deleteCoupon(id: string) {
  return mutate(`/api/billing/coupons/${id}`, { method: "DELETE" }, "Failed to delete coupon");
}
export async function applyCoupon(couponId: string, subscriptionId: string, expiresAt?: string) {
  return mutate(`/api/billing/coupons/${couponId}`, {
    method: "PUT",
    body: JSON.stringify({ action: "apply", subscriptionId, couponId, expiresAt }),
  }, "Failed to apply coupon");
}

// ---- Subscription Lifecycle ----
export async function runBillingLifecycle() {
  return mutate("/api/billing/subscriptions/lifecycle", { method: "POST" }, "Failed to run billing lifecycle");
}

// ---- Refunds ----
export function useRefunds(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/refunds?invoiceId=${invoiceId}` : "/api/billing/refunds";
  return useSWR(url, fetcher);
}
export async function createRefund(data: Record<string, unknown>) {
  return mutate("/api/billing/refunds", { method: "POST", body: JSON.stringify(data) }, "Failed to process refund");
}

// ---- Dunning ----
export function useDunningLog(invoiceId?: string) {
  const url = invoiceId ? `/api/billing/dunning?invoiceId=${invoiceId}` : "/api/billing/dunning";
  return useSWR(url, fetcher);
}
export async function runDunning(config?: Record<string, number>) {
  return mutate("/api/billing/dunning", { method: "POST", body: JSON.stringify(config ?? {}) }, "Failed to run dunning");
}

// ---- Stripe ----
export async function getStripeCheckout(invoiceId: string) {
  return mutate(`/api/billing/stripe?invoiceId=${invoiceId}`, { method: "GET" }, "Failed to get Stripe checkout");
}

// ---- Unified Checkout ----
export async function getCheckout(invoiceId: string, provider: "stripe" | "xendit" | "lemonsqueezy") {
  return mutate<{ checkoutUrl: string; provider: string }>(
    `/api/billing/checkout?invoiceId=${invoiceId}&provider=${provider}`,
    { method: "GET" },
    "Checkout failed",
  );
}

// ---- Billing Events ----
export function useBillingEvents(customerId?: string, limit?: number) {
  const params = new URLSearchParams();
  if (customerId) params.set("customerId", customerId);
  if (limit) params.set("limit", String(limit));
  const qs = params.toString();
  return useSWR(`/api/billing/events${qs ? `?${qs}` : ""}`, fetcher);
}

// ---- Webhooks ----
export function useWebhooks() {
  return useSWR("/api/billing/webhooks", fetcher);
}
export function useWebhook(id: string) {
  return useSWR(id ? `/api/billing/webhooks/${id}` : null, fetcher);
}
export async function createWebhook(data: Record<string, unknown>) {
  return mutate("/api/billing/webhooks", { method: "POST", body: JSON.stringify(data) }, "Failed to create webhook");
}
export async function updateWebhook(id: string, data: Record<string, unknown>) {
  return mutate(`/api/billing/webhooks/${id}`, { method: "PUT", body: JSON.stringify(data) }, "Failed to update webhook");
}
export async function deleteWebhook(id: string) {
  return mutate(`/api/billing/webhooks/${id}`, { method: "DELETE" }, "Failed to delete webhook");
}

// ---- PDF ----
export function getInvoicePdfUrl(invoiceId: string) {
  return `/api/billing/invoices/${invoiceId}/pdf`;
}

// ---- Analytics ----
export function useOverviewAnalytics() {
  return useSWR("/api/analytics/overview", fetcher);
}
export function useForecastAnalytics() {
  return useSWR("/api/analytics/forecasting", fetcher);
}
export function useReportsAnalytics() {
  return useSWR("/api/analytics/reports", fetcher);
}

// ---- Search ----
export function useSearch(query: string) {
  return useSWR(
    query && query.length >= 2 ? `/api/search?q=${encodeURIComponent(query)}` : null,
    fetcher,
    { dedupingInterval: 300 },
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add hooks/use-api.ts
git commit -m "feat: add SWR retry config, fetch timeout, structured mutation results, backend-down detection"
```

---

### Task 9: Wire ApiProvider and BackendBanner into dashboard

**Files:**
- Modify: `app/page.tsx`

- [ ] **Step 1: Wrap dashboard with ApiProvider and add BackendBanner**

Add imports at the top of `app/page.tsx`:
```tsx
import { ApiProvider } from "@/hooks/use-api";
import { BackendBanner } from "@/components/backend-banner";
```

Replace the return statement in the `Dashboard` component:
```tsx
  return (
    <ApiProvider>
      <div className="flex min-h-screen bg-background">
        <Sidebar
          activeSection={activeSection}
          onSectionChange={setActiveSection}
          collapsed={sidebarCollapsed}
          onCollapsedChange={setSidebarCollapsed}
        />
        <div
          className={`flex-1 flex flex-col transition-all duration-300 ease-out ${
            sidebarCollapsed ? "ml-[72px]" : "ml-[260px]"
          }`}
        >
          <BackendBanner />
          <Header activeSection={activeSection} onOpenSearch={openSearch} />
          <main className="flex-1 p-6 overflow-auto">
            <div
              key={activeSection}
              className="animate-in fade-in slide-in-from-bottom-4 duration-500"
            >
              {renderSection()}
            </div>
          </main>
        </div>
        <CommandPalette onNavigate={setActiveSection} />
      </div>
    </ApiProvider>
  );
```

- [ ] **Step 2: Commit**

```bash
git add app/page.tsx
git commit -m "feat: wire ApiProvider and BackendBanner into dashboard"
```

---

### Task 10: Update mutation call sites for new return type

**Files:**
- Modify: `components/management/products.tsx`
- Modify: `components/management/deals.tsx`
- Modify: `components/management/customers.tsx`
- Modify: `components/management/licenses.tsx`
- Modify: `components/management/subscriptions.tsx`
- Modify: `components/management/invoices.tsx`
- Modify: `components/management/coupons.tsx`
- Modify: `components/management/plans.tsx`
- Modify: `components/management/webhooks.tsx`
- Modify: `components/dashboard/sections/licenses.tsx`
- Modify: `components/dashboard/sections/settings.tsx`

The mutation helpers now return `{ success, data, error, status? }` instead of throwing. Every call site needs updating.

- [ ] **Step 1: Update standard mutation call sites (fire-and-forget pattern)**

For most files, the pattern is mechanical. Change from:
```tsx
// OLD
try {
  await createProduct(data);
  toast.success("Product created");
  mutate();
} catch {
  toast.error("Failed");
}
```
to:
```tsx
// NEW
const result = await createProduct(data);
if (result.success) {
  toast.success("Product created");
  mutate();
}
// error toast is handled inside mutate() helper — remove try/catch
```

Apply this to all 11 files listed above.

- [ ] **Step 2: Fix special case — `settings.tsx` createApiKey (reads return value)**

`settings.tsx` reads `result.key` from `createApiKey`. Update from:
```tsx
// OLD
const result = await createApiKey({ name: newKeyName.trim() });
setCreatedKey(result.key);
```
to:
```tsx
// NEW
const result = await createApiKey({ name: newKeyName.trim() });
if (result.success) {
  setCreatedKey(result.data.key);
}
```

- [ ] **Step 3: Fix special case — `settings.tsx` verifyLicenseFile (reads return value)**

Update from:
```tsx
// OLD
const result = await verifyLicenseFile(licenseFileContent);
setVerifyResult(result);
```
to:
```tsx
// NEW
const result = await verifyLicenseFile(licenseFileContent);
if (result.success) {
  setVerifyResult(result.data);
}
```

- [ ] **Step 4: Fix special case — `settings.tsx` generateKeypair (uses status code branching)**

The old code catches errors and checks `err.status === 409` to trigger a confirm dialog. Update from:
```tsx
// OLD
try {
  await generateKeypair();
  // success handling...
} catch (err) {
  if ((err as Error & { status?: number }).status === 409) {
    setConfirmRegenOpen(true);
  } else {
    toast.error("Failed to generate keypair");
  }
}
```
to:
```tsx
// NEW
const result = await generateKeypair();
if (result.success) {
  // success handling...
} else if (result.status === 409) {
  setConfirmRegenOpen(true);
}
// other errors show toast automatically
```

- [ ] **Step 5: Commit**

```bash
git add components/management/ components/dashboard/sections/
git commit -m "refactor: update all mutation call sites for structured result pattern"
```

---

## Chunk 3: Docker & Health

### Task 11: Create health endpoint

**Files:**
- Create: `app/health/route.ts`

- [ ] **Step 1: Create health Route Handler**

A Route Handler at `/health` (not under `/api/*`, so the middleware gate won't block it). Returns JSON directly without the layout overhead.

```ts
export const dynamic = "force-dynamic";

export function GET() {
  return Response.json({ status: "ok", timestamp: new Date().toISOString() });
}
```

The middleware already has `/health` in the public paths list (Task 4), so this is accessible without a session cookie.

- [ ] **Step 2: Commit**

```bash
git add app/health/route.ts
git commit -m "feat: add /health endpoint for Docker health checks"
```

---

### Task 12: Update docker-compose.prod.yml

**Files:**
- Modify: `docker-compose.prod.yml`

**Note:** `deploy.resources.limits` requires Docker Compose v2 plugin (`docker compose up`) or `docker-compose --compatibility`. Silently ignored in classic v1 without `--compatibility`.

- [ ] **Step 1: Rewrite docker-compose.prod.yml**

```yaml
services:
  postgres:
    image: postgres:17-alpine
    container_name: rantai-billing-db
    restart: unless-stopped
    environment:
      POSTGRES_USER: rantai_billing
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}
      POSTGRES_DB: rantai_billing
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rantai_billing"]
      interval: 5s
      timeout: 3s
      retries: 5
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "1.0"

  rust-backend:
    image: ${RUST_BACKEND_IMAGE:-rantai-billing-rs:latest}
    container_name: rantai-billing-rs
    restart: unless-stopped
    environment:
      DATABASE_URL: postgresql://rantai_billing:${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required}@postgres:5432/rantai_billing
    env_file:
      - .env
    depends_on:
      postgres:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:8080/health || exit 1"]
      interval: 5s
      timeout: 3s
      retries: 5
      start_period: 5s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "1.0"

  app:
    image: rantai-billing:latest
    container_name: rantai-billing-app
    restart: unless-stopped
    ports:
      - "${APP_PORT:-13300}:3000"
    environment:
      RUST_BACKEND_URL: http://rust-backend:8080
      NODE_ENV: production
    env_file:
      - .env
    depends_on:
      rust-backend:
        condition: service_healthy
    healthcheck:
      test: ["CMD-SHELL", "curl -sf http://localhost:3000/health || exit 1"]
      interval: 5s
      timeout: 3s
      retries: 5
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 512M
          cpus: "1.0"

volumes:
  pgdata:
```

- [ ] **Step 2: Commit**

```bash
git add docker-compose.prod.yml
git commit -m "feat: add Rust backend service, fix health checks, require POSTGRES_PASSWORD, add resource limits"
```

---

### Task 13: Fix TypeScript build errors

**Files:**
- Various (depends on what errors exist)

Since we set `ignoreBuildErrors: false` in Task 1, we need to ensure the build passes.

- [ ] **Step 1: Run build and capture errors**

Run: `bun run build 2>&1 | tail -50`

- [ ] **Step 2: Fix all TypeScript errors**

Work through each error. Common issues:
- Unused imports
- Type mismatches
- Missing type annotations

- [ ] **Step 3: Verify build passes**

Run: `bun run build`
Expected: Build completes successfully.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: resolve TypeScript build errors after enabling strict build checks"
```

---

### Task 14: Final verification

- [ ] **Step 1: Run linter**

Run: `bun lint`
Fix any issues.

- [ ] **Step 2: Run build one more time**

Run: `bun run build`
Expected: Clean build with no errors.

- [ ] **Step 3: Commit any final fixes**

```bash
git add -A
git commit -m "chore: fix lint issues for production readiness"
```
