import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

export async function GET() {
  const res = await fetch(`${BACKEND}/api/billing/tax-rules`);
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}

export async function POST(req: NextRequest) {
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/tax-rules`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
