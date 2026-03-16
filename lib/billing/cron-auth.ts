import { NextRequest, NextResponse } from "next/server";

/**
 * Verify that a request has the correct CRON_SECRET header.
 * Returns null on success, or a NextResponse error on failure.
 * If CRON_SECRET is not set (dev mode), always allows the request.
 */
export function verifyCronSecret(req: NextRequest): NextResponse | null {
  const secret = process.env.CRON_SECRET;
  if (!secret) return null; // Dev mode — no protection

  const provided =
    req.headers.get("x-cron-secret") ??
    req.headers.get("authorization")?.replace("Bearer ", "");

  if (provided !== secret) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 });
  }

  return null;
}
