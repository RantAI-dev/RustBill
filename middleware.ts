import { NextRequest, NextResponse } from "next/server";

const SESSION_COOKIE = "session";

export function middleware(req: NextRequest) {
  const { pathname } = req.nextUrl;

  // ---- 1. Public API (license verification) — CORS only, no auth ----
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
    return response;
  }

  // ---- 2. Auth routes — always public ----
  if (pathname.startsWith("/api/auth/")) {
    return NextResponse.next();
  }

  // ---- 3. Login page — redirect to / if already has session cookie ----
  if (pathname === "/login") {
    const hasSession = req.cookies.has(SESSION_COOKIE);
    if (hasSession) {
      return NextResponse.redirect(new URL("/", req.url));
    }
    return NextResponse.next();
  }

  // ---- 4. All other routes — require session cookie ----
  const hasSession = req.cookies.has(SESSION_COOKIE);

  if (!hasSession) {
    // API routes → 401 JSON
    if (pathname.startsWith("/api/")) {
      return NextResponse.json(
        { error: "Unauthorized" },
        { status: 401 }
      );
    }
    // Page routes → redirect to login with return path
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("from", pathname);
    return NextResponse.redirect(loginUrl);
  }

  return NextResponse.next();
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico|icon.*|apple-icon.*|manifest|logo/).*)",
  ],
};
