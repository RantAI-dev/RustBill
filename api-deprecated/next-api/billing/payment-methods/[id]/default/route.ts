import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL ?? "http://localhost:8080";

function makeHeaders(req: NextRequest) {
  const headers: HeadersInit = {};
  const cookie = req.headers.get("cookie");
  if (cookie) headers.cookie = cookie;
  return headers;
}

export async function POST(
  req: NextRequest,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const { searchParams } = new URL(req.url);
  const qs = searchParams.toString();
  const res = await fetch(
    `${BACKEND}/api/billing/payment-methods/${id}/default${qs ? `?${qs}` : ""}`,
    {
      method: "POST",
      headers: makeHeaders(req),
    },
  );
  const data = await res.json();
  return NextResponse.json(data, { status: res.status });
}
