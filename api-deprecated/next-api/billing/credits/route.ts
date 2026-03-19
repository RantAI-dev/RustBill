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
  const customerId = searchParams.get("customerId");
  const currency = searchParams.get("currency");
  if (!customerId) {
    return NextResponse.json({ error: "customerId is required" }, { status: 400 });
  }
  const qs = new URLSearchParams();
  if (currency) qs.set("currency", currency);
  const res = await fetch(
    `${BACKEND}/api/billing/credits/${customerId}${qs.toString() ? `?${qs.toString()}` : ""}`,
    { headers: makeHeaders(req, false) },
  );
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}

export async function POST(req: NextRequest) {
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/credits/adjust`, {
    method: "POST",
    headers: makeHeaders(req, true),
    body,
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
