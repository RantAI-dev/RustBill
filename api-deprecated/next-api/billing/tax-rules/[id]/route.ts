import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

function makeHeaders(req: NextRequest, includeJson: boolean) {
  const headers: HeadersInit = {};
  const cookie = req.headers.get("cookie");
  if (cookie) headers.cookie = cookie;
  if (includeJson) headers["Content-Type"] = "application/json";
  return headers;
}

export async function PUT(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const body = await req.text();
  const res = await fetch(`${BACKEND}/api/billing/tax-rules/${id}`, {
    method: "PUT",
    headers: makeHeaders(req, true),
    body,
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}

export async function DELETE(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const res = await fetch(`${BACKEND}/api/billing/tax-rules/${id}`, {
    method: "DELETE",
    headers: makeHeaders(req, false),
  });
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
