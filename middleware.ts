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
