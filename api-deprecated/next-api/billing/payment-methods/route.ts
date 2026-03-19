import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

function makeHeaders(req: NextRequest, includeJson: boolean) {
  const headers: HeadersInit = {};
  const cookie = req.headers.get("cookie");
  if (cookie) headers.cookie = cookie;
  if (includeJson) headers["Content-Type"] = "application/json";
  return headers;
}

export async function GET(req: NextRequest) {
  const { searchParams } = new URL(req.url);
  const qs = searchParams.toString();
  const url = `${BACKEND}/api/billing/payment-methods${qs ? `?${qs}` : ""}`;
  const res = await fetch(url, { headers: makeHeaders(req, false) });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}

export async function POST(req: NextRequest) {
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/payment-methods`, {
    method: "POST",
    headers: makeHeaders(req, true),
    body,
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
