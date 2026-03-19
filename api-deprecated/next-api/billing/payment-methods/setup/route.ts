import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

function makeHeaders(req: NextRequest) {
  const headers: HeadersInit = { "Content-Type": "application/json" };
  const cookie = req.headers.get("cookie");
  if (cookie) headers.cookie = cookie;
  return headers;
}

export async function POST(req: NextRequest) {
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/payment-methods/setup`, {
    method: "POST",
    headers: makeHeaders(req),
    body,
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
