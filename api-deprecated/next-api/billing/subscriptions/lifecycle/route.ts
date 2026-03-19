import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

function makeHeaders(req: NextRequest, includeJson: boolean) {
  const headers: HeadersInit = {};
  const cookie = req.headers.get("cookie");
  const auth = req.headers.get("authorization");
  const cronSecret = req.headers.get("x-cron-secret");

  if (cookie) headers.cookie = cookie;
  if (auth) headers.authorization = auth;
  if (cronSecret) headers["x-cron-secret"] = cronSecret;
  if (includeJson) headers["Content-Type"] = "application/json";

  return headers;
}

export async function POST(req: NextRequest) {
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/subscriptions/lifecycle`, {
    method: "POST",
    headers: makeHeaders(req, true),
    body,
  });

  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
