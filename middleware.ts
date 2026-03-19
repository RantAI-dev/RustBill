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
    "script-src 'self' 'unsafe-inline' 'unsafe-eval' https://va.vercel-scripts.com",
    "style-src 'self' 'unsafe-inline'",
    "connect-src 'self' https://vitals.vercel-insights.com https://va.vercel-scripts.com",
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

  // ---- 2. Rust API mode only ----
  if (pathname.startsWith("/api/")) {
    if (!RUST_BACKEND) {
      return NextResponse.json(
        { error: "RUST_BACKEND_URL is required for API routes" },
        { status: 503 },
      );
    }
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
    "/((?!_next/static|_next/image|favicon.ico|icon.*|apple-icon.*|manifest|logo/|rustbill-logo/).*)",
  ],
};
