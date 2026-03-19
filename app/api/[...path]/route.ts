import { NextRequest, NextResponse } from "next/server";

const BACKEND = process.env.RUST_BACKEND_URL;

function backendUrl(req: NextRequest, path: string[]): string {
  const base = BACKEND?.replace(/\/+$/, "");
  if (!base) return "";
  const search = req.nextUrl.search || "";
  const joinedPath = path.map(encodeURIComponent).join("/");
  return `${base}/api/${joinedPath}${search}`;
}

function forwardHeaders(req: NextRequest): Headers {
  const headers = new Headers();
  for (const [key, value] of req.headers.entries()) {
    const lower = key.toLowerCase();
    if (lower === "host" || lower === "content-length") continue;
    headers.set(key, value);
  }
  return headers;
}

async function proxy(req: NextRequest, path: string[]): Promise<NextResponse> {
  const url = backendUrl(req, path);
  if (!url) {
    return NextResponse.json(
      { error: "RUST_BACKEND_URL is required for /api proxy" },
      { status: 503 },
    );
  }

  const response = await fetch(url, {
    method: req.method,
    headers: forwardHeaders(req),
    body: req.method === "GET" || req.method === "HEAD" ? undefined : await req.arrayBuffer(),
    redirect: "manual",
  });

  const responseHeaders = new Headers(response.headers);
  return new NextResponse(response.body, {
    status: response.status,
    headers: responseHeaders,
  });
}

type RouteContext = { params: Promise<{ path: string[] }> };

export async function GET(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}

export async function POST(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}

export async function PUT(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}

export async function PATCH(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}

export async function DELETE(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}

export async function OPTIONS(req: NextRequest, ctx: RouteContext) {
  const { path } = await ctx.params;
  return proxy(req, path);
}
